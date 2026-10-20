# UO-23: sealed consumable and crafting stock (worknote)

> **Packet:** [UO-23](../UO-23.md) · **Role:** items-systems-advisor · **Wave:** W7
> **Sources:** [UO-06 worknote](UO-06-services.md) (§ Evidence and limits, § Lists, Q3); `db/resources/Items/Seed/items.sql` (and the other seed files, parsed for `INSERT INTO items`).
> **Provenance:** every row is CIMMERIA_AUTHORING (D-UO8 B, D-UO29). No seed or code changed.

## Method

**Parse.** A quote-aware reader (`''` escapes, `'{...}'` arrays, `NULL`) read all 6074 `INSERT INTO items` rows across `db/resources/Items/Seed/*.sql`. Failed to parse: 0 (no ids). No duplicate `item_id`. The UO-06 count of 101 unparsed rows was not reproduced by this reader.

**Eligibility (rules 2 and 5).** A design is eligible when `flags & 1024` (CAN_BE_SOLD) is set, it is not 21, 55 or 3437, its name has no `Test`, `NO ITEM NAME` or `Instrument`, and its `visual_component` does not start with `WP`, `AR_`, `AR-` or `ACC_` (weapons and armor). No Asgard program or mini-game item is in any pool: the pools are built from the name patterns below, and the only Asgard row that matches a pool pattern is `Asgard Astrocyte (Bio-Medical)`, a component, which is not picked.

**Consumables (rule 6).** Families by name, non-blueprint only:
- Slappack: name contains `Slappack`.
- Antidote: name ends `Antidote` (so `Antidote Darts` and `Antidote Administer` are not Antidotes).
- Stimpack: name contains `Stimpack`.
- `Blueprint:` rows are crafting blueprints, not consumables, and are not stocked (289 rows, 288 of them CAN_BE_SOLD).
- Any other consumable family is not stocked (rule 3).

Selection: UO-06's ten tier-1 rows (2893, 6657, 6659, 6656, 6677-6682), each re-checked. Then the eligible family rows above tier 1, sorted by `tier`, `tech_comp`, `item_id`, until 20 rows. Result: 10 + 10 = 20.

**Crafting (rule 7).** Per specialty, the pool is:
- the research kit, named `Beta <X> Research Kit` (`Beta Biomedical Research Kit` 5447, `Beta Electronics Research Kit` 8887, `Beta Power Systems Research Kit` 8888, `Beta Materials Research Kit` 8889);
- the field crafting tools, named `<Prefix>-<n> Field Crafting Tool`, one per `tc` 5, 10, 15, ... 55 (11 each);
- resources and subcombines, named `(<Specialty>)` or `(<Specialty> Subcombine <X>)`. The Bio-Medical specialty is written `Bio-Medical` in item names.

Power Systems tool prefix: `PSAS` (8405, 8453-8462; `PSAS-5` is 8405). The other prefixes are `EAS` (Electronics), `MAS` (Materials) and `BMAS` (Biomedical). `Efficient <prefix>-35 Field Crafting Tool` rows (8417, 8463, 8465, 8466) do not start with the prefix and are not picked.

Order: the kit and the 11 tools first, by `tech_comp` then `item_id` (12 rows). Then the resources and subcombines by `tech_comp` then `item_id`, until 20 rows (8 rows).

**Prices (rules 3 and 4).** Tier-1 consumables: Slappack 25, Antidote 50, Stimpack 100. Every other row: 25 × `tech_comp`. There is no price column; every price is a formula (CIMMERIA_AUTHORING).

**Eligible against picked, per list.**

| List | Eligible | Picked | Notes |
|---|---|---|---|
| 63816 Omega Consumables | 11 tier-1 family rows; 28 family rows above tier 1 | 20 | 10 UO-06 tier-1 rows, then 10 above tier 1 |
| 63817 Omega Crafting Biomedical | 89 (12 kit and tools, 77 resources and subcombines) | 20 | 12 kit and tools, 8 resources |
| 63818 Omega Crafting Electronics | 58 (12 and 46) | 20 | 12 kit and tools, 8 resources |
| 63819 Omega Crafting Power Systems | 92 (12 and 80) | 20 | 12 kit and tools, 8 resources |
| 63820 Omega Crafting Materials | 80 (12 and 68) | 20 | 12 kit and tools, 8 resources |

