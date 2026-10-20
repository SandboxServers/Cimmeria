# CS-08a2S — HA-1360 recipient interface seal

> **Type**: Reference. **Audience**: Castle coordinator, CS-07R and CS-07b implementers.
> **Date**: 2026-10-18. **Status**: SEALED, docs-only. **Client**: UATPending.
> **Companions**: [CS-08a worknote](CS-08a.md), [Castle ledger](../README.md), [R3 packets](../r3-packets.md).

Provenance labels: **VERIFIED** (read at the named git ref this session), **INFERRED** (derived from verified facts), **UNVERIFIED** (not checked). No lab, DB, compile or Harset write was done. Harset was read with `git show`, `git grep` and `git log` only.

## Verdict

**SEALED.** HA-1360a3c and HA-1360b are both in `git log content/harset-campaign`, so the gate is open.

- **CS-08a2: Dropped (Harset owns).** Harset's recipient tests already cover both CS-08a2 tests, and they run on a tree that holds both the Castle bridge and the Harset receipt. This holds whichever PR reaches `main` second.
- **CS-08a2M: Dropped.** Harset did not extend `mission_round_trip_tests.rs`, so there is nothing to move. Moving the Castle block would only manufacture a merge conflict.
- **Harset-facing note: no relay needed.** RESUME next-action 5 (point HA-1360b at `frosts_letter_round_trip_tests.rs`) is retracted by this seal.

## 1. Harset reference and the path from 4038 to completion

**VERIFIED.** Harset ref `content/harset-campaign`, tip `c8eb00fee` (`c8eb00feed14222b9efe3a9fed0906999574de3b`). Both ranges are ancestors of the tip:

| Packet | Commits |
|---|---|
| HA-1360a3c | `0c00de373` (apply correlated letter turn-in result), `1506cc4fb` (pin exact result frames and recovery paths) |
| HA-1360b2 | `b25502c9b`, `316e9ef78`, `63b29f0f5`, `79e7a1de9` (guard Marsh receipt end to end, review fixes A to C) |

The Harset ledger records the integrating merges as a3c `e7dd74336`, b1 `e1c03b107`, b2 `9455f2af2` and the Castle carry `d2a6913cd` (`harset-continuation/RESUME.md:21`, **VERIFIED** as ledger text only; I did not re-verify those merge SHAs).

Chain path, in `db/resources/Content/Seed/harset_opcore_chains.sql` at the tip, except chain 1366, which is in `castle_carry_in_chains.sql`:

| Step | Chain | Trigger | Conditions | Action | Lines |
|---|---|---|---|---|---|
| 4037 to 4038 | **1366** (Castle) | `stargate_crossed`, key `Harset` | `world 8`, `mission_status 1360 active`, `step_status 1360/4037 active` | `advance_step 1360 -> 4038` | `castle_carry_in_chains.sql:4-16` |
| Bind the receipt on load | **6502** | `player_loaded`, key `Harset_CmdCenter` | `world 68`, `step_status 1360/4038 active`, `step_status 1361/4694 neq active` | `add_dialog_set 120101` `{"slot":10,"mission_id":1360}` | `harset_opcore_chains.sql:274-307` |
| Bind after 1361 completes | **6505** | `mission_completed` key `1361` | `world 68`, `mission_status 1360 active`, `step_status 1360/4038 active` | `add_dialog_set 120101` | `:327-339` |
| Receipt and completion | **6501** | `interact_tag`, key `CmdCenter_Marsh` | `world 68`, `mission_status 1360 active`, `step_status 1360/4038 active`, `step_status 1361/4694 neq active` | one `complete_mission_with_item`, target 1360, params `{"expected_step_id":4038,"item_id":3730,"quantity":1,"dialog_id":4576,"dialog_set_id":120101,"slot":10}`, delay 0 | `:209-251` (conditions `:216-237`, action `:239-251`) |

