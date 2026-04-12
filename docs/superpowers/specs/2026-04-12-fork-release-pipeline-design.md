# VibePilot Takeover Pipeline

**Date:** 2026-04-12
**Owner:** harryy2510
**Goal:** Full sovereign fork of vibe-kanban running under `vibepilot.org`. One push to `main` rebuilds and redeploys every component. No upstream dependencies, no manual steps, no billing restrictions.

## Products shipped from this repo

| # | Product | What | Runs | Artifact |
|---|---|---|---|---|
| 1 | **CLI / MCP** | `vibe-kanban`, `vibe-kanban-mcp`, `vibe-kanban-review` | User machine (incl. the Dokploy host over SSH) | npm `@harryy2510/vibe-kanban` + binaries on R2 |
| 2 | **Remote API** | `crates/remote` | Dokploy stack | `ghcr.io/harryy2510/vibe-kanban-remote:latest` |
| 3 | **Relay** | `crates/relay-tunnel` | Dokploy stack | `ghcr.io/harryy2510/vibe-kanban-relay:latest` |

Supporting stack (already deployed via Dokploy, no pipeline changes): postgres, electric, azurite.

## Domain map

| Domain | Points to | Role |
|---|---|---|
| `vibepilot.org` | Dokploy → remote-web frontend (via remote-server) | User-facing web app (login, dashboard) |
| `api.vibepilot.org` | Dokploy → `remote-server:8081` | Remote API consumed by CLI + web |
| `relay.vibepilot.org` | Dokploy → `relay-server:8082` | WebRTC/WS relay |
| `cdn.vibepilot.org` | Dokploy → `azurite:10000` | File attachments (issue uploads) |
| `binaries.vibepilot.org` | Cloudflare R2 custom domain | CLI binary downloads |

## Non-Goals

- **Tauri desktop app.** Dropped. Can be added later as a separate spec.
- **Sentry / PostHog telemetry.** Stripped everywhere.
- **React Virtuoso paid license.** Not used.
- **Billing crate** (`crates/remote` `vk-billing` feature). Never enabled → all features unlocked, no seat limits, no Stripe. The fork's `remote` Dockerfile already strips the dep at build time when `FEATURES=""`.
- **Staging / prod split.** Single environment. One-click deploy to `server.hariom.cc` via Dokploy webhook on every `main` push.
- **Upstream auto-update mirror.** Syncing from upstream happens manually (`git fetch origin`); the pipeline doesn't attempt to track upstream.

## Architecture

```
 push to main
      |
      v
.github/workflows/release.yml  (single workflow)
      |
      +--> job: cli-pipeline
      |        bump version → build 6 platforms × 3 binaries
      |        → upload zips + manifest.json to R2 (binaries.vibepilot.org)
      |        → npm pack with R2_BASE_URL + BINARY_TAG baked
      |        → npm publish @harryy2510/vibe-kanban
      |        → create GitHub release (artifacts for audit)
      |
      +--> job: remote-image
      |        docker build -f crates/remote/Dockerfile (FEATURES="")
      |        → push ghcr.io/harryy2510/vibe-kanban-remote:latest
      |        → push ghcr.io/harryy2510/vibe-kanban-remote:<sha>
      |
      +--> job: relay-image
      |        docker build -f crates/relay-tunnel/Dockerfile
      |        → push ghcr.io/harryy2510/vibe-kanban-relay:latest
      |        → push ghcr.io/harryy2510/vibe-kanban-relay:<sha>
      |
      +--> job: dokploy-redeploy  (needs: remote-image, relay-image)
               curl -X POST $DOKPLOY_WEBHOOK_URL
```

**Path filters on jobs:**
- `cli-pipeline`: runs on changes to `crates/{mcp,server,review,executors,...}/**`, `npx-cli/**`, `packages/local-web/**`, `packages/web-core/**`, `packages/ui/**`, `shared/**`, or manual dispatch. Skipped for pure remote/relay/docs changes.
- `remote-image`: runs on changes to `crates/remote/**`, `packages/remote-web/**`, `packages/{web-core,ui}/**`, `shared/**`, or manual dispatch.
- `relay-image`: runs on changes to `crates/relay-tunnel/**`, `crates/relay-*/**`, or manual dispatch.
- `dokploy-redeploy`: runs iff at least one of `remote-image` or `relay-image` ran successfully.

