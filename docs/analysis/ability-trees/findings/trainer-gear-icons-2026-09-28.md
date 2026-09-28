# Finding: gear icons in the Archetype Skills Trainer (2026-09-28)

> Type: reference (investigation note). Status: investigated, not fixed.
> Base: `origin/main` @ `cf89f314a`. Companions: [campaign README](../README.md), [AT-E1 client finding](../../../reverse-engineering/findings/ability-trainer-ui.md), [ability system](../../../gameplay/ability-system.md#ability-trees-and-training), [cooked data pipeline](../../../engine/cooked-data-pipeline.md).

## Observation

Owner playtest, 2026-09-28: a level-1 Soldier with 0 training points opened the stock Ability window ("Archetype Skills Trainer") from the stasis-room debug-hub trainer in Castle_CellBlock. On tab 1, the 5-wide grid showed a gear/cog placeholder in row 3 slot 5, row 4 slot 4, and row 5 slots 1 and 2. Every other slot had a real icon.

The report said "five slots" but named four positions. Tab 1 has exactly four nodes with a placeholder icon (below), so four is the count the data predicts. A fifth gear on tab 1 would not come from this cause.

## Verdict

**(a) Stock client data.** The server sends the ids the tree says it should, and the icon never comes from the server's trainer or tree messages. All four abilities are real Soldier automatic-weapon abilities. Their cooked client entries have no art: `IconLocation="set:CoreWidgets image:IconMissing"`, and `IconMissing` is the gear. Any server that offered these ids would show gears. The tree placement is the owner's FINAL v2 workbook, so whether retail ever offered them on a trainer can't be recovered. The client's own data has no icon for them either way.

This is not specific to the Soldier tab. **306 of the 439 seeded tree nodes** show the gear, and Soldier Automatic Weapons has the fewest. See [scale](#scale).

The server can still fix it without a client patch, through the same cooked-data override path already used for item icons. See [proposed fix](#proposed-fix).

## 1. What draws the slot

- `Ability.lua:65` gets the tab's ids from `getTrainableList(treeIndex)`. AT-E1 §1 shows this walks the tree cache filled by `onAbilityTreeInfo` (client method 141), in wire order.
- `Ability.lua:75` hides a slot whose id is missing from the trainer's `onTrainerOpen` list (`Ability.lua:133`, `buttonWin:hide()`). A gear slot is therefore on both lists: it is shown, not hidden.
- `Ability.lua:79` calls `getAbilityInfo(trainableInfo.id)`, and `Ability.lua:115` sets the button's `Icon` to `abilityInfo.icon`.
- `getAbilityInfo` reads the client's cooked ability catalogue (category 2, `CookedDataAbilities.pak`; [cooked-data-pipeline.md](../../../engine/cooked-data-pipeline.md) category table). The icon is that entry's `IconLocation` attribute. No server message carries an icon: `onAbilityTreeInfo` is `ARRAY<ARRAY<INT32>>` (`entities/defs/SGWPlayer.def:1356-1358`), and `TrainerAbility` is `{INT32 abilityID, UINT8 trainable}` (`entities/defs/alias.xml:417-422`).
- The gear is not a Lua fallback. It is the literal image the cooked entry names. `CoreWidgets.imageset:121` defines `IconMissing` at (5, 319), 32×32. Cropping that rectangle from `CoreWidgets.tga` (512×512, 32-bit) gives a six-toothed white cog, the shape in the screenshot.

Client paths above are relative to `Stargate Worlds-QA/Working/SGWGame/Content/UI/` (`Core/Ability/`, `CEGUIData/imagesets/`).

## 2. Which ids land in those slots

`tree_info` (`crates/cell-catalog/src/ability_tree/tree_info.rs:38-42`) pushes each node into `trees[tree_index]` in catalog order `(tree_index, ability_index)` ([ability-system.md](../../../gameplay/ability-system.md#ability-trees-and-training)). Tab 1 is `tree_index = 0`, "Automatic Weapons", 22 nodes. Grid position = `ability_index`, so row r, slot s = index `5(r-1)+s`.

| Grid | Index | Ability | Name | Level | Seed line (`archetype_ability_tree.sql`) |
|---|--:|--:|---|--:|--:|
| row 3, slot 5 | 15 | 867 | Quickload | 35 | 42 |
| row 4, slot 4 | 19 | 1475 | Flushing Fire | 45 | 50 |
| row 5, slot 1 | 21 | 1476 | No Rest for the Weary | 45 | 54 |
| row 5, slot 2 | 22 | 1477 | All Out Assault (capstone) | 50 | 56 |

All four rows are `project_status = 'FINAL_V1'`, `branch_name = 'Automatic Weapons'`, generated from the owner workbook. For comparison, index 20 (1445 EMP Ammunition) borrows an item sprite (`set:WeaponIcon002 image:RPGAmmo_00`, which `TaharezLook.scheme:37` loads). It renders a real picture, which matches the screenshot.

## 3. Icon data per id

| Ability | `abilities.icon` seed (`db/resources/Abilities/Seed/abilities.sql`) | Server PAK `data/cache/CookedDataAbilities.pak` | Client PAK `SourceCache.en-us/CookedDataAbilities.pak` |
|--:|---|---|---|
| 867 | `set:CoreWidgets image:IconMissing` (line 4945) | same | same |
| 1475 | same (line 233) | same | same |
| 1476 | same (line 1473) | same | same |
| 1477 | same (line 1490) | same | same |

The seed column mirrors the PAK and is documentary: the client never reads it.

Are these abilities misplaced? No:

- **They are real.** Each has a real name, description, effects and monikers. 867: "Reduces Reload Time by 50%". 1475: "Channeled Medium Cone Attack". 1476: "Auto Weapons: Ranged Single Target Medium Cone Attack w/ High Interrupt Chance". 1477: "AutoWeapons: Ranged Single Target Medium Cone Channelled Attack". All are automatic-weapon abilities on the Soldier's Automatic Weapons branch, not test or placeholder rows (there is no `MS0xx_` prefix, unlike test copies such as `_1671 MS018_063008_Spray and Pray`).
- **No other cooked copy has art.** A name search of the PAK finds no second entry for any of the four names.
- **Their art was simply never made, or never wired.** The cooked catalogue uses `IconMissing` for **1,607 of 1,886** abilities, so a missing icon was the norm in this client build.

## Scale

Counted over the seeded tree (`archetype_ability_tree.sql`, 439 nodes) against the server PAK:

| Archetype | Tab 1 | Tab 2 | Tab 3 |
|---|--:|--:|--:|
| Soldier | 4/22 | 8/25 | 10/25 |
| Commando | 8/23 | 17/22 | 17/19 |
| Scientist | 22/22 | 15/20 | 16/17 |
| Archeologist | 20/21 | 14/20 | 24/24 |
| Asgard | 24/24 | 21/21 | 19/21 |
| Goa'uld | 16/21 | 13/21 | 13/20 |
| Shol'va | 7/21 | 5/8 | 13/22 |

Total: 306 of 439 nodes draw a gear. The owner saw the best tab of the best archetype.

## Proposed fix

No server message is wrong, so there is nothing to correct in `tree_info`, `onTrainerOpen` or the seeds. The server-authoritative improvement is a **cooked ability override**. It needs no client patch:

1. Add `crates/resources/src/base/ability_overrides.rs`, modelled on `item_overrides.rs`. That module already fixes item 2893's `IconMissing` this way (`item_overrides.rs:73-88`). Wire it into `apply_overrides.rs` for category 2 (`CookedDataAbilities`), with the same metadata bump, so `versionInfoRequest` → `onVersionInfo(InvalidKeys)` → `resourceFragment` pushes the patched `_<id>` entries. The PAK keeps the SOAP namespaces on `COOKED_ABILITY` ([cooked-data-pak-format.md](../../../engine/cooked-data-pak-format.md), "CookedDataAbilities -- SOAP Namespace Anomaly"), so the patcher must rewrite the attribute in place, not re-serialise the element.
2. Point `IconLocation` only at sprites the client already has. There is one clear match: **867 Quickload → `set:AbilityIcons001 image:Buff_Quickload`** (`AbilityIcons001.imageset:12`). No cooked ability uses that sprite today. Ten `AbilityIcons001` sprites are unused (`Buff_Quickload`, `Reload_Reload`, `AOE_Locked_Target`, `Cone_Strafe`, `AOE_High_Explosive_Mortar`, `Buff_Shock_and_Awe`, `Buff_Steady_Launch`, `Passive_Disarm_Device`, `Cone_Penetrating_Barrage`, `Buff_Quick_Launch`). `AbilityIcons002` has two named sprites, and the other 62 are `00000` blanks.
3. Mapping 1475, 1476, 1477 or the other ~300 nodes to existing art means choosing icons that were never assigned, such as re-using a sibling's sprite or `Cone_Strafe`. That is an **owner decision**. The candidates are one icon per node from the unused set, re-using a related ability's sprite, or leaving the gear as the honest "no art" marker.
4. Update the `abilities.icon` seed rows to match any override, as `item_overrides.rs` asks for items. Tests: a unit test that the patched XML carries the new `IconLocation` and still parses, and an override-category test like `crates/resources/src/base/resources/tests/overrides.rs:411`.

A small wording fix goes with this: `item_overrides.rs:75-77` calls `IconMissing` "the placeholder broken-square icon". It is a gear.

## UAT check

Hover each gear slot. The tooltip (`Ability.lua`, `setTooltipText(name .. "\n" .. description)`) should read Quickload, Flushing Fire, No Rest for the Weary and All Out Assault, in the four positions above.
