# Network Security & Auth Agent Memory

## Phase −0.5 triage status (2026-05-13)

- [security-audit.md](security-audit.md) — **[RE-VERIFY]** — captures behavioral comparison from 2026-03-04; needs re-verification against current Rust before promoting to `spec.protocol.cipher-and-auth` section 5. The deprecated C++ half is immutable and chapter-ready.

Inline-content section status:

- **Auth Architecture (Rust Rewrite)** — **[RE-VERIFY]** — file path `crates/services/src/auth.rs` is now a directory (`crates/services/src/auth/` with mod.rs/service.rs/handlers.rs). Update before chapter authoring. The C++ reference paths are correct (they were rewritten to `deprecated/` in the mechanical pass).
- **Known Security Issues (as of 2026-03-04)** — **[RE-VERIFY]** — the "OPEN" issues are snapshots; verify each against current `crates/services/src/` before treating as live. The FIXED items can promote into the chapter's section-5 "verified-against-section-3" notes.
- **Protocol Notes** — **[PROMOTE → spec.protocol.cipher-and-auth]** — V5-confirmed against `findings/mercury-protocol-internals.md` (AES-256-CBC + HMAC-MD5, zero IV, no KDF). The 4-phase flow description (SOAP Phase 1+2, unencrypted Phase 3, encrypted Phase 4+) is canonical and overlaps with `findings/world-entry-pipeline.md` Phase 1–4. Cross-link.
- **C++ BWMailBox is dynamic; Rust hardcodes "1"** — **[RE-VERIFY]** — verify whether the Rust hardcode is still in place; if it is, that's a section-5 gap to flag during chapter authoring.

## Auth Architecture (Rust Rewrite)

### Key Files
- `crates/services/src/auth.rs` -- SOAP login (Phase 1+2), credential validation, session/ticket management
- `crates/services/src/base.rs` -- Mercury UDP (Phase 3+), encrypted channel, tick-sync, entity lifecycle
- `crates/services/src/orchestrator.rs` -- Service wiring, DB pool distribution
- `crates/common/src/config.rs` -- ServerConfig defaults

### C++ Reference Files
- `src/authentication/logon_queue.cpp` -- DB credential validation (SOCI)
- `src/authentication/shard_client.cpp` -- Ticket generation, expiration, key exchange
- `src/authentication/service_main.cpp` -- Duplicate account detection (onlineAccounts_), session registry
- `src/authentication/frontend_connection.cpp` -- Shard registration, protected shard access
- `src/authentication/logon_connection.cpp` -- HTTP SOAP handler, Phase 1+2 flow
- `src/mercury/channel.cpp` -- Inactivity timeout (configurable via client_inactivity_timeout)
- `config/BaseService.config` -- C++ defaults: developer_mode=false, inactivity=300000ms

## Known Security Issues (as of 2026-03-04)
See [security-audit.md](security-audit.md) for full details.

### OPEN -- High Priority
- **developer_mode defaults to true** in Rust (config.rs:79), C++ defaults to false
- **Session key logged at DEBUG** in auth.rs:370 (exposes AES-256 key)

### OPEN -- Medium Priority
- **No ticket expiration** -- pending_logins HashMap entries never expire
- **No duplicate-account detection** -- no onlineAccounts_ equivalent
- **Non-constant-time password comparison** -- auth.rs:561, same weakness in C++
- **requestCharacterVisuals has no account_id filter** -- base.rs:1064

### FIXED
- DB credential validation implemented (validate_credentials in auth.rs)
- Inactivity timeout implemented (60s hardcoded in base.rs, C++ uses 300s from config)
- account_id no longer hardcoded (comes from DB, fallback=1 only in dev mode without DB)

## Protocol Notes
- Phase 1+2: HTTP/SOAP (no TLS), SID cookie for session binding
- Phase 3: Unencrypted UDP baseAppLogin, ticket consumed from pending_logins
- Phase 4+: AES-256-CBC encrypted Mercury UDP, HMAC-MD5 integrity
- C++ BWMailBox is dynamic (from shard_client FES_LOGON_ACK); Rust hardcodes "1"
- C++ has accessLevel field tracked through login flow; Rust omits it entirely
