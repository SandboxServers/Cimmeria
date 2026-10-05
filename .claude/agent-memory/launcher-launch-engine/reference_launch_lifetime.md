---
name: Desktop guest lifecycle ownership
description: Retained Windows handles, uncertain injection and graphics resource boundaries
type: reference
---

2026-10-04: `cimmeria-client-launch::process::resume_running` retains the original
process handle and avoids the short-lived guest's OpenProcess race. The desktop
x86 lifecycle helper reuses it. `start32::run` returns after resume and cannot
establish Wine guest lifetime. `SuspendedProcess::terminate` is best effort;
resume errors do not prove guest termination. Desktop injection/resume failures
therefore stay unknown rather than authorizing a fallback launch.

Verified implementation contracts and remaining native/graphics validation gates
are documented in `crates/launcher/desktop/docs/launch.md`. Managed Wine's archive
is separate from D3D9 overlays and optional x87 acceleration. Module-load
prerequisites alone establish neither graphics nor gameplay readiness.
