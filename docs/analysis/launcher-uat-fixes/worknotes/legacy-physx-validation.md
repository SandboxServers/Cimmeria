# Legacy PhysX offer validation

> **Type:** Reference · **Audience:** Launcher contributors and reviewers · **Updated:** 2026-10-23
> **Companions:** [Design](../design-legacy-physx.md), [ledger and acceptance checklist](../README.md#legacy-physx-offer-follow-up-2026-10-23), [native contract](../../../../crates/launcher/desktop/docs/prerequisites.md#native-windows-physx-system-software-lx-12), [player steps](../../../client/launcher-guide.md#install-physx)

## Checkpoint scope

This checkpoint records completed checks for the PhysX implementation. Local
workspace checks and fresh read-only review passed before the later
[catalog-key extension](../design-development-catalog-key.md), which needs its
own tests and final review. Native/package CI,
merge and release remain pending at this checkpoint. Packaged acceptance LP-U1..U4 remains
open. The change adds an explicit Windows main-page install offer when native
detection cannot find the exact legacy 2.6.3 core, including adopted installs
without the original retained installer. macOS retains its Wine setup.

## Frontend logic and appearance

The frontend suite passed 273 tests and the production build passed.
`uat:physx` passed as a standalone script; its bundled script also ran in a
persistent JS REPL. Both use controlled native IPC replies while exercising the
real view logic. They cover missing retained installers, a newer PhysX product
without the required core, immediate first-press feedback, duplicate refusal,
download failure, UAC cancellation and retry, successful detection, reopen and
absence of automatic installation. They do not establish real native IPC or
system persistence through a packaged application.

A browser review rendered the actual UI HTML/styles and real `mountPhysx`
against mocked IPC at 1280 × 720. The missing-core offer and cancellation state
fit the page. This is fixture visual verification, not a packaged Tauri window
or operating-system permission prompt.

A restored frontend mutation changed the legacy offer to
`legacy.disabled = busy || !installer_available`. The missing-core regression
failed once as expected, demonstrating that the test catches the original
retained-installer dependency. The correct implementation was restored.

Two restored native mutations bypassed the hash check and removed the streaming
size limit. The forged same-size bundle and close-delimited oversized-body tests
both failed as expected; after restoration the full workspace passed. A fresh
read-only reviewer returned ship after correcting download-specific error copy,
adding stream-limit coverage and making native policy constants private.

## Vendor package and guarded native path

A read-only download of the official NVIDIA 9.13.0604 legacy MSI verified:

- Length: 20,154,368 bytes.
- SHA-256: `2b78df4a9004f5c4220d62c363709e5ac5e2e58df9f2284df0254cbfa4970d91`.
- Authenticode: Valid, NVIDIA Corporation.

Read-only Windows Installer COM database inspection found
`CMP_LEGACY_DLLS_263_PhysXCore_dll` in the File table, and an HKLM
`SOFTWARE\AGEIA Technologies` Registry row setting `PhysXCore Path` to
`[APPLICATIONFOLDER]Engine`. These are package contents, not evidence that the
runtime has been installed on a machine.

An authenticated-vendor-bundle test passed with an injected fake elevation
boundary. It exercised verified bytes, the Windows deny-write guard, cleanup
and refusal after tampering. No installer process ran. The full standalone
desktop workspace passed 1,183 tests, zero failed, 13 ignored; strict workspace
all-target clippy and root/desktop formatting passed through the prescribed
checks. The native Windows build passed. The opt-in production NVIDIA download
smoke and authenticated-package fake-elevation test each passed separately.
These totals predate the catalog-key extension.

## Explicit exclusions

No actual UAC prompt, msiexec installation, system registry mutation, game or
Wine launch, lab session, production telemetry receipt or player acceptance was
performed. The download and MSI database inspection were read-only. No
performance improvement or completed packaged acceptance is claimed.

## Final bundled follow-ups before PR

The development-key diagnosis and automatic archive-to-Install handoff were
added after the initial PhysX checkpoint. Final local checks passed:

- Standalone desktop workspace: 1,186 passed, zero failed, 13 ignored.
- Frontend: 281 passed; TypeScript checking and production build passed.
- Strict workspace all-target clippy, root/desktop formatting and native Windows
  shell build passed through the prescribed lane/checks.
- Both opt-in PhysX tests passed separately: actual bounded HTTPS download and
  pinned identity verification; authenticated MSI scratch/handle lifecycle with
  fake elevation. Neither test ran an installer.
- PhysX and archive scripts passed. The final archive script also executed in
  the persistent JS REPL, covering native-state verification before one automatic
  Install request without another click. Real view logic used controlled IPC.
- Browser fixture visual review at 1280 × 720 covered the missing-core offer,
  cancellation, actionable development-key copy and automatic installation
  progress after archive selection. This was not a packaged Tauri window.

Six restored mutation proofs failed with assertion/code failures as intended:
retained-installer dependency on the legacy button; bypassed legacy hash pin;
removed streaming size limit; bypassed public-catalog default-test-key refusal;
dropped development-key IPC recognition; and removed archive-to-Install handoff.
After restoration, all final checks above passed. A fresh read-only review of
the combined designs and final implementation returned ship with no findings.

The handoff tests cover main-page and Settings selection, Downloads acceptance,
verification before dispatch, duplicate presses, cancellation with/without an
older selected archive, rejected archives, lost replies, reopen, blocked/playable
states and a late reply after disposal. Native tests pin the response-only
picker cancellation marker and separate key/verification/network errors. No
signature or archive checks are bypassed. The developer error directs users to
the published ZIP or a build configured with the public release verification key.

Native/package CI, merge, retirement and release remain pending at this checkpoint.
The explicit exclusions above still apply; LP/AS packaged acceptance rows remain
pending. No owner decision remains for the implemented scope.
