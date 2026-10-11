---
name: launcher-telemetry-tailer-traps
description: core Tailer traps - refresh() retain drops files of another root, so the egui runner's tail ships nothing once sessions/ exists; tick order across files is HashMap order; opt-out needs a discard counter
metadata:
  type: project
---

`cimmeria_launcher_core::telemetry::tail::Tailer::refresh(root, pred)` ends with
`retain(seen)`, so two refresh calls on overlapping roots each drop the other's
files. The egui runner (`telemetry/runner.rs::tick_once`) refreshes `sessions/`
then `Binaries/` (recursive, debug-log predicate). Whenever `sessions/` exists
(it holds the session marker), each call drops the other's files, so both sets
are re-added at EOF every tick and the egui tail ships no lines at all (read from
the code 2026-10-11, LX-08b, not seen live; egui left as is, deleted in LX-18). The desktop
session loop uses one walk of `Binaries` with a combined predicate.

`Tailer::tick()` iterates a HashMap: lines from different files come in no fixed
order. Tests must assert order per file.

`refresh` starts new files at EOF (baseline); `follow` (added in LX-08b) starts
them at 0. Baseline once before the game spawns, then `follow` each tick.

Opt-out mid-session: `set_game_telemetry` deletes the queue, but a `Consent`
outlives it. `transport/queue.rs` keeps a per-queue-dir discard counter
(bumped first in `discard_queue`); the transport checks it before and after each
write and deletes again if it changed during the write.

**Why:** these cost a debugging round each in LX-08b.
**How to apply:** any change to launcher tailing or consent handling, see
[[desktop-engine-tests-need-cargo-test]].
