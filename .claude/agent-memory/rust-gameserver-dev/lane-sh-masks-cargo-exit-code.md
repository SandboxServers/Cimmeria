---
name: lane-sh-masks-cargo-exit-code
description: The shared build lane wrapper exits 0 even when cargo failed; read the "[lane] released (exit N)" line, never the tool's exit status
metadata:
  type: feedback
---

`lane.sh` (the two-slot build semaphore under
`%TEMP%/cimmeria-castle/`) prints `[lane] released (exit N)` with cargo's
real status and then **exits 0 itself**. A `run_in_background` Bash call
therefore reports "completed (exit code 0)" for a build that failed with
nine compile errors.

**Why:** cost a full round of false confidence on the Harset H30/H31
packet (2026-09-19) — the session-1 test files had never compiled, the
lane said exit 0, and the failure was only caught by eyeballing the tail
of the captured output.

**How to apply:** never trust the exit status of anything run through
`lane.sh` or `live-db-test.sh`. Always redirect to a file and assert on
the content:

```bash
lane.sh cargo check -p cimmeria-services --tests > /tmp/out.txt 2>&1
grep -cE "^error" /tmp/out.txt     # must be 0
tail -3 /tmp/out.txt               # must end "[lane] released (exit 0)"
```

Two more traps in the same family:

- The Bash tool keeps only the **tail** of a long capture, so a `grep -A`
  for errors on the tool's own output can miss the first eight of nine.
  Redirect to a file and grep the file.
- `live-db-test.sh` takes **positional nextest substrings** and accepts
  several at once (`live-db-test.sh mission_1360 mission_1361 ...`), which
  folds a multi-filter verification into ONE lane hold instead of four.
  See [[tooling-filter-and-path-traps]].

Related: [[db-test-revert-verification]], [[local-postgres-port]].
