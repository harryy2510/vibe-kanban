# VibePilot Takeover Pipeline Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a single GitHub Actions workflow that, on every push to `main`, rebuilds and redeploys all three shipped products (CLI/MCP binaries on npm + R2, remote API on GHCR + Dokploy, relay on GHCR + Dokploy), fully rebranded to `vibepilot.org` with no upstream dependencies.

**Architecture:** One `release.yml` workflow replaces 8 upstream workflows. Three parallel jobs (CLI matrix, remote image, relay image) feed into deploy jobs (Dokploy webhook, oxmgr SSH restart). Consumer command becomes `bunx -y @harryy/vibe-kanban@latest`.

**Tech Stack:** GitHub Actions, Cargo + Rust nightly, esbuild, Docker Buildx, GHCR, Cloudflare R2 (via AWS CLI), npm, oxmgr (SSH).

**Spec:** `docs/superpowers/specs/2026-04-12-fork-release-pipeline-design.md`

---

## File Structure

**Create:**
- `.github/workflows/release.yml` — single release pipeline

**Modify:**
- `npx-cli/package.json` — rename to `@harryy/vibe-kanban`, update repo URL
- `crates/remote/Cargo.toml` — strip billing dep + feature flag
- `crates/review/src/main.rs` — change `DEFAULT_API_URL` default
- `oxfile.toml` — update command + env for new package + domains
- `.github/workflows/test.yml` — remove SSH agent step

**Delete:**
- `.github/workflows/pre-release.yml`
- `.github/workflows/publish.yml`
- `.github/workflows/remote-deploy-dev.yml`
- `.github/workflows/remote-deploy-prod.yml`
- `.github/workflows/remote-release.yml`
- `.github/workflows/relay-deploy-dev.yml`
- `.github/workflows/relay-deploy-prod.yml`
- `.github/workflows/relay-release.yml`
- `update.sh` (replaced by workflow)
- `setup-gh.sh` (was ignored; leaving it alone — user deletes manually)

