---
name: player-cast-fixtures-need-a-mechanic
description: Since AB-12 a player's cast of an ability with no damage NVP/script/binding is refused before the cooldown; effectless test fixtures must seed the shared no-op mechanic effect
metadata:
  type: project
---

Since AB-12 (2026-10-03) `handle_use_ability` refuses a **player's** press of an ability with no mechanic (`use_ability/no_mechanics.rs`, `ability_has_mechanics`): `onErrorCode(0, id, 167)` + "That ability has no effect yet.", no cooldown, no timer, returns `false`. NPC casts, weapon-granted abilities (`items_event_sets`) and `required_ammo > 0` shots are exempt.

**Why:** D-AB10, every press gets feedback. It broke ~40 tests whose `AbilityDef` fixtures had `effect_ids: vec![]` (auto-cycle, range, LOS, sequence tests in cell-combat, cell, cell-methods) and one live-DB test that fired a seeded NPC ability (1652) from a player and passed vacuously afterwards (the 167 refusal is not the 42 it looked for).

**How to apply:** a test ability that stands for "some attack" fired by a player needs `effect_ids: vec![MECHANIC_FIXTURE_EFFECT]` plus `seed_mechanic_effect(&mut mgr)` (cimmeria-cell-world `test_fixtures`, re-exported via each crate's `crate::test_support`); it adds a registered no-op script to whatever registry is installed. In cell-combat's use_ability tests, `make_ability` + `make_mgr` already do this. A count of seeded abilities with a mechanic is pinned in `no_mechanics_live_db.rs` (`HAS_MECHANICS_TODAY`, 108 on 2026-10-03) and moves when generator packets land. Related: [[damage-apply-miss-gate-and-seeded-rolls]].
