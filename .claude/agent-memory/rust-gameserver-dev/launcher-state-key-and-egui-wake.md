---
name: launcher-state-key-and-egui-wake
description: sgw-launcher traps - applied patches are recorded by state_key (<id>@sgw_game), and egui only repaints on input unless the worker wakes it
metadata:
  type: project
---

Two sgw-launcher display bugs found in the launcher-20260929-676f314 end-to-end test (fixed 2026-09-29):

- `launcher-installed.json` records a `root: sgw_game` patch as `<id>@sgw_game` (`PatchEntry::state_key`). Any lookup by bare `p.id` reports it missing forever. Use `InstalledState::has_applied_patch(&PatchEntry)` / `missing_patches(&[PatchEntry])`; the key-level lookup is private on purpose.
- egui paints only on input or `request_repaint`. Worker tasks finish on tokio threads, so every event goes through `worker::EventSender`, which calls the app's waker (`ctx.request_repaint()`, thread-safe) after each send. A bare `mpsc::UnboundedSender<Event>` in a new worker path would bring back "Fetching manifest..." until the mouse moves.

**How to apply:** new worker send paths take `&EventSender`; tests build `Worker::new(rt, no_waker())` or a counting waker (see `worker/event_sender.rs` tests).
