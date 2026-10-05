---
name: reference-colo-docker-layout
description: "How the colo's Docker setup is laid out (two compose projects on signoz-net, repo docker/ mirrors it since 2026-10-04) and the traps found while syncing it"
metadata:
  type: reference
---

Checked on the colo 2026-10-04 and synced into the repo the same day (`docker/compose.yml`, `docker/signoz/`, runbook `docs/operations/colo-deploy.md`).

- **Two compose projects.** `cimmeria` (`/opt/cimmeria`: game + watchtower, overlays via `COMPOSE_FILE`) and `signoz` (`/opt/cimmeria/signoz`: SigNoz v0.125.1, collector v0.144.4, ClickHouse 25.5.6). The game reaches `otel-collector:4317` over the network `signoz-net`. Before 2026-10-04 the repo's `docker/compose.yml` still vendored SigNoz v0.55 (UI on 3301), which the colo had stopped using on 2026-05-25.
- **Watchtower recreates from the old container, not from compose files.** A compose or `.env` edit reaches `cimmeria` only through `docker compose up -d`. The 2026-10-03 `wait-for-otel.sh` mount never reached the running container for this reason.
- **s6-overlay v3 starts the user services in parallel with `/etc/cont-init.d`.** A cont-init script cannot hold `cimmeria-server`; the colo log shows the server started before `legacy-cont-init`. The collector wait is in `docker/s6/cimmeria-server/run` instead.
- **Nothing binds 13001 or 50000/udp** in the container (listeners on 2026-10-04: 8081, 8443, 8444, 30000 tcp; 32832 udp). The CellApp is in-process.
- **Player telemetry enters through `:8081/api/telemetry/*`** and the server replays it to the collector; SigNoz's 8080 and 4317/4318 carry no player traffic. From the internet on 2026-10-04, 8080 (SigNoz UI) answered and 4317/4318/8443/8444 did not; SigNoz UI clients in the previous 24 h were all on the VPN.
- **The server runs as uid 1001**; a bind-mounted `discord.toml` must be readable by uid 1001 on the host.

See also [[reference-signoz-log-mining]].
