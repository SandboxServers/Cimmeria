---
name: reference_uat_capture_safety_2026_10_13
description: UAT capture safety and integer comparison guard locations
type: reference
---

AP-03 review on 2026-10-13 found that `capture = "tool"` and `capture = "server"` could invoke write tools because capture execution skipped the action tier path. Capture reads now check `tier::is_read_only` in `spec_validate.rs`, `runner/checks.rs`, and immediately before invocation in `runner/actions.rs`. Server reads are an explicit allowlist: `server_chaos_clear` and `server_packet_tap_stop` change server state and must fail capture validation. New server read tools must be added deliberately. Regression tests in `runner/capture_tests.rs` cover validation, preflight and execution refusal.

Runtime UNVERIFIED reasons from an unavailable server capture or id lookup belong in `RowEvidence.unverified`; keeping them only in the initial grade lets `lab_uat_attest` turn the row PASS after an unrelated SigNoz clause is attested. The same `grade::mark_unverified` rule runs before writing the row and after an attestation.

`clause::loose_eq` is used by `Op::Any` and JSON pointer selectors. Adjacent u64 IDs above 2^53 can have the same f64 value, so the integer branch compares exact integer values before its Lua string/number float fallback. The regression is in `clause_tests.rs`.

`compare_delta` accepts numeric strings up to the i128 bounds; subtract with `checked_sub` so extreme captured values do not panic in debug builds or wrap in release builds.
