---
title: Test Suite Review Report
type: explanation (Diátaxis)
audience: engineers, reviewers
last-updated: 2026-05-04
companion-docs: TESTING.md, docs/testing/inventory/README.md
---

# Test Suite Review Report

> **One-line verdict**: **green-leaning yellow** — the suite is in good shape (every heuristic-flagged test turns out to be either a deliberate panic-on-failure shape or a proptest with `prop_assert_eq!`), but a handful of small-surface tests assert too little to be regression guards in any meaningful sense and a few crates (`game`, `entity` outside of math/grid) have noticeably less per-feature depth than `services` and `mercury`.

> **Scope**: 1071 tests across 14 crates as of 2026-05-04. Sources: `docs/testing/.scratch/inventory.json`, `docs/testing/.scratch/summary.json`. The author read each of the 25 heuristic-flagged tests in full and sampled 2-3 tests from each of the seven crates listed in the task brief; the remaining ~1020 tests were not individually reviewed.

## How to read this

- **Flagged**: a test whose assertions don't pin the bug shape its name implies, or that asserts something so weak the test would survive a meaningful regression. Each flag cites file:LINE and a concrete bug-shape reason. Recommendations are *types* of fix (tighten / split / delete / promote), not implementations.
- **Strong example**: a test the author would point a contributor at as a model of the type. Calibration matters as much as criticism.
- **Gap**: a *class of bug* the suite doesn't pin in a given crate, named concretely (not "more tests please").
- A `green` crate verdict means the spot-check found tight, name-honest tests with no obvious bug-shape coverage holes in the area sampled. `yellow` means the area is broadly fine but had at least one tautology or under-asserted test. `red` would mean "stop and audit"; nothing the author sampled hit that bar.

## Heuristic-flagged tests (deep dive)

