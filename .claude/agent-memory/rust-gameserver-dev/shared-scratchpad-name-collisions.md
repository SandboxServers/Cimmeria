---
name: shared-scratchpad-name-collisions
description: Parallel worker agents of one coordinator share a scratchpad dir; generic script names (mutate.py) get overwritten by a sibling mid-task.
metadata:
  type: feedback
---

Prefix every scratchpad file with the packet id (`abt4_mutate.py`, not `mutate.py`).

**Why:** 2026-10-04, AB-T4: between applying and re-running a revert-mutation script, a sibling worker (AB-L2) wrote its own `mutate.py` into the same session scratchpad. My next `apply` ran its script instead; it failed on argument parsing, so nothing was damaged, but a script with matching arguments would have mutated another worktree's files.

**How to apply:** name scratch scripts and backups `<packet>_<purpose>.py`, and hard-code your own worktree path inside any script that edits files. Check `git status` after any mutate/restore cycle.
