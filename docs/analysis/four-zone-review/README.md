# Four-zone campaign: pre-PR review

> **Scope:** branch `campaign/four-praxis-zone-restoration` (Cellblock, Tollana, Harset, Castle), reviewed at `f08425985`. The diff base is `da1703d01` (the merge base with `origin/main`). There are 453 commits. `origin/main` was at `eb29b0c2c` when reviewed (78 commits ahead).
> **Reviewer:** architect (Opus), 2026-10-18. Read-only review: no code was changed.
> **Companions:** [decisions](decisions.md), [resume card](RESUME.md), packets in [packets/](packets/). Earlier reviews: `.claude/reports/harset-continuation/four-zone-re-review.md` (ship at `b28b4fa11`) and the Harset reviewer pass folded in below.

## Verdict

> **Update 2026-10-19 (FZ-11 triage):** FZ-01 to FZ-10 are integrated at `db22c4c00`, and nothing is pushed. FZ-11 failed with 7 items; F22 to F29 below triage them, and waves 6 to 8 (FZ-13 to FZ-17) fix them. The text under this note is the review as of 2026-10-18.
>
> **Update 2026-10-19 (waves 6-8 done):** FZ-13 to FZ-17 are integrated and the campaign tip is `d3d5d62e0` (nothing pushed). The FZ-17 rerun passes 11 of 12 blocking rows; row 9 is accepted as environmental (see [RESUME](RESUME.md)). Remaining: owner decisions, the campaign review loop, lab UAT FZ-U1 and one PR.

