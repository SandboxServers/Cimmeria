---
name: reference_launcher_platform_options
description: Shared native launcher versus SwiftUI research, evidence limits and proposed comparison gates
type: reference
---

# Launcher platform options research — 2026-10-03

The [research note](../../../docs/analysis/playtests/2026-10-03-macos-wine/launcher-platform-options.md)
compares shared egui/wgpu, Tauri and SwiftUI with shared Rust. It recommends an
experiment order, not an accepted architecture: no native Mac target was built
or benchmarked, and the Windows-native build policy still requires an explicit
exception before Mac implementation. CAB FDI, orchestration extraction, Wine
process ownership, app paths and distribution remain frontend-independent work.
PR #343 documents historical small-executable/no-JS-toolchain rationale, not a
current size measurement or a performance comparison. See the research note for
source links and proposed evaluation gates.
