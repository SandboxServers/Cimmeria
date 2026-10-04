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

## Combined validation checkpoint

`launcher/combined-validation` combines owner/current-release separation and Update
admission with [updater Apply](worknotes/updater-apply.md) and the
[post-handoff save-failure fix](worknotes/updater-handoff-fix.md). Combined native checks pass 429 tests (25 ignored), frontend tests pass 56,
and both updater native Effect UATs pass. These checks ran separately from the rebuild checkpoint. These changes have not
yet advanced PR #1164's integration branch. Game Update now has native retained execution and mounted confirmation with
real-store Effect UAT for Apply, lost reply, cleanup and reopen. Latest frontend
tests pass 60 and six scoped Update shell tests pass. Actual game Update,
effective settings, observability and real packaged upgrade gates remain required.

## Windows troubleshooting handoff

Investigate concrete Windows failures locally through separate GitHub issues,
not iterative GitHub-hosted CI diagnosis. The updater ShellExecute test stall is
tracked in [#1194](https://github.com/SandboxServers/Cimmeria/issues/1194), with
native log evidence, local reproduction, bounded acceptance and an unverified
candidate patch. That candidate is not applied to the combined branch. Windows
validation remains open; routine CI is not a substitute for the local handoff.

## Current wave

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
commits; their worktrees remain intact during review. The game is closed while
the operator adjusts framerate settings; independent implementation continues.

### Integration wave

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
  The corrected production path still needs a rerun. See [native UAT](worknotes/native-window-uat.md).
- Settings import is integrated. [Verified-copy adoption](worknotes/native-verified-copy-adoption.md)
  has a source-preserving native foundation, but no UI or Play enablement yet.
  Published RAR references, effective configuration and permanent-owner/current-
  release separation remain required.
- [Signed updater checks](worknotes/signed-updater.md) and minimum-version gates
  are integrated. Settings can check/download/reverify with native ownership;
  production configuration is disabled. Apply, restart, rollback and partial
  download resume remain open.
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
| Play with lifecycle observation | Engine/UI and minimum gate integrated; native-persistence Effect UAT and Windows locked-owner regression passed | Corrected production graphics path, real gameplay and current native parity |
| Repair | Integrated with review fixes; production preparation/commit/cleanup JS UAT passed | Native visual and original-client repair UAT |
| Confirmed uninstall | Existing Settings confirmation and native ownership checks | Integrated confirmation/dismiss/removal/recovery UAT; integration |
| Signed GitHub manifest patch notes | Actual Tauri tab rendered seven verified entries; wrong-key rejection observed | Refresh/reconnect after integrated changes; integration |
| Default-off optional summary consent | Separate persisted preference; exporter absent | Consent preservation plus local exporter race/failure evidence; external track and integration |
| Focused observability | Discovery reviewed and integrated; implementation assignment recorded | Full bounded producer-to-local-ingestion/query fixture; external track |
| Migration and identity/consent preservation | Explicit settings import and verified-copy foundation integrated | Published-client adoption, effective settings, UI, permanent owner/current release and game Update |
| Single updater owner and version/asset mapping | Native minimum gates and signed check/download integrated; composed persistence UAT passed | Apply/restart/rollback, resume, production configuration and Windows replacement parity |
| Tests, docs and project memory | Existing baseline artifacts | Per-packet guards, fresh bounded review, matching docs/indexes and findings |
| Windows native parity | Earlier Play checks passed; lab/helper/DLL builds verified; newer test snapshot failures identified | Corrected native tests, FDI preflight, latest helper and updater/adoption parity |
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