Where the letter is consumed (**VERIFIED**): not in the chain. The action only requests the receipt; the base transaction `commit_letter_turn_in_inner` (`crates/base-methods/src/base/world_entry/methods/missions/letter_turn_in.rs:171-356`) deletes (stack 1) or decrements the 3730 row (`:257-281`), updates `sgw_mission` to status 2, step NULL, `repeats = 1` (`:293-306`), and inserts a `sgw_mission_1360_receipt` row (`:328+`) in one transaction. The cell then installs the postimage and shows dialog 4576, removes the 120101 bind and fires `mission_completed 1360` (`crates/cell-content/src/cell/content/letter_turn_in_result.rs:191-246`).

**The one interface fact Castle must keep true (VERIFIED).** The base accepts only the exact Castle postimage (`letter_turn_in.rs:86-94`): status 1, step 4038, completed steps `[4037]`, completed objectives `[4650]`, active objectives `[4651]`, no failed objectives, `repeats 0`. Chain 1366's `advance_step` produces exactly that, and Harset's own b2 tests drive the real Castle chain to reach it (`mission_letter_receipt_tests.rs:422-441`, `RECEIPT_CHAINS = [1003, 1121, 1366, 6501, 6502]` at `:75`). A change to chain 1366, steps 4037/4038 or objectives 4650/4651 must be re-run against those tests.

Mission 1360 is one-shot on Harset: `num_repeats` 1 to 0 (`HA-1360b-seal.md:92`, **VERIFIED**).

## 2. Missing-letter refusal and its telemetry

**VERIFIED.** The base refuses with `LetterTurnInRefusal::NoAccessibleLetter` (`letter_turn_in.rs:231-245`) when no 3730 is owned by the character in an accessible bag with stack at least 1. That covers no letter, a foreign-owned letter and a storage-only letter (container 99). The refusal returns before any mutation, so the letter, the mission row (still active at 4038) and the receipt table stay unchanged.

The cell side (`letter_turn_in_result.rs:158-190`):

- clears the pending state;
- logs `tracing::info!` with fields `entity_id`, `player_id` and `?reason` (value `NoAccessibleLetter`), message `Frost letter turn-in refused` (`:164-169`);
- sends one `onPlayerCommunication` feedback line, `Frost's Letter is not in an accessible bag.` (`:171-173`);
- never calls `remove_letter_binding`, so the 120101 bind stays (**INFERRED** from the refusal branch containing no bind removal; **not asserted by a test**, see section 5).

Other refusal reasons share the same log line: `WrongMissionState`, `AlreadyCompleted`, `StateChanged`, `InvalidCorrelation`, `NoPlayer`. Executor-side drops use `reason = "ineligible_click"`, `"missing_session"`, `"missing_target"`, `"invalid_action"` (`executor/letter_turn_in.rs:154-239`). Castle adds no telemetry of its own.

## 3. Regression-contract steps 3 and 4: the Harset tests that already cover them

The CS-08a worknote contract, steps 3 and 4 ([CS-08a.md](CS-08a.md), "Regression and persistence contract"), mapped to Harset tests. All **VERIFIED by reading**; none was run (no DB, no compile).

Shared fixture (`mission_letter_receipt_tests.rs`): sentinel ids `0x7000_8600..=0x7000_86FF` (`:51`), production `base_plugins()` routing, chains `RECEIPT_CHAINS`. `stage()` (`:395-443`) does: fresh Cellblock hydrate with 622/2113 active; `fire_dialog_open(3995)` (seeded 1003/1121 grant and accept); assert one letter and step 4037; fresh Castle hydrate; `fire_stargate_crossed("Harset")`; assert step 4038; fresh Harset hydrate beside a tagged Marsh with the 6502 bind fired.

