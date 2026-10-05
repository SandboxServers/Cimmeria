---
name: reference_launcher_platform_options
description: Shared native launcher versus SwiftUI research, evidence limits and proposed comparison gates
type: reference
---

# Launcher platform options research — 2026-10-03

The [research note](../../../docs/analysis/playtests/2026-10-03-macos-wine/launcher-platform-options.md)
compares shared egui/wgpu, Tauri and SwiftUI with shared Rust. It recommends an
experiment order, not an accepted architecture. At the initial research stage,
no native Mac target was built or benchmarked. The later scoped authorization
is recorded below; production platform policy remains unchanged. CAB FDI, orchestration extraction, Wine
process ownership, app paths and distribution remain frontend-independent work.
PR #343 documents historical small-executable/no-JS-toolchain rationale, not a
current size measurement or a performance comparison. See the research note for
source links and proposed evaluation gates.

## Scoped packaging experiment authorization

On 2026-10-03 the user authorized the native Mac packaging proof documented in
[its runbook](../../../crates/launcher/prototype-packaging/README.md). This
supersedes the earlier no-build status for that experiment only, not production
platform policy. All compiling Cargo calls remain lane-wrapped. Consult the
runbook evidence ledger rather than inferring offline/visual success from a
build. Windows-native proof requires a Windows host and remains a separate check.
