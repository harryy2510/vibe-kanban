# VibePilot Fork

This is a sovereign fork of [`BloopAI/vibe-kanban`](https://github.com/BloopAI/vibe-kanban) maintained at [`harryy2510/vibe-kanban`](https://github.com/harryy2510/vibe-kanban), rebranded as **VibePilot** and hosted at `vibepilot.org`.

## Products shipped

| Product | Artifact | Consumer |
| --- | --- | --- |
| CLI + MCP (`vibe-kanban`, `vibe-kanban-mcp`, `vibe-kanban-review`) | npm `@harryy/vibe-kanban` + binaries on `binaries.vibepilot.org` | `bunx -y @harryy/vibe-kanban@latest [subcommand]` |
| Remote API | `ghcr.io/harryy2510/vibe-kanban/remote:latest` | Dokploy stack on `api.vibepilot.org` |
| Relay tunnel | `ghcr.io/harryy2510/vibe-kanban/relay:latest` | Dokploy stack on `relay.vibepilot.org` |

Full architecture and rebrand plan: [`docs/superpowers/specs/2026-04-12-fork-release-pipeline-design.md`](docs/superpowers/specs/2026-04-12-fork-release-pipeline-design.md).

## Release pipeline

Two workflows on every push to `main`:

```
  push to main
        |
        v
  bump.yml  (detects which components changed)
  - dorny/paths-filter computes cli/remote/relay flags
  - if no component changed -> skip bump, no release
  - else bump patch version, commit [skip ci], tag, push
  - dispatches release.yml with component flags
        |
        v
  release.yml  (runs only the jobs whose component changed)
  - meta: derive tag + component flags
  - build-frontend + build-cli (5 platforms) + upload-r2 + publish-npm   [cli]
  - build-remote-image                                                   [remote]
  - build-relay-image                                                    [relay]
  - deploy-dokploy (runs if remote OR relay rebuilt)
  - restart-oxmgr (runs if cli rebuilt and npm publish succeeded)
```

**Manual re-release:** GitHub Actions -> Release -> Run workflow -> enter `tag` and (optionally) toggle `cli` / `remote` / `relay` booleans.

### Path filter map

| Change touches | Triggers |
| --- | --- |
| `crates/{server,mcp,review,executors,services,db,git,git-host,worktree-manager,workspace-manager,local-deployment,deployment,embedded-ssh,preview-proxy,desktop-bridge}/**`, `packages/{local-web,public}/**`, `npx-cli/**` | CLI |
| `crates/remote/**`, `packages/remote-web/**` | Remote image |
| `crates/relay-{client,control,hosts,protocol,tunnel,tunnel-core,types,webrtc,ws}/**`, `crates/ws-bridge/**` | Relay image |
| `Cargo.toml`, `Cargo.lock`, `crates/{api-types,client-info,server-info,utils,trusted-key-auth}/**`, `packages/{ui,web-core}/**`, `shared/**`, `.github/workflows/{release,bump}.yml` | ALL (shared) |
| `docs/**`, `README.md`, `.gitignore`, `FORK.md` | Nothing (no release triggered) |

## Fork deltas from upstream

When pulling changes from `origin/main` (BloopAI), expect conflicts in these files. They are intentional fork-specific modifications.

### Permanent deltas (always preserve fork version)

| File | Why |
| --- | --- |
| `crates/remote/Cargo.toml` | `billing` private git dep stripped; `vk-billing` feature left empty. Upstream references `ssh://git@github.com/BloopAI/vibe-kanban-private` which the fork can't access. |
| `crates/review/src/main.rs` (`DEFAULT_API_URL`) | Points to `https://api.vibepilot.org` instead of upstream `https://api.vibekanban.com`. |
| `npx-cli/package.json` | Package renamed to `@harryy/vibe-kanban`; author, repo URL, description updated. |
| `.github/workflows/release.yml` | Our release pipeline (new). |
| `.github/workflows/bump.yml` | Our bump pipeline (new). |
| `.github/workflows/` deleted: `pre-release.yml`, `publish.yml`, `relay-deploy-*.yml`, `relay-release.yml`, `remote-deploy-*.yml`, `remote-release.yml`, `test.yml` | Upstream release/deploy/test infra not applicable; some require paid runners. |
| `FORK.md` | This file. |

### Intentionally not forked (marketing/docs)

- `docs/**/*.mdx` still reference `vibekanban.com` marketing URLs. Low priority; not blocking.
- `README.md` still upstream-flavored.
- Frontend marketing links in `packages/**` still reference upstream URLs.

If upstream updates these, take their version without hesitation.

## Pulling upstream changes

```bash
git fetch origin main
git merge origin/main
# resolve conflicts per the "Permanent deltas" table above
git push fork main
```

After merge, bump.yml triggers automatically (unless the merge only touched docs/README/gitignore).

## GitHub Actions secrets & variables

**Variables** (non-sensitive, visible in logs):

| Name | Example |
| --- | --- |
| `R2_BINARIES_ENDPOINT` | `https://<account>.r2.cloudflarestorage.com` |
| `R2_BINARIES_BUCKET` | `vibepilot-binaries` |
| `R2_BINARIES_PUBLIC_URL` | `https://binaries.vibepilot.org` |
| `SERVER_SSH_HOST` | `server.hariom.cc` |
| `SERVER_SSH_USER` | `ubuntu` |

**Secrets** (sensitive):

| Name | Purpose |
| --- | --- |
| `GITHUB_TOKEN` | auto-provided; push, GHCR, workflow dispatch |
| `DEPLOY_KEY` | SSH private key for CI push to main + tag |
| `R2_BINARIES_ACCESS_KEY_ID` / `R2_BINARIES_SECRET_ACCESS_KEY` | R2 API token |
| `NPM_TOKEN` | npm Automation token for `@harryy` scope (retire once Trusted Publishing is set up) |
| `DOKPLOY_WEBHOOK_URL` | Redeploy trigger |
| `SERVER_SSH_KEY` | SSH private key for `oxmgr restart vibe-kanban` |

## Consumer surfaces

**MCP / local server (any machine including the Dokploy host):**
```bash
bunx -y @harryy/vibe-kanban@latest        # local UI + server (port 4040 by default)
bunx -y @harryy/vibe-kanban@latest mcp    # MCP stdio server for AI agents
bunx -y @harryy/vibe-kanban@latest review # PR review CLI
```

**Web app:** `https://vibepilot.org`

**Dokploy redeploy:** automatic on `main` push. Manual: hit Dokploy UI redeploy.

## Related repos / directories

- `../vibe-pilot` - autopilot orchestrator + Dokploy `docker-compose.yaml` + `oxfile.toml` (deployment config lives there, not here).
- `harryy2510/git-sync` - reference repo for the GHCR publishing pattern this fork copied.