**Not changed (deferred):**
- All `docs/**/*.mdx` references — docs aren't deployed by this pipeline, low priority
- Frontend `vibekanban.com` URLs in `packages/**` (marketing links; don't block the pipeline)
- `local-build.sh` domain envs (irrelevant to CI)
- `crates/git/src/lib.rs` default committer email — functionally harmless

---

## Task 1: Strip billing dependency

**Files:**
- Modify: `crates/remote/Cargo.toml:11-17`

- [ ] **Step 1: Remove billing dep lines**

Edit `crates/remote/Cargo.toml`, replace the block:

```toml
[features]
default = []
vk-billing = ["dep:billing"]

[dependencies]
# private crate for billing functionality
billing = { git = "ssh://git@github.com/BloopAI/vibe-kanban-private", branch = "main", package = "billing", optional = true }

anyhow = "1.0"
```

with:

```toml
[features]
default = []
vk-billing = []

[dependencies]
anyhow = "1.0"
```

- [ ] **Step 2: Regenerate Cargo.lock**

Run: `cargo update -p remote`
Expected: Updates Cargo.lock, removes `billing` entry.

- [ ] **Step 3: Verify remote crate builds without billing**

Run: `cargo check -p remote`
Expected: clean compile, no mention of `billing` or `vibe-kanban-private`.

- [ ] **Step 4: Verify no other crates reference billing feature**

Run: `git grep -l "vk-billing\|feature = \"billing\"" crates/`
Expected: only `crates/remote/Cargo.toml` matches, no other callers.

- [ ] **Step 5: Commit**

```bash
git add crates/remote/Cargo.toml Cargo.lock
git commit -m "fork: strip private billing dependency"
```

---

## Task 2: Update default API URL in review CLI

**Files:**
- Modify: `crates/review/src/main.rs:21`

- [ ] **Step 1: Change DEFAULT_API_URL constant**

Replace line 21 of `crates/review/src/main.rs`:

```rust
const DEFAULT_API_URL: &str = "https://api.vibekanban.com";
```

with:

```rust
const DEFAULT_API_URL: &str = "https://api.vibepilot.org";
```

- [ ] **Step 2: Verify compile**

Run: `cargo check -p review`
Expected: clean compile.

- [ ] **Step 3: Commit**

```bash
git add crates/review/src/main.rs
git commit -m "fork: point review CLI default API URL to vibepilot.org"
```

---

## Task 3: Rename npm package to @harryy scope

**Files:**
- Modify: `npx-cli/package.json:2,17`

- [ ] **Step 1: Update package name and repo URL**

Replace `npx-cli/package.json` contents:

```json
{
  "name": "@harryy/vibe-kanban",
  "private": false,
  "version": "0.1.44",
  "main": "index.js",
  "bin": {
    "vibe-kanban": "bin/cli.js"
  },
  "scripts": {
    "build": "esbuild src/cli.ts --bundle --platform=node --target=node20 --format=cjs --outfile=bin/cli.js --external:adm-zip --banner:js=\"#!/usr/bin/env node\"",
    "check": "tsc --noEmit -p tsconfig.json"
  },
  "keywords": [],
  "author": "harryy2510",
  "repository": {
    "type": "git",
    "url": "https://github.com/harryy2510/vibe-kanban"
  },
  "engines": {
    "node": ">=20.19.0"
  },
  "license": "",
  "description": "VibePilot fork of vibe-kanban CLI (npx wrapper for binaries)",
  "devDependencies": {
    "esbuild": "^0.27.2"
  },
  "dependencies": {
    "adm-zip": "^0.5.16",
    "cac": "^7.0.0"
  },
  "files": [
    "bin",
    "dist"
  ]
}
```

- [ ] **Step 2: Verify npx-cli still type-checks**

Run: `cd npx-cli && npm install && npm run check`
Expected: clean TS output.

- [ ] **Step 3: Commit**

```bash
git add npx-cli/package.json npx-cli/package-lock.json
git commit -m "fork: rename npm package to @harryy/vibe-kanban"
```

---

## Task 4: Update oxfile.toml for new package + domains

**Files:**
- Modify: `oxfile.toml`

- [ ] **Step 1: Replace oxfile.toml contents**

```toml
version = 1

[defaults]
restart_policy = "on_failure"
max_restarts = 10
stop_timeout_secs = 5

[[apps]]
name = "vibe-kanban"
command = "bunx -y @harryy/vibe-kanban@latest"
health_cmd = "curl -fsS http://localhost:4040"

[apps.env]
VK_SHARED_API_BASE = "https://api.vibepilot.org"
VK_SHARED_RELAY_API_BASE = "https://relay.vibepilot.org"
PORT = "4040"
HOST = "0.0.0.0"
```

- [ ] **Step 2: Commit**

```bash
git add oxfile.toml
git commit -m "fork: point oxfile.toml at @harryy/vibe-kanban + vibepilot.org"
```

---

## Task 5: Delete unused upstream workflows and update.sh

**Files:**
- Delete: 8 workflow files + `update.sh`

- [ ] **Step 1: Delete upstream release/deploy workflows and update.sh**

```bash
git rm \
  .github/workflows/pre-release.yml \
  .github/workflows/publish.yml \
  .github/workflows/remote-deploy-dev.yml \
  .github/workflows/remote-deploy-prod.yml \
  .github/workflows/remote-release.yml \
  .github/workflows/relay-deploy-dev.yml \
  .github/workflows/relay-deploy-prod.yml \
  .github/workflows/relay-release.yml \
  update.sh
```

- [ ] **Step 2: Commit**

```bash
git commit -m "fork: remove upstream release/deploy workflows and update.sh"
```

---

## Task 6: Strip SSH agent step from test.yml

**Files:**
- Modify: `.github/workflows/test.yml` (lines around 246)

- [ ] **Step 1: Find the SSH agent step**

Run: `grep -n "VK_PRIVATE_DEPLOY_KEY\|webfactory/ssh-agent" .github/workflows/test.yml`
Expected: one or two line numbers around 242-250.

- [ ] **Step 2: Remove the `Setup SSH Agent for private dependencies` step**

Open `.github/workflows/test.yml`, locate the block (shape below, line numbers may differ):

```yaml
      - name: Setup SSH Agent for private dependencies
        id: ssh-setup
        if: ${{ github.event_name != 'pull_request' || github.event.pull_request.head.repo.full_name == github.repository }}
        uses: webfactory/ssh-agent@...
        with:
          ssh-private-key: ${{ secrets.VK_PRIVATE_DEPLOY_KEY }}
```

Delete the entire `- name: Setup SSH Agent ...` step (including all its sub-lines until the next `- name:` at the same indent). Do not delete the `uses: actions/checkout@v6` or any other step.

- [ ] **Step 3: Verify no remaining references**

Run: `grep -n "VK_PRIVATE_DEPLOY_KEY" .github/workflows/test.yml`
Expected: no output.

- [ ] **Step 4: Verify workflow YAML is still valid**

Run: `gh workflow view test.yml --repo harryy2510/vibe-kanban 2>&1 | head -5` (only if pushed), otherwise skip.

Local check: `cat .github/workflows/test.yml | python3 -c 'import sys, yaml; yaml.safe_load(sys.stdin); print("OK")'`
Expected: `OK`

- [ ] **Step 5: Commit**

```bash
git add .github/workflows/test.yml
git commit -m "fork: remove VK_PRIVATE_DEPLOY_KEY SSH agent step from test.yml"
```

---

## Task 7: Create release.yml — workflow skeleton + bump-version job

**Files:**
- Create: `.github/workflows/release.yml`

- [ ] **Step 1: Write initial workflow with bump-version job only**

Create `.github/workflows/release.yml`:

```yaml
name: Release

on:
  push:
    branches:
      - main
    paths-ignore:
      - 'docs/**'
      - 'README.md'
      - '.gitignore'
  workflow_dispatch:

concurrency:
  group: release-${{ github.ref_name }}
  cancel-in-progress: false

permissions:
  contents: write
  packages: write
  id-token: write

env:
  NODE_VERSION: 22
  PNPM_VERSION: 10.13.1
  RUST_TOOLCHAIN: nightly-2025-12-04

jobs:
  bump-version:
    runs-on: ubuntu-latest
    outputs:
      new_tag: ${{ steps.version.outputs.new_tag }}
      new_version: ${{ steps.version.outputs.new_version }}
    steps:
      - uses: actions/checkout@v6
        with:
          token: ${{ secrets.GITHUB_TOKEN }}
          ssh-key: ${{ secrets.DEPLOY_KEY }}

      - uses: actions/setup-node@v5
        with:
          node-version: ${{ env.NODE_VERSION }}

      - name: Install cargo-edit
        uses: taiki-e/cache-cargo-install-action@34ce5120836e5f9f1508d8713d7fdea0e8facd6f
        with:
          tool: cargo-edit
          git: https://github.com/killercup/cargo-edit
          rev: 96a3879fe3bafda6d0f943b642997fbf03e235cd

      - name: Determine new version
        id: version
        run: |
          latest_npm=$(npm view @harryy/vibe-kanban version 2>/dev/null || echo "0.0.0")
          current=$(node -p "require('./package.json').version")
          base=$(node -e "
            const a='$latest_npm'.split('.').map(Number), b='$current'.split('.').map(Number);
            for (let i=0;i<3;i++){
              if ((a[i]||0)>(b[i]||0)){console.log('$latest_npm');process.exit()}
              if ((a[i]||0)<(b[i]||0)){console.log('$current');process.exit()}
            }
            console.log('$current')
          ")
          npm version "$base" --no-git-tag-version --allow-same-version
          npm version patch --no-git-tag-version
          new_version=$(node -p "require('./package.json').version")
          timestamp=$(date +%Y%m%d%H%M%S)
          new_tag="v${new_version}-${timestamp}"
          echo "new_version=$new_version" >> $GITHUB_OUTPUT
          echo "new_tag=$new_tag" >> $GITHUB_OUTPUT
          (cd npx-cli && npm version "$new_version" --no-git-tag-version --allow-same-version)
          (cd packages/local-web && npm version "$new_version" --no-git-tag-version --allow-same-version)
          cargo set-version --workspace "$new_version"

      - name: Commit and tag
        run: |
          git config user.name "vibepilot-ci"
          git config user.email "ci@vibepilot.org"
          git add package.json npx-cli/package.json packages/local-web/package.json Cargo.toml crates/*/Cargo.toml Cargo.lock
          git commit -m "chore: bump version to ${{ steps.version.outputs.new_version }}"
          git tag "${{ steps.version.outputs.new_tag }}"
          git push origin HEAD:main
          git push origin "${{ steps.version.outputs.new_tag }}"
```

- [ ] **Step 2: Validate YAML syntax**

Run: `cat .github/workflows/release.yml | python3 -c 'import sys, yaml; yaml.safe_load(sys.stdin); print("OK")'`
Expected: `OK`

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "ci: add release.yml skeleton with bump-version job"
```

---

## Task 8: Add build-frontend job

**Files:**
- Modify: `.github/workflows/release.yml`

- [ ] **Step 1: Append build-frontend job after bump-version**

Append to the `jobs:` block in `.github/workflows/release.yml`:

```yaml
  build-frontend:
    needs: bump-version
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v6
        with:
          ref: ${{ needs.bump-version.outputs.new_tag }}

      - uses: actions/setup-node@v5
        with:
          node-version: ${{ env.NODE_VERSION }}

      - uses: pnpm/action-setup@v4
        with:
          version: ${{ env.PNPM_VERSION }}

      - name: Install dependencies
        run: pnpm install --frozen-lockfile

      - name: Build local-web
        run: pnpm -C packages/local-web build

      - name: Upload frontend artifact
        uses: actions/upload-artifact@v6
        with:
          name: frontend-dist
          path: packages/local-web/dist/
          retention-days: 1
```

- [ ] **Step 2: Validate YAML**

Run: `python3 -c 'import yaml; yaml.safe_load(open(".github/workflows/release.yml"))' && echo OK`
Expected: `OK`

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "ci: add build-frontend job to release workflow"
```

---

## Task 9: Add build-cli matrix job (5 platforms)

**Files:**
- Modify: `.github/workflows/release.yml`

- [ ] **Step 1: Append build-cli job**

Append to `release.yml`:

```yaml
  build-cli:
    needs: [bump-version, build-frontend]
    strategy:
      fail-fast: false
      matrix:
        include:
          - target: x86_64-unknown-linux-musl
            os: ubuntu-latest
            name: linux-x64
          - target: aarch64-unknown-linux-musl
            os: ubuntu-latest
            name: linux-arm64
          - target: x86_64-apple-darwin
            os: macos-latest
            name: macos-x64
          - target: aarch64-apple-darwin
            os: macos-latest
            name: macos-arm64
          - target: x86_64-pc-windows-msvc
            os: windows-latest
            name: windows-x64
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v6
        with:
          ref: ${{ needs.bump-version.outputs.new_tag }}

      - uses: dtolnay/rust-toolchain@master
        with:
          toolchain: ${{ env.RUST_TOOLCHAIN }}
          targets: ${{ matrix.target }}

      - uses: Swatinem/rust-cache@v2
        with:
          key: ${{ matrix.target }}

      - name: Download frontend artifact
        uses: actions/download-artifact@v7
        with:
          name: frontend-dist
          path: packages/local-web/dist/

      - name: Install Linux build deps
        if: runner.os == 'Linux'
        run: |
          sudo apt-get update
          sudo apt-get install -y musl-tools clang libclang-dev

      - name: Install cross for Linux arm64
        if: matrix.target == 'aarch64-unknown-linux-musl'
        uses: taiki-e/install-action@v2
        with:
          tool: cross

      - name: Build (Linux arm64 via cross)
        if: matrix.target == 'aarch64-unknown-linux-musl'
        shell: bash
        run: cross build --release --target ${{ matrix.target }} -p server -p mcp -p review --bin server --bin vibe-kanban-mcp --bin review

      - name: Build (native)
        if: matrix.target != 'aarch64-unknown-linux-musl'
        shell: bash
        run: cargo build --release --target ${{ matrix.target }} -p server -p mcp -p review --bin server --bin vibe-kanban-mcp --bin review

      - name: Package zips (Unix)
        if: runner.os != 'Windows'
        shell: bash
        run: |
          mkdir -p dist/${{ matrix.name }}
          cd target/${{ matrix.target }}/release
          zip ../../../dist/${{ matrix.name }}/vibe-kanban.zip server
          mv server vibe-kanban && zip ../../../dist/${{ matrix.name }}/vibe-kanban.zip vibe-kanban && rm vibe-kanban
          zip ../../../dist/${{ matrix.name }}/vibe-kanban-mcp.zip vibe-kanban-mcp
          mv review vibe-kanban-review && zip ../../../dist/${{ matrix.name }}/vibe-kanban-review.zip vibe-kanban-review

      - name: Package zips (Windows)
        if: runner.os == 'Windows'
        shell: pwsh
        run: |
          New-Item -ItemType Directory -Force -Path dist/${{ matrix.name }}
          cd target/${{ matrix.target }}/release
          Rename-Item server.exe vibe-kanban.exe
          Compress-Archive -Path vibe-kanban.exe -DestinationPath ../../../dist/${{ matrix.name }}/vibe-kanban.zip
          Compress-Archive -Path vibe-kanban-mcp.exe -DestinationPath ../../../dist/${{ matrix.name }}/vibe-kanban-mcp.zip
          Rename-Item review.exe vibe-kanban-review.exe
          Compress-Archive -Path vibe-kanban-review.exe -DestinationPath ../../../dist/${{ matrix.name }}/vibe-kanban-review.zip

      - name: Upload platform zips
        uses: actions/upload-artifact@v6
        with:
          name: cli-${{ matrix.name }}
          path: dist/${{ matrix.name }}/
          retention-days: 1
```

- [ ] **Step 2: Validate YAML**

Run: `python3 -c 'import yaml; yaml.safe_load(open(".github/workflows/release.yml"))' && echo OK`
Expected: `OK`

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "ci: add build-cli matrix for 5 platforms"
```

---

## Task 10: Add upload-r2 job

**Files:**
- Modify: `.github/workflows/release.yml`

- [ ] **Step 1: Append upload-r2 job**

```yaml
  upload-r2:
    needs: [bump-version, build-cli]
    runs-on: ubuntu-latest
    steps:
      - name: Download all platform zips
        uses: actions/download-artifact@v7
        with:
          pattern: cli-*
          path: binaries/
          merge-multiple: false

      - name: Flatten layout
        run: |
          mkdir -p flat
          for dir in binaries/cli-*; do
            platform=${dir#binaries/cli-}
            mkdir -p flat/$platform
            cp $dir/*.zip flat/$platform/
          done
          find flat/

      - name: Generate manifest
        run: |
          node -e "
            const fs=require('fs'), crypto=require('crypto');
            const manifest={version:'${{ needs.bump-version.outputs.new_tag }}',platforms:{}};
            const platforms=['linux-x64','linux-arm64','macos-x64','macos-arm64','windows-x64'];
            const binaries=['vibe-kanban','vibe-kanban-mcp','vibe-kanban-review'];
            for (const p of platforms) {
              manifest.platforms[p]={};
              for (const b of binaries) {
                const zp='flat/'+p+'/'+b+'.zip';
                if (fs.existsSync(zp)) {
                  const d=fs.readFileSync(zp);
                  manifest.platforms[p][b]={sha256:crypto.createHash('sha256').update(d).digest('hex'),size:d.length};
                }
              }
            }
            fs.writeFileSync('manifest.json', JSON.stringify(manifest,null,2));
          "
          cat manifest.json

      - name: Configure AWS CLI for R2
        run: |
          aws configure set aws_access_key_id ${{ secrets.R2_BINARIES_ACCESS_KEY_ID }}
          aws configure set aws_secret_access_key ${{ secrets.R2_BINARIES_SECRET_ACCESS_KEY }}
          aws configure set default.region auto

      - name: Upload zips to R2
        env:
          ENDPOINT: ${{ vars.R2_BINARIES_ENDPOINT }}
          BUCKET: ${{ vars.R2_BINARIES_BUCKET }}
          TAG: ${{ needs.bump-version.outputs.new_tag }}
        run: |
          for platform in linux-x64 linux-arm64 macos-x64 macos-arm64 windows-x64; do
            for b in vibe-kanban vibe-kanban-mcp vibe-kanban-review; do
              if [ -f "flat/$platform/$b.zip" ]; then
                aws s3 cp "flat/$platform/$b.zip" \
                  "s3://$BUCKET/binaries/$TAG/$platform/$b.zip" \
                  --endpoint-url "$ENDPOINT"
              fi
            done
          done
          aws s3 cp manifest.json "s3://$BUCKET/binaries/$TAG/manifest.json" --endpoint-url "$ENDPOINT" --content-type application/json
          echo "{\"latest\": \"${{ needs.bump-version.outputs.new_version }}\"}" | \
            aws s3 cp - "s3://$BUCKET/binaries/manifest.json" --endpoint-url "$ENDPOINT" --content-type application/json
```

- [ ] **Step 2: Validate YAML**

Run: `python3 -c 'import yaml; yaml.safe_load(open(".github/workflows/release.yml"))' && echo OK`
Expected: `OK`

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "ci: upload CLI zips + manifest to R2"
```

---

## Task 11: Add publish-npm job

**Files:**
- Modify: `.github/workflows/release.yml`

- [ ] **Step 1: Append publish-npm job**

```yaml
  publish-npm:
    needs: [bump-version, upload-r2]
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v6
        with:
          ref: ${{ needs.bump-version.outputs.new_tag }}

      - uses: actions/setup-node@v5
        with:
          node-version: ${{ env.NODE_VERSION }}
          registry-url: 'https://registry.npmjs.org'

      - name: Install npx-cli deps
        run: cd npx-cli && npm ci

      - name: Build npx-cli
        run: cd npx-cli && npm run build

      - name: Bake R2 URL and tag into bundle
        run: |
          cd npx-cli
          sed -i "s|__R2_PUBLIC_URL__|${{ vars.R2_BINARIES_PUBLIC_URL }}|g" bin/cli.js
          sed -i "s|__BINARY_TAG__|${{ needs.bump-version.outputs.new_tag }}|g" bin/cli.js

      - name: Pack and publish
        run: |
          cd npx-cli
          npm pack
          npm publish *.tgz --access public
        env:
          NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }}

      - name: Attach tgz to GitHub release
        uses: softprops/action-gh-release@153bb8e04406b158c6c84fc1615b65b24149a1fe
        with:
          tag_name: ${{ needs.bump-version.outputs.new_tag }}
          generate_release_notes: true
          files: |
            npx-cli/*.tgz
```

- [ ] **Step 2: Validate YAML**

Run: `python3 -c 'import yaml; yaml.safe_load(open(".github/workflows/release.yml"))' && echo OK`
Expected: `OK`

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "ci: add publish-npm job"
```

---

## Task 12: Add build-remote-image job

**Files:**
- Modify: `.github/workflows/release.yml`

- [ ] **Step 1: Append build-remote-image job**

```yaml
  build-remote-image:
    needs: bump-version
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v6
        with:
          ref: ${{ needs.bump-version.outputs.new_tag }}

      - uses: docker/setup-buildx-action@v3

      - name: Log in to GHCR
        uses: docker/login-action@v3
        with:
          registry: ghcr.io
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}

      - name: Build and push
        uses: docker/build-push-action@v6
        with:
          context: .
          file: crates/remote/Dockerfile
          push: true
          tags: |
            ghcr.io/harryy2510/vibe-kanban-remote:latest
            ghcr.io/harryy2510/vibe-kanban-remote:${{ github.sha }}
            ghcr.io/harryy2510/vibe-kanban-remote:${{ needs.bump-version.outputs.new_tag }}
          build-args: |
            VITE_RELAY_API_BASE_URL=https://relay.vibepilot.org
          cache-from: type=gha
          cache-to: type=gha,mode=max
```

- [ ] **Step 2: Validate YAML**

Run: `python3 -c 'import yaml; yaml.safe_load(open(".github/workflows/release.yml"))' && echo OK`
Expected: `OK`

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "ci: add build-remote-image job"
```

---

## Task 13: Add build-relay-image job

**Files:**
- Modify: `.github/workflows/release.yml`

- [ ] **Step 1: Append build-relay-image job**

```yaml
  build-relay-image:
    needs: bump-version
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v6
        with:
          ref: ${{ needs.bump-version.outputs.new_tag }}

      - uses: docker/setup-buildx-action@v3

      - name: Log in to GHCR
        uses: docker/login-action@v3
        with:
          registry: ghcr.io
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}

      - name: Build and push
        uses: docker/build-push-action@v6
        with:
          context: .
          file: crates/relay-tunnel/Dockerfile
          push: true
          tags: |
            ghcr.io/harryy2510/vibe-kanban-relay:latest
            ghcr.io/harryy2510/vibe-kanban-relay:${{ github.sha }}
            ghcr.io/harryy2510/vibe-kanban-relay:${{ needs.bump-version.outputs.new_tag }}
          cache-from: type=gha
          cache-to: type=gha,mode=max
```

- [ ] **Step 2: Validate YAML**

Run: `python3 -c 'import yaml; yaml.safe_load(open(".github/workflows/release.yml"))' && echo OK`
Expected: `OK`

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "ci: add build-relay-image job"
```

---

## Task 14: Add deploy-dokploy job

**Files:**
- Modify: `.github/workflows/release.yml`

- [ ] **Step 1: Append deploy-dokploy job**

```yaml
  deploy-dokploy:
    needs: [build-remote-image, build-relay-image]
    runs-on: ubuntu-latest
    steps:
      - name: Trigger Dokploy redeploy
        run: |
          curl -fsSL -X POST "${{ secrets.DOKPLOY_WEBHOOK_URL }}"
```

- [ ] **Step 2: Validate YAML**

Run: `python3 -c 'import yaml; yaml.safe_load(open(".github/workflows/release.yml"))' && echo OK`
Expected: `OK`

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "ci: add deploy-dokploy webhook trigger"
```

---

## Task 15: Add restart-oxmgr job

**Files:**
- Modify: `.github/workflows/release.yml`

- [ ] **Step 1: Append restart-oxmgr job**

```yaml
  restart-oxmgr:
    needs: publish-npm
    runs-on: ubuntu-latest
    steps:
      - name: Restart oxmgr vibe-kanban app
        uses: appleboy/ssh-action@v1
        with:
          host: ${{ vars.SERVER_SSH_HOST }}
          username: ${{ vars.SERVER_SSH_USER }}
          key: ${{ secrets.SERVER_SSH_KEY }}
          script: |
            source ~/.profile 2>/dev/null || true
            source ~/.bashrc 2>/dev/null || true
            export PATH="$HOME/.bun/bin:$HOME/.local/bin:/usr/local/bin:$PATH"
            oxmgr restart vibe-kanban
```

Note on `PATH`: upstream SSH session earlier did not find `oxmgr`/`bun` — the non-interactive shell skips `.bashrc`/`.zshrc`. The `source`+explicit `PATH` lines above cover common install locations. If oxmgr lives elsewhere, update the `export PATH` line to include that dir.

- [ ] **Step 2: Validate YAML**

Run: `python3 -c 'import yaml; yaml.safe_load(open(".github/workflows/release.yml"))' && echo OK`
Expected: `OK`

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "ci: add restart-oxmgr job after npm publish"
```

---

## Task 16: Merge + verify first run

**Files:**
- None (push + observe)

- [ ] **Step 1: Push branch and open PR**

```bash
git push -u origin HEAD
gh pr create --title "fork: vibepilot.org takeover pipeline" \
  --body "$(cat <<'EOF'
## Summary
- Strip upstream private billing dep
- Rebrand review CLI default URL to vibepilot.org
- Rename npm package to @harryy/vibe-kanban
- Update oxfile.toml for new package + domains
- Delete 8 upstream workflows + update.sh
- Strip SSH agent step from test.yml
- Add single release.yml workflow

## Test plan
- [ ] CI runs on merge to main
- [ ] R2 contains binaries/<tag>/<platform>/*.zip + manifest.json
- [ ] npm view @harryy/vibe-kanban version returns new version
- [ ] ghcr.io/harryy2510/vibe-kanban-{remote,relay}:latest updated
- [ ] Dokploy redeployed containers
- [ ] oxmgr restart succeeded on server
- [ ] bunx -y @harryy/vibe-kanban@latest mcp runs end-to-end
EOF
)"
```

- [ ] **Step 2: After PR review, merge to main**

Run: `gh pr merge --squash --auto`
Expected: PR merges; `release.yml` triggers automatically.

- [ ] **Step 3: Watch the run**

Run: `gh run watch` (select the most recent `Release` run)
Expected: all jobs green, final step `restart-oxmgr` succeeds.

- [ ] **Step 4: Verify R2**

Run: `curl -fsSL ${{ R2_BINARIES_PUBLIC_URL }}/binaries/manifest.json`
Expected: JSON with `"latest": "0.1.45"` (or whatever the bumped version is).

- [ ] **Step 5: Verify npm**

Run: `npm view @harryy/vibe-kanban version`
Expected: prints the new version.

- [ ] **Step 6: Verify GHCR**

Run:
```bash
docker pull ghcr.io/harryy2510/vibe-kanban-remote:latest
docker pull ghcr.io/harryy2510/vibe-kanban-relay:latest
```
Expected: both pull successfully (may need `docker login ghcr.io` first if package is still private; flip to public in GitHub UI if so).

- [ ] **Step 7: Verify consumer works**

SSH to server, run:
```bash
oxmgr status vibe-kanban
```
Expected: status `running`, uptime is recent (within last 2 min = just restarted).

Then from laptop:
```bash
bunx -y @harryy/vibe-kanban@latest mcp </dev/null &
sleep 5
kill %1 2>/dev/null
```
Expected: binary downloads from `binaries.vibepilot.org`, sha256 passes, MCP starts reading stdin.

---

## Task 17: Post-bootstrap — migrate to npm Trusted Publishing

**Files:**
- Modify: `.github/workflows/release.yml` (publish-npm job)

**Do this task only after Task 16 confirms `@harryy/vibe-kanban` exists on npm.**

- [ ] **Step 1: Configure Trusted Publishing on npmjs.com**

On npmjs.com → package `@harryy/vibe-kanban` → Settings → Trusted Publishing → Add GitHub publisher:
- Organization: `harryy2510`
- Repository: `vibe-kanban`
- Workflow: `release.yml`
- Environment: (leave empty)

- [ ] **Step 2: Remove NODE_AUTH_TOKEN from publish step**

In `release.yml`, edit the `publish-npm` job's `Pack and publish` step. Remove the `env:` block:

Before:
```yaml
      - name: Pack and publish
        run: |
          cd npx-cli
          npm pack
          npm publish *.tgz --access public
        env:
          NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }}
```

After:
```yaml
      - name: Pack and publish
        run: |
          cd npx-cli
          npm pack
          npm publish *.tgz --access public --provenance
```

(The `id-token: write` permission at the workflow level is already set; `--provenance` enables OIDC-based publishing.)

- [ ] **Step 3: Push and verify next release still publishes**

```bash
git add .github/workflows/release.yml
git commit -m "ci: migrate npm publish to Trusted Publishing (OIDC)"
git push
gh run watch
```

Expected: release succeeds; `npm view @harryy/vibe-kanban version` shows a new bumped version.

- [ ] **Step 4: Revoke NPM_TOKEN and delete from GitHub secrets**

On npmjs.com → Access Tokens → revoke the `vibe-kanban bootstrap publish` token.

Then:
```bash
gh secret delete NPM_TOKEN --repo harryy2510/vibe-kanban
```

- [ ] **Step 5: Commit (no code change, just documenting)**

```bash
git commit --allow-empty -m "chore: retire bootstrap NPM_TOKEN after trusted publishing migration"
git push
```

---

## Self-review (plan author, after writing)

**Spec coverage:**
- Products 1/2/3: Tasks 9 (CLI matrix) / 12 (remote image) / 13 (relay image) — covered.
- Billing strip: Task 1.
- API URL rebrand: Task 2 + build-args/env in Tasks 10/12 (R2 URL + relay URL).
- npm scope rename: Task 3.
- oxfile update: Task 4.
- Workflow deletion: Task 5.
- test.yml SSH strip: Task 6.
- release.yml creation: Tasks 7-15.
- Dokploy redeploy: Task 14.
- oxmgr restart: Task 15.
- First-run verify: Task 16.
- Trusted Publishing migration: Task 17.

**Known gaps (intentional, scoped out):**
- Frontend marketing URLs in `packages/**` — spec says "low priority"; user hasn't requested them.
- `docs/**` rebrand — spec says docs aren't deployed by pipeline.
- `crates/git/src/lib.rs` default committer email (`noreply@vibekanban.com`) — functionally harmless; a personal preference, not blocking.

**Placeholder scan:** none. All steps have exact code/commands.

**Type consistency:** no code-internal types in this plan; everything is YAML/TOML/JSON with exact strings.
