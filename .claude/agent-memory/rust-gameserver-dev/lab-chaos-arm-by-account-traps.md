---
name: lab-chaos-arm-by-account-traps
description: Lab chaos arms (CH-14) promote at playCharacter before RESET_ENTITIES; lab_chaos_callers bans grouped lab_chaos imports even in tests; feature-gated live-DB tests never run in CI
metadata:
  type: project
---

Facts from CH-14 (2026-10-11), arming lab chaos by account for the next world entry:

- The only safe promotion point is `handle_play_character`, after `query_world_entry` and before
  RESET_ENTITIES is sent: the receive loop awaits the handler, and the client's `onClientReady` (where
  `chaos_wants_hold`, `flush_delay` and `flush_chaos::apply` first read chaos, each after
  `clear_if_not(addr, witness_id)`) only follows ENABLE_ENTITIES. Tag the entry with the new entity id
  or `clear_if_not` drops it.
- Reach the registry from base-world-entry through `cell_dispatch::flush_chaos` (allowlisted, and its
  `registry()` honours the `lend_registry` test seam) instead of widening `lab_chaos_callers`.
- `lab_chaos_callers` flags `lab_chaos::{...}` grouped imports in **every** `.rs` under `crates/`,
  test files included; rustfmt does not create them, you do. Use one `use` per item.
- `lab_chaos::status` replies are compared whole in lab-mcp tests (`json!({"sessions": []})`); adding a
  top-level key means editing every such assertion.
- The lab-chaos CI step runs `nextest --profile=ci` without a DB, so a `live_db` test compiled only under
  the feature never runs. Put DB code the feature needs in a `#[cfg(any(test, feature = "lab-chaos"))]`
  module so the ordinary live-DB tier covers it (`lab-mcp/src/tools/account_lookup.rs`).

**Why:** each of these cost a lane round trip or would have left a path untested.
**How to apply:** any new lab-chaos hook in world entry or new lab-chaos tool argument.
Related: [[lab-chaos-flush-seams]], [[lab-chaos-net-shim]].
