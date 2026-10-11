---
name: launcher-state-key-and-egui-wake
description: Applied patches are recorded by state_key (<id>@sgw_game), still true in cimmeria-launcher-core; the egui repaint trap is retired with sgw-launcher (2026-10-11)
metadata:
  type: project
---

Two sgw-launcher display bugs found in the launcher-20260929-676f314 end-to-end test (fixed 2026-09-29):

- `launcher-installed.json` records a `root: sgw_game` patch as `<id>@sgw_game` (`PatchEntry::state_key`). Any lookup by bare `p.id` reports it missing forever. Use `InstalledState::has_applied_patch(&PatchEntry)` / `missing_patches(&[PatchEntry])`; the key-level lookup is private on purpose.
- egui paints only on input or `request_repaint`. Worker tasks finish on tokio threads, so every event goes through `worker::EventSender`, which calls the app's waker (`ctx.request_repaint()`, thread-safe) after each send. A bare `mpsc::UnboundedSender<Event>` in a new worker path would bring back "Fetching manifest..." until the mouse moves.

**How to apply:** new worker send paths take `&EventSender`; tests build `Worker::new(rt, no_waker())` or a counting waker (see `worker/event_sender.rs` tests).

**2026-10-11 (LX-18):** the egui launcher is retired, so the repaint bullet (`worker::EventSender`, `request_repaint`) no longer applies. The `state_key` rule still does: `InstalledState::has_applied_patch` / `missing_patches` live in `crates/launcher-core/src/state.rs` and `PatchEntry::state_key` in `crates/launcher-core/src/manifest.rs`, used by the desktop launcher and by in-place adoption of an egui install's `launcher-installed.json`.
