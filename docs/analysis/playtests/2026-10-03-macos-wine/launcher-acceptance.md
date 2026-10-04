# Launcher acceptance and ownership

> **Type:** Reference
> **Audience:** Launcher contributors and reviewers
> **Companions:** [Requirements](launcher-implementation-plan.md)
> **Status:** Execution in progress; no release-readiness claim
> **Last updated:** 2026-10-04

The [implementation plan](launcher-implementation-plan.md) defines the full
scope. This checklist tracks current evidence and remaining acceptance work;
historical ledger entries are not substitutes for validation of the integrated
revision. The root checkout is outside this campaign's write scope.

## Current wave

Common base: `0b10d869c869793ab506dbf9215ddb91714a244b`.
Integration: `prototype/launcher-packaging-proof`, draft PR #1164.
No merge, deployment, release publication or force-push is authorized.

| Owner / branch | Exclusive surface | Next acceptance evidence |
|---|---|---|
| Repair / `launcher/repair-ui` | Repair host modules; installation workflow/view and tests; app wiring and HTML; shell host/main registration | Settings confirmation through real retained repair; cancellation/recovery; native tests and Effect UAT |
| Launch / `launcher/launch-engine` | New launch engine/helper modules and tests; launch guide | Prepared game start, observed host/guest lifecycle, duplicate/early-exit/unknown handling |
| Migration / `launcher/legacy-migration` | New native migration storage modules and tests | Source-preserving preview/confirmed import; audit in [worknote](worknotes/migration-audit.md) |
| External Windows observability track | Summary schema, queue/export, ingestion and query fixtures | Discovery handoff pending; do not duplicate reserved work |
| Integration owner | Shared contracts, dependency manifests, module registration outside Repair, CI, indexes, ledger; integration and visual UAT | Combined journey and requirement-level evidence |

Workers isolate reserved-file changes in separate commits. Repair releases the
frontend before Play wiring begins. Worker worktrees remain intact during review.

### Integration wave

Repair, Launch and migration engine commits are integrated in draft PR #1164 at
`d73eea0ed96e9dfbc91553101a3923300ab9e87c`. Combined engine/probe validation passed
330 tests with 15 opt-in cases ignored; strict all-target clippy passed on Mac.
The native x86 helper workflow passed as run `37214723035`; the Windows desktop
engine job exposed a macOS-only accessor gate, now corrected pending native rerun.

- `launcher/repair-review-fixes` owns the actual backup-status correction and
  production host preparation/commit/cancellation/recovery guards. Standards
  review found two documented violations (test timeout and memory metadata);
  Spec review found two partial requirements (backup status and native wiring
  coverage). The coordinator fixed memory metadata; remaining fixes are active.
- `launcher/play-integration` owns new Play host/workflow/view modules, app and
  shell registration, and its UI. Repair fixes retain installation-status copy
  ownership, including suppression of misleading Install labels during Launch.
- Launch engine and migration engine workers have finished. Their handoffs are
  `worknotes/launch-engine.md` and `worknotes/migration-engine.md`.
- Migration UI, fresh engine reviews and updater research/implementation remain
  queued. Observability remains reserved for the external Windows track.

## Requirement and evidence checklist

| Required behavior or gate | Current evidence | Remaining verification / owner |
|---|---|---|
| Single-game dark-only Tauri interface | Native development window rendered; Settings chooser cancellation passed | Keyboard/focus, responsive layout and integrated feature visuals; integration |
| Real Effect orchestration and persistent settings | Existing workflow/services and native journal; prior ledger tests | Integrated native-persistence JS UAT and failed-save/reconnect behavior; integration |
| Installation and prerequisites | Existing retained native worker, managed runtime and packaged-helper contracts | Install through native UI into playable installation, failures and cancellation; integration |
| Play with lifecycle observation | Not implemented at common base | Launch worker, followed by frontend integration and real game UAT |
| Repair | Worker implementation and native-persistence Effect UAT complete; locally integrated, review pending | Repair worker, then native visual UAT |
| Confirmed uninstall | Existing Settings confirmation and native ownership checks | Integrated confirmation/dismiss/removal/recovery UAT; integration |
| Signed GitHub manifest patch notes | Actual Tauri tab rendered seven verified entries; wrong-key rejection observed | Refresh/reconnect after integrated changes; integration |
| Default-off optional summary consent | Separate persisted preference; exporter absent | Consent preservation plus local exporter race/failure evidence; external track and integration |
| Focused observability | External discovery reported, no handoff inspected | Full bounded producer-to-local-ingestion/query fixture; external track |
| Migration and identity/consent preservation | Read-only baseline audit complete; implementation/adoption validation pending | Existing-user fixtures and integrated adoption; migration track |
| Single updater owner and version/asset mapping | Read-only baseline audit complete; implementation/adoption validation pending | Failure/rollback and Windows replacement parity; migration track |
| Tests, docs and project memory | Existing baseline artifacts | Per-packet guards, fresh bounded review, matching docs/indexes and findings |
| Windows native parity | Historical checks only | Current native Windows build/tests, helper/DLL and updater evidence |
| Actual Mac graphics/login/world | No evidence from this execution | Rendered login, successful authentication and world entry are separate gates |
| Final self-contained packages (LAST) | Deferred until journeys complete | Offline first open, bundled dependencies, clean-machine checks, signing/notarization and per-OS artifacts |

Computer use for this launcher and game UAT is authorized. Preserve unrelated
sessions. Keep WireGuard off and use only local telemetry fixtures. Final startup
validation remains last; development-window UAT does not satisfy that gate.

Native window evidence and the resolved macOS Documents permission decision are
recorded in [the UAT worknote](worknotes/native-window-uat.md). The original
installation was admitted after permission and the UI reconnected to real
download progress; completion remains unproven.
