---
name: launcher-state-key-and-egui-wake
description: Applied patches are recorded by state_key (<id>@sgw_game), still true in cimmeria-launcher-core; the egui repaint trap is retired with sgw-launcher (2026-10-11)
metadata:
  type: project
---

The retired egui launcher (deleted 2026-10-11, LX-18) is gone with its repaint trap. One rule survives:

- `launcher-installed.json` records a `root: sgw_game` patch as `<id>@sgw_game` (`PatchEntry::state_key`). Any lookup by bare `p.id` reports it missing forever. Use `InstalledState::has_applied_patch` / `missing_patches` (`crates/launcher-core/src/state.rs`; `state_key` in `crates/launcher-core/src/manifest.rs`), used by the desktop launcher and by in-place adoption of an egui install's file.

INVALIDATED WHEN: PatchEntry::state_key or has_applied_patch is renamed or the installed-state file stops using <id>@root keys
checked: 2026-11-01
