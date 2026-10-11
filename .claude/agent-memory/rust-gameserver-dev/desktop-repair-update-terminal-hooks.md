---
name: desktop-repair-update-terminal-hooks
description: Desktop launcher Repair/Update finish_failure runs after EVERY commit (dropped handoff watcher); hooks on run end must check the op was still open
metadata:
  type: project
---

In `crates/launcher/desktop/engine/src/storage/{repair,update}/preparation/mod.rs`,
`Prepared._handoff` is a oneshot sender that is never sent. Dropping `Prepared`
at the end of `commit::start` makes the watcher task call
`finish_failure(ReconciliationRequired)` after every commit, success included.
`mark_uncertain` on an already-ReconciliationRequired op returns `Ok` without a
transition, so a commit that lost completion reaches "ended" twice.

**Why:** LX-08c (2026-10-11) emits `client.launcher.install_result` at run end;
without an "op was open before this call" check it queued two events.

**How to apply:** anything that reports or records a Repair/Update ending from
`finish_failure` must check the operation was neither terminal nor
ReconciliationRequired before the call. Install's equivalent is
`install_worker::publish`, reached once per worker run. `install_all`'s
per-patch report is discarded by default (`let (result, _) =`); LX-08c stashes it
in `DesktopState::stash_install_report`.
