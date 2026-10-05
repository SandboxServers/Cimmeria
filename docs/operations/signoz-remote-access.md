# SigNoz remote access via Cloudflare Tunnel

> **Status (checked 2026-10-04): not deployed.** The colo runs no
> `cloudflared` container, service or config. Operators reach the
> SigNoz UI over the private network (VPN). This guide is the plan for
> reaching it without the VPN; the commands below are untested on the
> colo.

The SigNoz UI and query API speak plain HTTP on port 8080 (the
`signoz` service in [`docker/signoz/compose.yaml`](../../docker/signoz/compose.yaml)).
SigNoz has its own login, but publishing that port to the internet
still puts the whole query API one password (or one leaked JWT
secret) away from anyone. This document covers the hardening path:
terminate auth at Cloudflare's edge, run an outbound-only tunnel from
the colo box, never open an inbound firewall port.

Cloudflare Tunnel was picked over Tailscale, WireGuard, and
Caddy+Authelia because it needs no inbound firewall hole, no client
software on the viewer's machine, and no certificate management — the
`cloudflared` daemon dials outbound and Cloudflare Access puts a
second authentication layer in front of SigNoz's own. It runs as a
standalone container on the `signoz-net` network; neither compose
project defines it.

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
cloudflared (in colo, docker container on signoz-net)
        │
        │ HTTP over signoz-net
        ▼
signoz (UI + query API) :8080
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

Copy the credentials JSON to the colo box at the path the container
mounts:

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
    service: http://signoz:8080
  - service: http_status:404
```

The `signoz` hostname resolves because the `cloudflared` container
joins the `signoz-net` network (step 6).

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

### 6. Start cloudflared

Run it as a standalone container on `signoz-net`, with a pinned image
tag (check [cloudflared releases](https://github.com/cloudflare/cloudflared/releases)
for the current one):

```bash
docker run -d --name cimmeria-cloudflared --restart unless-stopped \
  --network signoz-net \
  -v /etc/cloudflared:/etc/cloudflared:ro \
  cloudflare/cloudflared:<pinned tag> \
  tunnel --no-autoupdate run --config /etc/cloudflared/config.yml cimmeria-signoz
```

The game server and SigNoz don't depend on it; start and stop it on
its own. The container dials Cloudflare's edge, and the tunnel becomes
routable. Hit `https://signoz.<your-domain>` —
Cloudflare Access prompts for auth, then proxies you to the SigNoz
login page.

Once this works, the edge no longer needs to forward `8080`; set
`SIGNOZ_UI_BIND` back to `127.0.0.1` or the LAN/VPN address.

## How browser auth works

First load → Cloudflare Access sees no cookie → redirects to the
identity provider you configured (email magic link, GitHub OAuth,
Google, etc.) → on success, sets a CF Access JWT cookie scoped to the
application → subsequent requests are passed through with the JWT
attached as `Cf-Access-Jwt-Assertion` header.

The JWT is verifiable via Cloudflare's public key set if the SigNoz
backend ever needs to know who the user is. SigNoz doesn't check it;
behind Access, users still sign in to SigNoz with their own SigNoz
account.

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

Cloudflare's edge validates the pair and lets the request through.
SigNoz never sees those credentials; the request still needs a SigNoz
API key (created in the SigNoz UI) for SigNoz's own auth. Revoking machine access is a single
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
| 502 Bad Gateway | Tunnel up but SigNoz down, or `cloudflared` not on `signoz-net` | `docker logs signoz --tail 50`; `docker network inspect signoz-net` should list `cimmeria-cloudflared` |
| 403 with no auth prompt | Access policy denied (e.g. email mismatch) | Check **Access → Logs** for the denial reason |
| Cimmeria-MCP getting 401 from SigNoz | Service token missing or wrong env var name | Verify both `CF-Access-Client-Id` and `-Secret` headers are attached |
| Tunnel keeps reconnecting | `cloudflared` upgrade incompatibility | Pin a specific cloudflared image tag in the `docker run` command |

## Disabling remote access

Stop and remove the container; the game server and SigNoz keep
running:

```bash
docker rm -f cimmeria-cloudflared
```

The UI stays reachable on its `SIGNOZ_UI_BIND` address, or through an
SSH tunnel (`ssh -L 8080:127.0.0.1:8080 <host>`, then
`http://localhost:8080`).
