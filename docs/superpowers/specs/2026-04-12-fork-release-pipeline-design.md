# Fork Release Pipeline Design

**Date:** 2026-04-12
**Owner:** harryy2510
**Goal:** End-to-end CI pipeline on `harryy2510/vibe-kanban` fork that builds, packages, and publishes vibe-kanban binaries (including MCP) so `bunx @harryy2510/vibe-kanban mcp` works forever on `server.hariom.cc` and any other machine.

## Context

Upstream `vibe-kanban` ships via a two-workflow pipeline:

1. `pre-release.yml` -- manual dispatch, builds 6 platforms x 3 binaries, uploads zipped binaries to Cloudflare R2, attaches `.tgz` npm wrapper to a GitHub **prerelease**.
2. `publish.yml` -- fires when the prerelease is promoted to a full release (human clicks "publish" in GitHub UI), runs `npm publish`.

The npx wrapper (`npx-cli/`) is published to npm. At install time it reads a baked-in `R2_BASE_URL` plus `BINARY_TAG`, downloads the platform-specific binary zip, verifies sha256, unzips into `~/.vibe-kanban/bin/`, and executes.

The fork cannot publish under the `vibe-kanban` name on npm and cannot fetch from upstream's R2 bucket. It also cannot access `BloopAI/vibe-kanban-private` (billing crate dependency).

## Non-Goals

- **Desktop app (Tauri).** Dropped entirely from the fork pipeline. Apple notarization, Windows code signing, and the associated build matrix add ~600 lines of workflow and recurring third-party costs with zero overlap with MCP delivery. Can be added later as its own spec.
- **Sentry / PostHog telemetry.** Not wired up on the fork. Release uploads, source maps, and DSN injection are removed.
- **React Virtuoso license key.** Frontend builds without the paid key (reverts to trial/free rendering).
- **Remote deployment, relay-tunnel release workflows.** Out of scope -- those are separate products that don't block MCP. `.github/workflows/{remote-deploy-*,remote-release,relay-*}.yml` are left untouched; they fail silently on the fork because they require `VK_PRIVATE_DEPLOY_KEY` but are never auto-triggered on `main` push.
- **`test.yml` fork behavior.** The SSH agent step in `test.yml` (line 246) is guarded by `if: github.event.pull_request.head.repo.full_name == github.repository`, so it no-ops on external PRs but **still runs on fork-internal pushes**. If the fork's `test.yml` runs on `main` and hits this step without `VK_PRIVATE_DEPLOY_KEY`, it will fail at SSH setup. Mitigation: either add the same guard to `test.yml` on the fork, or remove the SSH agent step (the MCP/server binaries don't need the billing crate).

## Architecture

Mirror upstream's two-workflow split, trimmed to the essentials:

```
.github/workflows/
  pre-release.yml   # auto on push to main + workflow_dispatch
  publish.yml       # auto on GitHub release published + workflow_dispatch
```

### Flow

```
 push to main          workflow_dispatch           tag push v*
       |                     |                          |
       v                     v                          v
  pre-release.yml  <----- pre-release.yml        (not used; npm publish
       |                                          triggers on release
       v                                          event only)
  - bump version
  - build frontend
  - build 6x Rust targets (server, mcp, review)
  - package npx-cli/dist/<platform>/*.zip
  - upload zips + manifest.json to R2
  - npm pack with R2_BASE_URL and BINARY_TAG baked in
  - create GitHub prerelease, attach .tgz
       |
       v
  (human promotes prerelease -> release in GitHub UI,
   OR pushes a tag, OR clicks workflow_dispatch on publish.yml)
       |
       v
  publish.yml
  - download .tgz asset from release
  - npm publish @harryy2510/vibe-kanban (--access public)
```

### Trigger model decisions

