---
name: revert-verification-checkout-wipes-uncommitted
description: A `git checkout -- crates/` restore step in a revert-verification loop silently deletes ALL uncommitted work in that tree, not just the revert; checkpoint first, and prefer a revert script with an inverse.
metadata:
  type: feedback
---

A revert-verification loop that reverts a hunk, runs the guard, then restores
with `git checkout -- crates/` **destroys every uncommitted change under
`crates/`**, not only the hunk the script just wrote. If the packet's work is
not yet committed, the whole implementation is gone with no reflog entry —
`git checkout --` is a worktree operation and leaves nothing to recover from.

Seen 2026-09-19 on `harset/H52`: H52 was committed, H54 was not. The first
iteration of a three-case `for c in client action gm; do ... git checkout --
crates/; done` loop wiped ~16 files of H54 Rust work. Untracked *new* files
survived (checkout does not remove them), which makes the damage look smaller
than it is at first glance.

**Why:** revert-verification and "undo my edits" are the same command, and the
loop cannot tell them apart.

**How to apply:**

- **Always make a WIP checkpoint commit before the first revert**, per packet,
  not per branch. `git -c commit.gpgsign=false commit -q -m "WIP <packet>
  checkpoint"` then `git reset --soft HEAD~1` at the end to fold it into the
  real commit. This is the cheap insurance and it is what the campaign's
  "WIP commits as checkpoints, never git stash" rule is actually for.
- Scope the restore as narrowly as the revert: `git checkout -- <the one
  file>`, never a directory.
- Better still, give the revert script an inverse (apply/undo the same
  string pair) so no git command is involved in the restore at all.
- A revert of a **seed** row does not need a file edit or a DB reload: apply
  it as an in-place `UPDATE`/`DELETE` against the already-loaded test database
  with `external/postgresql_server/bin/psql.exe`, run the guard, then restore
  with the inverse statement. Both the "drop a condition row" and "reorder
  `content_actions.sort_order`" reverts fit in one lane hold that way.

Related: [[db-test-revert-verification]],
[[revert-verification-loses-uncommitted-fmt]],
[[resuming-a-dead-workers-wip]].
