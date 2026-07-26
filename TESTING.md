# Testing Guide

> **Audience**: Engineers writing or reviewing tests in the Cimmeria workspace.
> **Type**: Reference + how-to.
> **Last updated**: 2026-07-25
> **Companion docs**: [docs/architecture/integration-test-infra.md](docs/architecture/integration-test-infra.md) (live-DB infra rationale and local setup), [CLAUDE.md](CLAUDE.md) (pre-PR checklist), [.github/copilot-instructions.md](.github/copilot-instructions.md) (review checklist).
> **See also**: [docs/testing/inventory/README.md](docs/testing/inventory/README.md) — catalogue of every test in the workspace (the "what tests exist" reference; this file is the "how to write a test" playbook).

The Rust workspace currently has **2,936 `#[test]` / `#[tokio::test]` cases across 461 files**. CI's exclude list drops the GUI crates (`cimmeria-app`, `cimmeria-content-editor`, `cimmeria-scene-editor`, `sgw-launcher`) and the Windows-only `cimmeria-client-telemetry` cdylib, leaving **2,691 tests actually gated on every PR**. Of those, 224 are live-DB regression guards (`require_db_or_skip!`, all in `cimmeria-services`) and 3 are end-to-end PL/pgSQL smoke scripts. Per-test catalogue lives at [docs/testing/inventory/](docs/testing/inventory/) — PRs that add or remove ≥5% of the workspace test count (~147 tests at the current 2,936 baseline) update it in the same PR; smaller drifts get folded in by periodic sweeps. CI gates every PR on five jobs — `cargo fmt --check`, `cargo clippy -D warnings`, `cargo build`, `cargo nextest run --profile=ci` (workspace, no DB), and `cargo nextest run --profile=ci-live-db -p cimmeria-services --lib` against a live `postgres:17.9` service container. A sixth `coverage` job runs `cargo llvm-cov` over both passes but is `continue-on-error: true` and does not gate merges. nextest emits JUnit XML which is uploaded to Codecov Test Analytics for per-test history and flake detection.

This guide is the playbook for writing tests that survive review and catch real regressions. **Read it before opening a PR that adds tests.**

---

## TL;DR

