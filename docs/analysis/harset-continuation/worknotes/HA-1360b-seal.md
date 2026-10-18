# HA-1360b seal: Marsh letter receipt seed and cross-service guard

> Type: reference. Audience: Harset coordinator, packet-coder, reviewers. Sealed: 2026-10-17 by mission-systems-advisor. Docs only: no code, seed, lab or client use.
> Companions: [HA-1360a seal](HA-1360a-seal.md), [work packets](../work-packets.md), [ID allocation](../id-allocation.md).

## HA-1360b seal

Status after this seal: **Sealed, dispatch gated on one precondition (the Castle carry-in base, below)**. A1, A2a, A2b, A3a, A3b and A3c are integrated on `content/harset-campaign` (tip `e7dd74336` when sealed). Nothing here claims a reload, a test pass or any client acceptance.

### 1. The existing branch `harset/ha-1360b-marsh-receipt`

Verdict: **stale as a Harset 1360b branch, and it is Castle CS-08a1 work.**

- Its tip `3e0b0212d` ("feat(castle): bridge Frosts Letter to Harset delivery") is patch-equivalent to Castle's `65d3e88be` (same subject, Castle files only: chain 1366 in `castle_carry_in_chains.sql`, `chain_replay_tests/mission_1360.rs`, `crates/services` round-trip test, steps 4037/4038 and objectives 4650/4651, `docs/content/mission-chains.md`). It touches no file in the 1360b allowlist below.
- It is **not** an ancestor of the campaign tip (`git merge-base --is-ancestor 65d3e88be HEAD` exits 1 at `e7dd74336`), so the campaign does not yet contain chain 1366 or the 4037 to 4038 advance that the full replay needs.
- It is based before A1 through A3c, so as a branch for this packet it would silently drop the whole letter pipeline. Do not reuse the branch or worktree as-is.

Required path (ops/coordinator, not the packet-coder):

