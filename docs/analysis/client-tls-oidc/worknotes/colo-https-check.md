# CT-50 colo HTTPS read-only check

Date: 2026-10-25. Read-only: nothing was changed on any host. No IPs, no `.env` values, no account names.

## Answers

- **Is 443 free?** No. Something listens on `*:443` (and `*:80`). Nothing is bound on `:13480`.
- **What holds 443?** A `caddy` process. The `board-proxy` container (`local/board-caddy`) runs with host networking, so it binds the host's 80 and 443 directly and shows no port mapping in `docker ps`. `ss -ltnp` showed no process names without root, so this is inferred from the host-network Caddy container plus the `caddy` entry in the host process list; an `nginx` process also exists (the Discourse `app` container, bridge-mode, published only on loopback :8090).
- **Is 80 needed?** No. CT-51 uses TLS-ALPN-01, which needs only 443. 80 is also taken by the same Caddy.
- **Which overlays run?** `COMPOSE_FILE=compose.yml:compose.lab.yml` (file names, not a secret). SigNoz runs as its own project (`/opt/cimmeria/compose.signoz.yml`, network `signoz-net` exists). No HTTPS overlay yet.
- **Does `config/` exist?** Yes: `/opt/cimmeria/config` holds `discord.toml` and `wait-for-otel.sh` (no `auth-providers.toml`).
- **Is the login host a DNS name?** Yes. `default_servers()` uses the host `play.cimmeria.app` on port 8081; it resolves from the workstation, and it is the same name players use for 8081.
- **Same machine as node5?** Yes: `hostname` on `<colo-alias>` returns `node5`. There is no separate `node5` ssh alias (the colo has per-host and gateway aliases), so the comparison is by hostname only.

## Raw findings

- Compose project `cimmeria`: container `cimmeria` (image `cimmeria-server:latest-prerelease`) publishes 8081/tcp, 8443/tcp, 13001/tcp, 32832/udp, 30000/tcp, 50000/udp, and 8444/tcp on one LAN address. Not 443, not 13480.
- Other containers on the host: wisp (gamequery, bootstrapper on 2022 and 8080), gha-runners (egress proxy, runners, cache redis on loopback 6379, buildkitd, dind), signoz stack (8082, 4317-4318), board stack (app, broker, mailer, proxy), watchtower x2, agentcraft (25565).
- Docker networks: agentcraft_default, bridge, cimmeria_default, gha-runners_default, host, none, signoz-net, wisp_default, wisp_nw. `signoz-net` is present.
- `/opt/cimmeria`: compose.yml, compose.lab.yml, compose.signoz.yml, several compose.yml.bak-* files, compose.yml.save, config/, signoz/.
- `/opt/cimmeria/.env` is root-only (0600). Key NAMES read via `sudo -n`: MCP_API_KEY, POSTGRES_PASSWORD, OPENAI_ENDPOINT, OPENAI_KEY, OPENAI_EMBEDDING_DEPLOYMENT, OPENAI_CHAT_DEPLOYMENT, OPENAI_CODEX_DEPLOYMENT, SIGNOZ_NETWORK_NAME, MCP_BIND, COMPOSE_FILE, CIMMERIA_WG_IP, CIMMERIA_LAB_MCP_TOKEN, CIMMERIA_TELEMETRY_HMAC_SECRET, CIMMERIA_TELEMETRY_UPLOAD_ENDPOINT.
- Edge forwards: not checked (no gateway read commands run); the colo runbook lists 8081/tcp, 32832/udp and 30000/tcp.

## Consequence for CT-51

Host 443 is taken by the board's host-network Caddy, so a second Caddy for `/auth/v1` cannot bind host 443 on node5. CT-51 needs a decision: publish the auth Caddy on another host port with an edge forward to it, add a site block for the login host to the existing board Caddy, or move one of them. The existing `cimmeria` container does not publish 13480, which fits the plan (proxy listener inside the container only).