# SigNoz remote access via Cloudflare Tunnel

The SigNoz frontend speaks plain HTTP on port 3301 with no built-in
authentication. Publishing that port to the public internet would be
asking for trouble. This document covers the recommended hardening
path: terminate auth at Cloudflare's edge, run an outbound-only
tunnel from the colo box, never open an inbound firewall port.

Cloudflare Tunnel was picked over Tailscale, WireGuard, and
Caddy+Authelia because it needs no inbound firewall hole, no client
software on the viewer's machine, and no certificate management — the
`cloudflared` daemon dials outbound and Cloudflare Access handles
authentication in front of a UI that has none of its own. The service
definition is at [`docker/compose.yml:283-298`](../../docker/compose.yml)
(profile-gated behind `--profile tunnel`).

## Architecture

```text
Browser / Cimmeria-MCP
        │
        │ HTTPS to signoz.<your-domain>
        ▼
Cloudflare edge
        │
        │ Cloudflare Access policy
        │ (GitHub OAuth / email / service token)
        ▼
Cloudflare argo tunnel
        │
        │ outbound TCP from colo to *.cfargotunnel.com
        ▼
cloudflared (in colo, docker container)
        │
        │ HTTP localhost
        ▼
SigNoz frontend :3301
```

No inbound firewall ports. No Let's Encrypt automation to maintain.
Service-token auth for machine clients (the Cimmeria-MCP server is
one). Cookies + identity providers for browser users.

## Prerequisites

- A Cloudflare account (free tier — Access is free up to 50 seats).
- A domain on Cloudflare's nameservers.
- `cloudflared` installed on the machine you run the one-time setup
  commands from (the colo box itself, or your dev workstation).

## One-time setup

### 1. Authenticate cloudflared

On the machine where you'll run the setup commands:

```bash
cloudflared tunnel login
```

This opens a browser to associate `cloudflared` with your Cloudflare
account and writes a cert to `~/.cloudflared/cert.pem`.

### 2. Create the tunnel

```bash
cloudflared tunnel create cimmeria-signoz
```

This produces output like:

```text
Tunnel credentials written to /home/you/.cloudflared/abc123-de45-….json.
Created tunnel cimmeria-signoz with id abc123-de45-…
```

Note the UUID — you'll need it.

### 3. Drop credentials on the colo box

Copy the credentials JSON to the colo box at the path the compose
overlay expects:

```bash
scp ~/.cloudflared/<uuid>.json colo:/etc/cloudflared/credentials.json
```

On the colo box, also write a small `config.yml` describing what the
tunnel should route:

```yaml
# /etc/cloudflared/config.yml
tunnel: cimmeria-signoz
credentials-file: /etc/cloudflared/credentials.json
ingress:
  - hostname: signoz.<your-domain>
    service: http://frontend:3301
  - service: http_status:404
```

The `frontend:3301` hostname resolves inside the Docker network
because all services in `docker/compose.yml` share the project's
default network — no explicit network configuration needed.

### 4. Point DNS at the tunnel

```bash
cloudflared tunnel route dns cimmeria-signoz signoz.<your-domain>
```

This creates a CNAME `signoz.<your-domain>` → `<uuid>.cfargotunnel.com`.

### 5. Configure Cloudflare Access

In the Cloudflare Zero Trust dashboard:

