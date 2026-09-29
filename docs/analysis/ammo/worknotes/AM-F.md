# AM-F Worknotes

> Type: reference. Audience: the ammo coordinator and the Wave-1 packet workers.
> Companions: [README.md](../README.md), [work-packets.md](../work-packets.md), [audit.md](../audit.md), [AM-01 findings](../../../reverse-engineering/findings/ammo-system.md).

## Contract

- **Packet:** AM-F, the foundation. The only serial gate.
- **Decisions in force:** D-AM01 (reserve in the bags), D-AM02 (default free, special finite), D-AM05 (rounds, never punish), D-AM07 (the server applies the modifier directly), D-AM08 (daggers later), D-AM10 (widen Standard Pistol and Standard SMG).
- **Audit rows:** A-14 (ordinals unpinned), A-15 (ids 9000-9099 free), A-16/A-17 (the two families), A-28 (`\ir` lines).
- **Base:** `origin/main` @ `25101190`, branch `ammo/am-f-foundation`, test database `sgw_agent_a347796ab95926673`.

## What shipped

| Piece | Where |
|---|---|
| 15 reserve items, ids 9000-9014 | `db/resources/Items/Seed/ammo_items.sql` |
| `resources.ammo_item_types` table and seed | `db/resources/Items/Tables/ammo_item_types.sql`, `db/resources/Items/Seed/ammo_item_types.sql` |
| Standard Pistol (27) and Standard SMG (25) widened to the five bullet specials | `db/resources/Items/Seed/ammo_weapon_widening.sql` |
| `resources.ammo_modifiers` table, empty | `db/resources/Abilities/Tables/ammo_modifiers.sql` |
| Five `\ir` lines | `db/database.sql` |
| `EAmmoType` ordinals, `LABELS`, `label`, `is_special` | `crates/entity/src/ammo_type.rs` |
| `ammo.finite_special` flag, default off | `crates/entity/src/ammo_feature.rs`; init in `crates/server/src/main.rs` |
| Telemetry catalog constants | `crates/entity/src/ammo_telemetry.rs` |
| `AmmoReserve` (`count`, `draw`, `return_rounds`) | `crates/base-methods/src/base/world_entry/methods/inventory/ammo_reserve/` |
| `AmmoModifier`, `AmmoCatalog`, loaders | `crates/cell-catalog/src/cell/spawner/ammo_catalog.rs` |
| `SpaceManager::ammo_catalog`, loaded at cell startup | `crates/cell-world/src/cell/space_manager/mod.rs`, `crates/cell/src/cell/service/startup.rs` |
| `ammo=debug` `OTEL_FILTER` row and its pin | `crates/server/src/logging/filters.rs`, `target_scan_tests.rs` |
| Empty pre-split modules, declared | `player/world/reload_reserve.rs`, `bandolier/switch_return.rs` (AM-02); `effects/ammo_damage.rs` (AM-04); `effects/ammo_{incendiary,emp,explosive,dart_cc,dart_tech,dart_support}.rs` (AM-08..AM-11c) |

## Changes from the plan

Each is already folded into [work-packets.md](../work-packets.md) § Contract fixed by AM-F.

1. **`is_special(DAGGER_DEFAULT)` is false.** The plan listed only `AMMO_NONE`, `Bullet_Default` and `Dart_Default` as free. `Dagger_Default` is default ammo by the same rule (D-AM02), and the table CHECK constraints exclude the same four types.
2. **`AmmoDraw` and `AmmoReturn` carry more than the counts:** `item_id`, `stack_before`, `stack_after` and `changes: Vec<StackChange>`. AM-02 needs the touched rows to update the client (`onUpdateItem` / `onRemoveItem`) and the before/after totals for `reload_draw` / `ammo_switch_return`. `RESERVE_BAGS` (1, then 15) is public.
3. **`AmmoReserve` is a directory** (`ammo_reserve/{mod.rs,plan.rs,live_db_tests.rs}`), not one file. The module path is the same.
4. **`ammo_modifiers` has a `damage_type "EDamageType"` column** (nullable), because D-AM07 lists damage type among what the modifier sets. The row also has CHECKs (multipliers `> 0`, no free types).
5. **The loader is `spawner/ammo_catalog.rs`**, holding both tables in one `AmmoCatalog`, with `load_ammo_modifiers` kept under its planned name. `item_id_for` on the cell is `space_mgr.ammo_catalog.item_id_for(ammo_type)`.
6. **Five `\ir` lines, not four.** The plan left out the `ammo_modifiers` table. The `ammo_item_types` table file loads in the seed section, after `items.sql`, because its foreign key needs items' primary key.
7. **The feature-flag surface did not exist.** AM-F added `crates/entity/src/ammo_feature.rs`: an env var (`CIMMERIA_AMMO_FINITE_SPECIAL`) read once at startup into a process-wide switch.
8. **The `ammo` OTEL row moved from AM-02 to AM-F**, so no Wave-1 packet edits `crates/server/src/logging/`.
9. **AM-F also touches `SpaceManager` and the cell startup loader** (for the catalog), and declares every ammo module, so no later packet edits `effects/mod.rs`, `bandolier/mod.rs` or `player/world/mod.rs`.

