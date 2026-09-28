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

**UAT tuning (2026-09-26, branch fix/colo-escort-and-rally):** template 24 (NID
Guard, every Cellblock guard) seeds `assist_radius = 26`. Barracks guards
(spawns 25/26/36) are 13.1/19.3/25.0 u apart, so 10 u rallied nobody. On
`castle_cellblock.occ` the three barracks guards see each other; Hallway01/02/03
(18.6/20.5 u) are occluder-Blocked, so 26 u does not link them. Production
assist LoS comes from the occluder when a world ships one, not the navmesh.
The live-DB guard `barracks_guards_assist_radius_covers_the_room` replaced
`no_seeded_template_sets_an_assist_radius_yet` and pins the tuned set: since the 2026-09-28 Castle population it is {24, 181-186} (181-186 at 12 u), and `live_db_aggression::seed_overrides_only_the_chain_armed_spawns` pins the aggro radii 15/20 on 181-186 -- both pins must be updated by any packet that tunes a radius.

**How to apply:** when a test asserts "NPC X has no threat" next to a shot neighbour,
pin X NEUTRAL (`aggro.override_level`) or move it >10 u / change faction. The damage
gate reads faction==10, not aggression, so a NEUTRAL pin does not mask mis-aimed hits.
Related: [[faction-10-gates-everything]], [[leash-reset-na12]].
