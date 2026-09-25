# Ability Trees: Session Resume

> Type: how-to. Audience: the next coordinator session.
> Updated: 2026-09-25. Companions: [launch prompt and decisions](../README.md), [work packets](../work-packets.md), [audit](../audit.md).

## State at pause (2026-09-25)

The owner relayed a usage warning: 6% of the weekly limit was left. The coordinator stopped fan-out after Wave 0 and told every worker to finish with minimal test runs.

| Packet | Branch | State at pause |
|---|---|---|
| Plan | `docs/ability-trees-campaign` | PR #805 open, documentation only. Merge when green. |
| AT-01 | `trees/at01-tree-catalog` | Worker told to wrap up. Look for its PR, or for `handoffs/at01.md` on its branch. |
| AT-E1 | `trees/ate1-trainer-ui-evidence` | Worker told to wrap up. Unanswered questions are marked UNRESOLVED. |
| AT-05a | `trees/at05-seed-import` (based on #805) | Draft PR expected. Rebase onto `main` after #805 and AT-01 merge, and it becomes AT-05b. |
| AT-07 | `trees/at07-level-cap-50` | Worker told to wrap up. Merge after AT-01, because both edit `db/sgw/Players/Tables/sgw_player.sql`. |

Wave 1 (AT-02, AT-03, AT-04, AT-05b) and Wave 2 (AT-08, AT-09) have **not** been dispatched.

## Next actions

1. Run `gh pr list --search "head:trees/"` and check #805. For each worker branch without a PR, read `docs/analysis/ability-trees/handoffs/<packet>.md` on that branch.
2. Merge #805, then AT-01, once CI is green. Do a trial merge and test run if a PR's CI predates `main`.
3. Brief the Wave 1 workers with the module paths and gate plug-in pattern that the AT-01 PR body reports. Update the status lines in `work-packets.md` as packets land.
4. Merge AT-07 after AT-01, and rebase AT-05a into AT-05b.
5. Worktrees are under `.claude/worktrees/`. Remove each `external` junction with `cmd /c rmdir external` before deleting its worktree.
