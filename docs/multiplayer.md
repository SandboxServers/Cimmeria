---
title: "Multiplayer / LAN Setup"
type: how-to
audience: operators, players
last_updated: 2026-07-25
---

# Multiplayer / LAN Setup

## Key Environment Variable: BASE_EXTERNAL

`BASE_EXTERNAL` controls the IP address the auth server tells game clients to connect to for the BaseApp (Mercury UDP, port 32832). Default is `127.0.0.1` (localhost only).

For LAN play, set it to the server machine's LAN IP before starting:

```powershell
$env:BASE_EXTERNAL = "10.10.10.XXX"
```

**This is NOT a bind address** — it's the IP embedded in the Phase 2 XML response (`ServerLocation` element). The client literally connects to this IP. Setting it to `0.0.0.0` will NOT work.

The server already binds to `0.0.0.0` by default (no allow list), so any machine on the network can connect once `BASE_EXTERNAL` points to the right IP.

## Env Var Inheritance with Supervisor

If using the supervisor (admin panel stop/start), the child server process inherits env vars from the supervisor's PowerShell session. To change `BASE_EXTERNAL`:
1. Stop the supervisor
2. Set the env var in PowerShell
3. Restart the supervisor from that same window

## Login Flow

Full multiplayer session verified with a remote client:

1. **Phase 1** (HTTP/SOAP on port 8081) — credential check
2. **Phase 2** (HTTP/SOAP on port 8081) — shard selection, returns BaseApp IP:port from `BASE_EXTERNAL`
3. **Phase 3** (Mercury UDP on port 32832) — `baseAppLogin` with ticket, encrypted channel established
4. **Phase 4** — Character list + cooked data version checks (20 PAK categories)
5. **Phase 5a** — `playCharacter` → RESET_ENTITIES
6. **Phase 5b-A** — CREATE_BASE_PLAYER + onClientMapLoad
7. **Phase 5b-B** — Client sends mapLoaded → VIEWPORT + CELL + POSITION + entity data (fragmented)
8. **In-world** — AoI position updates streaming at ~10Hz

## Login Audit

- Auth handlers emit `LoginEvent`s at every exit point with client IP
- Real-time WebSocket stream at `/ws/events` (dashboard updates automatically)
- REST endpoint: `GET /api/audit/logins` with cursor pagination and filters (`?account_name=`, `?outcome=`, `?before=`, `?limit=`)
- DB persistence: `login_audit` table — run `db/scripts/add_login_audit.sql` on existing databases
- Dashboard "Recent Logins" card shows live events without page refresh

## Ports Required for Remote Clients

| Port  | Protocol | Service              | Notes                              |
|-------|----------|----------------------|------------------------------------|
| 8081  | TCP/HTTP | Auth (SOAP login)    | Phases 1 & 2                       |
| 13443 | TCP/HTTPS| Auth (SOAP over TLS) | Optional — see below               |
| 32832 | UDP      | BaseApp (Mercury)    | Phases 3-5, gameplay               |
| 8443  | TCP/HTTP | Admin API            | Dashboard / REST / WebSocket (opt) |

Defaults come from `ServerConfig` ([crates/common/src/config.rs](../crates/common/src/config.rs) L127-143).

## Optional: TLS on the auth listener

The auth service can serve the same SOAP router over HTTPS on `auth_tls_port` (default **13443**) alongside the plaintext listener. It only starts when **both** `auth_tls_cert_path` and `auth_tls_key_path` are set; otherwise the server stays HTTP-only, which is the default.

A background watcher polls the cert and key mtimes every `auth_tls_reload_interval_secs` and hot-swaps the live `rustls::ServerConfig` when they change. The swap happens only after a *successful* rebuild, so a half-written cert file mid-renewal leaves the previous config serving and is retried on the next tick.

The stock game client does not use this listener — it is for tooling and proxies in front of the auth endpoint.