1. Integrate the Castle carry-in onto `content/harset-campaign` with `git cherry-pick -x 65d3e88be` (it applied cleanly onto `e7dd74336` when checked; a clean apply is not a Castle review, so run Castle's own affected lane checks: `cimmeria-cell-content` and `cimmeria-services` nextest) and record the new campaign SHA. If the owner prefers Castle to merge first, the same SHA works as the base once it is an ancestor.
2. Retire the stale worktree and branch (`harset-ha-1360b`, `harset/ha-1360b-marsh-receipt`) with the retire script, never `git worktree prune`. Keep the Castle commit reachable on its own Castle branch before deleting.
3. Recreate the worktree from the base below.

### 2. Base SHA

The base is **the campaign tip that contains the CS-08a1 cherry-pick**; the SHA does not exist until step 1 above. The coder verifies instead of trusting a literal:

```powershell
git merge-base --is-ancestor e7dd74336 HEAD   # campaign tip at seal time is an ancestor
git grep -n "1366" HEAD -- db/resources/Content/Seed/castle_carry_in_chains.sql   # chain 1366 present
git log --oneline --grep "bridge Frosts Letter to Harset delivery" -1   # carry-in present
```

Any failure returns `blocked` to the coordinator. The coordinator writes the resolved SHA into the dispatch brief. HA-1360b-2 (below) has its own base: the HA-1360b-1 commit.

### 3. Split and budget

Combined this is over the ~40k Haiku working budget (one seed file, one cap-sized replay file, two linter files, a new live-DB suite with three revert probes). Split:

| Child | Scope | Branch / worktree / commit subject | Budget |
|---|---|---|---|
| HA-1360b-1 | Seed rows, mission cap, replay and linter tests, docs | `harset/ha-1360b-marsh-receipt` / `harset-ha-1360b` / `Harset HA-1360b1: seed Marsh letter receipt` | about 35k working |
| HA-1360b-2 | Cross-service live-DB guard and revert probes | `harset/ha-1360b2-cross-service` / `harset-ha-1360b2` / `Harset HA-1360b2: guard Marsh letter receipt end to end` | about 35k working; use `rust-gameserver-dev` if the harness proves heavy (Haiku stalls on L runner packets) |

b2 starts only after b1 is committed and independently reviewed (D-CC6 style: one read-only packet-reviewer per step). Reports go to `<wt>\.claude\reports\harset-continuation\HA-1360b1-coder.md` and `HA-1360b2-coder.md`.

### 4. HA-1360b-1: file allowlist (edit nothing else)

1. `db/resources/Content/Seed/harset_opcore_chains.sql`: only DSM 120101, the sequence advance, and chains 6501/6502/6505 (plus comments on those chains). 6503/6504 stay disabled and byte-identical; 6511 and every other chain untouched.
2. `db/resources/Missions/Seed/missions.sql`: only the mission 1360 row, `num_repeats` 1 to 0.
3. `crates/cell-content/src/cell/content/chain_replay_tests/mission_1360_harset.rs`: edit the existing tests below. This file is 421 lines; the hard cap is 700. If the edit would pass 600 lines, put the new tests in a new sibling `mission_1360_harset_receipt.rs` registered in `chain_replay_tests/mod.rs` (add that file to the allowlist in the report, nothing else).
4. `crates/cell-content/src/cell/content/chain_replay_tests/mission_1361/disjointness.rs`: update the 6501 expectations to the new action shape and the 120101 bind.
5. `crates/cell-content/src/cell/content/chain_replay_tests/mission_1361/handoff.rs`: comment-only.
6. `crates/content-engine/tests/it/interact_tag_linter.rs`: comment at about line 253 only, plus any expectation that names the old `remove_item`/`complete_mission` shape.
7. `docs/content/mission-chains.md`: the 6501/6502/6505 rows (engine semantics point to the bible chapter if one exists).
8. This worknote's "Result" subsection, `docs/analysis/harset-continuation/work-packets.md` (HA-1360b row) and `RESUME.md` (CRLF).

Not in b1: `crates/services/**`, any Rust under `src/`, `dialog_set_maps.sql` (the stock file), `castle_*`, `mission_steps.sql`, `mission_objectives.sql`, `db/database.sql`.

### 5. Exact seed rows and IDs

**DSM 120101** (insert after the file's comment block, before the chains; follow `harset_jaffa_chains.sql`):

```sql
INSERT INTO dialog_set_maps (dialog_set_map_id, dialog_set_id, dialog_id, topic_text, interaction_flags, min_level, missions_completed, missions_not_accepted, alignments, factions)
VALUES (120101, 1432, NULL, 'Letters Home', 268435456, 1, '{}', '{}', '{}', '{}');

SELECT pg_catalog.setval(
    'dialog_set_maps_dialog_set_map_id_seq',
    GREATEST(120101, (SELECT last_value FROM dialog_set_maps_dialog_set_map_id_seq)),
    true);
```

Comment must say: NULL dialog_id means interaction flag only (the screen is shown by chain 6501 / A3c, not by the bind); stock 5356 stays untouched because other templates may use it; GREATEST so the shared 120001+ block never lowers.

**Chain 6501** keeps its trigger (`interact_tag 'CmdCenter_Marsh'`, `once` false, scope player, sort 0) and its four conditions unchanged (`world eq 68`; `mission_status 1360 eq active`; `step_status 1360/4038 eq active`; `step_status 1361/4694 neq active`). Replace the four action rows with exactly one:

```sql
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (6501, 'complete_mission_with_item', 1360, NULL,
   '{"expected_step_id":4038,"item_id":3730,"quantity":1,"dialog_id":4576,"dialog_set_id":120101,"slot":10}',
   0, 0);
```

Read `crates/content-engine/src/loader/action_letter_turn_in.rs` first: the six keys, the value types and `target_id` 1360 must match its fixed tuple exactly (zero delay, no extra keys). Update the chain description to "take Frost's Letter via the committed receipt". Rewrite the long comments: delete the "missing letter completes the mission" paragraph and the container-filter paragraph (now false: the receipt refuses a missing item), keep the `has_item` is not authorable warning.

**Chain 6502**: change only the action `add_dialog_set 5356` to `add_dialog_set 120101` with the same params (`{"slot": 10, "mission_id": 1360}`). Conditions unchanged.

**Chain 6505**: same single change, `add_dialog_set 5356` to `120101`, params unchanged.

**Mission 1360**: `num_repeats` 1 to 0 in that one row. This is what makes the one-shot guard real: the lifecycle refuses a COMPLETED transition when `repeats > num_repeats` (`cell/missions/lifecycle.rs`), so a replayed completion is refused instead of being granted twice.

**Unchanged and asserted unchanged**: dialog set map 5356 row; 6503/6504 `enabled=false`; 6511 both triggers; chain 1361's rewards; no new chain ID (6506-6510 is not an allocation; if a new chain seems necessary stop and return `blocked` to HA-00b).

Preflight (coder runs before editing, records output in the report): `git grep -n "120101" -- db/resources` must show only the opcore file and the unrelated `dialog_screens.screen_id`; after `reload-db.ps1` on the worktree DB, `SELECT count(*) FROM dialog_set_maps WHERE dialog_set_map_id = 120101` is 1.

### 6. HA-1360b-1 tests (names are the contract)

All live-DB tests keep the existing `live_db_` prefix and skip pattern from the file; a skip is not a pass (run with the live-DB lane so they execute).

Rename or rewrite in `mission_1360_harset.rs` (old names in parentheses):

| Test | Kind | Assertion |
|---|---|---|
| `live_db_chain_6501_is_one_strict_receipt_action` (was `..._delivers_the_letter_on_the_first_marsh_click`, `..._removes_the_letter_exactly_once_in_the_seed`) | positive | Loaded chain 6501 has exactly one action, type `complete_mission_with_item`, the six-key tuple above, delay 0; no `remove_item`, `remove_dialog_set`, `display_dialog` or `complete_mission` row. |
| `live_db_chain_6501_receipt_action_reaches_the_letter_turn_in_executor` (was `..._emits_exactly_one_letter_removal_through_the_executor`) | positive | `execute_actions` on the loaded chain emits one A3b request/processing feedback and no completion, item removal or success dialog before a correlated result. |
| `live_db_chain_6501_does_not_refire_once_1360_is_completed`, `..._does_not_fire_on_the_earlier_step_4037`, `..._does_not_fire_from_harset`, `..._fails_closed_without_a_world_context`, `..._yields_to_the_praxis_turn_in` | adjacent negative | Keep as written (wrong state, wrong step, wrong world, no world, 1361/4694 overlap). |
| `live_db_chain_6501_malformed_receipt_action_drops_the_whole_chain` (new) | negative | Loading the seed text with `quantity` changed to 2 (or a key removed) yields no 6501 chain (A3a whole-chain refusal), not an ungated chain. |
| `live_db_chain_6502_rebinds_dsm_120101_on_world_entry` (was `..._rebinds_the_letter_indicator_on_world_entry`) | relog | `player_loaded 'Harset_CmdCenter'` with 1360 active at 4038 emits `add_dialog_set` 120101 slot 10; asserts the id is not 5356. |
| `live_db_chain_6502_does_not_rebind_after_delivery` | relog negative | Keep: no rebind once 1360 is completed. |
| `live_db_chain_6505_rebinds_dsm_120101_after_1361_completes` (new; add if absent) | relog/overlap | `mission_completed '1361'` with 1360 active at 4038 emits `add_dialog_set` 120101; with 1360 not active it emits nothing. |
| `live_db_dsm_120101_is_interaction_only_and_5356_is_untouched` (new) | seed | Row 120101: set 1432, `dialog_id IS NULL`, flags 268435456; row 5356 still set 1432, dialog 4576, same flags. Sequence `last_value >= 120101`. |
| `live_db_mission_1360_is_one_shot_in_the_seed` (new) | positive | `missions.num_repeats = 0` for 1360; refusing path covered by the lifecycle test in b2. |
| `live_db_567_files_chains_stay_disabled` (new; small) | negative | 6503 and 6504 `enabled = false`. |

In `mission_1361/disjointness.rs`: the existing sibling-disjointness cases must still pass with 6501's new shape (6501 vs 6512, 6527 on the same Marsh key) and a case that a click with 1361/4694 active fires only the 1361 chain. Linter test must still pass unchanged logic.

**Revert proof for b1** (committed disposable reverts, each restored afterwards; record the failing test name and the restored pass):

1. Restore the old four-action 6501 body: `live_db_chain_6501_is_one_strict_receipt_action` fails.
2. Set 6502 back to 5356: `live_db_chain_6502_rebinds_dsm_120101_on_world_entry` fails.
3. Set `num_repeats` back to 1: `live_db_mission_1360_is_one_shot_in_the_seed` fails.
4. Drop the 120101 insert: `live_db_dsm_120101_is_interaction_only_and_5356_is_untouched` fails.

### 7. HA-1360b-2: the cross-service guard

Base: the b1 commit. Allowlist: a **new** `crates/services/src/mission_letter_receipt_tests.rs` (the existing `mission_round_trip_tests.rs` is 486 lines at HEAD and 638 after the carry-in; do not grow it) registered in `crates/services/src/lib.rs` under `#[cfg(test)]`, and docs (this worknote Result, work-packets row, RESUME). If the harness needs a helper that is private in `crates/cell/src/cell/{mod.rs,service/mod.rs}` (`build_restored_missions_for_test`) or a dev-dependency in `crates/services/Cargo.toml`, the carry-in already exposes it; name any other file in the report before touching it.

Sentinel window: `0x7000_8200..=0x7000_84FF` is taken (round trip rows end at `TEST_PLAYER_BASE + 603`, abandon tests own `0x7000_84BC..=0x7000_84C1`). Use a **new** window: first confirm free with `git grep -n "0x7000_8[5-9A-F]" -- crates` and `0x7000_9`; the seal's recommendation is accounts and players `0x7000_8600..=0x7000_86FF` and mission/step/object ids not needed (real 1360/1361 rows). If it collides, pick the next free 256-block and note it in the report.

Harness rules: route the Plugin through `route_cell_message` with the installed base plugins (the older `handle_cell_message` helper has an empty plugin registry and would pass a negative for the wrong reason). Begin at actual Cellblock acceptance/grant chains 1003/1121, run the seeded CS-08a1 chain 1366 `stargate_crossed 'Harset'` advance so 4038 is persisted, hydrate fresh from the DB, then Harset world-68 6502 bind and a real nearby tagged template-10 Marsh `CM_INTERACT`. Injecting 4038 directly is a narrower receiver test only and does not satisfy this packet.

| Test | Kind | Assertion |
|---|---|---|
| `live_db_marsh_receipt_full_replay_delivers_the_letter_once` | positive | Before the result: processing feedback, no success. After the correlated result: one owned 3730 consumed; mission 1360 completed with exact terminal arrays and `repeats = 1`; `sgw_mission_1360_receipt` row; ordered dialog 4576, DSM 120101/slot 10 removal, objective 4651, step 4038, mission 1360 frames, then one 6511 offer. |
| `live_db_marsh_receipt_wrong_state_refuses_without_success` | adjacent negative | Parameterised: still on 4037, wrong world, wrong recipient (not Marsh), active 1361/4694 overlap: no consume, no completion, no success frames/events. |
| `live_db_marsh_receipt_missing_item_is_a_refusal` | adjacent negative | No 3730, foreign-owned 3730, and storage-only 3730 each refuse with a reason and no success; mission stays active. |
| `live_db_marsh_receipt_duplicate_click_is_one_attempt` | adjacent negative | Two clicks in one entry produce one request and one consume; second click reports processing. |
| `live_db_marsh_receipt_relog_after_commit_does_not_replay` | relog | Commit, drop the session, reload: DB terminal state, bind absent, 6511 offer restored by `player_loaded`, no replayed dialog or second consume. |
| `live_db_marsh_receipt_relog_before_commit_keeps_the_offer` | relog | Relog with the request unresolved: letter still held, 6502 re-adds 120101, click again succeeds. |
| `live_db_marsh_receipt_recovered_delivery_is_already_committed` | relog | Ambiguous commit then retry yields `AlreadyCommitted`; one consume total, one set of frames. |
| `live_db_marsh_receipt_two_characters_are_isolated` | adjacent negative | Two characters on the shared Marsh: A's commit does not consume, complete or unbind for B. |
| `live_db_marsh_receipt_without_6501_does_nothing` | negative | With 6501 absent from the loaded content but a registered-6501 control run in the same test: no Plugin, consume, completion or success feedback. The control must fail first-click-with-6501-removed otherwise the harness is empty. |
| `live_db_mission_1360_reaccept_is_refused_after_completion` | positive | Persisted completed with `repeats = 1` and `num_repeats = 0`: a re-accept or re-complete attempt is refused and the row is unchanged. |

Revert probes in committed disposable checkouts, each restored afterwards, results in the report:

1. Remove chain 6501 (or its action) from the seed: `live_db_marsh_receipt_full_replay_delivers_the_letter_once` and the `without_6501` control fail.
2. Remove the A3 executor/result arm for `complete_mission_with_item` (gate in `letter_turn_in*.rs`): the full replay fails.
3. Remove chain 1366's `advance_step` 4038 from `castle_carry_in_chains.sql`: the full replay fails at the 4038 persistence step.

### 8. Telemetry events (assertions, not new code)

Tests assert the existing structured events, and add none:

- `tracing` target `harset.letter_turn_in` in `letter_turn_in_route.rs` (base): events `payload_mismatch`, `stale_request`, `transaction_uncertain`, `missing_db_pool`, `transport_uncertain`, `client_sync_uncertain`, `client_sync_query_failed`, `missing_result_channel`, `closed_result_channel`.
- Cell executor `letter_turn_in.rs` refusal `reason` values: `invalid_action`, `missing_target`, `missing_session`, `ineligible_click`, `feedback_channel_unavailable`, plus the full/closed channel reason.
- Cell result consumer `letter_turn_in_result.rs`: `stale_result`, `invalid_postimage`, `mission_tracker_locked`, plus the "interaction update not queued" and "refusal feedback not queued" warns.
- The chain-level event `mission_completed` for 1360 (opens 6511). Negative tests assert the absence of this event and of success frames, not only the presence of a refusal reason.

### 9. Lane commands

Run from the packet worktree (`Set-Location <wt>;` in every call). No bash.

```powershell
pwsh tools/build-lane/lane.ps1 cargo fmt --all -- --check
pwsh tools/build-lane/reload-db.ps1                      # worktree DB only; expect 120101 row and 1360 num_repeats 0
pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-content-engine -p cimmeria-cell-content --all-targets
pwsh tools/build-lane/live-db-test.ps1 -p cimmeria-cell-content -- mission_1360_harset mission_1361
pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-content-engine -p cimmeria-cell-content
git diff --check
```

b2 adds: `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-services --all-targets` and `pwsh tools/build-lane/live-db-test.ps1 -p cimmeria-services -- mission_letter_receipt`. Confirm the exact script flags from `lane-build` skill before the first run; the commands above are the shape, and a lane failure is reported, never bypassed. After the seed change a server restart is needed for the dialog map and mission definition caches (hot reload is not enough); this matters for UAT, not for tests.

### 10. Stays BlockedEvidence or owner-manual UAT

- Witnessed client acceptance (owner-run, ask the user before any lab lease or client start): ordinary character takes 3730 in Cellblock, crosses Castle to Harset, relogs, delivers to Marsh, checks count/mission persistence and the 1361 offer, repeat click and relog, missing-letter, storage and overlap refusals. Planning, source inspection and Rust tests do not establish client acceptance.
- Whether the client renders the interaction prompt from a NULL-dialog bind (first Marsh click feedback) is unproven until the lab; the A3b processing feedback is the server-side mitigation.
- The mission 1360 chain's exactly-once visual dialog across a crash is not claimed (client has no acknowledgement).
- 567 (Romney's Files, 6503/6504) stays disabled (item 2698 has no grant path, CS-08).
- Seed rollback: restoring the old free-completion graph violates the accepted owner decision; any rollback keeps the engine primitive for in-flight durable results.
- Reward formula (`grant_xp` for 1360) is D-H10 content, not this packet.
- Items needing the Castle side: the carry-in cherry-pick, its own lane results and its UAT (CS-08a client proof) belong to Castle.

### 11. Next three packets after 1360b that can be sealed without lab or owner placement data

1. **HA-1360a2c (world-entry continuation race).** It is a general engine-side race (post-handoff continuation) provable with unit and live-DB tests; needs no positions, actors or client. Verify its current shape in `HA-1360a-seal.md` before sealing; I did not re-read it for this seal.
2. **HA-00c static half (authored/repeatable ID audit).** The stock-catalog inspection and active-branch collision audit are read-only seed scans like HA-00b; only the "first authored quest client proof" row stays blocked, so seal the audit as docs-only and leave the proof row open.
3. **HA-15b/HA-15c audit-only slice (vendor and trainer tier audit).** O14-12 already permits conservative stock/prices/trees; the existing-tier, level-band and ability-tier audit is a read-only seed measurement that needs no placement data. Seal the audit and policy table, not the vendor/trainer placement.

Not sealable now: HA-01a/b, 02-14, 16 (placement, encounter or recipient evidence), HA-17/18 and HA-19 (depend on HA-00c proof and UAT).

### Result

HA-1360b1 (coder attempt 3). Base `d2a6913cd`. Commits: `311bde360` (seed, replay and linter tests, wip 1/3), then the docs commit with the final subject.

- Preflight: `git log --grep "cherry picked from commit 65d3e88be" -1` present (carry-in). `120101` appears only in the opcore seed and the new DSM row; `reload-db.ps1` runs per live-DB invocation and the worktree DB is reloaded with the row and `num_repeats` 0.
- `live-db-test.ps1 -p cimmeria-cell-content -- mission_1360_harset mission_1361`: 42 passed (before the docs commit).
- `lane.ps1 cargo fmt -p cimmeria-content-engine -p cimmeria-cell-content`: status=ok. `lane.ps1 cargo clippy -p cimmeria-content-engine -p cimmeria-cell-content --all-targets`: status=ok.
- `lane.ps1 cargo nextest run -p cimmeria-content-engine -p cimmeria-cell-content`: 1240 passed, 0 skipped.
- Revert proofs (disposable edits to the seed or missions row, restored from a backup each time, `git status` clean after each): (1) old four-action 6501 body: `live_db_chain_6501_is_one_strict_receipt_action` fails (also the executor and malformed tests). (2) 6502 back to 5356: `live_db_chain_6502_rebinds_dsm_120101_on_world_entry` fails. (3) `num_repeats` 1: `live_db_mission_1360_is_one_shot_in_the_seed` fails. (4) 120101 insert dropped: `live_db_dsm_120101_is_interaction_only_and_5356_is_untouched` fails.
- Deviation: `mission_1360_harset.rs` is 681 lines (under the 700 hard cap, over the 600 split trigger). The new tests were not moved to `mission_1360_harset_receipt.rs`; a split is left for review.
- `git diff --check` flags the CR bytes of the CRLF docs; checked with `git -c core.whitespace=cr-at-eol diff --check`.
- Still UATPending: client acceptance, the NULL-dialog prompt in the client, and the cross-service replay (HA-1360b2).

### Result: HA-1360b2 (cross-service guard)

Branch `harset/ha-1360b2-cross-service`. Tests only; no production code changed. New files: `crates/services/src/mission_letter_receipt_tests.rs` (harness plus full-replay, pending and one-shot re-accept tests) and `crates/services/src/mission_letter_receipt_tests/negative.rs` (extra file, eight adjacent-negative, relog and refusal tests). The harness drives the real cell and base through `route_cell_message` with the shipped `base_plugins()` table and the production `build_engine(Some(&pool))`. Sentinel window `0x7000_8600..=0x7000_86FF`.

- Seal discrepancy: section 7 expects "one 6511 offer" after the receipt. It is not a frame. Chain 6511 (`mission_completed 1360`) runs in the same result application and adds a slot-10 bind (DSM 5254) via `add_dialog_set`, which pushes only InteractionType flags. The test asserts the bind (slot 10 holds 5254, not 120101) and exactly four frames [105, 82, 81, 80]. `load_single_chain_for_test` loads only 6511's first trigger, so the harness uses `build_engine`. No production change is needed; the seal wording should read "one 6511 bind".
- Also asserted: mission 1360 completed, step None, completed_steps [4037, 4038], completed_objectives [4650, 4651], active [4651] (production postimage), repeats 1, letter count 0, one receipt row, 1361 not accepted.
- Negative cases (negative.rs): wrong state (4037, wrong world, wrong tag, 1361 at 4694) refuses with no success frames; missing, foreign-owned or storage-container letter is a refusal; duplicate click is one attempt; relog after commit does not replay; relog before commit keeps the offer; recovered delivery is AlreadyCommitted; two characters are isolated; without chain 6501 nothing reaches base.
- Revert probes (disposable edit, `live-db-test.ps1 -p cimmeria-services -- mission_letter_receipt`, restored with `git checkout` and mtime touch, status clean each time): (1) 6501 disabled: 1 passed, 10 failed. (2) executor `CompleteMissionWithItem` arm no longer calls `letter_turn_in::request`: 1 passed, 10 failed. (3) chain 1366 `advance_step` target changed 4038 to 4037: 0 passed, 11 failed.
- Not covered: telemetry event assertions beyond what the test log capture exposes cheaply (seal section 8); left to review.
- Still UATPending: client acceptance and the NULL-dialog prompt in the client.