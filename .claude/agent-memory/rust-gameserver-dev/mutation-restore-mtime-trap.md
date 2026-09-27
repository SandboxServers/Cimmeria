---
name: mutation-restore-mtime-trap
description: Restoring a file from a pre-mutation backup copy leaves an mtime older than the mutated build, so cargo keeps the mutated binary and the next "clean" run fails or passes for the wrong reason
metadata:
  type: feedback
---

When proving a regression guard by mutate → test → restore, restore with a fresh mtime (`os.utime(path, None)` / `touch`). A `shutil.copy` backup gets its mtime at copy time, which is *before* the mutated write; moving it back gives the source an mtime older than the artifact cargo built from the mutation, so cargo skips the rebuild. Seen 2026-09-27 on SS-D1 (PR #888): the full nextest run after the proof failed two guards whose source was correct.

**Why:** cargo freshness is mtime-based, not content-hash-based, for workspace crates.

**How to apply:** after any scripted mutate/restore loop, touch every restored file before the final verification run; a mutated run itself is always fresh because the mutation write bumps the mtime. Related: [[lane-sh-masks-cargo-exit-code]].
