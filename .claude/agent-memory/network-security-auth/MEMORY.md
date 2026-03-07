# Network Security & Auth Agent Memory

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
