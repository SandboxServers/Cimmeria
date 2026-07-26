---
title: "Container distribution"
type: how-to
audience: operators
last_updated: 2026-07-25
---

# Container distribution

Self-contained Docker image: server binary + cooked game data + pre-loaded Postgres + s6-overlay supervisor, in one published artifact. The goal is `docker run` and you have a working Cimmeria server — no Rust toolchain, no MSVC, no PowerShell bootstrap, no `.sql` files to load.

> Looking to deploy this to a colo / single-host box with auto-update on every new release? See [Colo Auto-Update Deployment](colo-deploy.md) and the self-contained compose at [`docker/compose.yml`](../../docker/compose.yml) (single file — includes the SigNoz observability stack inlined).

## Release model

Releases are **explicit**. Pushing to `main` does not publish a container — many PRs land for routine work that isn't release-worthy. To cut a release, you trigger [`.github/workflows/release-container.yml`](../../.github/workflows/release-container.yml) one of three ways:

1. **Comment `/release` on a merged PR.** [`release-on-comment.yml`](../../.github/workflows/release-on-comment.yml) validates the commenter has repo write permission, confirms the PR is merged, and dispatches the release workflow. The PR you comment on is just the cut-point marker — the build always uses `main` HEAD, so this captures every PR that's been merged up to that point. Reactions on your comment indicate progress: 👀 received, 🚀 dispatched, 👎 rejected.
2. **Manual UI dispatch.** Open **Actions → `release-container`**, click "Run workflow", pick `main`. Useful when you decided post-merge that something is ship-worthy and don't want to leave a comment trail.
3. **CLI dispatch.** `gh workflow run release-container.yml --ref main`.

PRs that touch the container release surface still get built and smoke-tested by [`pr-container.yml`](../../.github/workflows/pr-container.yml) (path-filtered to crates / docker / db / `.cargo` / etc.) — the gate is on *publishing*, not on *catching breakage*. PR-level breakage is caught pre-merge regardless of release intent.

## Pull

```bash
# Latest pre-release pointer (rolling)
docker pull ghcr.io/sandboxservers/cimmeria-server:latest-prerelease

# Specific dated build (immutable)
docker pull ghcr.io/sandboxservers/cimmeria-server:2026-05-09.1
```

Versions are `YYYY-MM-DD.N` where `N` is the 1-based release count for that UTC calendar day.

## Run

```bash
docker run -d --name cimmeria \
  -p 13001:13001 \
  -p 32832:32832/udp \
  -p 50000:50000/udp \
  -p 8081:8081 \
  -p 8443:8443 \
  -p 30000:30000 \
  -e BASE_EXTERNAL=<your-LAN-or-WAN-ip> \
  ghcr.io/sandboxservers/cimmeria-server:latest-prerelease
```

