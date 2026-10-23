---
name: Explicit native Windows legacy PhysX offer
description: Exact core detection, pinned NVIDIA fallback and unchanged Mac flow.
type: reference
---

2026-10-23: [Legacy offer design](../../../docs/analysis/launcher-uat-fixes/design-legacy-physx.md)
and [native contract](../../../crates/launcher/desktop/docs/prerequisites.md#native-windows-physx-system-software-lx-12)
own this reference. Native Windows requires the 32-bit AGEIA registered
`v2.6.3/PhysXCore.dll`; a newer PhysX product or game loader is insufficient.
Adoption can lack the original retained 7.11.13 installer, and another product
can block that older MSI with exit 1638. Main-page Install legacy PhysX… uses
path-free install_legacy, a pinned official NVIDIA 9.13.0604 download and the
existing interactive elevated MSI path. Never accept a webview-selected URL,
path or hash, auto-install on reopen, or persist frontend success as readiness.
Settings retains the original installer; macOS keeps Wine prerequisites.

Automated native/frontend checks and three restored mutation guards passed;
see the validation worknote for exact coverage and pending packaged acceptance. No actual
MSI, UAC, registry change, game/Wine or lab acceptance is claimed. The preceding
phase-timing change #1998 is merged at 27493b05ae28dd8421419f1765b2982be3b3c04f;
its worktree is retired and Windows/Mac 2306 released in run 40014376577.
