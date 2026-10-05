---
title: "Colo / single-host auto-update deployment"
type: how-to
audience: operators
last_updated: 2026-10-04
---

# Colo / single-host auto-update deployment

How to host a public Cimmeria server on a Linux box you don't want to babysit, the way the project's colo runs it. You set it up once. After that, every new `latest-prerelease` image on GHCR replaces the running server within five minutes, and SigNoz keeps the logs, traces and metrics.

Everything here lives in [`docker/`](../../docker/). The files in the repo are the files the colo runs; what differs between hosts goes in two `.env` files.

This is **not** a hardened production setup. The image bundles Postgres with baked credentials and resets the database on every start ([container.md → Security](container.md#security)). It suits a public server for a trusted community, not a multi-tenant one.

## What runs

Two Compose projects, joined by one Docker network:

| Project | Directory on the host | Containers | Source |
|---|---|---|---|
| `cimmeria` | `/opt/cimmeria` | `cimmeria` (game server + bundled Postgres), `watchtower` | [`docker/compose.yml`](../../docker/compose.yml) and overlays |
| `signoz` | `/opt/cimmeria/signoz` | `signoz` (UI + query API), `signoz-otel-collector`, `signoz-clickhouse`, `signoz-zookeeper-1`, plus two one-shot init containers | [`docker/signoz/`](../../docker/signoz/), vendored from SigNoz v0.125.1 |

The game container reaches the collector as `otel-collector:4317` on the external network `signoz-net`. Only `signoz` and `otel-collector` join that network; ClickHouse (passwordless `default` user) and ZooKeeper (anonymous login) sit on SigNoz's own internal network, out of the game container's reach. Either project can be started first, and the game runs without SigNoz.

### Ports

| Port | Service | Who should reach it | Published on |
|---|---|---|---|
| `8081/tcp` | SOAP login; also launcher telemetry upload (`/api/auth/dev-session`, `/api/telemetry/*`) | Players | all interfaces |
| `32832/udp` | BaseApp, all game traffic after login | Players | all interfaces |
| `30000/tcp` | Minigame SmartFoxServer | Players | all interfaces |
| `8443/tcp` | Admin REST API. **No authentication** (#439) | Operators only | `ADMIN_PUBLISH`, default `127.0.0.1` |
| `8444/tcp` | Live Research Lab endpoint (overlay) | The owner's dev box over the VPN | `CIMMERIA_WG_IP` only |
| `8080/tcp` | SigNoz UI and query API | Operators | `SIGNOZ_UI_BIND`, default `127.0.0.1` |
| `4317/tcp`, `4318/tcp` | OTLP gRPC / HTTP into the collector. **No authentication** | Exporters outside Docker on the private network (a native server, workstation Claude Code telemetry) | `OTLP_BIND`, default `127.0.0.1` |

Forward only `8081/tcp`, `32832/udp` and `30000/tcp` from the internet. Docker-published ports bypass the host's `INPUT` firewall chain, so the network edge (the router or the provider's firewall) is what keeps the rest private. Ports `13001` and `50000/udp` appear in the image's `EXPOSE` list, but nothing binds them; don't publish or forward them.

**How player telemetry reaches SigNoz.** An opted-in launcher anywhere on the internet asks `:8081/api/auth/dev-session` for an HMAC-signed token, then uploads to `:8081/api/telemetry/*`. The game server checks the token and replays the rows into the collector over `signoz-net`, where they land as `service.name = cimmeria-client`. Nothing a player runs talks to SigNoz's own ports, so neither `8080` nor `4317`/`4318` needs to be reachable from the internet.

### Files on the host

```text
/opt/cimmeria/
├── compose.yml                 docker/compose.yml
├── compose.lab.yml             docker/compose.lab.yml               (overlay)
├── compose.discord-file.yml    docker/compose.discord-file.yml      (overlay)
├── compose.discord.yml         release asset, if you use that route (overlay)
├── .env                        from docker/.env.example, 0600, never committed
├── config/discord.toml         your Discord config, uid 1001, 0440, never committed
└── signoz/
    ├── compose.yaml            docker/signoz/compose.yaml
    ├── otel-collector-config.yaml
    ├── common/…                ClickHouse and OpAMP config
    └── .env                    from docker/signoz/.env.example, 0600
```

Server logs go to `CIMMERIA_LOG_DIR` (the colo uses a directory on its data disk). SigNoz data lives in the named volumes `signoz-clickhouse`, `signoz-sqlite` and `signoz-zookeeper-1`.

## Prerequisites

- A 64-bit Linux host with Docker Engine and the `docker compose` v2 plugin. The colo runs Debian 12 with Docker 29. Install Docker from [Docker's apt repository](https://docs.docker.com/engine/install/debian/); Debian's own `docker.io` package does not ship the compose v2 plugin.
- About 4 GB of RAM for SigNoz (ClickHouse is most of it) and disk for ClickHouse: the colo's grew to about 40 GB in its first four months. If the root filesystem is small, set Docker's `data-root` in `/etc/docker/daemon.json` to a bigger disk before you start.
- Outbound HTTPS to `ghcr.io`, Docker Hub and `github.com`. SigNoz's first boot downloads a ClickHouse function from a GitHub release.
- Edge forwards for `8081/tcp`, `32832/udp` and `30000/tcp` to the host, and a public address or DNS name for players.

## One-time setup

1. **Get the files.** Copy the repo's `docker/` directory to `/opt/cimmeria`. From a release tarball of `main`:

   ```bash
   sudo mkdir -p /opt/cimmeria && cd /opt/cimmeria
   curl -fsSL https://github.com/SandboxServers/Cimmeria/archive/refs/heads/main.tar.gz \
     | sudo tar -xz --strip-components=2 Cimmeria-main/docker
   sudo chown -R "$USER": /opt/cimmeria
   ```

   That also brings the `Dockerfile`, `entrypoint.sh` and `s6/`, which the host doesn't use; they're harmless.

   The commands below assume you run `docker compose` as a user in the `docker` group, not with `sudo`. The `docker compose` client reads `.env` and the compose files as the user who runs it, so that user owns `/opt/cimmeria` and each `.env` is mode 0600. If you run compose with `sudo` instead, skip the `chown`, keep the files owned by root, and use `sudo` for every `docker compose` command in this guide.

2. **Create the shared network**, once per host:

   ```bash
   docker network create signoz-net
   ```

3. **Start SigNoz** (skip this step to run without observability):

   ```bash
   cd /opt/cimmeria/signoz
   install -m 0600 .env.example .env
   $EDITOR .env         # SIGNOZ_JWT_SECRET=$(openssl rand -hex 32); SIGNOZ_UI_BIND / OTLP_BIND if operators reach it over a LAN or VPN
   docker compose up -d
   ```

   Allow about two minutes for the first boot (schema migrations). Then open `http://<SIGNOZ_UI_BIND>:8080` (or tunnel to it: `ssh -L 8080:127.0.0.1:8080 <host>`) and create the admin account. The first visitor creates it, so do this straight away.

4. **Start the game server:**

   ```bash
   cd /opt/cimmeria
   install -m 0600 .env.example .env
   $EDITOR .env         # BASE_EXTERNAL is the one required value
   docker compose up -d
   ```

   `BASE_EXTERNAL` is the address players connect to, handed to them during login. Compose refuses to start without it. Getting it wrong is the most common failure: players log in, then get bounced back to the login screen.

5. **Check it:**

   ```bash
   docker ps --format 'table {{.Names}}\t{{.Status}}'      # cimmeria (healthy) after ~1 min
   docker logs cimmeria 2>&1 | grep -E '\[otel\] Streaming|Discord notifications|dev-session telemetry'
   ```

   `[otel] Streaming to http://otel-collector:4317` means telemetry is flowing. In SigNoz, Logs → `service.name = cimmeria-server` shows rows within a minute.

## What happens on every update

Watchtower checks `ghcr.io/sandboxservers/cimmeria-server:latest-prerelease` every five minutes. When the digest changes, it pulls the new image, stops and removes the `cimmeria` container, starts a replacement with the same settings, and deletes the old image. Downtime is about 30 seconds.

It touches nothing else. `WATCHTOWER_LABEL_ENABLE=true` limits it to containers labelled `com.centurylinklabs.watchtower.enable=true`, which only the `cimmeria` service is. Never run a watchtower without that or a `WATCHTOWER_SCOPE`: it would update every container on the host. `DOCKER_API_VERSION=1.44` is required on Docker 29 and later, which refuses watchtower's default API version, and watchtower then fails every poll.

**Edits to the compose files or `.env` don't reach the running container until you run `docker compose up -d`.** Watchtower builds each replacement from the *old container's* settings, not from the files. Get into the habit: edit, then `docker compose up -d`, then check `docker inspect cimmeria` shows the change.

Releases are cut by hand (`/release` on a merged PR, or a manual run of `release-container.yml`), so merging to `main` alone never restarts the colo.

## The database resets on every start

[`docker/entrypoint.sh`](../../docker/entrypoint.sh) copies the image's baked database over `/var/lib/postgresql/data` every time the container starts. An image update, `docker restart cimmeria`, a Docker restart and a host reboot all start from a clean database. Characters, missions and inventory do not survive any of them. This is deliberate while the schema keeps changing.

The reset is in the entrypoint because Docker volume mechanics can't do it: watchtower attaches the old container's anonymous volume to its replacement before removing the old one, so Docker refuses to delete it, and the database would carry over. `WATCHTOWER_REMOVE_VOLUMES=true` is set but doesn't change this.

Mounting a volume does not make the database persistent; the entrypoint overwrites whatever is there. The route to persistence is an external Postgres via `DB_URL` ([container.md](container.md#volume--persistence)), and then you own schema drift between releases.

Hosts with automatic security updates and reboots (the colo reboots at 04:30 when an update needs it) reset the database on those reboots too.

## Optional overlays

Pick overlays with `COMPOSE_FILE` in `.env`, so a bare `docker compose up -d` always applies the same set. The colo uses:

```bash
COMPOSE_FILE=compose.yml:compose.discord-file.yml:compose.lab.yml
```

### Discord notifications

The server can post lifecycle and error events to Discord webhooks ([discord-notifications.md](../architecture/discord-notifications.md)). There are two ways to give it the config. Use one.

**From a host file**, [`compose.discord-file.yml`](../../docker/compose.discord-file.yml) (what the colo does). Write `config/discord.toml` yourself, starting from [config/discord.toml.example](../../config/discord.toml.example); any channels, muted accounts and event toggles work without a new release.

```bash
cd /opt/cimmeria && mkdir -p config
install -m 0600 /dev/null config/discord.toml && $EDITOR config/discord.toml
sudo chown 1001:1001 config/discord.toml && sudo chmod 0440 config/discord.toml
```

The server runs as uid 1001 inside the container and must be able to read the file. If it can't, it logs `Discord notifications disabled` with `file_present=true` and carries on without them. If the file is missing when the container is created, Docker creates an empty directory in its place; remove the directory, write the file, and run `docker compose up -d` again.

**From the release**, `compose.discord.yml`. The release workflow renders the webhook URLs from the repo's `DISCORD_LIFECYCLE_WEBHOOK` and `DISCORD_ERRORS_WEBHOOK` Actions secrets into an overlay attached to each GitHub release. It passes the TOML as `DISCORD_CONFIG_TOML`, which the entrypoint writes into the container. It carries only the channels the workflow knows about.

```bash
curl -fL -o compose.discord.yml \
  "https://github.com/SandboxServers/Cimmeria/releases/latest/download/compose.discord.yml"
chmod 0600 compose.discord.yml
```

Either way, check `docker logs cimmeria 2>&1 | grep 'Discord notifications'` says `enabled` after a deploy. To add a channel to the release route: a `DISCORD_<CHANNEL>_WEBHOOK` secret, a `[discord.channels.<channel>]` block with a `__DISCORD_<CHANNEL>_WEBHOOK__` sentinel in [docker/compose.discord.yml](../../docker/compose.discord.yml), and a matching substitution in the render step of [release-container.yml](../../.github/workflows/release-container.yml).

### Live Research Lab endpoint (VPN only)

The in-server lab endpoint (`cimmeria-lab-mcp`) lets the owner's dev box read live server state and run captured dot-console commands ([design](../architecture/live-research-lab.md), [rulebook](../guides/live-research-lab.md)). It is off unless both of these hold:

1. The server gets `CIMMERIA_LAB_MCP_BIND` and a `CIMMERIA_LAB_MCP_TOKEN` of 32+ bytes.
2. [`compose.lab.yml`](../../docker/compose.lab.yml) publishes `8444` on `CIMMERIA_WG_IP` only.

Add to `.env`:

```bash
CIMMERIA_WG_IP=<an address this host owns that the dev box reaches over the VPN; the LAN address when the VPN lands on the LAN>
CIMMERIA_LAB_MCP_TOKEN=<openssl rand -hex 32>
```

and `compose.lab.yml` to `COMPOSE_FILE`. Compose refuses to start the overlay if either value is missing. The address is also the endpoint's `Host` allowlist; anything else gets `403 Host header is not allowed`. From the dev box: `http://$CIMMERIA_WG_IP:8444/mcp` with `Authorization: Bearer <token>`, in the `lab-server` entry of `.mcp.json` ([.mcp.json.example](../../.mcp.json.example)). Every call logs one `lab.tool_call` event to SigNoz. On the colo, touch only the lab character and what it spawns unless the owner says otherwise in that session.

### Client telemetry from players

Off until two values are in `.env`:

```bash
CIMMERIA_TELEMETRY_HMAC_SECRET=<openssl rand -hex 64>
CIMMERIA_TELEMETRY_UPLOAD_ENDPOINT=http://<public host>:8081/api/telemetry
```

Then `docker compose up -d` and check `docker logs cimmeria 2>&1 | grep 'dev-session telemetry'` says `mint and ingest enabled`. A missing or short secret doesn't stop the server; it logs `dev_session_secret_unusable`. Details, secret rotation and checks: [telemetry.md](telemetry.md#enable-client-telemetry-on-the-colo).

### Claude Code telemetry

Workstations can export Claude Code's usage telemetry to the colo SigNoz over the private network: [claude-code-telemetry.md](claude-code-telemetry.md). That needs `OTLP_BIND` set to the host's private address. The `transform/claude-code-scrub` processor in [`docker/signoz/otel-collector-config.yaml`](../../docker/signoz/otel-collector-config.yaml) drops `user.email` from those logs and metrics; keep it when you upgrade SigNoz.

### Watchtower swap notifications

Watchtower can announce each image swap through [shoutrrr](https://containrrr.dev/shoutrrr/). That's separate from the server's own Discord events. Set `WATCHTOWER_NOTIFICATIONS=shoutrrr` and `WATCHTOWER_NOTIFICATION_URL=discord://<token>@<webhook-id>` in `.env` (the token and ID are the two parts of a Discord webhook URL, in that order).

## Operating it

```bash
cd /opt/cimmeria
docker logs -f cimmeria                                  # server console + postgres
docker exec watchtower /watchtower --run-once cimmeria   # check for a new image now
docker compose pull && docker compose up -d              # same, by hand
docker compose up -d                                     # apply edits to compose files or .env
docker compose down                                      # stop the game (SigNoz keeps running)
cd signoz && docker compose up -d                        # apply SigNoz edits
```

The server writes `logs/*.log` under `CIMMERIA_LOG_DIR` and moves the previous run's files to `logs/archive/<timestamp>/` at each start. Nothing rotates or deletes them; the colo accumulated 6 GB in a week of busy testing. Prune old archives with a cron job if the disk is small:

```bash
find /path/to/logs/archive -mindepth 1 -maxdepth 1 -type d -mtime +14 -exec rm -rf {} +
```

Container stdout is capped by the json-file driver (5 × 50 MB for `cimmeria`).

Container recreations occasionally leave an unused anonymous volume of about 100 MB (an old Postgres data directory). `docker volume prune` removes unused anonymous volumes; named volumes such as SigNoz's are kept.

SigNoz retention (how long logs, traces and metrics are kept) is set in the SigNoz UI under Settings. ClickHouse is the only thing on the host that grows without bound, so check `docker system df -v` now and then.

Upgrading SigNoz: [signoz-deployment.md → Upgrading SigNoz](signoz-deployment.md#upgrading-signoz).

## Troubleshooting

**Players log in, then return to the login screen.** `BASE_EXTERNAL` isn't an address they can reach, or `32832/udp` isn't forwarded. The server itself is fine.

**`network signoz-net declared as external, but could not be found`.** Run `docker network create signoz-net`.

**`[otel] ... not reachable after 120s; starting without it` in the log**, then export errors. SigNoz is down or not on `signoz-net`. To run without it, set `OTEL_EXPORTER_OTLP_ENDPOINT=` (empty) in `.env` and `docker compose up -d`; the server then skips the 120-second wait too.

**Watchtower never updates.** `docker logs watchtower --tail 20` should show a `Session done ... Scanned=1` line every five minutes. `Scanned=0`: the label is missing from `cimmeria`. `client version 1.25 is too old`: `DOCKER_API_VERSION` is missing (Docker 29+).

**A setting I changed isn't in effect.** You edited a file but didn't run `docker compose up -d`; see [What happens on every update](#what-happens-on-every-update).

**`docker compose` complains `required variable BASE_EXTERNAL is missing`.** You're running it outside `/opt/cimmeria`, or `.env` lacks the value.

**SigNoz UI loads but shows no data.** Check the game log for `[otel] Streaming` and `docker logs signoz-otel-collector --tail 50`. More in [signoz-deployment.md → Troubleshooting](signoz-deployment.md#troubleshooting).

## When to move off this setup

- **Persistent characters** → an external Postgres via `DB_URL`, and a migration plan for each release.
- **More than one host** → an orchestrator; watchtower doesn't coordinate across nodes.
- **Staged rollouts** → a CI/CD pipeline with a canary, not a polling auto-updater.

[container.md → Threat model](container.md#threat-model--deliberate-trade-offs) lists the image's other trade-offs.