## One visible change

The widening is visible, and it could not be otherwise: `player_load` and the inventory resync read `ammo_types` from `resources.items`, so a player's existing Standard Pistol or SMG offers five more types in the ammo picker at next login. With the flag off and no modifier rows yet, picking one changes nothing: reloads stay free and damage is unchanged until AM-04 seeds its rows. Items 9000-9014 exist but nothing grants or drops them until AM-05 / AM-06.

## Tests

| Test | Type | Proves |
|---|---|---|
| `ammo_type::tests` (3) | unit | `is_special` for all 24 ordinals and out-of-range values; `LABELS` lines up with the constants |
| `ammo_feature::tests` (3) | unit | default off; every spelling parses; a bad value falls back to off and is flagged |
| `ammo_telemetry::tests` | unit | names distinct, snake_case |
| `ammo_reserve::plan::tests` (10) | unit | D-AM05 arithmetic: exact fit, short stack, multi-stack, over-capacity return, conservation |
| `ammo_catalog::tests` (3) | unit | lookups both ways, misses |
| `live_db_ammo_catalog::ammo_type_ordinals_match_pg_enum` | live-DB | the Rust ordinals equal `pg_enum` label by label (A-14) |
| `live_db_ammo_catalog::ammo_item_types_maps_every_bullet_and_dart_special_once` | live-DB seed guard | 15 rows, the bullet and dart specials, each to an existing stackable 9000-9014 item outside `WeaponDef` |
| `live_db_ammo_catalog::standard_pistol_and_smg_accept_every_bullet_special` | live-DB seed guard | 27 + 25 ids list default plus the five specials once; `WeaponDef` carries them |
| `live_db_ammo_catalog::load_ammo_catalog_reads_both_tables` | live-DB | the loader |
| `ammo_reserve::live_db_tests` (6) | live-DB | draw across two stacks and delete the emptied one; never over-draw; the vault does not count; default ammo has no reserve; return merges first then opens a free slot; remainder when bags are full; rollback |
| `target_scan_tests::ammo_target_reaches_otlp_at_debug_info_and_warn` | logging guard | `ammo` reaches one OTLP index at DEBUG, INFO and WARN |

**Revert proof.** With the widening `\ir` line removed, the DELETE of an emptied stack turned into an UPDATE, and the free-slot filter ignoring occupied slots, five guards failed (`standard_pistol_and_smg_accept_every_bullet_special`, both draw guards, both return guards); restored, all pass.

## Notes for Wave 1

- **AM-02:** call `draw` / `return_rounds` inside the transaction that also writes the slot's ammo, read `finite_special()` once at the entrypoint, and push each `StackChange` to the client after the commit.
- **AM-03:** the refusal reasons are in `ammo_telemetry::reasons`.
- **AM-04:** `space_mgr.ammo_catalog.modifier(ammo_type)`; `None` means fire unmodified.
- **AM-05:** item ids come from `ammo_item_types`; loot rows can name 9000-9014 directly (seed data), and the seed guard above pins the mapping.
- **AM-06:** `space_mgr.ammo_catalog.item_id_for(ammo_type)` maps `.gmgiveammo`'s `EAmmoType` to the item.
- **AM-07:** the 15 definitions to push are ids 9000-9014; names, stack cap (500) and the placeholder icon are in `ammo_items.sql`.

## D-AM10 amendment (2026-09-28)

@Cadacious amended D-AM10: the **High Capacity SMG** family also accepts all five bullet special types. Without it the Hollow Point the Castle pre-Romney chest hands out does not fit the SGHC 6 SMG (3127) that same chest hands out.

- **Ids (27, every `items.sql` row described 'High Capacity SMG', all `{Bullet_Default}` before):** 21, 3126, 3127, 3129, 3130, 3131, 3132, 3135, 3136, 3138, 3139, 3140, 3531, 4693, 4694, 4695, 4696, 4697, 4698, 4699, 4700, 4701, 4703, 4704, 4705, 4706, 4707 (SGHC 6, 6 AP, 7, X and Gauss SMG).
- **Seed:** the family joins the `WHERE description IN (...)` list of `ammo_weapon_widening.sql`, the same UPDATE that widened Standard Pistol and Standard SMG.
- **Guard:** `standard_pistol_and_smg_accept_every_bullet_special` now pins 27 + 25 + 27 ids and checks that the `WeaponDef` for 3127 carries the five types. With the family removed from the UPDATE, the guard fails; restored, it passes.