**Version bumping:** `cli-pipeline` bumps patch version on every run (based on `npm view @harryy2510/vibe-kanban version`). Backend images use `:latest` + `:<git-sha>` tags — no semver bumping needed since Dokploy always pulls `:latest`.

## Code changes required (one-time rebrand)

### Rust / backend

| File | Change |
|---|---|
| `crates/remote/Cargo.toml` | Remove `billing` dep line + `# private crate for billing` comment. Change `vk-billing = ["dep:billing"]` to `vk-billing = []`. (Matches what `crates/remote/Dockerfile` already does via `sed`.) |
| All Rust defaults referencing `https://api.vibekanban.com` | Replace with `https://api.vibepilot.org`. |
| `Cargo.lock` | Regenerate after `billing` dep removal. |

### Frontend

| File | Change |
|---|---|
| Any hardcoded `vibekanban.com` in `packages/{local-web,remote-web,web-core,ui}/**` | Replace with `vibepilot.org` equivalent. |
| `VK_SHARED_API_BASE` default | `https://api.vibepilot.org`. |
| `VITE_RELAY_API_BASE_URL` default | `https://relay.vibepilot.org`. |

### npm package

| File | Change |
|---|---|
| `npx-cli/package.json` → `name` | `@harryy2510/vibe-kanban` |
| `npx-cli/package.json` → `repository.url` | `https://github.com/harryy2510/vibe-kanban` |
| Root `package.json` → `name` | Keep private, doesn't publish |

### Docs / misc

| File | Change |
|---|---|
| `update.sh` | Delete (replaced by workflow). |
| `README.md` | Add a "VibePilot fork" note; leave rest as-is. |
| `docs/**/*.mdx` references to `vibekanban.com` | Update or leave; docs aren't deployed by the pipeline, low priority. |

## Workflows to delete

- `.github/workflows/remote-deploy-dev.yml`
- `.github/workflows/remote-deploy-prod.yml`
- `.github/workflows/remote-release.yml`
- `.github/workflows/relay-deploy-dev.yml`
- `.github/workflows/relay-deploy-prod.yml`
- `.github/workflows/relay-release.yml`
- `.github/workflows/pre-release.yml` (replaced)
- `.github/workflows/publish.yml` (replaced)

## Workflows to keep

- `.github/workflows/test.yml` — PR CI. Strip the `VK_PRIVATE_DEPLOY_KEY` SSH agent step; remaining jobs build without the billing crate.

## Workflows to add

- `.github/workflows/release.yml` — the single workflow described in Architecture.

## GitHub secrets required

| Secret | Value |
|---|---|
| `GITHUB_TOKEN` | Auto-provided; used for GHCR push + GitHub release creation |
| `DEPLOY_KEY` | SSH deploy key (write-access) on fork for pushing version-bump commit back to `main` |
| `R2_BINARIES_ACCESS_KEY_ID` | Cloudflare R2 token access key |
| `R2_BINARIES_SECRET_ACCESS_KEY` | Cloudflare R2 token secret |
| `R2_BINARIES_ENDPOINT` | `https://<account-id>.r2.cloudflarestorage.com` |
| `R2_BINARIES_BUCKET` | e.g. `vibepilot-binaries` |
| `R2_BINARIES_PUBLIC_URL` | `https://binaries.vibepilot.org` |
| `NPM_TOKEN` | npmjs.com Automation token scoped to `@harryy2510` |
| `DOKPLOY_WEBHOOK_URL` | Redeploy webhook for the Dokploy stack |

All upstream secrets removed: `VK_PRIVATE_DEPLOY_KEY`, `SENTRY_*`, `POSTHOG_*`, `APPLE_*`, `AZURE_*` (code-signing, not Azurite), `APP_STORE_API_KEY`, `TAURI_SIGNING_*`, `PUBLIC_REACT_VIRTUOSO_LICENSE_KEY`, `VK_SHARED_API_BASE`, `VK_SHARED_RELAY_API_BASE`, `REMOTE_DEPLOYMENT_TOKEN`.

