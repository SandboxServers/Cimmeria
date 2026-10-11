---
name: lab-mcp-feature-gated-tool-router
description: How to add a cargo-feature-gated group of lab-mcp tools without growing tools/mod.rs - a second rmcp tool_router block in the submodule
metadata:
  type: reference
---

rmcp 3.5's `#[tool_router(router = chaos_router, vis = "pub(super)")]` on a
second `impl LabTools` block in a child module (`tools/chaos.rs`) generates
`LabTools::chaos_router()`; `LabTools::new` adds it under the feature with
`let tool_router = tool_router + Self::chaos_router();` (ToolRouter implements
`Add`). The child module reaches the parent's private `state` field and
`ok`/`err` helpers. Tests check listing with `tools.tool_router.has_route(name)`
against `cfg!(feature = ...)`; declare the test module from `mod.rs` so the
no-feature test still compiles. Tool methods must be `pub(super)` for a
sibling test module to call them. Used by CH-05 (lab-chaos).

Related: [[lab-daemon-lease-gate]] (the lab daemon side, a different tool set).
