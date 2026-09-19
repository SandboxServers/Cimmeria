---
name: stacked-branch-rebase-traps
description: Rebasing a stacked packet branch onto main after its parent squash-merged — the coordinator's base sha is often a rewritten commit and not an ancestor; find the real fork point by message
metadata:
  type: feedback
---

When rebasing a stacked worker branch onto `main` after its parent branch
squash-merged, do **not** trust the base sha you were handed. Verify it first:

```bash
git merge-base --is-ancestor <given-sha> HEAD && echo OK || echo "NOT AN ANCESTOR"
```

**Why:** a parent branch that went through a review round (force-push, amend,
its own rebase) rewrites its commits. The sha recorded when you branched off it
no longer exists in your history — in the Castle CA08/CA09 case the brief said
`0d3c9147`, but CA10 had rewritten that commit into `e47bda1a`, so
`git rebase --onto origin/main 0d3c9147` would have silently done the wrong
thing. `git rebase --onto` does not error on a non-ancestor base; it replays
whatever range it computes, which can drag the parent's pre-squash commits back
onto main.

**How to apply:** find the real fork point by *message*, not by sha —
`git log --oneline origin/main..HEAD` lists the parent's commits underneath
yours; the last one that isn't yours is the correct `--onto` base. Then
`git rebase --onto origin/main <that-sha>`.

Two more things that recur on these rebases:

- **Conflicts in this repo's content work are almost always add/add on one
  line** — the `\ir` list in `db/database.sql` and the `mod <name>;` list in
  `chain_replay_tests/mod.rs`. Resolution is always "keep both sides, ordered".
  Nothing to think about.
- **`Cargo.lock` regenerates a patch-version delta on every cargo invocation**
  (`thiserror 2.0.19` → `2.0.20`) because `main`'s lock is internally
  inconsistent. It comes back dirty after *every* build, so
  `git checkout -- Cargo.lock` immediately before `git add`, not once at the
  start. Never stage it from a packet branch.

**Forward-compat on test match arms:** when a test destructures a
`content_engine::Action` variant to assert the fields a chain owns, use `..`
rather than an exhaustive pattern. Sibling packets add loader-defaulted fields
to those variants (CA04 added `difficulty` to `Action::StartMinigame`), and an
exhaustive arm turns every such addition into a build break in unrelated tests
with no added signal. See [[content-chain-dispatch-traps]] and
[[chain-replay-executor-guards]].
