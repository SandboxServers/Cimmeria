---
name: reference-cell-teardown-skips-session-state
description: SpaceManager::destroy_entity does NOT clean cross-entity session state (trade); only the two lifecycle arms do, so every other teardown path strands the counterparty
metadata:
  type: reference
---

`SpaceManager::destroy_entity` (crates/services/src/cell/space_manager/entities.rs)
removes the entity from its space and grid and forgets its movement-validator
clock. It does **not** touch state that lives on *another* entity and points
back at this one.

Trade is the known instance: `CellEntity.trade_partner_entity_id`. Only
`handle_destroy_entity` and `handle_disconnect_entity`
(cell/service/base_messages/lifecycle.rs) call
`cell_methods::player::trade::cancel_trade_on_disconnect` before tearing down.
Every other destructive path — `cell/gate_travel.rs::handle_dial_gate`,
`cell/space_transfer` (P45), and by inspection the other GateTravel producers —
calls `destroy_entity` directly and skips it.

**Why it matters.** The helper early-returns `None` once the entity is gone
(`trade/state.rs`, first line is `get_entity(entity_id)?`), so it cannot be
called after the fact to recover. The surviving partner never receives
`onTradeResults(Cancelled)`, `clear_trade_state` never runs, and their
`trade_partner_entity_id` dangles at a freed id — which combines badly with
[[exploit-entity-id-recycling]].

**Review rule.** Any new call to `space_mgr.destroy_entity` on a *player*
entity is suspect. Ask: what other entity holds a reference back to this one?
Demand the same cleanup the two lifecycle arms do, placed after the point of no
return (so a failed enqueue still leaves origin state untouched).

Reachability matters for severity: self-initiated travel is client-gated, but a
GM `.summon`/`.goto` aims the same teardown at an uninvolved third party.

Regression-guard precedent: `cell/service/base_messages/tests/trade_disconnect.rs`.
