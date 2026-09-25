---
name: assist-aggro-na14
description: NA14 same-room assist hooks generate_threat; co-located same-faction NPCs in any test fixture now pull each other in; recruit gate is AggroCause::recruits_assist
metadata:
  type: project
---

NA14 (branch `npcai/na14-assist-aggro`, 2026-09-25) added same-room assist, a marked
deviation from legacy (2009 had none, D-NA04).

- Hook: end of `combat::generate_threat` (`combat/threat/aggro.rs`), only on a fresh
  Fighting entry and only when `AggroCause::recruits_assist()` (Damage | Proximity).
  Assist and ContentThreat never recruit, which is the whole no-chain rule. It runs
  AFTER the victim's `enter_player_combat`, so assisters' calls return `None`.
- Gates (`npc_ai/assist.rs`): same server faction as the victim, alive, Idle/Patrol/Wander
  (else `not_idle`), HOSTILE itself (so NEUTRAL spawns 10/20 never join), NA12 window,
  target GM `.aggro off`, then `aggro_gates::same_room` (4 u band, assister's own
  `entity_templates.assist_radius` default 10 u, LoS Unknown fails closed on a mesh).
- Only NPCs within 2x their assist radius are "considered" (get `assist_rejected` rows).

**Why it matters:** any test that spawns two faction-10 NPCs a few units apart and
shoots one now gets threat on BOTH. `auto_cycle_tick_refires_at_live_current_target`
broke this way (NPCs 50/75 were 2 u apart); fixed by pinning the bystander NEUTRAL.

**How to apply:** when a test asserts "NPC X has no threat" next to a shot neighbour,
pin X NEUTRAL (`aggro.override_level`) or move it >10 u / change faction. The damage
gate reads faction==10, not aggression, so a NEUTRAL pin does not mask mis-aimed hits.
Related: [[faction-10-gates-everything]], [[leash-reset-na12]].
