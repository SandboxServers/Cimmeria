# Starter-world vendor coverage map (SS-04)

> **Type:** campaign evidence (coverage map). **Audience:** the orchestrator, lomiada and the owner, who decide D-SS1.
> **Campaign:** [starter-uat-shops](README.md). **Packet:** SS-04. **Companions:** [work-packets.md](work-packets.md), [README.md](README.md), [contract § C7](../starter-uat-praxis/work-packets.md#c7-repo-rules-every-authoring-packet-carries).

This map lists every vendor and trainer spawn in the fourteen starter worlds named by SS-04, and says which campaign row covers each one. It changes no seed and no spec. Counts come from the seed files, not from the client.

## Method

- Templates: `db/resources/Entities/Seed/entity_templates*.sql`. A template is a **vendor** if any of `buy_item_list`, `sell_item_list`, `repair_item_list` or `recharge_item_list` is non-null, and a **trainer** if `trainer_ability_list_id` is non-null (the SS-04 rule).
- Spawns: `db/resources/Worlds/Seed/spawnlist*.sql` (table `spawnlist`, columns `spawn_id`, `world_id`, `template_id`, `tag`).
- Buy rows: the number of `item_list_items` rows for the template's `buy_item_list` (`db/resources/Items/Seed/item_list_items.sql` and the `item_lists_*.sql` files). Sell, repair and recharge lists are not counted.
- The Debug Area yard and slope NPCs (templates 1330-1343) have no vendor or trainer list and are not in table 1. World 80 has no vendor or trainer spawn. Omega's Castle Medic (63818) and auctioneer (63819) have no list column either, so they are not in table 1; UO-U1 S7 and S8 check them.
- Tollana's trainer (template 420, spawn 19001) sits in a multi-row `INSERT` in `entity_templates_tollana.sql`, which the rule above does not parse. It is added by hand from `entity_templates_tollana.sql:17` and `spawnlist_tollana.sql:10`. Tollana's food units (template 421, spawns 19002-19004) have no list and are excluded.

## Table 1: every vendor and trainer spawn

Lists are `buy / sell / repair / recharge` (list ids; `-` is NULL) and then `trainer list` where the template has one. Covered by uses README D-SS4: Castle hub to SP-DCA6; Omega to starter-uat-union UO-R5 (row ids from `docs/analysis/union-zone-planning/omega-packets/UO-U1.md`); stasis hub and Debug Area to this campaign's SS-D1 and SS-D2.

| World | Spawn | Tag | Template | Display name | Buy / sell / repair / recharge; trainer list | Buy rows | Covered by (campaign and row id) |
|---|---|---|---|---|---|---|---|
| 8 Castle | 13804 | Castle_HubQuartermaster | 415 | Castle_Quartermaster | 1300 / 2 / 2 / 2 | 19 | starter-uat-praxis SP-DCA6 (hub S1-S5) |
| 12 Castle_CellBlock | 400 | DebugHub_Vendor | 300 | Debug Hub - Vendor | 1 / 2 / 2 / 2 | 3 | SS-D2 (store open, F1 repro) |
| 12 Castle_CellBlock | 401 | DebugHub_Trainer | 301 | Debug Hub - Trainer | -; trainer list 1 | - | SS-D2 (both trainers) |
| 12 Castle_CellBlock | 414 | CraftHub_Supplies | 314 | Debug Hub - Crafting Supplies | 310 / - / - / - | 24 | SS-D2 (buy, sell) |
| 12 Castle_CellBlock | 450 | DebugHub_PetTrainer | 360 | Debug Hub - Pet Trainer | -; trainer list 350 | - | SS-D2 (both trainers) |
| 18 Omega_Site | 63804 | UnionOmega_SoldierTrainer | 63804 | Union Omega - Soldier Trainer | -; trainer list 1 | - | starter-uat-union UO-R5: UO-U1 S6 |
| 18 Omega_Site | 63805 | UnionOmega_ScientistTrainer | 63805 | Union Omega - Scientist Trainer | -; trainer list 1 | - | starter-uat-union UO-R5: UO-U1 S6 |
| 18 Omega_Site | 63806 | UnionOmega_CommandoTrainer | 63806 | Union Omega - Commando Trainer | -; trainer list 1 | - | starter-uat-union UO-R5: UO-U1 S6 |
| 18 Omega_Site | 63807 | UnionOmega_ArchaeologistTrainer | 63807 | Union Omega - Archaeologist Trainer | -; trainer list 1 | - | starter-uat-union UO-R5: UO-U1 S6 |
| 18 Omega_Site | 63808 | UnionOmega_JaffaTrainer | 63808 | Union Omega - Jaffa Trainer | -; trainer list 1 | - | starter-uat-union UO-R5: UO-U1 S6 |
| 18 Omega_Site | 63809 | UnionOmega_AsgardTrainer | 63809 | Union Omega - Asgard Trainer | -; trainer list 1 | - | starter-uat-union UO-R5: UO-U1 A1 (W10, BLOCKED until F42) |
| 18 Omega_Site | 63824 | UnionOmega_AsgardProgramsT1 | 63843 | Union Omega - Asgard Programs T1 | 63821 / 2 / 2 / 2 | 20 | starter-uat-union UO-R5: UO-U1 A2-A3 (W10, BLOCKED until F42) |
| 18 Omega_Site | 63825 | UnionOmega_AsgardProgramsT2 | 63844 | Union Omega - Asgard Programs T2 | 63822 / 2 / 2 / 2 | 20 | starter-uat-union UO-R5: UO-U1 A2-A3 (W10, BLOCKED until F42) |
| 18 Omega_Site | 63826 | UnionOmega_AsgardProgramsT3 | 63845 | Union Omega - Asgard Programs T3 | 63823 / 2 / 2 / 2 | 20 | starter-uat-union UO-R5: UO-U1 A2-A3 (W10, BLOCKED until F42) |
| 18 Omega_Site | 63827 | UnionOmega_AsgardProgramsT4 | 63846 | Union Omega - Asgard Programs T4 | 63824 / 2 / 2 / 2 | 20 | starter-uat-union UO-R5: UO-U1 A2-A3 (W10, BLOCKED until F42) |
| 18 Omega_Site | 63828 | UnionOmega_AsgardArmorT1 | 63847 | Union Omega - Asgard Armor T1 | 63825 / 2 / 2 / 2 | 14 | starter-uat-union UO-R5: UO-U1 A2-A3 (W10, BLOCKED until F42) |
| 18 Omega_Site | 63829 | UnionOmega_AsgardArmorT2 | 63848 | Union Omega - Asgard Armor T2 | 63826 / 2 / 2 / 2 | 14 | starter-uat-union UO-R5: UO-U1 A2-A3 (W10, BLOCKED until F42) |
| 18 Omega_Site | 63830 | UnionOmega_AsgardArmorT3 | 63849 | Union Omega - Asgard Armor T3 | 63827 / 2 / 2 / 2 | 14 | starter-uat-union UO-R5: UO-U1 A2-A3 (W10, BLOCKED until F42) |
| 18 Omega_Site | 63831 | UnionOmega_AsgardArmorT4 | 63850 | Union Omega - Asgard Armor T4 | 63828 / 2 / 2 / 2 | 14 | starter-uat-union UO-R5: UO-U1 A2-A3 (W10, BLOCKED until F42) |
| 18 Omega_Site | 63832 | UnionOmega_AsgardCraftingBiomed | 63851 | Union Omega - Asgard Crafting Biomedical | 63817 / 2 / 2 / 2 | 20 | starter-uat-union UO-R5: UO-U1 A2-A3 (W10, BLOCKED until F42) |
| 18 Omega_Site | 63833 | UnionOmega_AsgardCraftingElec | 63852 | Union Omega - Asgard Crafting Electronics | 63818 / 2 / 2 / 2 | 20 | starter-uat-union UO-R5: UO-U1 A2-A3 (W10, BLOCKED until F42) |
| 18 Omega_Site | 63834 | UnionOmega_AsgardCraftingPowerSys | 63853 | Union Omega - Asgard Crafting Power Systems | 63819 / 2 / 2 / 2 | 20 | starter-uat-union UO-R5: UO-U1 A2-A3 (W10, BLOCKED until F42) |
| 18 Omega_Site | 63835 | UnionOmega_AsgardCraftingMat | 63854 | Union Omega - Asgard Crafting Materials | 63820 / 2 / 2 / 2 | 20 | starter-uat-union UO-R5: UO-U1 A2-A3 (W10, BLOCKED until F42) |
| 18 Omega_Site | 63841 | UnionOmega_HumanWeaponsT1 | 63812 | Union Omega - Human Weapons Tier 1 | 63800 / 2 / 2 / 2 | 20 | starter-uat-union UO-R5: UO-U1 S12 (open); S13 for the six purchase-table items |
| 18 Omega_Site | 63842 | UnionOmega_HumanWeaponsT2 | 63813 | Union Omega - Human Weapons Tier 2 | 63801 / 2 / 2 / 2 | 20 | starter-uat-union UO-R5: UO-U1 S12 (open); S13 for the six purchase-table items |
| 18 Omega_Site | 63843 | UnionOmega_HumanWeaponsT3 | 63814 | Union Omega - Human Weapons Tier 3 | 63802 / 2 / 2 / 2 | 20 | starter-uat-union UO-R5: UO-U1 S12 (open); S13 for the six purchase-table items |
| 18 Omega_Site | 63844 | UnionOmega_HumanWeaponsT4 | 63815 | Union Omega - Human Weapons Tier 4 | 63803 / 2 / 2 / 2 | 20 | starter-uat-union UO-R5: UO-U1 S12 (open); S13 for the six purchase-table items |
| 18 Omega_Site | 63845 | UnionOmega_HumanWeaponsT1_Common | 63812 | Union Omega - Human Weapons Tier 1 | 63800 / 2 / 2 / 2 | 20 | starter-uat-union UO-R5: UO-U1 S12 (open); S13 for the six purchase-table items |
| 18 Omega_Site | 63846 | UnionOmega_HumanWeaponsT4_Common | 63815 | Union Omega - Human Weapons Tier 4 | 63803 / 2 / 2 / 2 | 20 | starter-uat-union UO-R5: UO-U1 S12 (open); S13 for the six purchase-table items |
| 18 Omega_Site | 63847 | UnionOmega_HumanArmorT1 | 63816 | Union Omega - Human Armor Tier 1 | 63804 / 2 / 2 / 2 | 18 | starter-uat-union UO-R5: UO-U1 S12 (open); S13 for the six purchase-table items |
| 18 Omega_Site | 63848 | UnionOmega_HumanArmorT2 | 63817 | Union Omega - Human Armor Tier 2 | 63805 / 2 / 2 / 2 | 18 | starter-uat-union UO-R5: UO-U1 S12 (open); S13 for the six purchase-table items |
| 18 Omega_Site | 63849 | UnionOmega_HumanArmorT3 | 63818 | Union Omega - Human Armor Tier 3 | 63806 / 2 / 2 / 2 | 18 | starter-uat-union UO-R5: UO-U1 S12 (open); S13 for the six purchase-table items |
| 18 Omega_Site | 63850 | UnionOmega_HumanArmorT4 | 63819 | Union Omega - Human Armor Tier 4 | 63807 / 2 / 2 / 2 | 18 | starter-uat-union UO-R5: UO-U1 S12 (open); S13 for the six purchase-table items |
| 18 Omega_Site | 63851 | UnionOmega_HumanArmorT1_Common | 63816 | Union Omega - Human Armor Tier 1 | 63804 / 2 / 2 / 2 | 18 | starter-uat-union UO-R5: UO-U1 S12 (open); S13 for the six purchase-table items |
| 18 Omega_Site | 63852 | UnionOmega_HumanArmorT4_Common | 63819 | Union Omega - Human Armor Tier 4 | 63807 / 2 / 2 / 2 | 18 | starter-uat-union UO-R5: UO-U1 S12 (open); S13 for the six purchase-table items |
| 18 Omega_Site | 63853 | UnionOmega_JaffaWeaponsT1 | 63831 | Union Omega - Jaffa Weapons Tier 1 | 63808 / 2 / 2 / 2 | 20 | starter-uat-union UO-R5: UO-U1 S12 (open); S13 for the six purchase-table items |
| 18 Omega_Site | 63854 | UnionOmega_JaffaWeaponsT2 | 63832 | Union Omega - Jaffa Weapons Tier 2 | 63809 / 2 / 2 / 2 | 20 | starter-uat-union UO-R5: UO-U1 S12 (open); S13 for the six purchase-table items |
| 18 Omega_Site | 63855 | UnionOmega_JaffaWeaponsT3 | 63833 | Union Omega - Jaffa Weapons Tier 3 | 63810 / 2 / 2 / 2 | 20 | starter-uat-union UO-R5: UO-U1 S12 (open); S13 for the six purchase-table items |
| 18 Omega_Site | 63856 | UnionOmega_JaffaWeaponsT4 | 63834 | Union Omega - Jaffa Weapons Tier 4 | 63811 / 2 / 2 / 2 | 20 | starter-uat-union UO-R5: UO-U1 S12 (open); S13 for the six purchase-table items |
| 18 Omega_Site | 63857 | UnionOmega_JaffaWeaponsT1_Common | 63831 | Union Omega - Jaffa Weapons Tier 1 | 63808 / 2 / 2 / 2 | 20 | starter-uat-union UO-R5: UO-U1 S12 (open); S13 for the six purchase-table items |
| 18 Omega_Site | 63859 | UnionOmega_JaffaArmorT1 | 63835 | Union Omega - Jaffa Armor Tier 1 | 63812 / 2 / 2 / 2 | 17 | starter-uat-union UO-R5: UO-U1 S12 (open); S13 for the six purchase-table items |
| 18 Omega_Site | 63860 | UnionOmega_JaffaArmorT2 | 63836 | Union Omega - Jaffa Armor Tier 2 | 63813 / 2 / 2 / 2 | 16 | starter-uat-union UO-R5: UO-U1 S12 (open); S13 for the six purchase-table items |
| 18 Omega_Site | 63861 | UnionOmega_JaffaArmorT3 | 63837 | Union Omega - Jaffa Armor Tier 3 | 63814 / 2 / 2 / 2 | 14 | starter-uat-union UO-R5: UO-U1 S12 (open); S13 for the six purchase-table items |
| 18 Omega_Site | 63862 | UnionOmega_JaffaArmorT4 | 63838 | Union Omega - Jaffa Armor Tier 4 | 63815 / 2 / 2 / 2 | 15 | starter-uat-union UO-R5: UO-U1 S12 (open); S13 for the six purchase-table items |
| 18 Omega_Site | 63863 | UnionOmega_JaffaArmorT1_Common | 63835 | Union Omega - Jaffa Armor Tier 1 | 63812 / 2 / 2 / 2 | 17 | starter-uat-union UO-R5: UO-U1 S12 (open); S13 for the six purchase-table items |
| 18 Omega_Site | 63865 | UnionOmega_Consumables_Common1 | 63828 | Union Omega - Consumables | 63816 / 2 / 2 / 2 | 20 | starter-uat-union UO-R5: UO-U1 S12 (open); S13 for the six purchase-table items |
| 18 Omega_Site | 63866 | UnionOmega_Consumables_Common2 | 63828 | Union Omega - Consumables | 63816 / 2 / 2 / 2 | 20 | starter-uat-union UO-R5: UO-U1 S12 (open); S13 for the six purchase-table items |
| 18 Omega_Site | 63867 | UnionOmega_Consumables_Common3 | 63828 | Union Omega - Consumables | 63816 / 2 / 2 / 2 | 20 | starter-uat-union UO-R5: UO-U1 S12 (open); S13 for the six purchase-table items |
| 18 Omega_Site | 63868 | UnionOmega_Consumables_Sgc1 | 63828 | Union Omega - Consumables | 63816 / 2 / 2 / 2 | 20 | starter-uat-union UO-R5: UO-U1 S12 (open); S13 for the six purchase-table items |
| 18 Omega_Site | 63869 | UnionOmega_Consumables_Sgc2 | 63828 | Union Omega - Consumables | 63816 / 2 / 2 / 2 | 20 | starter-uat-union UO-R5: UO-U1 S12 (open); S13 for the six purchase-table items |
| 18 Omega_Site | 63870 | UnionOmega_Consumables_Sgc3 | 63828 | Union Omega - Consumables | 63816 / 2 / 2 / 2 | 20 | starter-uat-union UO-R5: UO-U1 S12 (open); S13 for the six purchase-table items |
| 18 Omega_Site | 63871 | UnionOmega_Consumables_Jaffa1 | 63828 | Union Omega - Consumables | 63816 / 2 / 2 / 2 | 20 | starter-uat-union UO-R5: UO-U1 S12 (open); S13 for the six purchase-table items |
| 18 Omega_Site | 63872 | UnionOmega_Consumables_Jaffa2 | 63828 | Union Omega - Consumables | 63816 / 2 / 2 / 2 | 20 | starter-uat-union UO-R5: UO-U1 S12 (open); S13 for the six purchase-table items |
| 18 Omega_Site | 63960 | UnionOmega_HumanCraftingBiomed | 63820 | Union Omega - Human Crafting Biomedical | 63817 / 2 / 2 / 2 | 20 | starter-uat-union UO-R5: UO-U1 S12 (open); S13 for the six purchase-table items |
| 18 Omega_Site | 63961 | UnionOmega_HumanCraftingElec | 63821 | Union Omega - Human Crafting Electronics | 63818 / 2 / 2 / 2 | 20 | starter-uat-union UO-R5: UO-U1 S12 (open); S13 for the six purchase-table items |
| 18 Omega_Site | 63962 | UnionOmega_HumanCraftingPowerSys | 63822 | Union Omega - Human Crafting Power Systems | 63819 / 2 / 2 / 2 | 20 | starter-uat-union UO-R5: UO-U1 S12 (open); S13 for the six purchase-table items |
| 18 Omega_Site | 63963 | UnionOmega_HumanCraftingMat | 63823 | Union Omega - Human Crafting Materials | 63820 / 2 / 2 / 2 | 20 | starter-uat-union UO-R5: UO-U1 S12 (open); S13 for the six purchase-table items |
| 18 Omega_Site | 63964 | UnionOmega_CommonCraftingBiomed | 63824 | Union Omega - Common Crafting Biomedical | 63817 / 2 / 2 / 2 | 20 | starter-uat-union UO-R5: UO-U1 S12 (open); S13 for the six purchase-table items |
| 18 Omega_Site | 63965 | UnionOmega_CommonCraftingElec | 63825 | Union Omega - Common Crafting Electronics | 63818 / 2 / 2 / 2 | 20 | starter-uat-union UO-R5: UO-U1 S12 (open); S13 for the six purchase-table items |
| 18 Omega_Site | 63966 | UnionOmega_CommonCraftingPowerSys | 63826 | Union Omega - Common Crafting Power Systems | 63819 / 2 / 2 / 2 | 20 | starter-uat-union UO-R5: UO-U1 S12 (open); S13 for the six purchase-table items |
| 18 Omega_Site | 63967 | UnionOmega_CommonCraftingMat | 63827 | Union Omega - Common Crafting Materials | 63820 / 2 / 2 / 2 | 20 | starter-uat-union UO-R5: UO-U1 S12 (open); S13 for the six purchase-table items |
| 18 Omega_Site | 63968 | UnionOmega_JaffaCraftingBiomed | 63839 | Union Omega - Jaffa Crafting Biomedical | 63817 / 2 / 2 / 2 | 20 | starter-uat-union UO-R5: UO-U1 S12 (open); S13 for the six purchase-table items |
| 18 Omega_Site | 63969 | UnionOmega_JaffaCraftingElec | 63840 | Union Omega - Jaffa Crafting Electronics | 63818 / 2 / 2 / 2 | 20 | starter-uat-union UO-R5: UO-U1 S12 (open); S13 for the six purchase-table items |
| 18 Omega_Site | 63970 | UnionOmega_JaffaCraftingPowerSys | 63841 | Union Omega - Jaffa Crafting Power Systems | 63819 / 2 / 2 / 2 | 20 | starter-uat-union UO-R5: UO-U1 S12 (open); S13 for the six purchase-table items |
| 18 Omega_Site | 63971 | UnionOmega_JaffaCraftingMat | 63842 | Union Omega - Jaffa Crafting Materials | 63820 / 2 / 2 / 2 | 20 | starter-uat-union UO-R5: UO-U1 S12 (open); S13 for the six purchase-table items |
| 19 Tollana | 19001 | Tollana_Copplemann | 420 | Tollana - Capt. Copplemann | trainer list 968 | - | none: no row (tollana-continuation claims template 420, tollana-continuation/w1-packets.md:61) |
| 1300 DebugArea | 13001 | DebugArea_Vendor | 300 | Debug Hub - Vendor | 1 / 2 / 2 / 2 | 3 | SS-D1 (V1, V4-V6, V8, V12) |
| 1300 DebugArea | 13002 | DebugArea_Trainer | 301 | Debug Hub - Trainer | -; trainer list 1 | - | SS-D1 (V10, V11) |
| 1300 DebugArea | 13006 | DebugArea_MunitionsVendor | 1302 | Debug Area - Munitions Vendor | 1300 / 2 / 2 / 2 | 19 | SS-D1 (V2, V3) |
| 1300 DebugArea | 13007 | DebugArea_PetTrainer | 360 | Debug Hub - Pet Trainer | -; trainer list 350 | - | none: no row (pet trainer is not in SS-03a/b) |
| 1300 DebugArea | 13019 | DebugArea_CraftSupplies | 314 | Debug Hub - Crafting Supplies | 310 / - / - / - | 24 | SS-D1 (V7) |

Row ids in the Debug Area and stasis rows are this campaign's rows from [work-packets.md](work-packets.md): V1-V12 (SS-03a, SS-03b) run in SS-D1 at the plaza; SS-D2 runs the stasis hub. The Debug Area pet trainer (13007) and Tollana's trainer (19001) have no row yet.

## Table 2: the D-SS1 evidence

These worlds have no vendor in the seed. Table 2 records what a player could reach and what the zone ledgers say about vendors. Nothing here decides D-SS1; the owner does.

| World | Vendors today | Nearest vendor a player of that faction can reach (world, how) | Ledger mention of vendors (file:line, or "none") |
|---|---|---|---|
| Tollana (19) | None on a vendor list. One trainer: Copplemann, spawn 19001, template 420, trainer list 968 (`spawnlist_tollana.sql:10`) | Not established. The ledger names no route from Tollana to a vendor world. The seeded vendors are in worlds 12, 8, 18 and 1300. | none. `tollana-continuation` has no vendor or shop row; template 420 appears only as the Copplemann claim at `tollana-continuation/w1-packets.md:61` |
| SGC_W1 (58) | None (no spawn in world 58) | Omega (18), through the SGC-to-Omega crossing that `union-sgc-continuation/README.md:49-53` describes. Its arrival is checked by UO-U3. | none. `union-sgc-continuation` has no match for vendor, shop, store or trainer |
| Dakara_E1 (61, 62) | None (the seed has only the DHD in world 61: `dakara-e1-rebuild/audit.md:74`) | Not established. The ledger names crossings from Dakara to SGC_W1 (1651, 1654, `union-sgc-continuation/README.md:51`) and from SGC_W1 to Omega, so Omega is two crossings away. | `dakara-e1-rebuild/README.md:32` ("no vendor data") and `:293` ("Vendors, trainers, ambient population: BLOCKED (no data)") |
| Beta Site E1 (23) | None (no spawn in world 23) | Not established. The ledger names no route from Beta Site E1 to a vendor world. | none. `union-beta-e1-continuation/uat/UB-U2.md:21` is the walk row for the Quartermaster area (Satterfield, template 2302, and Life_QM_1/2, template 2308); neither has a vendor list in the seed. `union-beta-e1-continuation/reward-proposal.md:37` is mission 381 "Closing Shop", a reward row, not a vendor |
| Harset (57, 68, 69, 70) | None (no spawn in worlds 57, 68, 69 or 70) | Not built. Omega (18) is the stated reference for Harset vendors (`harset-continuation/lomiada-brief.md:31`, O18-04). No Harset route to Omega is in the ledger. | `harset-continuation/README.md:226-227` (PU-07 and PU-08, PROVISIONAL drafts, not merged) and `harset-continuation/work-packets.md:70` (HA-15b, BlockedDependency) |

## D-SS2 and OD-5 (Castle hub Quartermaster)

The Castle hub Quartermaster (template 415, spawn 13804, world 8) buys and sells from Debug Area list 1300, so a player pays 1 naquadah per special-ammo stack there. The castle ledger accepts this: "Accepted for now. Flag it for an economy pass before a public release" (`castle-continuation/README.md:66`, repeated at `castle-continuation/RESUME.md:29`). This map does not reopen it. The SP-DCA6 row that owns the hub is in `starter-uat-praxis/work-packets.md:92`; its citation comes from [README.md](README.md) line 39.
