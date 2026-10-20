# UO-38: sealed Asgard program and armor stock, four tiers (lists 63821-63828)

> **Type:** Worknote · **Packet:** [UO-38](../UO-38.md) · **Decision:** D-UO8 B, D-UO27, D-UO29, D-UO38

Provenance of every row: CIMMERIA_AUTHORING (D-UO8 B, D-UO29, D-UO38). No price column exists in `resources.items`, so every price is the formula in rule 4. The picks are reproducible with the survey scripts kept outside git at `.claude/reports/union-omega/stock/` (`parse.py`, `survey.py`, `classify.py`, `armor.py`, `verify.py`, `note.py`), run as `python -I classify.py` (programs) and `python -I armor.py` (armor).

## Method

- **Parse.** `db/resources/Items/Seed/items.sql` and `ammo_items.sql` (the only files with `INSERT INTO items`): 6074 rows, 6074 parsed, 0 failed. Parse failures listed: none (no ids to list).
- **Asgard survey (rule 1).** Every row whose `name`, `description`, `icon_location`, `discipline_ids` or `container_sets` contains `Asgard`: 104 rows. Query: `SELECT ... FROM items WHERE name ~* 'asgard' OR description ~* 'asgard' OR icon_location ~* 'asgard' OR discipline_ids ~* 'asgard' OR container_sets ~* 'asgard'`. Of the 104: 64 mini-game and blueprint rows (20 Asgard Mini-game Instrument, 22 Minigame Consumable, 22 Blueprint), 1 crafting resource (5214, `(Bio-Medical)`), 39 devices and consumables (Asgard Micro-Projector 2193, Asgard Medical Scanner, Consciousness Hub and similar), and 0 programs or armor. Every device is rejected: it is neither a program nor armor.
- **Asgard armor (rule 1, ruling R1).** The first survey looked for the word `Asgard` and for an `AR_A` visual; the armor rows carry neither (`WHERE visual_component ~ '^(AR_A|WP-Asgard|AR_Asgard)'` returns no armor row). They are found by the client's own names. `db/resources/Texts/Seed/texts.sql` 28243-28442 ships 195 `DN_Armor_Asgard_<Family>_<Slot>_<tier><letter>_<Prefix>` monikers (families Absorption, Phasic, Regeneration; slots Chest, Feet, Hands, Head, Legs). `db/resources/Items/Seed/items.sql` holds 369 designs named `<Prefix> <Family> <Slot word>`, and every name equals a `DN_Armor_Asgard_*` moniker text (`verify.py`: 0 missing). They have flags 36224, a NULL `visual_component` and description `NO ITEM DESCRIPTION`. The prefix is the tier (Fyrstyr 1, Annarr 2, Pridi 3, Fjordi 4, Fimm 5). The container is the slot: `{1,4,17}` Helmet, `{1,7,17}` Vest, `{1,8,17}` Gloves, `{1,11,17}` Pants, `{1,12,17}` Boots. `db/resources/Visuals/Seed/body_components.sql` ships 90 `AR_A_*` components for `BS_Asgard.BS_Asgard` only; they are not wired to these designs, and nothing here changes that. Classification: `WHERE name ~ '^(Fyrstyr|Annarr|Pridi|Fjordi|Fimm) (Absorption|Phasic|Regeneration) (Helmet|Vest|Gloves|Pants|Boots)$'`, plus the gates below.
- **Slot-word gate (armor).** Exclude a design whose slot word disagrees with its container slot. 24 rows fail it: 23 Absorption "Boots" in the hands container `{1,8,17}` (tiers 1-5; the client's `DN_Armor_Asgard_Absorption_Hands_*` texts are empty except `Pridi Absorption Gloves`, so the seed's "Boots" name is a reconstruction slip) and 1 Fimm Regeneration "Vest" in `{1,12,17}` (tier 5). Eligible after the gates: tier 1 **98**, tier 2 **84**, tier 3 **80**, tier 4 **42**. Tier 5 has 45 armor rows (4 of them slot-gated) and no list (D-UO28).
- **Asgard programs (rule 1, ruling R2).** Programs carry no `Asgard` word in any marked column, so they are found by kind: `WHERE name LIKE '% Program' AND description LIKE '% Program' AND container_sets = '{3,1,17}'`. Count: 205 rows, all with container `{3,1,17}`, flags 36224 (CAN_BE_SOLD set) or 35840. They are the Asgard program family: the kind word in `description` (Weapon, Reflex, Targeting, Protection, Hazard, Ward, Resilience, Diffusion, General Purpose) names the program kind and the name is the program family. Evidence: all 205 names equal a client `DN_Weap_Program_<Kind>_<tier><letter>_<Name>` moniker text in `texts.sql` (`verify.py`: 0 missing); `docs/analysis/ability-trees/worknotes/at05b.md:45` shows `ARCHETYPE_Asgard` trains Attack, Defensive and Scientific Programs, and no other archetype does. Correction: `container_sets '{3,1,17}'` is the weapon container that 1469 designs share, so it confirms that a program sits in the weapon slot and does not mark a design as Asgard; it stays in the query as a filter.
- **Gates (rules 2, 5).** `flags & 1024` set; not 21, 55 or 3437; name and description free of `Test`, `NO ITEM NAME`, `Instrument`, `Blueprint`; not mini-game (`Mini-game`, flags 36225 or 36226); no Human or Jaffa visual or name. Gate outcomes: {'eligible': 181, 'rule 3 tier 5 (no list, D-UO28)': 24}.
- **Tier (rule 3).** A design is eligible in its own tier list; tier 5 (24 rows) has no list (D-UO28).
- **Selection (rule 6, programs).** Within a list, each kind sorts by `tech_comp`, then `item_id`. Round-robin over the kinds in the order Weapon, Reflex, Targeting, Protection, Hazard, Ward, Resilience, Diffusion, General Purpose, one row per kind per round, until 20 rows. General Purpose exists in tier 1 only. Design 8536 is dropped from the picks (ruling R3, review note N4: flags 35840, empty `discipline_ids`, the same name, tier, `tech_comp` 1 and price 200 as 3676). It stays in the eligible counts.
- **Selection (rule 6, armor, ruling R1).** Per tier, take whole families at each family's lowest `tech_comp` in that tier, in the order Absorption, Phasic, Regeneration, until the next family would pass 20 rows. All three fit in every tier, so each list holds 14 rows (Absorption 4 in tiers 1, 2 and 4, because those tiers have no Absorption gloves; Regeneration 4 in tier 3, because tier 3 has no Regeneration pants). Order the rows by `tech_comp`, then `item_id`. Armor rows take whole families and stop at 14 rows, not at the 20-row cap. `armor.py` prints the picks.
- **Price (rule 4).** Asgard programs `naquadah = 200 x tech_comp`. Asgard armor is `100 x tech_comp` (D-UO29, D-UO38). `item_id` for list L, row k: 638000 + (L - 63800) x 50 + k (`armor.py`).
- **Sell-back.** List 2 buys back 21, 55 and 3437 only, and none is stocked, so the arbitrage guard passes by construction.

