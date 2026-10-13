---
name: reference_lab_typed_prompt_reader_2026_10_13
description: Typed client prompt reader structure and button handling
type: reference
---

The typed `client_window_read` prompt reader walks `Prompt1_PromptWin` through `Prompt5_PromptWin`, matching `ui_state_chunk` in `crates/lab/src/supervisor/flows/ui_state.rs`. For visible prompt instances, read title from the root text and body from `PromptN_Message`; enumerate only visible `PromptN_Button1`, `PromptN_Button2`, and `PromptN_SingleButton` controls and include enabled state. The live hub pass confirmed `Prompt1_SingleButton` is the visible OK button while `Prompt1_Button1` is hidden ([hub pass findings](../../../docs/analysis/hub-pass-uat/README.md)).