**Not ready for a PR yet.** The workspace nextest run (`--profile=ci`, CI's excludes) passes 12,534 tests and fails 2, and both failures block the merge queue:

- the Rule 6 unpaired-ID ratchet: 87 new fields in 13 files;
- the OTEL target scan: `harset.letter_turn_in` at DEBUG.

The branch is also 78 commits behind main, with 12 conflicting files. Once merged, main's persistent-player-data lint fails on the new table `sgw_mission_1360_receipt`. Everything else found is Medium or lower.

Counts: 29 findings (7 High, 5 Med, 8 Low, 9 Info), 31 packet rows and 23 decision rows. Twelve fix packets (FZ-01 to FZ-10, with FZ-03 split three ways) run in three waves. FZ-11 verifies the integrated branch. FZ-12 ran after the owner answered D-FZ1 "enable" (merge 086656cd8).

Evidence for the reds:

- the lane log `%LOCALAPPDATA%\cimmeria-build\logs\four-zone-campaign\20261018-205932-33376.log`;
- its `.failures.txt`, from the workspace run;
- `20261018-205307-10492.log`, from the `cimmeria-server logging` run: 111 passed, 2 failed.

## Findings

Severity:

| Severity | Meaning |
|---|---|
| High | Blocks the PR or merge queue |
| Med | A real defect or a guard gap that ships a silent failure |
| Low | Hardening or stale text |
| Info | No packet; noted or decided |

| Id | Sev | Where (file:line) | Evidence | Failure scenario | Packet |
|---|---|---|---|---|---|
| F1 | High | `crates/server/src/logging/unpaired_id_tests/mod.rs:351` (ratchet). 13 source files: see [FZ-03a](packets/FZ-03a.md), [FZ-03b](packets/FZ-03b.md), [FZ-03c](packets/FZ-03c.md) | `unpaired_id_fields_only_shrink` fails. The new tracing lines have 87 `*_id` fields with no `*_name` pair and no `// nt:id-only` marker (16 in `letter_turn_in_route.rs`, 17 in `executor/letter_turn_in.rs`, 15 in `letter_turn_in_result.rs`, ...). The baseline is `# total 0`. | CI `build-and-test` is red, so the merge queue rejects the PR. The log rows are also unreadable in SigNoz: an ID with no name. | FZ-03a, FZ-03b, FZ-03c |
| F2 | High | `crates/base-methods/src/base/world_entry/methods/missions/letter_turn_in_route.rs:97`; `crates/server/src/logging/filters.rs:314` | `every_source_target_reaches_signoz_at_its_level`: `harset.letter_turn_in at DEBUG ... reaches 0 OTLP log indexes`. `OTEL_FILTER` has no `harset.letter_turn_in` row, so its DEBUG `stale_request` event never exports. | CI is red. A stale letter turn-in request leaves no trace in SigNoz. | FZ-04 |
| F3 | High | `git merge-tree HEAD origin/main` | There are 12 conflicting files: `README.md`, `crates/resources/src/base/mission_additions.rs`, `.../tests/mission_additions.rs`, `.../tests/mod.rs`, `crates/resources/src/lib.rs`, `docs/agents/doc-update-map.md`, three in `docs/analysis/castle-continuation/` (README, RESUME, work-packets: add/add), `docs/analysis/four-zone-planning/id-allocation.md`, `docs/architecture/mission-pak-overrides.md`, `docs/testing/inventory/README.md`. | The PR cannot open clean. Main's Dakara 63100 mission addition must join the branch's ten entries, or one side's missions vanish from the served pak. | FZ-01 |
| F4 | Med | `db/sgw/Missions/Tables/sgw_mission_1360_receipt.sql:2`; on main: `db/persistent/persistent-public-tables.txt`, `crates/content-engine/tests/it/table_classification_linter.rs` (`assert_eq!(persistent.len(), 24`) | The branch adds one public table. Main's PP campaign requires every `db/sgw` table to be classified exactly once, and pins the persistent list at 24. The table is player data: FK to `sgw_player`, ON DELETE CASCADE. | After FZ-01, `real_tables_are_classified_exactly_once` and the live `persistent_schema_snapshot` fail. Worse, if forced through, a container rebuild would not carry the receipts, so a player could turn Frost's Letter in a second time after a release. | FZ-02 |
| F5 | Med | Repo root `lab-server.err.log`, `lab-server.err.prev.log`, `lab-server.out.log`, `lab-server.out.prev.log` (added in `836e12091`); `.claude/reports/harset-continuation/HA-1360b-seal-advisor.md` (force-added under the ignored `.claude/reports/`) | `git diff --diff-filter=A da1703d01 HEAD`. | Runtime logs ship to main and every clone. The report sits in a gitignored folder, where the next `git add .claude/reports` surprises someone. | FZ-05 |
| F6 | Low | `crates/cell-methods/src/cell/cell_methods/player/interaction/mod.rs` (879 lines) | The hard cap is 700. The file was 871 at the base and on main, and the campaign added 8. CI has no guard. | The file grows past the cap with each campaign. Most of its lines are an inline `mod tests` (lines 73-879). | FZ-06 |
| F7 | Low | `db/resources/Content/Seed/harset_opcore_chains.sql:345-373,408`; `crates/cell-content/src/cell/content/chain_replay_tests/mission_567.rs:4-12,29-31,69,83,89-91` | The text says item 2698 "HAS NO GRANT PATH ANYWHERE". Castle chain 1397 (`castle_567_chains.sql`) now grants 2698 (`add_item 2698`). | A reader enables 6503/6504 on that stale reasoning without the Harset HA-D04 pairing, or never enables them. | FZ-07 |
| F8 | Low | `crates/resources/src/base/resources/tests/seed_id_uniqueness.rs:267-302` | `CASTLE_CHAIN_FILES` is a hand list of 6 files. It already misses the campaign's own `castle_carry_in_chains.sql` (chain 1366). The four legacy files (`castle_701`, `castle_702_704`, `castle_706_708`, `castle_cellblock`) sit outside the Castle block by design (`id-allocation.md`), so the fix is a glob with an explicit legacy list. | A new `castle_*chains.sql` file escapes the Castle id-range check, so a chain id can land in another zone's block. | FZ-07 |
| F9 | Med | `crates/minigame/src/minigame/session_result.rs:47-57`; `crates/minigame/src/minigame/server/terminal_outcome_tests.rs:322` | `invalidate_entity_receipts` drops a deposited **victory** receipt on cancel, replacement or removal, and logs nothing. The test named `..._keeps_enqueued_victory` asserts the receipt is gone (`redeem_result(..).is_none()`). | A player wins Livewire and cancels or relogs in the same instant. The victory chain silently does not fire, and SigNoz shows only a `result_receipt_missing` WARN on the base, with no cause. The policy itself is D-FZ2. | FZ-08 |
| F10 | Low | `crates/minigame/src/minigame/session_result.rs:120` | `\|\| session.terminal_reported` is the only double-deposit guard, and no test pins it. | A refactor drops it, and two victory receipts fire the chain twice. | FZ-08 |
| F11 | Med | `crates/content-engine/src/loader/mod.rs:282`; `crates/cell-content/src/cell/content/event_dispatch/direct_invoke.rs:62` | The loader builds `__direct_invoke_{id}` only for triggerless chains, and the guarded victory path matches only that event. Only chain 1372 is pinned triggerless (`mission_1520_seed_guard.rs`); 1375 and future conditioned victory chains are not. | A content author adds a trigger to a conditioned victory chain (1375). The minigame win then logs `guarded_callback_not_matched` at INFO and never advances the mission. | FZ-09 |
| F12 | Low | `crates/cell-content/src/cell/content/event_dispatch/direct_invoke_tests.rs:165-217` | The disabled and real-trigger cases reuse a manager whose `callback` counter is already 1, and assert 1. | The asserts still catch a run (it would make 2), but they read as "it ran once". A future edit to `== 1` semantics hides a run. | FZ-09 |
| F13 | Low | `crates/base-methods/src/base/world_entry/methods/missions/letter_turn_in_concurrency_tests.rs:55-88` | The delete-race leg spawns the turn-in and deletes at once, with no proof that the turn-in is parked on the row lock. | The DELETE usually commits before the turn-in starts, so the test passes trivially (`NoPlayer`) even with `FOR KEY SHARE` removed. | FZ-10 |
| F14 | Low | `crates/base-world-entry/src/base/world_entry_appearance/client_ready/cell_handoff.rs:18,92-100`; `crates/base-session/src/base/helpers/letter_turn_in_send.rs:21-32` | Two copies of `TOKEN_DOMAIN` and the SHA-256 token derivation. | One copy changes and the other doesn't. Every turn-in is then refused `InvalidCorrelation`, and no test notices. | FZ-10 |
| F15 | Info | `encrypted/mod.rs` 697, `mission_1520.rs` 693, `mission_1360_harset.rs` 689, `event_dispatch/mod.rs` 684, `dialog_overrides/mod.rs` 674, `gate_travel/mod.rs` 612, `reliable_fit.rs` 603, `services/src/mission_letter_receipt_tests.rs` 698 | These are under the 700 hard cap but over the 500 soft cap. | The next edit to any of them must split it first. | none |
| F16 | Info | `.github/workflows/test.yml` `WORKSPACE_EXCLUDES`; `src-tauri/Cargo.toml` | CI excludes `cimmeria-app` (`src-tauri`) and the editors from clippy and build. The campaign did not touch the editors, but it changed `crates/services` (`lib.rs`, `plugins.rs`, `Cargo.toml`), which `cimmeria-app` depends on. | A services API change breaks the app build, and no CI job sees it. | D-FZ4 (FZ-11 row 12) |
| F17 | Info | `docs/gap-analysis*`, `docs/project-status.md` | The four campaigns' status-doc close-out is not done. | Status docs lag four zones. | D-FZ5 |
| F18 | Info | 453 commits | Decided: squash (D-FZ3, owner 2026-10-19). | A noisy `main` history, or the evidence SHAs cited in ledgers are lost on squash. | D-FZ3 |
| F19 | Info | `harset_opcore_chains.sql` 6503/6504; `castle_567_chains.sql` 1394-1397, 10240-10243 | Mission 567 ships fully gated: those ten chains are `enabled = false`, and only the cabinet feedback chain 1400 is enabled. `live_db_castle_567_chains_ship_gated` and `live_db_chains_6503_and_6504_are_disabled_pending_the_2698_grant` pin it. **Resolved by FZ-12** (D-FZ1 = enable, merge `086656cd8`): all ten chains are enabled; the gated-state tests were replaced by enabled-state tests. | None now: FZ-12 enabled the leg; the client proof is the owner's lab run FZ-U1. | Resolved (FZ-12) |
| F20 | Info | main `db/persistent/reconcile.sql` Block 1 and Block 3 | Reconcile can delete a letter instance whose type vanished. The receipt keeps `item_id`, with no FK. Block 3 has no count row for the receipt's `mission_id`. | Harmless: the receipt is history. A Block 3 count row is optional. | D-FZ6 |
| F21 | Info | `dialog_set_maps_dialog_set_map_id_seq` setval 100007 (`dialog_set_maps.sql:9366`); 14 seed ids above it, up to 1000018 | Resources are seed-only; console authoring emits SQL and never writes live rows. | Only a future live insert into `dialog_set_maps` would collide. | D-FZ7 |
| F22 | High | 7 files: `chain_replay_tests/{mod,na13_chain_armed_aggro}.rs`, `executor/tests/mod.rs`, `spawner_tests/mod.rs`, `interact_tag_linter/allowlist.rs`, `db/database.sql`, `docs/testing/inventory/README.md` | `git merge-tree HEAD origin/main` at `c992d645b`, 44 commits past the last merge base `102d15444`: main's Union Omega, Union SGC and Beta E1 work collide with the Tollana and Castle lines. All are union resolutions. | The PR cannot merge. | FZ-13 |
| F23 | Low | `docs/readme.md:41-43` | The campaign added a second copy of the `docs-md-count`, `tests-*` generated trio (values 1,120 / 14,696 / 1,813) above the real one. A merge with main makes three copies. | `regen.py` rewrites both copies forever, and the table shows the same row twice. | FZ-13 |
| F24 | High | `crates/cell-world/src/cell/spawner_tests/live_db_aggression.rs:166-184` | FZ-11 row 9: `seed_overrides_only_the_chain_armed_spawns` sees an extra `(425, 30.0)`. Template 425 `Castle_DefenseQ3_Giver` (CS-04b3s) is a sealed copy of 188 (`castle_side_defense_q3_giver_rows_frozen`). The test is stale, not the seed. | CI's live-DB job is red. | FZ-14 |
| F25 | High | `crates/names/src/namebook_gaps.txt` | FZ-11 row 9: 12 new unnamed dialogs (10700-10734), 9 new unnamed dialog sets (10760-10776), and 2043 now named (dsm 1000018). All come from campaign seeds; the `override_seed_agreement_castle_*` seals freeze the empty names. | CI's live-DB job is red. | FZ-14 (bless, D-FZ8) |
| F26 | High | `crates/lab/src/uat/seed_tags.rs:559-561` | FZ-11 row 10: `assert_eq!(seed.tags.len(), 608)`. The tag count is 616 at the merge base (main's Dakara E1 seed), 630 on the branch, and 713 on main. `lab.yml` is path-filtered, so main is red too and never noticed. The campaign touches `crates/lab`, so the PR runs it. | The lab job is red on the PR, and every future seed change breaks it again. | FZ-15 (D-FZ8) |
| F27 | Med | `crates/server/src/logging/filters.rs` | 741 lines, over the 700 hard cap. Main has it at 735; FZ-04 added 6. | The next OTEL row makes it worse. | FZ-16 |
| F28 | Info | `README.md`, `crates/README.md`, `docs/readme.md`, `docs/agents/doc-update-map.md`, `docs/reverse-engineering/*`, `docs/testing/inventory/README.md` | FZ-11 rows 11 and 13: `regen.py --check` exits 1. The drift is main's own plus the campaign's new tests. CI runs it as informational, and main's `regen-docs` bot rewrites the blocks after the merge. The campaign's three hand-drifted inventory rows go away in FZ-13 (main's side). | None after FZ-13; the counts refresh on main. | none (ledger note) |
| F29 | Info | `docs/reverse-engineering/findings/README.md`; `.claude/reports/persistent-player-data/PP-13-coder.md` | FZ-11's file checks diffed against the stale base `da1703d01`. Against the true merge base `102d15444` both files are identical to main's: the mixed line endings came with main's #1854 (stored `-text`, `merge: union`), and the report with main's #1851. Main also tracks `.claude/reports/union-*` reports. | Not the campaign's. A main-side cleanup issue is worth filing (CRLF of that README, tracked `.claude/reports/`). | none (main follow-up) |

Checked and clean (Harset reviewer plus this pass): the turn-in lock order, the one-shot PK, the idempotent retry, the FK cascade, `live_db_` prefixes, seed id uniqueness across files, the mission-addition ids versus `prepare` (`all_ids` rejects duplicates batch-wide), and the 62050 stale-response fixture (it mirrors production). Rejected as not defects: the `pending.take()` order, the missing retry cap for `UnknownRetryable`, no resync on `AlreadyCompleted`, no rate limit on the 1360 WARN, `LIMIT 1 FOR UPDATE` under advisory locks, the hard-coded `Harset_CmdCenter` / world 68, and promoting the direct-invoke miss from INFO to WARN (FZ-09 covers the cause instead).

## Packets and waves

All packets build on the campaign branch. They do not start from `origin/main`.

Each packet runs in its own worktree. `ops` creates it with `mk-worktree.ps1 four-zone-review/FZ-NN-<slug> fz-NN`, then rebases the empty branch onto the recorded campaign tip (a fast-forward). When the packet's review loop says ship, `ops` merges the packet branch into `campaign/four-praxis-zone-restoration` with `merge --no-ff` in the `four-zone-campaign` worktree and retires `fz-NN`. One PR ships the whole branch at the end.

| Packet | Wave | Title | Status | Depends on | Files (disjoint within a wave) |
|---|---|---|---|---|---|
| [FZ-01](packets/FZ-01.md) | 1 | Merge `origin/main` into the campaign branch | Integrated | ops fetch | 12 conflict files (in the campaign worktree itself) |
| [FZ-02](packets/FZ-02.md) | 2 | Classify `sgw_mission_1360_receipt` as persistent | Integrated | FZ-01 | `db/persistent/*`, linter test |
| [FZ-03a](packets/FZ-03a.md) | 2 | Rule 6: base side of the letter turn-in | Integrated | FZ-01 | route, send, missions/mod, cell_handoff |
| [FZ-03b](packets/FZ-03b.md) | 2 | Rule 6: cell side of the letter turn-in | Integrated | FZ-01 | executor/letter_turn_in, letter_turn_in_result, direct_invoke, phase |
| [FZ-03c](packets/FZ-03c.md) | 2 | Rule 6: minigame receipts | Integrated | FZ-01 | session_result, result_dispatch, base and cell minigame, new server/receipt_deposit_tests.rs, one line in server/mod.rs |
| [FZ-04](packets/FZ-04.md) | 2 | OTEL row for `harset.letter_turn_in` | Integrated | FZ-01 | filters.rs, target_scan_tests.rs, target catalog |
| [FZ-05](packets/FZ-05.md) | 2 | Remove committed logs; move the force-added report | Integrated | FZ-01 | root logs, `.gitignore`, one report |
| [FZ-06](packets/FZ-06.md) | 3 | Split `interaction/mod.rs` under the cap | Integrated | wave 2 | interaction/mod.rs plus 2 new test files; one doc path in cell-interactions |
| [FZ-07](packets/FZ-07.md) | 3 | 567 stale text; Castle chain-file guard by glob | Integrated | wave 2 | harset_opcore_chains.sql, mission_567.rs, seed_id_uniqueness.rs, one sentence in docs/content/mission-chains.md |
| [FZ-08](packets/FZ-08.md) | 3 | Minigame victory-drop WARN and receipt tests | Integrated | FZ-03c | session_result.rs, terminal_outcome_tests.rs, receipt_deposit_tests.rs |
| [FZ-09](packets/FZ-09.md) | 3 | Victory-chain trigger guard; fresh managers in direct_invoke tests | Integrated | wave 2 | new guard test, chain_replay_tests/mod.rs, direct_invoke_tests.rs |
| [FZ-10](packets/FZ-10.md) | 3 | Delete-race poll; one token derivation | Integrated | FZ-03a | concurrency test, cell_handoff.rs, base-world-entry Cargo.toml, Cargo.lock, workspace-hack (if hakari changes it) |
| [FZ-11](packets/FZ-11.md) | 4 | Full pre-PR verification (no code) | Done: failed at `db22c4c00` (F22-F29) | waves 1-3 | none; a report in the campaign worktree |
| [FZ-12](packets/FZ-12.md) | 5, after D-FZ1 | Enable mission 567's Castle and Harset legs | Merged (`086656cd8`) | D-FZ1, FZ-07 | 2 seeds, 3 mission_567 test files, mission-chains.md, unified-uat.md; then lab UAT FZ-U1 (ask the owner) |
| [FZ-13](packets/FZ-13.md) | 6 | Merge `origin/main` again; drop the duplicated readme trio | Integrated (merge commit `194043760`, main at `0d71b3b30`) | ops fetch; ops restores 2 Tauri schema files | 7 conflict files, `docs/readme.md` (in the campaign worktree itself) |
| [FZ-14](packets/FZ-14.md) | 7 | Live-DB pins: template 425's radius, NameBook gap bless | Integrated (`856ee1c60`) | FZ-13 | `live_db_aggression.rs`, `namebook_gaps.txt` |
| [FZ-15](packets/FZ-15.md) | 7 | Lab seed-tag test checks coverage, not a count | Integrated (`e699d0092`) | FZ-13 | `crates/lab/src/uat/seed_tags.rs` |
| [FZ-16](packets/FZ-16.md) | 7 | Move OTLP routing to `filters/otel_routing.rs` | Integrated (`3f599b388`) | FZ-13 | `logging/filters.rs`, new `logging/filters/otel_routing.rs` |
| [FZ-17](packets/FZ-17.md) | 8 | Rerun the pre-PR verification (no code) | Done: 11 of 12 blocking rows pass at `d3d5d62e0`; row 9 accepted as environmental | waves 6-7 (and FZ-12 if enabled) | none; a report in the campaign worktree |
| [FZ-18](packets/FZ-18.md) | 9 | Castle chain 1398 grants 1664 when 708 completes | Integrated (tip `a781d827c`) | campaign tip `ff847c6a3` | new `castle_1664_chains.sql`, `db/database.sql`, chain_replay `mod.rs`, new `castle_1664.rs` |
| [FZ-19](packets/FZ-19.md) | 9 | Harset actors: Sgt. Kaerlia (260/360), door control (261/380) | Integrated (tip `a781d827c`) | campaign tip | `entity_templates.sql`, `spawnlist.sql`, spawner_tests `harset/mod.rs`, `world57_placement.rs`, new `mission_1664_actors.rs` (maybe `namebook_gaps.txt`) |
| [FZ-20](packets/FZ-20.md) | 10 | Harset chains 6008-6016 and dsm 120301 | Integrated (tip `a781d827c`) | FZ-18, FZ-19 | new `harset_1664_chains.sql`, `db/database.sql`, `harset_space_chains.sql` (comments), linter `allowlist.rs`, chain_replay `mod.rs`, new `harset_1664.rs` |
| [FZ-21](packets/FZ-21.md) | 11 | 1664 lifecycle replay: credit order, binds, final abandon | Merged (`51782173d`) | FZ-20 | chain_replay `mod.rs`, new `harset_1664_lifecycle.rs` |
| [FZ-22](packets/FZ-22.md) | 11 | 1664 end to end on the whole engine | Merged (`943e45a43`) | FZ-20 | chain_replay `mod.rs`, new `mission_1664_flow.rs` |
| [FZ-23](packets/FZ-23.md) | 11 | 1664 docs: chain reference, UAT row FZ-U2, id ledgers | Merged (`464d3e3d2`) | FZ-20; after FZ-12 if it runs | `mission-chains.md`, `unified-uat.md`, two `id-allocation.md` |
| [FZ-24](packets/FZ-24.md) | 11 | 1664 seed `can_abandon = false` (P1, metadata only) | Not run (O18-02b = no) | FZ-22 | `missions.sql` (one value), `mission_1664_flow.rs` |
| [FZ-25](packets/FZ-25.md) | 11, after FZ-21/22 | Walk-in door credits step 5225 (chain 6017) | Merged (`6b6b54481`) | FZ-20 | `harset_1664_chains.sql`, `harset_1664.rs` |
| [FZ-26](packets/FZ-26.md) | 12 | Design step 5226's debrief (mission-systems-advisor, no code) | Done ([worknote](worknotes/FZ-26-step-5226.md)) | FZ-20 | one worknote; the architect then slices FZ-27 (chains 6018-6019) |
| [FZ-27a](packets/FZ-27a.md) | 12 | 5226 debrief: Marsh/Moh'katan chains 6018-6019, cue restores 6033-6034, dsm 120302-120303 | Merged (`77f4188b7`) | FZ-21, FZ-22 merged; FZ-25 merged or D-PU4 = No | `harset_1664_chains.sql`, `dialogs.sql`, `dialog_screens.sql`, linter `allowlist.rs`, chain_replay `mod.rs`, `harset_1664.rs`, new `harset_1664_debrief.rs` |
| [FZ-27b](packets/FZ-27b.md) | 12 | Praxis offer 6511/6512 stands down at 5226 and returns when 1664 ends | Merged (`77f4188b7`) | FZ-27a | `harset_opcore_chains.sql`, opcore `handoff.rs`, `disjointness.rs`, chain_replay `mod.rs`, new `mission_1664_debrief_flow.rs`, `harset_1664_lifecycle.rs` (doc text) |
| [FZ-27c](packets/FZ-27c.md) | 12 | 5226 docs: chain reference, FZ-U2 row, both id ledgers | Merged (`77f4188b7`) | FZ-23 merged, FZ-27b | `mission-chains.md`, `unified-uat.md`, two `id-allocation.md` |

Waves:

- **Wave 1:** FZ-01 runs alone, because every later packet needs main's files (the PP lint and the #1865 review agents).
- **Wave 2:** six packets on disjoint files. The ratchet test fails until FZ-03a, 03b and 03c are all merged. Each Rule 6 packet checks only that its own files are gone from the failure list.
- **Wave 3:** five packets. FZ-08 edits `session_result.rs` after FZ-03c, and FZ-10 edits `cell_handoff.rs` after FZ-03a, so wave 2 must be merged first.
- **Wave 4:** FZ-11 runs the whole checklist on the integrated branch. A `failed` return goes to the architect, which writes a fix packet; it is not fixed in place.
- **Wave 5:** FZ-12, only if D-FZ1 is "enable". It can run alongside wave 7 (its files are disjoint), and FZ-17 covers its checks. Put the lab UAT FZ-U1 to the owner before anyone uses the lab.
- **Wave 6:** FZ-13 alone. Before it, `ops` commits the triage docs in this folder, fetches `origin/main` and restores `src-tauri/gen/schemas/acl-manifests.json` and `desktop-schema.json`, which FZ-11's row 12 regenerated. Main moves fast: if FZ-13 waits, the conflict set can grow, and its union rule covers that.
- **Wave 7:** FZ-14, FZ-15 and FZ-16 in parallel, on disjoint files, each in its own `fz-NN` worktree from the FZ-13 tip. One review loop covers the wave.
- **Wave 8:** FZ-17, the FZ-11 rerun with four deltas: lab `--no-fail-fast`, `regen.py --check` informational, row 12 skipped, and file checks against `git merge-base HEAD origin/main`. A `failed` return comes back to the architect.
- **Wave 9 (mission 1664, D-PU1 = in this PR):** FZ-18 and FZ-19 in parallel from the campaign tip `ff847c6a3`. They share no file; FZ-18's `database.sql` line goes after the Castle block and FZ-20's after the Harset block. FZ-12 overlaps none of waves 9-10.
- **Wave 10:** FZ-20 alone. It needs FZ-19's spawns (the world-57 tag guard) and FZ-18's `database.sql` edit merged first.
- **Wave 11:** FZ-21, FZ-22 and FZ-23 in parallel. FZ-21 and FZ-22 each add one line to chain_replay `mod.rs` at different spots (a trivial merge). FZ-24 joins only if O18-02b says flip the column; FZ-25 runs after FZ-21 and FZ-22 merge, only if D-PU4 is yes. One review loop covers the wave; add `mission-systems-advisor` as a read-only pass for FZ-20 and FZ-25.
- **Wave 12 (D-PU3 = B, owner 2026-10-19):** FZ-26 is done; FZ-27a, FZ-27b and FZ-27c come from its worknote. They run **in sequence** in one worktree `fz-27` (branch `four-zone-review/FZ-27-1664-debrief`, test DB `sgw_fz_27`), one commit each, each with its own packet review loop. Order: wave 11 merged → FZ-25 (it edits `harset_1664_chains.sql` and `harset_1664.rs` too; skip it if D-PU4 = No) → `ops` creates `fz-27` from that campaign tip → FZ-27a → FZ-27b → FZ-27c (needs FZ-23 merged; `ops` merges the campaign tip into the fz-27 branch first). Then one wave review over the three commits, and `ops` merges the branch into the campaign branch once, `--no-ff`, and retires `fz-27`. No packet of wave 12 runs in parallel with another. If O18-02c reverses (re-offer after abandon), the architect slices a P3 packet on chain 6020; none is written.
- **Lab UAT FZ-U2** (after wave 12, ask the owner first): Kaerlia's name plate, facing and cue; the door control's mesh, position (a placeholder, D-PU6) and use cursor; the travel; relog restores; abandon. Wave 12 adds the debrief to the same row (FZ-27c writes it): Marsh's and Moh'katan's cues, each bark and objective tick in either order, completion on the second, a mid-5226 relog restoring only the unreported cue, the Praxis "?" back after completion or abandon, and an abandon at 5226. Replay tests cannot see any of these.
- Then the campaign-level review loop, the answers to D-FZ3 and D-FZ5, and one PR to main. If main moved again by then, one more merge packet in FZ-13's shape runs before the PR.

FZ-11 triage (2026-10-19): of its seven failures, four are the campaign's and get packets (F24, F25, F26's campaign half, and F23 found on the way); F27 was over the cap on main already, but the campaign added to it; F28 and F29 are main's and are ledger notes only.

## Dispatch note: review loops (main #1865)

Main's #1865 (`037c02b40`, ai-harness-docs W4a, D-AH42) retired the combined review-and-fix agent.

- Review is now read-only `packet-reviewer` loops:
  - one per packet, as soon as its coder returns;
  - one per wave;
  - one for the campaign, before the PR.
  
  The verdicts are `ship`, `fix` or `decision`.
- A `fix` verdict's numbered notes go to a **fresh** `packet-coder`, with "review fix, pass N of 3". There are at most 3 passes per loop.
- Report paths:
  - a pass: `<worktree>\.claude\reports\four-zone-review\FZ-NN-review-<N>.md`;
  - a fix: `FZ-NN-fix-<N>.md`;
  - the wave: `wave<W>-review-<N>.md`.

This worktree's `.claude/skills/campaign-packet/SKILL.md` predates #1865 until FZ-01 merges main. Follow main's copy, `git show origin/main:.claude/skills/campaign-packet/SKILL.md`.

Add `minigame-systems-advisor` as a parallel read-only pass for FZ-08, and `mission-systems-advisor` for FZ-07 and FZ-09.

## Owner decisions

D-FZ1 to D-FZ8 are in [decisions.md](decisions.md). D-FZ1 (enable) is answered and shipped by FZ-12; no packet waits on a decision. D-FZ8 (bless the gaps, drop the tag count) is the architect's, not the owner's. D-FZ3 is answered (squash); D-FZ5 (status docs) must be confirmed before the PR opens.

Mission 1664 (2026-10-19): the owner decided D-PU1 (1664 ships in this PR). All since answered or built (see [decisions.md](decisions.md)); only D-PU6 (the door position, BlockedEvidence until the FZ-U2 lab pin) is open. Architect calls D-FZ9 to D-FZ13 are Info. Waves 9 and 10 wait on none of them: FZ-18 to FZ-23 build option P0 with 5226 open, which every owner answer keeps or extends. PU-05 review findings: 1 and 3 were fixed in the worknote (`b49933a8c`); finding 2 (K1 order) is D-FZ9, applied in FZ-20's rows and pinned by FZ-20 test 1 and FZ-21 test 1.

Later on 2026-10-19 the owner decided **D-PU3 = B** (finish 1664 in this PR) and delegated the residual choices: take the FZ-26 worknote's recommendation where it has one, otherwise the smallest server-authoritative option that needs no client patch. Under that delegation the architect made D-FZ14 (the debrief's binds, the 6511/6512 guard and hand-back, the ids) and D-FZ15 (provisional, order-neutral bark text; the owner or a writer may replace it). D-PU4 (yes, FZ-25 merged), O18-02b (no), O18-02c (not now) and D-PU5 (open) were then decided by the orchestrator; only D-PU6 is open.