| List | Tier | Eligible (rule 5) | Eligible by kind | Picked | Mix |
|---|---|---|---|---|---|
| 63821 | 1 | 61 | Weapon 8, Reflex 7, Targeting 7, Protection 8, Hazard 7, Ward 7, Resilience 8, Diffusion 7, General 2 | 20 | Weapon 3, Reflex 3, Targeting 3, Protection 2, Hazard 2, Ward 2, Resilience 2, Diffusion 2, General Purpose 1 |
| 63822 | 2 | 48 | Weapon 6, Reflex 6, Targeting 6, Protection 6, Hazard 6, Ward 6, Resilience 6, Diffusion 6 | 20 | Weapon 3, Reflex 3, Targeting 3, Protection 3, Hazard 2, Ward 2, Resilience 2, Diffusion 2 |
| 63823 | 3 | 48 | Weapon 6, Reflex 6, Targeting 6, Protection 6, Hazard 6, Ward 6, Resilience 6, Diffusion 6 | 20 | Weapon 3, Reflex 3, Targeting 3, Protection 3, Hazard 2, Ward 2, Resilience 2, Diffusion 2 |
| 63824 | 4 | 24 | Weapon 3, Reflex 3, Targeting 3, Protection 3, Hazard 3, Ward 3, Resilience 3, Diffusion 3 | 20 | Weapon 3, Reflex 3, Targeting 3, Protection 3, Hazard 2, Ward 2, Resilience 2, Diffusion 2 |
| 63825 | 1 | 98 | Absorption 28, Phasic 35, Regeneration 35 | 14 | Absorption 4, Phasic 5, Regeneration 5 |
| 63826 | 2 | 84 | Absorption 24, Phasic 30, Regeneration 30 | 14 | Absorption 4, Phasic 5, Regeneration 5 |
| 63827 | 3 | 80 | Absorption 26, Phasic 30, Regeneration 24 | 14 | Absorption 5, Phasic 5, Regeneration 4 |
| 63828 | 4 | 42 | Absorption 12, Phasic 15, Regeneration 15 | 14 | Absorption 4, Phasic 5, Regeneration 5 |