| Contract item | Test (file) | Fixture and assertions |
|---|---|---|
| **Step 3: recipient completes 1360** (CS-08a2 test 1) | `live_db_marsh_receipt_full_replay_delivers_the_letter_once` (`mission_letter_receipt_tests.rs:601`) | `stage_at_marsh(.., connect = true)`; one click queues one request with processing feedback and no dialog or journal frame; routed through the base; correlated result applied. Frames are exactly `[105, 82, 81, 80]` (`:636-658`); letter count 0; row status completed, step NULL, steps `[4037, 4038]`, objectives `[4650, 4651]`, active `[4651]`, `repeats 1`; one receipt row; 120101 gone, 5254 bound on slot 10 (`:675-687`). |
| Step 3: duplicate receipt | `live_db_marsh_receipt_duplicate_click_is_one_attempt` (`negative.rs:228`) | second click is not a request and reports processing; one result; one consume; one receipt. |
| Step 3: relog | `live_db_marsh_receipt_relog_after_commit_does_not_replay` (`negative.rs:266`), `..._relog_before_commit_keeps_the_offer` (`:301`), `..._recovered_delivery_is_already_committed` (`:328`) | no replayed frames; offer restored; no second turn-in; terminal row and one receipt; pre-commit relog keeps the letter and re-binds. |
| Step 3: another player unchanged | `live_db_marsh_receipt_two_characters_are_isolated` (`negative.rs:413`) | B keeps letter, active row at 4038, no receipt, letter bind; B then delivers its own. |
| Step 3: no re-completion | `live_db_mission_1360_reaccept_is_refused_after_completion` (`reaccept.rs:25`) | completed row with `repeats 1` and `num_repeats 0` refuses re-accept. |
| **Step 4: missing letter refuses** (CS-08a2 test 2) | `live_db_marsh_receipt_missing_item_is_a_refusal` (`negative.rs:146`) | three scenarios (no letter, foreign-owned, storage-only), each with one request, outcome `Refused(NoAccessibleLetter)`, no mission frame and no 4576 dialog, mission not completed, no receipt row, step still `4038`; the storage letter is still owned. |
| Step 4: consume before complete | `crates/base-methods/src/base/world_entry/methods/missions/letter_turn_in_tests.rs:338` `live_db_letter_turn_in_rolls_back_after_item_and_mission_mutation`; `:364` `..._fences_stale_writers`; `:263` `..._refuses_unowned_or_inaccessible_letter`; `:303` `..._refuses_malformed_castle_postimage` | transaction rolls back both item and mission on failure; stale writer fenced; malformed Castle postimage refused. |
| Step 4: wrong state | `live_db_marsh_receipt_wrong_state_refuses_without_success` (`negative.rs:74`) | no request, nothing consumed or completed. |

The two CS-08a2 test names and their coverage:

- `live_db_frosts_letter_recipient_completes_1360` is covered by `live_db_marsh_receipt_full_replay_delivers_the_letter_once`, with the stronger precondition of driving the real Cellblock grant and Castle crossing instead of an injected 4038.
- `live_db_frosts_letter_missing_letter_refused` is covered by `live_db_marsh_receipt_missing_item_is_a_refusal`.

