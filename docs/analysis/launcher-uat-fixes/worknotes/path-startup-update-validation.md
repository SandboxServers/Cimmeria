# Path display and startup update validation

2026-10-23. One PR, as requested by the owner. Designs: [path display](../design-path-display.md), [startup update](../design-startup-update.md).

## Changes

Existing `displayPath` is now used by Settings, adoption status and backup copy, and Update/Repair/Uninstall confirmations. Canonical native paths remain in preferences/snapshots/identity checks. Startup checks once when safe; the main-page Update and restart action inspects, downloads/verifies when needed, then applies. Both frontend and native Play admission block known available/staged updates, active updates and failed attempts retaining an offer. A failed recheck preserves a known offer. A check failure without a known offer permits Play; failure of the initial inspection holds Play and offers a main-page retry. Nothing is downloaded or applied at startup.

## Completed checks

- Frontend: 264 tests, TypeScript check and UI build passed. Includes drive/UNC display, exact native preference preservation across consent save and remount, backup/status messages, maintenance confirmations, composed startup sequencing, one button/duplicate suppression, staged reuse, failed signature/feed, lost prepare/apply replies and retry after initial inspection failure.
- Engine: 973 passed, 3 ignored; standalone workspace all-target clippy passed. New native tests exercise persisted offers, failed recheck/reopen, ready and failed download, no-offer failure, native Play refusal before journal mutation and successful no-newer clearing.
- Mutation proof: removing display calls fails seven tests; removing frontend Play gate fails four; disabling startup check fails three. Removing native Play admission gate fails both new tests (lane `20261023-063553-147600`); restoring old offer-clearing fails known-offer test (`20261023-063605-355812`). Restored targeted native guards pass (`20261023-063619-280800`). Restoring the unsafe check-retry chain fails two additional tests: a check-only retry that discovers an offer must wait for a separate Update and restart press. All mutated files restored.
- JS REPL-style logic UAT: production view/Effect plus native disk store saves a verbatim Windows path, shows plain path, preserves exact stored path through consent save and process reopen. Native signed loopback feed check starts automatically; verified package survives reopen and tamper is refused. Main-page action drives a real isolated Windows zip swap with relaunch recorded, pending/reconciliation ownership retained and simulated compiled-version acknowledgment. Adoption footer workflow still passes.
- Browser fixture at 1280x720: main-page Update and restart appears above disabled Play, Settings shows `C:\Temp\sgw`, first update click immediately changes to Updating launcher. Unrelated fixture panels are unavailable; this is bounded visual evidence.

## Limits and shipping

No production release, production keys, actual running-launcher replacement/restart, physical folder picker, live game, Wine, power loss or live SigNoz end-to-end check. Standalone library nextest passed 991 tests / 3 skipped; whole-workspace nextest cannot enumerate the pre-existing harness=false helper_supervisor_process executable (exit 104), which passes under cargo test. macOS and final native shell/build/merge checks remain pending at this checkpoint. PDU-U1..U5 packaged/player acceptance remains pending. No owner decision is needed for implementation; a real release still requires owner GO.

Doc-map walk: desktop launcher workflows/IPC/packaging row (UI behavior and updater contract; no IPC change), player guide and self-update reference, UAT rows and campaign scope/resume, project memory and indexes. No schema, Plan, release workflow, key custody, server protocol, game content or generated status blocks changed.