Eligible minus picked is the 20-row cap, not a rule failure: the round-robin stops at 20 rows. The armor lists stop at 14 rows by the family rule (§ Method), not at the cap.

## Gaps

- Armor: none (UO-38 ruling R1).
- Programs: none. Each program list has enough eligible rows (tier 1: 61, tier 2: 48, tier 3: 48, tier 4: 24) for 20.
- Program rule: confirmed (UO-38 ruling R2).

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
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639067, 63821, 5674, 1, 1600); -- Warrior Attack Program, Weapon Program, tc 8
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639068, 63821, 5700, 1, 2000); -- Hound Attack Program, Reflex Program, tc 10
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639069, 63821, 5728, 1, 2000); -- Eagle Attack Program, Targeting Program, tc 10
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

```sql
-- CIMMERIA_AUTHORING (D-UO8 B, D-UO29, D-UO38, UO-38 ruling R1): Asgard Armor tier 1, 100 x tech_comp
INSERT INTO item_lists (item_list_id, name) VALUES (63825, 'Omega Asgard Armor Tier 1');
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639250, 63825, 7958, 1, 500); -- Fyrstyr Absorption Vest, Absorption, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639251, 63825, 7983, 1, 500); -- Fyrstyr Absorption Helmet, Absorption, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639252, 63825, 8008, 1, 500); -- Fyrstyr Absorption Pants, Absorption, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639253, 63825, 8058, 1, 500); -- Fyrstyr Absorption Boots, Absorption, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639254, 63825, 8083, 1, 500); -- Fyrstyr Phasic Helmet, Phasic, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639255, 63825, 8108, 1, 500); -- Fyrstyr Phasic Vest, Phasic, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639256, 63825, 8133, 1, 500); -- Fyrstyr Phasic Pants, Phasic, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639257, 63825, 8158, 1, 500); -- Fyrstyr Phasic Gloves, Phasic, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639258, 63825, 8183, 1, 500); -- Fyrstyr Phasic Boots, Phasic, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639259, 63825, 8208, 1, 500); -- Fyrstyr Regeneration Helmet, Regeneration, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639260, 63825, 8233, 1, 500); -- Fyrstyr Regeneration Vest, Regeneration, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639261, 63825, 8258, 1, 500); -- Fyrstyr Regeneration Pants, Regeneration, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639262, 63825, 8283, 1, 500); -- Fyrstyr Regeneration Gloves, Regeneration, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639263, 63825, 8308, 1, 500); -- Fyrstyr Regeneration Boots, Regeneration, tc 5
```

