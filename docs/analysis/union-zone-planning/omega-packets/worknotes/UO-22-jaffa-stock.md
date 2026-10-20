# UO-22: Jaffa weapon and armor stock, lists 63808-63815

> **Packet:** [UO-22](../UO-22.md) · **Role:** items-systems-advisor · **Status:** written, no runtime change
> **Consumers:** UO-12 (lists 63808-63811), UO-28 (lists 63812-63815). Every row is CIMMERIA_AUTHORING (D-UO8 B, D-UO29). The lists carry no retail evidence.

## Method

**Source.** Every `INSERT INTO items` row under `db/resources/Items/Seed/` (`items.sql`, 6059 rows, and `ammo_items.sql`, 15 rows): 6074 rows parsed, **0 parse failures**, 0 duplicate `item_id`. The survey script is not committed (it lives under `.claude/reports/`). Its `survey`, `build` and `gaps` commands reproduce every count below.

**Classification (rule 7).**

- Weapon class by name: `Staff` (name ends in ` Staff`), `Zat'ni'katel`, `Ribbon Device`, and heavy. Heavy is `item_id` 5094, 6617 or 4565, or any name with `Rak'tel` or `Jaffa Cannon`.
- A weapon must also have a non-empty `ammo_types` or `max_melee_range > 0`. Row 3849 (Boreal Staff) has a NULL `visual_component` and passes on name and melee range.
- Armor: the name ends in a slot word (Boots, Breastplate, Gauntlets, Helmet, Leggings) and either `visual_component` or `icon_location` contains `AR_J_`, or the name starts with a UO-06 tier-1 family word (Standard, Jackal Melee, Falcon Ranged, Praxis Defensive). The family is the name without the slot word, so `Standard` and `Standard Defensive` are two families.

**Exclusions (rule 6).** Name contains `Test`, `NO ITEM NAME`, `Instrument`, `Mini-game`, `Minigame`, `Program` or `Asgard`. `flags & 1024` must be set. `item_id` 21, 55 and 3437 are out. Any row with `WP-Human.` or `AR_H_` in its visual or icon is out. The `Mini-game`, `Minigame`, `Program` and `Asgard` terms are my addition to the name list, covering mini-game instruments and consumables and Asgard programs. None of them is a weapon or armor candidate.

**Selection.**

- Weapons (rule 9): per tier, each class is sorted by `(tech_comp, item_id)`. Classes go round-robin in the order staff, zat'ni'katel, ribbon, heavy until 20 rows or the designs run out. Price `200 x tech_comp`.
- Heavy at tier 1 is out. Rule 4 limits heavy weapons to tier 2 or higher, and it also applies to Rak'tel-named designs outside the three listed ids. Tier 1 has 16 heavy candidates, all excluded.
- Armor (rule 10): per tier, each family takes one piece per slot, the lowest `tech_comp` then `item_id` for that slot. Families sort by their lowest `tech_comp`, then name. Whole families are taken until the next family would pass 20 rows, and the scan stops there. Price `100 x tech_comp`. Reading "every slot at the family's lowest tech_comp" as per-slot minimum is my interpretation; a family with no piece at its overall lowest tc in a slot still contributes that slot.

**Per tier, eligible against picked.**

| List | Eligible | Picked | Note |
|---|---|---|---|
| 63808 Weapons T1 | staff 51, zat 51, ribbon 51, heavy 0 (16 excluded, rule 4) | 20 | 0 heavy |
| 63809 Weapons T2 | staff 42, zat 42, ribbon 41, heavy 6 | 20 | 5 heavy |
| 63810 Weapons T3 | staff 42, zat 42, ribbon 41, heavy 6 | 20 | 5 heavy |
| 63811 Weapons T4 | staff 21, zat 21, ribbon 21, heavy 3 | 20 | 3 heavy (all of them) |
| 63812 Armor T1 | 5 families, 142 pieces | 17 | stopped at family `Standard Defensive` (22 would pass 20) |
| 63813 Armor T2 | 4 families, 96 pieces | 16 | all families fit |
| 63814 Armor T3 | 3 families, 84 pieces | 14 | all families fit |
| 63815 Armor T4 | 3 families, 45 pieces | 15 | all families fit |

Non-eligible candidates: tier 1 has 2 staffs and 1 zat'ni'katel with `flags` lacking CAN_BE_SOLD, and 4 ribbon devices with no ammo and no melee range. No row matched the Human test.

