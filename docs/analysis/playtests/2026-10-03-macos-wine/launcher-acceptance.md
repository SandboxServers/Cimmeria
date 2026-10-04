# Launcher acceptance and ownership

> **Type:** Reference
> **Audience:** Launcher contributors and reviewers
> **Companions:** [Requirements](launcher-implementation-plan.md)
> **Status:** MacBook testing stopped; repository checkpoint; no release-readiness claim
> **Last updated:** 2026-10-04

The [implementation plan](launcher-implementation-plan.md) defines the full
scope. This checklist tracks current evidence and remaining acceptance work;
historical ledger entries are not substitutes for validation of the integrated
revision. The root checkout is outside this campaign's write scope.

## Current checkpoint

The [MacBook testing checkpoint](worknotes/macbook-testing-checkpoint.md) records
the current evidence and preserved state. `launcher/uat-integration` through
`d36dbebfa` combines adoption UI, effective settings and opt-in Wine identity.
Combined validation passed 505 Rust tests, 70 frontend tests, seven Wine fixtures
and the documented native-backed JS UATs. The normal signed app is based on
`e79e99b1e`; the subsequent hidden-panel HTML fix was native-verified in a
separate signed UAT app.

Real native Play reached the operator-confirmed SGW login screen, with stable
launcher status/rechecks. Native isolated legacy import passed, but verified-copy
preparation was interrupted: cleanup is offered and has not been requested.
No real adopted-copy prerequisites or Play passed. Actual SGW computer use still
timed out. MacBook testing and rebuilds are stopped at the user's request; retain
the preparation state. The existing Wine identity investigation may finish its
current assignment, with no further tasks; its pending work is not integrated.

The earlier `launcher/combined-validation` checkpoint and draft PR #1190 remain
historical integration evidence. None of these results establishes full launcher
acceptance or advances PR #1164 by itself.

## Windows troubleshooting handoff