Revert strength (**VERIFIED as claimed in Harset's ledger, not re-run**): `HA-1360b-seal.md` records three revert probes; removing chain 1366 strands the unchanged round-trip at 4037 (`mission_round_trip_tests.rs:494`, identical on both branches).

## 4. The D-CA15 call: which PR reaches `main` second

**Ledger state (VERIFIED, 2026-10-18).** `main` is `e3fe1e813`. `content/harset-campaign` is 91 commits ahead of it with merge-base `817d3792c`; `content/castle-campaign` is 161 ahead with merge-base `e3fe1e813`. `gh pr list --state open` shows no Castle or Harset PR (only #1735, a Union planning doc). Neither campaign has a PR, so the order is not fixed by any artifact.

- Harset: HA-1360 a1 to a3c, a2c, b1, b2 integrated; owner-manual UAT pending (`harset-continuation/README.md:220`, `RESUME.md:21-23`).
- Castle: R3b packets are in flight; OD-1, OD-2, OD-3 and D-CA16 are owner-blocked, CS-06a/06b wait on HA-01b, CS-07c is a user step, and the PR comes after all of them (`RESUME.md:20,37`).

**Call (INFERRED): Castle reaches `main` second.** Its tail is longer and owner-blocked; Harset's 1360 work is already integrated. This is a projection, not a commitment, and the packet verdicts below do not depend on it.

**Why CS-08a2 is Dropped under either order** (VERIFIED facts, INFERRED conclusion):

1. Harset's branch already holds both sides. `castle_carry_in_chains.sql`, `mission_1360.rs`, `mission_round_trip_tests.rs`, `mission_steps.sql` and `mission_objectives.sql` are byte-identical between Castle `224e7692e` and the Harset tip (`git diff --quiet` exit 0 on each). Harset carried them as `d2a6913cd`, a cherry-pick of `65d3e88be`. D-CA15's premise ("only a branch holding both sides can test the round trip") is satisfied by Harset's own branch today.
2. If Harset reaches `main` second, its PR carries the tests, which pass in its branch. CS-08a2 is Dropped (Harset owns), as the brief specifies.
3. If Castle reaches `main` second, CS-07R rebases onto a `main` that already contains Harset's receipt and its `mission_letter_receipt_tests*` files. The shared files are identical, so the rebase has no content conflict on them. Those tests then run in Castle's gate. A Castle copy of the same two tests would duplicate them under new names.

**Harset-facing note: no relay needed.** Harset's tests live in `crates/services/src/mission_letter_receipt_tests.rs` and its `negative.rs` and `reaccept.rs` submodules (698, 511, 70 lines at the tip; the parent is over the 500 soft cap but under 700). They are not in `mission_round_trip_tests.rs`, which is 672 lines (638 non-blank) on both branches. So the F16 pressure that motivated D-CA15's file move did not materialise.

## 5. Residuals and standing constraints

- **Standing constraint for CS-07R and CS-07b (INFERRED).** Because Harset carries Castle's slice byte-for-byte, any later Castle edit to chain 1366, steps 4037/4038, objectives 4650/4651, `mission_1360.rs` or `mission_round_trip_tests.rs` before the merge will conflict with Harset's copy. Keep those files frozen until the second PR merges, or re-run the Harset receipt tests after any change. This is a rule for Castle, not a request to Harset.
- **Small coverage gap, accepted (VERIFIED absence).** The missing-letter test asserts no completion, no receipt, no success frames and step 4038, but does not assert that the 120101 bind remains after a refusal. `live_db_marsh_receipt_relog_before_commit_keeps_the_offer` (`negative.rs:301-320`) covers the bind only on the pre-commit relog path. The refusal branch has no bind removal (section 2), so the behavior holds; it is not guarded by a test. Not worth a Castle-side duplicate test; the coordinator may mention it to Harset opportunistically.
- **Not proven by any of the above (UNVERIFIED):** any client behaviour (journal at 4038, Marsh dialog 4576, processing and refusal feedback lines), a lab run, or Harset's tests passing on `224e7692e` itself. The Harset tests were read, not run. Owner-manual UAT stays UATPending in both ledgers.
- **Ledger edits owed to the coordinator, not done here (allowlist):** README CS-08a2 row to Dropped (Harset owns); README CS-08a2M row to Dropped; D-CA15 row amended (file move not needed; Harset recipient tests are in `mission_letter_receipt_tests*`); F16 resolved; RESUME next-action 5 retracted, next-action 6 loses "CS-08a2".

## Frozen rows

**None.** This seal freezes no SQL row, seed edit or Castle test fixture, because both downstream packets are Dropped. The Harset tests in section 3 are the frozen reference; they are Harset-owned and not copied.

## Consumer packets

| Item | Consumer | Action |
|---|---|---|
| CS-08a2 verdict | Castle coordinator (README R3 row) | Mark Dropped (Harset owns). No coder dispatch. |
| CS-08a2M verdict | Castle coordinator | Mark Dropped. No test moves; `mission_round_trip_tests.rs` stays at 672 lines. |
| Standing constraint (section 5) | CS-07R (rebase and merge) | Treat chain 1366 and the five identical files as frozen; after the rebase run Harset's `mission_letter_receipt_tests*` and Castle's `mission_1360` / `castle_carry_in` guards. |
| Coverage gap, UAT rows | CS-07b (G-REVIEW), end-UAT | Record the unasserted bind-after-refusal; UATPending rows: crossing, journal 4038, Marsh success and refusal, relog. |