**Discipline finding (rule 8): the split does not hold for these picks.** The Jaffa range 21-27 is not exclusive to Jaffa. Picked weapons carry discipline sets such as `21,22,23,24,25,27`, `59,60,61,62,63,70`, `78,79,80,81,84,85` and `59,73,75`. Only 6 of 20 tier-1 weapons, 5 of 17 tier-1 armor, and 0 of 16, 14 and 15 armor at tiers 2 to 4 fall entirely within 21-27. The Praxis and Naga armor (`AR_J_Praxis.`, `AR_J_Naga.`) has discipline `40,41,42,43,49`, which is the Human range. Classification therefore rests on name and `AR_J_` visual, not on `discipline_ids`. The discipline sets are recorded per list in the build output.

**Parse-failure count:** 0 of 6074.

## Parameters

- Naquadah: weapons `200 x tech_comp`, armor `100 x tech_comp`. No price column exists, so every price is a formula with no retail evidence.
- Item ids: `638000 + (list - 63800) x 50 + k`, `k` in row order. Row order is selection order: round-robin for weapons, families then slot order (Boots, Breastplate, Gauntlets, Helmet, Leggings) for armor.
- UO-06's lists 63807 and 63808 are superseded. Their tier-1 rows are candidates only, not picks. The picks here are chosen by rule 9 and rule 10, not copied from those lists.

## SQL

### For UO-12

```sql
-- CIMMERIA_AUTHORING (D-UO8 B, D-UO29): Jaffa Weapons tier 1, 200 x tech_comp
INSERT INTO item_lists (item_list_id, name) VALUES (63808, 'Omega Jaffa Weapons Tier 1');
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638400, 63808, 2797, 1, 200); -- Serpent Staff, staff, tc 1
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638401, 63808, 5141, 1, 200); -- Discharge Zat'ni'katel, zat, tc 1
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638402, 63808, 4566, 1, 600); -- Serpent Ribbon Device, ribbon, tc 3
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638403, 63808, 3470, 1, 600); -- Serpent Staff, staff, tc 3
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638404, 63808, 5140, 1, 600); -- Discharge Zat'ni'katel, zat, tc 3
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638405, 63808, 4458, 1, 1000); -- Discharge Ribbon Device, ribbon, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638406, 63808, 3471, 1, 1000); -- Serpent Staff, staff, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638407, 63808, 3904, 1, 1000); -- Shriek Zat'ni'katel, zat, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638408, 63808, 4487, 1, 1000); -- Radiant Ribbon Device, ribbon, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638409, 63808, 3744, 1, 1000); -- Decay Staff, staff, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638410, 63808, 3908, 1, 1000); -- Discharge Zat'ni'katel, zat, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638411, 63808, 4513, 1, 1000); -- Boreal Ribbon Device, ribbon, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638412, 63808, 3773, 1, 1000); -- Discharge Staff, staff, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638413, 63808, 3940, 1, 1000); -- Radiant Zat'ni'katel, zat, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638414, 63808, 4540, 1, 1000); -- Pilgrim Ribbon Device, ribbon, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638415, 63808, 3792, 1, 1000); -- Shriek Staff, staff, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638416, 63808, 3965, 1, 1000); -- Boreal Zat'ni'katel, zat, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638417, 63808, 4567, 1, 1000); -- Serpent Ribbon Device, ribbon, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638418, 63808, 3818, 1, 1000); -- Radiant Staff, staff, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638419, 63808, 3977, 1, 1000); -- Decay Zat'ni'katel, zat, tc 5
```

