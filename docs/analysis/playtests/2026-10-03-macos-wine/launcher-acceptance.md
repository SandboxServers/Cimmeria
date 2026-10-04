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
| External Windows observability track | Summary schema, queue/export, ingestion and query fixtures | Discovery integrated; fresh implementation assignment recorded; do not duplicate reserved work |
| Integration owner | Shared contracts, dependency manifests, module registration outside Repair, CI, indexes, ledger; integration and visual UAT | Combined journey and requirement-level evidence |

Workers isolate reserved-file changes in separate commits. Repair releases the
frontend before Play wiring begins. Worker worktrees remain intact during review.

### Integration wave

Repair fixes and Play controls are integrated through `4513fd8d9`. Combined
shell tests pass 33 (five opt-in fixtures ignored), frontend tests pass 45,
TypeScript/build and strict shell clippy pass. Play persistence UAT and the
production Repair preparation/commit/cleanup JS UAT pass. A stale generated
Repair UAT module was rebuilt before its passing integration run.

- Repair review findings are fixed: actual backup status drives cleanup and copy;
  production host handoff/recovery have guards; timeout and memory metadata were
  corrected. See [fix evidence](worknotes/repair-review-fixes.md).
- Play controls are integrated; [worknote](worknotes/play-integration.md). Fresh
  review found a Windows locked-file second-handle read; `launcher/launch-lock-fix`
  owns the correction and regression guard. Real gameplay remains unproven.
- Native content Install and prerequisite preparation succeeded through the real
  app. The development Play bundle is being staged with pinned Windows-built
  helper/patch resources and pinned D9VK, with its upstream notice retained.
- `launcher/migration-ui` owns explicit preview/confirmed import UI and persistence
  UAT. Migration engine independent review remains queued.
- [Updater research](worknotes/updater-parity-research.md) is complete. Native
  minimum-version gates, signed download and platform apply/recovery remain next
  implementation packets. Legacy updater checksum verification is distinct from
  signed game manifests and Tauri signed updater packages.
- Native x86 run `37214723035` passed; Windows desktop rerun after the accessor
  fix is pending. Observability remains reserved for the external Windows track.

## Requirement and evidence checklist

| Required behavior or gate | Current evidence | Remaining verification / owner |
|---|---|---|
| Single-game dark-only Tauri interface | Native development window rendered; Settings chooser cancellation passed | Keyboard/focus, responsive layout and integrated feature visuals; integration |
| Real Effect orchestration and persistent settings | Existing workflow/services and native journal; prior ledger tests | Integrated native-persistence JS UAT and failed-save/reconnect behavior; integration |
| Installation and prerequisites | Real native UI content installation and compatibility preparation succeeded | Graphics/Play, failures and cancellation; integration |
| Play with lifecycle observation | Engine and UI integrated; native-persistence Effect UAT passed | Windows lock fix, real game UAT and native parity |
| Repair | Integrated with review fixes; production preparation/commit/cleanup JS UAT passed | Native visual and original-client repair UAT |
| Confirmed uninstall | Existing Settings confirmation and native ownership checks | Integrated confirmation/dismiss/removal/recovery UAT; integration |
| Signed GitHub manifest patch notes | Actual Tauri tab rendered seven verified entries; wrong-key rejection observed | Refresh/reconnect after integrated changes; integration |
| Default-off optional summary consent | Separate persisted preference; exporter absent | Consent preservation plus local exporter race/failure evidence; external track and integration |
| Focused observability | Discovery reviewed and integrated; implementation assignment recorded | Full bounded producer-to-local-ingestion/query fixture; external track |
| Migration and identity/consent preservation | Read-only baseline audit complete; implementation/adoption validation pending | Existing-user fixtures and integrated adoption; migration track |
| Single updater owner and version/asset mapping | Read-only baseline audit complete; implementation/adoption validation pending | Failure/rollback and Windows replacement parity; migration track |
| Tests, docs and project memory | Existing baseline artifacts | Per-packet guards, fresh bounded review, matching docs/indexes and findings |
| Windows native parity | Historical checks only | Current native Windows build/tests, helper/DLL and updater evidence |
| Actual Mac graphics/login/world | Controlled driver-selection diagnostic stayed running; operator reported login screen | Production-path rerun, windowed behavior, authentication and world entry remain separate gates |
| Final self-contained packages (LAST) | Deferred until journeys complete | Offline first open, bundled dependencies, clean-machine checks, signing/notarization and per-OS artifacts |

Computer use for this launcher and game UAT is authorized. Preserve unrelated
sessions. Keep WireGuard off and use only local telemetry fixtures. Final startup
validation remains last; development-window UAT does not satisfy that gate.

Native window evidence and the resolved macOS Documents permission decision are
recorded in [the UAT worknote](worknotes/native-window-uat.md). The original
installation completed after permission and status reconnection; prerequisite
preparation also succeeded. Graphics, login and world entry remain unproven.

## Follow-on integration

Migration preview/confirmed import UI and native minimum-version admission gates
are integrated. Combined shell tests pass 37 with six opt-in fixtures ignored;
frontend tests pass 50. Native-persistence migration and Play JS UAT pass.
Independent migration review found one source-error decoding gap; malformed
records now retain actionable corruption feedback, with a regression guard.
Minimum-version rejection has explicit frontend feedback, while updater UI and
installed-state adoption remain outstanding. See the linked worknotes above.

External observability discovery `56b2599aa` is integrated. The
[implementation assignment](worknotes/observability-implementation-assignment.md)
records current shared ownership, endpoint disabled, separate public-mount review
and the requirement to retain bounded phase-duration evidence.
