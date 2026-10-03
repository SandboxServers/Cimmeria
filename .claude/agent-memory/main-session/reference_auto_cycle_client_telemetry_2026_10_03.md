---
name: reference-auto-cycle-client-telemetry-2026-10-03
description: "Client Lua and SigNoz evidence for inconsistent auto-attack button behavior"
metadata:
  type: project
---

Reviewed 2026-10-03 UTC. Full behavior and sanitized SigNoz recipe:
`docs/protocol/auto-cycle-button.md` § Observed button failure. Client source
is outside git at `../SGW/Stargate Worlds-QA/Working/SGWGame/Content/UI/Core/AutoAttack/AutoAttack.lua`.

- The icon click calls `setAutoAttack(not AutoAttackMod.autoEnabled)` only;
  the T binding calls `targetNextEnemy()` first if no target. `autoEnabled`
  updates on server `Events.AutoCycle`. The client `client.net.out` event
  records the `setAutoCycle` RPC name and wire sub-index 22, but not its
  argument; `client.state.field_update` records no state bits.
- In the Sep 29 colo CellBlock→Castle session, ten client toggle sends all
  arrived as `setAutoCycle enabled=true`. A hostile right-click launched
  weapon ability 579 before the first toggle, but no `setTargetID` had been
  sent in the session; the auto-cycle tick eventually cleared with target 0.
  Another toggle followed `setTargetID(0)` by 0.26 s and cleared with target
  0. Castle showed the same pattern. Direct right-click shots kept working.
- A separate Castle press at 19:20:07 left a friendly NPC selected. In the
  19:20:40–19:20:46 slice, 62 auto-cycle tick re-fire attempts all met 62
  `useAbility rejected -- ... non-hostile target` results; the loop stopped
  only on target=0. The tick validates death/despawn/surrender and player
  duel legality, but does not pre-filter friendly NPCs. It can look armed
  while doing no damage and produce 10 rejected calls/s.
- `player/interaction/interact.rs` sends `onTargetUpdate` on hostile click
  and launches the weapon ability, but does not write `current_target_id`;
  `being.rs` writes it only on `setTargetID`. The loop reads that field.
  This is a concrete server/client target-state mismatch, not an assumption
  that button packets are lost. The toggle also has no ability to cycle
  until a prior ability has committed; it uses `last_fired_ability_id`.
- Future diagnosis: correlate client `client.net.out setAutoCycle` with
  server `setAutoCycle enabled`, `setTargetID target_id`, `useAbility:
  launched`, and `auto_cycle_tick: target gone or disengaged target_id`.
  Client `ts_ms` is event time. Do not publish session/install identifiers.
- Fixed 2026-10-03 (branch fix/auto-cycle-target-state): hostile interact
  writes `current_target_id`; the tick clears a not-hostile NPC loop
  (`reason=not_hostile`); a press with no prior shot stashes the weapon's
  ranged ability; clear and enable events carry the reason and decision
  fields. Search `auto_cycle_tick: clearing loop` from then on.
- Existing docs `docs/analysis/cellblock-autoplay/scenario-map.md` had a stale
  claim that the BSF bit only arms on first ability; current code lights it
  immediately. Corrected with this investigation.