```sql
-- CIMMERIA_AUTHORING (D-UO8 B, D-UO29): Jaffa Weapons tier 2, 200 x tech_comp
INSERT INTO item_lists (item_list_id, name) VALUES (63809, 'Omega Jaffa Weapons Tier 2');
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638450, 63809, 3746, 1, 4000); -- Venom Staff, staff, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638451, 63809, 3916, 1, 4000); -- Clarion Zat'ni'katel, zat, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638452, 63809, 4466, 1, 4000); -- Spark Ribbon Device, ribbon, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638453, 63809, 6638, 1, 4000); -- Venom Jaffa Cannon, heavy, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638454, 63809, 3755, 1, 4000); -- Disjunct Staff, staff, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638455, 63809, 3925, 1, 4000); -- Spark Zat'ni'katel, zat, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638456, 63809, 4494, 1, 4000); -- Flame Ribbon Device, ribbon, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638457, 63809, 6639, 1, 4600); -- Venom Jaffa Cannon, heavy, tc 23
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638458, 63809, 3785, 1, 4000); -- Spark Staff, staff, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638459, 63809, 3947, 1, 4000); -- Flame Zat'ni'katel, zat, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638460, 63809, 4522, 1, 4000); -- Gelid Ribbon Device, ribbon, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638461, 63809, 6640, 1, 5000); -- Venom Jaffa Cannon, heavy, tc 25
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638462, 63809, 3808, 1, 4000); -- Clarion Staff, staff, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638463, 63809, 3978, 1, 4000); -- Gelid Zat'ni'katel, zat, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638464, 63809, 4547, 1, 4000); -- Warrior Ribbon Device, ribbon, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638465, 63809, 6641, 1, 5600); -- Venom Jaffa Cannon, heavy, tc 28
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638466, 63809, 3826, 1, 4000); -- Flame Staff, staff, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638467, 63809, 3993, 1, 4000); -- Disjunct Zat'ni'katel, zat, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638468, 63809, 4583, 1, 4000); -- Clarion Ribbon Device, ribbon, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638469, 63809, 6642, 1, 6000); -- Venom Jaffa Cannon, heavy, tc 30
```

```sql
-- CIMMERIA_AUTHORING (D-UO8 B, D-UO29): Jaffa Weapons tier 3, 200 x tech_comp
INSERT INTO item_lists (item_list_id, name) VALUES (63810, 'Omega Jaffa Weapons Tier 3');
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638500, 63810, 3753, 1, 6600); -- Cobra Staff, staff, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638501, 63810, 3924, 1, 6600); -- Stentorian Zat'ni'katel, zat, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638502, 63810, 4472, 1, 6600); -- Surge Ribbon Device, ribbon, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638503, 63810, 6644, 1, 6600); -- Cobra Jaffa Cannon, heavy, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638504, 63810, 3768, 1, 6600); -- Degenerate Staff, staff, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638505, 63810, 3931, 1, 6600); -- Surge Zat'ni'katel, zat, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638506, 63810, 4500, 1, 6600); -- Star Ribbon Device, ribbon, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638507, 63810, 6645, 1, 7000); -- Cobra Jaffa Cannon, heavy, tc 35
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638508, 63810, 3793, 1, 6600); -- Surge Staff, staff, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638509, 63810, 3953, 1, 6600); -- Star Zat'ni'katel, zat, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638510, 63810, 4528, 1, 6600); -- Negation Ribbon Device, ribbon, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638511, 63810, 6646, 1, 7600); -- Cobra Jaffa Cannon, heavy, tc 38
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638512, 63810, 3814, 1, 6600); -- Stentorian Staff, staff, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638513, 63810, 3991, 1, 6600); -- Negation Zat'ni'katel, zat, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638514, 63810, 4553, 1, 6600); -- Battle Ribbon Device, ribbon, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638515, 63810, 6647, 1, 8000); -- Cobra Jaffa Cannon, heavy, tc 40
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638516, 63810, 3832, 1, 6600); -- Star Staff, staff, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638517, 63810, 4004, 1, 6600); -- Degeneration Zat'ni'katel, zat, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638518, 63810, 4589, 1, 6600); -- Stentorian Ribbon Device, ribbon, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638519, 63810, 6648, 1, 8600); -- Cobra Jaffa Cannon, heavy, tc 43
```