```sql
-- CIMMERIA_AUTHORING (D-UO8 B, D-UO29, D-UO38, UO-38 ruling R1): Asgard Armor tier 2, 100 x tech_comp
INSERT INTO item_lists (item_list_id, name) VALUES (63826, 'Omega Asgard Armor Tier 2');
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639300, 63826, 7965, 1, 2000); -- Annarr Absorption Vest, Absorption, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639301, 63826, 7990, 1, 2000); -- Annarr Absorption Helmet, Absorption, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639302, 63826, 8015, 1, 2000); -- Annarr Absorption Pants, Absorption, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639303, 63826, 8065, 1, 2000); -- Annarr Absorption Boots, Absorption, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639304, 63826, 8090, 1, 2000); -- Annarr Phasic Helmet, Phasic, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639305, 63826, 8115, 1, 2000); -- Annarr Phasic Vest, Phasic, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639306, 63826, 8140, 1, 2000); -- Annarr Phasic Pants, Phasic, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639307, 63826, 8165, 1, 2000); -- Annarr Phasic Gloves, Phasic, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639308, 63826, 8190, 1, 2000); -- Annarr Phasic Boots, Phasic, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639309, 63826, 8215, 1, 2000); -- Annarr Regeneration Helmet, Regeneration, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639310, 63826, 8240, 1, 2000); -- Annarr Regeneration Vest, Regeneration, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639311, 63826, 8265, 1, 2000); -- Annarr Regeneration Pants, Regeneration, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639312, 63826, 8290, 1, 2000); -- Annarr Regeneration Gloves, Regeneration, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639313, 63826, 8315, 1, 2000); -- Annarr Regeneration Boots, Regeneration, tc 20
```

```sql
-- CIMMERIA_AUTHORING (D-UO8 B, D-UO29, D-UO38, UO-38 ruling R1): Asgard Armor tier 3, 100 x tech_comp
INSERT INTO item_lists (item_list_id, name) VALUES (63827, 'Omega Asgard Armor Tier 3');
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639350, 63827, 7971, 1, 3300); -- Pridi Absorption Vest, Absorption, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639351, 63827, 7996, 1, 3300); -- Pridi Absorption Helmet, Absorption, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639352, 63827, 8021, 1, 3300); -- Pridi Absorption Pants, Absorption, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639353, 63827, 8046, 1, 3300); -- Pridi Absorption Gloves, Absorption, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639354, 63827, 8071, 1, 3300); -- Pridi Absorption Boots, Absorption, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639355, 63827, 8096, 1, 3300); -- Pridi Phasic Helmet, Phasic, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639356, 63827, 8121, 1, 3300); -- Pridi Phasic Vest, Phasic, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639357, 63827, 8146, 1, 3300); -- Pridi Phasic Pants, Phasic, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639358, 63827, 8171, 1, 3300); -- Pridi Phasic Gloves, Phasic, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639359, 63827, 8196, 1, 3300); -- Pridi Phasic Boots, Phasic, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639360, 63827, 8221, 1, 3300); -- Pridi Regeneration Helmet, Regeneration, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639361, 63827, 8246, 1, 3300); -- Pridi Regeneration Vest, Regeneration, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639362, 63827, 8296, 1, 3300); -- Pridi Regeneration Gloves, Regeneration, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639363, 63827, 8321, 1, 3300); -- Pridi Regeneration Boots, Regeneration, tc 33
```

```sql
-- CIMMERIA_AUTHORING (D-UO8 B, D-UO29, D-UO38, UO-38 ruling R1): Asgard Armor tier 4, 100 x tech_comp
INSERT INTO item_lists (item_list_id, name) VALUES (63828, 'Omega Asgard Armor Tier 4');
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639400, 63828, 7977, 1, 4500); -- Fjordi Absorption Vest, Absorption, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639401, 63828, 8002, 1, 4500); -- Fjordi Absorption Helmet, Absorption, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639402, 63828, 8027, 1, 4500); -- Fjordi Absorption Pants, Absorption, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639403, 63828, 8077, 1, 4500); -- Fjordi Absorption Boots, Absorption, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639404, 63828, 8102, 1, 4500); -- Fjordi Phasic Helmet, Phasic, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639405, 63828, 8127, 1, 4500); -- Fjordi Phasic Vest, Phasic, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639406, 63828, 8152, 1, 4500); -- Fjordi Phasic Pants, Phasic, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639407, 63828, 8177, 1, 4500); -- Fjordi Phasic Gloves, Phasic, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639408, 63828, 8202, 1, 4500); -- Fjordi Phasic Boots, Phasic, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639409, 63828, 8227, 1, 4500); -- Fjordi Regeneration Helmet, Regeneration, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639410, 63828, 8252, 1, 4500); -- Fjordi Regeneration Vest, Regeneration, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639411, 63828, 8277, 1, 4500); -- Fjordi Regeneration Pants, Regeneration, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639412, 63828, 8302, 1, 4500); -- Fjordi Regeneration Gloves, Regeneration, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (639413, 63828, 8327, 1, 4500); -- Fjordi Regeneration Boots, Regeneration, tc 45
```

