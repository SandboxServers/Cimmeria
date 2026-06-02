---
name: security-audit-2026-05-31
description: Red-team audit findings against main commit 14827c9c. Production-deployment scope, excludes dev-mode and seed-account issues.
metadata:
  type: project
---

# Red-team audit snapshot — 2026-05-31 (commit 14827c9c)

Ten substantive findings filed. Severity ordering and one-line gist for each:

1. **Critical** — Admin API on `0.0.0.0:8443` has zero auth middleware. `middleware.rs` has only a CORS layer; `/api/auth/login` is a stub returning `{"status":"ok"}` and a null token. Routes like `/api/config/stop`, `/api/editor/content` (DB-mutating), `/api/players/{id}/kick` are world-callable.
2. **Critical** — `/ws/logs` WebSocket streams every tracing event ≥DEBUG with no auth. `auth/handlers.rs:154` logs full SID at debug; `:249` logs full ticket at debug. Attacker connects to WS, races legitimate user to Phase 2/3 with stolen SID+ticket.
3. **High** — `/api/auth/dev-session` mints 8h HMAC-SHA256 tokens to any caller, `sub` claim is attacker-controlled `install_id`. Token grants `telemetry.write` which replays through `tracing::info!` → poisons SigNoz with arbitrary "client" events.
4. **High** — `PendingLogin` (auth/mod.rs:100) and `SessionRecord` are not bound to client IP. Phase 2 SID and Phase 3 ticket can be claimed from any source address. Duplicate-login eviction (base/login/mod.rs:67-114) helps the attacker (legitimate client gets kicked).
5. **High** — Phase 1 SOAP at `http://0.0.0.0:8081/SGWLogin/UserAuth` is plain HTTP. No `rustls`/`tokio-rustls` anywhere in auth path. SHA-1 password hash is THE credential (server compares hash-vs-hash) — captured hash = account takeover.
6. **Medium** — Password storage is unsalted SHA-1 (matching C++); compare uses `String::to_uppercase()` then `!=` short-circuit (handlers.rs:481). Both rainbow-tableable on DB leak and remotely timing-attackable.
7. **Medium** — Mercury cipher: zero IV per packet, same key for AES-256-CBC and HMAC-MD5 (encryption.rs:50-56, 78-84). Deterministic ciphertext enables traffic analysis; packet replay across window slides is possible.
8. **Medium** — `session_key: String` in `PendingLogin` (auth/mod.rs:107) — no zeroize wrapper. Key crosses Phase 2 wire in cleartext XML; HTTP capture = full decryption.
9. **Medium** — `format!`-based XML builders in handlers.rs:374, 397, 414 have no entity escaping. Shard name/host injection if those fields ever take network input.
10. **Low** — CORS `Any/Any/Any` (middleware.rs:12-17) + admin on `0.0.0.0` + no WS Origin check = drive-by CSRF/exfil from any operator-visited webpage.

## Re-verification notes for security-audit.md

The old [security-audit.md](security-audit.md) flagged 4 items as `OPEN`:
- developer_mode default: **FIXED** — `config.rs:83` now defaults to `false` (Phase 0.5 hardening).
- session_key logged at DEBUG in `auth.rs:370`: **FIXED** for session_key specifically (no longer logged), but **REGRESSED** in spirit — SID and ticket are still logged at debug (findings #2 above).
- No ticket expiration: **FIXED** — `service.rs:146-171` runs a reaper task with `TICKET_TTL = 30s` and `SESSION_TTL = 300s`.
- No duplicate-account detection: **PARTIALLY FIXED** — `base/login/mod.rs:67-114` evicts duplicate sessions at Phase 3, but this enables the hijack in finding #4 rather than preventing it.
- Non-constant-time password comparison: **STILL OPEN** (finding #6).
- requestCharacterVisuals account_id filter: **out of scope this audit**; verify separately.

## Things that look like findings but are NOT exploitable

- Editor `format!("DELETE FROM {table} WHERE chain_id = $1")` uses a hardcoded table-name allowlist — not SQL injection.
- `entity_stream.rs` is a TODO no-op handler — no actual leak yet.
- C++ Python console (port 8989) has no Rust implementation; the doc at `docs/architecture/python-console.md` describes C++ behavior only.
- `tracing::trace!(body = %body, "Phase 1 raw SOAP request")` at handlers.rs:44 — trace level is below BroadcastLayer's DEBUG filter (server/main.rs:531), so it doesn't reach `/ws/logs`. Still a disk-log concern but not a network-reachable leak.

## Files that need attention before next audit

- Re-verify whether finding #4 fix lands as IP-binding or as session_key challenge-response.
- Watch `crates/admin-api/src/middleware.rs` — if JWT middleware lands, re-verify it actually wraps all routes (not just `/api/*` nested router missing some sub-routes).
- The BroadcastLayer field-deny pattern (if implemented per finding #2) should be tested with a unit test that asserts known-sensitive field names never round-trip through it.