```sql
-- CIMMERIA_AUTHORING (D-UO8 B, D-UO29): Jaffa Weapons tier 4, 200 x tech_comp
INSERT INTO item_lists (item_list_id, name) VALUES (63811, 'Omega Jaffa Weapons Tier 4');
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638550, 63811, 3760, 1, 9000); -- Asp Staff, staff, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638551, 63811, 3937, 1, 9000); -- Bolt Zat'ni'katel, zat, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638552, 63811, 4478, 1, 9000); -- Bolt Ribbon Device, ribbon, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638553, 63811, 6650, 1, 9000); -- Asp Jaffa Cannon, heavy, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638554, 63811, 3782, 1, 9000); -- Toxic Staff, staff, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638555, 63811, 3959, 1, 9000); -- Sun Zat'ni'katel, zat, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638556, 63811, 4506, 1, 9000); -- Sun Ribbon Device, ribbon, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638557, 63811, 6651, 1, 9600); -- Asp Jaffa Cannon, heavy, tc 48
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638558, 63811, 3803, 1, 9000); -- Bolt Staff, staff, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638559, 63811, 3970, 1, 9000); -- Thunder Zat'ni'katel, zat, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638560, 63811, 4534, 1, 9000); -- Vitiate Ribbon Device, ribbon, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638561, 63811, 6652, 1, 10000); -- Asp Jaffa Cannon, heavy, tc 50
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638562, 63811, 3837, 1, 9000); -- Sun Staff, staff, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638563, 63811, 4003, 1, 9000); -- Vitiate Zat'ni'katel, zat, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638564, 63811, 4560, 1, 9000); -- Master Ribbon Device, ribbon, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638565, 63811, 3844, 1, 9000); -- Thunder Staff, staff, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638566, 63811, 4012, 1, 9000); -- Toxic Zat'ni'katel, zat, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638567, 63811, 4595, 1, 9000); -- Thunder Ribbon Device, ribbon, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638568, 63811, 3874, 1, 9000); -- Vitiate Staff, staff, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638569, 63811, 4058, 1, 9000); -- Asp Zat'ni'katel, zat, tc 45
```

### For UO-28

```sql
-- CIMMERIA_AUTHORING (D-UO8 B, D-UO29): Jaffa Armor tier 1, 100 x tech_comp
INSERT INTO item_lists (item_list_id, name) VALUES (63812, 'Omega Jaffa Armor Tier 1');
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638600, 63812, 4340, 1, 100); -- Standard Boots, Standard armor Boots, tc 1
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638601, 63812, 4341, 1, 100); -- Standard Leggings, Standard armor Leggings, tc 1
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638602, 63812, 6375, 1, 500); -- Falcon Ranged Boots, Falcon Ranged armor Boots, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638603, 63812, 6349, 1, 500); -- Falcon Ranged Breastplate, Falcon Ranged armor Breastplate, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638604, 63812, 6401, 1, 500); -- Falcon Ranged Gauntlets, Falcon Ranged armor Gauntlets, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638605, 63812, 6427, 1, 500); -- Falcon Ranged Helmet, Falcon Ranged armor Helmet, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638606, 63812, 6454, 1, 500); -- Falcon Ranged Leggings, Falcon Ranged armor Leggings, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638607, 63812, 6235, 1, 500); -- Jackal Melee Boots, Jackal Melee armor Boots, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638608, 63812, 6208, 1, 500); -- Jackal Melee Breastplate, Jackal Melee armor Breastplate, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638609, 63812, 6270, 1, 500); -- Jackal Melee Gauntlets, Jackal Melee armor Gauntlets, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638610, 63812, 6297, 1, 500); -- Jackal Melee Helmet, Jackal Melee armor Helmet, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638611, 63812, 6323, 1, 500); -- Jackal Melee Leggings, Jackal Melee armor Leggings, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638612, 63812, 6059, 1, 500); -- Praxis Defensive Boots, Praxis Defensive armor Boots, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638613, 63812, 5976, 1, 500); -- Praxis Defensive Breastplate, Praxis Defensive armor Breastplate, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638614, 63812, 6107, 1, 500); -- Praxis Defensive Gauntlets, Praxis Defensive armor Gauntlets, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638615, 63812, 6140, 1, 500); -- Praxis Defensive Helmet, Praxis Defensive armor Helmet, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638616, 63812, 6172, 1, 500); -- Praxis Defensive Leggings, Praxis Defensive armor Leggings, tc 5
```