The Power Systems list exists.

## SQL

Each block is one list. Item ids follow D-UO27: `638000 + (list − 63800) × 50 + k`. Every row has `quantity` 1.

### For UO-30

```sql
-- CIMMERIA_AUTHORING (D-UO8 B, D-UO29): Consumables, T1 25/50/100, above T1 25 x tech_comp
INSERT INTO item_lists (item_list_id, name) VALUES (63816, 'Omega Consumables');
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638800, 63816, 2893, 1, 25); -- Health Slappack TC1, tier 1, tc 1
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638801, 63816, 6657, 1, 50); -- Health Antidote, tier 1, tc 10
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638802, 63816, 6659, 1, 50); -- Health Antidote, tier 1, tc 10
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638803, 63816, 6656, 1, 50); -- Kinetic Antidote, tier 1, tc 10
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638804, 63816, 6677, 1, 100); -- Mark III Stimpack: Coordination, tier 1, tc 10
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638805, 63816, 6678, 1, 100); -- Mark III Stimpack: Engagement, tier 1, tc 10
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638806, 63816, 6679, 1, 100); -- Mark III Stimpack: Fortitude, tier 1, tc 10
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638807, 63816, 6680, 1, 100); -- Mark III Stimpack: Intellect, tier 1, tc 10
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638808, 63816, 6681, 1, 100); -- Mark III Stimpack: Morale, tier 1, tc 10
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638809, 63816, 6682, 1, 100); -- Mark III Stimpack: Perception, tier 1, tc 10
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638810, 63816, 6577, 1, 500); -- Kinetic/Mental Antidote, tier 2, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638811, 63816, 6597, 1, 500); -- Health/Mental Antidote, tier 2, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638812, 63816, 6598, 1, 500); -- Health/Kinetic Antidote, tier 2, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638813, 63816, 6660, 1, 625); -- Double Health/Kinetic Antidote, tier 2, tc 25
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638814, 63816, 6661, 1, 625); -- Double Health/Mental Antidote, tier 2, tc 25
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638815, 63816, 6662, 1, 625); -- Double Kinetic/Mental Antidote, tier 2, tc 25
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638816, 63816, 6697, 1, 625); -- Mark V Stimpack: Coordination/Engagement, tier 2, tc 25
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638817, 63816, 6719, 1, 625); -- Mark V Stimpack: Engagement/Perception, tier 2, tc 25
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638818, 63816, 6722, 1, 625); -- Mark V Stimpack: Fortitude/Coordination, tier 2, tc 25
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638819, 63816, 6725, 1, 625); -- Mark V Stimpack: Intellect/Morale, tier 2, tc 25
```

### For UO-29

