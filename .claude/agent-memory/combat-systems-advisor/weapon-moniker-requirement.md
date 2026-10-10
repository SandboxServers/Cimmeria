---
name: weapon-moniker-requirement
description: CS-07 (2026-10-05, review fixes 2026-10-10) player weapon-moniker gate: where it lives, what python did differently, the 592 redirect removal, weapon grants on every weapon change, right-click with no RANGED binding, the seed rows it refuses
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

**Review fixes (2026-10-10, decisions OD-CS14..16 in the class-start-v6 ledger).**

- Weapon-granted abilities are transient, so EVERY path that changes the active
  weapon must swap them: world entry (`player_init::grant_active_weapon_abilities`,
  tags before the world-entry known-list send, no extra send), slot change
  (`active_slot.rs`), and `SyncBandolierItems` / `UpdateBandolierItem`
  (`bandolier::on_active_weapon_changed`, which also interrupts the warmup
  with `InterruptReason::ActiveWeaponChanged`, cancels a reload in flight or
  queued, and clears last-fired, auto-cycle and the queued attack). The
  reload completion refills the pinned slot INDEX, not the weapon, so any
  weapon-change path that skips the reload cancel loads the new gun for free.
- `AbilityManager::add_ability` drops a weapon-granted tag: a content or
  trainer grant of an ability the weapon also grants (597 is a USE binding on
  158 items) must survive the next weapon change. "Changed" = different
  `(instance_id, item_id)` in the active slot; a same-weapon resync must not call it.
- Right-click (`interaction/hostile_attack.rs`): RANGED binding, else 594
  unarmed, else NOTHING + "This weapon has no ranged attack." (no 592
  fallback). 50 ITEM_Rifle sniper rifles got 581 (items_event_sets 2768-2817).
  Item 5481 Crafted Pistol of the Whale (no bindings at all) got 579 RANGED
  (row 2818) in the second review round.
- Python parity is narrower than "did the same": TargetTarget only, after the
  cooldown, and `SGWPlayer.useAbility` never sent onErrorCode for a refusal
  (`if not status` on a truthy code).
- 592 itself: cooldown 0 floored to 0.5 s, 150F/15H; right-click's 579 is
  1.5 s, 100F/10H, so the bar shot is ~4.5x right-click DPS (seed numbers).
- `wrong_weapon_refused` and the right-click `weapon_unbound` rows are
  throttled per player via `SpaceManager::ability_refusal_log` (10 s); the
  metric still counts every press.

Related: [[ability-mechanics-gaps]], [[ability-range-units]].