```sql
-- CIMMERIA_AUTHORING (D-UO8 B, D-UO29): Jaffa Armor tier 2, 100 x tech_comp
INSERT INTO item_lists (item_list_id, name) VALUES (63813, 'Omega Jaffa Armor Tier 2');
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638650, 63813, 6382, 1, 2000); -- Asian Ranged Boots, Asian Ranged armor Boots, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638651, 63813, 6356, 1, 2000); -- Asian Ranged Breastplate, Asian Ranged armor Breastplate, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638652, 63813, 6408, 1, 2000); -- Asian Ranged Gauntlets, Asian Ranged armor Gauntlets, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638653, 63813, 6434, 1, 2000); -- Asian Ranged Helmet, Asian Ranged armor Helmet, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638654, 63813, 6461, 1, 2000); -- Asian Ranged Leggings, Asian Ranged armor Leggings, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638655, 63813, 6246, 1, 2000); -- Cat Melee Boots, Cat Melee armor Boots, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638656, 63813, 6216, 1, 2000); -- Cat Melee Breastplate, Cat Melee armor Breastplate, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638657, 63813, 6277, 1, 2000); -- Cat Melee Gauntlets, Cat Melee armor Gauntlets, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638658, 63813, 6304, 1, 2000); -- Cat Melee Helmet, Cat Melee armor Helmet, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638659, 63813, 6330, 1, 2000); -- Cat Melee Leggings, Cat Melee armor Leggings, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638660, 63813, 6073, 1, 2000); -- Horse Defensive Boots, Horse Defensive armor Boots, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638661, 63813, 6025, 1, 2000); -- Horse Defensive Breastplate, Horse Defensive armor Breastplate, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638662, 63813, 6121, 1, 2000); -- Horse Defensive Gauntlets, Horse Defensive armor Gauntlets, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638663, 63813, 6154, 1, 2000); -- Horse Defensive Helmet, Horse Defensive armor Helmet, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638664, 63813, 6186, 1, 2000); -- Horse Defensive Leggings, Horse Defensive armor Leggings, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638665, 63813, 4377, 1, 2000); -- Veiled Stealth Boots, Veiled Stealth armor Boots, tc 20
```

```sql
-- CIMMERIA_AUTHORING (D-UO8 B, D-UO29): Jaffa Armor tier 3, 100 x tech_comp
INSERT INTO item_lists (item_list_id, name) VALUES (63814, 'Omega Jaffa Armor Tier 3');
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638700, 63814, 6079, 1, 3300); -- Bull Defensive Boots, Bull Defensive armor Boots, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638701, 63814, 6046, 1, 3300); -- Bull Defensive Breastplate, Bull Defensive armor Breastplate, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638702, 63814, 6127, 1, 3300); -- Bull Defensive Gauntlets, Bull Defensive armor Gauntlets, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638703, 63814, 6160, 1, 3300); -- Bull Defensive Helmet, Bull Defensive armor Helmet, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638704, 63814, 6193, 1, 3300); -- Bull Defensive Leggings, Bull Defensive armor Leggings, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638705, 63814, 6388, 1, 3300); -- Dragon Ranged Boots, Dragon Ranged armor Boots, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638706, 63814, 6362, 1, 3300); -- Dragon Ranged Breastplate, Dragon Ranged armor Breastplate, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638707, 63814, 6414, 1, 3300); -- Dragon Ranged Gauntlets, Dragon Ranged armor Gauntlets, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638708, 63814, 6440, 1, 3300); -- Dragon Ranged Helmet, Dragon Ranged armor Helmet, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638709, 63814, 6256, 1, 3300); -- Mayan Melee Boots, Mayan Melee armor Boots, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638710, 63814, 6222, 1, 3300); -- Mayan Melee Breastplate, Mayan Melee armor Breastplate, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638711, 63814, 6283, 1, 3300); -- Mayan Melee Gauntlets, Mayan Melee armor Gauntlets, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638712, 63814, 6310, 1, 3300); -- Mayan Melee Helmet, Mayan Melee armor Helmet, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638713, 63814, 6336, 1, 3300); -- Mayan Melee Leggings, Mayan Melee armor Leggings, tc 33
```

```sql
-- CIMMERIA_AUTHORING (D-UO8 B, D-UO29): Jaffa Armor tier 4, 100 x tech_comp
INSERT INTO item_lists (item_list_id, name) VALUES (63815, 'Omega Jaffa Armor Tier 4');
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638750, 63815, 6263, 1, 4500); -- Hyena Melee Boots, Hyena Melee armor Boots, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638751, 63815, 6228, 1, 4500); -- Hyena Melee Breastplate, Hyena Melee armor Breastplate, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638752, 63815, 6289, 1, 4500); -- Hyena Melee Gauntlets, Hyena Melee armor Gauntlets, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638753, 63815, 6316, 1, 4500); -- Hyena Melee Helmet, Hyena Melee armor Helmet, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638754, 63815, 6342, 1, 4500); -- Hyena Melee Leggings, Hyena Melee armor Leggings, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638755, 63815, 6394, 1, 4500); -- Naga Ranged Boots, Naga Ranged armor Boots, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638756, 63815, 6368, 1, 4500); -- Naga Ranged Breastplate, Naga Ranged armor Breastplate, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638757, 63815, 6420, 1, 4500); -- Naga Ranged Gauntlets, Naga Ranged armor Gauntlets, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638758, 63815, 6446, 1, 4500); -- Naga Ranged Helmet, Naga Ranged armor Helmet, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638759, 63815, 6473, 1, 4500); -- Naga Ranged Leggings, Naga Ranged armor Leggings, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638760, 63815, 6085, 1, 4500); -- Viking Defensive Boots, Viking Defensive armor Boots, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638761, 63815, 6053, 1, 4500); -- Viking Defensive Breastplate, Viking Defensive armor Breastplate, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638762, 63815, 6134, 1, 4500); -- Viking Defensive Gauntlets, Viking Defensive armor Gauntlets, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638763, 63815, 6166, 1, 4500); -- Viking Defensive Helmet, Viking Defensive armor Helmet, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638764, 63815, 6200, 1, 4500); -- Viking Defensive Leggings, Viking Defensive armor Leggings, tc 45
```

