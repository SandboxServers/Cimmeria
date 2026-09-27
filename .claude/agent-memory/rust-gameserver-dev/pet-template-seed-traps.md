---
name: pet-template-seed-traps
description: Authoring a pet template (350-359) or pet kit - NoPetLeveling freezes the pet at template level, most pet-kit abilities are silent no-ops, the NA43 allowlist, and the checks PT-03's guard puts on every summon row.
metadata:
  type: project
---

Learned authoring PT-11 (templates 351-353, 2026-09-27).

- **`ENTITYFLAG_NoPetLeveling` (8) freezes the pet at the template's level.**
  `spawn_pet_inner` (`crates/cell-world/src/cell/pets/spawn.rs`) only copies
  the owner's level when the bit is clear. Template 350 carried it (flags
  1032) until PT-11, so the L50-capstone Straegis spawned at level 1 / 250 HP.
  All roster pets (350-353) now use 1024 and follow D-PT02.
- **An ability with no visible result is refused/skipped for pets.**
  `cimmeria_entity::abilities::ability_is_unimplemented` (no event set, no
  effect with damage NVPs or a script): CM 88 refuses it
  (`reason=ability_not_implemented`) and the pet AI skips it. Test fixtures
  that seed bare ability defs (`seed_ability_defs`: no effects, no event set)
  are "unimplemented", so give them an event set or pet orders get refused.
- **Every `pet_summons` ability needs an `event_set_id`.** PT-03's live-DB
  guard `seeded_summons_match_the_summon_path` checks it (plus Self target,
  warmup > 0, SpeedPet). The Goa'uld summons use 1121; the 1122 target PFX is
  hard-coded in `use_ability/summon.rs` for every summon.
- **Pet-kit abilities are mostly empty.** 1654, 1653, 3326-3329: no effect
  NVPs, no script, no event set. (1652 Double Blast was the same, but PT-11
  gave it event set 3, the staff shot, so it needs no allowlist entry.) The NA43 linter
  (`npc_ability_animation.rs`) fails any template-set ability with no event
  set, so a pet kit needs either a data-backed event set or an
  `ANIMATION_ALLOWLIST` entry; `animation_allowlist_entries_deal_no_damage`
  then fails if an allowlisted ability ever gains damage/script/event set.
- **Heal abilities on a pet heal its enemy.** Pet AI and CM 88 aim every pet
  ability at a hostile combatant, so binding `HealHealth` to a pet kit needs an
  ally-target branch first.
- **`max_range` looks like centimetres** (1652 = 3000, i.e. 30 m), but the
  server reads world units; a ranged pet ability with a big `max_range` wins
  the reach filter at any distance. See [[npc-range-gate-and-weapon-range-columns]].
- `SpawnRecord` carries no `skin_tint`; compare it in SQL if a guard needs it.
- A worktree made by the Agent tool may have no `external` junction; create it
  (`New-Item -ItemType Junction`) before the first lane build. See
  [[build-environment]].

Related: [[entity-template-seed-authoring]], [[ability-event-sets-are-server-only]].
