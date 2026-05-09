# Container distribution

Self-contained Docker image: server binary + cooked game data + pre-loaded Postgres + s6-overlay supervisor, in one published artifact. The goal is `docker run` and you have a working Cimmeria server — no Rust toolchain, no MSVC, no PowerShell bootstrap, no `.sql` files to load.

Built and published by [`.github/workflows/release-container.yml`](../../.github/workflows/release-container.yml) on every push to `main` that changes code or runtime data; doc-only changes do not trigger a build.

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
  -p 8989:8989 \
  -e BASE_EXTERNAL=<your-LAN-or-WAN-ip> \
  -v cimmeria-data:/var/lib/postgresql/data \
  ghcr.io/sandboxservers/cimmeria-server:latest-prerelease
```

| Port | Protocol | Purpose |
|---|---|---|
| 13001 | TCP | Auth (BaseApp connections) |
| 32832 | UDP | BaseApp |
| 50000 | UDP | CellApp |
| 8081  | TCP | Auth HTTP (SOAP login) |
| 8443  | TCP | Admin REST API |
| 8989  | TCP | Python console (legacy) |

## Environment

Every variable that [`crates/server/src/main.rs`](../../crates/server/src/main.rs) reads is overridable at `docker run` time. The container ships with these defaults:

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
| `DB_URL` | `postgres://w-testing:w-testing@127.0.0.1:5432/sgw` | Bundled Postgres |
| `DEVELOPER_MODE` | `true` | Relaxed auth + multi-login |
| `RUST_LOG` | `info` | tracing-subscriber filter |

## Volume / persistence

The DB lives at `/var/lib/postgresql/data` (declared as a `VOLUME`). Two patterns:

**Named volume — recommended.** Docker auto-copies the baked pgdata into the volume on first run; subsequent runs use the persistent volume. DB survives container recreation.

```bash
-v cimmeria-data:/var/lib/postgresql/data
```

**Bind mount.** Docker does NOT copy image contents into bind mounts, so the container would start against an empty pgdata. The image's [entrypoint](../../docker/entrypoint.sh) self-heals this by copying from `/var/lib/postgresql/initial-data` (a baked fallback) into the bind-mounted directory the first time it's empty.

```bash
-v "$(pwd)/pgdata:/var/lib/postgresql/data"
```

### Reset to a fresh server

```bash
docker rm -f cimmeria
docker volume rm cimmeria-data
docker run ...   # same flags as before — image's pgdata seeds the new volume
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

Same caveat as the standalone server — **do not expose to the public internet**. The legacy Stargate Worlds protocol the client speaks is not designed to withstand hostile peers. Run on a LAN or a trusted VPN.
