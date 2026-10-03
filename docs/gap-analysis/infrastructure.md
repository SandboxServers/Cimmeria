---
title: "Gap Analysis: Infrastructure Systems (§1-§4)"
type: explanation
audience: engineers
last_updated: 2026-10-03
companion_docs:
  - ../gap-analysis.md
  - ../project-status.md
---

# Gap Analysis: Infrastructure Systems (§1-§4)

> Part of the [Gap Analysis](../gap-analysis.md), split out of it on 2026-10-03 with no change to any row. The status taxonomy, the evidence bar and the Summary Completion Matrix are in the main file; each matrix row counts the feature rows of its section here, so change both together.

## Infrastructure Systems (Solid)

### 1. Authentication and Login --- CW

- **Confidence**: HIGH (re-read 2026-09-25: `auth/`, `base/login/mod.rs`, `server/main.rs` audit writer, PRs #698/#738)
- **Documentation**: [connection-flow.md](../connection-flow.md), [protocol/login-handshake.md](../protocol/login-handshake.md), [architecture/negative-logging-convention.md](../architecture/negative-logging-convention.md) (credential-field rule and cross-IP seams)
- **Rust code**: [`crates/auth/src/auth/`](../../crates/auth/src/auth/): 3,382 lines across 9 files (handlers.rs, credentials.rs, credential_log_guard.rs, login_smoke.rs, mod.rs, service.rs, tls.rs, tls_smoke.rs, cert_watcher.rs), including a live-DB login smoke. Also [`crates/auth/src/credential_redaction.rs`](../../crates/auth/src/credential_redaction.rs) (`CredentialPrefix`) and the duplicate-login eviction in [`crates/base/src/base/login/mod.rs`](../../crates/base/src/base/login/mod.rs)
- **Recent PRs**: #414 (auth + base + world-entry instrumentation), #366 (dev-session telemetry HMAC), #566 (auth TLS listener, argon2id credentials), #577 (cert mtime watcher + hot reload), **#698 (SIDs, tickets and the Phase 1 SOAP body no longer logged in full, #440)**, **#738 (login sessions bound to the issuing client IP, warn-only, #442)**, **#740 (dev-session mint quota + bounded refresh chain, #441)**
- **Path forward**: argon2id already exists for the patched TLS client (`credentials.rs`, opportunistic migration from SHA-1 on plaintext login); the stock client still sends an unsalted SHA-1 hash, so that path cannot change without a client patch. Remaining work: login rate limiting; harden the #738 cross-IP seams from WARN to reject once the false-positive rate is measured; per-tick authenticate validation (#294); XML entity escaping in the auth responses (#447); add `plaintext_requires_tls` to the `login_audit.outcome` CHECK constraint (see Login audit row).

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| SOAP login endpoint | CW | -- | auth/handlers.rs | HTTP POST /SGWLogin/UserAuth |
| Password validation | CW | -- | auth/handlers.rs, auth/credentials.rs | Stock client: SHA-1 hash compared against `account`. Patched client over TLS: plaintext verified against argon2id (`password_algo = 2`), with SHA-1 accounts migrated on login (credentials.rs:1-21) |
| Shard list response | CW | -- | auth/handlers.rs | Returns shard name + key |
| Shard key exchange | CW | -- | auth/service.rs | Symmetric key for Mercury session |
| Session establishment | CW | -- | base/login | BaseApp accepts after auth, login_smoke verified |
| Login audit logging | CW | -- | server/main.rs:299-330 (`audit_writer_loop`), db/sgw/Audit/Tables/login_audit.sql | `login_audit` table, 6 outcome types for the stock-client path. Since #698, SIDs and tickets are logged only as a ≤6-char `sid_prefix`/`ticket_prefix`. Latent gap, re-verified 2026-09-25: handlers.rs:111 emits outcome `plaintext_requires_tls`, which the table's `outcome_check` constraint does not list, so that event fails to insert (logged at WARN, main.rs:320). Only the patched-client TLS path can reach it |
| Duplicate login prevention | NT | -- | base/login/mod.rs:89-135 | **Re-verified 2026-09-25 (was IM).** The old note ("checked at char-select, not continuous") was wrong. Phase 3 finds any existing session for the same `account_id`, sends `LOGGED_OFF` to the old address, and tears its entities down (`destroy_client_entities(..., "duplicate_login")`). Fan-out test at base/login/tests.rs:516. KI-7 is marked RESOLVED in known-issues.md. No two-client live record |
| Developer mode bypass | CW | -- | config | Allows duplicate logins, max access level |
| Dev-session telemetry token | CW | -- | server/main.rs, admin-api/routes/dev_session/ | `CIMMERIA_TELEMETRY_HMAC_SECRET` HMAC-signed token. #740 adds a per-account mint quota and a bounded refresh chain (dev_session/quota.rs) |
| TLS auth listener + cert hot-reload | IM | -- | auth/tls.rs, auth/cert_watcher.rs | `TlsCertStore::reload` swaps only on successful rebuild; PRs #566/#577. The owner confirmed on 2026-06-20 that the stock-client (non-TLS) path still works. The TLS path is not client-tested, and its `plaintext_requires_tls` audit row cannot be persisted (see above) |
| Continuous auth validation | KM | -- | -- | Only checked at login. Per-tick authenticate token validation is open as #294. #738's IP binding is a one-time check at SID/ticket consumption, not continuous |
| Login rate limiting | KM | -- | -- | No brute-force protection. No limiter anywhere in `auth/` (re-verified 2026-09-25) |
| Cross-IP session binding | IM | -- | auth/mod.rs (`client_ip`, `client_ips_match`), auth/handlers.rs, base/login/mod.rs | **New 2026-09-25.** #738: `SessionRecord` and `PendingLogin` record the issuing IP. Phase 2 SID and Phase 3 ticket consumption log WARN `session_ip_mismatch` / `ticket_ip_mismatch` on a different source IP, with IPv4-mapped-IPv6 normalised. **Warn-only by design** (NAT/dual-stack caveat); it does not reject yet. LogCapture guards were shown to fail on revert |

### 2. Mercury Protocol --- CW

- **Confidence**: HIGH (re-read 2026-09-25: `encryption/mod.rs`, `channel/channel_core.rs`, `channel_bundle/bundle/mod.rs`, `packet/mod.rs`, `lib.rs`; open issue #733)
- **Documentation**: [drafts/spec/mercury-wire-format.md](../drafts/spec/mercury-wire-format.md) (canonical, in-progress bible chapter), [protocol/mercury-wire-format.md](../protocol/mercury-wire-format.md) (legacy summary), [architecture/transport-trait.md](../architecture/transport-trait.md), [architecture/mercury-bundle.md](../architecture/mercury-bundle.md), [architecture/mercury-loopback-harness.md](../architecture/mercury-loopback-harness.md), [architecture/network-chaos-testing.md](../architecture/network-chaos-testing.md), [audits/mercury-rust-conformance-2026-05-15.md](../audits/mercury-rust-conformance-2026-05-15.md)
- **Rust code**: [`crates/mercury/`](../../crates/mercury/): 14,194 lines across 66 files, **265 tests** (bundle, channel/, channel_bundle/, clock, codec, encryption/, instrumentation, lossy_transport, messages, packet/, test_harness/, test_transport, transport, unified, unpacker/)
- **Recent PRs**: #358 (Transport trait), #361 (ChannelBundle), #363/#365 (bundle progression), #370 (loopback harness), #374 (network chaos), #404 (wire-log capture), #410 (backpressure), #415 (warn! on unhandled dispatch), #566 (v2 crypto foundation), #575 (v2 session-key rotation), **#704 (Account typeID stays 0x07 = clientIndex, guarded by a test; #313 closed as invalid)**, **#711 (inactivity constants split: `MERCURY_PEER_DEAD_MS` = 300 s, documented client-side `UE3_INACTIVITY_TIMEOUT_MS` = 15 s; tick_sync reap stays 60 s)**, **#708 (comment hygiene)**, **#716 (flaky tx-window-overflow chaos scenario fixed, #713)**
- **Path forward**: FLAG_PIGGYBACK sub-packets (0x02) are still unsupported. Mercury v2 needs a patched client to verify against, because no shipping client speaks it (`encryption/mod.rs` `#[default]` at L136 is v1). Make `Bundle::encode` refuse or fragment payloads over 64 KiB instead of writing a bare `0xFFFF` escape sentinel (#733, latent). Wider TX window needs a client patch (#353). Promote the bible chapter from draft to verified.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Reliable UDP transport | CW | -- | mercury/channel/, mercury/channel_bundle/ | Sequence numbers, ACK/NAK, retransmit |
| AES-256-CBC encryption (v1) | CW | -- | mercury/encryption/mod.rs | The stock client speaks AES-256-CBC + HMAC-MD5 (the old audit's Blowfish claim was wrong). v1 is the `#[default]` (encryption/mod.rs:136). **Do not change v1 output bytes** |
| Message framing | CW | -- | mercury/bundle.rs | Header + variable-length body, per-channel sequence. Latent (open #733, 2026-09-19): `Bundle::encode` clamps a >64 KiB payload to `0xFFFF` (bundle.rs:96) without the 32-bit escape length the client expects. No current caller sends a payload that large |
| Ordered delivery | CW | -- | mercury/channel/ | Per-channel sequence |
| Fragmentation + reassembly | CW | -- | mercury/lib.rs | Large message splitting + reassembly |
| ChannelBundle accumulator | CW | -- | mercury/channel_bundle/ | Cross-entity bundling, AoI burst migration |
| Transport trait + TestTransport | CW | -- | mercury/transport.rs | Wire seam for byte-exact fan-out tests |
| LossyTransport (chaos) | CW | -- | mercury/lossy_transport.rs | Drop / dup / reorder / latency primitives |
| Loopback session harness | CW | -- | mercury/test_harness/ | Tier 2 paired-channel end-to-end tests |
| Pcap replay (wireclient) | KM | Wireclient socket loop | -- | Re-verified 2026-09-25: unchanged. `crates/wireclient` has no `UdpSocket` (sole hit is a doc comment, handshake.rs:92). See §34 |
| Observability instrumentation | CW | -- | mercury/instrumentation.rs | Per-packet OTLP spans, SigNoz integration |
| Mercury v2 encryption | IM | -- | mercury/encryption/mod.rs | Per-packet random IV, HKDF-SHA256-split enc/mac keys (`v2_derive_keys`, mod.rs:179), 16-byte-truncated HMAC-SHA256, v1→v2 downgrade defense. **Untested against a live client**: opt-in only, and the owner confirmed on 2026-06-20 that v2 is untested |
| Mercury v2 session-key rotation | IM | v2 | mercury/encryption/rotation.rs (re-exported at mod.rs:68-75) | Server-initiated rotation, v2 sessions only; harness coverage in test_harness/tests/rotation.rs. Same caveat: not tested against a client |
| Cumulative ACKs | IM | -- | mercury/channel/channel_core.rs:434 | `process_acks` drains the TX window plus the unsent queue in one pass. #716 hardened the chaos scenario that exercises it. No other change |
| Piggyback ACKs | KM | -- | mercury/packet/mod.rs:58-59 | Re-verified 2026-09-25, clarified. ACKs already ride outgoing data packets (`ChannelBundle::add_ack`, channel_bundle/bundle/mod.rs:24-27, 60-64). What is missing is BigWorld's `FLAG_PIGGYBACK` (0x02) sub-packets, which are documented as "not supported by Cimmeria" |

### 3. Game Data Pipeline (Cooked Data + Resources) --- CW

- **Confidence**: HIGH (re-read 2026-09-25: `cooked_data.rs` version-negotiation branches, `resources/mod.rs` category map, `sequence_overrides.rs`; PRs #736/#754/#755/#767)
- **Documentation**: [engine/cooked-data-pipeline.md](../engine/cooked-data-pipeline.md), [engine/cooked-data-pak-format.md](../engine/cooked-data-pak-format.md), [game-data.md](../game-data.md), [architecture/mission-pak-overrides.md](../architecture/mission-pak-overrides.md), [analysis/ring-transport-cellblock-castle/README.md](../analysis/ring-transport-cellblock-castle/README.md) (sequence-override rationale), [analysis/dialog-ui-redesign/work-packets.md](../analysis/dialog-ui-redesign/work-packets.md) (DU-01 patch mode)
- **Rust code**: [`crates/base-session/src/base/cooked_data.rs`](../../crates/base-session/src/base/cooked_data.rs) (469), [`crates/resources/src/base/resources/`](../../crates/resources/src/base/resources/) (1,910), [`mission_overrides.rs`](../../crates/resources/src/base/mission_overrides.rs) (549), [`item_overrides.rs`](../../crates/resources/src/base/item_overrides.rs) (311), [`sequence_overrides.rs`](../../crates/resources/src/base/sequence_overrides.rs) (169, new), [`dialog_overrides/`](../../crates/resources/src/base/dialog_overrides/) (3,380)
- **Recent PRs**: #250 (equip-from-inventory PAK override pattern), #399 / #405 (server-side stacking + Slappack PAK override), **#736 (`behavior_event` registered at category 21, not 22, #267)**, **#754 (hotfix: Kismet sequence PAK back to version 7455 after #753's bump wiped every client's sequence table)**, **#755 (per-key Kismet sequence overrides for category 1)**, **#767 (patch-mode dialog overrides that emit buttons, DU-01)**
- **Path forward**: Hot reload of the override caches without a restart. Close the destructive fallback in `cooked_data.rs` branch 4 (L68): a category with no override list answers any version mismatch with `invalidate_all = true` and pushes nothing, which the client persists as an empty cache (the #754 incident). Either never invalidate-all without a push, or wire the unused `send_category_resources`. Client-test DU-01 patch mode (DU-UAT) and sequences 10187/10188 (ring transport Phase 1).

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Resource loading from DB | CW | -- | base/resources/ | 21 wire categories (client 1–21), 112,626 DB rows. Re-verified 2026-09-25: category 21 (`CookedBehaviorEvents.pak`, a stub PAK with 0 elements) was only registered by #736 (`CATEGORY_BEHAVIOR_EVENTS = 21`, resources/mod.rs:153,164). Before that, the Rust map stopped at 20 |
| Client version sync | CW | -- | base/cooked_data.rs | versionInfoRequest() handled. Latent hazard (#754 body, re-verified 2026-09-25): branch 4 (cooked_data.rs:68) invalidates the whole category and pushes nothing when there are no scoped overrides. A mis-bumped PAK therefore empties the client's cache permanently. This happened once on the colo and was repaired by hand |
| Cooked data (.pak) serving | CW | -- | base/cooked_data.rs | Binary pak files in data/cache/. `committed_paks` tests pin the Kismet PAK at 7455 (#754) |
| Mission PAK overrides | CW | -- | base/mission_overrides.rs | Injects new steps without reshipping pak |
| Item PAK overrides | CW | -- | base/item_overrides.rs | Health Slappack stack-size override (PR #399/#405) |
| InvalidKeys handshake | CW | -- | base/cooked_data.rs | Content-derived metadata bump |
| Hot reload at runtime | KM | -- | -- | Re-verified 2026-09-25: PAK and override caches still load only at startup. The admin API's `/api/content/reload` reloads the content engine, not cooked data |
| Kismet sequence overrides | NT | -- | base/sequence_overrides.rs | **New 2026-09-25.** #755: category 1 gets an in-memory override list, so a connecting client receives `InvalidKeys = [10187, 10188]` plus the two elements, and the on-disk PAK stays byte-identical to the client's copy. Guard tests: `kismet_sequences_take_the_per_key_handshake`, `on_disk_kismet_sequence_pak_is_the_client_shipped_file`. The in-client `.net_seq 10187 3` test is still pending (ring-transport README "Phase 1 status") |
| Dialog override patch mode | NT | -- | base/dialog_overrides/ | **New 2026-09-25.** #767 (DU-01): patch-mode overrides that edit a shipped dialog and emit buttons, instead of replacing it wholesale. The dialog-ui ledger (work-packets.md:7) says Wave 0/1 merged 2026-09-25 and "none of it has been tested in the client yet" |

### 4. Database Persistence --- CW

- **Confidence**: HIGH (re-read 2026-09-25: grep for `sqlx::query*!` macros, `require_db_or_skip!` invocations, pool construction, PR #756)
- **Documentation**: [architecture/service-architecture.md](../architecture/service-architecture.md), [architecture/integration-test-infra.md](../architecture/integration-test-infra.md), [../db/README.md](../../db/README.md)
- **Rust code**: **sqlx 0.9** (bumped from 0.8 by #597) throughout `crates/services/`: **118 files in `crates/services/src` use `sqlx::query`** (125 across `crates/`), all runtime-checked, none use the compile-time `query!` macros. Durable outbox at [`crates/base-session/src/base/outbox/`](../../crates/base-session/src/base/outbox/) (1,214 lines). Pool: `PgPool::connect` in [`crates/services/src/database.rs`](../../crates/services/src/database.rs):52
- **Recent PRs**: #403 (reseed pgdata on container start), #355 (cell_event_outbox infra), #366 (live-DB harness), #422 (cell_dispatch + executor live-DB coverage), **#597 (sqlx 0.8.6 → 0.9.0)**, **#702 (live-DB tests fail instead of skipping when `DATABASE_URL` is set but unreachable, #615)**, **#746 (live-DB container load recipe fixed, #742)**, **#756 (player position persisted on logout via `CellToBaseMsg::PersistPosition`)**
- **Path forward**: Pool tuning under load (production pool uses sqlx defaults). No migration framework (seeds in `db/resources/` plus a handful of manual `db/scripts/`). Adopting compile-time `query!` checking would need `.sqlx` offline data in CI. Client-test logout-position persistence (#756).

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Player state persistence | CW | -- | services/cell/cell_methods/player, base/world_entry/cell_dispatch/position.rs | Level, XP, position, stats, training points. Re-verified 2026-09-25: until #756, **logout never persisted position** (only gate travel and GM teleport wrote `pos_*`), so a returning character spawned at the last gate arrival or the creation point. Fixed on main by #756 (live-DB round-trip guard), but that fix has no client-test record yet. Level, XP and stats persistence are unaffected |
| Inventory persistence | CW | -- | base/world_entry/methods/inventory | sgw_inventory + bandolier_slots tables |
| Mission persistence | CW | -- | entity/missions.rs | sgw_mission + per-step state |
| Cell event outbox | CW | -- | base/outbox/ | Durable Base→Cell event delivery |
| Compile-time query checking | KM | -- | -- | **Re-verified 2026-09-25 (was CW).** No code: `crates/` contains zero `sqlx::query!` / `query_as!` / `query_scalar!` macro invocations and no `.sqlx` offline data. Every query is a runtime-checked `sqlx::query(...)` string, verified only by the live-DB suite at test time |
| Live-DB test infrastructure | CW | -- | `cimmeria-test-support` (live_db_gate.rs), test_support.rs shims, tools/test-live-db.sh | `require_db_or_skip!`: **764 invocations** (was 228 at 2026-07-25), all in `cimmeria-services`. Since #702 it fails, rather than skips, when `DATABASE_URL` is set but unreachable. CI `test-live-db` runs against postgres:17.9 |
| Connection pooling | CW | -- | database.rs:52 (`PgPool::connect`) | sqlx pool with default sizing. The old audit's "single connection per service" claim was wrong |
| Migration framework | KM | -- | db/scripts/ | Idempotent manual scripts, no Diesel/sqlx-migrate. Project rule: seeds are the source of truth, so no new `db/scripts/` without asking |
