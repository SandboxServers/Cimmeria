---
name: project-lab-spec-vocab-2026-10-11
description: "lab-spec-vocab closed out 2026-10-11 (SV-01..SV-14 merged and released); SV-15 live acceptance paused because a fully loaded workstation starved the lab client. Read before resuming SV-15 or any live lab run during heavy builds."
metadata:
  type: project
---

**State (2026-10-11).** SV-01 to SV-14 merged (#1391 to #1466) and released to the colo the same day; the close-out is SV-16. SV-15, the calibrated first-session acceptance run, is **paused**, not done: nothing was calibrated, and the FS-P3/FS-P4 poses are still the 2026-10-10 hand values. Resume procedure: `docs/analysis/lab-spec-vocab/worknotes/SV-15.md`; next-session brief: `docs/analysis/lab-spec-vocab/handoffs/2026-10-11-sv16-closeout.md`. GD-10 (the first-session golden) waits on SV-15.

**Lab under load.** Three SV-15 attempts failed on the lab, not the product. A heartbeat clause read once 5 s after `lab_client_start` saw `unreachable` (the bridge came up later; the display keep-awake engaged about 2 minutes after launch), fixed by polling 60 s. Then `lab_login` timed out at `list_characters` (a fixed 15 s bridge dispatch) although the login completed: the machine was at 91 % CPU with all four build-lane slots held by other sessions.

**Why:** at full build load the client misses the bridge's fixed timeouts, and each miss looks like a spec or product failure, costing a run and a diagnosis.

**How to apply:** before a live lab packet, check the lab lease is free and the build lane idle; if other sessions are building, wait or ask the user. Run the worktree's `pwsh -NoProfile -File tools/lab/lab.ps1` (`lab` is not on `PATH` in agent shells, and the installed copy can lag `main`). The 15 s dispatch timeout and the late keep-awake are filed as lab infrastructure issue #1483.
