---
name: unpaired-id-sweep-workflow-traps
description: Working an NT-2x Rule 6 sweep - getting per-site unpaired lists, the lane's quiet-mode log-dir race, and Python edits that mangle Rust string continuations
metadata:
  type: reference
---

- **No per-site list.** `unpaired_id_report` prints only per-key, per-crate and per-file counts. To see each field's `file:line`, add a temporary env-gated dump to `unpaired_id_report` in `crates/server/src/logging/unpaired_id_tests/mod.rs`. It iterates `scan.unpaired` and writes each `Site { at, key }` to a file. `git checkout` the patch before you bless or commit.
- **Bless only lowers the total.** A file that drops out entirely is just removed from the baseline. Any crate you touch shows up in the scan, even one outside your packet; the lane runs `-p cimmeria-server` (about 150 s cold).
- **Lane quiet mode can fail on its own.** Another worktree's `prune_logs` can delete the empty `logs/<worktree>` dir between mkdir and the job. You then get `status=failed`, `lane_summary.py` FileNotFoundError, and a sub-second run. Use `LANE_VERBOSE=1` and redirect to a scratch file.
- **The lane refuses below 10 GB free on B:** (exit 28). With 20+ sweep workers this happens often. Wait for space in a background loop. Never run `rm-worktree.sh --merged` or `sweep.ps1` yourself: they reach other workers' dirs.
- **Python bash-heredoc edits** of Rust lines that end in a `\` string continuation lose the backslash-newline. Use the Edit tool for those lines.

Related: [[lane-sh-masks-cargo-exit-code]], [[tooling-filter-and-path-traps]], [[base-side-log-naming-and-lock-traps]].
