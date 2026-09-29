---
name: launcher-self-update-handoff-traps
description: sgw-launcher self-update handoff — egui Close waits for a frame, lock lives in instance_lock, relaunch fallback kill, sleeper-child test trick
metadata:
  type: project
---

Self-update handoff facts (2026-09-29, fix after the 676f314 -> 4fcae33 "another instance" box):

- `ctx.send_viewport_cmd(ViewportCommand::Close)` only acts on the next egui frame. A process that must exit (and free `launcher.lock`) cannot rely on it; `self_update::handoff::hand_off` does spawn -> `instance_lock::release()` -> `std::process::exit(0)` from the worker task. The UI Close on `UpdateEvent::Restarting` is a backstop only.
- `launcher.lock` is parked in `crates/launcher/src/instance_lock.rs` (static `Mutex<Option<File>>`), not in `main`'s stack, so a worker thread can release it. Release unlocks explicitly before closing (Windows frees a closed handle's locks lazily).
- Lock is released AFTER a successful spawn, so a failed spawn (rollback) keeps the running launcher's lock.
- Relaunch env: `SGW_LAUNCHER_UPDATED_FROM` (tag) + `SGW_LAUNCHER_UPDATED_FROM_PID` (old pid; absent from 676f314/4fcae33 launchers -> fall back to parent pid via Toolhelp32). Kill only if the image path is `<exe>` or `<exe>.old` (the renamed image can report either).
- The launcher saves nothing on exit (eframe built without `persistence`, no `on_exit`), so a hard `process::exit` loses nothing.
- Testing a real TerminateProcess: spawn `current_exe()` with `--exact <path>::sleeper_child` and an env var that makes that test sleep; see `self_update/old_process.rs` tests.

**Why:** the handoff bug only shows on idle windows; unit tests with injected hooks (`HandoffHooks`) + a real fs4 lock catch it.
**How to apply:** any future "exit after X" path in the launcher must not go through a viewport command alone.
