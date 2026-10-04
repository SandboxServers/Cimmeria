# MacBook testing checkpoint

> **Type:** Reference
> **Audience:** Launcher contributors and reviewers
> **Last updated:** 2026-10-04
> **Status:** MacBook testing stopped at the user's request; repository checkpoint, not release acceptance
> **Companions:** [Acceptance](../launcher-acceptance.md), [ledger](../launcher-implementation-ledger.md), [combined validation](uat-integration.md)

## Integrated code and build identity

`launcher/uat-integration` contains the reviewed adoption UI, effective imported
settings and opt-in Wine app identity through `d36dbebfa`. The 30 FPS cap is
preserved. The combined fixture checkpoint passed 505 Rust tests, 70 frontend
tests, seven opt-in Wine fixtures, strict Clippy, formatting and native-backed
JS UAT as detailed in the [integration note](uat-integration.md). Those results
are bounded fixture evidence, not a completed real-client adoption journey.

The normal signed development app was built from `e79e99b1e`. Native testing
then found Play and Patch Notes nested under hidden Settings because a section
closing tag was missing. `d36dbebfa` fixes the boundary and adds a hidden-ancestor
regression guard; frontend tests, build and native-backed state UAT passed.
The corrected panels were verified in a separately identified, signed UAT app.
The normal app has not been rebuilt with that fix. Neither build establishes
notarization or release readiness.

## Native evidence at the stop point

| Journey | Observed result | Boundary |
|---|---|---|
| Existing installation: Play | Native Play started real SGW; the operator saw the login screen. Launcher running status and rechecks stayed stable. | No authentication or world entry; this was not an adopted copy. |
| Actual SGW computer use | Application lookup by bundle identifier and full path timed out after unlock and accepted permissions. Wine explorer and SGW registered the same bundle identifier. | Shared identity is an investigation hypothesis, not an established cause or a successful screenshot/input check. |
| Isolated legacy import | Native chooser cancellation left state unchanged. Explicit import then succeeded with the expected confirmation digest; diagnostics consent remained false and no installed owner was created. | Source was a clone of real game content with synthetic legacy identity/configuration and patches off. |
| Verified-copy preparation | Preparation used the production signed catalog and an authenticated cached seed. At the stop inspection, the UI reported preparation interrupted and said it would not repeat automatically. It offered **Remove preparation files**. | No completed copy, installed owner, adopted-copy prerequisite run or adopted Play was verified. Cleanup was not pressed. |
| Interrupted preparation feedback | The generic hero reported compatibility recovery unavailable while the adoption section offered preparation cleanup. | The UAT app had restarted before this inspection; the persisted operation still reported running. The differing feedback and interruption cause remain unresolved; no root cause is claimed. |

Native UAT supplements the JS logic/persistence passes. The latter cover bounded
host/state flows, cancellation and reopen behavior; they do not prove native
folder interaction, real prerequisite installers, graphics, focus, game input
or release packaging. The native pass above only adds its explicitly listed
observations.

## Preserved work and remaining gates

MacBook UAT and rebuilds stop here. Preserve isolated preparation state and
files for later inspection; no cleanup, retry or new test is implied by this
checkpoint. The UAT app is closed and no SGW process remains at the final stop
inspection. This does not establish why preparation was interrupted.

The publication plan keeps `launcher/uat-integration` as the canonical tested
checkpoint branch, without merging into the prototype branch or `main`. Draft
PR #1190 remains on `launcher/combined-validation`; its status should link to this
checkpoint.
The Wine follow-up stays on its own branch for separate review; no further
coordinator build or UAT is included.

The already-running Claude investigation on
`launcher/wine-identity-followup` may finish its existing assignment, but has
no further work assigned. Its pending changes are not part of this integrated
revision or its validation claims; review its final handoff separately.

Remaining acceptance includes:

- A completed real adopted-copy journey through prerequisites and Play, including
  the reviewed patch settings, interruption/recovery and clear UI feedback.
- Actual SGW computer-use screenshot/input verification and separate gameplay,
  focus/windowed and authentication/world-entry checks.
- A Windows-built archive helper containing shared archive preflight, followed
  by validation of that exact packaged helper. The currently reused helper
  predates the preflight change.
- Windows-native parity. The updater ShellExecute stall is handed off in
  [issue #1194](https://github.com/SandboxServers/Cimmeria/issues/1194);
  troubleshoot locally through issues, not iterative hosted CI runs.
- Production updater endpoint/key configuration, published package behavior,
  notarization and clean-machine/offline-first-open checks.
- The reserved external observability implementation and its bounded evidence.

This checkpoint does not close the full launcher goal or claim all four native
acceptance items complete. Earlier fixture and historical worknotes remain
valid within their stated revision and scope.
