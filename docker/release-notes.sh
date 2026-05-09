#!/usr/bin/env bash
# Emit GitHub Release body markdown for a Cimmeria container release.
# Invoked by .github/workflows/release-container.yml.
#
# Usage: docker/release-notes.sh <version>   (e.g. 2026-05-09.1)
set -euo pipefail

VERSION="${1:?usage: $0 <YYYY-MM-DD.N>}"
# GHCR namespaces are lowercase only — the runtime owner (e.g.
# `SandboxServers`) needs to be folded before substitution or the
# rendered `docker pull` command will fail with "name unknown".
OWNER="${GITHUB_REPOSITORY_OWNER:-sandboxservers}"
OWNER=$(echo "$OWNER" | tr '[:upper:]' '[:lower:]')
IMAGE="ghcr.io/${OWNER}/cimmeria-server"
SHA="${GITHUB_SHA:-}"
SHORT_SHA="${SHA:0:7}"
# Optional audit pointer: which PR's `/release` comment kicked this off.
# Empty for manual `gh workflow run` / Actions-UI dispatches.
TRIGGERING_PR="${TRIGGERING_PR:-}"

cat <<EOF
## Cimmeria server \`${VERSION}\`

Self-contained pre-release container. Bundles \`cimmeria-server\`, cooked game
data, navmeshes, and a pre-loaded Postgres 17 with schema + seeds.
Built from commit \`${SHORT_SHA}\`$([ -n "$TRIGGERING_PR" ] && echo ", dispatched via PR #${TRIGGERING_PR}").

### Pull

\`\`\`
docker pull ${IMAGE}:${VERSION}
\`\`\`

### Run (named volume — recommended, persists characters across restarts)

\`\`\`
docker run -d --name cimmeria \\
  -p 13001:13001 \\
  -p 32832:32832/udp \\
  -p 50000:50000/udp \\
  -p 8081:8081 \\
  -p 8443:8443 \\
  -e BASE_EXTERNAL=<your-public-or-LAN-ip> \\
  -v cimmeria-data:/var/lib/postgresql/data \\
  ${IMAGE}:${VERSION}
\`\`\`

> **\`BASE_EXTERNAL\` matters.** The default \`127.0.0.1\` only reaches a client
> on the same host as the container. Override it to your machine's LAN/WAN IP
> for any other client.

### Reset to a fresh server

\`\`\`
docker rm -f cimmeria
docker volume rm cimmeria-data
docker run ...   # same flags as above
\`\`\`

### Environment reference

| Variable | Default | Notes |
|---|---|---|
| \`AUTH_HOST\` | \`0.0.0.0\` | Auth bind |
| \`AUTH_PORT\` | \`13001\` | Auth TCP (BaseApp connections) |
| \`LOGON_PORT\` | \`8081\` | Auth HTTP (SOAP login) |
| \`BASE_HOST\` | \`0.0.0.0\` | BaseApp UDP bind |
| \`BASE_EXTERNAL\` | \`127.0.0.1\` | **Override for LAN/WAN play** |
| \`BASE_PORT\` | \`32832\` | BaseApp UDP |
| \`CELL_PORT\` | \`50000\` | CellApp UDP |
| \`ADMIN_PORT\` | \`8443\` | Admin REST API |
| \`DB_URL\` | \`host=127.0.0.1 port=5432 user=w-testing password=w-testing dbname=sgw\` | Libpq-style (NOT a URL DSN — see operations docs) |
| \`DEVELOPER_MODE\` | \`true\` | Relaxed auth + multi-login |
| \`RUST_LOG\` | \`info\` | tracing-subscriber filter |

### What's inside

- \`cimmeria-server\` (Linux x86_64, glibc, built against the workspace at \`${SHORT_SHA}\`)
- \`data/cache/*.pak\` — cooked game data
- \`data/spaces/*.nav\` — navmeshes (combat AI / line-of-sight)
- Postgres 17.9 + pgdata pre-loaded from \`db/database.sql\`
- s6-overlay v3 supervising postgres + cimmeria-server

### What's NOT inside

- No Rust source, no \`target/\`, no \`.sql\` files. The DB is identity-baked.

### Security

This container exposes the same legacy Stargate Worlds protocol the standalone
server does — **do not expose it to the public internet** without a trusted
client base. See the project README for context.
EOF
