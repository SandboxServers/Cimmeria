---
name: gm-kill-target-sentinel
description: Stock /gmkilltarget numeric arguments reach cell method 190 as INT64 -1; selected-target resolution and UAT location
metadata:
  type: project
---

Issue #1727's 2026-10-14 lab trace found that the stock
`/gmkilltarget <entityId>` command sends `gmKillTarget(INT64 TargetId)` with
`TargetId = -1`, even for a numeric id. The wire index and shape are recorded
in `docs/protocol/cell-method-dispatch-table.md`; the full extended-method
message is pinned by `crates/wireclient/src/session.rs`.

The cell handler at `crates/cell-console/src/cell/console/gm/world.rs` resolves
only `-1` to the caller's `current_target_id`, which method 0 `setTargetID`
updates. Other valid INT64 ids remain explicit. The regression tests in
`crates/cell-console/src/cell/console/gm/tests/world.rs` pin that precedence
and the rejection guards. Unlike combat damage, an out-of-band GM kill arrives
with positive Health: the handler sends a zero-Health `onStatUpdate` before the
canonical dead-state burst. UAT scripts using the stock slash command must
select the NPC before invoking it, even when the command text names an id.
