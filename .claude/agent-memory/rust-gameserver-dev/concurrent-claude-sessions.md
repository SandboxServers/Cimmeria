---
name: concurrent-claude-sessions
description: When other Claude sessions are running on the same repo with auto-branch-switching, use a dedicated git worktree to isolate work
metadata:
  type: feedback
---

When multiple Claude sessions run concurrently on the same Cimmeria checkout, branch switches and unstaged changes from one session bleed into another. Specifically: a session can find its working tree rewritten mid-flight (branch checkout, files restored from another branch, staged deletions appearing) because git share a single working tree per checkout.

**Why:** Discovered while implementing #65 — the working tree kept reverting my edits mid-session because another Claude session on `implement-trainer-consolidation-55` was actively checking out branches. The reflog confirmed automated checkouts (`HEAD@{0}: checkout: moving from implement-speaker-flags-65 to implement-trainer-consolidation-55`). Cost ~30 minutes of repeated re-edits before diagnosis.

**How to apply:** Before starting any non-trivial implementation, check `Get-Process -Name claude` for other running sessions, and `git worktree list` for active worktrees. If concurrent activity is suspected, create a dedicated worktree under `.claude/worktrees/<task-slug>/` and work there:

```powershell
git worktree add C:/Users/Steve/source/projects/Cimmeria/.claude/worktrees/<slug> <branch-name>
# external/ is git-ignored — junction it into the worktree:
New-Item -ItemType Junction -Path "<worktree>/external" -Target "C:/Users/Steve/source/projects/Cimmeria/external"
```

Then `cd` into the worktree for all bash/cargo commands. The worktree has independent HEAD + index + working tree, immune to checkouts in the main checkout.

Cleanup when the PR merges: `git worktree remove <path>`. The worktree pattern is already used heavily in this repo (see `git worktree list` — 30+ active worktrees for parallel feature branches).
