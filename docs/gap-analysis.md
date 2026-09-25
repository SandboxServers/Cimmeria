---
title: "Gameplay Systems Gap Analysis"
type: explanation
audience: engineers
last_updated: 2026-09-25
---

# Gameplay Systems Gap Analysis

> **Last updated**: 2026-09-25 (re-verification pass against `main` at `acbcc22e`; about 160 PRs merged since the 2026-07-25 edition)
> **Purpose**: Map every gameplay system's Rust implementation against what's needed for a complete server
> **Status**: Source of truth for project completion tracking
> **Measured against**: `main`. Work living only on an unmerged feature branch is called out explicitly in the affected section and is **not** counted as implemented.
> **Workspace scale**: **5,333 `#[test]` / `#[tokio::test]` attributes (4,975 gated in CI)** across **812 files**, **775 `require_db_or_skip!` live-DB guards** (774 in `cimmeria-services`, 1 in `cimmeria-content-engine`), **3 PL/pgSQL end-to-end smokes**, with a **first-class content engine** the original Python codebase did not have. CI excludes `cimmeria-app`, `cimmeria-content-editor`, `cimmeria-scene-editor`, `sgw-launcher`, `cimmeria-client-telemetry` and `cimmeria-lab`, which is the whole of the 5,333 → 4,975 difference. Counted on `main` by a grep of test attributes, so the figures are comparable run to run but include a few attributes inside comments.
>
> **Evidence bar for CW in this edition**: a written record of an in-client test (playtest report, UAT worknote, PR body or comment, issue comment, or a recorded confirmation). Rows that very likely work in-client but have no such record stay NT or IM.
>
> **Arithmetic note**: the 2026-07-25 edition's `TOTALS` row did not match its own per-system tables (its rows sum to 444 / CW 164 / NT 18 / IM 134 / KM 124 / NU 4; its totals line printed 443 / 159 / 18 / 134 / 128 / 4). The 2026-05-27 edition had the same problem. This edition recomputes every matrix row and total directly from the feature tables.

---

## How to read this doc

- **Code paths** cite the active Rust workspace under [`crates/`](../crates/). When a feature exists in the deprecated [`deprecated/python/`](../deprecated/python/) or [`deprecated/cpp/`](../deprecated/cpp/) trees but **not yet** in Rust, the row is marked `KM` (port pending). The Python and C++ trees are reference-only.
- **Confidence** reflects how sure we are about the status — HIGH means the code has been read and judged; MEDIUM means line counts and recent-PR evidence support the status but a deep read hasn't happened; LOW means inference from neighbouring code or .def files.
- **Recent PRs** are listed where they're load-bearing for the status.

## Status Taxonomy

| Status | Symbol | Meaning |
|--------|--------|---------|
| **Confirmed Working** | `CW` | Tested end-to-end with the game client (Castle Cellblock smoke + Lomiada captures) and verified correct |
| **Needs Test** | `NT` | Code exists, looks reasonable, but hasn't been verified with a live client |
| **Implemented** | `IM` | Code written but may be incomplete or have known issues |
| **Known / Missing** | `KM` | We know this needs to exist (from `.def` files, docs, or game design) but no code exists in `crates/` |
| **Needed / Unknown** | `NU` | Server-only system we infer must exist but have no direct evidence for |

---

## Infrastructure Systems (Solid)

### 1. Authentication and Login --- CW