## Checks

| List | Rows | Tier | Price rule | CAN_BE_SOLD | Arbitrage 21/55/3437 | Program gates | Ids and uniqueness | Result |
|---|---|---|---|---|---|---|---|---|
| 63821 | 20 | 1 | pass | pass | pass | pass | pass | pass |
| 63822 | 20 | 2 | pass | pass | pass | pass | pass | pass |
| 63823 | 20 | 3 | pass | pass | pass | pass | pass | pass |
| 63824 | 20 | 4 | pass | pass | pass | pass | pass | pass |
| 63825 | 14 | 1 | pass | pass | pass | n/a (armor) | pass | pass |
| 63826 | 14 | 2 | pass | pass | pass | n/a (armor) | pass | pass |
| 63827 | 14 | 3 | pass | pass | pass | n/a (armor) | pass | pass |
| 63828 | 14 | 4 | pass | pass | pass | n/a (armor) | pass | pass |

Rules 1 (row count 1-20, unique designs, quantity 1) and 3 (tier equals list tier) pass for all eight lists. Rule 4 is the formula above, applied to every row: programs `200 x tech_comp` (63821-63824), armor `100 x tech_comp` (63825-63828, all rows of a list share one `tech_comp`: 5, 20, 33 and 45).

## Excluded

- rule 3 tier 5 has no list (D-UO28): 24 programs (Mjolnir, Fenrir, Hraesvelg, Landvaettir, Sleipnir, Svalin, Huginn, Aesir).
- Mini-game items: 20 `Asgard Mini-game Instrument` rows, 22 `Minigame Consumable: Asgard` rows; excluded by rule 5.
- Blueprints: 22 `Blueprint: Asgard Minigame Consumable` rows (name carries `Minigame`, and `Blueprint`); excluded by rule 5.
- Crafting resource 5214 `Asgard Astrocyte (Bio-Medical)`: excluded by rule 5.
- Devices and consumables that name Asgard (39 rows, e.g. 2193 Asgard Micro-Projector, 763 Asgard Field Receiver, 3165 Asgard Medical Scanner, 4410 Consciousness Disruptor): not programs and not armor, so not stocked.
- Program rows failing no gate other than tier 5: none. Human, Jaffa, Test and NO ITEM NAME rows among programs: 0.
- Armor, slot-word gate (ruling R1), 24 rows by tier (ids): tier 1 8033-8039 (7); tier 2 8040-8045 (6); tier 3 8048-8051 (4); tier 4 8052-8054 (3); tier 5 8055-8057 and 8332 (4).
- Armor rule 3 tier 5 (no list, D-UO28): 45 armor rows in all, of which 4 are in the slot-word gate above and 41 are the rest.
- Design 8536 `Workhorse General Purpose Program`: flags/disciplines outlier duplicate of 3676 (N4, ruling R3); not picked.

## Store-open proof

For UO-U1 only, a vendor `INFO store_opened` event for each list should show this `buy_count` (the row count):

| List | Expected buy_count |
|---|---|
| 63821 | 20 |
| 63822 | 20 |
| 63823 | 20 |
| 63824 | 20 |
| 63825 | 14 |
| 63826 | 14 |
| 63827 | 14 |
| 63828 | 14 |
