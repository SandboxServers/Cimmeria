# Ability Trees: Session Resume

> Type: how-to. Audience: the next coordinator session.
> Updated: 2026-09-25. Companions: [launch prompt and decisions](../README.md), [work packets](../work-packets.md), [audit](../audit.md).

## State at pause (2026-09-25)

The owner relayed a usage warning: 6% of the weekly limit was left. The coordinator stopped fan-out after Wave 0 and told every worker to finish with minimal test runs.

| Packet | Branch | State at pause |
|---|---|---|
| Plan | `docs/ability-trees-campaign` | PR #805 open, documentation only. Merge first. |
| AT-01 | `trees/at01-tree-catalog` | **PR #813**, Review. Tests pass. The live-DB round-trip of the new `sgw_player` columns is deferred to AT-03. |
| AT-E1 | `trees/ate1-trainer-ui-evidence` | **PR #809**, Review. Four of the five questions are answered. Question 2 (`onErrorCode` rendering) is UNRESOLVED and needs a live-client trace. |
| AT-05a | `trees/at05-seed-import` | **Draft PR #807**, based on #805. Retarget it to `main` and rebase after #805 and #813 merge; it then becomes AT-05b. The validator passes. |
| AT-07 | `trees/at07-level-cap-50` | **PR #812**, Review. Tests pass. Merge after #813, because both edit `sgw_player.sql`. |

Wave 1 (AT-02, AT-03, AT-04, AT-05b) and Wave 2 (AT-08, AT-09) have **not** been dispatched.

## Next actions

1. Run `gh pr list --search "head:trees/"` and check #805. For each worker branch without a PR, read `docs/analysis/ability-trees/handoffs/<packet>.md` on that branch.
2. Merge #805, then AT-01, once CI is green. Do a trial merge and test run if a PR's CI predates `main`.
3. Brief the Wave 1 workers with the module paths and gate plug-in pattern that the AT-01 PR body reports. Update the status lines in `work-packets.md` as packets land.
4. Merge AT-07 after AT-01, and rebase AT-05a into AT-05b.
5. Worktrees are under `.claude/worktrees/`. Remove each `external` junction with `cmd /c rmdir external` before deleting its worktree.

## AT-01 API, for Wave 1 briefings

Module `crates/services/src/ability_tree/`:

- **`AbilityTreeCatalog`**: `load`, `tree(arch)`, `node(arch, id)`. The cell holds it as `SpaceManager.ability_tree_catalog`.
- **`TreeNode::with_defaults(..)`** builds test fixtures.
- **`evaluate_train(&TrainContext) -> Result<TrainPlan{player_id, archetype_id, ability_id, tree_index, cost, raw_training_cost}, TrainReject>`**.
- **`TrainContext`** already carries `tree_points_spent`.
- **Adding a gate:**
  1. Write `gates/<family>.rs` with `pub(super) fn(&TrainContext, &TreeNode) -> Result<(), TrainReject>`.
  2. Declare it in `gates/mod.rs` and append it to `NODE_GATES`. List order is priority: spend gates first, then trainer gates.
  3. Add the `TrainReject` variants and their `reason()` arms. `train.rs::log_rejection` matches exhaustively, so the compiler forces a log arm.
- **Player spend state:** `CellEntity::tree_progress: TreeProgress { trained_abilities, tree_points_spent }`. It is loaded by the `onClientReady` SELECT (`base/world_entry_appearance/client_ready/mod.rs`) and carried on `BaseToCellMsg::InitPlayerState.tree_progress`.

## Evidence that changes Wave 1 and Wave 2 (from AT-E1 and AT-07)

- **AT-04:** send the points property. It refreshes the counter with the window open or closed. Use the AT-E1 error-code mapping in `worknotes/at-e1.md`, and always re-send `onTrainerOpen` on a rejection: `onErrorCode` rendering is unconfirmed, so the re-send is the reliable feedback.
- **AT-08:** the client has **no action-bar cleanup** after abilities are removed. The server must clear refunded abilities from the saved hotbar, or accept stale buttons until relog. Decide before writing AT-08.
- **AT-07:** the XP bar divides by `onMaxExpUpdate` without a zero check, so never send 0. PR #812 keeps the value non-zero at the cap.
- Audit A-04 was wrong: the table already had a primary key in `_primary_keys.sql`. AT-01 added only the `UNIQUE (archetype, ability_id)`.
