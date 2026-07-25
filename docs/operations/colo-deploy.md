---
title: "Colo / single-host auto-update deployment"
type: how-to
audience: operators
last_updated: 2026-07-25
---

# Colo / single-host auto-update deployment

How to host a publicly-reachable Cimmeria server on a Debian box you don't want to babysit. Goal: bring the box up once, and every time a new `latest-prerelease` lands on GHCR the running container is automatically replaced with the fresh one — DB and all.

This is **not** a production hosting story. The image is a self-contained demo (bundled Postgres with baked credentials, server + DB sharing one container, see [container.md → Security](container.md#security)). It's the right shape for a colo box running a public-but-trusted server, not for a hardened multi-tenant deployment.

## What you get

- One `docker compose up -d` and you're done.
- New GHCR `latest-prerelease` digest → container hot-swapped within ~5 minutes, no human action.
- Every swap starts from a **clean DB** (the image's baked pgdata). No migration drift, no schema reconciliation, no "did the last release break the save format?"
- Box rebooting is self-healing (`restart: unless-stopped`).

## What you don't get (yet)

- **No persistence, full stop.** Player characters, mission progress, inventory — all reset every time a new image rolls, *and* on any plain `docker restart`. The container entrypoint reseeds the database from the image on every start; a named volume does not change this. See [container.md → Volume / persistence](container.md#volume--persistence). This is deliberate while we're churning the schema.
- **No staged rollouts / canaries.** Watchtower replaces the running container as soon as it sees a digest change. If a release breaks boot, the box is down until the next release lands or you intervene manually.
- **No HA.** Single container, single host. If you need redundancy, run two boxes behind a load balancer; this guide doesn't cover that.

## Prerequisites

- Debian 12+ (or any distro with a current Docker).
- Docker Engine 24+ and the `docker compose` v2 plugin:
  ```bash
  sudo apt-get update
  sudo apt-get install -y docker.io docker-compose-plugin
  sudo systemctl enable --now docker
  ```
- A user in the `docker` group (or sudo every command).
- Ports `13001/tcp`, `32832/udp`, `50000/udp`, `8081/tcp`, `8443/tcp`, and `30000/tcp` (minigame SmartFoxServer) reachable from your players — these are the ports [`docker/compose.yml`](../../docker/compose.yml) publishes.

## One-time setup

1. Drop [`docker/compose.yml`](../../docker/compose.yml) onto the box (any path — `/opt/cimmeria/compose.yml` is a reasonable convention). The file is self-contained: it includes the cimmeria server, watchtower auto-update, and the full vendored SigNoz observability stack with all its config files inlined. No companion files needed.
2. Edit the `BASE_EXTERNAL` environment variable to the public/LAN IP your players will connect to. **This is the single mandatory edit.** The image default of `127.0.0.1` only works for clients on the same host as the container.
3. Bring it up:
   ```bash
   cd /opt/cimmeria
   docker compose -f compose.yml up -d
   ```

That's the entire administrative cost. From this point on, the box self-maintains.

## What happens on every update

Watchtower polls `ghcr.io/sandboxservers/cimmeria-server:latest-prerelease` every 5 minutes (`WATCHTOWER_POLL_INTERVAL=300`). When the digest moves:

1. Watchtower pulls the new image.
2. Stops the running `cimmeria` container.
3. Removes the old container (`WATCHTOWER_REMOVE_VOLUMES=true` is set, but see below — it is not what actually gives you the fresh DB).
4. Starts a fresh container with the same config and the new image. The container's entrypoint reseeds pgdata from the image-baked copy before Postgres boots.
5. Removes the old image (`WATCHTOWER_CLEANUP=true`) so disk doesn't accumulate.

Total downtime per swap: ~30 seconds (mostly Postgres boot + s6 service init, same as a cold start).

## Why every swap gets a fresh DB

The reseed happens **in the entrypoint**, not through Docker volume mechanics. On every container start, [`docker/entrypoint.sh`](../../docker/entrypoint.sh) unconditionally copies the image-baked `/var/lib/postgresql/initial-data` over `/var/lib/postgresql/data`, then hands off to s6-overlay. That is what makes each deploy start clean.

This is worth understanding because the obvious explanation is wrong. You might expect that, with no `volumes:` entry for the Postgres path, Docker would simply hand each new container a new anonymous volume populated from the image. In practice watchtower **attaches the old container's anonymous volume to the new container before removing the old one**, so Docker refuses the volume removal and the new instance inherits the previous database. Earlier revisions of this setup relied on `WATCHTOWER_REMOVE_VOLUMES=true` alone and did not reliably get a fresh DB. The entrypoint-level reseed sidesteps the mechanic entirely; `WATCHTOWER_REMOVE_VOLUMES=true` is now belt-and-suspenders only.

Two consequences follow. First, the reseed is unconditional — anything written to pgdata at runtime is discarded on the next start, including a restart that isn't an update. Second, named SigNoz volumes are unaffected and persist across swaps, so your observability history survives.

If you ever **do** want persistence across updates, note that mounting a named volume will not get you there — the entrypoint reseed runs regardless of how the path is mounted, so the volume's contents are overwritten on every start. See [container.md → Volume / persistence](container.md#volume--persistence). The available route today is an external Postgres pointed at by `DB_URL`; at that point the auto-update story becomes "auto-update with persistent DB" and you've signed up for whatever schema drift the next release brings.

## Operational commands

```bash
# Tail server logs (postgres + cimmeria-server interleaved, prefixed by service name):
docker logs -f cimmeria

# Just the game server's lines:
docker logs -f cimmeria 2>&1 | grep cimmeria-server

# Force an immediate watchtower check (don't wait for the 5 min poll):
docker exec watchtower /watchtower --run-once cimmeria

# Force a manual update right now (equivalent to the above, from outside the container):
docker compose pull && docker compose up -d

# Take the box out of rotation:
docker compose down

# Bring it back:
docker compose up -d

# Wipe everything (containers, anon volumes, downloaded images):
docker compose down --volumes --rmi all
```

## Optional: Discord notifications

The cimmeria server can post lifecycle / error events (server up, server down, warnings, panics, …) to one or more Discord channels via webhooks. See [discord-notifications.md](../architecture/discord-notifications.md) for the full design and the per-event toggle catalog.

Deployment on the colo is a separate Compose overlay file generated by the release workflow:

1. **Configure GitHub Actions secrets** in the SandboxServers/Cimmeria repo settings:
   - `DISCORD_LIFECYCLE_WEBHOOK` — Discord webhook URL for the lifecycle channel (server up/down/panic).
   - `DISCORD_ERRORS_WEBHOOK` — Discord webhook URL for the errors channel (`warn!` / `error!` harvest + the gameplay error event types).

   Setting at least one of these enables overlay rendering. Setting neither (or the secrets being unavailable to forks) skips the overlay step entirely — release still completes, just without Discord wiring.

2. **Download the rendered overlay** from the latest GitHub release alongside `compose.yml`:

   ```bash
   cd /opt/cimmeria
   curl -L -o compose.discord.yml \
     "https://github.com/SandboxServers/Cimmeria/releases/latest/download/compose.discord.yml"
   ```

   The release artifact has the webhook URLs already substituted into the inlined TOML — no `.env` on the colo host, no secrets sitting on disk outside the compose file itself. Read access to the compose file == read access to the webhook URLs, which is why the deployed file should be `chmod 0600 cimmeria:cimmeria` (matches the `0440` mode the overlay uses for the in-container `/opt/cimmeria/config/discord.toml`).

3. **Bring up with both files** in a single compose invocation:

   ```bash
   docker compose -f compose.yml -f compose.discord.yml up -d
   ```

   The overlay only modifies the `cimmeria` service — it adds a `configs:` mount that writes `/opt/cimmeria/config/discord.toml` inside the container from the inlined content. Subsequent watchtower swaps continue to use the same overlay (it's a host-side configuration, not part of the image).

### Adding more channels

The overlay ships with `lifecycle` + `errors` defined. To enable additional channels (`auth`, `world`, `chat`, `gameplay`, `gm`, `ops`):

1. Add a `DISCORD_<CHANNEL>_WEBHOOK` GitHub Actions secret.
2. Extend [docker/compose.discord.yml](../../docker/compose.discord.yml) with a new `[discord.channels.<channel>]` block using a fresh `__DISCORD_<CHANNEL>_WEBHOOK__` sentinel.
3. Add a new awk-substitution arm to the render step in [.github/workflows/release-container.yml](../../.github/workflows/release-container.yml).
4. Cut a new release. The new overlay artifact will include the channel.

Channels without a `[discord.channels.X]` block are silently dropped from routing — see the `should_post` logic in [crates/discord/src/config/model.rs](../../crates/discord/src/config/model.rs) (line 86).

### Per-event toggle overrides

The rendered TOML inherits the crate's `EventToggles::default()` for every flag and overrides only what the colo needs to diverge on. The default colo overlay turns `warning = true` on (vs. `false` locally) so ops sees non-fatal regressions even when day-to-day development noise is muted elsewhere.

To adjust further, edit the `[discord.events]` block in [docker/compose.discord.yml](../../docker/compose.discord.yml) and cut a new release. cimmeria-discord supports live reload, but the colo's `configs:` mount snapshots content at container create time — so toggles only take effect on the next watchtower swap (or a manual `docker compose up -d --force-recreate`).

## Optional: watchtower notifications

Watchtower itself can ping Discord / Slack / email / Matrix via [shoutrrr](https://containrrr.dev/watchtower/notifications/) every time it swaps a container. This is independent of the cimmeria-discord crate above — watchtower notifications announce *image swaps*, while cimmeria-discord notifications announce *server events*. Set `WATCHTOWER_NOTIFICATIONS` and `WATCHTOWER_NOTIFICATION_URL` in the compose file if you want both.

## Optional: pin watchtower's image

The sample compose uses `containrrr/watchtower:latest`. For belt-and-braces, pin by digest:

```yaml
watchtower:
  image: containrrr/watchtower:1.7.1@sha256:<digest from docker hub>
```

This is the same hygiene rule the Cimmeria image itself follows — see [container.md → Image supply-chain posture](container.md#image-supply-chain-posture). Trade-off: you have to manually bump it for security fixes.

## Troubleshooting

**Container won't start / immediately exits.**

```bash
docker logs cimmeria --tail 200
```

Look for the s6 service prefix: `[cimmeria-server]` lines are the game server, `[postgres]` lines are the DB. The most common failure mode at colo is misconfigured `BASE_EXTERNAL` — players connect successfully but get kicked back to login because the BaseApp handshake hands them an unreachable address. The server itself starts fine in that case; the symptom is purely client-side.

**Watchtower isn't picking up new releases.**

```bash
docker logs watchtower --tail 50
```

Expect to see periodic "Session done" lines. If the cimmeria container isn't being inspected, check:

- `WATCHTOWER_LABEL_ENABLE=true` is set in compose AND the `com.centurylinklabs.watchtower.enable=true` label is on the `cimmeria` service.
- The package is public (it should be — see GHCR settings). If it isn't, watchtower needs credentials per its [private-registries docs](https://containrrr.dev/watchtower/private-registries/).

**Disk usage growing over time.**

Anonymous volumes from old containers should be cleaned up by `WATCHTOWER_REMOVE_VOLUMES=true`. Old images by `WATCHTOWER_CLEANUP=true`. If you see drift anyway:

```bash
docker system df          # what's using space
docker volume prune       # nuke unreferenced volumes
docker image prune -a     # nuke unreferenced images
```

## When to graduate off this setup

This pattern is right for: a single colo box, public demo or trusted-community play, no persistence requirement, no SLA. Move to something heavier when you need:

- **Persistence across updates** → switch to a named volume, accept schema drift as a cost of doing business.
- **More than one host** → orchestrator (k8s, Nomad). Watchtower doesn't coordinate across nodes.
- **Staged rollouts** → CI/CD pipeline with a canary tier, not a polling auto-updater.
- **External Postgres** → set `DB_URL` to a managed DB and run with `--external-db` (the orchestrator detects libpq-style `host=`/`port=` and skips the bundled Postgres bootstrap).

See [container.md → Threat model](container.md#threat-model--deliberate-trade-offs) for the full set of trade-offs the bundled image makes that this guide inherits.
