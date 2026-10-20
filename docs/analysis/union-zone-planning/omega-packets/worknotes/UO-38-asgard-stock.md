# UO-38: sealed Asgard program and armor stock, four tiers (lists 63821-63828)

> **Type:** Worknote · **Packet:** [UO-38](../UO-38.md) · **Decision:** D-UO8 B, D-UO27, D-UO29, D-UO38

Provenance of every row: CIMMERIA_AUTHORING (D-UO8 B, D-UO29, D-UO38). No price column exists in `resources.items`, so every price is the formula in rule 4. The picks are reproducible with the survey script kept outside git at `.claude/reports/union-omega/stock/` (`parse.py`, `survey.py`, `classify.py`, `note.py`), run as `python -I classify.py`.

## Method

- **Parse.** `db/resources/Items/Seed/items.sql` and `ammo_items.sql` (the only files with `INSERT INTO items`): 6074 rows, 6074 parsed, 0 failed. Parse failures listed: none (no ids to list).
- **Asgard survey (rule 1).** Every row whose `name`, `description`, `icon_location`, `discipline_ids` or `container_sets` contains `Asgard`: 104 rows. Query: `SELECT ... FROM items WHERE name ~* 'asgard' OR description ~* 'asgard' OR icon_location ~* 'asgard' OR discipline_ids ~* 'asgard' OR container_sets ~* 'asgard'`. Of the 104: 64 mini-game and blueprint rows (20 Asgard Mini-game Instrument, 22 Minigame Consumable, 22 Blueprint), 1 crafting resource (5214, `(Bio-Medical)`), 39 devices and consumables (Asgard Micro-Projector 2193, Asgard Medical Scanner, Consciousness Hub and similar), and 0 programs or armor. Every device is rejected: it is neither a program nor armor.
- **Asgard armor (rule 1).** No row has an `AR_A` visual or an `Asgard` marker with an armor name. Query: `WHERE visual_component ~ '^(AR_A|WP-Asgard|AR_Asgard)' OR (name ~* 'asgard' AND name ~* 'armou?r|vest|helmet|boots|gloves|pants')`. Count: 0. The only other armor visuals are `AR_Global` (4, prisoner gear) and `AR_G_Clo` (3, Goa'uld cloth), neither Asgard; they are not stocked here.
- **Asgard programs (rule 1).** Programs carry no `Asgard` word in any marked column, so they are found by kind: `WHERE name LIKE '% Program' AND description LIKE '% Program' AND container_sets = '{3,1,17}'`. Count: 205 rows, all with container `{3,1,17}`, flags 36224 (CAN_BE_SOLD set) or 35840. They are the Asgard program family: the kind word in `description` (Weapon, Reflex, Targeting, Protection, Hazard, Ward, Resilience, Diffusion, General Purpose) names the program kind and the name is the program family.
- **Gates (rules 2, 5).** `flags & 1024` set; not 21, 55 or 3437; name and description free of `Test`, `NO ITEM NAME`, `Instrument`, `Blueprint`; not mini-game (`Mini-game`, flags 36225 or 36226); no Human or Jaffa visual or name. Gate outcomes: {'eligible': 181, 'rule 3 tier 5 (no list, D-UO28)': 24}.
- **Tier (rule 3).** A design is eligible in its own tier list; tier 5 (24 rows) has no list (D-UO28).
- **Selection (rule 6, programs).** Within a list, each kind sorts by `tech_comp`, then `item_id`. Round-robin over the kinds in the order Weapon, Reflex, Targeting, Protection, Hazard, Ward, Resilience, Diffusion, General Purpose, one row per kind per round, until 20 rows. General Purpose exists in tier 1 only.
- **Selection (rule 6, armor).** No eligible armor design exists, so no armor family is taken (see § Gaps).
- **Price (rule 4).** Asgard programs `naquadah = 200 x tech_comp`. Asgard armor (none stocked) would be `100 x tech_comp`.
- **Sell-back.** List 2 buys back 21, 55 and 3437 only, and none is stocked, so the arbitrage guard passes by construction.

| List | Tier | Eligible (rule 5) | Eligible by kind | Picked | Mix |
|---|---|---|---|---|---|
| 63821 | 1 | 61 | Weapon 8, Reflex 7, Targeting 7, Protection 8, Hazard 7, Ward 7, Resilience 8, Diffusion 7, General 2 | 20 | Weapon 3, Reflex 3, Targeting 2, Protection 2, Hazard 2, Ward 2, Resilience 2, Diffusion 2, General Purpose 2 |
| 63822 | 2 | 48 | Weapon 6, Reflex 6, Targeting 6, Protection 6, Hazard 6, Ward 6, Resilience 6, Diffusion 6 | 20 | Weapon 3, Reflex 3, Targeting 3, Protection 3, Hazard 2, Ward 2, Resilience 2, Diffusion 2 |
| 63823 | 3 | 48 | Weapon 6, Reflex 6, Targeting 6, Protection 6, Hazard 6, Ward 6, Resilience 6, Diffusion 6 | 20 | Weapon 3, Reflex 3, Targeting 3, Protection 3, Hazard 2, Ward 2, Resilience 2, Diffusion 2 |
| 63824 | 4 | 24 | Weapon 3, Reflex 3, Targeting 3, Protection 3, Hazard 3, Ward 3, Resilience 3, Diffusion 3 | 20 | Weapon 3, Reflex 3, Targeting 3, Protection 3, Hazard 2, Ward 2, Resilience 2, Diffusion 2 |
| 63825 | 1 | 0 | none | 0 | gap (see § Gaps) |
| 63826 | 2 | 0 | none | 0 | gap (see § Gaps) |
| 63827 | 3 | 0 | none | 0 | gap (see § Gaps) |
| 63828 | 4 | 0 | none | 0 | gap (see § Gaps) |

Eligible minus picked is the 20-row cap, not a rule failure: the round-robin stops at 20 rows.

## Gaps

- **Asgard armor, lists 63825-63828 (all four tiers).** No design in `items.sql` or `ammo_items.sql` carries an Asgard armor marker: no `AR_A` visual, no `Asgard` armor name, no armor family in the Asgard kit. Each list gets no rows, and no `item_lists` row is written for it. UO-40 needs an architect ruling on which armor designs are the Asgard armor (27256-27259), or on a seed of Asgard armor designs first.
- Programs: none. Each program list has enough eligible rows (tier 1: 61, tier 2: 48, tier 3: 48, tier 4: 24) for 20.
- Program rule, for the architect to confirm: no program row names `Asgard`. The program family is identified by its kind word and container `{3,1,17}` (see § Method). If the owner means a different set, UO-39 changes only its SQL.

## SQL

### For UO-39 (lists 63821-63824)

```sql
-- CIMMERIA_AUTHORING (D-UO8 B, D-UO29, D-UO38): Asgard Programs tier 1, 200 x tech_comp
INSERT INTO item_lists (item_list_id, name) VALUES (63821, 'Omega Asgard Programs Tier 1');
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639050, 63821, 3677, 1, 600); -- Warrior Attack Program, Weapon Program, tc 3
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639051, 63821, 5698, 1, 1000); -- Hound Attack Program, Reflex Program, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639052, 63821, 5726, 1, 1000); -- Eagle Attack Program, Targeting Program, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639053, 63821, 3678, 1, 600); -- Wall Defense Program, Protection Program, tc 3
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639054, 63821, 5782, 1, 1000); -- Guardian Defense Program, Hazard Program, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639055, 63821, 5825, 1, 1000); -- Shield Defense Program, Ward Program, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639056, 63821, 3679, 1, 600); -- Crow Science Program, Resilience Program, tc 3
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639057, 63821, 5898, 1, 1000); -- Godi Science Program, Diffusion Program, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639058, 63821, 3676, 1, 200); -- Workhorse General Purpose Program, General Purpose Program, tc 1
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639059, 63821, 5673, 1, 1000); -- Warrior Attack Program, Weapon Program, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639060, 63821, 5699, 1, 1600); -- Hound Attack Program, Reflex Program, tc 8
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639061, 63821, 5727, 1, 1600); -- Eagle Attack Program, Targeting Program, tc 8
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639062, 63821, 5752, 1, 1000); -- Wall Defense Program, Protection Program, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639063, 63821, 5783, 1, 1600); -- Guardian Defense Program, Hazard Program, tc 8
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639064, 63821, 5828, 1, 1600); -- Shield Defense Program, Ward Program, tc 8
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639065, 63821, 5856, 1, 1000); -- Crow Science Program, Resilience Program, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639066, 63821, 5899, 1, 1600); -- Godi Science Program, Diffusion Program, tc 8
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639067, 63821, 8536, 1, 200); -- Workhorse General Purpose Program, General Purpose Program, tc 1
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639068, 63821, 5674, 1, 1600); -- Warrior Attack Program, Weapon Program, tc 8
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639069, 63821, 5700, 1, 2000); -- Hound Attack Program, Reflex Program, tc 10
```

```sql
-- CIMMERIA_AUTHORING (D-UO8 B, D-UO29, D-UO38): Asgard Programs tier 2, 200 x tech_comp
INSERT INTO item_lists (item_list_id, name) VALUES (63822, 'Omega Asgard Programs Tier 2');
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639100, 63822, 5680, 1, 4000); -- Rekkr Attack Program, Weapon Program, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639101, 63822, 5708, 1, 4000); -- Wolf Attack Program, Reflex Program, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639102, 63822, 5733, 1, 4000); -- Ari Attack Program, Targeting Program, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639103, 63822, 5759, 1, 4000); -- Husgord Defense Program, Protection Program, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639104, 63822, 5790, 1, 4000); -- Varda Defense Program, Hazard Program, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639105, 63822, 5837, 1, 4000); -- Lind Defense Program, Ward Program, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639106, 63822, 5863, 1, 4000); -- Rook Science Program, Resilience Program, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639107, 63822, 5906, 1, 4000); -- Munkr Science Program, Diffusion Program, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639108, 63822, 5681, 1, 4600); -- Rekkr Attack Program, Weapon Program, tc 23
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639109, 63822, 5709, 1, 4600); -- Wolf Attack Program, Reflex Program, tc 23
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639110, 63822, 5734, 1, 4600); -- Ari Attack Program, Targeting Program, tc 23
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639111, 63822, 5761, 1, 4600); -- Husgord Defense Program, Protection Program, tc 23
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639112, 63822, 5792, 1, 4600); -- Varda Defense Program, Hazard Program, tc 23
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639113, 63822, 5838, 1, 4600); -- Lind Defense Program, Ward Program, tc 23
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639114, 63822, 5864, 1, 4600); -- Rook Science Program, Resilience Program, tc 23
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639115, 63822, 5907, 1, 4600); -- Munkr Science Program, Diffusion Program, tc 23
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639116, 63822, 5682, 1, 5000); -- Rekkr Attack Program, Weapon Program, tc 25
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639117, 63822, 5710, 1, 5000); -- Wolf Attack Program, Reflex Program, tc 25
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639118, 63822, 5735, 1, 5000); -- Ari Attack Program, Targeting Program, tc 25
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639119, 63822, 5762, 1, 5000); -- Husgord Defense Program, Protection Program, tc 25
```

```sql
-- CIMMERIA_AUTHORING (D-UO8 B, D-UO29, D-UO38): Asgard Programs tier 3, 200 x tech_comp
INSERT INTO item_lists (item_list_id, name) VALUES (63823, 'Omega Asgard Programs Tier 3');
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639150, 63823, 5686, 1, 6600); -- Valkyrie Attack Program, Weapon Program, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639151, 63823, 5714, 1, 6600); -- Skoll Attack Program, Reflex Program, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639152, 63823, 5739, 1, 6600); -- Om Attack Program, Targeting Program, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639153, 63823, 5768, 1, 6600); -- Veggr Defense Program, Protection Program, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639154, 63823, 5797, 1, 6600); -- Tanngnjostr Defense Program, Hazard Program, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639155, 63823, 5843, 1, 6600); -- Rond Defense Program, Ward Program, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639156, 63823, 5875, 1, 6600); -- Raven Science Program, Resilience Program, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639157, 63823, 5912, 1, 6600); -- Prestr Science Program, Diffusion Program, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639158, 63823, 5687, 1, 7000); -- Valkyrie Attack Program, Weapon Program, tc 35
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639159, 63823, 5715, 1, 7000); -- Skoll Attack Program, Reflex Program, tc 35
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639160, 63823, 5740, 1, 7000); -- Om Attack Program, Targeting Program, tc 35
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639161, 63823, 5769, 1, 7000); -- Veggr Defense Program, Protection Program, tc 35
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639162, 63823, 5798, 1, 7000); -- Tanngnjostr Defense Program, Hazard Program, tc 35
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639163, 63823, 5844, 1, 7000); -- Rond Defense Program, Ward Program, tc 35
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639164, 63823, 5876, 1, 7000); -- Raven Science Program, Resilience Program, tc 35
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639165, 63823, 5913, 1, 7000); -- Prestr Science Program, Diffusion Program, tc 35
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639166, 63823, 5688, 1, 7600); -- Valkyrie Attack Program, Weapon Program, tc 38
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639167, 63823, 5716, 1, 7600); -- Skoll Attack Program, Reflex Program, tc 38
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639168, 63823, 5741, 1, 7600); -- Om Attack Program, Targeting Program, tc 38
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639169, 63823, 5770, 1, 7600); -- Veggr Defense Program, Protection Program, tc 38
```

```sql
-- CIMMERIA_AUTHORING (D-UO8 B, D-UO29, D-UO38): Asgard Programs tier 4, 200 x tech_comp
INSERT INTO item_lists (item_list_id, name) VALUES (63824, 'Omega Asgard Programs Tier 4');
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639200, 63824, 5692, 1, 9000); -- Einherjar Attack Program, Weapon Program, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639201, 63824, 5720, 1, 9000); -- Garm Attack Program, Reflex Program, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639202, 63824, 5746, 1, 9000); -- Vedrfolnir Attack Program, Targeting Program, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639203, 63824, 5775, 1, 9000); -- Kastali Defense Program, Protection Program, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639204, 63824, 5806, 1, 9000); -- Tanngrisnir Defense Program, Hazard Program, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639205, 63824, 5849, 1, 9000); -- Skjoldr Defense Program, Ward Program, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639206, 63824, 5885, 1, 9000); -- Muninn Science Program, Resilience Program, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639207, 63824, 5918, 1, 9000); -- Vanir Science Program, Diffusion Program, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639208, 63824, 5693, 1, 9600); -- Einherjar Attack Program, Weapon Program, tc 48
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639209, 63824, 5721, 1, 9600); -- Garm Attack Program, Reflex Program, tc 48
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639210, 63824, 5747, 1, 9600); -- Vedrfolnir Attack Program, Targeting Program, tc 48
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639211, 63824, 5777, 1, 9600); -- Kastali Defense Program, Protection Program, tc 48
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639212, 63824, 5807, 1, 9600); -- Tanngrisnir Defense Program, Hazard Program, tc 48
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639213, 63824, 5850, 1, 9600); -- Skjoldr Defense Program, Ward Program, tc 48
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639214, 63824, 5887, 1, 9600); -- Muninn Science Program, Resilience Program, tc 48
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639215, 63824, 5919, 1, 9600); -- Vanir Science Program, Diffusion Program, tc 48
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639216, 63824, 5694, 1, 10000); -- Einherjar Attack Program, Weapon Program, tc 50
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639217, 63824, 5722, 1, 10000); -- Garm Attack Program, Reflex Program, tc 50
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639218, 63824, 5748, 1, 10000); -- Vedrfolnir Attack Program, Targeting Program, tc 50
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639219, 63824, 5778, 1, 10000); -- Kastali Defense Program, Protection Program, tc 50
```

### For UO-40 (lists 63825-63828)

No SQL. All four armor lists are gaps (see § Gaps). UO-40 writes no `item_lists` or `item_list_items` row for them.

## Checks

| List | Rows | Tier | Price rule | CAN_BE_SOLD | Arbitrage 21/55/3437 | Program gates | Ids and uniqueness | Result |
|---|---|---|---|---|---|---|---|---|
| 63821 | 20 | 1 | pass | pass | pass | pass | pass | pass |
| 63822 | 20 | 2 | pass | pass | pass | pass | pass | pass |
| 63823 | 20 | 3 | pass | pass | pass | pass | pass | pass |
| 63824 | 20 | 4 | pass | pass | pass | pass | pass | pass |
| 63825 | 0 | 1 | n/a | n/a | n/a | n/a | n/a | gap, § Gaps |
| 63826 | 0 | 2 | n/a | n/a | n/a | n/a | n/a | gap, § Gaps |
| 63827 | 0 | 3 | n/a | n/a | n/a | n/a | n/a | gap, § Gaps |
| 63828 | 0 | 4 | n/a | n/a | n/a | n/a | n/a | gap, § Gaps |

Rules 1 (row count 1-20, unique designs, quantity 1) and 3 (tier equals list tier) pass for all four program lists. Rule 4 is the formula above, applied to every row.

## Excluded

- rule 3 tier 5 has no list (D-UO28): 24 programs (Mjolnir, Fenrir, Hraesvelg, Landvaettir, Sleipnir, Svalin, Huginn, Aesir).
- Mini-game items: 20 `Asgard Mini-game Instrument` rows, 22 `Minigame Consumable: Asgard` rows; excluded by rule 5.
- Blueprints: 22 `Blueprint: Asgard Minigame Consumable` rows (name carries `Minigame`, and `Blueprint`); excluded by rule 5.
- Crafting resource 5214 `Asgard Astrocyte (Bio-Medical)`: excluded by rule 5.
- Devices and consumables that name Asgard (39 rows, e.g. 2193 Asgard Micro-Projector, 763 Asgard Field Receiver, 3165 Asgard Medical Scanner, 4410 Consciousness Disruptor): not programs and not armor, so not stocked.
- Program rows failing no gate other than tier 5: none. Human, Jaffa, Test and NO ITEM NAME rows among programs: 0.

## Store-open proof

For UO-U1 only, a vendor `INFO store_opened` event for each list should show this `buy_count` (the row count):

| List | Expected buy_count |
|---|---|
| 63821 | 20 |
| 63822 | 20 |
| 63823 | 20 |
| 63824 | 20 |
| 63825-63828 | none: gaps, no store rows |
