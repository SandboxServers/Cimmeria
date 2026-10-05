---
name: weapon-moniker-requirement
description: CS-07 (2026-10-05) player weapon-moniker gate: where it lives, what python did differently, the 592 redirect removal, and the seed rows it refuses (own-weapon-refused basic attacks, FAIL abilities)
metadata:
  type: project
---

**Rule (OD-CS11, Class Start v6 CS-07, 2026-10-05).** A player's cast of an
ability with non-empty `abilities.item_monikers` fires only if the active
bandolier item's `items.moniker_ids` shares one (any-match). Empty slot fails;
no requirement = unchanged; NPC/pet casts never checked. Gate:
`crates/cell-combat/src/cell/abilities/use_ability/weapon_requirement.rs`,
called inside `handle.rs`'s `'validate` block right after the known-set check,
before the cooldown check. Pure rule: `AbilityDef::weapon_satisfies`
(`crates/entity/src/abilities/weapon_requirement.rs`). Data:
`AbilityDef::item_monikers` + `SpaceManager::item_monikers`
(`spawner::load_weapon_monikers`). Wire: `onErrorCode(0, ability, 63)` +
CHAN_FEEDBACK line, row `wrong_weapon_refused` (INFO), metric reason
`wrong_weapon_type`; clears an auto-cycle loop armed on that ability.

**Why:** python enforced it (`AbilityManager.py:543`, `SGWPlayer.hasItemMoniker`);
the Rust loader never selected the column, so any ability fired with any gun.

**How to apply:**
- Python checked it ONLY in the `TargetTarget` branch of `canUse`; Rust checks
  every target type (OD-CS11 is global). 7 self-target + 18 ground-target
  player abilities are newly gated (850 Scattershot, 1486 Rain of Steel, the
  grenade launchers, staff ground blasts...).
- The 592 -> active-weapon redirect (#495, `weapon_redirect.rs`) is GONE. 592
  needs ITEM_Pistol; with item 21 it is refused, not turned into 559. A
  weapon's own basic attack is the weapon-granted id from
  `swap_weapon_granted_abilities` (transient, never persisted).
- Content `launch_ability` goes through `content::effect_apply`, not
  `handle_use_ability`: unchecked.
- A test fixture that fires a moniker-requiring def must populate
  `mgr.item_monikers` and a bandolier item; seed-loaded defs carry monikers.
- Seed rows that the rule refuses (do NOT weaken the rule, fix the seed):
  ribbon devices (27 items, 4458..8341) bind 584/710 Staff AA (ITEM_Staff);
  Jaffa Cannons (25, 6617..6655) bind 2635/2636 Staff AA but carry an unnamed
  moniker 1028276165, not ITEM_Staff; dartguns (25, 3584..5039) bind 708 Pistol
  Melee AA but carry ITEM_DartPistol; 3128 Test Weapon and 3514 Base P90 +ACC
  carry no monikers. FAIL abilities (no bandolier item satisfies): 997 Full
  Mag: Dart Pistol (ITEM_Dart_Rifle), 1246 Stealthed Strike and 1355 Lethal
  Strike (ITEM_Melee), 1250 Escape (stealth ARMOUR monikers, never a weapon).
- Rerun `python tools/ability_mechanics/weapon_requirement_audit.py` after any
  seed change to monikers or grants; doc at
  `docs/analysis/class-start-v6/weapon-requirement-audit.md`.

Related: [[ability-mechanics-gaps]], [[ability-range-units]].
