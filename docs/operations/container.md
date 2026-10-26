---
title: "Container distribution"
type: how-to
audience: operators
last_updated: 2026-10-23
---

# Container distribution

Self-contained Docker image: server binary + cooked game data + pre-loaded Postgres + s6-overlay supervisor, in one published artifact. The goal is `docker run` and you have a working Cimmeria server — no Rust toolchain, no MSVC, no PowerShell bootstrap, no `.sql` files to load.

> Looking to deploy this to a colo / single-host box with auto-update on every new release? See [Colo Auto-Update Deployment](colo-deploy.md): the game stack is [`docker/compose.yml`](../../docker/compose.yml) and the optional SigNoz stack is [`docker/signoz/`](../../docker/signoz/), joined by the `signoz-net` Docker network.

## Release model

Releases are **explicit**. Pushing to `main` does not publish a container — many PRs land for routine work that isn't release-worthy. To cut a release, you trigger [`.github/workflows/release-container.yml`](../../.github/workflows/release-container.yml) one of three ways:

1. **Comment `/release` on a merged PR.** [`release-on-comment.yml`](../../.github/workflows/release-on-comment.yml) validates the commenter has repo write permission, confirms the PR is merged, and dispatches the release workflow. The PR you comment on is just the cut-point marker — the build always uses `main` HEAD, so this captures every PR that's been merged up to that point. Reactions on your comment indicate progress: 👀 received, 🚀 dispatched, 👎 rejected.
2. **Manual UI dispatch.** Open **Actions → `release-container`**, click "Run workflow", pick `main`. Useful when you decided post-merge that something is ship-worthy and don't want to leave a comment trail.
3. **CLI dispatch.** `gh workflow run release-container.yml --ref main`.

Breakage is still caught without a release. [`pr-container.yml`](../../.github/workflows/pr-container.yml) builds and smoke-tests the image, without pushing it, in two places:

- **On a PR that touches the container files** (`docker/**`, `Cargo.lock`, `.trivyignore`, the two container workflows, `tools/container-smoke.sh`), before merge.
- **On `main` after a merge that touches any image input** (crates, `db/`, `entities/defs/`, cooked data, `.cargo/`, and the container files). A burst of merges is checked once, at the newest commit. A failure opens a `main-red` issue, and `ship.py merge` refuses code PRs until a passing run closes it ([pre-pr-checks.md § Main is red](../agents/pre-pr-checks.md#main-is-red)). So a crates-only change that breaks the image is caught one merge later, not before it lands.

Until 2026-10-11 every crates or `db/` PR ran the container build, which made it the self-hosted runners' largest load for a check that never gated a merge. `release-container.yml` itself never runs on a push to `main`.

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
  -p 32832:32832/udp \
  -p 8081:8081 \
  -p 127.0.0.1:8443:8443 \
  -p 30000:30000 \
  -e BASE_EXTERNAL=<your-LAN-or-WAN-ip> \
  ghcr.io/sandboxservers/cimmeria-server:latest-prerelease
```

Add `-v cimmeria-server-pgdata:/var/lib/postgresql/data` to keep the database across restarts and image updates; without it Docker makes an anonymous volume that is lost when you remove and re-run the container by hand. Create the named volume once with `docker volume create cimmeria-server-pgdata`. See [Volume / persistence](#volume--persistence).

| Port | Protocol | Purpose |
|---|---|---|
| 32832 | UDP | BaseApp |
| 8081  | TCP | Auth HTTP (SOAP login); also launcher telemetry upload (`/api/auth/dev-session`, `/api/telemetry/*`) |
| 8443  | TCP | Admin REST API (no authentication; publish on loopback) |
| 30000 | TCP | Minigame SmartFoxServer (Livewire, Hack, Bypass, GoauldCrystals, Alignment, Activate, Analyze, Converse). Admission and timeouts: [Connection admission](../gameplay/minigame-system.md#connection-admission) |

> The image also `EXPOSE`s `13001/tcp` (`AUTH_PORT`) and `50000/udp` (`CELL_PORT`), but nothing binds either: client login is the SOAP listener on `8081`, and the CellApp runs in-process. Don't publish them.
>
> The legacy C++ BaseApp had a Python console on port 8989; the Rust server does not implement it, so the container does not expose it.

## Environment

Every variable that [`crates/server/src/main.rs`](../../crates/server/src/main.rs) reads is overridable at `docker run` time. The image bakes defaults for the subset below (see the `ENV` block at [`docker/Dockerfile:254-268`](../../docker/Dockerfile)); the full catalogue, including anything not listed here, is the env-var table in `main.rs`'s module header.

| Variable | Default | Notes |
|---|---|---|
| `AUTH_HOST` | `0.0.0.0` | Auth bind address |
| `AUTH_PORT` | `13001` | Read into the config but bound by no listener; client login uses `LOGON_PORT`. |
| `LOGON_PORT` | `8081` | |
| `BASE_HOST` | `0.0.0.0` | |
| `BASE_EXTERNAL` | `127.0.0.1` | **Must override for non-localhost clients.** |
| `BASE_PORT` | `32832` | |
| `CELL_PORT` | `50000` | Read into the config but bound by no listener; the CellApp runs in-process. |
| `ADMIN_PORT` | `8443` | |
| `ADMIN_BIND` | `0.0.0.0` (image) / `127.0.0.1` (bare binary) | The admin API has no auth (#439). The bare binary binds loopback; the image binds wide because a Docker published port forwards to the container's bridge address and cannot reach an in-container loopback bind. **In a container the `-p` publish is the exposure control**, not this variable: publish as `-p 127.0.0.1:8443:8443` for operator-only access, or drop the publish. A plain `-p 8443:8443` exposes unauthenticated admin control to every host that can route to the port. Launcher telemetry ingest is also served on the login port (`8081`), so launchers on other hosts don't need `8443`. |
| `DB_URL` | `host=127.0.0.1 port=5432 user=w-testing password=w-testing dbname=sgw` | Libpq-style — see note below. Note this differs from the non-container default (`port=5433`). |
| `DEVELOPER_MODE` | `false` | Skips the client protocol-digest check at login. It never bypasses the password check while `DB_URL` is set. Images built before this default changed shipped `true`; see [Database login check](#database-login-check). |
| `RUST_LOG` | `info` | tracing-subscriber filter |

Not baked into the image, but read by the server and worth setting on a real deployment:

| Variable | Default | Notes |
|---|---|---|
| `PROTOCOL_DIGEST` | (compiled-in) | 32-char hex digest sent in the auth response — only override if you're testing protocol changes |
| `MERCURY_ENCRYPTION_VERSION` | `1` | Server-wide Mercury wire-encryption version. `1` = legacy (the only version unpatched clients understand); `2` = modernized. No per-client negotiation. |
| `AUTH_TLS_RELOAD_INTERVAL_SECS` | `30` | Poll interval for hot-reloading the auth TLS cert/key on change (e.g. a Let's Encrypt renewal). `0` disables. Only active when the TLS listener is configured. |
| `MINIGAME_PORT` | `30000` | Minigame SmartFoxServer TCP port, also advertised to the client. Publish the same port (`-p <port>:<port>`). |
| `CIMMERIA_DEPLOY_ENV` | `dev` | Sets `deployment.environment` and `cimmeria.deploy_env` on every span/log/metric. Set to `colo` on a colo box so its data doesn't mix with dev-laptop noise. `OTEL_RESOURCE_ATTRIBUTES` cannot override either. |
| `CIMMERIA_GIT_SHA` | `unknown` | **Build argument, not a runtime variable.** Baked in by `crates/server/build.rs` as the OTLP `service.version` resource attribute. `release-container.yml` passes `github.sha` and `pr-container.yml` the PR head SHA (`github.sha` on `main`); a source build falls back to `git rev-parse HEAD`. |
| `OTEL_EXPORTER_OTLP_ENDPOINT` | (unset) | OTLP collector endpoint. Unset ⇒ exporter disabled. [`docker/compose.yml`](../../docker/compose.yml) sets this to `http://otel-collector:4317`, the SigNoz collector on the `signoz-net` network. When set, the container waits up to `OTEL_WAIT_TIMEOUT` seconds (default 120) for it at start, then starts without it. See [signoz-deployment.md](signoz-deployment.md). |
| `OTEL_EXPORTER_OTLP_PROTOCOL` | `grpc` | `grpc` or `http/protobuf`. |
| `OTEL_SERVICE_NAME` | `cimmeria-server` | `service.name` in SigNoz's service map. |
| `OTEL_RESOURCE_ATTRIBUTES` | (unset) | Comma-separated `k=v` resource attributes. |
| `OTEL_TRACES_SAMPLER` | `always_on` | `always_on`, `always_off`, or `traceidratio` with `OTEL_TRACES_SAMPLER_ARG`. |
| `CIMMERIA_TELEMETRY_HMAC_SECRET` | (unset) | Secret for the launcher dev-session token mint. Unset ⇒ the endpoint returns 500. See [telemetry.md](telemetry.md). |
| `CIMMERIA_TELEMETRY_UPLOAD_ENDPOINT` | (unset; no default until LT-41) | Upload base URL handed back to the launcher; use an `https://` URL you serve, e.g. `https://play.cimmeria.app/api/telemetry`. Blank counts as unset. Unset, the server derives `https://<Host>/api/telemetry` only when a proxy sends `Host` and `X-Forwarded-Proto: https`, and otherwise the mint answers 503 `no_upload_endpoint`; there is no localhost fallback, so **Docker Desktop self-hosters get 503 until they set it**. The proxy must pass `Host`, `X-Forwarded-Proto: https` and `X-Forwarded-For`. See [telemetry-upload-endpoint.md](telemetry-upload-endpoint.md). |
| `CIMMERIA_TELEMETRY_KILL_SWITCH` | (unset) | Set to the literal `1` to pause telemetry ingest: mint, refresh, the launcher-summary route and the uploads. |
| `CIMMERIA_TELEMETRY_UPLOAD_QUOTA_PER_IP` | `600` | Chunk uploads per peer address per minute; `0` disables. Shared by everyone behind one address. See [telemetry operations](telemetry.md#upload-size-and-rate-limits). |
| `CIMMERIA_TELEMETRY_BUNDLE_QUOTA_PER_IP` | `30` | Bundle uploads per peer address per hour; `0` disables. |
| `CIMMERIA_TELEMETRY_SUMMARY_QUOTA_PER_IP` | `12` | Requests per peer address per minute on the anonymous launcher-summary route; `0` disables. Everyone behind one address shares it, so raise it for a large deployment behind one. See [telemetry operations](telemetry.md). |
| `CIMMERIA_COOKED_PRESYNC_DISABLED` | (unset) | `1` or `true` turns the launcher cooked-data presync routes off (503 `disabled`); launchers skip the presync and launch. The bypass for a broken catalog. Read per request. See [cooked-data pipeline](../engine/cooked-data-pipeline.md#launcher-presync). |
| `CIMMERIA_COOKED_PRESYNC_QUOTA_PER_IP` | `60` | Requests per peer address per minute on the two presync routes; `0` disables. Everyone behind one address shares it. See [telemetry operations](telemetry.md#cooked-data-presync). |
| `CIMMERIA_DB_BACKUP_KEEP` | `10` | How many pre-rebuild backups `db-sync` keeps in `/var/backups/cimmeria`. Must be a whole number of 1 or more, or the start stops. |
| `CIMMERIA_DB_FORCE_REBUILD` | (unset) | `1` rebuilds the resources tables at the next start even if the image is unchanged. Player data is kept. Remove it afterwards. See [the runbook](persistent-player-data.md#forced-resources-rebuild). |
| `CIMMERIA_DB_RESTORE` | (unset) | Disaster recovery only. A backup file name (or container path) that `db-sync` restores over the live database once, after taking a safety backup. See [Restore](persistent-player-data.md#restore). |
| `CIMMERIA_BACKUP_DIR` | `./backups` | A compose variable, not a container one: the host directory bound to `/var/backups/cimmeria`. |
| `DISCORD_CONFIG_TOML` | (unset) | Read by the entrypoint, not the server: when set, its contents are written to `/opt/cimmeria/config/discord.toml` (mode 0440) before the server starts. [`docker/compose.discord.yml`](../../docker/compose.discord.yml) sets it. Unset leaves any existing file alone. |
| `CIMMERIA_LAB_MCP_ALLOWED_HOSTS` | (unset) | Extra `Host` header values the lab MCP endpoint accepts, comma-separated, on top of `localhost`, `127.0.0.1` and `::1`. [`docker/compose.lab.yml`](../../docker/compose.lab.yml) sets it to `CIMMERIA_WG_IP`. |

> `DB_URL` must not be empty in this image. An empty value makes the server run with no database, so the container refuses to start with it (see [Database login check](#database-login-check)).
>
> `DB_URL` must be in libpq key-value form, not URL DSN form. `crates/services/src/orchestrator_postgres.rs::ensure_postgresql_running` parses `host=` / `port=` tokens to decide whether to auto-start the bundled Postgres. A URL like `postgres://...` would silently fall back to `localhost:5433` and emit warnings, even though sqlx itself accepts either form.

## Volume / persistence

> **The database persists on the named volume `cimmeria-server-pgdata`.** A restart, a reboot and an image update keep accounts and characters; an image update rebuilds only the resources tables. Operating it: [persistent-player-data.md](persistent-player-data.md).

The database lives at `/var/lib/postgresql/data` (declared as a `VOLUME`). On start, the [entrypoint](../../docker/entrypoint.sh) copies the baked database from `/var/lib/postgresql/initial-data` into that path only when it holds no cluster (`PG_VERSION` absent), and keeps an existing cluster as it is.

[`docker/compose.yml`](../../docker/compose.yml) mounts the external named volume `cimmeria-server-pgdata` there. A bare `docker run` with no `-v` gets an anonymous volume, so `docker rm` and `docker run` by hand start a fresh server. (Watchtower's swap attaches the old container's anonymous volume to the replacement, so the data would carry over there, but the volume is then unnamed and easy to lose.) Pass the named volume to keep the data.

After Postgres is up, the s6 oneshot `db-sync` ([`docker/db-sync.sh`](../../docker/db-sync.sh)) decides what to do with the database: `first-boot` for a new volume, `skip` for the same image, `rebuild` for a new image or `CIMMERIA_DB_FORCE_REBUILD`, `adopt` for a database from before this feature, `restore` for `CIMMERIA_DB_RESTORE`. The server starts only when it exits 0. Backups of `sgw` go to `/var/backups/cimmeria` (a `VOLUME`; compose binds `${CIMMERIA_BACKUP_DIR:-./backups}` to it).

An external Postgres via `DB_URL` still works, with the schema drift between releases being yours to manage.

### Reset to a fresh server

A restart keeps the data, so a fresh server is a deliberate action: remove the container and its volume, then start again. The next start is a first boot from the image's seeds.

```bash
docker rm -f cimmeria
docker volume rm cimmeria-server-pgdata
docker volume create cimmeria-server-pgdata
docker run ...   # same flags as before, with -v cimmeria-server-pgdata:/var/lib/postgresql/data
```

Take a backup first if you might want the players back: [persistent-player-data.md → Backups](persistent-player-data.md#backups). These commands are untested on the colo.

## What's inside

- `cimmeria-server` (Linux x86_64), built `cargo build -p cimmeria-server --release`.
- `data/cache/*.pak` — cooked game data (22 files, required at runtime).
- `data/spaces/*.nav` — navmeshes (5 files, optional but shipped — without them combat AI / line-of-sight degrades).
- Postgres 17.9 + pgdata pre-loaded from `db/database.sql` (schema + seeds, `\ir`'d from `db/sgw/` and `db/resources/`), plus a fresh dump and the `db/persistent/` SQL that `db-sync` uses to rebuild the resources on an image update.
- s6-overlay v3 supervising postgres, `db-sync` and cimmeria-server. Postgres starts first, `db-sync` decides what to do with the database, the server waits for `pg_isready` and then for a working login with `DB_URL`, and server death brings the container down so the orchestrator restarts it cleanly.

## What's NOT inside

- No Rust source, no `target/`, no build tooling.
- No `db/*.sql` seed files for the server to apply lazily. The DB is *identity-baked* — schema + seed data are part of the image's tag; the `db/persistent/` SQL and the fresh dump ship only so `db-sync` can rebuild the resources.
- No `data/scripts/`, `entities/defs/`, `external/`, or `tools/`. Those are compile-time inputs; the runtime never reads them.

## Image architecture (Dockerfile stages)

1. **build-base / planner / dependencies** — pinned Rust toolchain and native inputs; cargo-chef 0.1.78 prepares a workspace recipe and cooks server/probe dependencies separately into reusable layers.
2. **builder** — restores real sources, seeds empty/new-recipe Cargo cache mounts from dependency layers, then runs separate locked release builds for the server and login probe. The `login-probe` export target contains only the CI probe.
3. **db-init** — `postgres:17.9-bookworm`, `initdb`, loads `db/database.sql`, captures pgdata.
4. **runtime** — `postgres:17.9-bookworm` + s6-overlay v3, copies the binary + cooked data + baked pgdata.

Source: [`docker/Dockerfile`](../../docker/Dockerfile). The [container build reference](../../docker/README.md) explains layer versus mount reuse and cold/fresh-builder validation. Hosted publication, security scans, provenance/SBOM and smoke-before-promotion gates remain. Changing the build cache does not publish or deploy an image.

## Healthcheck

`HEALTHCHECK` has a 300 s start period, so a resources rebuild by `db-sync` after an image update does not count against the container. It checks two things:

1. `cimmeria-server --check-db`: the server's own connection string can log in and run `SELECT 1`. `pg_isready` alone passes while the user, password or database name is wrong.
2. A TCP probe (`bash -c 'exec 3<>/dev/tcp/127.0.0.1/$LOGON_PORT'`) confirming the SOAP login listener is bound — this is the port the game client logs in through.

It does not probe `AUTH_PORT` (13001): no listener binds that port, and probing it marked every container unhealthy. A bound port also says nothing about whether a login works, which is why the CI smoke below logs in.

## CI login smoke

`pr-container.yml` (on PRs that touch the container files, and on `main` after a merge that touches the image's inputs; see [Release model](#release-model)) and `release-container.yml` (on every release) run the same check, [`tools/container-smoke.sh`](../../tools/container-smoke.sh), against the image they just built. It starts the container with the image's defaults, publishing only the login port (`LOGON_PORT`, TCP) and the BaseApp port (`BASE_PORT`, UDP) as the image sets them, and then:

1. **Waits for readiness, probed from inside the container**: `pg_isready`, a TCP connect to the login port, and the BaseApp's UDP socket (the image's `BASE_PORT`) in `/proc/net/udp`. Auth starts before the BaseApp, so the login port alone is not enough. A host-side probe of a published port always connects, because docker-proxy accepts the handshake itself. The budget is about 120 s; a container that exits during the wait is reported as "container exited".
2. **Logs in as a client with `login-probe`**, from the runner through the published ports. The probe is a binary in `cimmeria-wireclient` (`crates/wireclient/src/login_probe.rs`), compiled in the image's builder stage and exported by the Dockerfile's `login-probe` target, so it never ships in the image. Its four steps run in order.

| Step | What must happen | What it guards against |
|---|---|---|
| Wrong password | The seeded `test` account with a wrong password is rejected with exactly "The account name or password is incorrect." | A server that accepts any password: developer mode without a database answers every login as account 1. A rejection for another reason also fails, so a broken database ("A request to the database server failed.") cannot pass as a correct rejection. |
| Real login | The real password passes SOAP Phase 1 and Phase 2, and Phase 1 returns the account id the container's own database holds for `test`. The shard is the first row of the container's `shards` table. | Broken credential checking, a missing or changed seed account, no registered shard, or a Phase 2 that fails. |
| Advertised endpoint | Phase 2 advertises `127.0.0.1` (the image's documented `BASE_EXTERNAL` default) and the host port Docker published for the BaseApp's UDP port (`docker port`). Neither value is read from the server's own `BASE_*` variables, so a wrong default in the image fails, as does one that never reaches the config. A job that overrides `BASE_EXTERNAL` must set `SMOKE_EXPECT_BASE_HOST` to match; neither workflow does. | A wrong image default or wiring for `BASE_EXTERNAL` / `BASE_PORT`, including `0.0.0.0` or the container's bridge address, which the handshake alone would accept because both are reachable from the Docker host. |
| Mercury handshake | `baseAppLogin` runs against the endpoint Phase 2 advertised, as a client would, and returns the encrypted login reply and time-sync packets under the Phase 2 session key. | A BaseApp that is unreachable at the advertised endpoint, or that does not accept the auth service's ticket and session key. |

On every run the script prints the container state and `docker logs`, and copies the server's log files out of `/var/log/cimmeria`; when the job fails, the workflow uploads them as the `container-smoke-logs` artifact (kept 14 days).

The smoke stops at the handshake. Character select and world entry need a character row, and the image's database is the plain seed; that path is covered by the live-DB wireclient tests instead (`crates/wireclient/tests/it/`).

A second smoke, [`tools/container-persistence-smoke.sh`](../../tools/container-persistence-smoke.sh), runs in `pr-container.yml` on the same triggers. It boots the image against its own scratch volume and walks first boot, skip, forced rebuild with seed-account top-up, reconcile, a failed rebuild, adopt, restore and a full wipe. It never touches `cimmeria-server-pgdata`.

To run the login smoke locally with Docker:

```bash
docker buildx build -f docker/Dockerfile -t cimmeria-server:local --load .
docker buildx build -f docker/Dockerfile --target login-probe --output type=local,dest=/tmp/probe .
bash tools/container-smoke.sh cimmeria-server:local /tmp/probe/login-probe /tmp/smoke-logs
```

In `release-container.yml` the order is: build for scan, Trivy, export the probe, push the dated tag, smoke, then promote `latest-prerelease`. A smoke failure ends the job before the promote, so the rolling tag never points at an image that fails to log in. The dated tag is already pushed by then; it is immutable, and the next release's version step skips past it.

## Database login check

The server never runs without the database `DB_URL` names. Three layers enforce it:

1. **Before the server starts**, the s6 run script ([`docker/s6/cimmeria-server/run`](../../docker/s6/cimmeria-server/run)) waits for `pg_isready`, then runs `cimmeria-server --check-db`. That mode reads `DB_URL` from the environment, so the password never appears in a process's arguments, parses it exactly as the server does (libpq key-value or a percent-encoded `postgres://` URL), runs `SELECT 1`, and gives up after `PGCONNECT_TIMEOUT` seconds (the script sets 5). The script retries `DB_AUTH_CHECK_ATTEMPTS` times (default 10, one second apart; a value that is not a positive whole number falls back to 10), then prints a fixed reason such as `the database does not exist (SQLSTATE 3D000)` and exits 1. The reason never includes the connection string or the server's own message. An empty `DB_URL` fails the same way.
2. **The server itself** refuses to start when `DB_URL` is set and the connection fails or times out (30 s). It logs `reason=database_connect_failed` and exits 1.
3. **The container exits non-zero.** The s6 finish script ([`docker/s6/cimmeria-server/finish`](../../docker/s6/cimmeria-server/finish)) passes the server's non-zero exit code on as the container's, so `docker ps -a` shows `Exited (1)`, not a clean stop, and a `restart:` policy retries it.

To check it by hand, start the image with a database name that does not exist and confirm it exits 1 without binding the login port:

```bash
docker run --name dbcheck \
  -e DB_URL="host=127.0.0.1 port=5432 user=w-testing password=w-testing dbname=does_not_exist" \
  ghcr.io/sandboxservers/cimmeria-server:latest-prerelease
docker inspect -f '{{.State.ExitCode}}' dbcheck   # 1
docker logs dbcheck 2>&1 | grep 'cannot log in to the database'
docker rm dbcheck
```

`pr-container.yml` runs the same check on every container PR and on `main`. The bundled Postgres trusts loopback connections, so in this image a wrong password is not refused, but a wrong user or database name is. Against an external Postgres that checks passwords, a wrong password fails the same way.

The login fallback that accepts any credential as account 1 with access level 99 now needs developer mode **and** an empty `DB_URL`, and logs a WARN (`reason=dev_mode_no_db_login`) at startup and on every login it accepts. This image never meets the second condition.

## Logs

The server writes one file per subsystem to `/opt/cimmeria/logs/`, which is a symlink to the declared volume `/var/log/cimmeria`. At each start it moves the previous run's files into `logs/archive/<timestamp>/`. Mount a host directory there (`docker/compose.yml` uses `${CIMMERIA_LOG_DIR:-./logs}`) or the logs go with the container on the next update. The files are not rotated, so put that directory on a disk with room. The entrypoint hands the directory to the `cimmeria` user, so a root-owned bind mount works.

The files keep every DEBUG and TRACE row, where SigNoz samples the noisiest targets, and the container's stdout carries Postgres's own log, which never reaches SigNoz.

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
  -p 32832:32832/udp -p 8081:8081 -p 127.0.0.1:8443:8443 -p 30000:30000 \
  -e BASE_EXTERNAL=<your-LAN-or-WAN-ip> \
  ghcr.io/sandboxservers/cimmeria-server:latest-prerelease
```

| Flag | Why |
|---|---|
| `--read-only` | Root filesystem is immutable. Stops a compromised process from persisting changes to the image content. The `VOLUME`-declared pgdata, log and backup paths are the only writable locations, by design. |
| `--tmpfs /run /tmp /var/run` | s6-overlay and Postgres both write status / sockets / lock files to these paths. Tmpfs satisfies them without dropping `--read-only`. |
| `--cap-drop=ALL` + selective `--cap-add` | Default Docker capabilities are over-broad. The added five are what s6-overlay's `s6-setuidgid` privilege drops and Postgres's directory-perm enforcement actually need. Anything else is excess. |
| `--security-opt=no-new-privileges` | Hard kernel-level block on `setuid` / `setcap` escalation paths. Belt to the cap-drop suspenders. |

### Image supply-chain posture

- **Base images pinned by SHA digest** (`postgres:17.9-bookworm@sha256:...`, `rust:1-bookworm@sha256:...`). Tag-only references are vulnerable to tag mutation; the digest is the integrity anchor. Dependabot ([`.github/dependabot.yml`](../../.github/dependabot.yml)) opens weekly PRs to bump the digests when upstream rebuilds.
- **Third-party downloads SHA256-verified.** `external/recast` and the `s6-overlay` tarballs are checksum-checked before extraction; bumping the version ARG without bumping its companion SHA ARG is a loud build failure, not a silent acceptance of a different file.
- **CVE scanning on every build.** Trivy runs against the locally-built image before any `docker push` in [`release-container.yml`](../../.github/workflows/release-container.yml) and in every [`pr-container.yml`](../../.github/workflows/pr-container.yml) build (container PRs, and `main` after a merge that touches the image). Severity threshold is CRITICAL, fixable-only — the goal is to block known-exploited CVEs, not chase every theoretical issue.
- **OCI provenance + SBOM attestations** generated by buildx (`provenance: true, sbom: true` in the workflow). Enables downstream verification via `cosign verify-attestation` once we set up signing.
- **Network exposure caveat.** Same as the standalone server — do not expose to the public internet without a trusted client base. The legacy Stargate Worlds protocol the client speaks is not designed to withstand hostile peers. Run on a LAN or a trusted VPN.
