---
name: support-shot-inverse-gate
description: The client's useAbility emit has no friend-or-foe check (Ghidra 2026-09-28); beneficial ammo is the one inverse #444 gate, classified at launch, warmup and fire in use_ability/support_shot.rs
metadata:
  type: project
---

**Client side (headless Ghidra, 2026-09-28, AM-11d).** `useAbility` Lua thunk `0x00aa2910` -> `FUN_00ad78e0` (target lookup, GameBeing cast, both branches identical) -> `FUN_00d2afc0` -> `FUN_00d2ae40` (branches only on target type 3 = ground) -> `Event_NetOut_UseAbility`. No faction or hostility read anywhere, and `Ability.lua` passes `Unit.Target` unconditionally. So the server's #444 gate is the only friend/foe check. Right-click sends `interact`, never a shot at an ally. Not verified: whether the client can target self, and what it sends with no target.

**Server seam.** A beneficial `ammo_modifiers` row (`beneficial` column) turns a player's weapon shot into a support shot: `crates/cell-combat/src/cell/abilities/use_ability/support_shot.rs`. It is classified three times (launch in `handle.rs`, warmup re-check in `warmup/tick.rs`, fire in `fire.rs`), and `damage_apply` drops a beneficial row's on-hit effect as a belt. A future supportive *ability* flag needs the same three sites.

**Why:** a new friendly-target feature that patches only the launch gate is interrupted by the warmup re-check (TargetLost) or refused again at apply time (`player_hit_refusal`).

**How to apply:** grep for `player_may_attack` in cell-combat before widening any targeting; every hit is a site to keep in step. Related: [[npc-caster-player-ordered-gates]].