- **Pre-release auto-runs on push to `main`** -- every merged PR produces a tested artifact + GitHub prerelease, but does not hit npm.
- **Pre-release also runs on `workflow_dispatch`** -- for manual re-runs (e.g., retrying after a flaky build).
- **Publish runs on GitHub `release: published` event** -- promoting a prerelease to a full release pushes to npm. Also `workflow_dispatch` for manual publish.
- **No auto-publish on tag push.** Tag pushes from `pre-release.yml`'s version bump create prereleases, not full releases. Keeping publish gated on the "published" event prevents accidental npm releases.

The `autopilot` branch (current default) does **not** trigger pre-release. Only `main`.

## Components

### 1. R2 bucket

- **Provider:** Cloudflare R2.
- **Bucket name:** `vibe-kanban-binaries-fork` (or user's choice; referenced only by secrets).
- **Public access:** enabled via R2.dev subdomain OR custom domain (e.g., `binaries.hariom.cc`). Public read required; npx wrapper fetches manifest + zips over plain HTTPS GET.
- **Layout** (mirrors upstream):
  ```
  binaries/manifest.json                  # { "latest": "0.1.45" }
  binaries/<tag>/manifest.json            # per-version sha256 + size per platform/binary
  binaries/<tag>/<platform>/<binary>.zip  # actual binaries, e.g. linux-x64/vibe-kanban-mcp.zip
  ```
- **API token:** single token, Object Read & Write, scoped to the bucket. Stored in GitHub secrets.

### 2. npm scope

- **Package name:** `@harryy2510/vibe-kanban` (public scoped package; free).
- **Org:** `@harryy2510` npm organization (must be created on npmjs.com).
- **Auth:** npm Automation access token stored as `NPM_TOKEN` secret.
- **Provenance:** disabled (requires OIDC + public source repo; not worth the setup for fork scope).

### 3. GitHub Actions workflows

Two workflows, derived from upstream's `pre-release.yml` and `publish.yml` with the trimming described in **Non-Goals** above.

### 4. npx wrapper (`npx-cli/`)

- `package.json` rename: `"name": "vibe-kanban"` -> `"name": "@harryy2510/vibe-kanban"`.
- `repository.url`: point at fork.
- No code changes to `src/cli.ts` or `src/download.ts`; they already parameterize on `R2_BASE_URL` and `BINARY_TAG` placeholders.

### 5. Cargo workspace

- **Billing crate dependency** (`crates/remote/Cargo.toml` line 17 -- `billing = { git = "ssh://.../BloopAI/vibe-kanban-private", optional = true }`) stays in `Cargo.toml` untouched, but no build target enables the `billing` feature. SSH agent step is removed from the fork's workflow. The `remote` crate is itself optional and not built by the `pre-release.yml` binary matrix.
- **Verification:** the existing build matrix already compiles `vibe-kanban`, `vibe-kanban-mcp`, `vibe-kanban-review` without the billing feature. The SSH agent step exists solely for `remote-release.yml` / `remote-deploy-*` workflows, which are out of scope.

## Required secrets (fork `Settings -> Secrets and variables -> Actions`)

| Secret | Required? | Source |
|---|---|---|
| `GITHUB_TOKEN` | auto | Provided by Actions runtime |
| `R2_BINARIES_ACCESS_KEY_ID` | yes | Cloudflare R2 API token (access key) |
| `R2_BINARIES_SECRET_ACCESS_KEY` | yes | Cloudflare R2 API token (secret) |
| `R2_BINARIES_ENDPOINT` | yes | `https://<account-id>.r2.cloudflarestorage.com` |
| `R2_BINARIES_BUCKET` | yes | bucket name, e.g. `vibe-kanban-binaries-fork` |
| `R2_BINARIES_PUBLIC_URL` | yes | public read URL, e.g. `https://pub-<hash>.r2.dev` or `https://binaries.hariom.cc` |
| `NPM_TOKEN` | yes | npmjs.com Automation token, scoped to `@harryy2510` |
| `DEPLOY_KEY` | yes | SSH deploy key on fork with **write** access (used by `pre-release.yml` to push the version-bump commit + tag back to `main`) |

All other upstream secrets (`VK_PRIVATE_DEPLOY_KEY`, `SENTRY_*`, `POSTHOG_*`, `APPLE_*`, `AZURE_*`, `APP_STORE_API_KEY`, `TAURI_SIGNING_*`, `PUBLIC_REACT_VIRTUOSO_LICENSE_KEY`, `VK_SHARED_API_BASE`, `VK_SHARED_RELAY_API_BASE`) are **not required** and their corresponding workflow steps/jobs are stripped.

## Version bumping

Upstream's version logic reads `npm view vibe-kanban version` to avoid downgrades. Fork replaces this with `npm view @harryy2510/vibe-kanban version` (returns `0.0.0` on first run before the package exists). All three `package.json` files stay in lock-step: root `package.json`, `npx-cli/package.json`, `packages/local-web/package.json`.

**Tag format:** unchanged -- `v<version>-<timestamp>` for regular releases, `v<version>.<timestamp>` for prereleases with branch suffix. The timestamp suffix allows multiple pre-releases per version to coexist in R2.

## Consumer usage

On `server.hariom.cc`:

```bash
bunx -y @harryy2510/vibe-kanban@latest mcp
```

The wrapper reads the baked-in `R2_BASE_URL` (pointing at the fork's R2 bucket) + `BINARY_TAG` (the tag of that npm version), downloads the linux-x64 MCP binary zip, verifies sha256, caches to `~/.vibe-kanban/bin/`, and execs it.

## Error handling

- **Build failure on any of the 6 platforms:** job fails, no release is created, no npm publish triggered. The version bump commit still gets pushed to `main`, so the next pipeline run starts from the bumped version (no manual cleanup needed).
- **R2 upload failure:** release is not created; `.tgz` never gets baked with `BINARY_TAG`, so no broken package can reach npm.
- **npm publish failure:** GitHub release is already created and R2 zips are already uploaded. Re-run via `workflow_dispatch` on `publish.yml` after fixing the issue.
- **Duplicate version on npm:** npm rejects with 403. Bump version (workflow does this automatically on next `main` push) and retry.

## Testing

- **Dry run:** run `pre-release.yml` via `workflow_dispatch` on a scratch branch first; confirm R2 zips appear and `.tgz` artifact is attached to the prerelease.
- **End-to-end:** manually `bunx @harryy2510/vibe-kanban@<version> mcp` on a linux machine; confirm MCP binary downloads, sha256 verifies, binary runs.
- **Server integration:** replace `server.hariom.cc` command with the fork package and verify MCP tool discovery works from the client side.

## Bootstrap order (first-time setup)

1. Create R2 bucket + API token + public URL.
2. Create npm org `@harryy2510` + Automation token.
3. Add all 7 non-`GITHUB_TOKEN` secrets to fork's Actions settings.
4. Generate fork SSH deploy key, add to fork repo with write access, add private key as `DEPLOY_KEY` secret.
5. Merge PR that:
   - Renames `npx-cli/package.json` name + repository URL.
   - Adds trimmed `pre-release.yml` + `publish.yml` on the fork.
   - Removes `.github/workflows/{relay-*,remote-*,test.yml}` references to `VK_PRIVATE_DEPLOY_KEY` if any conflict (likely untouched).
6. Push to `main`. First pipeline run publishes `@harryy2510/vibe-kanban@0.1.45` (or whatever `patch` bump from current).
7. Update `server.hariom.cc` consumer command to `bunx -y @harryy2510/vibe-kanban@latest mcp`.

## Open questions

None at spec time. The following are **decisions** made during brainstorming:

- Tauri desktop: dropped.
- Trigger: auto on `main` push + dispatch; publish on release-published event + dispatch.
- Distribution: R2 (mirror upstream).
- Consumer: scoped npm package.
- Platforms: all 6 (linux/macOS/windows x x64/arm64).
