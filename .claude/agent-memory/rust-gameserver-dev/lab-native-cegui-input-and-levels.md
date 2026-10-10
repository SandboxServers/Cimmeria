---
name: lab-native-cegui-input-and-levels
description: Lab cursor/drag go through native CEGUI injectors (supervisor/cegui_native.rs); fake client in cegui_fake.rs; ui native-level words must be mapped in uat/tier.rs
metadata:
  type: project
---

Since 2026-10-10 the lab moves the UI cursor and drags through the client's own
`CEGUI::System` injectors via `call_native` (`crates/lab/src/supervisor/cegui_native.rs`).
Posted `WM_MOUSEMOVE` and Lua `setPosition` never give CEGUI a `MouseMove`
(docs/reverse-engineering/findings/cegui-mouse-input-feed.md).

- A drag with no `split` makes no HWND call, so it runs against the fake bridge:
  `supervisor/cegui_fake.rs` models memory (System, userdata -> Window*, vtable,
  `d_dropTarget`) and records injector calls in order. Reuse it for any new
  native-CEGUI flow test.
- `d_dropTarget` (+0x270) stays null live; the tool fires
  `notifyDragDropItemDropped` and labels it `native_call` (N3). Cause unconfirmed
  (suspect: no `DragDropTarget` flag up the slot's parent chain).
- UI tools report `native_level` words from `supervisor/ui/mod.rs::NativeLevel`
  (`native_cegui`, `client_ui_lua`, `native_call`). The UAT runner only downgrades
  a tier for words `uat/tier.rs::from_reported` knows; `client_ui_lua` was missing
  before this change, so UI fallbacks silently graded N1. A new word needs a row there.

**Why:** the cursor path was silently dead for hover/drag/minigame input for weeks.
**How to apply:** any new lab input that must reach CEGUI uses `Cegui` helpers; any new
`NativeLevel` variant also gets a `from_reported` mapping and a test. See
[[lab-ui-reader-lua-traps]], [[lab-flow-first-live-run-findings]].