Of the **25 tests** flagged with `no_assert_or_question_mark`, **24 are heuristic false positives** (the assertion shape exists but the regex didn't catch it: `expect(...)`, `prop_assert_eq!` inside a `proptest!{}` block, `#[should_panic(...)]`, or a helper that itself panics). **1 needs human attention**. Grouped by crate.

### services (12 flagged)

- [`crates/services/src/orchestrator.rs:294`] `new_orchestrator_creates_stopped_services` — **needs-attention**. Body is `let _ = orch.state();` after construction; no `.await` and the comment says "verify it constructs without panicking" — but `Orchestrator::new` is infallible (no unwrap / no `?`) and the discarded `state()` call is a no-op at this level. Suggested fix: **delete or tighten** — either drop it (the type system already proves construction works) or tighten to assert the per-service initial state matches `ServiceState::Stopped`.
- [`crates/services/src/base/smoke_tests.rs:60`] `vendor_store_smoke_passes_against_seed_data` — **legitimate**. Calls `.expect("vendor_store_smoke.sql must run without raising an exception")` — assertion-on-failure is the design (PL/pgSQL `RAISE EXCEPTION` surfaces as `Err`).
- [`crates/services/src/base/smoke_tests.rs:90`] `inventory_move_smoke_passes_against_seed_data` — **legitimate**. Same shape (`.expect` on `sqlx::raw_sql`).
- [`crates/services/src/base/smoke_tests.rs:116`] `progression_smoke_passes_against_seed_data` — **legitimate**. Same shape.
- [`crates/services/src/cell/combat/damage.rs:455`] `extreme_qr_does_not_panic` — **legitimate**. The doc-comment names the invariant ("neither beta parameter can go non-positive for any finite QR"); the test pins it via panic-on-failure (any out-of-range parameter would panic inside `calculate_result`). Heuristic false positive — the test legitimately exercises a no-panic contract.
- [`crates/services/src/cell/content/mod.rs:158`] `fire_exit_region_uses_tag_as_key` — **needs-attention-adjacent / weak**. Body sets up a region exit and the comment says `// No panic = success`. The function under test has interior unwraps that would surface, but unlike the sibling `fire_exit_region`-style test at line 145 (which asserts `rx.try_recv().is_err()`), this one drops `_rx` and asserts nothing. Suggested fix: **tighten** — bind `mut rx` and assert `rx.try_recv().is_err()` like the sibling does. (Counted as legitimate above because `mgr.create_entity(...).unwrap()` does panic-on-failure during setup, but the test's *named* invariant — exit-key construction — is unobserved.)
- [`crates/services/src/cell/dispatch/tests.rs:169`] `dispatch_trigger_region_exit` — **legitimate (weak)**. Calls `.unwrap()` on entity creation; the test body itself has `// No panic = success`. Same shape as the sibling above. Tighten by asserting `rx` produced no messages.
- [`crates/services/src/cell/dispatch/tests.rs:223`] `dispatch_trigger_region_ignores_short_args` — **legitimate (weak)**. Asserts "silently skip"; only enforced via setup unwraps. Suggested fix: **tighten** by binding `mut rx` and asserting `rx.try_recv().is_err()`. The sibling at line 203 (`dispatch_trigger_region_unknown_id_warns`) does this correctly — copy that shape.
- [`crates/services/src/cell/space_manager/aoi_differential_test.rs:85`] `witness_set_equals_diff_accumulated_set_after_every_tick` — **legitimate**. The body uses `prop_assert_eq!` inside a `proptest!` block, which the regex misses. This is one of the strongest tests in the codebase (see Strong examples below).
- [`crates/services/src/cell/cell_methods/player/vendor.rs:418`] `happy_path_returns_session_with_template_id` — **legitimate**. Calls `assert_session(...)`, a helper that internally `.expect("vendor_context should return Some")` and runs three `assert_eq!`s on the fields. The regex didn't follow into the helper.
- [`crates/services/src/cell/cell_methods/player/vendor.rs:436`] `happy_path_returns_session_when_vendor_lacks_template_id` — **legitimate**. Same `assert_session` helper.
- [`crates/services/src/cell/cell_methods/player/vendor.rs:457`] `returns_session_with_no_template_when_vendor_entity_id_is_stale` — **legitimate**. Same helper.

### mercury (4 flagged)

- [`crates/mercury/src/packet/parse_proptest.rs:39`] `parse_incoming_never_panics_on_arbitrary_bytes` — **legitimate**. Documented "no-panic" contract test; the assertion is structural ("the call returns rather than panicking"), enforced by proptest's panic catcher. Genuinely needs no `assert!` — a panic in the body fails the property and proptest shrinks the input.
- [`crates/mercury/src/packet/parse_proptest.rs:59`] `parse_incoming_handles_truncated_footer_input_without_panic` — **legitimate**. Same shape.
- [`crates/mercury/src/packet/proptest_round_trip.rs:71`] `build_outgoing_round_trips_through_parse_incoming` — **legitimate**. Body uses `prop_assert_eq!` heavily; the regex's body extraction missed the `proptest!{}` macro contents.
- [`crates/mercury/src/packet/proptest_round_trip.rs:118`] `fragmented_packet_round_trips_through_parse_incoming` — **legitimate**. Same.

### entity (5 flagged)

- [`crates/entity/src/movement.rs:241`] `waypoint_controller_empty_path_panics` — **legitimate**. `#[should_panic(expected = "waypoints must not be empty")]` — the panic IS the assertion, byte-string-pinned to the expected message.
- [`crates/entity/src/navigation.rs:572`] `load_and_raycast_castle_cellblock` — **needs-attention**. Body falls through and prints `tracing::info!("Raycast result: {has_los}")` — there's no assertion at all on raycast correctness, and the comment explicitly says "We can't assert this is true without knowing the mesh geometry, but at least verify it doesn't crash". This test passes on a missing data file (silent return on `if !path.exists()`), passes when the loader works, and passes when the raycast returns either bool. Suggested fix: **delete or tighten**. If the only contract is "doesn't crash", say so in the name (`navmesh_raycast_does_not_panic`) and assert at least that the loader's result is `Ok`. Better: pick a known-LOS pair and a known-blocked pair from the mesh and pin both directions. As-written, it's the canonical "tests asserting code-under-test executed but not what it produced" anti-pattern from the persona file.
- [`crates/entity/src/properties.rs:156`] `index_out_of_range_panics` — **legitimate**. `#[should_panic(expected = "property index 256 exceeds maximum")]` — message is byte-pinned.
- [`crates/entity/src/space.rs:162`] `remove_absent_entity_is_safe` — **legitimate (weak)**. Documents "should not panic" via comment; relies on `make_space()` and the call itself not panicking. Could tighten by also asserting `space.entity_count() == 0` post-call so a regression that decremented the counter on absent removal would surface, but the no-panic contract is genuine.
- [`crates/entity/src/world_grid.rs:245`] `zero_cell_size_panics` — **legitimate**. `#[should_panic(expected = "cell_size must be positive")]`.

### defs (1 flagged)

- [`crates/defs/src/registry.rs:103`] `registry_constructs_without_panic` — **legitimate (very weak)**. The doc-comment is honest ("don't assert on the count itself — just that `new()` doesn't trip an internal invariant"). But `EntityRegistry::new()` is `pub fn new() -> Self` with no observable side effects beyond an empty `HashMap`-of-defs — there is no internal invariant to trip. Suggested fix: **delete**. The next test (`registry_default_is_same_as_new`) already exercises construction *and* asserts a relationship.

### content-engine (3 flagged)

- [`crates/content-engine/src/actions.rs:285`] `action_serialization_roundtrip` — **legitimate (weak)**. Body deserializes and then `let _ = format!("{:?}", deserialized)` — that proves `Debug` doesn't panic but doesn't assert the round-trip is faithful. The conditions sibling at line 266 (`faction_relation_serialization_roundtrip`) asserts `*rel == deserialized` — copy that shape. Suggested fix: **tighten** to `assert_eq!` (requires `Action: PartialEq`) or to `match deserialized { Action::GrantXP { amount } => assert_eq!(amount, 500), _ => panic!(...) }` (the test below it at line 308 already does this).
- [`crates/content-engine/src/actions.rs:293`] `property_op_serialization_roundtrip` — **legitimate (weak)**. Same `let _ = format!(...)` shape; no equality check. Suggested fix: **tighten** to `assert_eq!(*op, deserialized)`.
- [`crates/content-engine/src/conditions.rs:280`] `condition_serialization_roundtrip` — **legitimate (weak)**. Same shape. Suggested fix: **tighten**.

### Heuristic-flagged summary

| Category | Count |
|---|---|
| Legitimate — `expect`/`unwrap` assertion shape | 4 |
| Legitimate — `proptest!` `prop_assert_eq!` (regex miss) | 5 |
| Legitimate — `#[should_panic]` | 3 |
| Legitimate — helper-internal asserts (`assert_session`) | 3 |
| Legitimate — designed no-panic contract (smokes, fuzz proptests) | 3 |
| Legitimate but weak — would benefit from tightening | 5 |
| Needs attention | 2 |

Both **needs-attention** items (`new_orchestrator_creates_stopped_services`, `load_and_raycast_castle_cellblock`) are the canonical "code-under-test executed, but not asserted on what it produced" shape from the persona file and TESTING.md.

---

## Crate spot-checks

### services (550 tests)
**Verdict**: **green**
**Sampled**: 3 tests (one each from `base/world_entry/`, `cell/combat/`, `mercury/protocol/`), plus the 12 services tests in the heuristic deep-dive, plus incidental reading of `cell/cell_methods/player/vendor.rs` neighbourhood, `cell/content/mod.rs` neighbourhood, and `cell/dispatch/tests.rs` neighbourhood while resolving heuristic flags.

#### Flagged
- [`crates/services/src/orchestrator.rs:294`] `new_orchestrator_creates_stopped_services` — covered above; canonical "construction does not panic" non-assertion. Suggested: **delete or tighten**.
- [`crates/services/src/cell/dispatch/tests.rs:223`] `dispatch_trigger_region_ignores_short_args` — comment says "silently skip" but no `rx` is bound to verify. Suggested: **tighten** to assert `rx.try_recv().is_err()` like `dispatch_trigger_region_unknown_id_warns` at line 203.
- [`crates/services/src/cell/content/mod.rs:158`] `fire_exit_region_uses_tag_as_key` — body drops `_rx` and the named invariant (key construction by tag) is not observed. Suggested: **tighten** to assert no message arrived AND that the relevant chain registry was queried with the tag-form key (if observable).

#### Strong examples
- [`crates/services/src/cell/space_manager/aoi_differential_test.rs:85`] `witness_set_equals_diff_accumulated_set_after_every_tick` — proptest-driven differential check (`after_witnesses == apply_diff(before_witnesses, events)`). Pins a deep invariant (no dropped `LeftAoI`, no double-fired `EnteredAoI`) over 16 cases × 12 entities × 25 ticks, with a shrinker-friendly trajectory representation and an assertion message that names the *specific tick + player + missing/extra entities* on failure. The doc-comment also justifies the `±80` step bound based on AoI radius math, which is exactly the kind of "explain the magic number" shape reviewers want.
- [`crates/services/src/base/world_entry/methods/player_load/core.rs:351`] `equipped_item_at_inactive_bandolier_slot_is_excluded_from_visuals` — live-DB regression guard that re-fetches the expected `visual_component` from the seed (relationship assertion, not hard-coded id), seeds two slots (active and inactive), counts occurrences in the merged components list and asserts `== 1`. The comment names *both* failure modes (0 occurrences = OR branch dropped, 2 occurrences = OR widened) — that's the bug shape, not the happy path.
- [`crates/services/src/cell/combat/damage.rs:428-488`] `mean_drops_at_positive_qr` / `mean_climbs_at_negative_qr` / `crit_band_remains_reachable_at_zero_qr` / `negative_qr_increases_crit_band_observance` — each pins a *different* statistical invariant of the QR distribution, with the python branch identity in the comment so reviewers can see the formula being preserved. Tight tolerances (`< 0.03` from analytic mean over 10k samples).
- [`crates/services/src/mercury/protocol/tests.rs:113-127`] `ongoing_tick_sync_changes_with_*` — distinguishes "different tick" vs "different seq" causes of ciphertext divergence as separate tests, so a regression that broke one input axis surfaces under its own name.

#### Gaps
- **No concurrency tests in `base/world_entry/methods/inventory/ammo.rs`, `repair.rs`, `recharge.rs`** — only `inventory/grant.rs` (1) and `inventory/move_/concurrency_tests.rs` (3) cover races. The vendor sell/buyback path has TOCTOU-shaped UPDATEs and no `concurrency_tests.rs` sibling. (TESTING.md §5 names exactly this shape — sell/buyback agreement on `flags` is smoke-tested but not raced.)
- **`cell/combat/state.rs` has unit tests but no live-DB integration** — combat outcomes that persist (e.g. death credit, threat decay across re-login) would benefit from a regression guard pinning the SQL.
- **No chain-replay test for `cell/dispatch/tests.rs` short-args path** — the "silent skip on truncated args" branch has no test that proves the skip happens vs. the dispatcher landing on the wrong handler.

### mercury (97 tests)
**Verdict**: **green**
**Sampled**: 3 tests (`channel/tests.rs:306`, `codec.rs:164`, `encryption.rs:260`), plus the 4 proptest tests from the deep-dive, plus the proptest infrastructure read for context.

#### Flagged
- *None.* Every sampled test pins a tight invariant.

#### Strong examples
- [`crates/mercury/src/packet/proptest_round_trip.rs:71`] `build_outgoing_round_trips_through_parse_incoming` — independently generates flag bits and footer values, derives footer-presence flag bits from `Option::is_some()`, asserts every field round-trips with named messages ("flags byte round-trip", "body round-trip", "ack list round-trip in order"). The doc-comment explicitly calls out the bug shape ("a regression that conflates two of the footer slots").
- [`crates/mercury/src/packet/proptest_round_trip.rs:118`] `fragmented_packet_round_trips_through_parse_incoming` — the comment at lines 137-142 explains *why* the test pins the full flags byte and not just `FLAG_FRAGMENTED` ("a regression that dropped one of RELIABLE / ON_CHANNEL / INDEXED / PIGGYBACK passthrough bits would have passed the previous assertion silently"). That's the persona's "tighten assertions" rule made explicit.
- [`crates/mercury/src/encryption.rs:260`] `round_trip_block_aligned` — pins the *exact* ciphertext length (`32 + HMAC_TAG_LEN`) for 16-byte plaintext, with the comment explaining PKCS7's full-block-of-padding rule. Reviewer can predict from the name that the boundary-aligned case is the focus.
- [`crates/mercury/src/codec.rs:164`] `empty_body_round_trip` — handles the empty-body edge case as a separate test from the populated-body one. Persona §"split if a name predicts an assertion".

#### Gaps
- **Wire-format symmetry coverage is asymmetric**: many `build_*` functions are pinned to byte-exact outputs, but `parse_incoming` only has the no-panic proptest and a round-trip property. Hand-written byte-string inputs that exercise pathological-but-legal frames (max-`first_req_offset`, max-ack-count, sequence near `NULL_SEQUENCE`) would catch regressions the round-trip can't (the round-trip can't catch a parser that accepts something `build_outgoing` would never produce).
- **No async-cancellation tests** for the channel/codec stack. `tokio::test`-flavoured tests exist but none drop a future mid-decode.

### entity (151 tests)
**Verdict**: **yellow**
**Sampled**: 3 tests (`stats/tests.rs:94`, `base_entity.rs:202`, `space.rs:141`), plus the 5 entity tests in the heuristic deep-dive.

#### Flagged
- [`crates/entity/src/navigation.rs:572`] `load_and_raycast_castle_cellblock` — silently passes when the data file is missing (`if !path.exists() { return; }` then no `eprintln!` skip notice unlike `require_db_or_skip!`'s explicit message), prints the result via `tracing::info!`, and asserts nothing about the raycast outcome. The sibling `load_and_height_query` at line 590 has the same shape. Suggested: **delete or split** — the file-loading half of this function already has a covering test (`navmesh_loads_*`); the raycast half should pin a known-LOS and a known-blocked pair, and the silent-skip path should follow the `require_db_or_skip!` pattern (eprintln a reason).

#### Strong examples
- [`crates/entity/src/movement.rs:241`] `waypoint_controller_empty_path_panics` — `#[should_panic(expected = "waypoints must not be empty")]` byte-pins the panic message, so renaming the message at the source becomes a deliberate, reviewable change rather than a silent semver-major.
- [`crates/entity/src/properties.rs:156`] `index_out_of_range_panics` — same `#[should_panic(expected = ...)]` discipline.
- [`crates/entity/src/world_grid.rs:245`] `zero_cell_size_panics` — same shape.
- [`crates/entity/src/space.rs:141`] `remove_entity` — asserts both `entity_count == 0` AND `!contains_entity`, double-pinning the post-state from independent observable axes. Persona §"pin one invariant from several angles is fine".

#### Gaps
- **`base_entity.rs:202`'s `property_value_display`** is a tight Display test, but **there's no round-trip test** for `PropertyValue` serialization / wire encoding in this crate. Given properties cross the wire to clients, that's a coverage gap — though it may live in `mercury` or `services/mercury/protocol` (worth confirming with the inventory).
- **`navigation.rs` raycast/height correctness is unproven** — only "loads without crashing" is asserted. If raycast LOS computation has a bug, no test in the suite would catch it.
- **No proptest in `entity`** beyond the math/grid invariants typically need; the differential AoI test lives in `services` (correctly — that's where the diff machinery is).

### game (70 tests)
**Verdict**: **yellow**
**Sampled**: 3 tests (`npc.rs:64`, `social/mail.rs:101`, `combat/stats.rs:121`).

#### Flagged
- [`crates/game/src/npc.rs:64`] `npc_roles` — asserts six different `is_*`/`has_*` predicates in one fn (three before mutation, three after). The narrative is fine ("predicates flip when the underlying ids are set"), but the test would survive deletion of any single predicate's logic. The persona warns about "multi-assertion tests with no narrative"; this one has a narrative but is borderline. Suggested: **split** into `dialog_predicate_tracks_dialog_set_id` / `vendor_predicate_tracks_vendor_list_id` / `trainer_predicate_tracks_trainer_list_id` so a regression that broke one surfaces in its own name.

#### Strong examples
- [`crates/game/src/social/mail.rs:101`] `mail_with_item` — short, named for the assertion (`with_item` builder + `attached_item_id == Some(42)`), and pairs cleanly with the sibling `mail_with_attachments` to cover the money/item axes separately.
- [`crates/game/src/combat/stats.rs:121`] `flat_modifier_adds_to_base` — asserts the exact post-modifier value via float-tolerant compare (`< f32::EPSILON`), and the test name predicts the assertion.

#### Gaps
- **No live-DB tests in `game`** — all 70 are unit-level. That's appropriate if `game` is a domain model crate with no SQL of its own (which it appears to be), but any code path that ends up persisted (mail, faction relations) needs a regression guard *somewhere*. If the guard lives in `services`, the inventory should make that traceability obvious; if it doesn't, that's a gap.
- **No proptest** for stat-modifier composition. `flat_modifier_adds_to_base` + `multiplier_scales_total` cover the additive/multiplicative axes individually, but the *commutativity* invariant (does the order of three modifiers matter?) isn't pinned.

### content-engine (63 tests)
**Verdict**: **yellow**
**Sampled**: 3 tests (`actions.rs:285`, `actions.rs:338`, `conditions.rs:356`), plus the 3 content-engine tests in the heuristic deep-dive.

#### Flagged
- [`crates/content-engine/src/actions.rs:285`] `action_serialization_roundtrip` — body ends with `let _ = format!("{:?}", deserialized);` — proves Debug doesn't panic but doesn't prove the round-trip is *faithful*. A regression that flipped serialization fields would silently survive. Suggested: **tighten** to `assert_eq!(action, deserialized)` (requires PartialEq on Action — the conditions module already does this for `FactionRelation` at line 275) or to a `match` arm that pulls and asserts the inner fields (the test below at line 308 demonstrates this for `Action::Teleport`).
- [`crates/content-engine/src/actions.rs:293`] `property_op_serialization_roundtrip` — same shape. Suggested: **tighten**.
- [`crates/content-engine/src/conditions.rs:280`] `condition_serialization_roundtrip` — same shape. Suggested: **tighten**.

#### Strong examples
- [`crates/content-engine/src/actions.rs:338`] `grant_item_with_container` — deserializes, then `match`-destructures and asserts every field individually, *and* checks the JSON contains the literal `"container_id"` key (so a serde `#[serde(skip)]` regression on the container field would surface). The pattern is the model the three flagged tests above should adopt.
- [`crates/content-engine/src/conditions.rs:266`] `faction_relation_serialization_roundtrip` (read while resolving the conditions flag) — uses `assert_eq!(*rel, deserialized)` over a vector of variants. Cleanest of the round-trip patterns in the crate.
- [`crates/content-engine/src/conditions.rs:356`] `archetype_neq` — narrowly named, sets up the `Neq` operator and a non-matching value, asserts true. Pairs cleanly with `archetype_eq` and (presumably) other operator tests.

#### Gaps
- **Serde round-trip discipline is inconsistent**: some round-trip tests use `assert_eq!`, some use `match`-destructure, some only Debug-format. The three flagged tests are direct neighbours of correct ones — copying the neighbour's pattern is the fix.
- **No chain-replay coverage from this crate** — chain-replay tests live in `services/src/cell/content/chain_replay_tests.rs`, which is correct per TESTING.md §6, but the inventory should make that cross-crate traceability obvious for reviewers searching for "where do we test chain X".

### common (31 tests)
**Verdict**: **green**
**Sampled**: 3 tests (`math.rs:163`, `config.rs:148`, `error.rs:105`).

#### Flagged
- [`crates/common/src/config.rs:148`] `load_config_returns_default_for_now` — name says "returns default", body asserts `config.auth_port == 13001`. The name and the assertion don't agree — is `13001` the *default* port or some specific value the test is pinning? If it's the default, the test should re-fetch via `ServerConfig::default().auth_port` so a default-port change in code doesn't have to be applied in two places. Persona §"don't trust seed data; assert by relationship". Suggested: **tighten** to `assert_eq!(config.auth_port, ServerConfig::default().auth_port)` AND name the test `load_config_with_missing_path_returns_default`.

#### Strong examples
- [`crates/common/src/math.rs:163`] `vector3_sub` — pins exact post-state via `assert_eq!`, named for the operation, sits in a 14-test math suite that covers add/sub/mul/dot/cross/normalize as separate tests (one assertion focus per test, per TESTING.md §1).
- [`crates/common/src/error.rs:105`] `entity_error_display` — pins the *exact* Display output byte-string, including the prefix punctuation and message body. A regression that changed the Display impl would surface immediately.

#### Gaps
- **No proptest for `Vector3` math** — additive associativity, scalar distributivity, and cross-product anti-commutativity are textbook proptest invariants that the 14 example-driven tests can't catch a regression in. Math crates with proptest typically catch these in their first 100 cases.

### commands (29 tests)
**Verdict**: **green**
**Sampled**: 3 tests (`parser.rs:112`, `permissions.rs:71`, `parser.rs:186`).

#### Flagged
- *None.* The parser tests pair "parse_command_*" cases tightly to their inputs, and the permission tests cover same/higher/lower-level matrices in named tests.

#### Strong examples
- [`crates/commands/src/parser.rs:112`] `parse_command_no_args` — name predicts the assertion ("no args" → `args.is_empty()`), pinned alongside `parse_command_only_slash` (asserts `is_none()`) and `parse_command_empty_input` (covers `""` and `"   "`). The empty/edge-case axis is well-decomposed.
- [`crates/commands/src/permissions.rs:71`] `can_execute_higher_level` — asserts on *two independent* level pairs (Developer/Player, Admin/Moderator), so a regression that broke one direction of the comparison surfaces. Pairs with `can_execute_same_level` (line 65) and `cannot_execute_insufficient_level` (line 76) — three tests, three cases, one focus each.

#### Gaps
- **No proptest on parser robustness** — random-string-into-`parse_command` would catch panic-on-pathological-input regressions cheaply. The `mercury::parse_incoming` proptest is the model.

---

## Workspace-level patterns

- **The `proptest` story is concentrated**: of 1071 tests, only one file in `mercury` (4 tests) and one in `services/cell/space_manager` (1 test) use property-based testing. That's appropriate where it lives, but `common::math`, `entity::stats`, and `commands::parser` are textbook proptest surfaces that are entirely example-driven today.
- **Live-DB ↔ unit ratio**: ~110 tests reference `require_db_or_skip!` (per body-text grep), all in `services`. Other crates have zero. This is correct given the monolithic SQL surface lives in `services`, but it does mean **the 462 non-live-DB `services` tests carry the unit-test burden alone**, and a few of them (the cell/dispatch and cell/content "no panic = success" shape) are doing the work that should be a tighter mock-receiver assertion.
- **Concurrency coverage is narrow**: only 4 tests use `multi_thread` worker_threads, all under `inventory/{grant,move_}/`. The vendor sell/buyback path is smoke-tested but not raced — and TESTING.md §5 explicitly names racing two `join!`ed handler futures as a class of bug the smoke can't catch.
- **Wire-format vs live-DB balance**: protocol-touching crates (`mercury`, `services/mercury/protocol`) have strong byte-exact wire-format coverage. Live-DB coverage is solid for inventory/vendor/progression flows. The two surfaces don't *cross-reference* — there's no test that spans "wire packet arrived → handler executed → DB row written" end-to-end except the three PL/pgSQL smokes (which start at the SQL boundary, not the wire one).
- **Naming honesty**: across the seven sampled crates, two tests (`load_config_returns_default_for_now`, `npc_roles`) had names that didn't strongly predict their assertion. That's a low rate (~5% of sampled), but the persona file flags this exact issue (TESTING.md §"Naming"), so reviewers should scan for it.
- **Round-trip pattern drift**: `content-engine` has three different round-trip shapes (Debug-format-only / `assert_eq!` / `match`-destructure) sitting next to each other. The Debug-format-only shape is the weakest and survives serialization regressions silently. Picking one shape per crate and applying it consistently would close that drift.
- **Test density vs surface area**: `services` (550 tests / 88 files = 6.3/file) and `mercury` (97/12 = 8.1/file) are dense. `game` (70/21 = 3.3/file) and `entity` outside math/grid are sparser. `game` in particular is a model-only crate, so the lower density may be appropriate, but the absence of *any* live-DB or proptest there means a stat-stack regression that survives the unit tests has no second line of defense.

---

## Recommendations

**Priority 1 (review-blocker shape):**

- [`crates/entity/src/navigation.rs:572`] `load_and_raycast_castle_cellblock` (and its sibling at `:590` `load_and_height_query`) — asserts nothing about the raycast/height result and silently passes on missing data. **Delete, or rewrite to pin known-LOS and known-blocked pairs.** This is the canonical "code-under-test executed but not asserted on what it produced" shape; if it ever caught a regression, it caught it by accident.
- [`crates/services/src/orchestrator.rs:294`] `new_orchestrator_creates_stopped_services` — construction-doesn't-panic test of an infallible `new()`. **Delete or tighten** to assert `ServiceState::Stopped` for each service.
- [`crates/services/src/cell/dispatch/tests.rs:223`] `dispatch_trigger_region_ignores_short_args` and [`crates/services/src/cell/content/mod.rs:158`] `fire_exit_region_uses_tag_as_key` — name a behaviour but drop `_rx` so the behaviour is unobserved. **Tighten** to bind `mut rx` and assert `rx.try_recv().is_err()` (the sibling `dispatch_trigger_region_unknown_id_warns` at line 203 is the model).

**Priority 2 (taste calibration):**

- The three `content-engine` round-trip tests at `actions.rs:285`, `actions.rs:293`, `conditions.rs:280` should adopt the `match`-destructure or `assert_eq!` shape from their direct neighbours (`actions.rs:338`, `conditions.rs:266`). **Tighten.**
- `crates/common/src/config.rs:148` — rename to predict the assertion AND re-fetch the expected port from `ServerConfig::default()` so the test asserts a *relationship*, not a hard-coded constant. **Tighten + rename.**
- `crates/game/src/npc.rs:64` `npc_roles` — split into per-predicate tests so a regression on `is_trainer` doesn't fail under the `npc_roles` name. **Split.**
- `crates/defs/src/registry.rs:103` `registry_constructs_without_panic` — covered by the next test (`registry_default_is_same_as_new`). **Delete.**

**Priority 3 (nice to have):**

- Add proptest coverage for `common::math` (vector arithmetic invariants), `entity::stats` (modifier composition), `commands::parser` (no-panic on arbitrary input) — three crates with textbook proptest surfaces and zero proptest tests today.
- Add a `concurrency_tests.rs` sibling to `vendor/sell/tests.rs` and `vendor/buyback/tests.rs` racing the two handlers' UPDATE paths; the smoke catches divergence but not the race.
- Audit `entity::navigation` raycast / height correctness as a coverage gap, not just a flagged-test problem — no test in the suite proves either is computed correctly.

---

## What this report doesn't cover

- The author **did not run any tests**. All judgments are static reads of the test bodies. A regression-guard claim ("would fail on revert") is structural; only `git revert` + `cargo test` can prove it.
- Of 1071 tests, the author **read ~50 in full** (the 25 heuristic-flagged + 21 spot-checks + ~5 incidentally read while resolving flags). The other ~1020 are unaudited; this report is calibration, not coverage.
- **Cross-crate traceability** (e.g., does `game::stats` have a corresponding live-DB regression guard somewhere in `services`?) was not chased through the inventory; the documentation-writer agent's per-crate inventories are the right place to follow that.
- **DB schema** (`db/database.sql`, `db/sgw/`, `db/resources/`) was not reviewed against the SQL invariants the live-DB tests claim to pin.
- **`fuzz/` and `tools/SGWLauncher`, `tools/ContentEditor`, `tauri-app`, `launcher`, `upk-objects`, `server`** — none of these were spot-checked. The task brief named seven crates and the author respected that scope. `server` having only 2 tests (per `summary.json`) is worth a flag in any future pass.
- **Flakiness** — the author cannot judge flakiness from static reads. If any test in this report is currently flaky in CI, that supersedes the static recommendation.
- **TESTING.md gotchas around sentinel-id discipline and cleanup-by-exact-id** — the author trusted that the existing `services` live-DB tests follow the conventions documented in TESTING.md §"Sentinel id discipline" rather than re-auditing each one.