1. **Access → Applications → Add an application → Self-hosted.**
2. Application domain: `signoz.<your-domain>`, path: `/`.
3. Add policies:
   - **Browser users**: "Allow if email matches `you@example.com`"
     (or your team's GitHub org / Google Workspace domain).
   - **Machine clients (Cimmeria-MCP)**: "Allow if service token
     equals `cimmeria-mcp-prod`". Create the service token from
     **Access → Service Auth → Service Tokens**; this generates a
     `CF-Access-Client-Id` and `CF-Access-Client-Secret` pair.

### 6. Bring up the stack

```bash
docker compose -f docker/compose.yml --profile tunnel up -d
```

The single self-contained compose file already defines the
`cloudflared` service guarded by `profiles: [tunnel]`; `--profile
tunnel` flips it on. Without the flag, the rest of the stack (game
server + SigNoz) comes up without any outbound tunnel.

The cloudflared container starts, dials Cloudflare's edge, and the
tunnel becomes routable. Hit `https://signoz.<your-domain>` —
Cloudflare Access prompts for auth, then proxies you to the SigNoz UI.

## How browser auth works

First load → Cloudflare Access sees no cookie → redirects to the
identity provider you configured (email magic link, GitHub OAuth,
Google, etc.) → on success, sets a CF Access JWT cookie scoped to the
application → subsequent requests are passed through with the JWT
attached as `Cf-Access-Jwt-Assertion` header.

The JWT is verifiable via Cloudflare's public key set if the SigNoz
backend ever needs to know who the user is — but for now we treat
Access as a black-box auth wall and let SigNoz serve all authenticated
requests as anonymous admin.

## How machine auth works (Cimmeria-MCP)

The Cimmeria-MCP server holds a service-token pair as environment
variables (set in its Azure Function App config, not in the repo):

```bash
CF_ACCESS_CLIENT_ID=abc123.access
CF_ACCESS_CLIENT_SECRET=def456…
```

Every HTTP request from Cimmeria-MCP to SigNoz attaches:

```text
CF-Access-Client-Id: ${CF_ACCESS_CLIENT_ID}
CF-Access-Client-Secret: ${CF_ACCESS_CLIENT_SECRET}
```

Cloudflare's edge validates the pair, lets the request through, and
SigNoz never sees the credentials. Revoking machine access is a single
click in the Cloudflare dashboard — "delete service token" — which
takes effect at the edge within seconds. No coordinated key rotation
across multiple systems.

Audit log entries for each service-token request show up under
**Access → Logs** in the Cloudflare Zero Trust dashboard, with the
authenticated identity column showing the service token's name.

## Cimmeria-MCP integration plan

See [docs/architecture/observability.md](../architecture/observability.md#cimmeria-mcp-integration)
for the planned MCP tools that query SigNoz over this tunnel.

The short version: Cimmeria-MCP gets two new tool families.

- `signoz_query_logs(query: string, time_range: …)` — runs a ClickHouse
  query against the `signoz_logs` table via SigNoz's REST API and
  returns structured results.
- `signoz_query_packets(filters: PacketFilters, time_range: …)` —
  same surface but specialised on the `target = "mercury.packet"`
  rows, with field-aware helpers for filtering by direction, msg_id,
  player session, etc.

These tools are implemented in the separate `Cimmeria-MCP` repository
(C# Azure Functions); only the integration plan lives here.

## Rotating credentials

| Credential | Rotation | Frequency |
|---|---|---|
| User identity provider tokens | Handled by IdP (GitHub / Google) | Per IdP policy |
| CF Access service tokens | Delete + re-create in Cloudflare dashboard, update Cimmeria-MCP env config | Annually, or on suspected compromise |
| Tunnel credentials JSON | `cloudflared tunnel delete` + `cloudflared tunnel create` (new UUID, new file, new DNS) | Rarely — only on suspected compromise of the colo box itself |

## Troubleshooting

| Symptom | Likely cause | Fix |
|---|---|---|
| Browser stuck on Cloudflare login | Identity provider blocked / cookies disabled | Try a private window; check IdP status |
| 502 Bad Gateway | Tunnel up but frontend down | `docker compose logs frontend` |
| 403 with no auth prompt | Access policy denied (e.g. email mismatch) | Check **Access → Logs** for the denial reason |
| Cimmeria-MCP getting 401 from SigNoz | Service token missing or wrong env var name | Verify both `CF-Access-Client-Id` and `-Secret` headers are attached |
| Tunnel keeps reconnecting | `cloudflared` upgrade incompatibility | Pin a specific cloudflared image tag in the overlay |

## Disabling remote access

Stop just the cloudflared service; the rest of the stack keeps
running:

```bash
docker compose -f docker/compose.yml stop cloudflared
```

Or bring the whole stack back up without `--profile tunnel`:

```bash
docker compose -f docker/compose.yml up -d
```

The local-machine UI at `http://localhost:3301` (or an SSH tunnel
forwarded equivalent) still works.