```sql
-- CIMMERIA_AUTHORING (D-UO8 B, D-UO29): Crafting Bio-Medical, 25 x tech_comp
INSERT INTO item_lists (item_list_id, name) VALUES (63817, 'Omega Crafting Biomedical');
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638850, 63817, 5369, 1, 125); -- BMAS-5 Field Crafting Tool, tier 1, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638851, 63817, 5447, 1, 125); -- Beta Biomedical Research Kit, tier 1, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638852, 63817, 8407, 1, 125); -- BMAS-10 Field Crafting Tool, tier 1, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638853, 63817, 8408, 1, 375); -- BMAS-15 Field Crafting Tool, tier 1, tc 15
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638854, 63817, 8409, 1, 500); -- BMAS-20 Field Crafting Tool, tier 2, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638855, 63817, 8410, 1, 625); -- BMAS-25 Field Crafting Tool, tier 2, tc 25
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638856, 63817, 8411, 1, 750); -- BMAS-30 Field Crafting Tool, tier 2, tc 30
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638857, 63817, 8412, 1, 875); -- BMAS-35 Field Crafting Tool, tier 3, tc 35
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638858, 63817, 8413, 1, 1000); -- BMAS-40 Field Crafting Tool, tier 3, tc 40
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638859, 63817, 8414, 1, 1125); -- BMAS-45 Field Crafting Tool, tier 4, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638860, 63817, 8415, 1, 1250); -- BMAS-50 Field Crafting Tool, tier 5, tc 50
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638861, 63817, 8416, 1, 1375); -- BMAS-55 Field Crafting Tool, tier 5, tc 55
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638862, 63817, 5188, 1, 75); -- Protein Complex (Bio-Medical), tier 1, tc 3
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638863, 63817, 5189, 1, 125); -- T1 Cell (Bio-Medical), tier 1, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638864, 63817, 5360, 1, 125); -- Rich Substrate (Bio-Medical Subcombine A), tier 1, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638865, 63817, 5362, 1, 125); -- Strombine (Bio-Medical Subcombine A), tier 1, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638866, 63817, 5363, 1, 200); -- Xeno-bacterium (Bio-Medical Subcombine B), tier 1, tc 8
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638867, 63817, 5364, 1, 200); -- Chiral Lysine (Bio-Medical Subcombine B), tier 1, tc 8
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638868, 63817, 5366, 1, 275); -- X-Cosmid (Bio-Medical Subcombine C), tier 1, tc 11
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638869, 63817, 5367, 1, 275); -- Mimic Rhinovirus (Bio-Medical Subcombine C), tier 1, tc 11
```

```sql
-- CIMMERIA_AUTHORING (D-UO8 B, D-UO29): Crafting Electronics, 25 x tech_comp
INSERT INTO item_lists (item_list_id, name) VALUES (63818, 'Omega Crafting Electronics');
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638900, 63818, 8402, 1, 125); -- EAS-5 Field Crafting Tool, tier 1, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638901, 63818, 8887, 1, 125); -- Beta Electronics Research Kit, tier 1, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638902, 63818, 8433, 1, 250); -- EAS-10 Field Crafting Tool, tier 1, tc 10
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638903, 63818, 8434, 1, 375); -- EAS-15 Field Crafting Tool, tier 1, tc 15
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638904, 63818, 8435, 1, 500); -- EAS-20 Field Crafting Tool, tier 2, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638905, 63818, 8436, 1, 625); -- EAS-25 Field Crafting Tool, tier 2, tc 25
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638906, 63818, 8437, 1, 750); -- EAS-30 Field Crafting Tool, tier 2, tc 30
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638907, 63818, 8438, 1, 875); -- EAS-35 Field Crafting Tool, tier 3, tc 35
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638908, 63818, 8439, 1, 1000); -- EAS-40 Field Crafting Tool, tier 4, tc 40
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638909, 63818, 8440, 1, 1125); -- EAS-45 Field Crafting Tool, tier 4, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638910, 63818, 8441, 1, 1250); -- EAS-50 Field Crafting Tool, tier 5, tc 50
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638911, 63818, 8442, 1, 1375); -- EAS-55 Field Crafting Tool, tier 5, tc 55
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638912, 63818, 5224, 1, 75); -- Integrated Circuit (Electronics), tier 1, tc 3
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638913, 63818, 5225, 1, 125); -- Integrated Psuedocore (Electronics), tier 1, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638914, 63818, 5373, 1, 125); -- IC-K-layer (Electronics Subcombine A), tier 1, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638915, 63818, 5374, 1, 125); -- Output Processor (Electronics Subcombine A), tier 1, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638916, 63818, 5375, 1, 125); -- Minimatrix (Electronics Subcombine A), tier 1, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638917, 63818, 5376, 1, 200); -- Signal Damp (Electronics Subcombine B), tier 1, tc 8
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638918, 63818, 5377, 1, 200); -- Optical Rectifier (Electronics Subcombine B), tier 1, tc 8
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638919, 63818, 5378, 1, 200); -- Cell Blade (Electronics Subcombine B), tier 1, tc 8
```

