# Main-page layout validation

> 2026-10-23. Design: [main layout](../design-main-layout.md). Direct follow-up,
> one PR; packaged acceptance rows ML-U1..ML-U6 remain pending.

The main page contains Play, the existing patch/launcher-diagnostics/game-
diagnostics controls and the native client-changes summary. Details expand in
place. Native launch inspection hides setup controls for ready content, including
while a separate launcher update gates Play. Successful saved adoption no longer
shows the installer's false compatibility warning. Cimmeria branding includes
the native window, about line, dialogs and Mac product/bundle display labels;
compatibility paths and published zip names remain stable.

The adoption dialog selects SGW.exe. The native resolver rejects other names,
directories, missing files, flat layouts and linked layout components, then maps
Working/Binaries to the selected installation root. The existing review still
precedes changes; no Plan or IPC fields were added.

## Automated checks

- Frontend: 269 tests pass, TypeScript/build pass. Removed the two retired
  Patch Notes UI tests; added readiness/reopen, native-picker binding and branding
  coverage, and extended placement/first-press/composition tests.
- Windows engine: 977 pass, 3 ignored in the library tier. Windows shell:
  153 pass, 8 ignored. Standalone workspace all-target clippy and native executable
  build pass. Focused executable-selection tests pass after mutation restoration.
- Launcher workflow policy: 5 Python tests pass. No release was cut for this PR.
- Nine deliberate frontend mutations each produced an assertion failure:
  restored tab, ready recheck visibility, false adoption message, Settings-only
  preferences, old product name, folder picker, hidden first-press detail,
  idle archive offers after setup and the Settings inspection binding.
  Two failing view runs retained handles and were stopped after their assertions;
  the restored full suite then passed cleanly.
- Five native mutations fail their intended guards: returning
  Binaries instead of the install root, and accepting another executable name.
  The additional native mutations restore nested-layout retargeting, couple setup
  to the update gate and remove the
  path-free picker-refusal event. All were restored; focused/full shell tests and
  clippy pass afterward.

The broad standalone nextest command cannot enumerate its pre-existing
`harness=false` helper-supervisor integration binary. It fails test listing, not
a test assertion. The engine library nextest and shell libtest tiers above use
their supported runners. This is not reported as a full integration-suite pass.

## JS REPL and visual checks

A persistent Node REPL mounted production `mountDesktop` and Effect workflows.
Launcher-summary and client-patch writes used the real state_bridge store;
game-diagnostics writes used the production native host's ignored UAT bridge.
All three showed first-press feedback and saved correctly. Store reopen/native
process restart and UI remount retained choices. A final compact-card pass
confirmed collapsed idle status, expanded saving feedback and native opt-out.
The first main-page REPL used controlled readiness/installed/client-change
responses. After review, native Play UAT also drove production launch status:
setup_complete remains true while admission installation_id is withheld for a
required update or running launch; unknown/reconciliation makes setup incomplete.
Settings inspection shows immediate feedback and sends one read. These use inert
fixture content and observations, not an installed game or an actual adoption.

The browser preview used the production built HTML/CSS/app with isolated native
fixture stores. At 620x700, Play, all three preference cards and the change
summary are visible together. At 480x560, scrolling reaches every preference and
expanded change list with no document-width overflow. Expand/collapse and the
native-derived ready view were checked. The preview tab and server are temporary.

Not covered locally: physical Windows/macOS picker interaction, packaged Tauri
layout/focus, real signed updater replacement/restart, game launch/login, Wine,
SigNoz reception, or macOS-only link/application-identity tests. Native Mac and
packaging CI remain the next gates; none of these fixture checks attest player
acceptance. No laptop/lab lease, client start or real release occurred.

## Independent review fixes

Review 1 found that installation_id is an admission credential and disappears
while Play is gated. Native LaunchStatus now exposes setup_complete separately,
using valid content, verified launch resources, runtime eligibility and known
operation state. Play admission still uses installation_id and its existing
gates. Settings now has its own read-only Check Play status action/result.
Executable-picker failures also emit the existing path-free refused step before
returning; cancellation remains a separate event. Tests cover all three findings. Review 2 confirmed those fixes and found a
nested game-shaped layout below Binaries could retarget the executable pick.
The resolver now passes the root derived from the selected executable’s parents;
a regression checks both outer and nested executable selections.

## Documentation duty

Walked the desktop-launcher workflows/IPC/packaging, tester behavior/UAT, and
campaign scope/decision rows of docs/agents/doc-update-map.md. Updated desktop
README/contracts, player/Mac guides, implementation ledger and mirrored ML rows.
Project memory records the readiness and picker seams. Existing indexes suffice;
generated blocks, shared project status and gap-analysis files were not edited.