Investigate concrete Windows failures locally through separate GitHub issues,
not iterative GitHub-hosted CI diagnosis. The updater ShellExecute test stall is
tracked in [#1194](https://github.com/SandboxServers/Cimmeria/issues/1194), with
native log evidence, local reproduction, bounded acceptance and an unverified
candidate patch. That candidate is not applied to the combined branch. Windows
validation remains open; routine CI is not a substitute for the local handoff.

## Campaign ownership

Original campaign base: `0b10d869c869793ab506dbf9215ddb91714a244b`.
Integration: `prototype/launcher-packaging-proof`, draft PR #1164.
No merge, deployment, release publication or force-push is authorized.

| Owner / branch | Exclusive surface | Next acceptance evidence |
|---|---|---|
| Game Update / `launcher/game-update` | Retained preparation, replacement, recovery and rollback | Native two-release journey integrated; mounted Apply/cleanup UAT passes; real-client/platform validation remains |
| Integration owner | Shared extraction preflight, contracts, registrations, CI, indexes and integrated UAT | Windows cabinet validation; adoption integration; updater Apply/recovery; effective configuration and game Update |
| External Windows observability track | Summary schema, queue/export, ingestion and query fixtures | Discovery integrated; fresh implementation assignment recorded; do not duplicate reserved work |

Repair, Launch, migration-import UI, adoption reference preparation and signed
updater check/download workers have completed their packets. Workers isolate shared-file changes in separate
commits; their worktrees remain intact during review. The current stop state and outstanding investigation are recorded above; older
packet summaries below describe their original evidence boundaries.

### Earlier integration wave (historical packet evidence)

Through `b3ef86658`, Repair, Play, explicit settings import, signed updater
check/download and the verified-copy adoption foundation are integrated.
Combined local engine/shell validation passed 391 tests (22 opt-in/platform
fixtures ignored); frontend tests passed 54 with TypeScript/build. Production
Repair, Play and composed updater persistence UAT have passed within their
documented fixture scopes. These are not gameplay or release evidence.

- Repair review findings are fixed: actual backup status drives cleanup and copy;
  production host handoff/recovery have guards; timeout and memory metadata were
  corrected. See [fix evidence](worknotes/repair-review-fixes.md).
- Play's production Windows locked-owner accessor fix passed native Windows CI
  at `590050084`. Newer Windows runs exposed second-handle reads in test-only
  filesystem snapshots; those fixture corrections require native revalidation.
- Native content Install and prerequisite preparation succeeded through the real
  app. The first recorded Play failed early; a separate graphics diagnostic
  reached the login screen according to the operator and later exited cleanly.
  The current production Play path reached the SGW login screen, visually confirmed
  by the operator; authentication/world entry remain unverified. See [native UAT](worknotes/native-window-uat.md).
- Settings import is integrated. [Verified-copy adoption](worknotes/native-verified-copy-adoption.md)
  has a source-preserving native foundation. Its Settings UI on macOS and the
  effective imported settings for adopted Play are combined on
  `launcher/uat-integration` with fixture evidence and partial native import/preparation UAT; see the
  [checkpoint](worknotes/macbook-testing-checkpoint.md). Completed real-copy
  adoption, prerequisites and adopted Play remain required.
- [Signed updater checks](worknotes/signed-updater.md) and minimum-version gates
  are integrated. Settings can check/download/reverify with native ownership;
  production configuration is disabled. Later fixture Apply/restart/rollback UAT
  passed as recorded above; actual packaged upgrade and download-resume gates
  remain open.
- Windows-built lab MCP, starter and injected DLL artifacts passed their build
  and hash checks. No lab injection or authentication/world-entry UAT is claimed.
- Shared RAR/cabinet name preflight is committed in `198573ce7`. Mac tests and
  engine clippy passed; Windows FDI tests passed in run `37219800831`, while the
  rebuilt-helper Wine validation is pending.
  Observability remains reserved for the external Windows track.
- Retained adoption references are integrated as `211226a8`/`2857705a7`; isolated
  real RAR/CAB helper fixtures passed on the worker's earlier helper. Latest local
  engine suite passes 363 tests (18 ignored) after a narrow lock-release fix.
  Mac CI failed two immediate owner-lock reacquisition checks. A duplicated-handle
  regression demonstrates the corresponding lifetime hazard; the affected guards
  now explicitly unlock at logical-owner drop. Native CI revalidation is pending.

## Requirement and evidence checklist

| Required behavior or gate | Current evidence | Remaining verification / owner |
|---|---|---|
| Single-game dark-only Tauri interface | Native development window rendered; Settings chooser cancellation passed | Keyboard/focus, responsive layout and integrated feature visuals; integration |
| Real Effect orchestration and persistent settings | Existing workflow/services and native journal; prior ledger tests | Integrated native-persistence JS UAT and failed-save/reconnect behavior; integration |
| Installation and prerequisites | Real native UI content installation and compatibility preparation succeeded | Graphics/Play, failures and cancellation; integration |
| Play with lifecycle observation | Native Play starts real SGW; operator confirms login screen; running status/rechecks stable; fixture lifecycle checks pass | Authentication/world entry, actual SGW computer use and current native parity |
| Repair | Integrated with review fixes; production preparation/commit/cleanup JS UAT passed | Native visual and original-client repair UAT |
| Confirmed uninstall | Existing Settings confirmation and native ownership checks | Integrated confirmation/dismiss/removal/recovery UAT; integration |
| Signed GitHub manifest patch notes | Actual Tauri tab rendered seven verified entries; wrong-key rejection observed | Refresh/reconnect after integrated changes; integration |
| Default-off optional summary consent | Separate persisted preference; exporter absent | Consent preservation plus local exporter race/failure evidence; external track and integration |
| Focused observability | Discovery reviewed and integrated; implementation assignment recorded | Full bounded producer-to-local-ingestion/query fixture; external track |
| Migration and identity/consent preservation | Fixture host/JS journeys pass; isolated native import preserves consent false; preparation ends interrupted with cleanup offered | Complete real-copy adoption and resolve interruption/feedback; real prerequisites and adopted Play; adopted Update and Windows adoption |
| Single updater owner and version/asset mapping | Native minimum/check/download and fixture Apply/restart/rollback persistence UAT pass | Real packaged Apply/restart/rollback, resume, production configuration and Windows replacement parity |
| Tests, docs and project memory | Existing baseline artifacts | Per-packet guards, fresh bounded review, matching docs/indexes and findings |
| Windows native parity | Earlier Play checks passed; lab/helper/DLL builds verified; newer test snapshot failures identified | Corrected native tests, FDI preflight, latest helper and updater/adoption parity |
| Actual Mac graphics/login/world | Production native Play starts SGW; operator reports login screen | Actual game computer use, windowed behavior, authentication and world entry remain open |
| Final self-contained packages (LAST) | Deferred until journeys complete | Offline first open, bundled dependencies, clean-machine checks, signing/notarization and per-OS artifacts |

Further MacBook testing is stopped at the current checkpoint. Preserve unrelated
sessions. Keep WireGuard off and use only local telemetry fixtures. Final startup
validation remains last; development-window UAT does not satisfy that gate.

Native window evidence and the resolved macOS Documents permission decision are
recorded in [the UAT worknote](worknotes/native-window-uat.md). The original
installation completed after permission and status reconnection; prerequisite
preparation also succeeded. Later native Play reached the operator-confirmed login screen; world entry remains
unverified. See the current checkpoint above.

## Earlier follow-on integration (historical packet evidence)

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
