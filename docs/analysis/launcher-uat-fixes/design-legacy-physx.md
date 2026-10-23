# Legacy PhysX offer design

> Type: Explanation · Audience: launcher contributors · Date: 2026-10-23
> Companions: [Validation checkpoint](worknotes/legacy-physx-validation.md), [ledger](README.md#legacy-physx-offer-follow-up-2026-10-23), [native contract](../../../crates/launcher/desktop/docs/prerequisites.md#native-windows-physx-system-software-lx-12)

The owner requests an install offer when the game's PhysX runtime is missing.
Native Windows already checks the 32-bit AGEIA registry path for
`v2.6.3/PhysXCore.dll`; a newer PhysX product or the game's own loader DLL does
not satisfy that check. Existing adoption can omit the retained 7.11.13 EXE,
and another product version can block that older MSI with exit 1638.

Add an explicit **Install legacy PhysX…** action beside Play while the required
core is missing. It uses a new path-free `install_legacy` PhysX command, with
immediate download/install feedback and duplicate-click protection. The existing
Settings action continues to use the retained installer. Both show the result;
successful detection hides the offer. Reinspection on launcher reopen derives
availability from native detection, never a saved frontend success flag. Play's
existing admission policy remains independent. macOS keeps its Wine prerequisite
flow; native Windows PhysX controls remain hidden there.

Native code alone selects NVIDIA's fixed HTTPS endpoint:
`https://us.download.nvidia.com/Windows/9.13.0604/PhysX-9.13.0604-SystemSoftware-Legacy.msi`.
The [vendor page](https://www.nvidia.com/en-us/drivers/physx/9_13_0604/physx-9-13-0604-legacy-driver/)
identifies version 9.13.0604 and support for SDK 2.7.0 and earlier. A read-only
fetch verified 20,154,368 bytes, SHA-256
`2b78df4a9004f5c4220d62c363709e5ac5e2e58df9f2284df0254cbfa4970d91`,
and a valid Authenticode signature from NVIDIA Corporation. No vendor binary
is committed, redistributed or executed as part of investigation.

Bound the request by size, a connection deadline and an overall deadline; refuse
redirects and every non-200 response. Verify exact length and hash before any
execution. Reuse the existing create-new scratch MSI writer, readback check,
deny-write/delete handle, elevated system msiexec, interactive vendor license,
restart suppression and result mapping. Download only after an explicit press;
never use paths, URLs or hashes supplied by the webview. Keep the existing
one-installer guard and active-operation admission. No game files, registry-only
workaround, Plan fields, signing secrets or release identity change. Log closed
PhysX start/failure/finish events and monotonic duration, including download.

Validation: unit/negative-log tests for pinned bytes, tampering, response status,
size limits, redirects and download failures; native-host command/admission tests;
DOM tests for missing/installed/unsupported state, immediate feedback, duplicate
presses, result/retry and reopen. Extend the existing PhysX JS logic UAT. Prove
regressions fail with the offer or verification reverted. Run frontend tests/build,
standalone engine/shell tests, formatting and strict clippy through the lane, plus
native Windows/macOS CI. Inspect the actual page visually. Explicit exclusions:
no real MSI/UAC execution, system registry changes, actual game/Wine launch,
live player acceptance or performance claim. Merge via ship-pr; use the owner's
standing instruction to publish a launcher release after the merge.
