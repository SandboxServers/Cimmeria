---
name: Launcher main page and executable selection
description: Main-page preferences, setup visibility and SGW.exe adoption picker.
type: reference
---

2026-10-23: the Play page owns the three existing preference controls and the
native client-changes summary. There is no Patch Notes tab or frontend fetch.
The whole page scrolls; diagnostics status/details and the full changes expand.
Saving game diagnostics opens its detailed feedback on the first press.

`gameSetupComplete(LaunchStatus)` controls setup visibility only. It uses native
installation identity/resources/reopen/operation/observation state. The updater
keeps its separate Play gate. A known running launch does not resurrect setup;
unknown/reconciliation state retains inspection. Do not infer readiness from a
folder name. Successful adoption is not an unfinished compatibility operation.

The native picker targets SGW.exe. `adoption::resolve_executable` verifies its
name/type, rejects linked layout components and resolves Working/Binaries through
the existing folder resolver before catalog access. Selection neither runs the
executable nor commits adoption. The existing review and root safety checks stay.
The IPC name `choose_existing_game_folder` remains compatible; no Plan fields.

Cimmeria Launcher is the product/window title. Mac packaging must use the same
productName for its .app. Existing bundle identifiers, game paths, Wine bundle
path and public zip filenames keep compatibility identities; their display
labels can change. Direct-folder adoption follows current patch preferences;
only legacy-import-bound adoption retains the reviewed binding until Repair.

Validation and limits: see the main-layout worknote under the launcher UAT fixes.
