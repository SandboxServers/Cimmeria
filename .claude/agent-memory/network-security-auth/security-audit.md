# Security Audit: C++ vs Rust Auth Behavioral Comparison

> [!WARNING] Confidence: STALE — verify against current `crates/services/src/auth/` and `base.rs`
>
> Triaged 2026-05-13 (Phase −0.5 step 4). Captured 2026-03-04; the auth + base modules have been refactored since (auth is now a `crates/services/src/auth/` directory; mercury extension code was split out of `mercury_ext.rs`). The behavioral observations (no ticket expiration, no duplicate-account detection, non-constant-time password comparison) need to be re-verified against current code before being treated as canonical for `spec.protocol.cipher-and-auth` section 5.
>
> The deprecated C++ flow descriptions (logon_queue.cpp, shard_client.cpp, service_main.cpp) are immutable evidence and can promote to section 3 of the chapter as-is. Keep until chapter authoring.

Full audit performed 2026-03-04. See plan file for detailed findings.

## Credential Validation Flow Comparison

### C++ (logon_queue.cpp)
1. Query: `SELECT account_id, upper(password), accesslevel, enabled FROM account WHERE account_name = :accname`
2. Compare: `item->request.password != item->dbPassword` (non-constant-time, upper-cased in SQL)
3. Check: `item->enabled != "t"`
4. Return: accountId + accessLevel

### Rust (auth.rs validate_credentials)
1. Query: `SELECT account_id, password, enabled FROM account WHERE account_name = $1`
2. Compare: `row.password.to_lowercase() != client_password_hash.to_lowercase()` (non-constant-time)
3. Check: `!row.enabled`
4. Return: account_id only (no accessLevel)

### Differences
- Rust omits accessLevel -- no protected shard support
- C++ uppercases in SQL, Rust lowercases in Rust -- functionally equivalent
- Both use non-constant-time comparison

## Ticket Lifecycle Comparison

### C++ (shard_client.cpp)
- ShardLogonQueue::logonClient() stores ticket with expiration timestamp
- ShardLogonQueue::checkExpiration() runs before every logon/openSession
- ShardLogonQueue::openClientSession() validates ticket + removes it (consume once)

### Rust (auth.rs + base.rs)
- PendingLogin stored in HashMap<String, PendingLogin> with no expiration
- take_pending_login() removes ticket on consumption (consume once)
- No periodic cleanup of unclaimed tickets

## Duplicate Account Detection Comparison

### C++ (service_main.cpp)
- onlineAccounts_ map: accountId -> FrontendConnection
- accountOnline(): if account already connected, kick BOTH connections
- accountOffline(): remove from map, handle shard mismatch

### Rust
- No equivalent. Connected map keyed by SocketAddr, not account_id.
- Multiple simultaneous logins with same account_id are accepted.

## Inactivity Timeout Comparison

### C++ (channel.cpp)
- InactivityTimeout loaded from config (default 10000ms, config overrides to 300000ms)
- Checked every tick: `nub_.lastTick() - lastPeerActivity_ > InactivityTimeout`
- Keepalive sent when server is idle for InactivityKeepaliveInterval

### Rust (base.rs)
- INACTIVITY_TIMEOUT = 60s (hardcoded constant)
- Checked in tick-sync loop: `last_recv.elapsed() > INACTIVITY_TIMEOUT`
- Tick-sync itself serves as keepalive (every 100ms)