## Checks

| List | Rows | Tier | Price rule | CAN_BE_SOLD | Arbitrage (sellable designs) | Heavy weapons | Checks |
|---|---|---|---|---|---|---|---|
| 63808 Omega Jaffa Weapons Tier 1 | 20 | 1 | 200 x tc | pass | 20 | 0 | all pass |
| 63809 Omega Jaffa Weapons Tier 2 | 20 | 2 | 200 x tc | pass | 20 | 5 | all pass |
| 63810 Omega Jaffa Weapons Tier 3 | 20 | 3 | 200 x tc | pass | 20 | 5 | all pass |
| 63811 Omega Jaffa Weapons Tier 4 | 20 | 4 | 200 x tc | pass | 20 | 3 | all pass |
| 63812 Omega Jaffa Armor Tier 1 | 17 | 1 | 100 x tc | pass | 17 | 0 | all pass |
| 63813 Omega Jaffa Armor Tier 2 | 16 | 2 | 100 x tc | pass | 16 | 0 | all pass |
| 63814 Omega Jaffa Armor Tier 3 | 14 | 3 | 100 x tc | pass | 14 | 0 | all pass |
| 63815 Omega Jaffa Armor Tier 4 | 15 | 4 | 100 x tc | pass | 15 | 0 | all pass |

## Excluded

- Tier 1 heavy weapons, 16 candidates (Rak'tel and heavy class). Rule 4: heavy only at tier 2 or higher.
- Tier 1 `not sellable` (no CAN_BE_SOLD): 2 Staff and 1 Zat'ni'katel.
- Tier 1 ribbon devices with no ammo and no melee range: 4.
- Armor families cut by the whole-family rule: tier 1 `Standard Defensive` (5 slots, would take the list to 22).
- Mini-game instruments and consumables, Jaffa-named but not weapons or armor (for example `Jaffa Mini-game Instrument`, `Blueprint: Jaffa Minigame Consumable`): excluded by the name rule.
- Human designs: none matched the `WP-Human.` or `AR_H_` test among the candidates.
- Non-candidate Jaffa-named rows (`Jaffa Apparatus Parts`, `Jaffa Disguise`) are neither weapon nor armor.

## Gaps

- **63808 (tier 1 heavy):** no heavy weapon is in this list. Tier 1 has no eligible heavy weapon under rule 4.
- **63812 (armor tier 1):** 17 rows, not 20. The next family (`Standard Defensive`, 5 slots) would pass 20.
- **63813, 63814, 63815 (armor tiers 2-4):** 16, 14 and 15 rows. Each tier has only 3 or 4 armor families, so the whole-family rule takes all of them and the lists stay short. The architect should decide whether to fill them from a family outside the rule.
- **No tier 5 list** (D-UO28). Tier 5 candidates exist in the seed (weapons and armor) and are not stocked.
- **Discipline split:** rule 8 does not hold (see Method). Classification uses names and `AR_J_` visuals. The architect should confirm that the name and visual rule is the intended one.

## Store-open proof

Expected `buy_count` per list equals the row count, if the store takes one buy item per list row (`store.rs:140`, `buy_count = buy_items.len()`). Not run: no lab.

| List | Expected buy_count |
|---|---|
| 63808 | 20 |
| 63809 | 20 |
| 63810 | 20 |
| 63811 | 20 |
| 63812 | 17 |
| 63813 | 16 |
| 63814 | 14 |
| 63815 | 15 |
