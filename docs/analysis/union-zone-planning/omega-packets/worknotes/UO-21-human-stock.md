# UO-21: sealed Human weapon and armor stock, four tiers (lists 63800-63807)

> **Type:** Worknote · **Packet:** [UO-21](../UO-21.md) · **Decision:** D-UO8 B, D-UO27, D-UO29

Provenance of every row: CIMMERIA_AUTHORING (D-UO8 B, D-UO29). No price column exists in `resources.items`, so every price is the formula in rule 5. The picks are reproducible with the survey script kept outside git at `.claude/reports/union-omega/stock/` (`parse.py`, `survey.py`, `stock.py`, `note.py`), run as `python -I stock.py`.

## Method

- **Parse.** `db/resources/Items/Seed/items.sql` and `ammo_items.sql`: 6074 `INSERT INTO items` rows, 6074 parsed, 0 failed, 0 duplicate item ids. Quoted strings keep `''` escapes and `'{...}'` arrays are read as text; every row's column count is checked. Parse failures listed: none (no ids to list).
- **Human weapon (rule 7).** `visual_component LIKE 'WP-Human.%'`, or item_id in (4990, 6491, 6480); and `ammo_types` not empty or `max_melee_range > 0`. Query: `SELECT ... FROM items WHERE (visual_component LIKE 'WP-Human.%' OR item_id IN (4990,6491,6480)) AND (ammo_types NOT IN ('{}') OR max_melee_range > 0)`.
- **Human armor (rule 7).** `visual_component LIKE 'AR_H_%'`.
- **Gates (rules 2, 6, 8).** `flags & 1024` set; name without `Test`, `NO ITEM NAME`, `Instrument`, `Mini-game`, `Asgard`, `Program`; item not 21, 55 or 3437; no `AR_J_` visual and no Jaffa name (Staff, Zat'ni'katel, Ribbon Device, Rak'tel, Jaffa Cannon).
- **Heavy (rule 4).** Class `heavy` (grenade launchers and the three named heavies). Eligible only in a tier 2+ list of its own tier, so the tier 1 heavy designs (4990, 6491, 6480) are out.
- **Weapon class (rule 9).** From the `icon_location` image prefix: `WP_AR_` assault rifle; `WP_SMG_` SMG; `WP_Pistol_` pistol; `WP_Shotgun_` shotgun; `WP_LMG_` LMG; `WP_Rifle_` rifle or sniper; `EM-CombatKnife` melee; `WP_Dartgun_` and `EM-C02Pistol` dart gun; `WP_GL_`, `EM-M60` and `IconMissing` heavy. Name fallback used for 0 designs.
- **Weapon selection (rule 9).** Within a class sort by `tech_comp`, then `item_id`. Round-robin in the class order assault rifle, SMG, pistol, shotgun, LMG, rifle or sniper, melee, dart gun, heavy, one per class per round, to 20 rows.
- **Armor family (rule 10).** Family = name without its last word; the last word must be Boots, Gloves, Helmet, Pants or Vest. Other last words (Jacket, Legs) are not in the rule's slot list and are excluded. A family's lowest tc is taken in its tier; every piece at that tc is picked. Families sort by (lowest tc, name); the first family that would pass 20 rows stops the list.
- **Price (rule 5).** Weapons `naquadah = 200 x tech_comp`; armor `naquadah = 100 x tech_comp`. Sell-back: list 2 buys 21, 55 and 3437 only, and none of them is stocked, so the arbitrage guard passes by construction.
- **Discipline (rule 8).** The split does not hold for these picks. Of 152 picked rows (all lists), 30 carry a 21-27 discipline and 65 a 40-49 one; the Human CO2 dartguns and several Human SMGs carry 21-27. The Jaffa exclusion therefore rests on visual, name and the `AR_J_` prefix, not on `discipline_ids`.

| List | Tier | Candidates (rule 7) | Eligible after gates | Picked | Mix or stop |
|---|---|---|---|---|---|
| 63800 | 1 | 130 | 119 | 20 | assault rifle 3, SMG 3, pistol 3, shotgun 3, LMG 2, rifle or sniper 2, melee 2, dart gun 2 |
| 63801 | 2 | 106 | 106 | 20 | assault rifle 3, SMG 3, pistol 2, shotgun 2, LMG 2, rifle or sniper 2, melee 2, dart gun 2, heavy 2 |
| 63802 | 3 | 97 | 97 | 20 | assault rifle 3, SMG 3, pistol 2, shotgun 2, LMG 2, rifle or sniper 2, melee 2, dart gun 2, heavy 2 |
| 63803 | 4 | 47 | 47 | 20 | assault rifle 3, SMG 3, pistol 2, shotgun 2, LMG 2, rifle or sniper 2, melee 2, dart gun 2, heavy 2 |
| 63804 | 1 | 239 | 229 | 18 | 7 families, stop at ('Aramid Fiber Ballistic', 5) |
| 63805 | 2 | 192 | 192 | 18 | 4 families, stop at ('Carbon n-Tube Ballistic', 5) |
| 63806 | 3 | 174 | 174 | 18 | 4 families, stop at ('Trinium Ablative', 4) |
| 63807 | 4 | 93 | 93 | 18 | 4 families, stop at ('Tri-Naq Alloy Ablative', 5) |

Eligible minus picked is the 20-row cap, not a rule failure: the round-robin and family rules stop at 20 rows.

## Gaps

None. Every list has 1-20 rows. Tier 1 has no heavy row by rule 4, which bars its heavy designs (4990, 6491, 6480) at tier 1, and the tier 1 weapon list is otherwise full.

## SQL

### For UO-11 (lists 63800-63803)

```sql
-- CIMMERIA_AUTHORING (D-UO8 B, D-UO29): Human Weapons tier 1, 200 x tech_comp
INSERT INTO item_lists (item_list_id, name) VALUES (63800, 'Omega Human Weapons Tier 1');
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638000, 63800, 3145, 1, 1000); -- AR 21 Assault Rifle, assault rifle, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638001, 63800, 3126, 1, 600); -- SGHC 6 SMG, SMG, tc 3
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638002, 63800, 3235, 1, 600); -- SI 3 9mm Pistol, pistol, tc 3
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638003, 63800, 3313, 1, 1000); -- Pump Shotgun, shotgun, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638004, 63800, 3260, 1, 600); -- SK37 LMG, LMG, tc 3
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638005, 63800, 3299, 1, 600); -- Nenz 24 Rifle, rifle or sniper, tc 3
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638006, 63800, 3325, 1, 600); -- Combat Knife, melee, tc 3
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638007, 63800, 3584, 1, 1000); -- CO2 Pistol Dartgun, dart gun, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638008, 63800, 3173, 1, 1000); -- DMR 12 Assault Rifle, assault rifle, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638009, 63800, 3127, 1, 1000); -- SGHC 6 SMG, SMG, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638010, 63800, 3236, 1, 1000); -- SI 3 9mm Pistol, pistol, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638011, 63800, 3314, 1, 1600); -- Pump Shotgun, shotgun, tc 8
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638012, 63800, 3261, 1, 1000); -- SK37 LMG, LMG, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638013, 63800, 3287, 1, 1000); -- SR1 .50-Cal Rifle, rifle or sniper, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638014, 63800, 3326, 1, 1000); -- Combat Knife, melee, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638015, 63800, 3585, 1, 1600); -- CO2 Pistol Dartgun, dart gun, tc 8
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638016, 63800, 3188, 1, 1000); -- Razor 12 Assault Rifle, assault rifle, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638017, 63800, 3147, 1, 1000); -- MPX 77 SMG, SMG, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638018, 63800, 3249, 1, 1000); -- Mule Pistol, pistol, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638019, 63800, 3315, 1, 2000); -- Pump Shotgun, shotgun, tc 10
```

```sql
-- CIMMERIA_AUTHORING (D-UO8 B, D-UO29): Human Weapons tier 2, 200 x tech_comp
INSERT INTO item_lists (item_list_id, name) VALUES (63801, 'Omega Human Weapons Tier 2');
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638050, 63801, 4772, 1, 4000); -- AR 21A Assault Rifle, assault rifle, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638051, 63801, 4739, 1, 1000); -- Kotchner V17 RF SMG, SMG, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638052, 63801, 4883, 1, 4000); -- SI 4 9mm Pistol, pistol, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638053, 63801, 4926, 1, 4000); -- Semi Auto Shotgun, shotgun, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638054, 63801, 4836, 1, 4000); -- SK37 B LMG, LMG, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638055, 63801, 4946, 1, 4000); -- Nenz 25 Rifle, rifle or sniper, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638056, 63801, 5070, 1, 4000); -- Heavy Combat Knife, melee, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638057, 63801, 3591, 1, 4600); -- CO2 Rifle Dartgun, dart gun, tc 23
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638058, 63801, 4997, 1, 4000); -- GL 22 Grenade Launcher, heavy, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638059, 63801, 4787, 1, 4000); -- DMR 21 Assault Rifle, assault rifle, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638060, 63801, 4693, 1, 4000); -- SGHC 6 AP SMG, SMG, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638061, 63801, 4900, 1, 4000); -- Mule MKII Pistol, pistol, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638062, 63801, 3320, 1, 4600); -- Semi Auto Shotgun, shotgun, tc 23
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638063, 63801, 4858, 1, 4000); -- ZEV HE LMG, LMG, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638064, 63801, 4960, 1, 4000); -- SR2 .50-Cal Rifle, rifle or sniper, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638065, 63801, 3334, 1, 5000); -- Heavy Combat Knife, melee, tc 25
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638066, 63801, 3592, 1, 5000); -- CO2 Rifle Dartgun, dart gun, tc 25
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638067, 63801, 4998, 1, 4600); -- GL 22 Grenade Launcher, heavy, tc 23
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638068, 63801, 4801, 1, 4000); -- Razor 12 SP Assault Rifle, assault rifle, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638069, 63801, 4708, 1, 4000); -- MPX 77a SMG, SMG, tc 20
```

```sql
-- CIMMERIA_AUTHORING (D-UO8 B, D-UO29): Human Weapons tier 3, 200 x tech_comp
INSERT INTO item_lists (item_list_id, name) VALUES (63802, 'Omega Human Weapons Tier 3');
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638100, 63802, 4774, 1, 6600); -- AR 21A TACMOD Assault Rifle, assault rifle, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638101, 63802, 4711, 1, 6600); -- MPX 78 SMG, SMG, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638102, 63802, 4885, 1, 6600); -- SI 5 9mm Pistol, pistol, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638103, 63802, 4928, 1, 6600); -- Combat Shotgun, shotgun, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638104, 63802, 4841, 1, 6600); -- SK38 LMG, LMG, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638105, 63802, 4948, 1, 6600); -- Nenz 25S Rifle, rifle or sniper, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638106, 63802, 5077, 1, 6600); -- Composite Combat Knife, melee, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638107, 63802, 5025, 1, 6600); -- CO2 Semi Auto Dartgun, dart gun, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638108, 63802, 5003, 1, 6600); -- GL 23 Grenade Launcher, heavy, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638109, 63802, 4789, 1, 6600); -- DMR 21 X Assault Rifle, assault rifle, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638110, 63802, 4726, 1, 6600); -- PVR SMG, SMG, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638111, 63802, 4902, 1, 6600); -- Mule MKIII Pistol, pistol, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638112, 63802, 4929, 1, 7000); -- Combat Shotgun, shotgun, tc 35
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638113, 63802, 4861, 1, 6600); -- ZEV 2 LMG, LMG, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638114, 63802, 4949, 1, 7000); -- Nenz 25S Rifle, rifle or sniper, tc 35
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638115, 63802, 5078, 1, 7000); -- Composite Combat Knife, melee, tc 35
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638116, 63802, 5026, 1, 7000); -- CO2 Semi Auto Dartgun, dart gun, tc 35
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638117, 63802, 5004, 1, 7000); -- GL 23 Grenade Launcher, heavy, tc 35
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638118, 63802, 4803, 1, 6600); -- Razor Elite Assault Rifle, assault rifle, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638119, 63802, 4744, 1, 6600); -- Kotchner V17 VRF SMG, SMG, tc 33
```

```sql
-- CIMMERIA_AUTHORING (D-UO8 B, D-UO29): Human Weapons tier 4, 200 x tech_comp
INSERT INTO item_lists (item_list_id, name) VALUES (63803, 'Omega Human Weapons Tier 4');
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638150, 63803, 4780, 1, 9000); -- AR 200 Assault Rifle, assault rifle, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638151, 63803, 4717, 1, 9000); -- MPX 78 EX SMG, SMG, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638152, 63803, 4891, 1, 9000); -- SI-P 5 9mm Pistol, pistol, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638153, 63803, 4933, 1, 9000); -- Assault Shotgun, shotgun, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638154, 63803, 4849, 1, 9000); -- SK38 X LMG, LMG, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638155, 63803, 4954, 1, 9000); -- Nenz 25S-II Rifle, rifle or sniper, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638156, 63803, 5083, 1, 9000); -- Microblade Combat Knife, melee, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638157, 63803, 5034, 1, 9000); -- CO2 ACR Dartgun, dart gun, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638158, 63803, 5009, 1, 9000); -- GL 24 Grenade Launcher, heavy, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638159, 63803, 4795, 1, 9000); -- DMR 210 Assault Rifle, assault rifle, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638160, 63803, 4732, 1, 9000); -- PVK New SMG, SMG, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638161, 63803, 4909, 1, 9000); -- Big Mule Pistol, pistol, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638162, 63803, 4934, 1, 9600); -- Assault Shotgun, shotgun, tc 48
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638163, 63803, 4869, 1, 9000); -- ZEV 3 LMG, LMG, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638164, 63803, 4955, 1, 9600); -- Nenz 25S-II Rifle, rifle or sniper, tc 48
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638165, 63803, 5085, 1, 10000); -- Microblade Combat Knife, melee, tc 50
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638166, 63803, 5035, 1, 9600); -- CO2 ACR Dartgun, dart gun, tc 48
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638167, 63803, 5010, 1, 9600); -- GL 24 Grenade Launcher, heavy, tc 48
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638168, 63803, 4809, 1, 9000); -- Razor X Assault Rifle, assault rifle, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638169, 63803, 4754, 1, 9000); -- Kotchner V17 X SMG, SMG, tc 45
```

### For UO-27 (lists 63804-63807)

```sql
-- CIMMERIA_AUTHORING (D-UO8 B, D-UO29): Human Armor tier 1, 100 x tech_comp
INSERT INTO item_lists (item_list_id, name) VALUES (63804, 'Omega Human Armor Tier 1');
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638200, 63804, 4330, 1, 100); -- BDU Boots, Boots slot, tc 1
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638201, 63804, 2971, 1, 100); -- Titanium Ablative Vest, Vest slot, tc 1
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638202, 63804, 7374, 1, 300); -- Armored BDU Boots, Boots slot, tc 3
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638203, 63804, 2967, 1, 300); -- Armored BDU Gloves, Gloves slot, tc 3
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638204, 63804, 7376, 1, 300); -- Armored BDU Gloves, Gloves slot, tc 3
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638205, 63804, 7375, 1, 300); -- Armored BDU Helmet, Helmet slot, tc 3
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638206, 63804, 2949, 1, 300); -- Armored Prison Boots, Boots slot, tc 3
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638207, 63804, 2925, 1, 300); -- Armored Prison Helmet, Helmet slot, tc 3
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638208, 63804, 3401, 1, 300); -- Covert Stealth Boots, Boots slot, tc 3
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638209, 63804, 3387, 1, 300); -- Covert Stealth Gloves, Gloves slot, tc 3
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638210, 63804, 3346, 1, 300); -- Covert Stealth Helmet, Helmet slot, tc 3
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638211, 63804, 3372, 1, 300); -- Covert Stealth Pants, Pants slot, tc 3
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638212, 63804, 3359, 1, 300); -- Covert Stealth Vest, Vest slot, tc 3
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638213, 63804, 3488, 1, 300); -- Prison Jumpsuit Ablative Vest, Vest slot, tc 3
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638214, 63804, 7089, 1, 500); -- Aramid Fiber Ablative Gloves, Gloves slot, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638215, 63804, 6861, 1, 500); -- Aramid Fiber Ablative Helmet, Helmet slot, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638216, 63804, 7014, 1, 500); -- Aramid Fiber Ablative Pants, Pants slot, tc 5
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638217, 63804, 6938, 1, 500); -- Aramid Fiber Ablative Vest, Vest slot, tc 5
```

```sql
-- CIMMERIA_AUTHORING (D-UO8 B, D-UO29): Human Armor tier 2, 100 x tech_comp
INSERT INTO item_lists (item_list_id, name) VALUES (63805, 'Omega Human Armor Tier 2');
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638250, 63805, 4168, 1, 2000); -- Boron-Carbide Ablative Gloves, Gloves slot, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638251, 63805, 4072, 1, 2000); -- Boron-Carbide Ablative Helmet, Helmet slot, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638252, 63805, 4152, 1, 2000); -- Boron-Carbide Ablative Pants, Pants slot, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638253, 63805, 4094, 1, 2000); -- Boron-Carbide Ablative Vest, Vest slot, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638254, 63805, 4190, 1, 2000); -- Boron-Carbide Ballistic Boots, Boots slot, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638255, 63805, 4163, 1, 2000); -- Boron-Carbide Ballistic Gloves, Gloves slot, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638256, 63805, 4077, 1, 2000); -- Boron-Carbide Ballistic Helmet, Helmet slot, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638257, 63805, 4130, 1, 2000); -- Boron-Carbide Ballistic Pants, Pants slot, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638258, 63805, 4112, 1, 2000); -- Boron-Carbide Ballistic Vest, Vest slot, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638259, 63805, 4284, 1, 2000); -- Boron-Carbide Contamination Boots, Boots slot, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638260, 63805, 4273, 1, 2000); -- Boron-Carbide Contamination Gloves, Gloves slot, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638261, 63805, 4222, 1, 2000); -- Boron-Carbide Contamination Helmet, Helmet slot, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638262, 63805, 4261, 1, 2000); -- Boron-Carbide Contamination Pants, Pants slot, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638263, 63805, 7171, 1, 2000); -- Carbon n-Tube Ablative Boots, Boots slot, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638264, 63805, 7096, 1, 2000); -- Carbon n-Tube Ablative Gloves, Gloves slot, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638265, 63805, 6868, 1, 2000); -- Carbon n-Tube Ablative Helmet, Helmet slot, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638266, 63805, 7021, 1, 2000); -- Carbon n-Tube Ablative Pants, Pants slot, tc 20
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638267, 63805, 6945, 1, 2000); -- Carbon n-Tube Ablative Vest, Vest slot, tc 20
```

```sql
-- CIMMERIA_AUTHORING (D-UO8 B, D-UO29): Human Armor tier 3, 100 x tech_comp
INSERT INTO item_lists (item_list_id, name) VALUES (63806, 'Omega Human Armor Tier 3');
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638300, 63806, 7102, 1, 3300); -- Ceramic Polymer Ablative Gloves, Gloves slot, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638301, 63806, 6876, 1, 3300); -- Ceramic Polymer Ablative Helmet, Helmet slot, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638302, 63806, 7027, 1, 3300); -- Ceramic Polymer Ablative Pants, Pants slot, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638303, 63806, 6951, 1, 3300); -- Ceramic Polymer Ablative Vest, Vest slot, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638304, 63806, 7202, 1, 3300); -- Ceramic Polymer Ballistic Boots, Boots slot, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638305, 63806, 7127, 1, 3300); -- Ceramic Polymer Ballistic Gloves, Gloves slot, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638306, 63806, 6901, 1, 3300); -- Ceramic Polymer Ballistic Helmet, Helmet slot, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638307, 63806, 7052, 1, 3300); -- Ceramic Polymer Ballistic Pants, Pants slot, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638308, 63806, 6976, 1, 3300); -- Ceramic Polymer Ballistic Vest, Vest slot, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638309, 63806, 7227, 1, 3300); -- Ceramic Polymer Contamination Boots, Boots slot, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638310, 63806, 6926, 1, 3300); -- Ceramic Polymer Contamination Helmet, Helmet slot, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638311, 63806, 7077, 1, 3300); -- Ceramic Polymer Contamination Pants, Pants slot, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638312, 63806, 7002, 1, 3300); -- Ceramic Polymer Contamination Vest, Vest slot, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638313, 63806, 4379, 1, 3300); -- Refractive Stealth Boots, Boots slot, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638314, 63806, 4368, 1, 3300); -- Refractive Stealth Gloves, Gloves slot, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638315, 63806, 4297, 1, 3300); -- Refractive Stealth Helmet, Helmet slot, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638316, 63806, 4357, 1, 3300); -- Refractive Stealth Pants, Pants slot, tc 33
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638317, 63806, 4322, 1, 3300); -- Refractive Stealth Vest, Vest slot, tc 33
```

```sql
-- CIMMERIA_AUTHORING (D-UO8 B, D-UO29): Human Armor tier 4, 100 x tech_comp
INSERT INTO item_lists (item_list_id, name) VALUES (63807, 'Omega Human Armor Tier 4');
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638350, 63807, 7108, 1, 4500); -- Bio-Silk Ablative Gloves, Gloves slot, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638351, 63807, 6882, 1, 4500); -- Bio-Silk Ablative Helmet, Helmet slot, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638352, 63807, 7033, 1, 4500); -- Bio-Silk Ablative Pants, Pants slot, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638353, 63807, 6957, 1, 4500); -- Bio-Silk Ablative Vest, Vest slot, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638354, 63807, 7208, 1, 4500); -- Bio-Silk Ballistic Boots, Boots slot, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638355, 63807, 7133, 1, 4500); -- Bio-Silk Ballistic Gloves, Gloves slot, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638356, 63807, 6907, 1, 4500); -- Bio-Silk Ballistic Helmet, Helmet slot, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638357, 63807, 7058, 1, 4500); -- Bio-Silk Ballistic Pants, Pants slot, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638358, 63807, 6982, 1, 4500); -- Bio-Silk Ballistic Vest, Vest slot, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638359, 63807, 7233, 1, 4500); -- Bio-Silk Contamination Boots, Boots slot, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638360, 63807, 7158, 1, 4500); -- Bio-Silk Contamination Gloves, Gloves slot, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638361, 63807, 6932, 1, 4500); -- Bio-Silk Contamination Helmet, Helmet slot, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638362, 63807, 7083, 1, 4500); -- Bio-Silk Contamination Pants, Pants slot, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638363, 63807, 7008, 1, 4500); -- Bio-Silk Contamination Vest, Vest slot, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638364, 63807, 4385, 1, 4500); -- Distortive Stealth Boots, Boots slot, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638365, 63807, 4303, 1, 4500); -- Distortive Stealth Helmet, Helmet slot, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638366, 63807, 4363, 1, 4500); -- Distortive Stealth Pants, Pants slot, tc 45
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638367, 63807, 4333, 1, 4500); -- Distortive Stealth Vest, Vest slot, tc 45
```

## Checks

| List | Rows | Tier | Price rule | CAN_BE_SOLD | Arbitrage 21/55/3437 | Heavy | Human and gates | Ids | Result |
|---|---|---|---|---|---|---|---|---|---|
| 63800 | 20 | 1 | pass | pass | pass | 0 (tier >= 2) | pass | pass | pass |
| 63801 | 20 | 2 | pass | pass | pass | 2 (tier >= 2) | pass | pass | pass |
| 63802 | 20 | 3 | pass | pass | pass | 2 (tier >= 2) | pass | pass | pass |
| 63803 | 20 | 4 | pass | pass | pass | 2 (tier >= 2) | pass | pass | pass |
| 63804 | 18 | 1 | pass | pass | pass | 0 (tier >= 2) | pass | pass | pass |
| 63805 | 18 | 2 | pass | pass | pass | 0 (tier >= 2) | pass | pass | pass |
| 63806 | 18 | 3 | pass | pass | pass | 0 (tier >= 2) | pass | pass | pass |
| 63807 | 18 | 4 | pass | pass | pass | 0 (tier >= 2) | pass | pass | pass |

Rules 1 (row count 1-20, unique designs, quantity 1) and 3 (tier equals list tier) also pass for all eight lists.

## Excluded

Designs that looked Human (visual `WP-Human.`, heavy id, or `AR_H_`) but failed a rule:

- rule 10 last word 'Jacket' is not a slot in the rule: 3 (3482, 4336, 7373)
- rule 10 last word 'Legs' is not a slot in the rule: 1 (3486)
- rule 2 design 21/55/3437 (list 2 buys back): 2 (21, 55)
- rule 3 tier 5 has no list (D-UO28): 144 (4720, 4721, 4722, 4736, 4737, 4738, 4757, 4758, 4760, 4783, 4784, 4785 and 132 more)
- rule 4 heavy weapon at tier 1: 9 (4990, 4991, 4992, 4993, 4994, 4995, 4996, 6480, 6491)
- rule 6 name contains 'NO ITEM NAME': 6 (3023, 3024, 3085, 3087, 3484, 4332)

No Human candidate matched the `Test`, `Instrument`, `Mini-game`, `Asgard`, `Program`, Jaffa-name or `AR_J_` gates (0 each). The Jaffa designs of rule 8 (Staff, Zat'ni'katel, Ribbon Device, Rak'tel, Jaffa Cannon, `AR_J_` armor) sit outside the Human visual prefixes and were never candidates.

## Store-open proof

For UO-U1 only, a vendor `INFO store_opened` event for each list should show this `buy_count` (the row count):

| List | Expected buy_count |
|---|---|
| 63800 | 20 |
| 63801 | 20 |
| 63802 | 20 |
| 63803 | 20 |
| 63804 | 18 |
| 63805 | 18 |
| 63806 | 18 |
| 63807 | 18 |
