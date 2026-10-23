---
name: Launcher main page and executable selection
description: Main-page preferences, setup visibility and SGW.exe adoption picker.
type: reference
---

2026-10-23: the Play page owns the three existing preference controls and the
native client-changes summary. There is no Patch Notes tab or frontend fetch.
The whole page scrolls; diagnostics status/details and the full changes expand.
Saving game diagnostics opens its detailed feedback on the first press.

`gameSetupComplete(LaunchStatus)` controls setup visibility only. It consumes the
native setup_complete flag, separate from the temporary admission installation_id.
Native code checks valid installed content/resources/runtime and operation state. The updater
keeps its separate Play gate. A known running launch does not resurrect setup;
unknown/reconciliation state retains inspection. Settings also has its own
read-only Check Play status action and adjacent result. Do not infer readiness from a
folder name. Successful adoption is not an unfinished compatibility operation.

The native picker targets SGW.exe. `adoption::resolve_executable` verifies its
name/type, rejects linked layout components and resolves Working/Binaries through
the existing folder resolver before catalog access. Pass the explicit root to
that resolver: a nested game-shaped tree beneath Binaries must not retarget
the executable selection. Selection neither runs the
executable nor commits adoption. The existing review and root safety checks stay.
The IPC name `choose_existing_game_folder` remains compatible; no Plan fields.

Cimmeria Launcher is the product/window title. Mac packaging must use the same
productName for its .app. Existing bundle identifiers, game paths, Wine bundle
path and public zip filenames keep compatibility identities; their display
labels can change. Direct-folder adoption follows current patch preferences;
only legacy-import-bound adoption retains the reviewed binding until Repair.

Validation and limits: see the main-layout worknote under the launcher UAT fixes.