- **Confidence**: HIGH (re-read 2026-09-25: `auth/`, `base/login/mod.rs`, `server/main.rs` audit writer, PRs #698/#738)
- **Documentation**: [connection-flow.md](connection-flow.md), [protocol/login-handshake.md](protocol/login-handshake.md), [architecture/negative-logging-convention.md](architecture/negative-logging-convention.md) (credential-field rule and cross-IP seams)
- **Rust code**: [`crates/services/src/auth/`](../crates/services/src/auth/): 3,382 lines across 9 files (handlers.rs, credentials.rs, credential_log_guard.rs, login_smoke.rs, mod.rs, service.rs, tls.rs, tls_smoke.rs, cert_watcher.rs), including a live-DB login smoke. Also [`crates/services/src/credential_redaction.rs`](../crates/services/src/credential_redaction.rs) (`CredentialPrefix`) and the duplicate-login eviction in [`crates/services/src/base/login/mod.rs`](../crates/services/src/base/login/mod.rs)
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
- **Documentation**: [drafts/spec/mercury-wire-format.md](drafts/spec/mercury-wire-format.md) (canonical, in-progress bible chapter), [protocol/mercury-wire-format.md](protocol/mercury-wire-format.md) (legacy summary), [architecture/transport-trait.md](architecture/transport-trait.md), [architecture/mercury-bundle.md](architecture/mercury-bundle.md), [architecture/mercury-loopback-harness.md](architecture/mercury-loopback-harness.md), [architecture/network-chaos-testing.md](architecture/network-chaos-testing.md), [audits/mercury-rust-conformance-2026-05-15.md](audits/mercury-rust-conformance-2026-05-15.md)
- **Rust code**: [`crates/mercury/`](../crates/mercury/): 14,194 lines across 66 files, **265 tests** (bundle, channel/, channel_bundle/, clock, codec, encryption/, instrumentation, lossy_transport, messages, packet/, test_harness/, test_transport, transport, unified, unpacker/)
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
- **Documentation**: [engine/cooked-data-pipeline.md](engine/cooked-data-pipeline.md), [engine/cooked-data-pak-format.md](engine/cooked-data-pak-format.md), [game-data.md](game-data.md), [architecture/mission-pak-overrides.md](architecture/mission-pak-overrides.md), [analysis/ring-transport-cellblock-castle/README.md](analysis/ring-transport-cellblock-castle/README.md) (sequence-override rationale), [analysis/dialog-ui-redesign/work-packets.md](analysis/dialog-ui-redesign/work-packets.md) (DU-01 patch mode)
- **Rust code**: [`crates/services/src/base/cooked_data.rs`](../crates/services/src/base/cooked_data.rs) (469), [`crates/services/src/base/resources/`](../crates/services/src/base/resources/) (1,910), [`mission_overrides.rs`](../crates/services/src/base/mission_overrides.rs) (549), [`item_overrides.rs`](../crates/services/src/base/item_overrides.rs) (311), [`sequence_overrides.rs`](../crates/services/src/base/sequence_overrides.rs) (169, new), [`dialog_overrides/`](../crates/services/src/base/dialog_overrides/) (3,380)
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
- **Documentation**: [architecture/service-architecture.md](architecture/service-architecture.md), [architecture/integration-test-infra.md](architecture/integration-test-infra.md), [../db/README.md](../db/README.md)
- **Rust code**: **sqlx 0.9** (bumped from 0.8 by #597) throughout `crates/services/`: **118 files in `crates/services/src` use `sqlx::query`** (125 across `crates/`), all runtime-checked, none use the compile-time `query!` macros. Durable outbox at [`crates/services/src/base/outbox/`](../crates/services/src/base/outbox/) (1,214 lines). Pool: `PgPool::connect` in [`crates/services/src/database.rs`](../crates/services/src/database.rs):52
- **Recent PRs**: #403 (reseed pgdata on container start), #355 (cell_event_outbox infra), #366 (live-DB harness), #422 (cell_dispatch + executor live-DB coverage), **#597 (sqlx 0.8.6 → 0.9.0)**, **#702 (live-DB tests fail instead of skipping when `DATABASE_URL` is set but unreachable, #615)**, **#746 (live-DB container load recipe fixed, #742)**, **#756 (player position persisted on logout via `CellToBaseMsg::PersistPosition`)**
- **Path forward**: Pool tuning under load (production pool uses sqlx defaults). No migration framework (seeds in `db/resources/` plus a handful of manual `db/scripts/`). Adopting compile-time `query!` checking would need `.sqlx` offline data in CI. Client-test logout-position persistence (#756).

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Player state persistence | CW | -- | services/cell/cell_methods/player, base/world_entry/cell_dispatch/position.rs | Level, XP, position, stats, training points. Re-verified 2026-09-25: until #756, **logout never persisted position** (only gate travel and GM teleport wrote `pos_*`), so a returning character spawned at the last gate arrival or the creation point. Fixed on main by #756 (live-DB round-trip guard), but that fix has no client-test record yet. Level, XP and stats persistence are unaffected |
| Inventory persistence | CW | -- | base/world_entry/methods/inventory | sgw_inventory + bandolier_slots tables |
| Mission persistence | CW | -- | entity/missions.rs | sgw_mission + per-step state |
| Cell event outbox | CW | -- | base/outbox/ | Durable Base→Cell event delivery |
| Compile-time query checking | KM | -- | -- | **Re-verified 2026-09-25 (was CW).** No code: `crates/` contains zero `sqlx::query!` / `query_as!` / `query_scalar!` macro invocations and no `.sqlx` offline data. Every query is a runtime-checked `sqlx::query(...)` string, verified only by the live-DB suite at test time |
| Live-DB test infrastructure | CW | -- | live_db_gate.rs, test_support.rs | `require_db_or_skip!`: **764 invocations** (was 228 at 2026-07-25), all in `cimmeria-services`. Since #702 it fails, rather than skips, when `DATABASE_URL` is set but unreachable. CI `test-live-db` runs against postgres:17.9 |
| Connection pooling | CW | -- | database.rs:52 (`PgPool::connect`) | sqlx pool with default sizing. The old audit's "single connection per service" claim was wrong |
| Migration framework | KM | -- | db/scripts/ | Idempotent manual scripts, no Diesel/sqlx-migrate. Project rule: seeds are the source of truth, so no new `db/scripts/` without asking |

## Core Gameplay Systems

### 5. Character Creation --- CW (core create-and-enter flow; was NT)

- **Confidence**: HIGH (re-read 2026-09-25)
- **Documentation**: [gameplay/character-creation.md](gameplay/character-creation.md)
- **Rust code**: [`crates/services/src/base/character_create.rs`](../crates/services/src/base/character_create.rs) (636), [`crates/services/src/base/character/`](../crates/services/src/base/character/) (1,068 across `mod.rs`, `delete_live_db_tests.rs`, `request_visuals_live_db_tests.rs`), [`crates/services/src/base/chardef.rs`](../crates/services/src/base/chardef.rs) (333), plus `character_create_live_db_tests.rs` (209). About 2,250 lines in total. No code change since 2026-07-25.
- **Recent PRs**:
  - #473 / #516 / #518: SGWGmPlayer.
  - **#704**: the Account typeID is pinned at `0x07` (its clientIndex) with a guard. The owner re-verified this against the binary: `0x08` would break character select.
- **In-client record (new 2026-09-25)**: the 2026-09-18 colo playtest ran the full flow with a real client. It covered the character list (count 1 → 2 → 3) and two creations: player 71, a Human Soldier (archetype 1), and player 72, a Jaffa (archetype 8). The creation-time boot item (3438, from `char_creation_choices`) showed in the character-select preview ("jaffa had the boot in main menu after creation"). Both characters entered Castle_CellBlock and played through to Castle. Source: [analysis/playtests/2026-09-18-colo-castle/README.md](analysis/playtests/2026-09-18-colo-castle/README.md) §3, rows 6:58 PM and 8:04 PM.
- **Path forward**:
  - Confirm that the client shows the name-reject feedback. P1 logged two `createCharacter` rejects for a surname with trailing whitespace; the tester retried and succeeded, but no one recorded what the client displayed.
  - Exercise **Delete**. P1's delete never reached the server.
  - Confirm the starting hotbar on a fresh character.
  - Name filtering and a per-account slot limit are still missing.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Character list display | CW | -- | base/character/ | **Promoted 2026-09-25.** SELECT from sgw_player. In-client: P1 list count 1 → 2 → 3 across two creations, each character selected and played ([playtest README](analysis/playtests/2026-09-18-colo-castle/README.md) §3, 6:58 PM). The typeID `0x07` guard (#704) protects this path. Re-verified 2026-09-25 |
| Character visual preview | CW | -- | base/character/request_visuals_live_db_tests.rs | **Promoted 2026-09-25.** Lazy-load from sgw_inventory. In-client: P1 8:04 PM, the Jaffa's creation-time boot (item 3438) rendered in the character-select preview, `Sending character visuals component_count=14` (playtest README §3). Re-verified 2026-09-25 |
| Name validation | NT | -- | base/character_create.rs:549 | `validate_character_name` (3–20 chars, no leading/trailing or doubled spaces, `[A-Za-z0-9 '-]`) plus SQL uniqueness. The reject path fired in-client: P1 logged two rejects for surname `"will "`, sent as `send_char_create_failed(.., 2)` at character_create.rs:62. What the client displayed is unrecorded (playtest README §7, tester question 1) |
| Visual choice validation | NT | -- | base/chardef.rs | char_creation_choices table. Valid choices were accepted and rendered in P1. The reject path has not been exercised in a client |
| Archetype selection | CW | -- | base/character_create.rs | **Promoted 2026-09-25.** 8 archetypes from resources. In-client: P1 created archetype 1 (Human Soldier) and archetype 8 (Jaffa). Both played with archetype-correct content, for example chain 1098 vs chain 1099 crate loot (playtest README §3, 7:18 PM and 8:10 PM). Re-verified 2026-09-25 |
| Starting equipment | CW | -- | base/character_create.rs | **Promoted 2026-09-25.** BagFillOrder insertion. In-client: the item attached to the chosen visual (3438) was inserted and showed equipped in the preview and in world (P1 8:04 PM; playtest README §5 "boot-lock movement gate"). Re-verified 2026-09-25 |
| Starting abilities | NT | -- | base/character_create.rs | From the charDef ability list (known-issues KI-10 resolved). Not specifically observed in P1 |
| Character deletion | NT | -- | base/character/delete_live_db_tests.rs | CASCADE to inventory and missions. In P1 the delete **never reached the server**; it is unknown whether the tester pressed it (playtest README §5) |
| GM character creation | IM | -- | mercury/world_data/phases.rs:46 | **Corrected 2026-07-25.** SGWGmPlayer is ported: seed accounts get `access_level`, and a GM enters the world as entity class `0x03` instead of `0x02` (PRs #473 / #516 / #518, merged 2026-06-17). There is no GM-only *creation* UI |
| Name filtering | KM | -- | -- | No profanity or reserved-name check. `validate_character_name` is format-only |
| Character slot limit | KM | -- | -- | No per-account limit |

### 6. World Entry and Spaces --- CW

- **Confidence**: HIGH (re-read 2026-09-25)
- **Documentation**: [protocol/world-entry-phases.md](protocol/world-entry-phases.md), [engine/space-management.md](engine/space-management.md), [connection-flow.md](connection-flow.md), [gameplay/death-respawn-system.md](gameplay/death-respawn-system.md)
- **Rust code**: [`crates/services/src/base/world_entry/`](../crates/services/src/base/world_entry/) — **95 files, 32,526 lines**. It covers `cell_dispatch/` (now including `position.rs` and `player_ghost.rs`), `gate_travel/`, `methods/{inventory, mail, player_load, progression, vendor}`, `reanchor_player.rs` (590) and `teleport.rs`. The post-reanchor replay is `cell/service/base_messages/player_init/resync.rs:88` (`resync_after_pawn_recreate`).
- **Recent PRs**:
  - #410, #414 and #422: earlier edition.
  - **#756**: the reanchor replays the hotbar, active slot, journal and `state_field`, and logout persists position.
  - **#682**: region hints are replayed after the reanchor. This was playtest finding H8.
  - **#662 / H01** and **#795**: stargate arrival is validated, and Harset gate-row arrival is restored.
  - **#640 / P45**: a cross-space transfer primitive.
  - **#644**: GM world-name lookup is case-insensitive, and the snap-back loop is fixed.
  - **#747**: first-login cinematic AoI hold.
- **In-client record**: P1 ran Castle_CellBlock world entry for two fresh characters. The Cellblock → Castle cross-world hop was clean: 7:21:11 PM teleport, 7:21:14 PM Castle entry (6,608 B / 6 packets), 16 missions reloaded, no errors (playtest README §3; `appendix-session-timeline.md` rows 00:21:11 and 00:21:14). There were 19 same-world respawns, measured at 120.2–120.9 s, "position snapped + state cleared".

> **Open defect — cold-client direct login into a non-Cellblock world.** Four colo sessions went silent within about 15 s of `onClientReady` and hit the 60 s inactivity reap. Each was a freshly started client logging straight into Castle (at a GM-teleported spot) or SGC_W1. The server sequence was identical to healthy entries, with no WARN or ERROR, and the same worlds load fine through gate travel. Recorded in PR #756 "Not in this PR"; no issue is filed. It now matters more because #756 makes logout persist position, so more returning characters will log straight into Castle or Harset. It needs the client's own log or crash dump. The *Map load sequence* row stays `CW`: the server-side sequence is the part that row claims, and the record says that part was correct.

- **Path forward**:
  - File and root-cause the cold-client direct-login hang.
  - Run a respawn UAT for #756: the hotbar, journal, region hints and auto-cycle should survive a death.
  - Log out and back in inside Castle.
  - Verify the other published spaces end to end. Castle_CellBlock and Castle are now routinely played; Harset has placements but no written in-client pass.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Space loading | CW | -- | services/cell/space_manager | NavMesh + entity loading. A navmesh now loads for every world (#794). Castle has one since #709 |
| Player entity creation | CW | -- | base/world_entry/methods/player_load | Creates the SGWPlayer entity, two-stage base + cell |
| Map load sequence | CW | -- | base/world_entry/ | All 30+ client setup messages. See the cold-client direct-login callout above: the server sequence matches healthy entries, and the cause is unknown |
| Stat sync to client | CW | -- | base/world_entry/methods/player_load | All stats sent on entry |
| Ability tree sync | CW | -- | base/world_entry/methods/player_load | 3 trees per archetype |
| Zone transition | NT | -- | base/world_entry/gate_travel/ | **Changed 2026-09-25 (was IM).** The single-player hop is proven in-client: P1's Cellblock → Castle cross-world teleport was "CONFIRMED clean", with 16 missions reloaded and no errors (playtest README §3, 7:23 PM). The earlier IM reason, "multi-player sync incomplete", is now addressed in code: #737 introduces arriving players with the `SGWPlayer` ghost cascade once their client has loaded, and #640 (P45) adds a cross-space transfer primitive. Neither has a two-client run, hence NT rather than CW. Re-verified 2026-09-25 |
| Forced position handling | CW | -- | services/cell/cell_methods | BASEMSG_FORCED_POSITION authoritative move. #644 bounded the snap-back recovery: nearest navmesh point, then respawner, then AABB clamp, with a 5-correction budget |
| World-entry observability | CW | -- | base/world_entry/ | OTLP spans across the whole pipeline |
| Cell dispatch arms | IM | -- | base/world_entry/cell_dispatch/ | tests_dispatch_arms/ has live-DB coverage. New arms `position.rs` (logout persist) and `player_ghost.rs` are not client-validated |
| Same-world respawn client resync | NT | -- | base/world_entry/reanchor_player.rs; cell/service/base_messages/player_init/resync.rs:88 | **New 2026-09-25.** The reanchor's `CREATE_BASE_PLAYER` wipes the client's per-entity caches. P1 finding H8: after respawning, the client sent zero region hints for 28 minutes. Main now replays inventory, region hints (#682), hotbar, active slot, journal and `state_field`, and keeps the auto-cycle bit (#756). Regression guards are `combat::tests::respawn_resync` and `base_messages::tests::disconnect_persist_position`. There is no in-client respawn record after the fix |

### 7. Movement and Navigation --- IM

- **Confidence**: HIGH (re-read 2026-09-25). Detour is live everywhere; every world now ships a mesh. Two defects from the 2026-09-18 playtest are still open: the speed-layer arithmetic and Castle's split mesh.
- **Documentation**:
  - [protocol/position-updates.md](protocol/position-updates.md) and [drafts/spec/position-updates.md](drafts/spec/position-updates.md) (the canonical bible draft).
  - [architecture/movement-validation.md](architecture/movement-validation.md), [architecture/navmesh-containment-modes.md](architecture/navmesh-containment-modes.md) and [architecture/movement-telemetry.md](architecture/movement-telemetry.md).
  - [engine/navmesh-build-pipeline.md](engine/navmesh-build-pipeline.md), [engine/navbuilder-recast-limits.md](engine/navbuilder-recast-limits.md) and [engine/castle-navmesh-connectivity.md](engine/castle-navmesh-connectivity.md).
  - [`data/spaces/README.md`](../data/spaces/README.md), the per-world mesh provenance.
- **Rust code**:
  - Entity crate:
    - [`crates/entity/src/movement.rs`](../crates/entity/src/movement.rs) (351).
    - [`crates/entity/src/navigation/`](../crates/entity/src/navigation/) (4,571): `mod.rs`, `load.rs`, `load_tiled.rs`, `path.rs`, `surface.rs`, `line_of_sight.rs`, `verdict.rs`, `fingerprint.rs`, `poly_block.rs`, `xrc.rs`, tests.
    - [`crates/entity/src/movement_validation/`](../crates/entity/src/movement_validation/) (870).
    - [`crates/entity/src/detour_ffi.rs`](../crates/entity/src/detour_ffi.rs) (138, now with the tiled-mesh entry points). Detour is compiled from source by `crates/entity/build.rs`.
  - Cell seam:
    - [`cell/space_manager/client_move.rs`](../crates/services/src/cell/space_manager/client_move.rs) (664).
    - [`navmesh_mode.rs`](../crates/services/src/cell/space_manager/navmesh_mode.rs) (329).
    - `movement_telemetry/` (999).
  - NPC stepping: `cell/service/ticks/npc_movement.rs` (682) and `npc_ground.rs` (102).
  - Build tooling: [`crates/navmesh-extractor/`](../crates/navmesh-extractor/). The meshes are 24 `.nav` files under `data/spaces/`, with per-world `.occ` occluders.
- **Recent PRs**:
  - Earlier edition: #437, #478, #428.
  - Validator fixes:
    - **#643**: a jump no longer trips the navmesh reject.
    - **#644**: the snap-back rubber-band loop is fixed, with a recovery ladder and a correction budget.
    - **#645**: facing-preserving position primitive.
    - **#639**: `onPhysics` 221 GM fly/ghost bypass.
  - Mesh tooling and data:
    - **#683 / #710**: UE3 Terrain + BSP extraction pipeline.
    - **#694**: rebuilt `castle_cellblock.nav`.
    - **#709**: `castle.nav`, with Castle seeded advisory.
    - **#682**: per-world `navmesh_mode`.
    - **#794**: a mesh for every world; the new ones are seeded advisory.
    - **#796**: tiled meshes for the seven large exteriors.
    - **#700 / #726**: navmesh telemetry.
  - NPC stepping:
    - **#677**: `pack_angle` wrap and attacker re-face.
    - **#774**: storey-aware ground height.
    - **#779**: zero velocity when stopped.
    - **#783**: grounded every step.
    - **#788**: island edges and off-mesh starts.
    - **#707**: `move_waypoint` witness snap.
- **In-client record**: P1 is the source for these, with the fixes now shipped:
  - **H1**, backwards facing: #677.
  - **H2 / H4**, floating and the Y sawtooth at leg boundaries: #783, #774.
  - **H3**, no Castle mesh: #709.
  - **H4b**, attackers could not turn: #677.
  - **Rubber-band snap-backs**: 212 in Castle_CellBlock against the 2013 mesh. The rebuilt mesh accepts 195 of those 212 positions (#694).

  Still open from P1: the speed validator divides by per-packet wall-clock (35–60 ms windows, `Inf` reaching the metric, 756 warnings in one session; playtest README §5). U1 confirmed in play that no guard aggroed across floors and that guards held their cover slots. That is NPC AI evidence; it does not establish movement correctness.
- **Path forward**:
  - Window the speed layer over game ticks, *then* calibrate `SPEED_WARN_TOLERANCE` and switch it to enforce (#461 CAT-B-01 residual).
  - Join Castle's exterior and interior meshes with off-mesh links or per-region handling ([engine/castle-navmesh-connectivity.md](engine/castle-navmesh-connectivity.md) §4).
  - Walk Cellblock as a non-GM to confirm the rebuilt mesh.
  - Promote more worlds from advisory to enforce as coverage is verified.
  - `unstuck` is still a stub (#461 CAT-B-08).

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Client position updates | CW | -- | services/cell/cell_methods; cell/space_manager/client_move.rs | playerUpdate accepted. P1 ran two characters for about 90 minutes with no position-sync complaint beyond the snap-backs covered below |
| Dead reckoning / interpolation | IM | -- | entity/movement.rs | Extrapolates between updates. No change since 2026-07-25 |
| Space bounds check | IM | -- | entity/movement_validation/bounds.rs | AABB + NaN/infinity + Z-floor-clip. **Re-pointed 2026-09-25:** now wired at cell/space_manager/client_move.rs:260 (`entities.rs:288` is stale) |
| NavMesh / Detour FFI | NT | -- | entity/detour_ffi.rs; entity/navigation/ | **Changed 2026-09-25 (was IM).** The FFI now also loads tiled meshes (`detour_create_tiled_navmesh` / `detour_add_tile`, #796). Load is header-first with size caps (#726), and ground height is storey-aware (#774). `find_path` is at navigation/path.rs:149 and `raycast` at line_of_sight.rs:217. NPC line of sight now prefers per-world collision occluders (`.occ`, #797). Exercised in two colo sessions (P1 Cellblock paths, U1 cover-slot routing), but neither record judges the query results themselves. Re-verified 2026-09-25 |
| NPC pathfinding | IM | -- | entity/navigation/path.rs | Path query, off-mesh start snap and island-edge hold (#788). **Known gap:** Castle's mesh is still split exterior ↔ interior with no stair or ramp geometry between them (549 components; [engine/castle-navmesh-connectivity.md](engine/castle-navmesh-connectivity.md) §4). There are no off-mesh links, so an NPC cannot route between storeys joined by scripted lifts or rings |
| NPC waypoint movement | NT | -- | cell/service/ticks/npc_movement.rs, npc_ground.rs | **Changed 2026-09-25 (was IM).** Every P1 stepping defect has a merged fix: facing wrap (#677), per-step navmesh grounding including ramps and stairs (#783, #774), zero velocity when stopped (#779), witness snap for `move_waypoint` (#707). The NPC AI ledger lists each as UATPending ([session-resume](analysis/npc-ai-restoration/handoffs/session-resume.md) "Owner UAT checklist" item 2). Re-verified 2026-09-25 |
| NPC patrol | IM | -- | cell/service/npc_ai/patrol.rs | 262 lines. `AiState::Patrol` is set by the GM `.path_*` authoring commands (cell/console/patrol.rs). Few seeded routes exist, and there is no in-client patrol record |
| Server-side speed validation | IM | -- | entity/movement_validation/mod.rs:279 | Still **warn-only**. **Defect on record (P1, playtest README §5):** `implied_speed = distance / dt` per packet on 35–60 ms windows, with `dt < 1e-4` mapped to `f32::INFINITY` (mod.rs:308-312). That produced 756 warnings in one session, with `Inf`/`NaN` reaching the metric. The code is unchanged. #644 added the `movementSpeedMod` stat to the budget. Tracked in #461 (CAT-B-01 residual). Re-verified 2026-09-25 |
| Teleport detection | NT | -- | entity/movement_validation/mod.rs:314; cell/space_manager/client_move.rs:490-530 | **Changed 2026-09-25 (was IM).** Dual gate: distance > `TELEPORT_JUMP_UNITS` (50) **and** implied speed > `top_speed × TELEPORT_SPEED_FACTOR`, then snap-back. The live-incident rubber-band loop (`.goto harset`) is fixed by #644: `Rejected` / `Recovered` / `CorrectionSuppressed`, a budget of 5, and `note_authorized_teleport` clearing strikes at every authorized move. Jumping no longer false-rejects (#643). Not re-tested in-client after the fixes |
| Player navmesh containment (per-world mode) | IM | -- | cell/space_manager/client_move.rs:275; cell/space_manager/navmesh_mode.rs:135 | **New 2026-09-25.** Layer 4 of the validator, gated per world by `resources.worlds.navmesh_mode` (#682). Castle_CellBlock is the only `enforce` world; every other world is `advisory`: accepted, logged at TRACE (#709, #794). GMs bypass it (`navmesh_gm_bypass`, `onPhysics` fly/ghost #639). **Defect on record:** P1 logged 212 Cellblock snap-backs against the 2013 mesh. The rebuilt mesh (#694) accepts 195 of 212. A spot at (-196.2, 55.5, -139.1) is still uncovered and "needs an in-client look" (PR #694) |
| Navmesh coverage (a mesh per world) | IM | -- | data/spaces/*.nav; crates/navmesh-extractor/ | **New 2026-09-25.** 24 meshes built from the cooked client maps (StaticMesh + Terrain + BSP), with tiled builds for the seven big exteriors (#683, #709, #794, #796). Seeded-point probe hit rates vary. Castle is 62/78, Harset 51/67, Cellblock 46/92 ([data/spaces/README.md](../data/spaces/README.md); these are prefab and area corners, not only standable points). No mesh except Cellblock's has been walked in-client under enforcement |

### 8. Entity Lifecycle (AoI) --- IM (open entity-introduction defect; see project-status Known Issues)

- **Confidence**: MEDIUM (re-read 2026-09-25). Downgraded 2026-07-25 and still MEDIUM. The invisible-entity defect below has no validated fix; the #747 experiment is shipped but unobserved. Do not plan against "AoI is done". Note that [project-status.md](project-status.md) lists this system as `IM`; the two docs disagree on the heading.
- **Documentation**: [engine/entity-lod-system.md](engine/entity-lod-system.md), [engine/entity-type-catalog.md](engine/entity-type-catalog.md), [architecture/first-login-cinematic-aoi-hold.md](architecture/first-login-cinematic-aoi-hold.md), [architecture/player-ghost-aoi-cascade.md](architecture/player-ghost-aoi-cascade.md)
- **Rust code**:
  - [`crates/entity/src/cell_entity/`](../crates/entity/src/cell_entity/): bandolier, state_flags, system_options, `witness_aoi.rs` (with `is_introducible` at :55), tests, mod.
  - [`crates/entity/src/world_grid.rs`](../crates/entity/src/world_grid.rs) and [`crates/entity/src/space.rs`](../crates/entity/src/space.rs).
  - [`base/world_entry/cell_dispatch/aoi.rs`](../crates/services/src/base/world_entry/cell_dispatch/aoi.rs) (599), [`cell_dispatch/player_ghost.rs`](../crates/services/src/base/world_entry/cell_dispatch/player_ghost.rs) (369) and [`cell_dispatch/deferred_flush.rs`](../crates/services/src/base/world_entry/cell_dispatch/deferred_flush.rs) (455).
  - [`base/deferred_aoi_lifecycle.rs`](../crates/services/src/base/deferred_aoi_lifecycle.rs) (205).
  - [`base/world_entry_appearance/cinematic_aoi_hold/`](../crates/services/src/base/world_entry_appearance/cinematic_aoi_hold/mod.rs) (551).
  - [`mercury/aoi/`](../crates/services/src/mercury/aoi/): `create.rs`, `leave.rs`, `method.rs`, `update.rs`, `player_ghost.rs` (337).
  - Witness fan-out helpers in `cell/abilities/messaging.rs:98,153`.
- **Recent PRs**:
  - Earlier: #279 (BeingAppearance recomposite broadcast), #418, #408/#410, #580 (player combat and death state fanned out to witnesses; closes #232), #582 (`aoi.create_emit` / `aoi.create_send_failed` seams).
  - **#737**: players in a shared world see each other.
  - **#747**: first-login cinematic AoI hold, plus the `aoi.create_emit` OTLP export.
  - **#707**: `move_waypoint` fans `EntityMoved` to witnesses immediately (closes #616).
  - **#779**: the malformed `onSequence` "movement type" broadcast is no longer sent.
  - **#688 / #695**: lab witness queries and packet taps (tooling).

> **Open defect — invisible entity until relog.** In Castle Cellblock a GuardBody corpse (a `class_id 0` static mesh) is not visible to a first-login player until they relog. The 2026-06-20 repro disproved the address-gate hypothesis. The 2026-09-19 repro retired Mercury delivery: every create was ACKed on the first try, so the drop is client-side. #747 (merged 2026-09-21) ships a first-login cinematic AoI hold as the experiment on the one n=1 differential ([architecture/first-login-cinematic-aoi-hold.md](architecture/first-login-cinematic-aoi-hold.md)). **No in-game observation of the hold exists as of 2026-09-25.** U1 (2026-09-25, a build that includes #747) does not mention the corpse either way. The Cellblock UAT guide's results table is still blank ([uat-guide.md](analysis/castle-cellblock-rebuild/uat-guide.md) "Results", T03 and T28). Treat entity-introduction *rendering* as unproven.

- **Path forward**:
  - Run the #747 check: one fresh character watches the first-login movie to the end, another presses Esc, and both look at the GuardBody corpse.
  - Run the two-client checklist in [architecture/player-ghost-aoi-cascade.md](architecture/player-ghost-aoi-cascade.md).
  - #278 (the witness-fanout consolidation) was closed as done on 2026-09-25.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Entity creation | CW | -- | entity/manager.rs | From template or dynamic |
| Entity destruction | CW | -- | entity/manager.rs | Cleanup + witness notification |
| Grid-based AoI | CW | -- | entity/world_grid.rs | Chunk-based witness management |
| Witness enter/leave | IM | -- | entity/cell_entity/mod.rs; base/world_entry/cell_dispatch/aoi.rs | **Downgraded 2026-07-25; unchanged 2026-09-25.** onEnter/onLeave fire, but a witness can still fail to *render* an entity it was correctly introduced to (see the callout above). The #747 hold buffers `EnteredAoI` / `LeftAoI` / witness methods / `EntityInvisible` until `cancelMovie` or 16 s. `deferred_aoi_lifecycle.rs` keeps leave-before-create order on flush. Not observed in game. Re-verified 2026-09-25 |
| Property synchronization | CW | -- | entity/properties.rs | Per-distribution-flag write paths |
| State flag conventions | CW | -- | entity/cell_entity/state_flags.rs | bStateField, BSF_InCombat lifecycle |
| Bandolier state | CW | -- | entity/cell_entity/bandolier.rs | Slot lifecycle, type_id vs item_id discipline |
| LOD system | KM | -- | -- | No entity detail levels |
| Witness-fanout helper | NT | -- | cell/abilities/messaging.rs:98,153 | **Changed 2026-09-25 (was IM).** `send_entity_method_to_witnesses` and `send_entity_method_to_self_and_witnesses` carry combat, effects, stats, death and corpse state (#336, #580). All five child issues are closed, and parent #278 was closed as complete on 2026-09-25 (owner triage comment). Fan-out to *other players* has no two-client record, hence not CW |
| Player-to-player introduction | NT | -- | mercury/aoi/player_ghost.rs; base/world_entry/cell_dispatch/player_ghost.rs; entity/cell_entity/witness_aoi.rs:55 | **New 2026-09-25.** Players in a shared world (Castle, Harset) are introduced with a dedicated `SGWPlayer` / `SGWGmPlayer` ghost cascade. It carries appearance, name, level, alignment and live combat/death state. `is_introducible` holds a loading player out of AoI until it is initialised, and level-ups fan out (#737). Pinned by wire-format, fan-out byte and negative-log tests. **Not run with two real clients**; the UAT checklist is in [architecture/player-ghost-aoi-cascade.md](architecture/player-ghost-aoi-cascade.md) |

### 9. Combat and Abilities --- IM

- **Confidence**: HIGH for primitives, MEDIUM for end-to-end coverage (re-read 2026-09-25). Two rows were corrected against the code, and one is promoted on in-client playtest records.
- **Documentation**: [gameplay/combat-system.md](gameplay/combat-system.md), [gameplay/ability-system.md](gameplay/ability-system.md), [architecture/abilities-and-effects-system.md](architecture/abilities-and-effects-system.md) (decisions 17-19: health-threshold drain, surrendered-NPC floor, single `resolve_death`), [reverse-engineering/findings/combat-wire-formats.md](reverse-engineering/findings/combat-wire-formats.md), [reverse-engineering/findings/combat-formulas-status.md](reverse-engineering/findings/combat-formulas-status.md) (#673: which formulas can be recovered and which are fan guesses)
- **Rust code**: [`crates/services/src/cell/combat/`](../crates/services/src/cell/combat/) (3,400 lines, 13 files: `damage/{pipeline,qr}.rs`, `threat/{aggro,player_combat}.rs`, `auto_cycle.rs`, `state.rs`, `health_threshold.rs`, `faction_reaction.rs`, ...), [`crates/services/src/cell/abilities/`](../crates/services/src/cell/abilities/) (7,994 lines, 31 files, including `death/`, `damage_apply/`, `cone_aoe/`, `use_ability/`). That makes **11,394 lines in production combat and 194 tests** (90 in combat, 104 in abilities). [`crates/game/src/combat/`](../crates/game/src/combat/) (463 lines, 10 tests) has **no consumer outside `cimmeria-game`**. It is a dead parallel model and the live pipeline does not use it.
- **Recent PRs**: #420 (the complete ability and effect system) is still the base. Since 2026-07-25:
  - **#747**: an effect-script bleed to 0 HP kills on the same shot, for NPC and player victims. Every death goes through `abilities::death::resolve_death`, and a DoT kill is a real kill.
  - **#785**: leash drains the NPC from player combat.
  - **#787 / #789**: proximity aggro and same-room assist aggro feed the threat list.
  - **#791**: a dying player leaves every threat list, and `useItem` is refused while `BSF_DEAD`.
  - **#786 / #793 / #797**: NPC attack line of sight (stationary-relaxed policy, cover peek point, collision-geometry occluders).
  - **#734 / #744**: `onTimerUpdate` SecondaryId.
  - **#722 / #725**: docs-only verification of the cooldown SourceID and the PAK layout.
  - **#673**: the combat-formulas evidence ledger.
  - **#677**: attackers re-face their target.
  - **#635-#637**: `.combatinfo`, `.stats` and `.listabilities`.
  - Harset H04 (`560a8bd5`, via #662/#682): `entity_health_below` fires from every damage path.
- **Path forward**:
  - Enforce LOS on player `useAbility`.
  - Add a min-range check.
  - Add positional (front/flank/rear) checks.
  - Enforce prerequisite monikers.
  - Fix the two divergences #673 found: `EF_DONT_USE_QR` is 32 and is never read (the original is 16), and the `EDamageType` wire values are 0-4 (the original is 13-18; check a client capture before changing them).
  - Add threat decay.
  - Fix the combat animation (the shoot animation does not play, UAT-1 finding 10).
  - Model deploy abilities.
  - Delete or re-home the dead `crates/game/src/combat/`.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| QR calculation | IM | -- | cell/combat/damage/qr.rs:50 | Hit/miss/crit roll via `calculate_qr` / `calculate_result`. **Re-verified 2026-09-25.** Per #673 the QR-to-outcome mapping is a FAN-GUESS. `EF_DONT_USE_QR` (entity/abilities/defs.rs:58) is 32, but the original is 16, and it is never read, so the 754 no-QR effects still roll |
| Damage calculation | IM | -- | cell/combat/damage/pipeline.rs:33 | Resist → AF → absorb pipeline. The wire `damage_code` sends 0-4, but the original `EDamageType` is 13-18 (#673, unverified against a capture) |
| Stat resistance | IM | -- | cell/combat/damage/pipeline.rs:167 | **Path corrected 2026-09-25** (was game/combat/stats.rs, which is dead code). Flat-percent model. #673 shows that the original resistances were gating QR rolls, not flat reductions |
| Armor factor | IM | -- | cell/combat/damage/pipeline.rs:181 | Per-damage-type AF. **Path corrected 2026-09-25** |
| Absorption | IM | -- | cell/combat/damage/pipeline.rs:195 | 15 absorption stats. **Path corrected 2026-09-25** (was game/combat/damage.rs, which is dead code) |
| Auto-cycle (auto-fire) | CW | -- | cell/combat/auto_cycle.rs | Re-fires when the cooldown completes |
| In-combat state lifecycle | CW | -- | cell/combat/state.rs, threat/player_combat.rs | BSF_InCombat per-player threat tracking. Leash now drains player combat (#785) |
| Threat list | IM | -- | cell/combat/threat/aggro.rs:147 | `generate_threat` is exercised in-client: 50 player-initiated aggros in the [2026-09-18 playtest](analysis/playtests/2026-09-18-colo-castle/README.md). Proximity and assist sources were added (#787/#789), and dead players are dropped (#791, UAT-1 finding 4). **Still no threat decay**, and #791 has not been re-UAT'd. Re-verified 2026-09-25 |
| Single-target abilities | IM | -- | cell/abilities/use_ability/handle.rs | TCM_Single. Kills in-client are recorded both ways (playtest README 7:37, UAT-1), but the shoot animation does not play ([UAT-1](analysis/npc-ai-restoration/worknotes/uat-1.md) finding 10, the pre-existing combat-animation issue) |
| AoE abilities (radius) | IM | -- | cell/abilities/dispatch.rs | PR #420. A `TCM_AERadius` effect on a single-target ability still does not fan out (ADR follow-up) |
| AoE abilities (cone) | IM | -- | cell/abilities/cone_aoe/ | PR #420 |
| Group targeting | KM | Groups | -- | TCM_Group. No code, and **no seeded effect uses it**: the seed has only TCM_Single, TCM_AERadius and TCM_AECone (entity/abilities/defs.rs:40-48) |
| Aura targeting | KM | -- | -- | TCM_Aura. No code, and no seeded effect uses it |
| Ability warmup | IM | -- | cell/abilities/use_ability/handle.rs:528 | Ability_Begin sequence on warmup, and speed modifiers are applied |
| Ability cooldowns | CW | -- | entity/abilities/manager.rs:303 | Per-ability and per-moniker timers. The SourceID emit paths were verified (#722). Open follow-up: #271/#718 (absolute `bigWorldTimeComplete`) |
| Position/facing checks | KM | -- | -- | **Corrected 2026-09-25 (was IM).** `use_ability/handle.rs` has no front/flank/rear or facing test, and neither does any file under `cell/abilities/` or `cell/combat/`. "Flank" exists only as the cover mission trigger `player_flanked_npc` (#671). Re-verified 2026-09-25 |
| Weapon range checks | IM | -- | cell/abilities/use_ability/handle.rs:239 | Only max range is enforced (default 30 u, error code 42). **No min-range check** on the player path. Re-verified 2026-09-25 |
| Ammo consumption | CW | -- | cell/abilities/use_ability/handle.rs:366 | Decrements under the bandolier discipline |
| Auto-reload | CW | -- | cell/abilities/use_ability/auto_reload.rs | PR #394. Open verify-only issue #720 (reload timer type 2 against the binary's types 12/13) |
| Damage application | CW | -- | cell/abilities/damage_apply/mod.rs, death/mod.rs | **Promoted 2026-09-25 (was IM).** In-client record, [2026-09-18 playtest](analysis/playtests/2026-09-18-colo-castle/README.md): 26 lootable kills, 47 kill-XP grants, 19 player deaths to NPCs with 120 s respawns. [UAT-1 (2026-09-25)](analysis/npc-ai-restoration/worknotes/uat-1.md): a guard killed the player. The 2026-09-19/21 bleed-to-0 defects (NPC and player) are fixed by #747 with regression guards |
| LOS checks | IM | -- | cell/space_manager/spatial.rs:22,41,125; cover_sight.rs:113 | NPC side: navmesh ray, a stationary-relaxed policy (#786), a cover peek point (#793), per-world collision-geometry occluders (#797), and LoS gates on aggro and assist (#787/#789). UAT-1 found guards shooting through walls (fixed by #793, not re-UAT'd). **Player `useAbility` still has no LOS check**: no LoS call exists under `cell/abilities/`. Re-verified 2026-09-25 |
| Prerequisite monikers | KM | -- | -- | `AbilityManager::can_use_ability` (entity/abilities/manager.rs:303) checks known/cooldown/moniker-cooldown only. **No `canUseWithMonikers` exists in `crates/`**: the old "loaded, not checked" note was wrong. Re-verified 2026-09-25 |
| Deploy abilities | IM | -- | entity/abilities/defs.rs:16 | Only `AF_SPEED_DEPLOY` is used, as a speed-modifier category. There are no deploy semantics |
| Health/focus regen tick | IM | -- | cell/service/ticks/regen.rs | **Path corrected 2026-09-25.** 1 Hz out-of-combat regen keyed on an empty `threatened_mobs`, with a floor of 1 per pool. Effect-driven HoT goes through `cell/effects/pulsing/tick.rs`. No in-client record |

### 10. Effects and Buffs --- IM

- **Confidence**: HIGH for the framework, MEDIUM for content coverage (re-read 2026-09-25). Four rows were corrected after the code showed that the `EF_ClearOn*` / "permanent" machinery the old notes cite does not exist.
- **Documentation**: [gameplay/effect-system.md](gameplay/effect-system.md) (**stale**: it lists the clear-on flags as DONE), [architecture/abilities-and-effects-system.md](architecture/abilities-and-effects-system.md)
- **Rust code**: [`crates/services/src/cell/effects/`](../crates/services/src/cell/effects/): 3,742 lines across 9 files and **57 tests**:
  - `registry.rs` (63 lines);
  - `pulsing/{register,tick,channel_cancel,mod}.rs` (1,041 lines, plus a 649-line `tests.rs`);
  - `scripts.rs` (1,648 lines, of which tests start at line 696);
  - `cover_stance.rs` (173 lines, new in #790);
  - `mod.rs` (168 lines).

  **11 registered scripts**: HealHealth, HealFocus, MeleeDamage, MeleePhysicalDamage, AbsorbShield, Stun, Suppression, RangedPhysicalDamage, RangedEnergyDamage, CoverStance and RemoveCoverStance. **17 of 3,216 seeded effect rows** name a registered script (a further row names the unregistered `Reload`). The rest flow through the NVP `HealthDamage`/`FocusDamage` path.
- **Recent PRs**: #420 is the headline. Since 2026-07-25:
  - **#744** (with #734): the `onTimerUpdate` DurationEffect packet now sends the effect ID as SecondaryId. The client keys its active-effect timer lookup on that field, and it was 0 before.
  - **#747**: DoT and effect-script kills route through `resolve_death`.
  - **#619**: server-authoritative `launch_ability` / `apply_effect` content actions (`cell/content/effect_apply.rs`).
  - **#790**: Cover Stance buff and unbuff scripts.
  - ADR decision 18: surrendered NPCs are floored at 1 HP by pulses.
- **Path forward**:
  - Honour the effect-clear flags. The original vocabulary is `EEffectFlag`, but none of `EF_ClearOnDeath`, `EF_ClearOnDamage`, `EF_ClearOnRez` or `EF_RemoveOnBandolierSlotChange` has a Rust constant.
  - Send a wire packet for single-shot scriptless effects: `register_active_effect` returns early (pulsing/register.rs:50), so Stasis Sickness, the Prison Boot and similar effects are server no-ops.
  - Dispatch `effect_*` content triggers (#610; open PR #745).
  - Persist effects across logout.
  - Add script coverage for the long tail.
  - Wire `COVER_DEFENSE` into QR, because Cover Stance changes a stat that combat never reads.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Effect application | CW | -- | cell/effects/pulsing/register.rs:42 | A same-source re-apply refreshes (and never shortens), and different sources stack. UAT-1 recorded Cover Stance granted and removed in balance. Caveat: single-shot scriptless effects register nothing and send no packet (register.rs:50) |
| Effect pulse/tick | CW | -- | cell/effects/pulsing/tick.rs | Timer-driven at 100 ms. A DoT kill is now a real kill (#747) |
| Effect removal | CW | -- | cell/effects/pulsing/tick.rs:141 | Expiry sweep, then the script's `on_remove` (Stun, AbsorbShield, RemoveCoverStance), then an `onTimerUpdate` clear carrying SecondaryId (#744). Note corrected 2026-09-25: there is no general "revert non-permanent stat changes" |
| Stat change tracking | IM | -- | cell/effects/scripts.rs:390,454; cover_stance.rs | **Corrected 2026-09-25 (was CW).** No permanent/non-permanent distinction exists anywhere in `crates/` (a grep for "permanent" in cell/effects and entity finds nothing). Stat changes are reverted only by an individual script's `on_remove` (AbsorbShield, Stun, CoverStance). Direct HEALTH/FOCUS writes are one-way. Re-verified 2026-09-25 |
| Shared QR per pulse | IM | -- | cell/effects/pulsing/tick.rs:325 | Pulses hold QR neutral; the cast's roll is authoritative |
| Clear on death | IM | -- | cell/effects/pulsing/tick.rs:283; channel_cancel.rs | **Note corrected 2026-09-25.** There is no `EF_ClearOnDeath` constant or check. What exists: pulses on a dead target are skipped (the instances stay and age out), and a dying channeller's channels are cancelled (death/mod.rs:119) |
| Clear on damage | KM | -- | -- | **Corrected 2026-09-25 (was IM).** No `EF_ClearOnDamage` constant, and no code removes an effect when damage is taken. The `EF_*` constants are only defs.rs:56-62. Re-verified 2026-09-25 |
| Clear on revive | KM | -- | -- | **Corrected 2026-09-25 (was IM).** No `EF_ClearOnRez`, and the respawn/revive paths never touch `active_effects` (the only writers are `cell/effects/pulsing/*`). Re-verified 2026-09-25 |
| Clear on bandolier swap | KM | -- | -- | **Corrected 2026-09-25 (was IM).** No `EF_RemoveOnBandolierSlotChange`, and the bandolier handlers never touch `active_effects`. Re-verified 2026-09-25 |
| Effect scripts (registry) | IM | -- | cell/effects/registry.rs:21-33 | 11 scripts, bound by 17/3,216 effect rows. The `effect_*` content triggers are inert (#610, open) |
| Effect persistence | KM | -- | -- | EF_AlwaysPersist is not honoured across logout |
| Channeled effects | IM | -- | cell/effects/pulsing/channel_cancel.rs | Four cancel triggers (new ability, death, movement over 0.5 m, safety cap), with the `AF_CHANNEL_ALLOWS_MOVEMENT` override |
| Stealth-related flags | KM | -- | -- | EF_RemoveOnStealthZeroed and the others are not handled |

### 11. Stats --- IM

- **Confidence**: HIGH (infrastructure), MEDIUM (formula coverage), re-read 2026-09-25. No status changes, only path corrections.
- **Documentation**: [gameplay/stat-system.md](gameplay/stat-system.md) (**stale**: "Item stat bonuses PARTIAL, `inventoryAdjustments` exists", but no Rust code references it), [gameplay/progression-system.md](gameplay/progression-system.md), [reverse-engineering/findings/combat-formulas-status.md](reverse-engineering/findings/combat-formulas-status.md) (stat units verified from `alias.xml`; curves unknown)
- **Rust code**: [`crates/entity/src/stats/`](../crates/entity/src/stats/): 1,137 lines, 6 files, 23 tests. [`crates/game/src/combat/stats.rs`](../crates/game/src/combat/stats.rs) (159 lines) has **no consumer** and is dead code.
- **Recent PRs**: 29d46a65 (`StatList::scale_for_level()`, full heal on level-up). Since 2026-07-25: #636 (`.stats` plus the six stat-group dumps, verified field by field against legacy), #790 (the first effect to write `COVER_DEFENSE`), #756 (the reanchor replays client caches).
- **Path forward**:
  - Equipment stat bonuses (still no code).
  - Derived-stat formulas: none of them can be recovered from shipped data (#673), so they are a design decision parameterized by the verified 0.01-QR units.
  - Read `COVER_DEFENSE` and `coverAccuracy` in combat.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Stat class (min/cur/max) | CW | -- | entity/stats/stat.rs | 6 values plus dirty flags |
| Dirty stat sync | CW | -- | entity/stats/stat_list.rs | Incremental updates |
| Public/private split | CW | -- | entity/stats/stat_ids.rs | 11 public, the rest private |
| Archetype base stats | CW | -- | entity/stats/archetype.rs | Applied on first load |
| Per-level stat growth | CW | -- | entity/stats/stat_list.rs:324 | **Path corrected 2026-09-25** (was game/player.rs). `scale_for_level()` applies health/focus per level |
| Derived stat formulas | KM | -- | -- | No general stat derivation system. #673: the original curves are not in any shipped artifact |
| Stat soft caps | NU | -- | -- | No diminishing returns. #673: `EEffectFlag` has no cap or DR vocabulary |
| Item stat bonuses | KM | Inventory | -- | Equipment still does not modify stats. No `inventoryAdjustments` or item-stat code in `crates/`. #673: no shipped item field is a combat stat |

### 12. Inventory and Items --- IM

- **Confidence**: HIGH (re-read 2026-09-25)
- **Documentation**: [gameplay/inventory-system.md](gameplay/inventory-system.md), [reverse-engineering/findings/inventory-wire-formats.md](reverse-engineering/findings/inventory-wire-formats.md), [reverse-engineering/findings/inventory-state-machine.md](reverse-engineering/findings/inventory-state-machine.md), [content/equip-from-inventory-pattern.md](content/equip-from-inventory-pattern.md), [content/consumable-via-onitemuse-pattern.md](content/consumable-via-onitemuse-pattern.md)
- **Rust code**: [`crates/services/src/base/world_entry/methods/inventory/`](../crates/services/src/base/world_entry/methods/inventory/) — **5,585 lines** across `core/`, `grant/`, `move_/`, `ammo.rs`, `appearance.rs` + live-DB regression guards; [`crates/services/src/cell/cell_methods/inventory/`](../crates/services/src/cell/cell_methods/inventory/) (cell-side item ops + bandolier / active slot); [`crates/game/src/inventory/`](../crates/game/src/inventory/) (370 lines); [`crates/entity/src/inventory.rs`](../crates/entity/src/inventory.rs) (318 lines)
- **Recent PRs**: #405 (server-side stacking + Slappack PAK override), #399 (Slappack stacks to 10), #214 (bandolier + content + UI sync), #250 (equip-from-inventory pattern), #409 (full inventory re-init bundle on respawn); since 2026-07-25: #756 (reanchor also replays hotbar, active slot, journal and `state_field` after the inventory snapshot), #791 (`useItem` refused while dead with `onErrorCode(NotLiving)`), #731 (`OnItemUse` / `remove_item` pairing lint for consumable chains), #743 (bandolier guards exercise production helpers), #697 (bandolier ammo doc correction), #609 (store methods moved to 109/110 — see §15; voids pre-2026-07-26 buyback testing)
- **In-client evidence**: 2026-09-18 colo playtest ([appendix-session-timeline.md](analysis/playtests/2026-09-18-colo-castle/appendix-session-timeline.md) rows 00:03:14, 00:09:21, 00:24:38, 00:37:25) — item grant, equip-from-inventory (mission 622 completed on `item_equipped 55`), SMG grant + equip, Slappack use + consume, full inventory resync on reanchor.
- **Path forward**: Durability wear (nothing lowers `durability`; only vendor repair raises it); bind-on-pickup / bind-on-equip triggers (the `bound` flag is honored but only ever set by character-creation seed rows); client smoke of the vendor → buyback loop after #609.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Bag system (20 types) | CW | -- | base/world_entry/methods/inventory/core | Main, mission, equipment, bank |
| Item add/remove | CW | -- | base/world_entry/methods/inventory/grant | Live-DB regression guards; 2026-09-18 playtest grants + Slappack consume |
| Item stacking (server-side) | CW | -- | PR #405 | PR landed full server-side stack semantics |
| Equipment slots | NT | -- | base/world_entry/methods/inventory/ | Head through Artifact2. Weapon-slot equips are recorded in the 2026-09-18 playtest; no written record of armor-slot equips |
| Bandolier (4 weapon sets) | CW | -- | entity/cell_entity/bandolier.rs | Slot type_id/item_id discipline; #743 guards now call production helpers |
| Buyback bag | NT | Vendors | base/world_entry/methods/vendor/buyback | **Demoted 2026-09-25 (CW → NT).** 12-slot bag, filled only by vendor sells. PR #609 found the store payload had been emitted on the Missionary indices 80/81 instead of 109/110, so "the vendor UI could never have worked" and earlier manual vendor testing "is void". No vendor client test since #609. Re-verified 2026-09-25 |
| Cash (naquadah) | CW | -- | base/world_entry/methods/inventory/ | addCash/removeCash; `onCashChanged` pushed by `GrantCash` (base/world_entry/methods/progression/mod.rs:427) |
| Equip-from-inventory pattern | CW | -- | docs/content/equip-from-inventory-pattern.md | Mission 622/641 worked examples (PR #250); 622 completed on equip in the 2026-09-18 playtest |
| Visual sync | NT | -- | base/world_entry_appearance/ | Equipment visual updates; other-player visibility (#737) not two-client validated |
| DB persistence | CW | -- | sqlx live-DB tests | sgw_inventory + bandolier tables |
| Item durability | KM | -- | Column exists | Value is loaded, carried on the wire (entity/inventory.rs:79) and restored by vendor repair; nothing ever wears it down |
| Item binding | IM | -- | trade/execute/swap.rs:319; vendor/data | **Corrected 2026-09-25 (KM → IM).** The `bound` column is not unused: trade aborts with `TradeAbort::BoundItemOffered` (base/world_entry/methods/trade/execute/swap.rs:319), vendor sell-price load excludes `bound = true` rows (vendor/data, test at vendor/data/tests.rs:296), stack merge skips bound rows (inventory/grant/grant_item.rs:144), and `isBound` ships on the wire. Only character-creation seed rows ever set it (base/character_create.rs:239); no bind-on-pickup/equip. Re-verified 2026-09-25 |
| Respawn re-init bundle | CW | -- | PR #409, #756 | Full inventory bundle on reanchor, recorded in the 2026-09-18 playtest (00:37:25); #756 adds hotbar / active slot / journal replay |

### 13. Missions --- IM

- **Confidence**: HIGH for framework (re-read 2026-09-25), MEDIUM for content coverage
- **Documentation**: [gameplay/mission-system.md](gameplay/mission-system.md), [reverse-engineering/findings/mission-wire-formats.md](reverse-engineering/findings/mission-wire-formats.md), [content/mission-chains.md](content/mission-chains.md), [architecture/mission-pak-overrides.md](architecture/mission-pak-overrides.md), [content/dialog-ui-client-contract.md](content/dialog-ui-client-contract.md), [analysis/playtests/2026-09-18-colo-castle/](analysis/playtests/2026-09-18-colo-castle/README.md)
- **Rust code**: [`crates/services/src/cell/missions/`](../crates/services/src/cell/missions/mod.rs) (1,654: `lifecycle.rs` 541, `progression.rs` 713, `persist.rs` 280, `resend.rs` 87), [`cell/content/executor/mission.rs`](../crates/services/src/cell/content/executor/mission.rs) (528), [`base/world_entry/methods/missions/`](../crates/services/src/base/world_entry/methods/missions/mod.rs) (839), [`crates/game/src/missions/`](../crates/game/src/missions/) (364), [`crates/entity/src/missions.rs`](../crates/entity/src/missions.rs) (572), [`crates/services/src/base/mission_overrides.rs`](../crates/services/src/base/mission_overrides.rs) (549). That is about 4,500 lines of mission code. About 30 missions are chain-authored across 11 seed files: Cellblock 622-689 + 1360, Castle 701-708, Harset 567/742/1200/1324/1326/1360/1361, SGC_W1 1559/1561/1562.
- **Recent PRs**: #214 (marsh quest loop), #250 (equip-from-inventory PAK), **#646/#648/#649/#650/#653/#655/#671 (Castle Cellblock rebuild C00-C08, GC1)**, **#659/#660/#668 (Castle 701-708)**, **#682 (H50 per-objective state survives relog, fixes #657; H52 step-activation region replay; H54 `mission_abandoned` trigger)**, **#714 (advance_step reports implicitly completed objectives, fixes #656)**, **#748 (replay `player_entered_cover` on step activation)**, **#756 (mission journal resent after same-world respawn)**, #767/#770/#771/#773 (dialog overrides with buttons and the offered-dialog set, UATPending)
- **In-client record**: the 2026-09-18 colo playtest ran the Cellblock chain (622 → 688) on two characters and Castle 701, 702, 703, 704 and 706 to completion, then reached 708 step 4462 (timeline rows 00:05-01:28 UTC). "Mission state persisted correctly throughout", including a cross-world load of 16 saved missions.
- **Path forward**: Mission reward dispatch (#310: `chosenRewards` is `UNIMPLEMENTED` and nothing sends `onMissionRewardsDisplay`; `reward_xp` / `reward_naq` are 0 on all 1,041 mission rows). Delete the `sgw_mission` row on abandon and honour `can_abandon`. Failed-objective status needs RE first (#612). Decide on hidden-mission frame suppression (#715). Mission sharing for groups; mission-gated loot filtering. Client UAT of H50 relog restore and the DU dialog packets.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Mission accept | CW | -- | cell/missions/lifecycle.rs | From NPC dialog or chain (`accept_mission`). Re-seen in client in the 2026-09-18 colo playtest (622, 638, 639, 680, 701-708 accepted on two characters). Re-verified 2026-09-25 |
| Mission tracking | CW | -- | entity/missions.rs | Steps, objectives, status. Journal is now resent after a same-world respawn (#756). Open divergence: hidden missions still send client frames (#715, client effect unknown) |
| Objective completion | CW | -- | cell/missions/progression.rs, cell/content/executor/mission.rs | Content-engine action. #714 now sends `onObjectiveUpdate` for objectives that `advance_step` completes implicitly (closed #656) |
| Step advancement | CW | -- | cell/missions/progression.rs | Content-engine action. Playtest step advances confirmed in telemetry and client |
| Mission completion | CW | -- | cell/missions/progression.rs `complete_mission_direct` | **Note corrected 2026-09-25.** Completion works in the client (playtest: 641, 686, 688, 701-706 completed). The old note "Rewards XP + naquadah" was wrong: completion flips state only, no code reads `reward_xp`/`reward_naq` (both 0 in every seed row), and there is no reward window. Playtest: "missions don't give xp". See #310 |
| Mission failure | IM | RE | entity/missions.rs `fail()` | Mission-level `fail()` is reachable only from the GM `.`-console (cell/console/mission.rs:44). `FailObjective` has no executor arm, and there is no failed-objective status or wire value (#612) |
| Mission abandon | IM | -- | cell/missions/lifecycle.rs:186 | Removes the instance and sends the removal frame; H54 (#682) fires `mission_abandoned` from all four entry points. Known gaps: `can_abandon` is never checked, the `sgw_mission` row is not deleted, and the removal send is a silent `let _ =`. Re-verified 2026-09-25 |
| DB persistence | CW | -- | cell/missions/persist.rs, base/world_entry/methods/missions/ | `sgw_mission` round trip (playtest: 16 saved missions reloaded on Castle entry). H50 (#682) now persists per-objective state, fixing the step-id-as-objective relog bug (#657, closed). The relog restore itself is not yet client-verified |
| Mission PAK overrides | CW | -- | base/mission_overrides.rs | Mid-chain step injection |
| Repeatable missions | IM | -- | cell/missions/lifecycle.rs | Repeat-cap and `can_repeat_on_fail` offer guards with unit tests; no seeded repeatable content has been exercised |
| Mission sharing | KM | Groups | cell/cell_methods/missionary.rs | `shareMission` / `shareMissionResponse` log `UNIMPLEMENTED` |
| Mission-gated loot | KM | Loot | -- | No mission filter in loot generation |

### 14. Loot --- IM

- **Confidence**: MEDIUM-HIGH (re-read 2026-09-25; logic exists and has in-client evidence, content sparse)
- **Documentation**: [gameplay/loot-system.md](gameplay/loot-system.md), [reverse-engineering/findings/loot-generation.md](reverse-engineering/findings/loot-generation.md)
- **Rust code**: [`crates/services/src/cell/interactions/loot.rs`](../crates/services/src/cell/interactions/loot.rs) (519 lines incl. tests), [`crates/services/src/cell/abilities/loot_drop.rs`](../crates/services/src/cell/abilities/loot_drop.rs) (311 lines), [`crates/game/src/inventory/loot.rs`](../crates/game/src/inventory/loot.rs) (prototype; `instantiate_loot_drop` is still `todo!()` at :77 and unused by the live path)
- **Recent PRs**: #446 (looter distance re-validated per `lootItem`), #491; since 2026-07-25: #649 (mission 1360 Frost's Letter accepted from the Frost loot dialog — content, not loot mechanics), #638 (`GrantCash` feedback-recipient split touches the loot cash grant)
- **In-client evidence**: 2026-09-18 colo playtest, [appendix-session-timeline.md](analysis/playtests/2026-09-18-colo-castle/appendix-session-timeline.md) line 121: 26 lootable kills; Health Slappack rolled on 26/26 (`probability = 1`), naquadah on 17/26 (`probability = 0.8`, 5–50); tester complaint "too many slap-packs" (README line 85) and a looted Slappack used at 00:24:38.
- **Path forward**: Loot table content (2 tables / 4 loot rows in `db/resources/Loot/Seed/`, table 1 marked DEPRECATED); per-roll debug logging (playtest gap G9); per-player eligibility and group-loot modes after Groups; mission-gated loot.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Loot table definitions | CW | -- | db/resources/Loot/ | Schema present; table 2 "Cellblock NID guard default" drove the 2026-09-18 playtest |
| Loot generation algorithm | CW | -- | cell/abilities/loot_drop.rs | **Promoted 2026-09-25 (NT → CW).** Per-item probability roll observed in-client: 26/26 at p = 1, 17/26 at p = 0.8 across 26 kills; tester received and used the drops (2026-09-18 playtest, appendix-session-timeline.md:121, :144; README.md:85). Re-verified 2026-09-25 |
| Item drops | CW | -- | cell/abilities/loot_drop.rs | Drops + transfer to inventory |
| Cash drops | NT | -- | cell/interactions/loot.rs:218 | LOOT_Cash → `CellToBaseMsg::GrantCash`. Naquadah was rolled in the 2026-09-18 playtest, but no written record that it reached the wallet |
| Loot bag take-all | CW | -- | cell/interactions/loot.rs | Castle Cellblock smoke verified |
| Per-player eligibility | KM | Groups | -- | No eligibility list anywhere in `crates/` (re-checked 2026-09-25). Looting is gated on distance only (PR #446) |
| Group loot modes | KM | Groups | -- | No RoundRobin/FreeForAll logic in `crates/` |
| Mission-gated loot | KM | Missions | -- | No missionId filtering in loot_drop.rs / loot.rs |
| Loot table content | KM | -- | -- | 2 loot tables, 4 loot rows seeded; one is DEPRECATED. Castle mission items are explicit `add_item` grants by design |

### 15. Stores / Vendors --- NT

- **Confidence**: HIGH for code, re-read 2026-09-25. **No vendor has ever been driven in-client on working code** (see #609).
- **Documentation**: [gameplay/inventory-system.md](gameplay/inventory-system.md)
- **Rust code**: [`crates/services/src/base/world_entry/methods/vendor/`](../crates/services/src/base/world_entry/methods/vendor/) — **7,453 lines** across `buyback/`, `paid_recharge/`, `paid_repair/`, `purchase/`, `sell/`, `data/` submodules plus `store.rs`, `repair.rs`, `recharge.rs`, `serializers.rs`
- **End-to-end smoke**: [`tools/vendor_store_smoke.sql`](../tools/vendor_store_smoke.sql) (server-side PL/pgSQL, no client)
- **Recent PRs**: #214 (vendor sync), live-DB regression guards across each operation; since 2026-07-25: **#609** (store open/update were emitted on SGWPlayer indices 80/81 — Missionary's `onMissionUpdate`/`onStepUpdate` — and now go out on the correct 109/110; the PR states "the vendor UI could never have worked" and prior manual vendor testing "is void"), #737 (vendor emit path touched by the shared-world AoI change)
- **Content state**: `item_lists.sql` holds exactly two test lists, and template 25 ("Interaction Debug NPC - DO NOT USE") is the only vendor template. Harset packet H13 removed its only spawn, so **no world spawns a vendor today**; `.spawn 25` is the only route ([harset-rebuild/worknotes/H13.md](analysis/harset-rebuild/worknotes/H13.md) lines 182–190).
- **Path forward**: First in-client smoke on post-#609 code (`.spawn 25`, open store, buy / sell / buyback / repair / recharge) would move most rows to CW; real vendor lists and placed vendor NPCs (Harset GH2); client-initiated `repairItemRequest` (CM 40) is still a log-only stub (cell/cell_methods/inventory/item_ops.rs:229).

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Buy items | NT | -- | vendor/purchase/ | Validates cash, creates item, live-DB tests |
| Sell items | NT | -- | vendor/sell/ | Validates ownership, adds cash; bound items excluded |
| Repair items | NT | -- | vendor/paid_repair/ | Cost calculation. Client `repairItemRequest` path still a stub |
| Recharge items | NT | -- | vendor/paid_recharge/ | Ammo recharge |
| Buyback | NT | -- | vendor/buyback/ | 12-slot |
| Vendor stock from DB | NT | -- | vendor/data/ | **Demoted 2026-09-25 (CW → NT).** Lists load from DB, but the stock only reaches the client via `onStoreOpen`, which was misrouted to index 80 until PR #609 (2026-07-26); #609 voids earlier client observations. Only two test lists exist. Re-verified 2026-09-25 |
| Transactional safety | IM | -- | sqlx transactions | Live-DB tests verify atomicity; open security findings in #464 (CAT-E) |
| PL/pgSQL smoke | CW | -- | tools/vendor_store_smoke.sql | End-to-end *server-side* test; not an in-client check (see Notes) |

## NPC Systems

### 16. NPC AI and Behavior --- IM

- **Confidence**: HIGH for the state machine, aggro, leash and cover code (re-read 2026-09-25 after the NPC AI restoration campaign). MEDIUM for tuning: the D-NA09 radii (aggro 18 u, assist 10 u, leash 50 u, band 4 u) are starting guesses. One in-client session (UAT-1, 2026-09-25) covers the build before the NA23/NA24/NA27 fixes.
- **Documentation**:
  - [gameplay/npc-ai.md](gameplay/npc-ai.md);
  - [architecture/cover-system.md](architecture/cover-system.md);
  - [operations/npc-ai-telemetry-runbook.md](operations/npc-ai-telemetry-runbook.md);
  - campaign ledger [analysis/npc-ai-restoration/](analysis/npc-ai-restoration/README.md), with the UAT-1 worknote [worknotes/uat-1.md](analysis/npc-ai-restoration/worknotes/uat-1.md);
  - the [2026-09-18 colo playtest](analysis/playtests/2026-09-18-colo-castle/README.md).
- **Rust code**:
  - [`crates/services/src/cell/service/npc_ai/`](../crates/services/src/cell/service/npc_ai/): **51 files, 8,130 lines excluding tests** (11,500 with them). It holds dispatch, the `transition.rs` state-change helper, `idle_aggro.rs`, `aggro_gates.rs`, `assist.rs`, `fight.rs`, `fight_cover.rs`, `fight_target.rs`, `chase/`, `leash/`, `path_failure/`, `movement_stop/`, patrol, wander, investigate, follow, `lifecycle/` and the NA02 `detectors/`.
  - [`crates/services/src/cell/cover/`](../crates/services/src/cell/cover/): **3,038 lines excluding tests** (4,556 with them). Includes `peek.rs` and `stance.rs`.
  - [`crates/services/src/cell/space_manager/cover_sight.rs`](../crates/services/src/cell/space_manager/cover_sight.rs): LoS policy.
  - [`crates/services/src/cell/combat/aggression.rs`](../crates/services/src/cell/combat/aggression.rs) and [`faction_reaction.rs`](../crates/services/src/cell/combat/faction_reaction.rs).
  - [`crates/services/src/cell/service/ticks/npc_ground.rs`](../crates/services/src/cell/service/ticks/npc_ground.rs) and `npc_movement.rs`.
  - [`crates/occluder/`](../crates/occluder/): 4,048 lines, with a `data/spaces/<world>.occ` for all 23 worlds.
  - [`crates/services/src/cell/console/aggro.rs`](../crates/services/src/cell/console/aggro.rs): the `.aggro` GM toggle.
- **Recent PRs**:
  - Earlier: #368 (ability buckets and auto-aggro), #428 (movement states), #429 (cover).
  - #677 (wire facing, attacker re-face, follower height).
  - Telemetry: #776 (NA00, transition helper and OTLP identity), #781 (NA02, detectors), #782 (NA03, dashboard and runbook).
  - #779 (NA10, zero velocity on stop; the malformed movement-type `onSequence` removed).
  - #783 (NA11, per-step ground clamp).
  - **#785 (NA12, leash on NPC-to-spawn, walk home, evade, reset, player combat drain).**
  - **#787 (NA13, faction-derived proximity aggro with radius, floor band, LoS and the GM `.aggro` toggle).**
  - **#789 (NA14, same-room assist).**
  - #788 (NA15, path robustness).
  - #786 (NA16, stationary LoS relaxation).
  - #780 (NA21, world-space cover seeds).
  - **#790 (NA22, cover as firing positions and Cover Stance).**
  - #791 (NA24, UAT-1 findings 4-8).
  - #793 (NA23, cover peek point and flank hysteresis).
  - **#797 (NA27, collision-geometry occluder line of sight, closes #784).**
- **Path forward**:
  - Owner re-UAT on a build that has #791/#793/#797. The [session-resume checklist](analysis/npc-ai-restoration/handoffs/session-resume.md) covers leash walk-home, ramps, running-in-place, cover hold/seek and `.aggro off`.
  - Tune the radii from UAT.
  - Send `onAggressionOverrideUpdate` to the client (#330; the SGWMob method index is unverified).
  - Give Cover Stance a combat effect: `COVER_DEFENSE` is not read by hit resolution, and the magnitude is undecided.
  - Add a player fire-time LoS check (`NoLOS = 40`, still open after #797).
  - Decide whether SGC_W1 Ba'al Jaffa should stay passive (D-NA01 made them hostile).
  - Hearing radius, mob groups and kill-credit tapping remain unimplemented.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| AI state machine | NT | -- | cell/service/npc_ai/dispatch.rs, transition.rs | **Re-verified 2026-09-25.** All 11 states are dispatched (dispatch.rs:171-180). Every change now goes through the NA00 `transition.rs` helper, which logs the reason and cause (#776). Hostile Idle NPCs are ticked for the aggro scan (NA13), and a finished fight no longer parks the NPC Idle forever (NA12, #785). UAT-1 exercised Idle→Fighting→Dead in-client, but leash, return-home and several fixes landed after it |
| Spawning state | CW | -- | spawner/npcs.rs | Loads ammo, transitions to Idle |
| Idle state | CW | -- | spawner/npcs.rs | Waits for threat. Chain-armed spawns 20 and 10 waited for their chains at UAT-1 ([uat-1.md](analysis/npc-ai-restoration/worknotes/uat-1.md)) |
| Fighting state | NT | -- | cell/service/npc_ai/fight.rs, fight_cover.rs, fight_target.rs, chase/ | **Re-verified 2026-09-25.** The chase moved into `chase/` (NA15, #788). The attack check uses `attack_line_of_sight` (fight.rs:160). Attackers re-face every tick (#677). At UAT-1 the guards fought and killed the tester, but guards in cover shot through walls and flank-churned. Both are fixed on main by #793, and then #797 (occluder LoS), which a client has not seen yet |
| Threat accumulation | IM | -- | cell/combat/threat/ | -healthChange*2 - focusChange. **Re-verified 2026-09-25:** a dying player now leaves every threat list (NA24, #791) |
| Top-threat targeting | IM | -- | cell/combat/threat/, npc_ai/fight_target.rs | Linear scan with dead pruning. A `BSF_DEAD` target counts as dead whatever its HEALTH (UAT-1 finding 4, fixed in #791). Multi-attacker targeting is not client-tested |
| Ability bucket selection | CW | -- | spawner/abilities.rs, npc_ai/ability_select.rs | PR #368 three-bucket model (usable/cooling/needs-ammo) |
| Auto-reload | CW | -- | cell/abilities/ | PR #394 |
| Loot on death | CW | -- | cell/abilities/loot_drop.rs | Generates loot, sets interaction |
| Aggression override | IM | -- | spawner/npcs.rs, cell/combat/aggression.rs, console/aggro.rs | **Re-verified 2026-09-25.** The override comes from `spawnlist.aggression_override`, a chain's `set_aggression` or the console, else the faction table (NA13, #787). The server side held in-client at UAT-1: spawns 20 and 10 waited for chains 1032 and 1008. **Still IM:** the client is never told the level, because `onAggressionOverrideUpdate` is unsent (open #330, SGWMob method index unverified) |
| Auto-aggro | CW | -- | PR #368, chains 1008/1032 | Chain-armed aggro. The Castle drone encounter was verified end to end, and at UAT-1 the PRU fired across the med-station desk once armed ([uat-1.md](analysis/npc-ai-restoration/worknotes/uat-1.md) "Confirmed working"). Proximity aggro is its own row below |
| Investigating state | IM | -- | cell/service/npc_ai/investigate.rs | 197 lines; POI set by content action `Action::SetNpcPoi` (PR #428). NA11 snaps the endpoints onto the mesh |
| Leashing state | NT | -- | cell/service/npc_ai/leash/ (mod.rs, begin.rs, policy.rs) | **Re-verified 2026-09-25.** Rewritten by NA12 (#785), replacing the snap-home teleport that the 2026-09-18 playtest (H6) found never fired and would desync. Leash is now measured on the NPC's own distance from spawn (`entity_templates.leash_distance`, default 50 u, 5 u band). The NPC walks home on the navmesh, evades, heals on arrival, drains player combat and holds off re-aggro for 5 s. It snaps only with no route or after 20 s. UAT-1 did not record a leash |
| Patrol state | IM | -- | cell/service/npc_ai/patrol.rs | 262 lines; Idle auto-promotes to Patrol when `has_patrol`. NA11 snaps the endpoints onto the mesh |
| Wander state | IM | -- | cell/service/npc_ai/wander.rs | 241 lines; off-mesh candidates rejected; Idle auto-promotes on `has_wander` |
| Follow state | IM | -- | cell/service/npc_ai/follow.rs | 407 lines. **Re-verified 2026-09-25.** At UAT-1 Col Marsh's follow never ticked, because being-class NPCs were excluded from the AI tick (finding 6, fixed in #791, `ai_driven_npc_entity_ids`). The playtest's H7 silent early-returns were addressed only partly: Coppleman still has no follow chain. Not re-tested |
| Despawning state | IM | -- | cell/service/npc_ai/lifecycle/ | Terminal states (Despawning / Submit / Error) are dispatched at npc_ai/dispatch.rs:177-179 |
| Cover system | IM | -- | cell/cover/, npc_ai/fight_cover.rs, cover/peek.rs, cover/stance.rs | **Re-verified 2026-09-25.** The cover seeds are now real world-space nodes extracted from the maps: 236 for Castle_CellBlock and 3,788 for Castle (NA21, #780). The 9,346 prefab-local `.pak` rows were dropped. NPCs hold authored cover from spawn, seek a covered firing position and get Cover Stance (NA22, #790). **In-client at UAT-1:** guards held their slots, and Stance was granted and removed in balance. **Defects found:** guards in cover never aggroed, shot through walls and flank-churned; fixed by #793 and #797, not re-tested. **Still IM:** Cover Stance has no combat effect (`COVER_DEFENSE` is unread). The crouch pose has no server wire (D-NA10); an owner experiment decides it |
| Hearing system | KM | -- | -- | hearingRadius defined; still no runtime consumer (grep 2026-09-25) |
| Mob groups | KM | -- | -- | mobGroup is defined but unused. Assist aggro (below) recruits by faction and radius, not mobGroup |
| Tapping / kill credit | KM | -- | -- | tappedEntity is defined but unused. `handle_use_ability_with_kill_credit` credits the killing blow, not a tap |
| XP on kill | CW | -- | cell/abilities/damage_apply/ | kill_xp(), 10×mob_level Cell→Base pipeline |
| Faction proximity aggro | NT | -- | npc_ai/idle_aggro.rs, aggro_gates.rs, cell/combat/aggression.rs:49, console/aggro.rs | **New 2026-09-25.** NA13 (#787), D-NA01/02. A HOSTILE NPC, by override or else the 2009 `FACTION_REACTION_TABLE`, engages the nearest player within `aggro_radius` (18 u), inside a 4 u floor band, with LoS. An `Unknown` LoS verdict fails closed. GMs are aggroed unless they set `.aggro off`. **At UAT-1** there was no cross-floor aggro and one proximity aggro at 4.6 u. Guards spawned in cover never aggroed (finding 1); the fix is on main (#793, then the occluder in #797) and has not been re-tested. Before this, proximity aggro was structurally dead (playtest H5) |
| Assist aggro | CW | -- | npc_ai/assist.rs:61 | **New 2026-09-25.** NA14 (#789), D-NA04, a marked deviation from legacy. Same-faction hostile Idle/Patrol/Wander NPCs within `assist_radius` (10 u) of the victim join, with no chaining. **In-client:** at UAT-1 "the MessHall guards assisted each other, both ways" ([uat-1.md](analysis/npc-ai-restoration/worknotes/uat-1.md)). Caveat: #797 has since moved the assist LoS source to the occluder |
| NPC line of sight | NT | -- | space_manager/cover_sight.rs:113, crates/occluder/, data/spaces/*.occ | **New 2026-09-25.** NA27 (#797), D-NA13, closes #784. Aggro, assist, the attack check (`los_policy=occluder`) and cover sight all use a paged collision-geometry occluder at 1.5 m eyes; all 23 worlds ship one, 131 MB. This replaces the navmesh ray, which was wrong on 38-49% of its "blocked" answers. The navmesh stopgaps, the stationary relaxation (D-NA11, #786) and the cover peek point (D-NA12, #793), now apply only where there is no `.occ`. **At UAT-1** (pre-occluder) the drone fired across the desk under `stationary_relaxed`. The occluder itself has not been seen in a client |
| NPC grounding and stop hygiene | NT | -- | ticks/npc_ground.rs:45, ticks/npc_movement.rs, npc_ai/movement_stop/, npc_ai/chase/, npc_ai/path_failure/ | **New 2026-09-25.** These fix the 2026-09-18 playtest symptoms: floating (H2/H4), running in place, and stuck NPCs. **Ground and stop:** NA01 (#774) makes the height query storey-aware. NA10 (#779) sends zero velocity on stop. NA11 (#783) clamps every step and arrival to the ground, backs up with `moveAlongSurface`, and grounds the spawn Y. **Paths:** NA15 (#788) holds at mesh-island edges, snaps off-mesh starts, stops 1 u short of the target and routes to off-mesh targets. **Navmesh coverage:** every world now has a navmesh (#794), with tiled meshes for the large exteriors (#796). **Wire facing:** fixed in #677. None of this is recorded as seen in a client |

### 17. Spawn System --- IM

- **Confidence**: HIGH (re-read 2026-09-25). The **direct cell-side spawn from `resources.spawnlist`** covers the full spawn, death and respawn lifecycle, and the 2026-09-18 playtest measured 19 of 19 respawns at 120 s. The original `SGWSpawnRegion` / `SGWSpawnSet` population-control layer was **never built**: the Python server had empty stubs, and Rust spawns straight from the cell. Rows describing that layer were previously credited to `spawner/regions.rs`, but that file loads GenericRegion (client-hinted trigger regions). Issue #62 (triaged 2026-09-25) records the misattribution.
- **Documentation**: [gameplay/spawn-system.md](gameplay/spawn-system.md). Its "Original Design (not implemented as such)" section is accurate.
- **Rust code**: [`crates/services/src/cell/spawner/`](../crates/services/src/cell/spawner/) is **2,849 lines excluding tests, about 96 tests** (7,660 lines with `spawner/tests/`). It holds:
  - `npcs.rs`: `SpawnRecord`, the spawnlist + template load, `aggression_override`, `leash_distance`, `use_cover`;
  - `templates.rs`: prototype records for content-engine `spawn_entity`;
  - `regions.rs`: GenericRegion, not spawn regions;
  - `respawners.rs`: player defeat-window respawn points;
  - `stargates.rs`, `dialogs.rs`, `loot.rs`, `missions.rs`, `abilities.rs`, `worlds.rs`.

  NPC respawn is [`crates/services/src/cell/service/ticks/npc_respawn/`](../crates/services/src/cell/service/ticks/npc_respawn/). Mission-scoped spawns are [`crates/services/src/cell/space_manager/spawn.rs`](../crates/services/src/cell/space_manager/spawn.rs) and `cell/content/executor/spawn/`. The standalone `crates/game/src/world/spawning.rs` `SpawnSet` model has no caller in `cimmeria-services`.
- **Recent PRs**:
  - Castle and Harset population: #667 (CA05, Castle World 8 story actors and respawn timers), #662 and #682 (Harset spawn/despawn actions, templates and spawns), #717 (Harset placements).
  - #640 (GM `.spawn` / `.despawn`).
  - #783 (NA11, spawn Y grounded on the navmesh).
  - #785, #787, #789 and #790 add the `leash_distance`, `aggression_override`, `assist_radius` and `use_cover` spawn/template columns.
  - #791 (spawn 244 moved onto the mesh).
  - #795 (NA29, five world-57 spawns made mobile).
- **Path forward**:
  - Decide whether region/set population control (MaxActiveSets, population caps, set cooldowns, weighted spawn tables) is needed at all. The shipped content spawns fixed rows per spawnlist entry, and `spawn_sets.sql` / `spawn_points.sql` are empty in the seed.
  - If it is needed, port it cell-side against `resources.spawnlist`.
  - Time-of-day spawns and linked sets remain unknown in semantics.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| SpawnRegion entity | KM | -- | -- | **Downgraded 2026-09-25** (was IM). No `SGWSpawnRegion` in `crates/services`. `spawner/regions.rs` is GenericRegion loading. See issue #62 and spawn-system.md "not implemented as such" |
| SpawnSet entity | KM | -- | -- | **Downgraded 2026-09-25** (was IM). No `SGWSpawnSet` in `crates/services`. `crates/game/src/world/spawning.rs` has an unused model; `spawn_sets.sql` is empty. Issue #62 |
| Region activation | KM | -- | -- | **Downgraded 2026-09-25** (was CW). No spawn-region Activated/Deactivated lifecycle exists; the old citation was GenericRegion. Issue #62 |
| Set activation | KM | -- | -- | **Downgraded 2026-09-25** (was CW). No set Activate/Deactivate hooks (grep 2026-09-25). Issue #62 |
| Mob spawning | CW | -- | spawner/npcs.rs, space_manager/spawn.rs | Every `spawnlist` row is spawned at cell startup. Seen in-client in Castle_CellBlock and Castle (2026-09-18 playtest; UAT-1) |
| Mob registration | KM | -- | -- | **Downgraded 2026-09-25** (was IM). There is no `RegisterMobBase` analog; NPCs are registered only in the cell's SpaceManager. Likely obsolete under the direct-spawn design |
| Population tracking | KM | -- | -- | **Downgraded 2026-09-25** (was IM). No CurrentPopulation or reportPopulation (grep 2026-09-25). Issue #62 |
| Mob death notification | CW | -- | ticks/npc_respawn/mod.rs, abilities/death/ | `mark_npc_dead` stamps `respawn_at`. At the 2026-09-18 playtest "every death that reached `mark_npc_dead` also armed a respawn" ([appendix-npc-ai.md](analysis/playtests/2026-09-18-colo-castle/appendix-npc-ai.md)). Code pointer corrected 2026-09-25 (respawners.rs is player respawn points) |
| Respawn timers | CW | -- | ticks/npc_respawn/mod.rs, spawner/npcs.rs:176 | `COALESCE(spawnlist.respawn_secs, template.respawn_secs)`. The 2026-09-18 playtest had 19 deaths and 19 respawns at 120.2-120.9 s ([playtest README](analysis/playtests/2026-09-18-colo-castle/README.md) "CONFIRMED — respawn path is healthy"). Seeded: Castle 120 s (#667), Harset 30 s. Code pointer corrected 2026-09-25 |
| Set cooldowns | KM | -- | -- | **Downgraded 2026-09-25** (was IM). No min/maxCooldownSeconds consumer. Issue #62 |
| Max active sets | KM | -- | -- | **Downgraded 2026-09-25** (was IM). `grep MaxActiveSets crates/` finds nothing. Issue #62 |
| Spawn tables (weighted) | KM | -- | -- | **Downgraded 2026-09-25** (was IM). No (id, weight) spawn-table roll; each spawnlist row names one template |
| Spawn point randomization | KM | -- | -- | **Downgraded 2026-09-25** (was IM). No bRandomizeSpawnPoints consumer; spawns use the fixed spawnlist position |
| Level range filtering | KM | -- | -- | **Downgraded 2026-09-25** (was IM). No minMOBLevel/maxMOBLevel. Level comes from `entity_templates.level` (npcs.rs:162) |
| Player detection radius | KM | -- | -- | detectionRadius is defined but not wired. NA13's `entity_templates.aggro_radius` is a per-NPC aggro gate, not this region property |
| Time-of-day spawns | KM | -- | -- | onTimeOfDayTick. Only the client `onTimeOfDay` push exists |
| Mission integration | IM | -- | space_manager/spawn.rs, spawner/templates.rs, content/executor/spawn/ | Content-engine `spawn_entity` / despawn actions create mission-scoped NPCs from template prototypes with `respawn_secs` forced to None (#662). Chain-armed spawns keep a seeded passive override (D-NA01a). Code pointer corrected 2026-09-25 (spawner/missions.rs is the mission-definition cache) |
| Linked sets | KM | -- | -- | bLinked flag, semantics unknown |
| Population scaling | NU | -- | -- | timerReduction suggests dynamic spawn rates |
| Stargate spawning | CW | -- | spawner/stargates.rs | Castle ↔ neighbor verified |
| Loot-drop integration | CW | -- | spawner/loot.rs | Castle smoke covers loot bag drop |
| Dialog NPC spawning | CW | -- | spawner/dialogs.rs | Castle Cellblock NPCs |
| Ability NPC spawning | CW | -- | spawner/abilities.rs | PR #368 three-bucket |

## Secondary Gameplay Systems

### 18. XP and Leveling --- IM

- **Confidence**: HIGH (re-read 2026-09-25; kill-XP and level-up re-seen in client at the 2026-09-18 colo playtest)
- **Documentation**: [gameplay/progression-system.md](gameplay/progression-system.md) (stale, see Notes), [.claude/plans/2026-03-08-xp-leveling-design.md](../.claude/plans/2026-03-08-xp-leveling-design.md)
- **Rust code**: [`crates/game/src/player.rs`](../crates/game/src/player.rs) (210; `PlayerState::grant_xp`, level table), [`crates/services/src/base/world_entry/methods/progression/`](../crates/services/src/base/world_entry/methods/progression/mod.rs) (632 in `mod.rs`), [`crates/services/src/cell/abilities/death/side_effects.rs`](../crates/services/src/cell/abilities/death/side_effects.rs) (200; `grant_kill_xp`), [`crates/services/src/cell/abilities/loot_drop.rs`](../crates/services/src/cell/abilities/loot_drop.rs) (311; `kill_xp`), [`crates/services/src/cell/content/executor/mod.rs`](../crates/services/src/cell/content/executor/mod.rs) (`Action::GrantXP` arm, :512)
- **Recent PRs**: 29d46a65 (full XP + leveling system), **#618 (`grant_xp` content action: loader and executor arms, closes #611)**, #638 (`.givexp` GM target grant)
- **In-client record**: 2026-09-18 colo playtest. There were 47 kill-XP grants (10 each against level-1 Castle NPCs) and levels 2 and 3 were reached on both characters, "lvl 2 now with animation" (timeline 00:19:10, 00:38:56, 01:15:08, 01:24:48). Mission XP grants: 0.
- **Path forward**: Mission XP needs a reward formula (Castle/Harset decision GC3 / D-H10: the 2009 values are not in any recovered data), then either `grant_xp` chain rows or a `reward_xp`-driven dispatch on completion (#310). ASP grant on level-up.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| XP accumulation | CW | -- | game/player.rs | grant_xp(): additive, fires onExpUpdate |
| Level-up detection | CW | -- | game/player.rs | Multi-level-up supported, 16+ tests |
| Client notification | CW | -- | base/world_entry/methods/progression/ | 5 wire messages |
| XP from mob kills | CW | -- | cell/abilities/death/side_effects.rs, loot_drop.rs:118 | 10×mob_level, Cell→Base pipeline |
| XP curve | CW | -- | game/player.rs | LEVEL_XP[21] ported from Python Constants |
| Level cap (20) | CW | -- | game/player.rs | MAX_LEVEL enforced |
| DB persistence | CW | -- | sqlx | sgw_player.level + .exp |
| Stat scaling on level-up | CW | -- | entity/stats/ | scale_for_level(), full heal on level-up |
| Training points on level-up | CW | -- | game/player.rs | 2 TP/level, 38 by level 20 |
| XP from missions | IM | Content / design | cell/content/executor/mod.rs:512 | **KM → IM 2026-09-25.** A delivery path now exists: `Action::GrantXP` has loader and executor arms (#618, chain-replay guard). No XP flows yet. There are zero `grant_xp` seed rows (Harset chains carry `-- GC3: grant_xp` placeholders), `reward_xp = 0` on all 1,041 missions, and completion does not read it (#310). Playtest recorded 0 mission XP. Re-verified 2026-09-25 |
| ASP on level-up | KM | -- | -- | No ASP grant on level-up (ASP is granted only by GM `gmGiveAppliedSciencePoints`) |

### 19. Crafting --- KM (Phase 1 landed; the crafting *verbs* are still stubs)

- **Confidence**: HIGH (code re-read 2026-09-25; no crafting runtime change since 2026-07-25)
- **Documentation**: [gameplay/crafting-system.md](gameplay/crafting-system.md), [reverse-engineering/findings/crafting-wire-formats.md](reverse-engineering/findings/crafting-wire-formats.md), [reverse-engineering/findings/crafting-state-machine.md](reverse-engineering/findings/crafting-state-machine.md)
- **Rust code**: [`crates/entity/src/crafting.rs`](../crates/entity/src/crafting.rs) (191 — `CraftingState`), [`crates/services/src/base/crafting/`](../crates/services/src/base/crafting/) (1,103 — handlers.rs 440, persistence.rs 645, mod.rs 18), [`crates/services/src/cell/cell_methods/player/crafting.rs`](../crates/services/src/cell/cell_methods/player/crafting.rs) (232 — method routing), [`crates/services/src/cell/console/crafting.rs`](../crates/services/src/cell/console/crafting.rs) (136)
- **Recent PRs**: **#427 (Phase 1 — `CraftingState` + persistence + ASP dispatch fix, #53)**, #521 (GM crafting grants), #728 (docs only, 2026-09-25: records the method-138 racial-paradigm wire schema `INT32 paradigmId + INT8 level` and confirms Rust has no progression, emission, or login sync for it; follow-up #723)
- **Open issues**: #567 (crafting activity handlers), #723 (racial-paradigm progression + client sync), #465 (security audit CAT-F)
- **State of play**: Phase 1 shipped the *state* layer. `sgw_player.discipline_ids`, `blueprint_ids`, `applied_science_points` and `racial_paradigm_levels` load and save transactionally, and GM grant commands drive expertise. Every player-facing crafting verb still logs `UNIMPLEMENTED` (cell_methods/player/crafting.rs:33-84).
- **Path forward**: Phase 2 (#567): ASP-spend validation (paradigm gate + prerequisite expertise + DB UPDATE), then the craft / research / reverse-engineer / alloy flows. #723: emit method 138 on paradigm change and at login.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Craft from blueprint | KM | -- | stub | `CRAFT` arm parses `craft_id` then logs `UNIMPLEMENTED: craft` (cell_methods/player/crafting.rs:48). Re-verified 2026-09-25 |
| Research item | KM | -- | stub | `UNIMPLEMENTED: research` (crafting.rs:60) |
| Reverse engineer | KM | -- | stub | `UNIMPLEMENTED: reverseEngineer` (crafting.rs:65) |
| Alloy | KM | -- | stub | `UNIMPLEMENTED: alloying` (crafting.rs:72) |
| Discipline learning | KM | -- | stub | `spendAppliedSciencePoints` routes but does not mutate: "Phase 1: route only" (crafting.rs:24-33) |
| Expertise system (0-100) | IM | -- | entity/crafting.rs, base/crafting/handlers.rs:36 | **Corrected 2026-07-25.** `set_expertise` with an explicit `[0,100]` clamp, first-grant discipline registration, transactional save, and an `onUpdateDiscipline` client push (handlers.rs:86-140). Driven only by GM grants; no in-client record |
| Racial paradigm gating | KM | -- | state only | `racial_paradigm_levels` loads/saves as a `{paradigm_id → level}` map (persistence.rs:132-152) but **no gate function consumes it**, and method 138 is never emitted (#728 audit, #723). Re-verified 2026-09-25 |
| Blueprint management | IM | -- | base/crafting/persistence.rs:91 | **Corrected 2026-07-25.** `blueprint_ids` round-trips through `load_crafting_state` / `save_crafting_state`; no acquire/dedupe verbs yet |
| Crafting respec | KM | -- | stub | `UNIMPLEMENTED: respecCrafting` (crafting.rs:84) |

### 20. Stargate Travel --- IM

- **Confidence**: MEDIUM-HIGH (code re-read 2026-09-25; the new dial → open → cross flow has not been seen in a client)
- **Documentation**: [gameplay/gate-travel.md](gameplay/gate-travel.md), [reverse-engineering/findings/gate-travel-wire-formats.md](reverse-engineering/findings/gate-travel-wire-formats.md), [reverse-engineering/findings/stargate-dhd-state-machine.md](reverse-engineering/findings/stargate-dhd-state-machine.md), [gameplay/cinematic-system.md](gameplay/cinematic-system.md)
- **Rust code**: [`crates/services/src/cell/gate_travel/`](../crates/services/src/cell/gate_travel/mod.rs) (778 + 1,410 test: `mod.rs`, `address_book.rs`, `sequences.rs`, `tick.rs`), [`cell/space_manager/gate_dial_state.rs`](../crates/services/src/cell/space_manager/gate_dial_state.rs) (238), [`cell/interactions/dhd.rs`](../crates/services/src/cell/interactions/dhd.rs) (346), [`cell/arrival.rs`](../crates/services/src/cell/arrival.rs) (662), [`cell/content/executor/stargate.rs`](../crates/services/src/cell/content/executor/stargate.rs) (195), [`cell/content/event_dispatch/stargate.rs`](../crates/services/src/cell/content/event_dispatch/stargate.rs) (124), [`base/world_entry/gate_travel/`](../crates/services/src/base/world_entry/gate_travel/mod.rs) (1,343 + 1,048 test: `mod.rs`, `persist_arrival.rs`, `address_grant.rs`), plus `cell_methods/gate_travel.rs` (47), `client_methods/gate_travel.rs` (10) and `cell/kismet.rs` (58). About 3,800 non-test lines. `crates/game/src/interactions/stargate.rs` no longer exists.
- **Recent PRs**: **#663 (CA10: `Stargate_MakeGate` 6100 after a 4 s dial timer, `Stargate_CrossGate` 6113 on the crossing, both fanned to witnesses; `stargate_dialed` / `stargate_crossed` triggers)**, **#662 (H01: validated gate arrival and the `INT_DHD` → `onDisplayDHD` interaction)**, **#682 (H06: address-book dial gate, a gate trip learns both ends, world-scoped region hints; H55: `grant_stargate_address` action)**, #729 (DHD reply audit), #795 (Harset gate-row arrival)
- **In-client record**: the 2026-09-18 playtest ended at 708 step 4462 because the DHD had no `INT_DHD` interact arm (finding H10, 76 dead clicks). #662 added that arm the next day. Nobody has dialed a gate in a client since. Castle UAT M4 (PR #663 test plan) is unchecked.
- **Path forward**: Client UAT of dial → open → cross with a second observer (Castle M4). Dial-rejection feedback via `onDHDReply` (#727; today a refused dial is silent, which breaks the first-press feedback rule). 18 of about 30 seeded gate worlds have no gate volume and still travel on the dial. No hidden-address list. No gate cooldown.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| DHD interaction | NT | -- | cell/interactions/dhd.rs | **IM → NT 2026-09-25.** `INT_DHD` interact arm emits `onDisplayDHD` with the origin address, range-checks `address_origin` (1-38) and refuses worlds with no gate (#662 H01). This fixes playtest finding H10. It has not been clicked in a client since. Re-verified 2026-09-25 |
| Gate dialing | IM | -- | cell/gate_travel/mod.rs:67, tick.rs | 4 s dial timer + `gate_dial_state` (#663), address-book gate (#682 H06). Known issue: every rejected dial is silent to the player (#727) |
| Gate cancel | NT | -- | cell/gate_travel/mod.rs:83-96, space_manager/gate_dial_state.rs | **IM → NT 2026-09-25.** Cancel sentinel (-1), re-dial, every reject branch, and leaving the space all drop the armed dial. Timer-cancel and `destroy_entity` scrub guards are in #663. Re-verified 2026-09-25 |
| Gate passage | CW | -- | base/world_entry/gate_travel/ | moveTo destination, verified end to end before 2026-07-25. The crossing now also requires an open gate plus a walk into the `REGION_FLAG_STARGATE` volume (#663), with validated arrival (#662). That new front half has not been client-verified |
| Known address tracking | IM | -- | cell/gate_travel/address_book.rs, base/.../persist_arrival.rs | **Note corrected 2026-09-25.** A known list only. Cimmeria has no hidden list, no column and no wire slot (`map_loaded` always sends an empty hidden array; gate_travel/mod.rs:121-125) |
| Address discovery | NT | -- | cell/content/executor/stargate.rs, base/.../address_grant.rs | **IM → NT 2026-09-25.** `grant_stargate_address` action (#682 H55; Castle chain 1357 grants Harset). Arrival appends both origin and destination gates, deduplicated (`persist_arrival.rs`). Live-DB guards. Not client-verified. Re-verified 2026-09-25 |
| Cell-dispatch AoI defer | CW | -- | base/world_entry/cell_dispatch/tests_dispatch_arms/aoi_defer_gate.rs | Regression test |
| Multi-player gate sync | NT | -- | cell/gate_travel/sequences.rs | **KM → NT 2026-09-25.** 6100/6113 `onSequence` are sent to the dialer and every witness (#663; fan-out byte test in `tests_dispatch_arms/stargate_fanout.rs`). No two-client run (#663 UAT M4 unchecked; player-to-player AoI #737 is itself not two-client validated). Re-verified 2026-09-25 |
| Return trips | IM | -- | base/world_entry/gate_travel/persist_arrival.rs | **KM → IM 2026-09-25.** A gate trip now teaches the traveller the origin gate too, so they can dial home (#682 H06). There is still no shared open-wormhole or return state: each direction is a fresh dial. Re-verified 2026-09-25 |
| Gate cooldown | KM | -- | -- | No use-after-dial cooldown |

### 21. Chat --- NT

- **Confidence**: MEDIUM-HIGH (code re-read 2026-09-25)
- **Documentation**: [gameplay/chat-system.md](gameplay/chat-system.md), [reverse-engineering/findings/chat-wire-formats.md](reverse-engineering/findings/chat-wire-formats.md)
- **Rust code**: [`crates/services/src/cell/chat.rs`](../crates/services/src/cell/chat.rs) (454), [`crates/services/src/base/dispatch/chat.rs`](../crates/services/src/base/dispatch/chat.rs) (219), [`crates/services/src/base/world_entry_chat.rs`](../crates/services/src/base/world_entry_chat.rs) (237): 910 lines including tests
- **Recent PRs**: #739 (stored DND message bounded to 128 characters, security finding CAT-L-02), #737 (players in a shared world now witness each other, so say/emote/yell can reach another player; not two-client validated), #769 (content-engine `npc_bark` speaks NPC lines over `onPlayerCommunication` on the say channel, a content feature that reuses the chat wire, not player chat)
- **Open issues**: #471 (security audit CAT-L, chat / contact list, 9 findings)
- **In-client record**: the 2026-09-18 colo playtest logged 20 say-channel sends from the real client, all `.`-prefixed GM console lines, which chat.rs:88-97 intercepts before broadcast ([appendix-session-timeline.md](analysis/playtests/2026-09-18-colo-castle/appendix-session-timeline.md) line 123). That proves client-to-server say routing. It does not prove witness rendering of ordinary chat.
- **Path forward**: message *routing* on the non-spatial channels (they are registered but carry no traffic), direct tells, and moderation tools (mute, flood protection). A two-client say/emote/yell check now that #737 has landed.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Say/emote/yell (AoI) | NT | AoI | cell/chat.rs:99-110, 126-190 | Witness broadcast plus sender echo. Since #737 a second player can be a witness; no two-client test on record. Re-verified 2026-09-25 |
| Direct tells | KM | -- | -- | `sendPlayerCommunication` parses the `target` WSTRING, logs it, and forwards every message to the cell as a spatial broadcast (base/dispatch/chat.rs:22-108) |
| User channels | KM | -- | -- | requestCreateChannel not ported |
| Pre-defined channels | IM | -- | base/world_entry_chat.rs:20-29 | **Corrected 2026-07-25.** All 8 canonical channels (say/emote/yell/team/squad/command/server=7/tell=9) are auto-joined on world entry and pushed as `onChatJoined` (world_entry_appearance/builders.rs:89). `chatJoin` is acknowledged as a no-op (dispatch/chat.rs:113-125). **No cross-player routing on the non-spatial channels yet** |
| AFK / DND status | IM | -- | base/dispatch/chat.rs:82-90, 147-219 | `dnd_message` sets `SPEAKER_DND` on outgoing messages, matching `Chat.py::getSpeakerFlags`; stored text truncated to 128 chars (#739). `chatSetAFKMessage` is acknowledged but the auto-reply is not implemented (dispatch/chat.rs:134-145). The 2026-09-18 playtest logged one `chatSetDNDMessage: WSTRING decode failed` WARN (appendix-session-timeline.md line 124), not yet explained |
| Channel ops | KM | -- | -- | setPlayerOp not ported |
| Chat flood protection | KM | -- | -- | No rate limiting |
| Profanity filter | KM | -- | -- | No filtering |
| Mute system | KM | -- | -- | No per-player muting |
| GM broadcast | KM | Admin | -- | No system-wide message tool. GM feedback rides the `tell` channel to the caller only (cell/chat.rs:33-40) |

### 22. Trading --- IM (ported 2026-06; was KM)

- **Confidence**: HIGH (code re-read 2026-09-25; no trade code changed since 2026-07-25)
- **Documentation**: [gameplay/trade-system.md](gameplay/trade-system.md), [reverse-engineering/findings/trade-wire-formats.md](reverse-engineering/findings/trade-wire-formats.md)
- **Rust code**: [`crates/services/src/cell/cell_methods/player/trade/`](../crates/services/src/cell/cell_methods/player/trade/) — 3,166 lines (handlers 463, state 292, handoff 212, wire 202, mod 49 + 1,948 lines in 6 test modules); [`crates/services/src/base/world_entry/methods/trade/`](../crates/services/src/base/world_entry/methods/trade/) — 2,441 lines (execute/mod.rs, execute/swap.rs + commit / whitelist / slot-reservation live-DB guards); [`crates/entity/src/trade.rs`](../crates/entity/src/trade.rs) (491 lines)
- **Recent PRs**: **#438 (player-to-player trading system, closes #54)**; since 2026-07-25: #737 (players in a shared world can see each other — a prerequisite for targeting a trade partner in-client; not two-client validated)
- **Why IM and not CW**: unit + live-DB + wire coverage, never driven through two live clients. Before #737 (2026-09-19) two players could not see each other at all, so no in-client trade was possible. Open security findings: #467 (CAT-H, 10 findings).
- **Path forward**: Two-client smoke on post-#737 `main`; trade-request spam throttle (see Rate Limiting); work through #467.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Trade initiation | IM | -- | trade/handlers.rs:59 | `TRADE_REQUEST` → `begin_trading` (state.rs:28); partners re-checked for range each transition (`partners_in_range`, state.rs:275) |
| Proposal update | IM | -- | trade/state.rs:140 | `apply_proposal` (handler at handlers.rs:176); stale-version proposals rejected |
| Lock state machine | IM | -- | trade/handlers.rs:271-463 | None → Locked → LockedAndConfirmed, with an explicit truth table for which transitions reset the partner's lock |
| Confirmation | IM | -- | trade/handlers.rs | Commit fires only when both sides reach LockedAndConfirmed |
| Item swap | IM | Inventory | trade/execute/swap.rs:41 | `atomic_swap` — advisory lock, `SELECT … FOR UPDATE`, two-phase parked-row move, destination slot reservation; bound items refused (swap.rs:319) |
| Cash swap | IM | -- | trade/execute/swap.rs:251 | `read_naquadah_for_update`; balances validated before the delta is applied |
| Cancel | IM | -- | trade/state.rs:222 | `cancel_session`; `TRADE_REQUEST_CANCEL` arm at handlers.rs:40 |
| Disconnect cleanup | IM | -- | trade/state.rs:247 | `cancel_trade_on_disconnect`; regression guard at cell/service/base_messages/tests/trade_disconnect.rs |

## Stub-Only Systems

### 23. Organizations / Guilds --- KM

- **Confidence**: HIGH that nothing exists (code re-read 2026-09-25)
- **Documentation**: [gameplay/organization-system.md](gameplay/organization-system.md), [reverse-engineering/findings/organization-wire-formats.md](reverse-engineering/findings/organization-wire-formats.md)
- **Rust code**: [`crates/services/src/cell/cell_methods/organization.rs`](../crates/services/src/cell/cell_methods/organization.rs) (162), [`crates/services/src/cell/client_methods/organization.rs`](../crates/services/src/cell/client_methods/organization.rs) (38): 200 lines of handler stubs. All 12 inbound cell methods (indices 8-19) decode their arguments and log `UNIMPLEMENTED`. `onOrganizationCreation` logs `UNIMPLEMENTED` at cell_methods/player/social.rs:62. No `sgw_organization` table under `db/sgw/`.
- **Recent PRs**: none since 2026-07-25
- **Open issues**: #568 (implement organization / squad / guild system)
- **Path forward**: DB schema (`sgw_organization` table) + full org lifecycle (#568).

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Organization creation | KM | DB schema | stub | `UNIMPLEMENTED: onOrganizationCreation` (cell_methods/player/social.rs:62) |
| Invite/accept | KM | Creation | stub | `UNIMPLEMENTED: organizationInviteResponse` (organization.rs:36) |
| Leave organization | KM | -- | stub | `UNIMPLEMENTED: organizationLeave` (organization.rs:44) |
| Rank system (9 ranks) | KM | Creation | -- | EORG_RANK_None through Leader |
| Permission system (26 perms) | KM | Ranks | -- | Bitmask in enums |
| MOTD | KM | Creation | stub | organization.rs:94 |
| Officer notes | KM | Ranks | stub | organization.rs:108 |
| Rank name customization | KM | Ranks | stub | organization.rs:135 |
| Permission editing | KM | Ranks | stub | organization.rs:122 |
| Cash transfer to bank | KM | Creation | stub | organization.rs:155 |
| Organization vault | KM | Creation, Inventory | -- | INV_TeamBank, INV_CommandBank |
| Squad loot mode | KM | Groups | stub | organization.rs:143 |
| Minimap ping | KM | -- | stub | organization.rs:60 |
| Strike teams | KM | -- | stub | organization.rs:73 |
| PvP org leave | KM | -- | stub | organization.rs:86 |

### 24. Mail --- IM (read side only; sending, attachments and COD are stubs)

- **Confidence**: HIGH (code read line by line 2026-09-25). **The previous edition over-stated this section.** Send, attachment, take-cash, take-item, return and COD all route to `UNIMPLEMENTED` log lines; only the read side (headers, body, delete, archive) touches the database.
- **Documentation**: [gameplay/mail-system.md](gameplay/mail-system.md), [reverse-engineering/findings/mail-wire-formats.md](reverse-engineering/findings/mail-wire-formats.md)
- **Rust code**: [`crates/services/src/base/world_entry/methods/mail/`](../crates/services/src/base/world_entry/methods/mail/) (939 lines: mod.rs 316, tests.rs 623 live-DB), [`crates/services/src/cell/mail.rs`](../crates/services/src/cell/mail.rs) (412, cell-to-base hop + wire serializers), [`crates/services/src/cell/cell_methods/mail.rs`](../crates/services/src/cell/cell_methods/mail.rs) (99, dispatch of cell methods 43-51), [`crates/services/src/cell/client_methods/mail.rs`](../crates/services/src/cell/client_methods/mail.rs) (10)
- **Recent PRs**: none since 2026-07-25
- **Open issues**: #72 (mail: send, receive, attachments, COD)
- **Note**: nothing on `main` writes to `sgw_gate_mail`. The only server-side sender, the Black Market expiry sweep's `send_mail_to_player`, is on the unmerged `feat/571-black-market-phase1` branch ([game-systems.md](game-systems.md) Mail section agrees).
- **Path forward**: `sendMailMessage` with sender-side item/cash escrow, then take-item / take-cash, COD payment, return-to-sender, a `bArchive` filter on the header query, and new-mail fanout to an online recipient.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Open mailbox (headers) | IM | -- | base/world_entry/methods/mail/mod.rs:67-141 | `SELECT ... FROM sgw_gate_mail WHERE character_id = $1`, then `onMailHeaderInfo`. **The query ignores `b_archive`**, so archived mail still lists in the inbox. Live-DB test `mail_inserted_for_character_b_is_queryable_via_request_headers_select`. Re-verified 2026-09-25 |
| Read mail body | NT | -- | mail/mod.rs:143-226 | Ownership-checked `SELECT message`, stamps `read_time` once, sends `onMailRead`. Live-DB tests for read-time stamping and cross-character isolation (tests.rs:383-536). Re-verified 2026-09-25 |
| Send mail | KM | -- | stub | `sendMailMessage` (CM 44) logs `UNIMPLEMENTED` (cell_methods/mail.rs:33-36); no INSERT path exists. The earlier "live-DB tests" note was wrong: the tests seed rows by hand. Re-verified 2026-09-25 |
| Delete mail | NT | -- | mail/mod.rs:228-268 | Ownership-checked DELETE, zero-row WARN, `onMailHeaderRemove`. Live-DB test `delete_only_affects_target_character_not_account_siblings`. Re-verified 2026-09-25 |
| Archive mail | IM | -- | mail/mod.rs:270-313 | Sets `flags \| 1` (idempotent, live-DB tested), but the header query never filters on it, so archiving has no visible effect beyond removing the row until the next refresh |
| Attach item | KM | Send, Inventory | stub | Nothing to attach to: `sendMailMessage` is a stub. Re-verified 2026-09-25 |
| Attach gold | KM | Send | stub | Same. `cash` is read back in headers but never written by the server |
| Cash on Delivery | KM | Send, Receive | stub | `payCODForMailMessage` (CM 51) logs `UNIMPLEMENTED` (cell_methods/mail.rs:90-95) |
| Take item from mail | KM | Inventory | stub | `takeItemFromMailMessage` (CM 50) parses mail/container/slot then logs `UNIMPLEMENTED` (cell_methods/mail.rs:74-88). Re-verified 2026-09-25 |
| Take cash from mail | KM | -- | stub | `takeCashFromMailMessage` (CM 49) logs `UNIMPLEMENTED` (cell_methods/mail.rs:67-72). Re-verified 2026-09-25 |
| Return to sender | KM | Send | stub | `returnMailMessage` (CM 47) logs `UNIMPLEMENTED` (cell_methods/mail.rs:52-58) |
| New mail notification | KM | Send | -- | No fanout when recipient online |
| Mail expiry/TTL | NU | DB | -- | No TTL in schema |

### 25. Black Market (Auction House) --- KM

- **Confidence**: HIGH (re-read against `main` 2026-09-25)
- **Documentation**: [gameplay/black-market.md](gameplay/black-market.md), [reverse-engineering/findings/black-market-wire-formats.md](reverse-engineering/findings/black-market-wire-formats.md), [reverse-engineering/findings/black-market-restoration.md](reverse-engineering/findings/black-market-restoration.md), [reverse-engineering/findings/black-market-client-window-patch.md](reverse-engineering/findings/black-market-client-window-patch.md)
- **Rust code on `main`**: [`crates/services/src/cell/cell_methods/black_market.rs`](../crates/services/src/cell/cell_methods/black_market.rs) (80) + [`crates/services/src/cell/client_methods/black_market.rs`](../crates/services/src/cell/client_methods/black_market.rs) (14) — **94 lines of handler stubs, unchanged since 2026-07-25**; all six cell methods (61–66) log `UNIMPLEMENTED`.

> **Still unmerged (checked 2026-09-25).** `feat/571-black-market-phase1` is PR **#586, still OPEN**, last updated 2026-06-22. The branch tip `6dc1b6c6` (2026-06-21) is not an ancestor of `main`; it is 13 commits ahead and 395 behind. No other black-market PR has merged since 2026-07-25 (`gh pr list --search "black market"`; `git log` on both stub files is empty since then). Every row stays `KM`/`NU` and the totals do not count the branch.
>
> **Client-side blocker too.** The client never binds the BM client methods into its player dispatch map, so a server-sent `onBMOpen` (method 90) is silently dropped. The window opens only with the runtime patch described in the client-window-patch finding; launcher integration is open issue #587. Merging #586 alone will not produce a usable in-client black market.

- **Path forward**: Rebase and land #586; integrate the client-window patch in the launcher (#587); then buyout, my-auctions / my-bids views, listing fees, and transaction mail (depends on Mail sending, §24).

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Search listings | KM | DB schema | stub | Sort enums defined; `UNIMPLEMENTED: BMSearch` |
| Create listing | KM | DB, Inventory | stub | 5 duration tiers; args parsed and logged only |
| Place bid | KM | Listing, Economy | stub | -- |
| Buyout | KM | Listing, Economy | stub | -- |
| Cancel listing | KM | Listing | stub | -- |
| View my auctions | KM | Listing | stub | -- |
| View my bids | KM | Listing | stub | -- |
| Auction expiry | KM | Scheduler | -- | Timer-based cleanup (exists only on #586) |
| Listing fees | NU | Economy | -- | Standard MMO pattern |
| Transaction mail | KM | Mail | -- | Results via mail |

### 26. Contact Lists --- CW (shipped 2026-06-20; was KM)

- **Confidence**: HIGH. Code re-read 2026-09-25 (no runtime change since 2026-07-25), and the system is owner-confirmed working in-game as of 2026-06-20 (`project_confirmed_working_2026_06.md`).
- **Documentation**: [gameplay/contact-list.md](gameplay/contact-list.md), [reverse-engineering/findings/contact-list-wire-formats.md](reverse-engineering/findings/contact-list-wire-formats.md)
- **Rust code**: [`crates/services/src/base/contact_list/`](../crates/services/src/base/contact_list/): 2,156 lines across `handlers/` (header_ops, member_ops, presence_fanout), `persistence/` and `wire.rs`; dispatch at [`base/world_entry/cell_dispatch/contact_list_dispatch.rs`](../crates/services/src/base/world_entry/cell_dispatch/contact_list_dispatch.rs); cell side at [`cell/cell_methods/contact_list/`](../crates/services/src/cell/cell_methods/contact_list/) (520). The earlier "2,851 lines" figure does not match either tree.
- **Schema**: `sgw_contact_list`, `sgw_contact_list_member`, and the `list_id` sequence, all under [`db/sgw/Social/`](../db/sgw/Social/) with seed data
- **Recent PRs**: **#572 / #574 (schema, login-push, client methods 55-60, presence), #578 (`eventId` is a bitfield; LoggedInStatus is 1, not 0), #579 (GainLevel / Death / GateTravel), #581 (initial presence to the logging-in player), #583 (light up an already-online contact on add)**. None since 2026-07-25.
- **Open issues**: #471 (security audit CAT-L covers chat and contact list)
- **Path forward**: Nothing outstanding at the feature level. Work through the #471 audit findings.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Create list | CW | -- | contact_list/handlers/header_ops.rs:22 | `handle_create` → `persistence::create_list`; system lists ensured at login (`ensure_system_lists`) |
| Delete list | CW | -- | header_ops.rs:79 | `persistence::delete_list`; `onContactListDelete` wire builder in wire.rs |
| Rename list | CW | -- | header_ops.rs:144 | `persistence::rename_list` |
| Update flags | CW | -- | header_ops.rs:231 | `persistence::update_flags`; byte layout pinned in wire.rs |
| Add members | CW | -- | handlers/member_ops.rs:23 | `persistence::add_members`, capped at `MAX_MEMBERS_PER_REQUEST = 100` (wire.rs:88) |
| Remove members | CW | -- | member_ops.rs:124 | `persistence::remove_members` |
| Online status events | CW | -- | handlers/presence_fanout.rs | `fanout_login_status` fired from base/dispatch/session.rs:88 (logout) and world_entry_appearance/client_ready/mod.rs:453 (login). `EVENT_LOGGED_IN_STATUS = 1`, a **bitfield**, not an ordinal (#578) |
| Level-up events | CW | -- | base/world_entry/methods/progression/mod.rs:214 | `fanout_contact_event` with `EVENT_GAIN_LEVEL = 2` |
| Death events | CW | -- | cell/abilities/death/mod.rs:71-98 | Cell→Base `ContactListPresenceEvent` with `EVENT_DEATH = 4`; **player deaths only**. NPC deaths skip the fanout to avoid flooding the channel during combat |
| Gate-travel events | CW | -- | base/world_entry/gate_travel/mod.rs:416 | `EVENT_GATE_TRAVEL = 8`; `dataValue` carries the destination world id |

### 27. Dueling --- KM

- **Confidence**: HIGH that nothing exists (code re-read 2026-09-25)
- **Documentation**: [gameplay/duel-system.md](gameplay/duel-system.md), [reverse-engineering/findings/duel-wire-formats.md](reverse-engineering/findings/duel-wire-formats.md)
- **Rust code**: **No dedicated duel module.** `sendDuelResponse` (CM 102) and `duelForfeit` (CM 103) log `UNIMPLEMENTED` in `cell_methods/player/social.rs:92-101`. Client-method constants for `onDuelChallenge` (143) and `onDuelEntitiesSet/Remove/Clear` (151-153) exist in `cell/client_methods/player.rs:93-114` but nothing emits them.
- **Recent PRs**: none since 2026-07-25
- **Open issues**: #569 (implement duel system)
- **Path forward**: 5-state machine port; 7 defeat-condition enum (#569).

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Duel challenge | KM | -- | -- | State: ResponsePending. No challenge method is dispatched |
| Duel response | KM | -- | stub | social.rs:92-97 |
| Duel start | KM | Combat | -- | StartPending → Engaged |
| Duel forfeit | KM | -- | stub | social.rs:100-101 |
| Defeat conditions | KM | Combat | -- | 7 types |
| Duel marker entity | KM | -- | -- | SGWDuelMarker not ported |

### 28. Pets --- KM

- **Confidence**: HIGH that nothing exists (code re-read 2026-09-25)
- **Documentation**: [gameplay/pet-system.md](gameplay/pet-system.md), [reverse-engineering/findings/pet-wire-formats.md](reverse-engineering/findings/pet-wire-formats.md)
- **Rust code**: No dedicated pet module. `petInvokeAbility`, `petAbilityToggle` and `petChangeStance` decode their args and log `UNIMPLEMENTED` (`cell_methods/player/social.rs:15-55`). The pet entity extends SGWMob in Python; no Rust equivalent.
- **Recent PRs**: none since 2026-07-25
- **Open issues**: #570 (implement pet / companion system)
- **Path forward**: Pet entity (extends spawner mob), Follow AI state, command handling (#570). The NPC follow tick now covers being-class followers (#791), which a pet port could reuse.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Pet ability list sync | KM | -- | -- | -- |
| Pet stance list sync | KM | -- | -- | -- |
| Invoke pet ability | KM | Combat | stub | social.rs:15-25 |
| Toggle pet ability | KM | -- | stub | social.rs:31-41 |
| Change pet stance | KM | -- | stub | social.rs:47-55 |
| Pet following | KM | NPC AI (Follow) | -- | Needs a pet owner; the follow state itself exists for escort NPCs |
| Pet combat AI | KM | NPC AI | -- | Inherits SGWMob |

### 29. Minigames --- IM (was KM)

- **Confidence**: HIGH (code re-read 2026-09-25; Livewire played in client 12 times at the 2026-09-18 colo playtest)
- **Documentation**: [gameplay/minigame-system.md](gameplay/minigame-system.md), [reverse-engineering/findings/minigame-wire-formats.md](reverse-engineering/findings/minigame-wire-formats.md)
- **Rust code**: [`crates/services/src/minigame/`](../crates/services/src/minigame/mod.rs), 3,442 lines including 592 of tests: `server/` (751 non-test: `mod.rs`, `framing.rs`, `handshake.rs`, `result_dispatch.rs`), `session.rs` (764, ticket registry + TTL sweep), `protocol.rs` (443, SFS codec), `game.rs` (45), `games/livewire/` (742 + tests), `games/placeholder.rs` (68). Also [`cell/cell_methods/minigame.rs`](../crates/services/src/cell/cell_methods/minigame.rs) (174, all player-facing methods still stubs), [`base/world_entry/cell_dispatch/minigame.rs`](../crates/services/src/base/world_entry/cell_dispatch/minigame.rs) (130, ticket + seed issue) and the content `start_minigame` arm.
- **Recent PRs**: **#652 (CA04: pending-session TTL sweep, abort on SWF close reports result 0, `difficulty` 1-5, Livewire replay guards)**, #609 (GM-gate the minigame debug quartet)
- **In-client record**: 2026-09-18 colo playtest, "12 sessions, all `game_name=Livewire`; 11 victories, 1 abort (DHD, 01:28:08)". Each victory fired its follow-on chain in client-visible ways (1017, 1042, 1061, 1234, 1293, 1357: step advances, mission 641 completed, Data Crystal granted). The abort path ("client closed without reporting a result") was exercised and the retry succeeded.
- **Path forward**: Port Alignment and GoauldCrystals (factory TODOs at games/mod.rs:13-15; the playtest noted the DHD repair "should have the crystal minigame"). Stop the unknown-name fallback to the auto-win placeholder (#66). Player-initiated start, helper calls, spectating and contacts need client RE (#66 items 4-6). DoS hardening: read timeout, connection cap, one live connection per ticket (#532).

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Minigame session start | CW | -- | cell/content/executor/mod.rs:299, base/world_entry/cell_dispatch/minigame.rs, minigame/session.rs | **IM → CW 2026-09-25.** Chain `start_minigame` → ticket + session registry → SWF launch, seen 12 times in client (2026-09-18 playtest). #652 added the 180 s pending TTL and abort-on-close, and the abort + retry was also seen in that session. Code citation corrected: the `cell_methods/minigame.rs` player methods are stubs. Re-verified 2026-09-25 |
| Seed generation | CW | -- | base/world_entry/cell_dispatch/minigame.rs:37 | **IM → CW 2026-09-25.** Server-issued random seed builds the Livewire board that the client rendered and the server validated: 11 in-client victories (playtest). Re-verified 2026-09-25 |
| Result callback | CW | -- | minigame/server/result_dispatch.rs | **IM → CW 2026-09-25.** Victory results reached the cell and fired `on_victory_chains` on every one of the 11 wins; the abort reported result 0 (playtest timeline 00:05:11, 00:08:35, 00:10:37, 00:22:59, 00:34:13, 01:28:16). Re-verified 2026-09-25 |
| Mission integration | CW | Missions | content-engine `start_minigame` / victory chains | **IM → CW 2026-09-25.** Livewire wins advanced Cellblock and Castle missions in client (e.g. chain 1061: dialog 3998, 641 completed, 680 accepted; chain 1293: Data Crystal). Re-verified 2026-09-25 |
| 8 game types | IM | -- | minigame/games/ | Livewire fully implemented and client-verified. Hack / Activate / Analyze / Bypass / Converse / ConverseBasicHumanoid resolve to `PlaceholderGame` (accepts any input). Alignment and GoauldCrystals are commented-out TODOs (games/mod.rs:13-15). Unknown names silently auto-win (#66) |
| Spectating | KM | -- | cell/cell_methods/minigame.rs | `requestSpectateList` / `spectateMinigame` log `UNIMPLEMENTED` |
| Co-op / help | KM | -- | cell/cell_methods/minigame.rs | `registerToMinigameHelp`, call accept/decline/abort log `UNIMPLEMENTED` |
| Contact system | KM | -- | cell/cell_methods/minigame.rs | `minigameContactRequest` logs `UNIMPLEMENTED` |
| Minigame server (SmartFox 1.x) | CW | -- | minigame/server/ | **IM → CW 2026-09-25.** In-process SFS listener (`API_VERSION = 154`) completed handshake and play with the real Flash SWF 12 times (playtest). Open hardening, not function: no read timeout or connection cap, and re-auth of a connected ticket is still accepted (#532, #470). Re-verified 2026-09-25 |

### 30. Groups / Parties --- KM

- **Confidence**: HIGH that nothing exists (code re-read 2026-09-25)
- **Documentation**: [gameplay/group-system.md](gameplay/group-system.md), [reverse-engineering/findings/group-wire-formats.md](reverse-engineering/findings/group-wire-formats.md)
- **Rust code**: None. The unwired 97-line `Group` / `LootMode` sketch in `crates/game/src/social/groups.rs` was deleted by #699 (closing issue #614); `crates/game/src/social/` no longer exists. `squadSetLootMode` logs `UNIMPLEMENTED` (cell_methods/organization.rs:140-143).
- **Recent PRs**: #699 (deleted the unused social module)
- **Open issues**: #568 (organizations, including the Squad type)
- **Path forward**: Implement as a lightweight Squad-type Organization on top of #568.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Group creation | KM | -- | -- | EORG_TYPE_Squad = 0 |
| Group invite | KM | -- | -- | -- |
| Group leave | KM | -- | -- | -- |
| Member info sync | KM | -- | -- | 9 EMEMBER_INFO types defined |
| Loot mode setting | KM | Loot | stub | squadSetLootMode, organization.rs:143 |
| Group combat assist | KM | Combat, NPC AI | -- | onGroupMateEnteredCombat. (NPC-side same-room assist aggro, #789, is NPC-to-NPC and unrelated) |
| Threat transfer | KM | NPC AI | -- | onGroupMateThreatTransfer |

## Systems New Since the Original Audit (March 2026)

These didn't exist in the deprecated Python codebase and so weren't in the audit. They're substantial in Rust today.

### 31. Content Engine --- CW

- **Confidence**: HIGH (re-read 2026-09-25)
- **Documentation**: [content/content-engine.md](content/content-engine.md), [content/extending-the-engine.md](content/extending-the-engine.md), [architecture/data-driven-content-engine.md](architecture/data-driven-content-engine.md), [content/dialog-ui-client-contract.md](content/dialog-ui-client-contract.md)
- **Rust code**: [`crates/content-engine/`](../crates/content-engine/) (7,630 lines, 254 tests) + [`crates/services/src/cell/content/`](../crates/services/src/cell/content/) (43,386 lines, of which about 11,200 is non-test and 27,618 is `chain_replay_tests/`; 707 tests). 277 seeded chains across 11 files in `db/resources/Content/Seed/`.
- **Recent PRs**: **#618 (`MoveEntity` + `GrantXP` arms)**, **#619 (`launch_ability` / `apply_effect` server-authoritative entry point)**, #646-#671 (Cellblock rebuild: `destroy_tagged_entity`, `player_flanked_npc`, cover triggers), #659/#660/#668 (Castle), **#662/#682 (spawn/despawn actions, health trigger, world condition, `grant_stargate_address`, `mission_abandoned`, step-activation region replay)**, #663 (`stargate_dialed` / `stargate_crossed`), #748 (cover replay on step activation), #755 (per-key delivery of new Kismet sequences, so `play_sequence` can target new ids), **#769/#772 (`npc_bark`)**, #768 (dialog button linter)
- **In-client record**: the 2026-09-18 colo playtest drove about 22 missions through chains on two characters. Region, interact, entity-death, dialog-choice, cover, minigame-victory and deferred (`delay_ms`) chains all fired, and the timeline logs `matched` for each.
- **Path forward**: Dispatch the `effect_*` triggers so `effects_chains.sql` and the `apply_effect` arm can run (#610); `remove_effect`, `start_timer` / `cancel_timer`, `roll_loot_table`; persist counters to `content_counters`; the `system_message` wire format (#268); client UAT of the barks and the new triggers.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Trigger / condition / action chain | CW | -- | content-engine/chain.rs | Castle Cellblock and Castle 701-706 end to end in client (2026-09-18 playtest) |
| Loader (DB → Action enum) | CW | -- | content-engine/loader/ | Boundary validation here (e.g. `start_minigame` difficulty range-checked at load, #652) |
| Executor (action → side effects) | CW | -- | cell/content/executor/ | Dispatched arms covered by chain-replay tests that now run through `execute_actions` (#618 pattern) |
| Event dispatch | CW | -- | cell/content/event_dispatch/ | OnEntityDeath, OnInteract, OnDialog, region, cover, minigame victory: client-verified. Triggers added since 2026-07-25 (`stargate_dialed/crossed`, `mission_abandoned`, `player_flanked_npc`, health) are not yet seen in a client |
| Mission-context populator | CW | -- | cell/content/mission_context.rs | -- |
| Chain replay tests | CW | -- | cell/content/chain_replay_tests/ | Pins observed chain behavior (50 modules) |
| Action::ApplyEffect / RemoveEffect | IM | #610 | cell/content/executor/mod.rs:644, content/effect_apply.rs | **KM → IM 2026-09-25.** `ApplyEffect` and `LaunchAbility` arms wired (#619). The only seeded `apply_effect` row sits on an effect-scoped chain that no dispatched trigger reaches (#610). `RemoveEffect` has no arm. Playtest: `launch_ability` 1597 fired but was a no-op because the effect is scriptless. Re-verified 2026-09-25 |
| Action::StartTimer / CancelTimer | KM | -- | -- | Still no arm (falls to `other =>`). Per-action `delay_ms` deferral (executor/deferred.rs) covers one-shot delays only |
| Action::GrantXP | NT | Content | cell/content/executor/mod.rs:512 | **KM → NT 2026-09-25.** Loader + executor arms (#618, closes #611). Refuses `amount == 0`, error-logs send failure, and has an executed chain-replay guard. Zero seed rows, so it has never fired in a client. Re-verified 2026-09-25 |
| Persistent counters | KM | -- | content_counters table | Counters still live in `CellEntity::counters` in memory (executor/counter.rs); the table is only read and written by the admin editor |
| NPC barks (non-modal NPC lines) | NT | -- | cell/content/executor/bark.rs | **New 2026-09-25.** `npc_bark` action sends a dialog screen's text as `onPlayerCommunication` (method 28) on `say`, with no window (#769). Marsh escort lines use it in chains 1176-1178 (#772). Byte-exact wire test; not relogged-replayed by design. UATPending (Cellblock UAT T32). Re-verified 2026-09-25 |

### 32. Mercury Bundle / ChannelBundle --- CW

- **Confidence**: HIGH (re-read 2026-09-25; the only post-July activity is open issue #733 against `bundle.rs`)
- **Documentation**: [architecture/mercury-bundle.md](architecture/mercury-bundle.md), [architecture/transport-trait.md](architecture/transport-trait.md)
- **Rust code**: [`crates/mercury/src/channel_bundle/`](../crates/mercury/src/channel_bundle/mod.rs) (split into `bundle/` + `channel/` by #538), [`crates/mercury/src/bundle.rs`](../crates/mercury/src/bundle.rs)
- **Recent PRs**: #361 (ChannelBundle + AoI burst), #363 (bundle onClientReady), #365 (bundle progression + teleport), #410 (backpressure), #538 (module split)
- **Path forward**: Fix the latent >64 KiB clamp in the legacy `Bundle::encode`/`decode` (#733). This is not the ChannelBundle path.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Cross-entity bundling | CW | -- | channel_bundle/ | -- |
| AoI burst bundling | CW | -- | PR #361 | -- |
| onClientReady appearance/chat bundling | CW | -- | PR #363 | -- |
| Progression + teleport bundling | CW | -- | PR #365 | -- |
| Backpressure handling | CW | -- | PR #410 | -- |

### 33. Observability Pipeline --- CW

- **Confidence**: HIGH (re-read 2026-09-25: `server/src/logging/`, `cell/console/bookmark.rs`, `cell/player_journal.rs`, `cell/playtest_friction*.rs`, `cell/service/npc_ai/detectors/`; evidence in the NPC-AI UAT-1 worknote and the SigNoz mining record)
- **Documentation**: [architecture/observability.md](architecture/observability.md), [architecture/instrumentation-discipline.md](architecture/instrumentation-discipline.md), [operations/signoz-deployment.md](operations/signoz-deployment.md), [operations/signoz-remote-access.md](operations/signoz-remote-access.md), [operations/telemetry.md](operations/telemetry.md), [operations/npc-ai-telemetry-runbook.md](operations/npc-ai-telemetry-runbook.md) (new), [operations/signoz/](operations/signoz/) (dashboard JSON + saved views, new), [architecture/negative-logging-convention.md](architecture/negative-logging-convention.md), [analysis/playtests/2026-09-18-colo-castle/telemetry-design.md](analysis/playtests/2026-09-18-colo-castle/telemetry-design.md)
- **Rust code**: OTLP exporter in [`crates/server/src/otel.rs`](../crates/server/src/otel.rs) and [`crates/server/src/logging/`](../crates/server/src/logging/) (now a directory: `filters`, `parity_tests`, `target_scan_tests`); metrics facade [`crates/observability/`](../crates/observability/) (435 lines); Mercury packet instrumentation in [`crates/mercury/src/instrumentation.rs`](../crates/mercury/src/instrumentation.rs); playtest tooling in [`cell/console/bookmark.rs`](../crates/services/src/cell/console/bookmark.rs), [`cell/player_journal.rs`](../crates/services/src/cell/player_journal.rs), [`cell/playtest_friction.rs`](../crates/services/src/cell/playtest_friction.rs) + `playtest_friction_watch.rs`; NPC AI detectors in [`cell/service/npc_ai/detectors/`](../crates/services/src/cell/service/npc_ai/detectors/); negative-logging convention enforced by the `LogCapture` test helper
- **Recent PRs**: #396, #398, #400, #402, #404, #410, #414, #483 (full pipeline); **#676 (`.bug` bookmark, stuck-player detectors, NPC nav seams)**, **#678 (remaining stuck-player detectors + outbound-intent logging)**, **#679 (cover + trigger-ordering seams)**, **#680 (per-player journal, deferred-action reports, `npc_ai.tick`)**, **#700 / #726 (navmesh observability + review fixes)**, **#776 (NPC AI transition helper, aggro cause, OTLP identity)**, **#781 (NPC AI stuck/float/path/LoS/leash/cover detectors)**, **#782 (NPC AI health dashboard, 9 saved views, runbook)**, **#792 (disk-to-SigNoz parity: `cimmeria-trace` index, sampled firehoses)**, #791 (`.bug` witness fix)
- **Path forward**: The colo still reports `cimmeria.deploy_env = dev` because Watchtower does not re-apply the compose file. This is an owner action (UAT-1 finding 9). Filter on `service.version` until it is fixed. Also: confirm the friction detectors fire in a real session, and populate or confirm the NPC AI dashboard panels.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| OTLP log appender | CW | -- | server/otel.rs, server/logging/ | PR #398. Colo sessions reconstructed from SigNoz: playtest 2026-09-18 (~110k rows), NPC-AI UAT-1 (2026-09-25) |
| Hot-path tracing spans | CW | -- | mercury/, base/, cell/ | PR #398 |
| Mercury packet logging | CW | -- | mercury/instrumentation.rs | Per-packet OTLP |
| Wire-log capture stream | CW | -- | PR #404 | Per-message decode to SigNoz. Since #792 the AoI position-update firehose is sampled 1-in-101 to SigNoz and stays complete on disk |
| SigNoz self-hosted overlay | CW | -- | operations/signoz-deployment.md | ClickHouse-backed |
| Cloudflare Tunnel + Access | CW | -- | operations/signoz-remote-access.md | No inbound ports |
| Dev-session telemetry | CW | -- | architecture/dev-session-telemetry.md | HMAC-signed token. Mint quota added by #740 |
| Negative-logging convention | CW | -- | architecture/negative-logging-convention.md | LogCapture regression-guard helper. Now includes the credential-field rule (#698) and cross-IP seams (#738) |
| `.bug` playtest bookmark + per-player journal | CW | -- | cell/console/bookmark.rs, cell/player_journal.rs | **New 2026-09-25.** #676/#680, witness fix #791. `.bug <note>` snapshots the tester's scene (`playtest.bookmark` + `playtest.bookmark.entity`) and attaches the last 24 `player.journal` events. Written in-client record: 19 NPC-relevant bookmarks from colo sessions 2026-09-19..21 (analysis/npc-ai-restoration/evidence/signoz-npc-mining.md §A), and the owner's `.bug` notes drove UAT-1 (worknotes/uat-1.md). UAT-1 finding 5 (empty witness list) was fixed by #791 |
| NPC AI telemetry + anomaly detectors | CW | -- | cell/service/npc_ai/detectors/, npc_ai.tick | **New 2026-09-25.** #680/#776/#781: `npc_ai.tick`, aggro cause, off-mesh/stuck/float/path/LoS/leash/cover detectors. UAT-1 (worknotes/uat-1.md, 2026-09-25, colo, build `059d6038`) diagnosed findings 1-8 from these rows (`npc_ai.off_mesh`, `cover.flank_check`, `los_policy`). Finding 8 (idle tick volume) was fixed by #791 |
| Stuck-player friction detectors | NT | -- | cell/playtest_friction.rs, playtest_friction_watch.rs | **New 2026-09-25.** #676/#678: `repeat_interact_no_effect`, `repeat_item_use_no_chain`, `console_reject_streak`, `escort_separated`, `escort_leader_teleported`, plus outbound-intent logging. Unit-tested. No written record of a detector firing in a live session |
| NPC AI health dashboard + saved views | NT | -- | docs/operations/signoz/ | **New 2026-09-25.** #782: 11-panel SigNoz dashboard and 9 saved Logs Explorer views, created via MCP, with re-importable JSON and an operator runbook. The PR states that most panels populate only after the first colo session on an NA00/NA02 build. No record of the dashboard itself being read after a session |
| Disk-to-SigNoz log parity (`cimmeria-trace`) | NT | -- | server/logging/ | **New 2026-09-25.** #792: a third, TRACE-only SigNoz service carries ~70 formerly disk-only `trace!` sites. Prime-N sampled firehoses (`DECRYPT_OK`, `UDP_IN` 1-in-53; the `UDP_IN` sample carries no hex). 14 previously invisible DEBUG targets exported. `parity_tests` + `target_scan_tests` guard it. Merged 2026-09-25, not yet exercised in a recorded session |

### 34. Wireclient + Network Chaos Testing --- IM

- **Confidence**: HIGH (re-read 2026-09-25; `crates/wireclient` has no commits since 2026-07-25)
- **Documentation**: [architecture/wireclient.md](architecture/wireclient.md), [architecture/network-chaos-testing.md](architecture/network-chaos-testing.md), [architecture/mercury-loopback-harness.md](architecture/mercury-loopback-harness.md)
- **Rust code**: [`crates/wireclient/`](../crates/wireclient/): 1,947 lines across 8 files (auth, handshake, session_trace, client, error, lib + 2 test files), **30 tests**; LossyTransport in mercury; loopback harness for Tier 2; 9 chaos scenarios under `mercury/src/test_harness/tests/chaos/`
- **Recent PRs**: #370 (Tier 2 loopback harness), #374 (network chaos L1+L2+L3), #376 (Tier 3 wireclient scaffold), **#716 (flaky `tx_window_overflow` chaos scenario fixed: 7/30 → 30/30 under load, #713)**
- **Path forward**: Phase 1.5 socket loop (`client.rs:56-59`), then pcap replay against a live server (#281). Also: `Credentials`/`AuthSession` derive `Debug`, so one `{:?}` log would leak the password hash or session key (#698 follow-up).

> **Correction, 2026-07-25 (still true 2026-09-25): wireclient cannot send a UDP packet.** There is **no `UdpSocket` anywhere in `crates/wireclient`**. The only textual hit is a doc comment at handshake.rs:92. `Client::connect()` does not exist, and `client.rs:56-59` says Phase 1 "stops at *produce the bytes*". The Tier 2 loopback and chaos rows below are unaffected.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| SOAP auth client (Phase 1+2) | IM | -- | wireclient/src/auth.rs | 357 lines, driven against an in-process `AuthService` over real TCP by tests/auth_smoke.rs. This is a live SOAP client, not replay |
| Mercury phase-3 handshake | IM | Socket loop | wireclient/src/handshake.rs | 546 lines: `build_baseapp_login` + reply parser. It produces and consumes bytes but **cannot perform a handshake**, because nothing sends them |
| Pcap+key replay | KM | Socket loop | tools/pcap_to_session.py only | The Python tool converts `.pcap` + `keys.txt` → JSONL. No replay engine exists on either side |
| Session-trace JSONL | IM | -- | wireclient/src/session_trace.rs | 567 lines: `Trace::from_jsonl_path`, c2s/s2c iterators, `Diff` + `DefaultPolicy`. 10 tests + tests/trace_load.rs |
| LossyTransport drop/dup/reorder/latency | CW | -- | mercury/lossy_transport.rs | -- |
| Loopback paired-channel tests | CW | -- | mercury/test_harness/ | 22 tests |
| Network-chaos scenarios | CW | -- | mercury/test_harness/tests/chaos/ | 9 scenarios incl. `replay_lomiada`, `sustained_5pct_loss_60s`, `tx_window_overflow_with_recovery`. The last was flaky until #716 |

### 35. Discord Notifications --- CW

- **Confidence**: HIGH (re-read 2026-09-25; `crates/discord` has no commits since 2026-06-19)
- **Documentation**: [architecture/discord-notifications.md](architecture/discord-notifications.md)
- **Rust code**: [`crates/discord/`](../crates/discord/): 6,079 lines, 76 tests (config/, embed/, event/, sender/, layer.rs, router.rs, color.rs)
- **Recent PRs**: #397 (notification crate + tracing-layer harvest + panic hook), #540 (module split), #554 (account names, new gameplay events, player IPs dropped), #560 (config test coverage). No change since 2026-07-25. #676 routes `.bug` notes to the GM channel through the existing console audit relay

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| EventKind enum | CW | -- | discord/src/event/ | Variant-count pinning test |
| Channel routing | CW | -- | discord/src/router.rs | channel_for() |
| Embed formatting | CW | -- | discord/src/embed/ | format_event() |
| Panic hook capture | CW | -- | discord/src/ | -- |
| Per-channel toggles | CW | -- | EventToggles | -- |
| Colo deploy wiring | CW | -- | docker/compose.discord.yml | -- |

### 36. Tauri Admin App + Tools --- IM

- **Confidence**: HIGH for admin-api (every route file read for TODO/"not implemented" stubs 2026-09-25); MEDIUM for the Tauri front ends (no changes since 2026-07-25, not re-read)
- **Documentation**: [tools/admin-api.md](tools/admin-api.md), [tools/admin-panel.md](tools/admin-panel.md), [client/sgw-launcher.md](client/sgw-launcher.md), [architecture/live-research-lab.md](architecture/live-research-lab.md), [guides/live-research-lab.md](guides/live-research-lab.md), [engine/ue3-package-format.md](engine/ue3-package-format.md#writing-packages--the-append-only-patcher), [analysis/ring-transport-cellblock-castle/README.md](analysis/ring-transport-cellblock-castle/README.md)
- **Rust code**: [`crates/admin-api/`](../crates/admin-api/) (axum REST + WS, 5,217 lines); `src-tauri/` (admin panel app, `cimmeria-app`); [`tools/ContentEditor`](../tools/ContentEditor/), [`tools/SceneEditor`](../tools/SceneEditor/) (Tauri); [`crates/launcher/`](../crates/launcher/) (sgw-launcher, egui, 7,252 lines); [`crates/upk/`](../crates/upk/) (4,724 lines, incl. `patcher/` + `upk_patch` CLI); live research lab: [`crates/lab/`](../crates/lab/), [`crates/lab-mcp/`](../crates/lab-mcp/) (1,187), [`crates/client-launch/`](../crates/client-launch/) (6,606 lines together), plus the `lab-bridge` feature of `crates/client-telemetry`
- **Recent PRs**: **#724 (admin API binds 127.0.0.1 by default until JWT lands, #439)**, **#740 (dev-session mint quota)**, **#751 / #753 (UPK append-only patcher + Kismet rig cloning, ring transport Phases 0-1)**, **#692 / #696 / #706 / #693 / #695 / #703 (live research lab 1-6/6)**
- **Path forward**: JWT middleware and login (#439, #25). Entity WebSocket stream (#27). Entity, player, content and config endpoints still return "not implemented" in places (#26/#28/#29). Tighten CORS from `Any`. Live validation of the research lab exit criteria. Ring transport Phase 1 in-client test. Three.js space viewer.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| REST admin API | IM | JWT | crates/admin-api/routes/ | Real: players, spaces, audit, editor, telemetry, dev-session. Stubs returning "not implemented": entities.rs:38/62/86, content.rs:73/94/352, config.rs:105, editor.rs:553/604 (issues #26/#28/#29 open). Binds loopback by default since #724 |
| WebSocket entity stream | KM | -- | admin-api/ws/entity_stream.rs | **Re-verified 2026-09-25 (was IM).** Stub only: `handle_entity_socket` accepts the upgrade and logs, and its body is three `TODO` comments (entity_stream.rs:26-30). Issue #27 open |
| WebSocket log stream | IM | JWT | admin-api/ws/log_stream.rs, ws/broadcast_layer.rs | Real: ring-buffer replay + live broadcast. Unauthenticated (`/ws/logs` is named in #439) |
| JWT auth for remote | KM | -- | admin-api/middleware.rs:19, routes/auth.rs:41-81 | **Re-verified 2026-09-25 (was IM).** No code: the middleware is a `// TODO` block, and `/api/auth/login`, `/logout` and `/me` return `"not implemented"`. `jsonwebtoken` is declared but unused. Critical issue #439 is open (re-rated P3 on 2026-09-25 because the colo edge keeps 8443 private); #25 open. #724 is containment only |
| Content editor (Tauri) | IM | -- | tools/ContentEditor | React + xyflow visual chain editor |
| Scene editor (Tauri) | IM | -- | tools/SceneEditor | -- |
| Admin panel (Tauri) | IM | -- | src-tauri/ | -- |
| SGW launcher (egui) | CW | -- | crates/launcher/ | Seed + patch manifest, Ed25519 signed |
| Three.js space viewer | KM | -- | -- | Phase 2 of the admin UI plan |
| UPK append-only package patcher | CW | -- | crates/upk/src/patcher/, bin/upk_patch.rs | **New 2026-09-25.** #751. Written in-client record: ring-transport README "Phase 0 status", **2026-09-19 PASSED**. The owner loaded CellBlock with the patched stasis-hall chunk, and the cloned ring station rendered lit and at floor height |
| UPK Kismet rig cloning | NT | -- | crates/upk/src/patcher/ (`clone_objects`) | **New 2026-09-25.** #753 (re-land of #752): clones region 3's 32-object ring rig onto the Armory pad (62 new exports). README "Phase 1 status" says **built, awaiting the in-client test** (`Install-Phase1.ps1 rig`, then `.net_seq 10187 3`) |
| Live research lab: client bridge, native probes, supervisor | IM | -- | crates/client-telemetry (`lab-bridge`), crates/lab/, crates/client-launch/ | **New 2026-09-25.** #692/#696/#706/#703. Lua eval bridge, dynamic hooks and memory write/call probes, crash-recovery supervisor with autologin, merged `lab_timeline`. The PR bodies record that each exit criterion still needs live validation on the owner's box. The Lua C API export and the console-exec address are unconfirmed (#706), and autologin screen reads depend on `lua_eval` capture (#696). Issues #684-#690 still open |
| Live research lab: in-server MCP endpoint | NT | -- | crates/lab-mcp/ | **New 2026-09-25.** #693/#695: console passthrough, sessions, logs, read-only SQL, LabQuery snapshots, witness reports, packet taps. The end-to-end AoI reproduction is left to a UAT step (#695). Colo exposure is WireGuard-only (#703) |

### 37. Ring Transport --- IM

**Added 2026-07-25.** Cross-region and cross-world transporter rings. A player steps onto a ring pad and picks a destination. The server then drives a multi-second state machine: it plays Kismet sequences at both ends, hides the players, teleports them and shows them again.

- **Confidence**: HIGH (re-read 2026-09-25)
- **Documentation**: [gameplay/ring-transport-system.md](gameplay/ring-transport-system.md) (includes "Bounded aborts"), [analysis/ring-transport-cellblock-castle/README.md](analysis/ring-transport-cellblock-castle/README.md) (mission 688 client-patch plan), [engine/ue3-package-format.md](engine/ue3-package-format.md) (append-only patcher)
- **Rust code**:
  - [`crates/services/src/cell/ring_transport/`](../crates/services/src/cell/ring_transport/) has **5,856 lines**: 3,191 production and 2,665 test. It spans `regions.rs` (349), `transporter/{mod, manager, source, destination, effects}`, `runtime/{entry, tick, teardown}`, `dispatch.rs`, `wire.rs` and `wire_helpers.rs`.
  - Chains enter through the content action at `cell/content/executor/transport.rs:14`, which calls `handle_interact`.
  - Sequence delivery for the new rig is in `base/sequence_overrides.rs` (169).
  - The package patcher is `crates/upk/src/patcher/` (2,213).
  - `python/cell/RingTransporter.py` remains the spec for the state graph and timings.
- **Recent PRs**:
  - Server:
    - **#662 / Harset H02**: bounded FSM stall timeouts, departing-player cleanup, four FSM review fixes, ring-arrival validation.
    - **#755**: sequences 10187 / 10188 delivered per key.
    - **#754**: Kismet PAK version restored. #753 had wiped every client's sequence table.
  - Client patch:
    - **#750**: 688 ring-ceremony audit.
    - **#751**: package writer, Phase 0.
    - **#752 / #753**: Kismet rig clone onto the Armory pad, Phase 1.
- **In-client record**: P1 rode the Cellblock rings four times with a real client: regions 1 → 2 and 2 → 3 on both characters. In each case the chain triggered the transporter and the client returned a destination selection. The FSM ran, the player was teleported to the destination pad (`TeleportPlayer` snap `[-192.66, 55.26, -154.84]` = region 2), `teleport_in` fired, and mission 640 completed or 680 advanced ([appendix-session-timeline.md](analysis/playtests/2026-09-18-colo-castle/appendix-session-timeline.md) rows 00:08:37, 00:11:31, 01:10:12). That build predates #662. The timeout and cleanup changes are unit-tested only. Separately, the Phase 0 client-patch load test passed on 2026-09-19 (the PR #751 owner comment).
- **Why still IM**:
  - No record says the ring *animation* or the hide/show was seen.
  - `setRingTransporterDestination` does not check that the caller is on or near the pad (#461 CAT-B-03).
  - Cross-world ring travel (Omega Site ↔ Command Center, regions 14/17) has never run.
  - The mission 688 ceremony is still a direct `cross_world_teleport`.
- **Path forward**:
  - Close CAT-B-03.
  - Run the Phase 1 in-client test: repair the client `Cache.en-US` Kismet PAK, then `.net_seq 1951 3` at region 3 and `.net_seq 10187 3` at the Armory pad.
  - Phase 2 (Castle pad) and Phase 3 (route chain 1109 through the FSM).
  - Run an Omega cross-world ring smoke.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Ring region loading | CW | -- | ring_transport/regions.rs | **Promoted 2026-09-25.** `ring_transport_regions` → `RingRegion`, 30 seeded rows. In-client: P1's hops landed on the seeded region coordinates, for example region 2 at `(-192.657, 55.258, -154.844)` ([appendix-session-timeline.md](analysis/playtests/2026-09-18-colo-castle/appendix-session-timeline.md) 01:10:12). Re-verified 2026-09-25 |
| Destination list to client | CW | -- | ring_transport/wire.rs; dispatch.rs:201 | **Promoted 2026-09-25.** `interact()` emits `SendDestinationList` → `onRingTransporterList`. In-client: in all four P1 hops the client answered with a valid `setRingTransporterDestination`, which the FSM requires before it leaves Idle (`validate_destination`, transporter/mod.rs:397). The trips completed (appendix rows 00:08:37, 00:11:31). Re-verified 2026-09-25 |
| Transport state machine | CW | -- | ring_transport/transporter/mod.rs | **Promoted 2026-09-25.** 454 lines; the manager is transporter/manager.rs. In-client: P1 trigger → teleport_in in 18 s (00:08:37 → 00:08:55), four successful intra-world trips over two characters, each followed by the chain that expects arrival. Caveat: tested on the 2026-09-18 build. #662's stall timeouts landed after and are covered by unit tests (tests/stall.rs, deadline_scan.rs). Re-verified 2026-09-25 |
| Kismet sequence playback | NT | -- | ring_transport/wire_helpers.rs | `onSequence` at both origin and destination (event sets 10000/874/875). Sent in P1, but no record says the animation was seen. #754 fixed a regression (#753's PAK version bump) that would have stopped every ring animation for any client connecting to that release. **Status changed 2026-09-25 (was IM).** |
| Hide / show + movement lock | NT | -- | ring_transport/wire_helpers.rs | `onVisible`, `onStateFieldUpdate`, `BSF_MOVEMENT_LOCK`. The abort path releases only `ShowPlayer` / `UnlockMovement` (`dispatch_release_effects`). Sent in P1; not recorded visually. **Status changed 2026-09-25 (was IM).** |
| Region-trigger entry | IM | -- | ring_transport/runtime/entry.rs | `handle_interact` (:38), `handle_select_destination` (:94), `handle_region_trigger` (:276). All three ran in P1's hops. **Stays IM on a recorded defect:** `setRingTransporterDestination` never checks that the caller is on or near the source pad or in its world, so any client can start or grief any ring. Open in #461 (CAT-B-03). Re-verified 2026-09-25 |
| Cross-world ring travel | NT | -- | ring_transport/dispatch.rs:126; runtime/tick.rs | **Changed 2026-09-25 (was IM).** `Effect::TeleportCrossWorld` goes through GateTravel. The destination waits for `AdvanceRingDestination` or `REMOTE_LOAD_WAIT_TIMEOUT` (90 s, a judgement value), and the traveller is not stranded if the source is destroyed (tests/disconnect.rs:93). One seeded route (Omega Site 14 ↔ Cmd Center 17) has never run in-client. The Cellblock → Castle exit does **not** use it (chain 1109 is a direct `cross_world_teleport`) |
| Stall timeouts + departure cleanup | NT | -- | ring_transport/transporter/mod.rs:105-114; runtime/teardown.rs | **New 2026-09-25.** Bounded per-state stall timeouts (`SEND_WAIT` 60 s, `RECV_WAIT` 65 s, `RECV_WARMUP` 15 s, `REMOTE_LOAD_WAIT` 90 s). The 2009 server had none. A departing or disconnecting player is removed from the ring (`forget_player`), and aborts release the lock and visibility without firing arrival content (#662 H02). Unit and negative-log tested; not client-exercised |
| CellBlock → Castle ring ceremony (mission 688 client patch) | IM | -- | crates/upk/src/patcher/; base/sequence_overrides.rs; db/resources/Events/Seed/sequences.sql (10187/10188) | **New 2026-09-25.** Owner-approved exception to "no client patch". Phase 0 **passed in-client** on 2026-09-19: the cloned station rendered, lit and at floor height (PR #751 owner comment). Phase 1 is built: region 3's Kismet rig is cloned onto the Armory pad, and sequences 10187 / 10188 are delivered per key (#753, #755). Its in-client animation test is pending. Phases 2–3 (Castle pad, route chain 1109 through the FSM) have not started, and the exit is still a direct teleport |

## Server Infrastructure (Cross-Cutting)

### Session Management --- IM

- **Confidence**: HIGH (code re-read 2026-09-25)
- **Documentation**: [architecture/server-infrastructure-proposals.md](architecture/server-infrastructure-proposals.md) §1 (session-resume design), [protocol/login-handshake.md](protocol/login-handshake.md) (cross-IP session binding). [architecture/server-systems.md](architecture/server-systems.md) is the superseded survey this section replaced.
- **Rust code**: `crates/services/src/base/connect_loop/` (per-client lifecycle), `crates/services/src/base/login/mod.rs` (Phase 3 + duplicate eviction), `crates/services/src/base/tick_sync.rs`, `crates/services/src/auth/`
- **Recent PRs**: #711 (inactivity constants split, closes #293), #738 (login sessions bound to issuing IP, warn-only, closes #442), #698 (stop logging full SIDs / tickets / SOAP body), #756 (position persisted on logout)
- **Open issues**: #460 (security audit CAT-A, auth / session / character lifecycle)

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Inactivity timeout | IM | -- | mercury/lib.rs:135, base/tick_sync.rs:84 | Two layers since #711: `MERCURY_PEER_DEAD_MS = 300_000` (Mercury peer-dead bookkeeping) and a 60 s tick-sync client reap. `UE3_INACTIVITY_TIMEOUT_MS = 15_000` documents the client-side tolerance only. Faster reaping is an open owner decision. Re-verified 2026-09-25 |
| Duplicate login check | IM | -- | base/login/mod.rs:89-134 | **Corrected 2026-09-25.** Runs at Phase 3 login, not character select: an existing session for the same account gets `LOGGED_OFF` and its entities are destroyed with reason `duplicate_login` |
| Developer mode bypass | IM | -- | auth/handlers.rs:129, 172 | **Corrected 2026-09-25 (was CW).** `developer_mode` skips the protocol-digest check and accepts credentials without a DB. It does **not** bypass duplicate-login eviction: base/login/mod.rs:89-134 never reads it, although the config doc comment promises "multi-login" (common/src/config.rs:97). No written test record backed the CW |
| Reconnection grace period | KM | -- | -- | Instant disconnect = session lost |
| Session token persistence | KM | -- | -- | No resume after network blip |
| Continuous auth validation | KM | -- | -- | Only at login |
| Login session IP binding | IM | -- | auth/mod.rs, auth/handlers.rs, base/login/mod.rs | **New 2026-09-25.** #738: `SessionRecord` / `PendingLogin` record the issuing IP; Phase 2 SID and Phase 3 ticket consumption log `session_ip_mismatch` / `ticket_ip_mismatch` WARNs. **Warn-only by design** until NAT false positives are measured, so nothing is refused yet |

### Rate Limiting --- KM

- **Confidence**: HIGH (code searched 2026-09-25: no chat, action, trade or login throttle anywhere in `crates/services`)
- **Recent PRs**: #740 (dev-session token mint and refresh quotas, closes #441)

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Ability cooldown enforcement | CW | -- | cell/abilities/ | Per-ability timers |
| Chat flood protection | KM | -- | -- | No rate limit on messages |
| Action throttling | KM | -- | -- | No per-action rate tracking |
| Trade request spam | KM | Trade | -- | No request cooldown |
| Login attempt limiting | KM | -- | -- | No brute-force protection on the auth service |
| Dev-session token mint quota | NT | -- | admin-api/src/routes/dev_session/quota.rs, handlers.rs | **New 2026-09-25.** #740: per-IP and per-`install_id` fixed-window mint quotas and a per-IP refresh quota (429 + `Retry-After`), bounded refresh chain, `telemetry.write` scope check. Handler-level tests; not exercised by a launcher session on record |

### Anti-Cheat Validation --- IM (was KM)

- **Confidence**: HIGH (code re-read 2026-09-25)
- **Recent PRs**: #437 / #478 (four-layer movement validation), #643 (jump apex no longer trips navmesh containment), #644 (snap-back rubber-band loop replaced by Rejected / Recovered / CorrectionSuppressed with a 5-correction budget; player validation honours `movementSpeedMod`), #639 (`onPhysics` fly/ghost GM bypass), #700 / #726 (diagnosed and throttled navmesh rejects), #741 (dead actors rejected before interaction dispatch), #791 (item use refused while `BSF_DEAD`)

Four layers of server-authoritative movement validation landed in PRs #437 and #478; see §7 for the detail. The 2026-09-18 colo playtest exercised them with the real client: 756 `movement.speed_warning` rows and a jump/snap-back defect, fixed by #643 and #644 ([appendix-session-timeline.md](analysis/playtests/2026-09-18-colo-castle/appendix-session-timeline.md) line 124). The remaining gap is damage-side sanity checking.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Position bounds check | IM | -- | entity/movement_validation/bounds.rs | AABB from the loaded navmesh with a generous fallback for navmesh-less spaces; catches NaN / infinity / Z-floor-clip |
| Ability target validation | IM | -- | cell/abilities/use_ability/handle.rs:182-260 | Target exists + alive; faction gate added by #444. Adjacent dead-actor gates on interaction (#741) and item use (#791) |
| Inventory ownership check | CW | -- | base/world_entry/methods/inventory | Live-DB regression guards |
| Speed hack detection | IM | -- | entity/movement_validation/mod.rs:18-22, 173 | Server-monotonic-clock `dt`, `top_speed × SPEED_WARN_TOLERANCE (1.5)`, now scaled by `movementSpeedMod` (#644). **Warn-only by design** until SigNoz telemetry calibrates the threshold |
| Teleport detection | IM | -- | entity/movement_validation/mod.rs:24-29 | Hard reject on the dual distance-AND-implied-speed gate. Recovery path rewritten by #644 after the playtest rubber-band loop; not re-tested in client since |
| Damage sanity check | KM | -- | -- | No max-damage cap |
| Action-at-distance exploit | IM | -- | cell/abilities/use_ability/handle.rs:238-262 | `useAbility` rejects targets beyond the ability's `max_range` (30.0 default) with `OutsideWeaponRange`. Player-side LOS is still *not* checked on this path; the #797 occluder LoS serves NPC AI only |

### Economy Sinks / Faucets --- IM

Re-read 2026-09-25. Every vendor-priced sink and faucet depends on a store window that could not open in-client until PR #609 (2026-07-26), and nothing has been client-tested since, so those rows drop to NT. Mission cash rewards do not exist at all (#310).

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Vendor buy/sell prices | NT | Vendors | base/world_entry/methods/vendor/ | **Demoted 2026-09-25 (CW → NT).** Static from DB; server-side live-DB + PL/pgSQL coverage only. PR #609 voids pre-2026-07-26 client observations of the store. Re-verified 2026-09-25 |
| Mission cash rewards | KM | Missions | -- | **Demoted 2026-09-25 (CW → KM).** No code grants cash on mission completion: the content executor has no cash action (only `GrantXP`, cell/content/executor/mod.rs:512), `chosenRewards` (CM 87) is a stub, nothing sends `onMissionRewardsDisplay`, and every seeded mission has `reward_naq = 0`. Open issue #310, re-verified by a comment on 2026-09-25: "Rewards still never dispatch". Re-verified 2026-09-25 |
| Loot cash drops | NT | -- | cell/interactions/loot.rs:218 | **Demoted 2026-09-25 (CW → NT)** to match §14 "Cash drops", which is the same code path. Naquadah rolls are recorded in the 2026-09-18 playtest; its arrival in the wallet is not. Re-verified 2026-09-25 |
| Repair costs | NT | Vendors | vendor/paid_repair/ | **Demoted 2026-09-25 (CW → NT).** Cost formula is covered by live-DB tests only; §15 already rates repair NT; #609 voids earlier client tests. Re-verified 2026-09-25 |
| Recharge costs | NT | Vendors | vendor/paid_recharge/ | **Demoted 2026-09-25 (CW → NT).** Same reasoning as Repair costs. Re-verified 2026-09-25 |
| AH listing fees | KM | Black Market | -- | -- |
| Cash flow tracking | KM | -- | -- | No currency ledger, counter or metric in `crates/`; design only ([server-infrastructure-proposals.md](architecture/server-infrastructure-proposals.md) §5) |

### World State Persistence --- IM

- **Confidence**: HIGH (code re-read 2026-09-25)
- **Recent PRs**: #756 (position persisted on logout), #663 (stargate open/cross events, transient only)

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Player position persistence | NT | -- | base/world_entry/cell_dispatch/position.rs | **Demoted 2026-09-25 (was CW).** #756 records that before it, logout never persisted position: only gate travel and GM teleport wrote `sgw_player.pos_*`, so returning characters spawned at the last gate arrival. #756 adds `CellToBaseMsg::PersistPosition` on disconnect with live-DB tests. No in-client relog test on record. The same PR notes an open "returning character hangs on world load" report |
| Cell event outbox | CW | -- | base/outbox/ | Durable Base→Cell |
| Space scripts | IM | -- | content-engine | Reset on restart |
| Gate state persistence | KM | DB | -- | Open/closed not saved; the #663 stargate events and 4 s dial timer are in-memory only |
| Door state persistence | KM | DB | -- | Not saved |
| World state table | KM | DB | -- | No sgw_world_state |

### Event / Scheduler System --- IM

- **Confidence**: HIGH (code searched 2026-09-25: no cron or global scheduler in `crates/`)

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Per-entity timers | IM | -- | content-engine | Per-chain timers wire through |
| Global event scheduler | KM | -- | -- | No cron-like system |
| Daily resets | KM | Scheduler | -- | -- |
| Holiday events | KM | Scheduler | -- | -- |

### Admin / GM Tools --- IM (GM command surface is CW; admin panel and dot-command parity still IM)

- **Confidence**: HIGH (code re-read 2026-09-25)
- **Documentation**: [analysis/legacy-command-parity/](analysis/legacy-command-parity/) (README, audit, work-packets), [tools/admin-api.md](tools/admin-api.md)
- **Rust code**: native GM methods `cell/cell_methods/gm/` (6,070 lines); GM dot-console `cell/console/` (12,730 lines, 89 registered dot commands, up from 71 at the 2026-09-16 audit baseline); `crates/admin-api/`
- **Recent PRs**: #609 (GM gate extended to the minigame debug quartet, CM 20-23), #635-#640 and #642 (legacy dot-command parity packets P01-P05, P08, P18, P26, P44-P47), #644 (case-insensitive world lookup, `.gotospace`, account identity on console logs), #749 (`.summon` always brings the player to the caller), #787 (`.aggro` toggle), #676 (`.bug` playtest bookmark; rejected console commands now logged), #724 (admin API binds loopback by default), #740 (dev-session quotas)
- **Open issues**: #439 (admin API has no authentication; the loopback bind is step 1 only), #473 (security audit CAT-N, 40 GM findings)

The GM command surface shipped in June via the client's **native `/` console**: the `SGWGmPlayer` class flip (PR #473, merged in #518 on 2026-06-17) makes a GM enter the world as entity class `0x03`, which unlocks the client's built-in GM command tail. Owner-confirmed working 2026-06-20. The **legacy dot-command parity campaign** has integrated 12 of 49 P packets (P01-P05, P08, P18, P26, P44-P47); P49 is implemented and awaiting UAT; P06, P07 and P48 are Ready; the other P packets wait on dependencies and all 14 G design groups (G01-G14) are BlockedDesign ([work-packets.md](analysis/legacy-command-parity/work-packets.md)). None of the six milestone UATs (M1-M6) has run. Ban/mute is still missing.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Admin API (REST) | IM | -- | crates/admin-api/ | Loopback bind by default (#724); still **no authentication** (#439 open), so it must not be published |
| Tauri admin panel | IM | -- | tools/ | Per-page features partial |
| Native GM console (SGWGmPlayer) | CW | -- | cell/cell_methods/gm/ | 6,070 lines across give / stats / missions / travel / spawn / query / world / feedback + tests. PRs #473 / #516 / #518 / #521 / #524. Owner-confirmed 2026-06-20 |
| Access level system | CW | -- | cell/dispatch/gm_gate.rs | `enforce_gm_gate` refuses the whole gated method range; #609 added the minigame debug methods 20-23 to the allow-list. Owner-confirmed 2026-06-20 |
| Python console | KM | -- | -- | C++ console not ported (intentional security) |
| Console commands | IM | -- | crates/commands/ | Generic command framework (registry / parser / permissions). **Not** the active dot roster: the live path is cell/chat.rs → cell/console/ (legacy-command-parity README, "Architecture Guardrails") |
| Dev/authoring `.`-console | IM | -- | cell/console/ | 12,730 lines, 89 registered dot commands. 12 of 49 parity packets integrated, milestone UATs pending. In-client record: the 2026-09-18 playtest ran `.speed`, `.gotoxyz`, `.location` and `.searchmission` successfully (12 of 20 accepted; appendix-session-timeline.md lines 76, 83, 123, 148). Re-verified 2026-09-25 |
| Player info lookup | IM | -- | admin-api/routes/players.rs, cell/console/query.rs | Plus `gmShowPlayer` / `gmUsers` / `testLOS`, and `.info` / `.players` (P02, P04; `.players` now CellApp-wide) |
| Ban/mute system | KM | -- | -- | No `GM_BAN` / `GM_MUTE` index and no handler anywhere in `crates/`. The admin API has a `/players/{id}/kick` route only |
| Teleport command | CW | -- | cell/cell_methods/gm/travel.rs, cell/console/travel/ | Native `gmGotoXYZ` / `gmGoto` / `gmSummon` / `gmGotoLocation` / `gmDHD`, plus the dot commands `.gotoxyz` / `.goto` / `.summon` / `.gotolocation` / `.gotospace` (P26, P44-P46, #644, #749). `.gotoxyz` confirmed in the 2026-09-18 colo playtest (appendix-session-timeline.md line 148, "Works as designed"). Re-verified 2026-09-25 |
| Item grant | CW | -- | cell/cell_methods/gm/give.rs | `gmGiveItem`, alongside give-xp / give-cash / remove-item / give-expertise / give-ASP; base-side confirmation. Owner-confirmed 2026-06-20. Dot `.giveitem` (P06) not yet built |
| Action logging | NT | -- | cell/console/dispatch.rs:47-116, cell/playtest_friction.rs | **Promoted 2026-09-25 (was IM).** Accepted commands log with `account_id` / `player_id` / `access_level` (#644) and relay to the Discord GM channel; rejections (unknown command, argc, bad target) now log with a `reason` (#676), closing playtest gap G7. The accepted-command audit reconstructed the 2026-09-18 playtest; rejection logging not yet seen in a session |
| Announcement broadcast | KM | Chat | -- | -- |

### Metrics / Telemetry --- CW

- **Confidence**: HIGH (code and PRs re-read 2026-09-25)
- **Documentation**: [architecture/observability.md](architecture/observability.md), [operations/telemetry.md](operations/telemetry.md), [operations/npc-ai-telemetry-runbook.md](operations/npc-ai-telemetry-runbook.md), [analysis/playtests/2026-09-18-colo-castle/telemetry-design.md](analysis/playtests/2026-09-18-colo-castle/telemetry-design.md)
- **Rust code**: `crates/server/src/otel.rs`, `crates/server/src/logging/` (mod, filters, parity_tests, target_scan_tests), `crates/mercury/src/instrumentation.rs`, `crates/services/src/cell/playtest_friction.rs`, `crates/services/src/cell/console/bookmark.rs`, `crates/services/src/cell/service/npc_ai/detectors/`
- **Recent PRs**: #676 / #678 / #679 / #680 (`.bug` bookmark, stuck-player detectors, outbound-intent logging, per-player journal), #700 / #726 (navmesh observability), #776 / #781 (NPC AI state transitions and detectors), #782 (NPC AI SigNoz dashboard + 9 saved views), **#792 (NA25: disk-to-SigNoz log parity, `cimmeria-trace` index, sampled firehoses)**, #620 (client-telemetry DLL observes dropped inbound methods; not runtime-verified)
- **In-client record**: [npc-ai-restoration/worknotes/uat-1.md](analysis/npc-ai-restoration/worknotes/uat-1.md) (owner session 2026-09-25 on the colo) was reconstructed from colo SigNoz filtered on `service.version` plus the owner's `.bug` notes. It found that `.bug` bookmarks listed no witnesses (fixed in #791) and that the colo still reports `deploy_env = dev` until the compose file is re-applied.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Category logging | CW | -- | tracing crate, server/logging/ | -- |
| OTLP export | CW | -- | server/otel.rs, server/logging/mod.rs | One `log_provider` helper since #792 |
| Mercury packet metrics | CW | -- | mercury/instrumentation.rs | -- |
| Player count tracking | IM | -- | admin-api/ | Exposed via REST |
| Performance metrics | IM | -- | tracing + SigNoz | Visible in dashboards |
| Custom dashboards | NT | -- | SigNoz, docs/operations/signoz/ | **Promoted 2026-09-25 (was IM).** #782 created the 11-panel "Cimmeria — NPC AI health" dashboard and 9 saved views, with re-importable JSON exports. The PR says most panels populate only after a colo session on an NA00/NA02 build; no record of the dashboard being read after UAT-1 |
| Cloudflare-Access remote ops | CW | -- | operations/signoz-remote-access.md | -- |
| Playtest `.bug` bookmark + stuck-player detectors | NT | -- | cell/console/bookmark.rs, cell/playtest_friction.rs | **New 2026-09-25.** #676 / #678: `.bug <note>` freezes a `playtest.bookmark` snapshot of the scene; once-per-episode `playtest.friction` detectors. The owner used `.bug` in UAT-1 (uat-1.md), which found the witness list empty; #791 fixed that, not re-tested |
| Disk-to-SigNoz log parity | NT | -- | server/logging/ | **New 2026-09-25.** #792 (NA25): a TRACE-only `cimmeria-trace` index, prime-N sampled firehoses (`sampled_1_in` / `suppressed`), 14 hidden DEBUG targets exported, `launcher.key_dump` pinned off. Parity and target-scan guards; not yet observed on the colo |

## Summary Completion Matrix

Recomputed 2026-09-25 directly from the feature rows above.

| # | System | Total | CW | NT | IM | KM | NU |
|---|--------|-------|----|----|----|----|-----|
| 1 | Authentication and Login | 13 | 8 | 1 | 2 | 2 | 0 |
| 2 | Mercury Protocol | 15 | 10 | 0 | 3 | 2 | 0 |
| 3 | Game Data Pipeline | 9 | 6 | 2 | 0 | 1 | 0 |
| 4 | Database Persistence | 8 | 6 | 0 | 0 | 2 | 0 |
| 5 | Character Creation | 11 | 4 | 4 | 1 | 2 | 0 |
| 6 | World Entry and Spaces | 10 | 7 | 2 | 1 | 0 | 0 |
| 7 | Movement and Navigation | 11 | 1 | 3 | 7 | 0 | 0 |
| 8 | Entity Lifecycle | 10 | 6 | 2 | 1 | 1 | 0 |
| 9 | Combat and Abilities | 24 | 6 | 0 | 14 | 4 | 0 |
| 10 | Effects and Buffs | 13 | 3 | 0 | 5 | 5 | 0 |
| 11 | Stats | 8 | 5 | 0 | 0 | 2 | 1 |
| 12 | Inventory and Items | 13 | 8 | 3 | 1 | 1 | 0 |
| 13 | Missions | 12 | 7 | 0 | 3 | 2 | 0 |
| 14 | Loot | 9 | 4 | 1 | 0 | 4 | 0 |
| 15 | Stores / Vendors | 8 | 1 | 6 | 1 | 0 | 0 |
| 16 | NPC AI and Behavior | 26 | 8 | 6 | 9 | 3 | 0 |
| 17 | Spawn System | 23 | 7 | 0 | 1 | 14 | 1 |
| 18 | XP and Leveling | 11 | 9 | 0 | 1 | 1 | 0 |
| 19 | Crafting | 9 | 0 | 0 | 2 | 7 | 0 |
| 20 | Stargate Travel | 10 | 2 | 4 | 3 | 1 | 0 |
| 21 | Chat | 10 | 0 | 1 | 2 | 7 | 0 |
| 22 | Trading | 8 | 0 | 0 | 8 | 0 | 0 |
| 23 | Organizations / Guilds | 15 | 0 | 0 | 0 | 15 | 0 |
| 24 | Mail | 13 | 0 | 2 | 2 | 8 | 1 |
| 25 | Black Market | 10 | 0 | 0 | 0 | 9 | 1 |
| 26 | Contact Lists | 10 | 10 | 0 | 0 | 0 | 0 |
| 27 | Dueling | 6 | 0 | 0 | 0 | 6 | 0 |
| 28 | Pets | 7 | 0 | 0 | 0 | 7 | 0 |
| 29 | Minigames | 9 | 5 | 0 | 1 | 3 | 0 |
| 30 | Groups / Parties | 7 | 0 | 0 | 0 | 7 | 0 |
| 31 | Content Engine | 11 | 6 | 2 | 1 | 2 | 0 |
| 32 | Mercury Bundle / ChannelBundle | 5 | 5 | 0 | 0 | 0 | 0 |
| 33 | Observability Pipeline | 13 | 10 | 3 | 0 | 0 | 0 |
| 34 | Wireclient + Network Chaos Testing | 7 | 3 | 0 | 3 | 1 | 0 |
| 35 | Discord Notifications | 6 | 6 | 0 | 0 | 0 | 0 |
| 36 | Tauri Admin App + Tools | 13 | 2 | 2 | 6 | 3 | 0 |
| 37 | Ring Transport | 9 | 3 | 4 | 2 | 0 | 0 |
| -- | Session Management | 7 | 0 | 0 | 4 | 3 | 0 |
| -- | Rate Limiting | 6 | 1 | 1 | 0 | 4 | 0 |
| -- | Anti-Cheat Validation | 7 | 1 | 0 | 5 | 1 | 0 |
| -- | Economy Sinks / Faucets | 7 | 0 | 4 | 0 | 3 | 0 |
| -- | World State Persistence | 6 | 1 | 1 | 1 | 3 | 0 |
| -- | Event / Scheduler System | 4 | 0 | 0 | 1 | 3 | 0 |
| -- | Admin / GM Tools | 13 | 4 | 1 | 5 | 3 | 0 |
| -- | Metrics / Telemetry | 9 | 4 | 3 | 2 | 0 | 0 |
| | **TOTALS** | **471** | **169** | **58** | **98** | **142** | **4** |

### Summary Percentages

Recomputed 2026-09-25 directly from the rows above; the columns sum to the totals line and the totals line sums to 471.

| Status | Count | Percentage |
|--------|-------|-----------|
| Confirmed Working (CW) | 169 | 35.9% |
| Needs Test (NT) | 58 | 12.3% |
| Implemented (IM) | 98 | 20.8% |
| Known/Missing (KM) | 142 | 30.1% |
| Needed/Unknown (NU) | 4 | 0.8% |

**Code exists (CW + NT + IM)**: 325 features (69.0%)
**Missing (KM + NU)**: 146 features (31.0%)

**Tested end-to-end (CW)**: 169 features (35.9%).

### What moved since 2026-07-25

The 2026-07-25 edition's matrix again disagreed with its own tables. Its rows sum to 444 / CW 164 / NT 18 / IM 134 / KM 124 / NU 4, while its totals line printed 443 / 159 / 18 / 134 / 128 / 4. The mismatches were in Authentication, Combat, Loot, Missions, NPC AI, Spawn and Mail. The comparison below is row against row.

| | CW | NT | IM | KM | NU | Total |
|---|---:|---:|---:|---:|---:|---:|
| 2026-07-25 rows | 164 | 18 | 134 | 124 | 4 | 444 |
| 2026-09-25 | 169 | 58 | 98 | 142 | 4 | 471 |
| **Delta** | **+5** | **+40** | **-36** | **+18** | **+0** | **+27** |

The headline percentages barely moved, for two opposite reasons. About 160 PRs landed, most of them tested and merged but not yet run against a live client, so NT roughly tripled. At the same time this pass read the code behind every row and demoted claims that did not hold up. The evidence bar for CW was a written record of an in-client test (playtest report, UAT worknote, PR or issue note).

- **Gained ground.** Character creation `0 CW → 4 CW` (two characters created and played in the 2026-09-18 colo playtest). Minigames `0 → 5 CW` (12 in-client Livewire sessions in the same playtest). Ring transport `7 IM → 3 CW / 4 NT / 2 IM` (four in-client Cellblock ring trips; stall timeouts and the mission 688 client-patch ceremony are new rows). NPC AI `+4 rows` and `+1 CW` (assist aggro, UAT-1 on 2026-09-25); proximity aggro, collision-geometry line of sight and grounding are new NT rows. Loot generation and damage application to CW (colo playtest). Stargate DHD, cancel, address discovery and multi-player sync to NT (#662, #663, #682). Observability and tools `+9 rows` for the `.bug` bookmark, NPC AI telemetry and dashboard, the log index, the UPK patcher (Phase 0 passed in-client 2026-09-19) and the live research lab.
- **Corrected down.** Spawn system `9 CW / 10 IM → 7 CW / 1 IM / 14 KM`: SpawnRegion/SpawnSet activation, population, set cooldowns, weighted tables and level ranges were attributed to `spawner/regions.rs`, which loads client-hinted trigger regions instead (#62). Mail `9 IM → 2 NT / 2 IM / 8 KM`: send, attachments, cash and COD are stubs that log "unimplemented". Economy `5 CW → 0 CW / 4 NT`: mission cash rewards are not paid at all (#310), and vendor rows lost CW when #609 found that earlier vendor testing had routed the vendor window to the mission handler; vendors have not been re-tested since. Effects `-1 CW, +3 KM`: no clear-on-damage, clear-on-revive or clear-on-bandolier-swap flags exist, and permanent vs non-permanent stat tracking does not exist. Database persistence `-1 CW`: there are no compile-time checked `sqlx::query!` macros; all queries are checked at runtime. Combat position/facing checks `IM → KM`. Player position persistence `CW → NT` (#756 found logout never saved position; the fix has no in-client relog test). Tauri tools: JWT auth for remote and the WebSocket entity stream are TODO stubs (`IM → KM`).
- **New rows (+27).** Mostly the systems above, plus Kismet sequence overrides (#755), dialog override patch mode (#767), NPC barks, same-world respawn resync (#756), per-world navmesh containment and coverage, player-to-player introduction (#737), login IP binding (#738), dev-session token quota (#740) and disk-to-SigNoz log parity (#792).

**Rows awaiting a tester.** Each reviewer listed rows that very likely work in-client but have no written test record. They are the fastest way to move NT to CW. The largest groups are the NPC AI changes merged after UAT-1, gate dial/open/cross with a second observer, the dialog-UI buttons and barks, two-client chat and player visibility, relog position, and vendors.

---

## What changed since the previous (deprecated-codebase) gap analysis

| Metric | Audit (Python+C++) | 2026-05-27 rows | 2026-07-25 rows | This pass (2026-09-25) | Why the change |
|---|---:|---:|---:|---:|---|
| Total features tracked | 369 | 428 | 444 | 471 | New rows for NPC AI (aggro, assist, line of sight, grounding), ring transport, observability and tools, navmesh containment, Kismet and dialog overrides |
| Confirmed Working | 31 (8.4%) | 151 (35.3%) | 164 (36.9%) | 169 (35.9%) | Character creation, minigames, ring trips, damage, loot and assist aggro moved in on the 2026-09-18 playtest and UAT-1; vendors, spawn population, economy and effect clear-flags moved out after a code re-read |
| Code exists (CW+NT+IM) | 175 (47.4%) | 260 (60.7%) | 316 (71.2%) | 325 (69.0%) | Needs Test tripled (18 → 58): most of the ~160 PRs since July are merged and tested but not yet run in a client |
| Missing | 194 (52.6%) | 168 (39.3%) | 128 (28.8%) | 146 (31.0%) | Rows that had been credited to the wrong code (spawn population, mail send and attachments, mission cash) were corrected to KM |

Every column is the sum of that edition's own rows, not the headline it printed. The 2026-05-27 totals line said 437 / CW 139 / KM 184 / NU 5, and the 2026-07-25 totals line said 443 / CW 159 / KM 128; neither matched its tables.

The shape of "done" as of this pass: Mercury, observability and the content engine are firmly done. Two zones (Castle Cellblock and Castle) have been played end to end in a client, and a third (Harset) has been rebuilt but not yet played. The NPC AI restoration campaign (NA00-NA33) replaced the aggro, leash, cover and line-of-sight logic, and every world now has a navmesh (#794). The biggest gap has shifted from *missing code* to *missing client tests*: 58 rows are merged and waiting for a tester. The long-tail social systems (organizations, dueling, pets, groups) are still stubs, mail can only read, and the black market is still queued behind an unmerged branch.

---

## Critical Path for Playability

Re-ranked 2026-09-25.

1. **Client-test the September landings** — 58 rows are NT: the NPC AI changes merged after UAT-1, gate dial/open/cross with a second observer, the dialog-UI buttons and barks, two-client chat and player visibility, relog position, and vendors (untested since #609). This is the cheapest way to move the headline number
2. **Effect-script content coverage** — the framework works but the long tail of the 3,216 effect rows still needs scripts, and the clear-on-damage / clear-on-revive / clear-on-bandolier-swap flags are not implemented. `cell/effects/scripts.rs` is 1,648 lines
3. **AoI invisible-entity defect** — a witness can be correctly introduced to an entity and still not render it (Castle Cellblock GuardBody corpse). The 2026-09-19 repro put the drop inside the client, after a fully ACKed delivery, and fixed the `OTEL_FILTER` gap that had kept `aoi.create_emit` out of SigNoz. The first-login cinematic hold ships as the experiment on the one remaining lead; this needs an in-game repro to confirm or kill it, not more code
4. **Mission rewards** — the `GrantXP` action exists (#618), but `mission.reward_xp` is 0 in every seed row and the reward formula needs a maintainer decision. Mission cash and item rewards are not dispatched at all (#310)
5. **Crafting Phase 2** — state and persistence landed (#427); every player-facing verb still logs `UNIMPLEMENTED`
6. **Multi-zone end-to-end** — Castle Cellblock and Castle have been played in a client (2026-09-18 colo playtest); Harset is rebuilt but unplayed; the other spaces have navmeshes but no content campaign
7. **Two-client verification** — trading, player-to-player introduction (#737) and chat between players have never been exercised with two clients

Quality-of-life items (organizations, mail polish, black market, dueling, pets, remaining minigame ports, groups) are still gated on the above but each can be picked up independently. GM tooling and contact lists have left this list.

---

## Cross-Reference Tables

### Documentation Exists but Rust Doesn't (port pending)

Corrected 2026-07-25 — trading and contact lists have left this table.

| System | Gameplay Doc | Wire Format Doc | Rust Code Status |
|--------|-------------|----------------|-------------------|
| Crafting | crafting-system.md | crafting-wire-formats.md | State + persistence ported (#427); all crafting verbs still stubs |
| Organizations | organization-system.md | organization-wire-formats.md | 200 lines stubs — unchanged |
| Black Market | black-market.md | black-market-wire-formats.md | 94 lines stubs on `main`; full Phase 1 waiting on `feat/571-black-market-phase1` |
| Dueling | duel-system.md | duel-wire-formats.md | Not ported |
| Pets | pet-system.md | pet-wire-formats.md | Not ported |
| Groups | group-system.md | group-wire-formats.md | Not ported |

### Rust Code Exists but Doc Lags

These have substantial Rust implementations the per-system docs haven't fully caught up on. P3-equivalent doc-refresh pending.

| System | Code Location | Doc Status |
|--------|--------------|-----------|
| Content Engine | crates/services/src/cell/content/ + crates/content-engine/ | docs/content/content-engine.md is the canonical reference but is currently labelled audience: "engineers" — could use a "what's done vs. planned" callout |
| Mercury Bundle | crates/mercury/src/channel_bundle.rs | docs/architecture/mercury-bundle.md is the ADR |
| Observability | crates/server/, crates/mercury/instrumentation.rs | docs/architecture/observability.md + operations/signoz-*.md |
| Wireclient | crates/wireclient/ | docs/architecture/wireclient.md — **verify this doc's Tier 3 claims**; the crate has no UDP socket (see §34) |
| Discord Notifications | crates/discord/ | docs/architecture/discord-notifications.md |
| Trading | crates/services/src/cell/cell_methods/player/trade/ + base/world_entry/methods/trade/ | **Added 2026-07-25.** docs/gameplay/trade-system.md still describes Python `Trade.py` as the implementation |
| Ring Transport | crates/services/src/cell/ring_transport/ | **Added 2026-07-25.** About 5,856 lines (3,191 production); docs/gameplay/ring-transport-system.md does not yet cover the mission 688 client-patch route |
| Cover system | crates/services/src/cell/cover/ | **Added 2026-07-25.** docs/game-systems.md still says "CoverSet entity is a stub" — corrected in that file on 2026-07-25 |
| NPC AI movement states | crates/services/src/cell/service/npc_ai/ | **Added 2026-07-25.** docs/gameplay/npc-ai.md predates PR #428 |
| GM command surface | crates/services/src/cell/cell_methods/gm/ + cell/console/ | **Added 2026-07-25.** About 6,070 + 12,730 lines (89 dot-commands as of 2026-09-25); no consolidated GM command reference |
| Minigame server | crates/services/src/minigame/ | **Added 2026-07-25.** docs/gameplay/minigame-system.md still describes an external SmartFox process |
| Movement validation | crates/entity/src/movement_validation/ | **Added 2026-07-25.** Four-layer anti-cheat with no ADR |

### Server-Only Blind Spots (Ranked by Gameplay Impact)

Re-ranked 2026-09-25. #5 (speed-hack detection) is implemented but deliberately warn-only; #7 (mission rewards) moved up after #310 confirmed missions pay neither cash nor items.

| Rank | System | Impact | Status | Notes |
|------|--------|--------|--------|-------|
| 1 | Crafting verbs | HIGH — entire skill tree unplayable | KM | Phase 1 state landed (#427); craft / research / RE / alloy / ASP-spend all log `UNIMPLEMENTED` |
| 2 | AoI entity-introduction drop | HIGH — entities silently invisible | IM | Known-open. Address-gate hypothesis disproved 2026-06-20; Mercury delivery retired 2026-09-19 (every create ACKed first try), so the drop is client-side. `aoi.create_emit` now actually exports to SigNoz. The first-login cinematic hold (#747) is the experiment on the n=1 cinematic lead, and as of 2026-09-25 nobody has recorded an in-game look since it shipped |
| 3 | Organizations / guilds | MEDIUM — no persistent social layer | KM | 200 lines of stubs, no schema |
| 4 | Rate Limiting | MEDIUM — exploitable | KM | No throttle on chat / trade-request / login. Trading shipped without a request cooldown, so this got *worse* |
| 5 | Speed-hack enforcement | MEDIUM — detection lands, action doesn't | IM | Layer is live but warn-only by design pending tolerance calibration from SigNoz |
| 6 | Damage sanity checking | MEDIUM — no max-damage cap | KM | The one anti-cheat layer with no implementation at all |
| 7 | Mission rewards | MEDIUM — missions pay nothing | KM | `GrantXP` action exists (#618) but no seed rows use it and `reward_xp` is 0 everywhere; cash and item rewards are never dispatched (#310) |
| 8 | Multi-zone verification | LOW — 2 zones played in client | NT | Castle Cellblock and Castle played 2026-09-18; Harset unplayed; other spaces have navmeshes only |
| 9 | `sequences_nvp` unread | LOW — cinematic sound-bank / params never reach the client | KM | `db/resources/Events/Seed/sequences_nvp.sql` seeds **2,042 rows** (SoundBankName and friends). No Rust code reads the table, and all six `onSequence` emit sites hardcode a NameValuePairs count of 0: abilities/damage_apply/mod.rs:351, abilities/use_ability/handle.rs:540 and :569, cell/console/net.rs:104, content/executor/mod.rs:122, ring_transport/wire_helpers.rs:42 |

---

## Related Documents

- [project-status.md](project-status.md) — human-readable summary of this analysis
- [../README.md](../README.md) — high-level project status
- [gameplay/](gameplay/) — per-system gameplay docs
- [content/](content/) — content audit + content engine
- [protocol/](protocol/) — wire formats
- [architecture/](architecture/) — server architecture and ADRs
- [reverse-engineering/](reverse-engineering/) — RE findings + Ghidra work
- [../CONTRIBUTING.md](../CONTRIBUTING.md) — how to pick a feature and ship it
