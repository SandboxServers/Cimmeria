---
name: launcher-telemetry-tailer-traps
description: core Tailer traps - one refresh per tick (retain drops other roots), NTFS coarse creation times vs a precise baseline, same-tick creation-time ties, shared queue across sessions, undo must not bump the discard counter
metadata:
  type: project
---

`cimmeria_launcher_core::telemetry::tail::Tailer::refresh(root, pred)` ends with
`retain(seen)`, so two refresh calls on overlapping roots each drop the other's
files and re-add them at EOF: the egui runner shipped no lines whenever
`sessions/` existed (fixed in the LX-08b review, 2026-10-11: one walk of
`Binaries` with `runner::is_tailed`, shared with the desktop session loop).

`Tailer::tick()` iterates a HashMap: lines from different files come in no fixed
order. Tests must assert order per file.

`follow(root, pred, baseline)` starts a new file at 0 only if its creation time
is >= baseline. NTFS stamps creation times from the coarse timer tick (~16 ms
behind `SystemTime::now()`), so a file made right after a precise baseline can
read as older: the desktop loop draws the baseline 1 s early, and core tests
sleep 50 ms either side. Two files made in one tick share a creation time; the
rename-inherits-offset match also requires `len >= tracked.pos`. NTFS tunneling
gives a name re-created right after a rename the old creation time.

The desktop queue is shared by every session: a finishing session's shutdown
flush sends everything queued under its own token, so a new Play waits for the
previous loop (`SessionLoop::start`, bounded 35 s, then abort).

Opt-out mid-session: `transport/queue.rs` keeps a per-queue-dir discard counter
(bumped first in `discard_queue`); the transport checks it before and after each
write. The after-write undo deletes via `DiskQueue::discard` without bumping,
or it would stop a session on a newer consent.

**Why:** these cost a debugging or review round each in LX-08b.
**How to apply:** any change to launcher tailing or consent handling, see
[[desktop-engine-tests-need-cargo-test]].
