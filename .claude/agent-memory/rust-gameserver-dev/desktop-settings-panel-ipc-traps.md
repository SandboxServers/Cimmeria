---
name: desktop-settings-panel-ipc-traps
description: Desktop launcher Settings panels that call native IPC on mount break view.test.ts call-order pins and sync-throwing UAT fakes; inspect on the first snapshot and wrap invoke
metadata:
  type: project
---

Found 2026-10-11 while adding the LX-12 PhysX panel (`frontend/src/physx-view.ts`).

- `view.test.ts` "approved tabs..." pins the exact IPC call list of `mountLauncher`. Any panel in the `panels` array that invokes on its own changes it; the order is `launcher_command`, then the panel's call (fired when the first snapshot is published), then `fetch_patch_notes`.
- Start a panel's first inspect from `context.onSnapshot` (or `context.snapshot()` if already set), not unconditionally at mount: the "disposing during pending IPC" test counts exactly one call while `launcher_command` never resolves.
- `uat.mjs` and other fakes `throw` synchronously for unknown commands. A panel listener runs inside the Effect snapshot stream, so wrap the call: `Promise.resolve().then(()=>invoke(...))`.
- `npm run uat` needs a native `state_bridge` binary; it is not part of F1.
- `tools/lint-md.ps1` ignores path arguments and lints the whole repo; filter its output with `Select-String`.

Related: [[launcher-preferences-schema-change-fanout]].
