---
title: "NPC Combat Reference Pipelines"
type: reference
audience: engineers
last_updated: 2026-11-27
---

# NPC combat reference pipelines

> **Type**: reference. **Audience**: engineers adding or changing NPC combat
> profiles. **Last updated**: 2026-11-27. **Campaign**:
> [npc-combat-reference](../analysis/npc-combat-reference/README.md) (NC-32).
> **Companions**: [npc-ai.md](npc-ai.md), [combat-system.md](combat-system.md),
> [abilities-and-effects-system.md](../architecture/abilities-and-effects-system.md),
> [Combat reference range](../content/debug-area.md#combat-reference-range).

Four reference NPCs show the four ways an NPC attacks. Each pipeline runs
data, then selection, then effect, then presentation. This page names the
test that guards each stage and the evidence class of each claim. The
per-row evidence lives in the
[evidence registry](../analysis/npc-combat-reference/evidence-registry.md);
this page does not repeat it.

Evidence classes are the campaign's six (ORIGINAL CLIENT VERIFIED, EARLIEST
LEGACY DATA, CURRENT CIMMERIA IMPLEMENTATION, STRONGLY INFERRED,
RECONSTRUCTED, UNRESOLVED), defined in the
[campaign README](../analysis/npc-combat-reference/README.md#evidence-classifications).
Reconstructed values stay RECONSTRUCTED until a client record names them.

## Shared stages

- **Data.** `entity_templates.ability_set_id` points at `ability_set_abilities`
  rows; the template's components name the weapon mesh. Seeds under
  `db/resources/` are the only source.
- **Resolver.** `resolve_npc_combat_profile`
  (`crates/cell-world/src/cell/combat/combat_profile/mod.rs`) turns a template
  into a profile. `PLACEHOLDER_POLICY` is `RequireWeaponEvidence` (NC-30,
  D-NC1): a template with no set earns Pistol Shot (592) only through a
  `WP-Human.` component. Every other set-less template idles and logs
  `combat_profile_unresolved`. A template granted 592 logs
  `combat_profile_placeholder` instead. Both rows carry the template name
  next to the id.
- **Selection.** `choose_npc_ability` walks the set in ascending id order.
  Inside `NPC_MELEE_RANGE` an off-cooldown melee ability goes first (NC-20).
- **Presentation.** `abilities.event_set_id` is the server's whole lever; the
  client has no ability-keyed lookup. See the
  [presentation chain](../analysis/npc-combat-reference/presentation-chain.md).

### Combat profile table and diagnostics

`combat_profile` (`crates/cell-world/src/cell/combat/combat_profile/`) resolves
which abilities a spawned NPC gets. Spawning and the AI share the rule: an
explicit `ability_ids` set always wins, and an empty set takes the placeholder
that the policy allows.

| `ability_ids` | Policy | Components | Abilities | Source |
|---|---|---|---|---|
| non-empty | any | any | the set, in order | `ExplicitSet` |
| empty | `Legacy` | any | `[592]` | `LegacyPlaceholder` |
| empty | `RequireWeaponEvidence` | a `WP-Human.*` component | `[592]` | `LegacyPlaceholder` |
| empty | `RequireWeaponEvidence` | otherwise | `[]` | `Unresolved` |

592 Pistol Shot is a Giza/Python placeholder for an NPC with no ability set
(`spawn.rs:314-317` before NC-10), not Cimmeria behaviour, and it is a human
firearm. The policy rule in the Resolver bullet above decides who keeps it.

Two diagnostics log on target `npc.combat_profile`, at most once per template
id per process:

- `combat_profile_placeholder` (INFO): the template got the 592 placeholder.
  Fields: `template_id`, `template_name`, `body_set`, `weapon_component`
  (or `none`), `ability_id`, `ability_name`.
- `combat_profile_unresolved` (WARN): the template has no ability set and no
  weapon evidence, so it holds fire. Fields: as above, without `ability_id`.

An explicit set logs nothing. In the AI, an NPC that knows no ability gets
no fallback: `placeholder_ability(policy)` returns 592 under `Legacy` and
`None` under `RequireWeaponEvidence`, so the NPC idles (D-NC1). The 592 for a
`WP-Human.*` template comes from spawn resolution, not from the AI.

## The four pipelines

All four Z12 tests live in
`crates/cell/src/cell/service/tests/npc_ai/debug_area_combat/reference/`.

| Reference | Data (set, ability, effect) | Selection | Effect | Presentation | Evidence class | Z12 tests |
|---|---|---|---|---|---|---|
| NID Guard (template 24) | Set 3, ability 559, event set 15, component `WP-Human.WP_SMG_1A`, loot 2, `use_cover` | Ranged; cover through `move_to_cover` / `stay_in_cover` | Effect 5310 `RangedPhysicalDamage`, 10 Health and 100 Focus, RECONSTRUCTED from the 559 tooltip (D-NC8, NC-25) | Event set 15; generic KIS human sequences | EARLIEST LEGACY DATA (set, 559, event sets, loot); ORIGINAL CLIENT VERIFIED (component exports); RECONSTRUCTED (effect 5310); UNRESOLVED (KIS KeyNames) | `nid.rs`: `nid_reference_carries_smg_evidence`, `nid_reference_takes_cover`, `live_db_reference_nid_smg_damages_a_player`, `live_db_reference_nid_loot_and_respawn`, `live_db_reference_nid_559_uses_reconstructed_effect_5310` |
| Praxis Jaffa (159, 160; clones 1380, 1381) | Set 4: 584 ranged, 710 melee; staff `WP-Jaffa.WP_Staff_Plasma_4A` (D-NC3, RECONSTRUCTED) | Melee first in reach, 584 elsewhere (NC-20) | Effect 736 `MeleePhysicalDamage` (NC-12, RECONSTRUCTED) | Event sets from the 35 legacy links | EARLIEST LEGACY DATA (set 4, 584, 710); RECONSTRUCTED (staff component, effect 736); ORIGINAL CLIENT VERIFIED (staff and melee exports) | `jaffa.rs`: `live_db_reference_jaffa_melee_effects_carry_damage`, `live_db_reference_jaffa_staff_swing_damages_in_reach`; `melee_preference.rs`: `live_db_reference_jaffa_swings_staff_in_reach` |
| ScavDog (76) | Set 6: 1433 Mob Strike (RECONSTRUCTED, D-NC5); no weapon component | Melee only; never fires at range | Effect 1717 (NC-12, RECONSTRUCTED) | Event set 300 (PR #822 Tier 3, RECONSTRUCTED); `mob_scav.fev` ownership UNRESOLVED | RECONSTRUCTED (set 6, effect 1717, event set 300 link); UNRESOLVED (mob_scav.fev) | `scavdog.rs`: `scavdog_reference_is_melee_only`, `live_db_reference_scavdog_bites_in_reach_only`, `live_db_reference_scavdog_set_6_loads` |
| Prisoner Retrieval Unit (4) | Set 2: ability 221 Energy Shock; integrated weapon | Ranged drone attack | Effect 264 `RangedEnergyDamage`, 16 Health and 80 Focus | Event sets 802 and 803 | EARLIEST LEGACY DATA (set 2, 221, effect 264); UNRESOLVED (slot KeyNames, 1869 anomaly) | `pru.rs`: `pru_reference_has_integrated_weapon`, `live_db_reference_pru_energy_shock_defs`, `live_db_reference_pru_shock_damages_a_player` |

Range-level guards are in `reference/mod.rs`
(`reference_range_holds_the_four_references`, `reference_pens_are_apart`,
`reference_profiles_today`, `live_db_debug_area_reference_rows_load`) and the
policy guard in `placeholder_policy.rs`.

RE findings: [reference-npc-presentation.md](../reverse-engineering/findings/reference-npc-presentation.md) (NID, Jaffa, PRU), [scavdog-combat-assets.md](../reverse-engineering/findings/scavdog-combat-assets.md) (ScavDog), [ability-animation-links.md](../reverse-engineering/findings/ability-animation-links.md) (the event-set links).

## How to add a new NPC combat profile

NC-34's [roster follow-up](../analysis/npc-combat-reference/roster-followup.md)
follows these steps for each roster NPC.

1. **Find the client evidence first.** Look for a record or sequence that
   names the attack in the NPC's own package. Without it the profile is
   RECONSTRUCTED, and the seed comment says so with the owner decision.
2. **Pick the matching pipeline** from the table above (organic: ScavDog;
   armed humanoid: NID; staff bearer: Jaffa; construct: PRU).
3. **Add the set in the seed**, not a migration: a new `ability_set_id`, its
   `ability_set_abilities` rows, and `entity_templates.ability_set_id`. Keep
   each INSERT on one line in the file's column order.
4. **Give every effect a payload.** An effect with no script and no NVP damage logs
   `effect_inert` and does nothing; add the script and NVPs (NC-12 is the
   pattern).
5. **Check the event set** carries an `Ability_End` (1001) sequence, or the
   client plays nothing.
6. **Add the Z12 tests**: a unit test for the profile, plus a `live_db_`
   test for the damage and the in-reach or out-of-reach choice. They must
   fail when the seed row is reverted.
7. **Add a UAT row** to the [Z12 checklist](../analysis/npc-combat-reference/uat-checklist.md)
   and a row to the evidence registry.
8. **Confirm telemetry**: no `combat_profile_placeholder`,
   `combat_profile_unresolved` or `effect_inert` row for the new template.

## Not yet verified in the client

W4 lab runs (2026-11-27, [w4-lab-results.md](../analysis/npc-combat-reference/w4-lab-results.md)) confirmed the server side: NID 559/5310, PRU 221/264, ScavDog 1433, Jaffa 584/710/736, and no ability 592 row in the session. Not verified: animation, muzzle effects, VFX and sound (the lab driver reads chat and the combat log only); the NID 20 s respawn; the T2 Z9 squad. Nothing here is ORIGINAL CLIENT VERIFIED from a visual read.

Rows that rest on RECONSTRUCTED values (N3, the J2 damage number, S2, S3)
stay RECONSTRUCTED even when they pass.

## Open questions

- Whether the client plays an effect's target event set is UNRESOLVED
  (NC-06); the server reads only the ability's set.
- `mob_scav.fev` has five variants of one `attack` event and no notify names
  them (NC-05, registry O5).