```sql
-- CIMMERIA_AUTHORING (D-UO8 B, D-UO29): Crafting Power Systems, 25 x tech_comp
INSERT INTO item_lists (item_list_id, name) VALUES (63819, 'Omega Crafting Power Systems');
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638950, 63819, 8405, 1, 125); -- PSAS-5 Field Crafting Tool, tier 1, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638951, 63819, 8888, 1, 125); -- Beta Power Systems Research Kit, tier 1, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638952, 63819, 8453, 1, 250); -- PSAS-10 Field Crafting Tool, tier 1, tc 10
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638953, 63819, 8454, 1, 375); -- PSAS-15 Field Crafting Tool, tier 1, tc 15
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638954, 63819, 8455, 1, 500); -- PSAS-20 Field Crafting Tool, tier 2, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638955, 63819, 8456, 1, 625); -- PSAS-25 Field Crafting Tool, tier 2, tc 25
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638956, 63819, 8457, 1, 750); -- PSAS-30 Field Crafting Tool, tier 2, tc 30
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638957, 63819, 8458, 1, 875); -- PSAS-35 Field Crafting Tool, tier 3, tc 35
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638958, 63819, 8459, 1, 1000); -- PSAS-40 Field Crafting Tool, tier 3, tc 40
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638959, 63819, 8460, 1, 1125); -- PSAS-45 Field Crafting Tool, tier 4, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638960, 63819, 8461, 1, 1250); -- PSAS-50 Field Crafting Tool, tier 5, tc 50
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638961, 63819, 8462, 1, 1375); -- PSAS-55 Field Crafting Tool, tier 5, tc 55
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638962, 63819, 5257, 1, 75); -- Wave Guide (Power Systems), tier 1, tc 3
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638963, 63819, 5259, 1, 125); -- Particle Guide (Power Systems), tier 1, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638964, 63819, 5385, 1, 125); -- Optical Fiber (Power Systems Subcombine A), tier 1, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638965, 63819, 5386, 1, 125); -- Fehu Particle Accelerator (Power Systems Subcombine A), tier 1, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638966, 63819, 5387, 1, 125); -- Ring Turbine (Power Systems Subcombine A), tier 1, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638967, 63819, 5388, 1, 200); -- Particle Dynamo (Power Systems Subcombine B), tier 1, tc 8
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638968, 63819, 5389, 1, 200); -- Counterspin Net (Power Systems Subcombine B), tier 1, tc 8
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638969, 63819, 5390, 1, 200); -- Epicycle Multiplier (Power Systems Subcombine B), tier 1, tc 8
```

```sql
-- CIMMERIA_AUTHORING (D-UO8 B, D-UO29): Crafting Materials, 25 x tech_comp
INSERT INTO item_lists (item_list_id, name) VALUES (63820, 'Omega Crafting Materials');
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639000, 63820, 8406, 1, 125); -- MAS-5 Field Crafting Tool, tier 1, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639001, 63820, 8889, 1, 125); -- Beta Materials Research Kit, tier 1, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639002, 63820, 8443, 1, 250); -- MAS-10 Field Crafting Tool, tier 1, tc 10
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639003, 63820, 8444, 1, 375); -- MAS-15 Field Crafting Tool, tier 1, tc 15
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639004, 63820, 8445, 1, 500); -- MAS-20 Field Crafting Tool, tier 2, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639005, 63820, 8446, 1, 625); -- MAS-25 Field Crafting Tool, tier 2, tc 25
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639006, 63820, 8447, 1, 750); -- MAS-30 Field Crafting Tool, tier 2, tc 30
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639007, 63820, 8448, 1, 875); -- MAS-35 Field Crafting Tool, tier 3, tc 35
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639008, 63820, 8449, 1, 1000); -- MAS-40 Field Crafting Tool, tier 3, tc 40
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639009, 63820, 8450, 1, 1125); -- MAS-45 Field Crafting Tool, tier 4, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639010, 63820, 8451, 1, 1250); -- MAS-50 Field Crafting Tool, tier 5, tc 50
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639011, 63820, 8452, 1, 1375); -- MAS-55 Field Crafting Tool, tier 5, tc 55
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639012, 63820, 5254, 1, 75); -- Steel Core (Materials), tier 1, tc 3
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639013, 63820, 5256, 1, 125); -- Titanium Core (Materials), tier 1, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639014, 63820, 5398, 1, 125); -- Steel Plating (Materials Subcombine A), tier 1, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639015, 63820, 5399, 1, 125); -- Steel Brushing (Materials Subcombine A), tier 1, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639016, 63820, 5400, 1, 125); -- Steel Shunt (Materials Subcombine A), tier 1, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639017, 63820, 5401, 1, 200); -- Titanium Plating (Materials Subcombine B), tier 1, tc 8
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639018, 63820, 5403, 1, 200); -- Titanium Scoring (Materials Subcombine B), tier 1, tc 8
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639019, 63820, 5404, 1, 200); -- Titanium Reinforcement (Materials Subcombine B), tier 1, tc 8
```

