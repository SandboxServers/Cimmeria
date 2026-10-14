---
name: reference_uat_capture_safety_2026_10_13
description: UAT capture safety and integer comparison guard locations
type: reference
---

AP-03 review on 2026-10-13 found that `capture = "tool"` and `capture = "server"` could invoke write tools because capture execution skipped the action tier path. Capture reads now check `tier::is_read_only` in `spec_validate.rs`, `runner/checks.rs`, and immediately before invocation in `runner/actions.rs`. New capture tools must be classified correctly by `tier::is_read_only`; `server_console_exec` and `client_lua_eval` are write examples rejected by regression tests in `runner/capture_tests.rs`.

`clause::loose_eq` is used by `Op::Any` and JSON pointer selectors. Adjacent u64 IDs above 2^53 can have the same f64 value, so the integer branch compares exact integer values before its Lua string/number float fallback. The regression is in `clause_tests.rs`.