## Third-party setup (one-time)

### Cloudflare R2

1. Create bucket `vibepilot-binaries`.
2. Attach custom domain `binaries.vibepilot.org` (R2 dashboard → bucket → Settings → Custom Domains). Requires the domain on Cloudflare DNS.
3. Create API token with **Object Read & Write** scoped to that bucket.

### npm

1. Create npm account (if needed).
2. Create org `@harryy2510` (free for public packages).
3. Generate **Automation** access token.

### GHCR

No setup required. The workflow's `GITHUB_TOKEN` can push to `ghcr.io/harryy2510/*`. On first push, the package becomes visible at `github.com/harryy2510?tab=packages`; may need to flip it to **public** manually once so Dokploy can pull without auth.

### Dokploy

1. Enable the stack's redeploy webhook (Dokploy UI → project → Settings → Webhooks).
2. Copy the webhook URL, add as `DOKPLOY_WEBHOOK_URL` secret.

### DNS (all on Cloudflare)

Point each subdomain per the Domain map. TLS via Let's Encrypt is already handled by Dokploy for the `*-server` / `cdn` entries; `binaries.vibepilot.org` is handled by R2's custom domain flow.

## Consumer usage

**CLI on any machine (including `server.hariom.cc`):**
```
bunx -y @harryy2510/vibe-kanban@latest mcp
```

**Web app:** `https://vibepilot.org`

**Backend redeploy:** automatic on `main` push. Manual re-run: GitHub Actions → `release` workflow → Run workflow.

## Error handling

| Failure | Effect | Recovery |
|---|---|---|
| CLI build fails on a platform | No release, no npm publish | Fix, push again; workflow auto-retries from bumped version |
| R2 upload fails | `.tgz` never baked with `BINARY_TAG` → no broken npm version | Re-run workflow |
| `npm publish` fails (409 duplicate) | Version collision | Push any commit → auto-bumps → retries |
| GHCR push fails | No new image tag | Re-run workflow |
| Dokploy redeploy webhook fails | Image is pushed but not deployed | Hit redeploy in Dokploy UI manually |

## Testing

1. Merge PR with code changes + new workflow. First `main` push triggers full pipeline.
2. Verify R2 contains `binaries/v<version>-<ts>/linux-x64/vibe-kanban-mcp.zip`.
3. Verify npm: `npm view @harryy2510/vibe-kanban version`.
4. Verify GHCR: `docker pull ghcr.io/harryy2510/vibe-kanban-remote:latest`.
5. Verify Dokploy pulled the new image (check `docker inspect` timestamp on the host).
6. From `server.hariom.cc`: `bunx -y @harryy2510/vibe-kanban@latest mcp` — MCP binary downloads, sha256 verifies, binary runs and exposes new tool endpoints.
7. From browser: log in at `https://vibepilot.org`, confirm all features work (billing is disabled, so no paywall).

## Bootstrap order

1. Create R2 bucket + API token + custom domain `binaries.vibepilot.org`.
2. Create npm org `@harryy2510` + Automation token.
3. Configure Cloudflare DNS for all `vibepilot.org` subdomains (point at Dokploy LB / R2).
4. Update Dokploy stack `.env` with new domains (replace `*.hariom.cc` → `*.vibepilot.org`); enable redeploy webhook; update OAuth redirect URIs for new domain.
5. Update GitHub OAuth App + Google OAuth consent screen with new redirect URIs (`https://api.vibepilot.org/...`).
6. Add all 7 non-`GITHUB_TOKEN` secrets to the fork's Actions settings.
7. Generate + add `DEPLOY_KEY` to fork repo.
8. Merge the rebrand + workflow PR to `main`. First run kicks off the full pipeline.
9. Once GHCR images exist, flip the two packages to **public** (one-time UI toggle on github.com).
10. Once npm publish succeeds, update CLI consumers (any `bunx vibe-kanban@...` call sites).
