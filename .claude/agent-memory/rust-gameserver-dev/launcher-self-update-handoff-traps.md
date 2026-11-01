---
name: launcher-self-update-handoff-traps
description: Retired egui launcher handoff (deleted 2026-10-11, LX-18); only the Windows lazy-unlock rule survives, and the desktop launcher's handoff is the zip swap in storage/updater
metadata:
  type: reference
---

The egui launcher's self-update handoff is gone (deleted 2026-10-11, LX-18); its viewport-close and `instance_lock` bullets were removed with it.

**Method that still applies.** A path that must exit and free a lock cannot rely on a UI close request alone; unlock a Windows file lock explicitly before closing the handle, because Windows frees a closed handle's locks lazily. The desktop launcher's update handoff is the zip swap in `crates/launcher/desktop/engine/src/storage/updater/` (contract: `crates/launcher/desktop/docs/updater.md`, `--launcher-update-restart <old pid>`).

INVALIDATED WHEN: the desktop launcher moves its handoff off the zip swap in storage/updater
checked: 2026-11-01