No volume mount for the database — the container reseeds pgdata from the image on every start, so mounting a volume there preserves nothing. See [Volume / persistence](#volume--persistence).

| Port | Protocol | Purpose |
|---|---|---|
| 13001 | TCP | Auth (BaseApp connections) |
| 32832 | UDP | BaseApp |
| 50000 | UDP | CellApp |
| 8081  | TCP | Auth HTTP (SOAP login) |
| 8443  | TCP | Admin REST API |
| 30000 | TCP | Minigame SmartFoxServer (Livewire, Hack, Bypass, GoauldCrystals, Alignment, Activate, Analyze, Converse) |

> The legacy C++ BaseApp had a Python console on port 8989; the Rust server does not implement it, so the container does not expose it.

## Environment

Every variable that [`crates/server/src/main.rs`](../../crates/server/src/main.rs) reads is overridable at `docker run` time. The image bakes defaults for the subset below (see the `ENV` block at [`docker/Dockerfile:254-268`](../../docker/Dockerfile)); the full catalogue, including anything not listed here, is the env-var table in `main.rs`'s module header.

| Variable | Default | Notes |
|---|---|---|
| `AUTH_HOST` | `0.0.0.0` | Auth bind address |
| `AUTH_PORT` | `13001` | |
| `LOGON_PORT` | `8081` | |
| `BASE_HOST` | `0.0.0.0` | |
| `BASE_EXTERNAL` | `127.0.0.1` | **Must override for non-localhost clients.** |
| `BASE_PORT` | `32832` | |
| `CELL_PORT` | `50000` | |
| `ADMIN_PORT` | `8443` | |
| `DB_URL` | `host=127.0.0.1 port=5432 user=w-testing password=w-testing dbname=sgw` | Libpq-style — see note below. Note this differs from the non-container default (`port=5433`). |
| `DEVELOPER_MODE` | `true` | Relaxed auth + multi-login |
| `RUST_LOG` | `info` | tracing-subscriber filter |

Not baked into the image, but read by the server and worth setting on a real deployment:

| Variable | Default | Notes |
|---|---|---|
| `PROTOCOL_DIGEST` | (compiled-in) | 32-char hex digest sent in the auth response — only override if you're testing protocol changes |
| `MERCURY_ENCRYPTION_VERSION` | `1` | Server-wide Mercury wire-encryption version. `1` = legacy (the only version unpatched clients understand); `2` = modernized. No per-client negotiation. |
| `AUTH_TLS_RELOAD_INTERVAL_SECS` | `30` | Poll interval for hot-reloading the auth TLS cert/key on change (e.g. a Let's Encrypt renewal). `0` disables. Only active when the TLS listener is configured. |
| `CIMMERIA_DEPLOY_ENV` | `dev` | Sets `deployment.environment` on every span/log/metric. Set to `colo` on a colo box so its data doesn't mix with dev-laptop noise. |
| `OTEL_EXPORTER_OTLP_ENDPOINT` | (unset) | OTLP collector endpoint. Unset ⇒ exporter disabled. [`docker/compose.yml`](../../docker/compose.yml) sets this to the bundled SigNoz collector. See [signoz-deployment.md](signoz-deployment.md). |
| `OTEL_EXPORTER_OTLP_PROTOCOL` | `grpc` | `grpc` or `http/protobuf`. |
| `OTEL_SERVICE_NAME` | `cimmeria-server` | `service.name` in SigNoz's service map. |
| `OTEL_RESOURCE_ATTRIBUTES` | (unset) | Comma-separated `k=v` resource attributes. |
| `OTEL_TRACES_SAMPLER` | `always_on` | `always_on`, `always_off`, or `traceidratio` with `OTEL_TRACES_SAMPLER_ARG`. |
| `CIMMERIA_TELEMETRY_HMAC_SECRET` | (unset) | Secret for the launcher dev-session token mint. Unset ⇒ the endpoint returns 500. See [telemetry.md](telemetry.md). |
| `CIMMERIA_TELEMETRY_UPLOAD_ENDPOINT` | `http://localhost:8443/api/telemetry` | Upload URL handed back to the launcher. **Must** be overridden when the launcher and server are on different hosts. |
| `CIMMERIA_TELEMETRY_KILL_SWITCH` | (unset) | Set to the literal `1` to pause telemetry ingest. |

> `DB_URL` must be in libpq key-value form, not URL DSN form. `crates/services/src/orchestrator_postgres.rs::ensure_postgresql_running` parses `host=` / `port=` tokens to decide whether to auto-start the bundled Postgres. A URL like `postgres://...` would silently fall back to `localhost:5433` and emit warnings, even though sqlx itself accepts either form.

## Volume / persistence

> **The database in this image is ephemeral. There is currently no way to persist it across container starts.**

The DB lives at `/var/lib/postgresql/data` (declared as a `VOLUME`), but the image's [entrypoint](../../docker/entrypoint.sh) **unconditionally reseeds** that path from the baked fallback at `/var/lib/postgresql/initial-data` on *every* container start, before Postgres boots. It clears the directory and copies the pristine layer over it — there is no "only if empty" guard.

That applies regardless of how the path is mounted: anonymous volume, named volume, or bind mount. A named volume will survive as a Docker object, but its contents are overwritten on each start, so it buys you nothing. A plain `docker restart cimmeria` — no image swap involved — also wipes the database.

This is deliberate. The design intent is "ephemeral DB, fresh every deploy" while the schema is still churning, and the reseed lives in the entrypoint because the alternative (relying on `WATCHTOWER_REMOVE_VOLUMES=true` to drop the anonymous volume) does not actually work — watchtower attaches the old volume to the new container before removing the old one, so Docker refuses the deletion and the stale database survives. Observed on the colo on 2026-05-26. The entrypoint's header comment documents the full mechanic.

If you need a database that outlives a restart, run Postgres outside this image and point `DB_URL` at it — see [colo-deploy.md → When to graduate off this setup](colo-deploy.md#when-to-graduate-off-this-setup).

### Reset to a fresh server

Any restart already gives you a fresh server. To also discard the container and image:

```bash
docker rm -f cimmeria
docker run ...   # same flags as before — the entrypoint reseeds pgdata on start
```

## What's inside

- `cimmeria-server` (Linux x86_64), built `cargo build -p cimmeria-server --release`.
- `data/cache/*.pak` — cooked game data (22 files, required at runtime).
- `data/spaces/*.nav` — navmeshes (5 files, optional but shipped — without them combat AI / line-of-sight degrades).
- Postgres 17.9 + pgdata pre-loaded from `db/database.sql` (schema + seeds, `\ir`'d from `db/sgw/` and `db/resources/`).
- s6-overlay v3 supervising postgres + cimmeria-server. Postgres starts first, server waits for `pg_isready`, server death brings the container down so the orchestrator restarts it cleanly.

## What's NOT inside

- No Rust source, no `target/`, no build tooling.
- No `.sql` files. The DB is *identity-baked* — schema + seed data are part of the image's tag, not lazily applied state.
- No `data/scripts/`, `entities/defs/`, `external/`, or `tools/`. Those are compile-time inputs; the runtime never reads them.

## Image architecture (Dockerfile stages)

1. **builder** — `rust:1-bookworm`, hydrates `external/recast`, `cargo build -p cimmeria-server --release`.
2. **db-init** — `postgres:17.9-bookworm`, `initdb`, loads `db/database.sql`, captures pgdata.
3. **runtime** — `postgres:17.9-bookworm` + s6-overlay v3, copies the binary + cooked data + baked pgdata.

Source: [`docker/Dockerfile`](../../docker/Dockerfile). Stage names are stable and used as build-cache keys.

## Healthcheck

`HEALTHCHECK` checks two things:
1. `pg_isready` against the bundled Postgres.
2. A TCP probe (`bash -c 'exec 3<>/dev/tcp/127.0.0.1/$AUTH_PORT'`) confirming the auth listener is bound — this is the port the game client hits.

Admin API is ignored by the healthcheck on purpose; it isn't in the critical login path.

## Retention

The published GHCR package keeps the last 14 dated tags plus rolling pointers (`latest-prerelease`, future `latest`/semver). Pruning runs weekly via [`.github/workflows/prune-container.yml`](../../.github/workflows/prune-container.yml).

## Security

### Threat model & deliberate trade-offs

The image is designed for "just works" demo / LAN play, not for production hosting. Two design choices flow from that:

- **Two services per container.** Postgres and the game server share an image so `docker run` is a single command. The conventional one-process-per-container rule trades that UX for orchestrator-friendliness; we made the opposite call. s6-overlay v3 supervises both processes and propagates signals correctly, so it's not a stability issue, but blast radius is wider than two separate containers would be.
- **Baked Postgres credentials.** `POSTGRES_USER=w-testing` / `POSTGRES_PASSWORD=w-testing` are baked into pgdata at build time and visible via `docker history`. This is acceptable for a self-contained demo image where Postgres is bound to `127.0.0.1` inside the container and never exposed externally — but it is **not** a production secret-management story. Production deployments should run an external Postgres, set `DB_URL` to point at it, and source the password from Docker secrets / a secrets manager (Vault, SOPS, etc.).

### Required hardening at `docker run` time

The image runs the server as the unprivileged `cimmeria` user (UID 1001) and Postgres as `postgres` (from the base image). Layer the standard runtime hardening flags on top:

```bash
docker run -d --name cimmeria \
  --read-only \
  --tmpfs /run --tmpfs /tmp --tmpfs /var/run \
  --cap-drop=ALL \
  --cap-add=CHOWN --cap-add=SETUID --cap-add=SETGID --cap-add=DAC_OVERRIDE --cap-add=FOWNER \
  --security-opt=no-new-privileges \
  -p 13001:13001 -p 32832:32832/udp -p 50000:50000/udp \
  -p 8081:8081 -p 8443:8443 -p 30000:30000 \
  -e BASE_EXTERNAL=<your-LAN-or-WAN-ip> \
  ghcr.io/sandboxservers/cimmeria-server:latest-prerelease
```

| Flag | Why |
|---|---|
| `--read-only` | Root filesystem is immutable. Stops a compromised process from persisting changes to the image content. The `VOLUME`-declared pgdata and log paths are the only writable locations, by design. |
| `--tmpfs /run /tmp /var/run` | s6-overlay and Postgres both write status / sockets / lock files to these paths. Tmpfs satisfies them without dropping `--read-only`. |
| `--cap-drop=ALL` + selective `--cap-add` | Default Docker capabilities are over-broad. The added five are what s6-overlay's `s6-setuidgid` privilege drops and Postgres's directory-perm enforcement actually need. Anything else is excess. |
| `--security-opt=no-new-privileges` | Hard kernel-level block on `setuid` / `setcap` escalation paths. Belt to the cap-drop suspenders. |

### Image supply-chain posture

- **Base images pinned by SHA digest** (`postgres:17.9-bookworm@sha256:...`, `rust:1-bookworm@sha256:...`). Tag-only references are vulnerable to tag mutation; the digest is the integrity anchor. Dependabot ([`.github/dependabot.yml`](../../.github/dependabot.yml)) opens weekly PRs to bump the digests when upstream rebuilds.
- **Third-party downloads SHA256-verified.** `external/recast` and the `s6-overlay` tarballs are checksum-checked before extraction; bumping the version ARG without bumping its companion SHA ARG is a loud build failure, not a silent acceptance of a different file.
- **CVE scanning on every build.** Trivy runs against the locally-built image before any `docker push` in [`release-container.yml`](../../.github/workflows/release-container.yml) and on every release-relevant PR via [`pr-container.yml`](../../.github/workflows/pr-container.yml). Severity threshold is CRITICAL, fixable-only — the goal is to block known-exploited CVEs, not chase every theoretical issue.
- **OCI provenance + SBOM attestations** generated by buildx (`provenance: true, sbom: true` in the workflow). Enables downstream verification via `cosign verify-attestation` once we set up signing.
- **Network exposure caveat.** Same as the standalone server — do not expose to the public internet without a trusted client base. The legacy Stargate Worlds protocol the client speaks is not designed to withstand hostile peers. Run on a LAN or a trusted VPN.