## Checks

Run by `.claude/reports/union-omega/stock/build.py` (not committed). Every check passes for all five lists.

| List | Rows | Price rule | CAN_BE_SOLD | Arbitrage designs (21, 55, 3437) | Other rules (1, 5) |
|---|---|---|---|---|---|
| 63816 Omega Consumables | 20 (1-20) | pass | pass | 0 present, pass | pass (unique, range 638800-638819, no Test/NO ITEM/Instrument, no WP/AR visual) |
| 63817 Omega Crafting Biomedical | 20 (1-20) | pass | pass | 0 present, pass | pass (range 638850-638869) |
| 63818 Omega Crafting Electronics | 20 (1-20) | pass | pass | 0 present, pass | pass (range 638900-638919) |
| 63819 Omega Crafting Power Systems | 20 (1-20) | pass | pass | 0 present, pass | pass (range 638950-638969) |
| 63820 Omega Crafting Materials | 20 (1-20) | pass | pass | 0 present, pass | pass (range 639000-639019) |

The script's check pass prints `pass` for all five lists.

## Excluded

- `Blueprint:` rows (crafting blueprints, flags 35840): not consumables.
- Tier-1 Slappack 4735 (`Health Slappack TC1`, tc 18): an eligible tier-1 row that UO-06 did not pick. Not picked.
- Above-tier-1 family rows past the first ten by the sort: 6599 (tc 30), 6664, 6665, 6666, 6717 and the tc 35/50 Stimpacks, and Mark VII and Mark X rows.
- `Antidote Darts` 9013 and `Adrenaline Darts` 9014 (ammo, flags 3072); `Antidote Administer` 777 (not a consumable of the three families).
- `Efficient <prefix>-35 Field Crafting Tool` rows (8417, 8463, 8465, 8466): the prefix rule does not match them.
- Weapons and armor: none in any pool (the visual check removes any that appear).
- Asgard programs (for example `Aesir Science Program`, `Ari Attack Program`) and minigame consumables (the `Archaeologist Minigame Consumable` rows are blueprints): none in any pool. The seed has no mini-game marker; the 100 picked names were checked by eye.
- `Asgard Astrocyte (Bio-Medical)`: an Asgard component, not a program, so the rule does not exclude it by name. It is in the Bio-Medical pool but not picked.
- Test, `NO ITEM NAME`, `Instrument`: none picked.

## Gaps

- No price column (rule: every price is a formula). Any retail-price evidence would change prices, not stock.
- The resource names (`(Bio-Medical)`, `(Power Systems)`) and kit names are written as they appear in the seed; the kit name for Bio-Medical is `Beta Biomedical Research Kit`, not `Beta Bio-Medical Research Kit`.
- Consumable families beyond Slappack, Antidote and Stimpack (for example `Antidote Darts`) are not stocked, per rule 3.
- The packet's "arbitrage designs" check is read as the 21, 55, 3437 guard; no sell list is in scope for this packet.

## Store-open proof

For each list, `store_opened` with `buy_count` = the list's row count:

| List | Expected `buy_count` |
|---|---|
| 63816 Omega Consumables | 20 |
| 63817 Omega Crafting Biomedical | 20 |
| 63818 Omega Crafting Electronics | 20 |
| 63819 Omega Crafting Power Systems | 20 |
| 63820 Omega Crafting Materials | 20 |

The proof is a later lab check (UO-U1, run by the owner); this packet runs no lab and no build.
