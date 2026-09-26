# Ability Trees: Session Resume

> Type: how-to. Audience: the next coordinator session.
> Updated: 2026-09-26. Companions: [launch prompt and decisions](../README.md), [work packets](../work-packets.md), [audit](../audit.md).

## Why the campaign stopped (2026-09-26)

The owner ordered a full stop, relayed by the `cimmeria-e3` session, for a build-system overhaul: a crate split of `cimmeria-services`, a toolchain pin, cargo-hakari, target cleanup and a build-concurrency change. The coordinator did the following:

- stopped all five workers;
- killed the campaign's build-lane jobs, keeping only other sessions' jobs;
- committed each worktree's uncommitted work as a `WIP:` commit and pushed it to that worktree's own branch.

Nothing was merged after the stop.

**The overhaul will probably move files.** If `cimmeria-services` is split into crates, every open branch below must be rebased onto the new layout, and paths such as `crates/services/src/ability_tree/` may change. Rebase first, then build, then run tests.

## State at the stop

`main` @ `f153138b`. Merged this session: #805 (plan), #813 (AT-01), #809 (AT-E1), #812 (AT-07), #818 (ledger and AT-10), #820 (AT-03).

| Packet | Branch @ pushed tip | PR | State |
|---|---|---|---|
| AT-02 | `trees/at02-one-source-of-truth` @ `2927aaaa` | none yet | Done, rebased onto `a2dfa22f` (AT-03). The last commit is a 2-line WIP test tweak in `tree_info.rs`. Tests were **not** re-run after the rebase. |
| AT-04 | `trees/at04-trainer-authority` @ `0d5af789` | none yet | Done, and rebased onto `a2dfa22f` with the AT-03 conflicts resolved. The WIP commit adds the `training_points` fixture fixes to `spend_gates.rs`, `train_spend_tests.rs` and `ability_tree/mod.rs`. Tests were **not** run after the rebase: its `verify.sh` was killed mid-build. |
| AT-05b | `trees/at05-seed-import` @ `5cbcd057` | **#807** (draft, base `main`) | Done, and rebased onto `a2dfa22f` with doc conflicts resolved. The WIP commit adds a new live-DB guard, `ability_tree/tests/seed_reachability_live_db.rs` (173 lines, **never compiled or run**). Clippy after the rebase was killed. |
| AT-10 | `trees/at10-ability-warmup` @ `1a2f924d` | none yet | Code, tests and docs are committed (`b6b1d751`, `3eb16b5d`, `465f6aa4`). In `worknotes/at10.md`, the **Commands run** and **Regression proof** sections are still `RESULTS_PLACEHOLDER` / `PROOF_PLACEHOLDER`, so the final suite run and revert proof are owed. The WIP commit is agent memory only. |
| AT-08 | — | — | Not started. Wave 2, after AT-03 (merged) and AT-04. |
| AT-09 | — | — | Close-out, after everything else. |

AT-03's worktree has been removed. Every other campaign worktree is still under `.claude/worktrees/` (`at02`, `at04`, `at10`, `agent-a9a037326bb90baa4` for AT-05b, and `at-coord`). Each has an `external` junction: delete the junction with `[System.IO.Directory]::Delete(path, $false)` in PowerShell (`cmd /c rmdir` failed from Git Bash) before removing a worktree.

## Next actions, in order

1. After the owner restarts the campaign, read the overhaul's notes (the new crate layout, toolchain and lane rules). Rebase each open branch onto `main`.
2. **AT-02:** run the targeted tests and the full services suite (live DB), then open a PR and merge it when CI is green.
3. **AT-05b:** compile and run `seed_reachability_live_db`, then the full live-DB suite. Paste the validator block from `worknotes/at05b.md` into the #807 description, mark it ready and merge it when CI is green.
4. **AT-04:** run the full suite and clippy on the CI toolchain, open a PR and merge it when CI is green. AT-05b's real seed can change trainer-test outcomes, so re-run AT-04's tests after AT-05b merges if AT-04 lands second.
5. **AT-10:** run the full suite, fill in the two placeholder sections of the worknote (commands with exit codes; revert proof), open a PR and merge it when CI is green. Two owner questions are in its worknote: should a stun interrupt a warmup, and should an interrupt carry a relaunch lockout?
6. **Update the ledger status lines** (`work-packets.md`): AT-02, AT-04, AT-05b and AT-10 are Review/Integrated. Note that AT-05b found that the "existing `handle_train_ability` live-DB fixture" never existed.
7. **Dispatch AT-08 (respec).** The owner decided the server strips refunded abilities from the saved hotbar; it is written into the AT-08 scope. It needs AT-04's pinned-trainer gate.
8. **AT-09 close-out,** then `/release` on the last PR (D-AT05), then the owner's AT-06 UAT (10 steps in `work-packets.md`).

## Findings to carry into the close-out

- **AT-03:** the cell never hydrated a player's `level`, so every player trained as level 1 on `main` before #820. Fixed in #820 with `ProgressionChanged`. It also means other players saw this player as level 1 when first introduced (`request_entity_update.rs:118`).
- **AT-04:** `interact_target_in_range` accepted a target in another space (`get_entity` searches every space). Fixed on the AT-04 branch; rated MEDIUM by server-authority-enforcer.
- **AT-02:** the trainer's `onTrainerOpen` order still comes from `trainer_abilities`, not the catalog. It is harmless because the client joins by id, but D-AT07 ("offer the whole tree") should iterate `catalog.tree()`.
- **AT-10, doc conflicts:**
  - Timer-type numbering in `docs/reverse-engineering/findings/combat-wire-formats.md` (`AbilityWarmup = 2`) disagrees with `enumerations.xml` (`AbilityWarmup = 1`, `AbilityCooldown = 2`). Fix the losing doc.
  - `AF_CHANNEL_ALLOWS_MOVEMENT = 16384` is actually `SpeedPet` in `enumerations.xml:51`.
- **Disk:** C: ran out during this session. The main checkout's `target/` is 141 GB. Two orphan checkouts beside the repo (`Cimmeria-issue-356`, `Cimmeria-504`, about 11 GB, no git data) await an owner decision. `Cimmeria-504/external` is a junction into the main checkout's `external/`.

## AT-01 API (still valid, for briefings)

Module `crates/services/src/ability_tree/` (the path may move with the crate split):

- **Catalog:** `AbilityTreeCatalog` (`load`, `tree(arch)`, `node(arch, id)`). AT-02 adds `shared_catalog(pool)`, a process-wide `OnceCell`.
- **Predicate:** `evaluate_train(&TrainContext) -> Result<TrainPlan, TrainReject>`.
- **Gate order in `gates::NODE_GATES`:** node gates, then spend gates (`gates/spend.rs`, AT-03), then trainer gates (`gates/trainer.rs`, AT-04).
- **Rejection feedback (AT-04):** `onErrorCode` then the `onTrainerOpen` re-send. Codes: archetype 6, level 9, prerequisite 167, trainer gates 43, spend and points 35 (a documented reuse).
- **Player state on the cell:** `CellEntity::level`, plus `CellEntity::tree_progress { trained_abilities, tree_points_spent, training_points }`. Hydrated at `InitPlayerState` and updated by `AbilityGranted` and `ProgressionChanged`.