1. **Pick the right test type** for the bug shape. The taxonomy is below; the picker is in [Choosing a test type](#choosing-a-test-type).
2. **Pin the bug shape, not the happy path.** A regression guard must reproduce the bug if the fix is reverted. If the test still passes after `git revert`-ing the fix, it isn't a regression guard.
3. **Tighten assertions.** `== 1` beats `>= 1`; `(player_id, mission_id)` lookup beats filtering by `player_id` alone; exact final positions beat "two distinct positions".
4. **Don't trust seed data.** Re-fetch baseline values inside the test, or assert by relationship (`slot.cur_ammo_type == slot.default_ammo_type`) rather than by hard-coded id.
5. **Name tests by what they assert,** not by what the code under test happens to do today. Rename if the assertion changes.
6. **One feature can need multiple test types.** A new vendor handler typically needs a unit test (logic) + a wire-format test (serializer) + a live-DB regression guard (SQL) + a slot in the smoke script (cross-handler invariants).

---

## Test types we use

### 1. Unit tests (`#[test]` / `#[tokio::test]` inline)

**Where**: `#[cfg(test)] mod tests` in the same file as the code under test, or in a sibling `tests.rs` / `tests/` submodule when the host file approaches the 700-line cap.

**For**: Pure functions, normalizers, parsers, state machines, anything you can exercise without a network socket or a database. The large majority of the workspace's tests are this kind — everything not accounted for by the 224 live-DB guards, the 3 smokes, and the wire/fan-out/harness categories below.

**Patterns to follow:**
- One assertion focus per test. Multi-assertion tests are fine when they pin one invariant from several angles, but if the test name doesn't predict the assertion, split it.
- Reuse a small `make_state()` / `make_ctx()` helper rather than copying setup. When PR #150 split concurrency tests off, reviewers required reuse of the existing helper, not a parallel one.
- Cover the negative path: if the code returns early on `connected: empty`, write the test that constructs that empty map and asserts the early-return shape.

**Examples**: `crates/mercury/src/unpacker/` (20 tests, byte-level cursor edge cases); `crates/common/src/math.rs` (14 tests, vector/quaternion math); `crates/services/src/cell/combat/threat/` (27 tests, threat list state machine).

### 2. Wire-format tests

**Where**: Same module as the serializer; conventionally `crates/mercury/src/**/*.rs` and `crates/services/src/mercury/protocol/tests.rs`.

**For**: Anything that produces bytes the BigWorld client must accept. This is the single most "byte-exact" surface in the codebase — the client is unforgiving, and we have no way to renegotiate the protocol.

**Patterns to follow:**
- Assert the **exact byte string** the function emits, hand-written in the test as a `&[u8]` literal or a hex string. Don't assert "length is at least N" — that's trivially true.
- For frame builders, assert the offset of structurally meaningful footer fields (e.g., the `seq_id` footer position), not just the total frame length. PR #142 review caught a "length tautology" of exactly this shape.
- Round-trip both directions when the codec is symmetric (`build_x` then `parse_x` then assert equality of the input).
- Confirm method indices against `docs/protocol/client-method-dispatch-table.md` and byte layout against `entities/defs/*.def` before writing the test, not after.

**Examples**: `crates/mercury/src/packet/` (24 tests), `crates/services/src/base/world_entry/methods/vendor/serializers.rs` (12 byte-exact tests for the store payload), `crates/services/src/mercury/aoi/` (14 wire-layout tests for the AoI builders, split across `create.rs` and `tests.rs`).

### 3. Live-DB regression guards

**Where**: Same module as the handler being guarded, gated by `let pool = require_db_or_skip!();`.

**For**: SQL invariants that pure unit tests can't reach — `WHERE` clauses, `rows_affected` shapes, advisory locks, `ON CONFLICT` semantics, the `flags` column's role in vendor buyback, multi-character isolation. **Every Group A regression guard in PRs #143–#175 is this kind.**

**Patterns to follow:**
- Pick a **positive `0x7000_xxxx` sentinel base** for the module's test ids (e.g., `const TEST_BASE: i32 = 0x7000_0400;` for missions, `0x7000_1000` for character-list, `0x7000_0800` for vendor sell). Each module reserves its own slot in this range; the existing modules document neighbours in a doc-comment so the next contributor can step past them. See `crates/services/src/base/character/mod.rs:288-296` and `crates/services/src/base/world_entry/methods/missions.rs:150-154` for the canonical comment shape.
- The base must fit in `i32` because the `entity_id`/`account_id`/`player_id` columns are `INTEGER`. `0x7000_xxxx` does (it's well below `i32::MAX`); a `u32` like `0xDEAD_0000` wraps to a negative when bound `as i32` and lands in another module's territory — don't reach for high-bit constants.
- Run serialised. Under nextest the `ci-live-db` profile in `.config/nextest.toml` pins `threads-required = "num-test-threads"`, which makes each test claim every available thread; under raw `cargo test`, pass `-- --test-threads=1`. Even within the partitioned-range scheme, some guards share rows in `resources.*` and collide under parallel execution. CI enforces this; local repro must match.
- Cleanup must `DELETE WHERE <id> = $sentinel` (or `IN (...)` over the exact ids the test inserted), not a range predicate like `WHERE entity_id < 0` or `WHERE account_id BETWEEN base AND base+0xFF`. Range deletes can reach into a sibling module's slot if the partitioning ever drifts.
- For shared rows (resources.items inserts), use `ON CONFLICT DO NOTHING` so test B's insert doesn't conflict with test A's leftover, and **don't `DELETE` shared rows in cleanup** — let them leak for the next run.
- **Reproduce the bug shape.** A `handle_grant_cash` regression guard must seed two characters on the same account, grant to one, and assert the other's balance is unchanged. That's the shape the bug took (PR #143). A test that just grants and asserts the credit went through is a happy-path test, not a regression guard.

**Examples**: `crates/services/src/base/world_entry/methods/progression/tests.rs` (PR #143), `crates/services/src/base/world_entry/methods/vendor/sell/tests.rs` (PR #154 — pin the `flags` column's role as buyback unit price), `crates/services/src/base/character/mod.rs` (4 guards on `query_character_list`).

### 4. End-to-end PL/pgSQL smoke tests

**Where**: SQL script in `tools/<feature>_smoke.sql`, embedded into a `#[tokio::test]` in `crates/services/src/base/smoke_tests.rs` via `include_str!`.

**For**: Whole-stack invariants that span multiple handlers and would still pass each handler's own per-handler tests. Today's three:
- `vendor_store_smoke.sql` — sell → buyback → grant → purchase round-trip; catches drift between `handle_sell_vendor_items` and `handle_buyback_vendor_items` on the meaning of the `flags` column.
- `inventory_move_smoke.sql` — split → simple-move → three-step swap; pins the parking-at-sentinel procedure that dodges the `(character_id, container_id, slot_id)` unique index mid-swap.
- `progression_smoke.sql` — multi-character isolation for `handle_grant_cash` and `handle_grant_xp`.

**Patterns to follow:**
- Wrap the whole script in `BEGIN ... ROLLBACK` so seed data is byte-identical after a passing run.
- Assert via PL/pgSQL `RAISE EXCEPTION` — sqlx surfaces it as `Err`, the harness panics, and on failure the connection-release `ROLLBACK` still cleans up.
- The Rust harness is intentionally tiny: `include_str!` the SQL, strip `\set` psql-only directives, run via `sqlx::raw_sql(...).execute(&pool)`, assert `is_ok()`.
- Seed-data drift hardens the script: before asserting "naquadah == X", read X from the row first.

**When to add a smoke test (vs another live-DB unit test):** when at least two handlers share an invariant whose violation wouldn't fail either handler's own tests. The vendor smoke caught exactly that — both handlers internally consistent, round-trip price wrong.

### 5. Concurrency regression guards

**Where**: Sibling `concurrency_tests.rs` next to the per-handler `tests.rs` (the split is required by file caps; see PR #150).

**For**: Race conditions, TOCTOU between `SELECT` and `UPDATE`, advisory-lock correctness, multi-task `join!` behavior, outbox drainer-vs-caller interleaving.

**Patterns to follow:**
- A naked `tokio::join!` of two handler futures often serializes via the scheduler or DB and produces "the right answer" by accident. Add a barrier-coordinated start, or run the race in a small loop, so a no-lock regression actually fails.
- Wrap each `JoinHandle::await` in `tokio::time::timeout(...)` so a deadlock fails the test instead of wedging the suite.
- Capture and assert each spawned task's `Result`. Dropping the result lets an `Err` that left the DB in a state still satisfying the post-conditions become a false positive.
- For TOCTOU guards on `update_X WHERE type_id = $1`, the racing replacement row must use the **same `type_id`** as the original. A different-`type_id` race doesn't exercise the predicate the bug lives in.
- Validate `rows_affected() == 1` on staged setup `UPDATE`s. A fixture drift fails loudly at the staging step rather than as a confusing assertion mismatch.

**Examples**: `crates/services/src/base/world_entry/methods/inventory/move_/concurrency_tests.rs` (PR #150, PR #175), `crates/services/src/base/world_entry/methods/inventory/grant/` concurrency tests (PR #145).

### 6. Chain-replay tests

**Where**: `crates/services/src/cell/content/chain_replay_tests/` (47 tests).

**For**: Content chains in `db/resources/Content/Seed/space_*_chains.sql` — guarding against converter bugs (auto-generated `accept_mission` where `complete_mission` was meant), shadow conditions, missing `interact_tag`/`set_interaction_type` pairings.

**Patterns to follow:**
- Phrase the `expect` so the failure points at the right component: "chain X must exist *and* successfully load" (covers both the seed and the loader). PR #173 review insisted on this.
- When the loader rejects a chain for an unknown trigger/action, the replay test must distinguish "row missing" from "row present but skipped" — those have different fixes.

### 7. C++ legacy + Python script tests

The `src/` (C++) and `python/` (game scripts) trees are reference-only for active development. They have their own (smaller) test surface that is not part of the CI gate. **Don't write new tests there** unless you are specifically validating the reference behavior we're trying to match in Rust; in that case, document why in the test header and link the corresponding Rust test.

### 8. Fan-out byte tests

**Where**: The handler's own module/`tests.rs`, using `crate::test_support::TestTransport` (the recording `cimmeria_mercury::transport::Transport` fake).

**For**: Asserting the exact ordered set of `(addr, bytes)` pairs a BaseApp handler emits — "when handler H runs with state S, exactly these N packets, in this order, with these bytes, hit the transport." This sits between a wire-format test (one serializer's bytes in isolation) and a live-DB guard (SQL state): it pins the **dispatch + routing** layer that decides *which* serializer output goes to *which* addr. Enabled by the `Transport` trait — see [docs/architecture/transport-trait.md](docs/architecture/transport-trait.md).

**The failure modes it catches** (uninspectable before the trait landed):

- **Witness-list amplification** — handler sends to N±1 witnesses (`assert_eq!(transport.len(), N)` / `send_count_to(addr)`).
- **Wrong-recipient routes** — owner-only packet leaks to a witness, or vice versa (`assert_eq!(sent[i].0, expected_addr)`).
- **Stale-addr sends / no-send invariants** — early-return paths must not emit (`assert!(transport.is_empty())`), replacing the old `receiver`-socket `try_recv_from() == WouldBlock` ceremony.

**Patterns to follow:**

- Build a typed `Arc<TestTransport>` for inspection; clone it to an `Arc<dyn Transport>` for the handler call (the inspection methods aren't on the trait). This "two-handle" pattern is in `world_entry/teleport.rs` / `base/character/mod.rs`.
- Assert **count first, then per-recipient addr, then per-recipient bytes** — each catches a distinct bug class; don't stop at count.
- Build the expected bytes from the same serializer the handler uses, then assert byte-for-byte. A length-only assertion is a tautology (same lesson as wire-format tests).
- If the handler reads `local_addr()`, construct with `TestTransport::with_local(addr)` — the default is a synthetic `127.0.0.1:0` placeholder.
- `drain()` consumes the records (a second `drain()` is empty); `clear()` resets without returning; `filter_to(addr)` / `send_count_to(addr)` scope to one recipient.

**Examples**: `crates/services/src/base/world_entry/teleport.rs` (forced-position snap to the player addr, zero witness fan-out), `reanchor_player.rs` (owner-only burst), `crates/services/src/base/login/` (phase 1→4 ordered sequence), `crates/services/src/base/world_entry/cell_dispatch/aoi.rs` (`left_aoi_fans_out_one_packet_per_witness_to_each_addr` — witness fan-out cardinality + per-addr bytes).

### 9. Mercury session tests (loopback harness)

**Where**: `crates/mercury/src/test_harness/tests/{reliable,fragment,keepalive,encryption,handshake,ack,rto}.rs`, using `crate::test_harness::LoopbackSession` (the paired-channel loopback harness). Behind the `test-harness` Cargo feature on `crates/mercury` — see [docs/architecture/mercury-loopback-harness.md](docs/architecture/mercury-loopback-harness.md).

**For**: Mercury **protocol-layer** end-to-end behavior — anything that needs two real `Channel`s exchanging real packets on real loopback sockets. Fan-out byte tests (type 8) cover the outbound side only; this covers the recv loop, channel state evolution, timing-sensitive paths (RTO / keepalive / inactivity), and the encryption pipeline round-trip.

**The failure modes it catches** (uninspectable before the harness landed):

- **End-to-end reliable delivery under loss** — send → drop → RTO → retransmit → ack → tx-window clear. Every leg has a unit test; the **pipeline** had none.
- **Fragment reassembly across paired channels** — wrong fragment numbering on the sender that the in-memory unpacker tests would happily round-trip but a real peer can't reassemble.
- **Keepalive cadence end-to-end** — a real keepalive packet, decoded by the peer, recognised, updates `last_received`.
- **Encryption keystream behavior across multiple bundles** — single-shot encrypt/decrypt unit tests can't catch a regression where the cipher state was inadvertently shared across bundles.
- **Retransmit-uses-cached-bytes invariant** — the retransmit must emit the **same** bytes that went on the wire originally, not a fresh encryption.
- **Ack aggregation under realistic bursts** — N inbound reliable packets coalesce into one outbound packet's ack footer.
- **Adaptive RTO floor engagement on real loopback** — the floor exists for sub-millisecond LAN RTTs; only paired channels on actual sockets exercise that.

**Patterns to follow:**

- **Feature gate.** The harness is behind the `test-harness` Cargo feature on `cimmeria-mercury`. Tests inside `crates/mercury/src/test_harness/tests/` get it for free (the `test` cfg activates the module). A consumer crate wanting to use the harness from its own integration tests must add `cimmeria-mercury = { path = "...", features = ["test-harness"] }` as a dev-dependency.
- Default to `LoopbackSession::connected(None)` — both peers handshaked, no encryption. Reach for `LoopbackSession::connected(Some(enc))` only when the test is about the encryption path.
- Advance time via `peer.clock.advance(Duration::from_millis(...))` rather than `tokio::time::sleep`. The clock is a `Clock`-trait `TestClock` injected into the inner `Channel`; advancing it drives keepalive / RTO / inactivity deterministically with no wall-clock cost.
- Drop / latency / duplicate via `session.policy.lock().unwrap().drop_next.a_to_b = N`. Applied on the sender side — the receiver-side outcome is identical to a wire-side drop, and the sender's TX-window / RTO state stays correct.
- Use `session.quiesce(Duration::from_millis(500)).await` to assert "both peers have empty TX windows and zero pending acks". Avoids brittle timing assertions.
- Use `peer.recv_n_bundles(n, timeout).await` to wait for assembled bundles. Returns fewer than `n` if the timeout fires first — assert the length.
- Per-direction ordering is preserved (A's 1, 2, 3 → B observes 1, 2, 3). **Cross-direction interleaving is implementation-defined** unless serialized via `session.tick().await` — document explicitly when a test depends on a specific interleaving.
- Default `#[tokio::test]` (current-thread). Tests asserting "neither direction stalls the other" need `#[tokio::test(flavor = "multi_thread", worker_threads = 2)]` with a doc-comment naming the reason.
- For the encryption category, use `MercuryEncryption::from_session_key([0x42u8; 32])` for synthetic keys. Known-answer ciphertext tests against the real C++ client need Ghidra-extracted samples (the 22nd test in the inventory; deferred — see the ADR).

**Examples**: `crates/mercury/src/test_harness/tests/reliable.rs` (drop → RTO → retransmit → ack pipeline), `fragment.rs` (3-fragment reassembly under latency policy), `keepalive.rs` (clock-advance past `KEEPALIVE_INTERVAL_MS`), `encryption.rs` (multi-bundle keystream + retransmit-cached-bytes), `rto.rs` (floor engagement on zero-latency loopback).

### 10. Network chaos tests

**Where**: `crates/mercury/src/test_harness/tests/chaos/` for protocol-state scenarios; `crates/services/tests/chaos_*.rs` for `LossyTransport` integration; the pcap-replay infrastructure lives at `crates/mercury/src/test_harness/pcap_replay.rs`. Behind the `test-harness` Cargo feature for the L1 scenarios; behind `test-support` for `LossyTransport`.

**For**: Reproducing a real protocol-state recovery shape — single-packet drop mid-stream, burst loss, asymmetric ack loss, sustained probabilistic loss, lossy-socket integration, or a pcap replay of a captured production session. Pins regressions in the Mercury retransmit / TX-window / RTO / inactivity-timeout paths. The lomiada-class regression (a single transatlantic UDP drop that kills the session 60s later because the TX window can't drain) is the canonical bug shape this catches.

**The three layers** (see [docs/architecture/network-chaos-testing.md](docs/architecture/network-chaos-testing.md) for the full ADR):

- **L1 — Channel-level scenarios** under `tests/chaos/`. Use `LoopbackSession` + `NetworkPolicy.drop_at_send_count` / `drop_probability` / `duplicate_next_count` / `reorder_buffer_size` to construct the failure shape. Assert recovery with `peer.recv_n_bundles` + `invariants::all_safety_invariants`.
- **L2 — `LossyTransport`** wrapping `BidirectionalTransport`. Use `LossyConfig::from_profile(LossyProfile::Transatlantic)` for the canonical "real wire" profile. Integration tests in `crates/services/tests/chaos_*.rs` wrap a real UDP socket and exercise the services-layer recv loop under chaos.
- **L3 — Pcap replay** via `PcapReplay::load(...).with_key_from(...)`. Loads a real pcap (e.g. `debug/lomiada-broke-in-hallway02/`), decrypts via the saved session key, yields ordered events. Tests should skip silently if the fixture isn't present so dev environments without `debug/` still pass.

**Patterns to follow:**

- **Use seeded RNG.** Any probabilistic drop (`DropProbability::pct(N)`) or `LossyTransport` config must explicitly set the seed (`rng_seed: Some(0xC0FFEE)`) so the test is bitwise-reproducible across CI runs and platforms.
- **`reset_counters` between phases.** Tests that combine handshake + chaos primitives should call `policy.reset_counters()` after the harness handshake completes so `send_count`-indexed primitives (`drop_at_send_count`) align to the test's sends only.
- **Respect `RETRANSMIT_BUDGET_PER_TICK = 5`.** A burst that exceeds the budget needs multiple ticks to fully retransmit, and between ticks the clock must advance enough to exceed the (potentially doubled) RTO. The `lomiada_single_packet_gap` test stays inside the budget; `burst_loss_mid_stream` exercises the multi-tick drain explicitly.
- **Assert safety invariants at scenario end.** Call `invariants::all_safety_invariants(&channel)` to catch any silent tx_window overflow / orphan-retransmit blowup the scenario might have left behind.
- **Pcap-replay tests should skip-not-fail when fixtures are missing.** Some dev environments exclude `debug/`; gate the test body on `if !pcap.exists() || !keys.exists() { return; }`.
- **L2 integration tests are slow.** A `Transatlantic` profile run over 500 packets takes ~30s wall time because of the 60ms-per-recv latency. Use a smaller send count for quick smoke; reserve full-volume runs for a slower CI tier if one exists.

**Examples**:

- L1: `crates/mercury/src/test_harness/tests/chaos/lomiada_single_packet_gap.rs` (the canonical recovery shape), `sustained_5pct_loss_60s.rs` (seeded Monte Carlo), `asymmetric_ack_loss.rs` (lomiada-shape silent peer).
- L2: `crates/services/tests/chaos_lossy_transport_integration.rs` (round-trip through `LossyTransport`, transatlantic-profile loss).
- L3: `crates/mercury/src/test_harness/tests/chaos/replay_lomiada.rs` (real pcap fixture).

### 11. Wire-level replay tests (`cimmeria-wireclient`) — **Phase 1 only; replay not yet built**

> **Status check before you rely on this section.** `cimmeria-wireclient` today is a
> *byte-builder and trace-parser library*, not a replay engine. The crate contains no
> `UdpSocket`, no `send_to`, and no `recv_from` — `Client::connect()` does not exist,
> and `Client::from_handshake` is documented in-source as a test-only constructor
> (`crates/wireclient/src/client.rs:64-67`). `Client::build_login_packet` stops at
> "produce the bytes"; the doc-comment states the caller does the UDP send and that
> "Phase 1.5 wires the socket loop" (`crates/wireclient/src/client.rs:56-59`).
> `Trace::c2s()` / `Trace::s2c()` are plain iterator filters with no consumer.
> Everything below marked **(planned)** describes the ADR's target state, not shipped
> behavior. See the [wireclient ADR](docs/architecture/wireclient.md) phase table.

**Where**: `crates/wireclient/` — 30 tests across 5 files: `src/auth.rs` (6), `src/handshake.rs` (10), `src/session_trace.rs` (10), `tests/auth_smoke.rs` (3), `tests/trace_load.rs` (1). Uses [`cimmeria_wireclient::session_trace::Trace`](crates/wireclient/src/session_trace.rs) to load a JSONL trace produced by [`tools/pcap_to_session.py`](tools/pcap_to_session.py) from a decrypted `.pcap` + AES `keys.txt`.

**What works today:**

- **SOAP auth Phase 1+2** driven against an in-process `AuthService` — `tests/auth_smoke.rs` covers the happy-path round trip, the Phase 1 `sid` cookie, and the Phase 2 same-`sid` replay rejection.
- **Handshake byte builders / parsers** — `src/handshake.rs` builds the unencrypted `baseAppLogin` datagram and parses `connect_reply` / `time_sync`, pinned byte-exactly by 10 unit tests.
- **JSONL trace load + diff classification** — `src/session_trace.rs` parses the header + event stream and classifies message pairs via `ComparisonPolicy`.

**What it will catch once Phases 1.5–5 land (planned):**

- **Wire-path drift** — the server emits an `onCharacterList` body shape the Flash client wouldn't parse; existing wire-format tests pin one serializer at a time but don't catch when the *sequence* would desync the client.
- **Client-invariant violations (planned — Phase 5)** — the ADR calls for wireclient to enforce equipped-weapon, ammo, range, LOS, and cooldown on its own outbound `useAbility` sends, so the server can't accept a fire no real client could have made. **None of that enforcement is written yet** — there is no stats mirror, entity mirror, or ability gate in the crate.
- **Behaviour diffs across a full playthrough (planned — Phase 3)** — requires the semantic behavior-trace decoder, which does not exist.

**Patterns to follow:**

- **Know what `DefaultPolicy` actually compares.** Byte-exact for static msg_ids `0x00`–`0x7F` *and* `0xFF` (`BASEMSG_REPLY_MESSAGE`, deliberately excluded from the drift band). For entity-method msg_ids `0x80`–`0xFE` it compares **body length only** (`crates/wireclient/src/session_trace.rs:292-328`): equal length is reported as `Diff::Drift`, unequal as `Diff::Regression`. A trace diff therefore cannot validate gameplay *content* today. Swap in a custom `ComparisonPolicy` impl if you need more.
- **The corpus is one 5-event head fixture.** The only trace checked into the repo is `crates/wireclient/tests/fixtures/castle_cellblock_head.jsonl` (6 lines: 1 header + 5 events). The `castle-cellblock-full-run` corpus named in the ADR is **not in the repo** — it is a recording that has to be produced locally. Don't write a test that assumes it exists.
- **Reuse the dissector.** `tools/pcap_to_session.py` rides on `tools/pcap_dissect.py` so the decoder stays a single source of truth.
- **The auth + handshake smoke is byte-exact.** SOAP Phase 1+2 and Mercury phase-3 are deterministic and pinned in `crates/wireclient/tests/auth_smoke.rs` plus the handshake unit tests in `handshake.rs`.

**Examples**: `crates/wireclient/tests/auth_smoke.rs` (SOAP round-trip against in-process `AuthService`); `crates/wireclient/tests/trace_load.rs` (loads the head fixture); `crates/wireclient/src/handshake.rs` (byte-exact `baseAppLogin` + reply parsers). Follow-up phases land the UDP socket loop and then the Castle Cellblock e2e smoke against a spawned server.

### 12. Negative-log regression guards

**Where**: Co-located with the production code (same `mod tests` or a sibling `tests.rs`). Uses `crate::test_support::LogCapture` to record events and `find_event(level, message_substr, reason_value)` / `find_message(level, message_substr)` to assert.

**For**: Any change that promotes a silent error swallow (`let _ = tx.send(...)`, `let _ = query.execute(...)`, `trace!`-level miss) to a queryable structured log per [docs/architecture/negative-logging-convention.md](docs/architecture/negative-logging-convention.md). Without a guard, a future revert to `let _` or `trace!` is invisible.

**The failure modes it catches:**

- **Silent revert of a level promotion** — a `warn!` that quietly becomes `trace!` again (pinning the level traps it).
- **Silent removal of a structured field** — `reason="entity_to_addr_miss"` dropped during a refactor (pinning the `reason` value via `find_event` traps it).
- **Silent revert of the `if let Err` wrap** — going back to `let _ = tx.send(...)` (the test fails because the event no longer fires).

**Patterns to follow:**

- **Install before the action.** `let capture = LogCapture::install();` MUST happen before the function under test runs, so the per-thread subscriber is in place when the log fires.
- **Pin level AND field.** Use `find_event(Level::WARN, "<message_substr>", "<reason_value>")` to trip on both a level-only revert and a field-removing revert. For paths where the new log doesn't carry a `reason` field, `find_message(Level::WARN, "<substr>")` is the fallback.
- **Force the failure path.** For `mpsc::Sender::send().await`, drop the receiver before invoking the function (`let (tx, rx) = mpsc::channel(8); drop(rx);`). For `transport.send_to`, supply an inline `FailingTransport` impl (see `crates/services/src/base/world_entry/cell_dispatch/tests.rs` and `enable_entities.rs::tests`). For `rows_affected == 0`, use a live-DB test that binds a non-existent id.
- **Default `#[tokio::test]` (current_thread).** `LogCapture::install()` PANICS if called inside a `multi_thread` runtime — `set_default` is thread-local, and events on worker threads would be silently dropped. The panic message names the fix.
- **Treat `reason` values as stable API.** `find_event` matches `reason` by exact string equality. Renaming a value (even a typo fix) trips every guard pinned to the old string — coordinate via the convention doc.
- **One test per seam.** Don't multiplex unrelated negative paths in one test — when one assertion fails, you want to know which seam broke.

**Examples**: `crates/services/src/base/helpers/` (3× witness-miss WARN + 3× client-disconnect DEBUG), `crates/services/src/base/world_entry/map_loaded.rs::tests::map_loaded_fragment_send_failure_errors_and_logs` (FailAfter transport + state-not-mutated invariant), `crates/services/src/base/world_entry_appearance/` (`on_client_ready_errors_each_cell_tx_send_independently_when_closed` — 3 ERROR sites in one test with closed receiver).

---

## Choosing a test type

| If you are testing… | Use… |
|---|---|
| A pure function or a state machine | Unit test |
| A byte producer/consumer (Mercury, vendor serializer, AoI builder) | Wire-format test |
| A `WHERE` clause, `rows_affected`, advisory lock, `ON CONFLICT` | Live-DB regression guard |
| An invariant that spans two or more handlers | PL/pgSQL smoke + Rust harness |
| A race condition or `join!`-of-futures correctness | Concurrency regression guard |
| Content seed correctness (chains, triggers, action wiring) | Chain-replay test |
| A BaseApp handler's outbound fan-out (which addrs, in what order, with which bytes) | Fan-out byte test |
| Mercury protocol-layer behavior (reliable delivery under loss, fragment reassembly, keepalive cadence, encryption round-trip, RTO convergence) | Mercury session test |
| A protocol-state recovery shape (single-packet drop in a long stream, burst loss, asymmetric ack loss, sustained probabilistic loss, lossy-socket integration), or a pcap-replay regression against a captured production session | Network chaos test |
| An end-to-end client-intent + server-behaviour pair (full Castle Cellblock playthrough, wire-path drift, client-invariant violations on `useAbility`) | Wire-level replay test (wireclient) — **not available yet**; the crate ships auth + handshake byte builders and a trace parser only. Pick another type until Phase 1.5 lands the socket loop. |
| Promoting a silent error swallow (`let _ = tx.send(...)`, `let _ = query.execute(...)`, `trace!`-level miss) to a queryable structured log | Negative-log regression guard |

### When one feature needs more than one test

The **vendor sell/purchase** stack is the canonical example, and it carries every type:

- **Unit test**: `normalize_item_quantities` and `normalize_item_ids` (pure transforms — PR #137).
- **Wire-format test**: `vendor::serializers` byte-exact tests — PR #136.
- **Live-DB regression guard**: per-handler tests in `vendor/{sell,buyback,purchase,paid_repair,paid_recharge,recharge,repair,data}/tests.rs` — PRs #154–#161, #156–#157.
- **Smoke**: `vendor_store_smoke.sql` round-trips the entire stack — PR #172.
- **Concurrency guard**: separate from the per-handler test, in `concurrency_tests.rs` — PR #150, PR #175.

Each layer catches a different class of bug. **Skipping a layer because "the next layer up will catch it" is the bug shape this list exists to prevent.** PR #172's review explicitly argued the smoke test catches whole-stack regressions the per-handler tests can't.

---

## Common gotchas (from PRs #131–#175)

This section is mined from review comments since the test push began. Each item is a real reviewer correction; the parenthetical citation is where it came from.

### Tightness

- **Scope DB lookups by composite key**, not by `player_id` alone. Use `fetch_one` on `(player_id, mission_id)` instead of `fetch_optional` on `player_id` (PR #146).
- **Assert "exactly one row"** with `SELECT COUNT(*)` or by fetching all rows. `fetch_optional` without `ORDER BY` or count check passes when the single-insert invariant is violated (PR #145).
- **Assert exact final positions** after a deterministic swap (`item A at (1,5)`, `item B at (1,0)`), not "two distinct non-sentinel positions" — the loose form passes when both moves rolled back (PR #150).
- **Use `== 1` not `>= 1`** for counts you control. Reviewers will flag the loose form (PR #111).
- **Validate `rows_affected() == 1`** on staged UPDATEs in setup so fixture drift fails at the UPDATE, not later (PR #175).
- **Assert the seq_id footer offset** in wire-format tests, not a slice length that's trivially satisfied (PR #142).

### Naming

- **Test names must match the assertion.** A test named `bails_silently_*` that asserts a `tracing::warn!` was logged is lying — rename it (PRs #139, #143).
- **A test that returns `Some(VendorSession { server_template_id: None, .. })`** must not be named as if it returned `None` (PR #141).
- **A test pinning JSON-tag stability** must serialize and inspect the JSON `kind` field. Asserting `event_type()` defeats the persistence contract the name promises (PR #133).

### Don't trust seed data

- **Re-fetch baseline values inside the test.** Hard-coded constants like `WEAPON_TYPE_ID = 3241` or specific clip-size enums break when seeds change (PRs #160, #158, #162).
- **Mirror the handler's predicate** when the picker query selects a fixture row. If the handler filters on a sellable-flag bitmask, the picker must too — otherwise the test passes for the wrong reason (PR #154).
- **Assert by relationship**, not by id: `slot.cur_ammo_type == slot.default_ammo_type` survives seed churn; `slot.cur_ammo_type == 3241` does not.
- **Never use a "definitely nonexistent" id like `99_999_999`.** The sequence may have advanced past it. Use a sentinel from your module's reserved `0x7000_xxxx` slot (see "Sentinel id discipline") or `MAX(id) + 1` computed at runtime (PR #144).
- **Assert fixture cardinality up front.** If `pick_main_bag_type_ids(2)` can return one row, assert `types.len() == 2` so the failure points at missing fixture data, not a cryptic out-of-bounds panic (PR #144).
- **Validate the discriminator against actual seed values, not the schema spec.** `clip_size IS NOT NULL` looks like a clean "is this a weapon?" predicate, but the SGW seed uses `clip_size = 0` for non-weapons rather than NULL — so the predicate misclassifies every consumable as a weapon. When a handler infers item type (or any boolean trait) from a column, write a live-DB test that exercises both sides of the partition with **real seed rows** (a slappack at `clip_size = 0` AND a pistol at `clip_size > 0`) so the test catches the value-vs-sentinel mismatch. The robust fix is usually to read the authoritative discriminator field directly — for "is this auto-equippable to the bandolier?", `3 = ANY(container_sets)` beats any inference from a sibling column.

### Sentinel id discipline

- **Reserve a positive `0x7000_xxxx` base per module** and partition the low byte (or low two bytes) for individual tests. Existing reservations include `0x7000_0100` (grant_cash), `0x7000_0200` (move_inventory), `0x7000_0300` (grant_item), `0x7000_0400` (missions), `0x7000_0600` (vendor repair), `0x7000_0800` (vendor sell), `0x7000_0B00` (inventory ammo), `0x7000_1000` (character-list), `0x7000_1200` (purchase_helpers), `0x7000_1300` (vendor recharge). Document the neighbours in a module-doc comment so the next contributor can step past them (`crates/services/src/base/character/mod.rs:288-296` is the canonical shape).
- **Sentinels for `INTEGER` columns must fit in `i32` range.** `0x7000_xxxx` does. `0xDEAD_0000` as `u32` wraps to negative when bound `as i32` and lands in another module's territory — don't reach for high-bit constants (PRs #134, #150).
- **Cleanup must delete by exact id**, not by range. `DELETE WHERE entity_id = $sentinel` (or `IN (...)` over the exact ids the test inserted) beats `DELETE WHERE entity_id BETWEEN base AND base+0xFF` — range deletes can reach into a sibling module's slot if partitioning ever drifts (PRs #154, #163).
- **Don't share-row `DELETE` in cleanup.** For rows you `INSERT INTO resources.items` to set up the fixture, use `ON CONFLICT DO NOTHING` and let the row leak — otherwise test B's cleanup yanks a row out from under test A (PR #164).

### Concurrency

- **Two `join!`ed handler futures often serialize by accident.** Use a barrier-coordinated start or a small loop so a no-lock regression actually fails (PR #145).
- **Wrap each `JoinHandle::await` in `tokio::time::timeout(...)`.** A deadlock regression should fail the test, not wedge the suite (PR #150).
- **Capture and assert each spawned task's `Result`.** Dropping it lets an `Err` that left the DB in a satisfying state become a false positive (PR #150).
- **Run live-DB tests serialised.** Sentinel ranges are shared and parallel runs collide. The `ci-live-db` nextest profile pins `threads-required = "num-test-threads"`; with `cargo test`, pass `-- --test-threads=1`. CI enforces this in the `test-live-db` job.

### Regression-guard shape

- **The bug shape must reproduce if the fix is reverted.** A `update_bandolier_ammo` TOCTOU guard needs a **same-`type_id`** racing replacement row — different `type_id` doesn't exercise the predicate (PR #158).
- **Negative-path tests must assert what's actually missing.** "No wire packet was sent" is unprovable when the test set up an empty `connected` map; either set up a real receiver or narrow the doc-comment to the DB invariant you actually checked (PR #143).
- **A loop that filters on `method == 16`** doesn't prove "no combat-side messages emitted." Either tighten the doc-comment to "no `onTargetUpdate`" or extend the loop to fail on `onTimerUpdate`/`onSequence`/`onStateFieldUpdate` (PR #138).

### Failure messages

- **Word the assertion message to match the failure mode you're protecting against.** "world::dispatch handled it instead" misleads when the regression actually returns `false` from `world::dispatch` (PR #140).
- **Distinguish "row missing" from "row present but skipped".** "chain 3026 must exist in seeded content_chains" hides the case where the loader rejected the row for an unknown trigger; phrase as "must exist *and* successfully load/convert" (PR #173).

### Test-DB hygiene

- **Live-DB tests run against `sgw` loaded from `db/database.sql` in CI**, and against a developer-supplied `DATABASE_URL` locally. The bundled local Postgres binds to **port 5433** (not 5432) — see [docs/architecture/integration-test-infra.md](docs/architecture/integration-test-infra.md) for setup.
- **The skip message must distinguish unset vs unreachable.** `test_pool()` returns a `SkipReason` enum so the developer sees "DATABASE_URL not set" vs "DATABASE_URL set but connect failed: …" (PR #134, see [crates/services/src/test_support.rs](crates/services/src/test_support.rs)).

### File and module hygiene

- **When a test module pushes the host file past 700 lines**, extract concurrency helpers and multi-threaded tests into a sibling `concurrency_tests.rs` or `tests/` submodule. **Reuse the existing `make_state()` / `make_ctx()` helper** — don't clone setup (PRs #143, #150).
- **De-duplicate setup** across routing tests. PR #140 review required a `make_ctx()` helper so each test focuses on its routing assertion.

### Comment hygiene

- **No PR or issue numbers in source comments.** Describe the invariant directly. Numbers rot, the rationale should be self-contained (PRs #139, #150, #164, #172, #173). Provenance lives in the PR body.
- **No line-numbered citations of the system under test.** Reference the function name only (PR #172).

### CI gotchas

- **Run `cargo test --workspace`**, not just `-p cimmeria-services`. A workspace test job that only runs one crate lets failures in `cimmeria-mercury` and `cimmeria-content-engine` merge green (PR #151).
- **Pass `--locked` in CI** so a stale `Cargo.lock` fails the job rather than silently re-resolving (PR #151).
- **Install `clang` and `mold` explicitly in CI.** A `.cargo/config.toml` that selects them as the linker means the runner image's preinstalls aren't enough — the suite must be self-contained (PR #151).

---

## Running the test suite

### Locally (no DB — covers ~2,520 tests)

```bash
cargo nextest run --profile=ci --workspace \
  --exclude cimmeria-app --exclude cimmeria-content-editor \
  --exclude cimmeria-scene-editor --exclude sgw-launcher \
  --exclude cimmeria-client-telemetry
# nextest can't run doctests; cimmeria-commands is the only crate
# with runnable ones today.
cargo test --doc -p cimmeria-commands
```

The exclude list must match `WORKSPACE_EXCLUDES` in [.github/workflows/test.yml](.github/workflows/test.yml) — it selects 2,691 of the workspace's 2,936 tests, of which the 224 live-DB guards self-skip without `DATABASE_URL`.

`cargo test --workspace ...` still works for quick sanity checks if you don't have nextest installed, but CI uses nextest and that's what the JUnit upload to Codecov Test Analytics expects.

### Locally (live DB — adds the 247 `require_db_or_skip!` guards + 3 smokes)

Start the bundled Postgres on port 5433 (via `setup.ps1`'s bootstrap), then:

```bash
DATABASE_URL=postgres://w-testing:w-testing@localhost:5433/sgw \
  cargo nextest run --profile=ci-live-db -p cimmeria-services --lib
```

The `ci-live-db` profile in `.config/nextest.toml` serialises every test (`threads-required = "num-test-threads"`) — equivalent to the old `cargo test ... -- --test-threads=1`. Without `DATABASE_URL`, those 247 tests self-skip with `module_path!: skipping live-DB test (DATABASE_URL not set)`. **Self-skipped tests are not failures** — but a green "no DB" run does not prove the live-DB suite passes. Always run both before declaring a PR ready.

### CI (every PR)

`.github/workflows/test.yml` defines six jobs. Five gate merge: `fmt`, `clippy`, `build`, `test` (workspace, no DB, nextest), `test-live-db` (postgres:17.9 service container, nextest). The sixth, `coverage` (`cargo llvm-cov` over a no-DB workspace pass plus a services live-DB pass), is `continue-on-error: true` — its artifact and summary are advisory, so a coverage-tool flake never blocks a merge. Nextest's JUnit XML output from the `test` and `test-live-db` jobs is uploaded to Codecov Test Analytics, which surfaces per-test history, flaky-test detection, and PR comments naming the failed tests.

---

## Adding a new test — checklist

Before opening a PR that adds tests:

- [ ] Test type matches the bug shape (see [Choosing a test type](#choosing-a-test-type)).
- [ ] Test name matches the assertion.
- [ ] Assertions are tight (`==` not `>=`, composite keys not single-column filters, exact positions not "distinct").
- [ ] No hard-coded resource ids; baseline values re-fetched at runtime.
- [ ] Sentinels are positive `0x7000_xxxx` ids in your module's reserved slot; cleanup deletes by exact id, not by range.
- [ ] Live-DB tests use `require_db_or_skip!`.
- [ ] Concurrency tests use `tokio::time::timeout` and capture `Result`s.
- [ ] No PR/issue numbers in source comments.
- [ ] Setup helper reused, not cloned.
- [ ] Reverting the fix makes the test fail (regression-guard test).
- [ ] Test runs locally serialised against a live DB if applicable (`cargo nextest run --profile=ci-live-db ...`, or `cargo test ... -- --test-threads=1`).

---

## See also

- [docs/architecture/integration-test-infra.md](docs/architecture/integration-test-infra.md) — the "no testcontainers, no `sqlx::test`" decision and local setup.
- [.github/workflows/test.yml](.github/workflows/test.yml) — the canonical CI definition.
- [crates/services/src/test_support.rs](crates/services/src/test_support.rs) — `require_db_or_skip!` and `test_pool`.
- [crates/services/src/base/smoke_tests.rs](crates/services/src/base/smoke_tests.rs) — the three end-to-end smokes and their rationale.
- [tools/vendor_store_smoke.sql](tools/vendor_store_smoke.sql), [tools/inventory_move_smoke.sql](tools/inventory_move_smoke.sql), [tools/progression_smoke.sql](tools/progression_smoke.sql) — the smoke scripts themselves.
- [.github/copilot-instructions.md](.github/copilot-instructions.md) — review checklist; the testing checklist in this file feeds into it.
