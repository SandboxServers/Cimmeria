---
name: lane-log-dir-pruned-while-waiting
description: A new worktree's first lane jobs fail in 0.3 s with "No such file or directory" on the job log; another lane's prune_logs deleted the empty log dir while the job waited for a slot.
metadata:
  type: project
---

Seen 2026-10-04 (sa01): every `tools/build-lane/lane.sh` call from a fresh
worktree failed at once with `line 316: .../cimmeria-build/logs/<worktree>/<job>.log:
No such file or directory`, then a `lane_summary.py` FileNotFoundError and
`status=failed exit=1 ran=0.3s`. Cargo never ran.

**Why:** `lane.sh` does `mkdir -p "$LOG_DIR"` up front but writes the first
line only after it gets a slot. While it waits, any other session's lane job
finishing runs `prune_logs`, whose `find "$LANE_ROOT/logs" -mindepth 1 -maxdepth 1
-type d -empty -delete` removes the still-empty directory. A worktree with no
earlier log is the only one exposed; with all four slots busy the window is
minutes long.

**How to apply:** pin the directory with a file before the first lane call:
`mkdir -p "$LOCALAPPDATA/cimmeria-build/logs/<worktree>" && touch .../.keep`
(old-file pruning uses `-mmin +days`, so a fresh `.keep` survives). The real
fix is in `lane.sh` (create the dir after the slot is acquired, or exclude it
from the empty-dir sweep); not done in sa01's PR.

Related: B: (Dev Drive) was at 9 GB free the same day, so the lane refused
with exit 28; `mkdir <worktree>/target` moved the build to C: as
[[build-environment]] describes.
