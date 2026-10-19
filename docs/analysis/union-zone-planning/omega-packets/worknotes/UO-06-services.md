# UO-06: Omega vendor stock and roster proposal

> **Type:** Reference (worknote) · **Packet:** [UO-06](../UO-06.md) · **Status:** Proposal. Awaiting the owner on D-UO8 and D-UO16. No seed is edited here.
> **Copied by:** UO-11 (Tau'ri templates 63812-63816, lists 63802-63806) and UO-12 (Jaffa templates 63817-63819, lists 63807-63809).

## Summary

- Eight vendor templates, one per client moniker role, each with exactly one vendor bit and its own list. Everything structural is a **decision** (§ Decision or owner question). Stock and prices are **owner questions** with a recommended option, because no comparable zone's vendor list exists in the tree to mirror.
- The arbitrage guard binds only designs 21, 55 and 3437 (the three rows of sell list 2). No Omega row uses them, so every sell-back comparison below is "none".
- The success lines UO-11/12 quote are in § Telemetry.

## Evidence and limits

- **No sibling zone to mirror.** `db/resources/Items/Seed/` holds no vendor list for another zone. The only buy lists are the Debug Area plaza list 1300 (`item_lists_debug_area_plaza.sql`, ammo at 1, pistol 300, SMG 1000) and hub fixtures 1 and 310 (1 naquadah each). Harset vendors are an unpopulated gap (agent memory `vendor_trainer_seed_gap.md`). So there is nothing high-confidence to copy for stock.
- **No price column.** `resources.items` has no price, level or faction column. Every price below is CIMMERIA_AUTHORING, a formula, with no retail evidence.
- **Tier source.** `resources.items.tier` (1-5) and `tech_comp` (`tc`, 1-58). Weapon tiers line up with tc (T1 about 5-18, T2 20-33, T3 33-45, T4 45-50, T5 50-55). That `tc` gates equipping (a level proxy) is **unconfirmed**: no doc under `docs/gameplay`, `docs/content`, `docs/protocol` or `docs/reverse-engineering/findings` states it (hits for `tech_comp` there are crafting, minigame and combat-formula notes). "Tier 1" below means `tier = 1`, `tc` at most 18 (weapons, crafting) or at most 10 (armor, consumables).
- **Sold items only.** Every stocked design has `flags & 1024` (CAN_BE_SOLD), so a player can also sell it back to a vendor whose sell list prices it. Excluded: mini-game instruments, "NO ITEM NAME", test items, Asgard programs (UO-16, held).
- **Faction.** Armor and weapon names carry the faction (`AR_H_*`, `AR_J_*`; Staff, Zat'ni'katel, Ribbon Device are Jaffa). No item column restricts a race, so the split is by design, not enforced.
- 101 of 6059 item rows could not be parsed by the survey script (quoting). None of the 8 lists use an id from those; every id below was read from a parsed row.

## Templates

Shape: copy `db/resources/Entities/Seed/entity_templates_debug_area_plaza.sql:43` (template 1302). Keep `event_set_id 570`, level 1, alignment 0, faction 1, class `mob`, `sell_item_list`, `repair_item_list` and `recharge_item_list` = 2, `use_cover false`, `training_dummy false`. Change `interaction_type` from 49152 (two bits) to the one bit below, `name_id` and the buy list. Faction stays 1 (UO-11 contract); what faction 10 means is not verified, so the Jaffa bodies take `body_set` and `components` only.

| Template | name_id | moniker_name | interaction_type (one bit) | Body source (`body_set` + `components`) | UO-01 key | List |
|---|---|---|---|---|---|---|
| 63812 | 27240 | `DN_npc_ven_OmegaSite_HumanWeaponsTier01` | 16384 WEAPONS | template 1302, plaza file line 43 | `svc.human_weapons` | 63802 |
| 63813 | 27244 | `DN_npc_ven_OmegaSite_HumanArmorTier01` | 8192 ARMOR | template 1302, plaza file line 43 | `svc.human_armor` | 63803 |
| 63814 | 27249 | `DN_npc_ven_OmegaSite_HumanCraftingElec` | 2097152 CRAFT_ELECTRONICS | template 1302, plaza file line 43 | `svc.human_crafting` | 63804 |
| 63815 | 27239 | `DN_npc_ven_OmegaSite_CommonCraftingMat` | 1048576 CRAFT_MATERIALS | template 1302, plaza file line 43 | `svc.common_crafting` | 63805 |
| 63816 | 27264 | `DN_npc_ven_OmegaSite_Consumables` | 32768 CONSUMABLES | template 1302, plaza file line 43 | `svc.consumables` | 63806 |
| 63817 | 27224 | `DN_npc_ven_OmegaSite_JaffaWeaponsTier01` | 16384 WEAPONS | template 100 "Standard Jaffa", `entity_templates.sql:171` | `svc.jaffa_weapons` | 63807 |
| 63818 | 27228 | `DN_npc_ven_OmegaSite_JaffaArmorTier01` | 8192 ARMOR | template 100, `entity_templates.sql:171` | `svc.jaffa_armor` | 63808 |
| 63819 | 27232 | `DN_npc_ven_OmegaSite_JaffaCraftingBiomed` | 262144 CRAFT_BIO | template 129 "Standard Jaffa Female", `entity_templates.sql:235` | `svc.jaffa_crafting` | 63809 |

Notes for UO-11/12:

- Monikers are RESTORED_SOURCE (`texts.sql:37049-37137`). Template 63816 shares `name_id` 27264 with Debug Area template 1302; a shared moniker is a plain text lookup, so that is allowed.
- Template 1302's `components` end in `WP-Human.WP_SMG_1A`, so the Human armor, crafting and consumables vendors would hold an SMG. Dropping that one component is a cosmetic nit for UO-11 to flag to the owner, not a stock question. Template 15 (`entity_templates.sql:43`) is the alternative and carries a pistol.
- Template 58 "Walter Harriman" is a named character with `speaker_id` 843, and 59 is Bra'tac: both avoided as bodies.
- Crafting specialty is the owner's call (Q3). The packet's bits give the four specialties; option A has three crafting vendors, so one specialty (Power Systems, bit 524288) is not sold anywhere.

## Lists

Rows: `item_id` = 638000 + (list - 63800) x 100 + k. Columns: `(item_id, item_list_id, design_id, quantity, naquadah)`, quantity 1 throughout. Price rule: weapons 200 x tc, armor 100 x tc, crafting 25 x tc, consumables 25 (Slappack), 50 (Antidote), 100 (Mark III Stimpack). Sell-back on list 2 is **none** for every row below: list 2 holds only design 21 at 1000, 55 at 300 and 3437 at 100, none of them stocked here. The arbitrage guard therefore passes by construction. If the owner later adds any of 21, 55 or 3437, the buy price must be at least 1000, 300 or 100 times quantity.

Each block's provenance: CIMMERIA_AUTHORING (D-UO8 option A, pending the owner). UO-11/12 re-tag it CIMMERIA_RECONSTRUCTION once approved.

### List 63802: Omega Human Weapons (17 rows)

```sql
-- CIMMERIA_AUTHORING (D-UO8 option A, pending owner)
INSERT INTO item_lists (item_list_id, name) VALUES (63802, 'Omega Human Weapons');
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638201, 63802, 3145, 1, 1000); -- AR 21 Assault Rifle tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638202, 63802, 3210, 1, 1000); -- MHK 72 Assault Rifle tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638203, 63802, 3188, 1, 1000); -- Razor 12 Assault Rifle tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638204, 63802, 3173, 1, 1000); -- DMR 12 Assault Rifle tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638205, 63802, 3147, 1, 1000); -- MPX 77 SMG tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638206, 63802, 3217, 1, 1000); -- Kotchner V17 SMG tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638207, 63802, 3180, 1, 1000); -- PVK SMG tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638208, 63802, 3126, 1, 600); -- SGHC 6 SMG tc3, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638209, 63802, 3235, 1, 600); -- SI 3 9mm Pistol tc3, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638210, 63802, 3249, 1, 1000); -- Mule Pistol tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638211, 63802, 3313, 1, 1000); -- Pump Shotgun tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638212, 63802, 3274, 1, 1000); -- ZEV LMG tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638213, 63802, 3260, 1, 600); -- SK37 LMG tc3, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638214, 63802, 3287, 1, 1000); -- SR1 .50-Cal Rifle tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638215, 63802, 3299, 1, 600); -- Nenz 24 Rifle tc3, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638216, 63802, 3325, 1, 600); -- Combat Knife tc3, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638217, 63802, 3584, 1, 1000); -- CO2 Pistol Dartgun tc5, sell-back none
```

### List 63803: Omega Human Armor (20 rows)

```sql
-- CIMMERIA_AUTHORING (D-UO8 option A, pending owner)
INSERT INTO item_lists (item_list_id, name) VALUES (63803, 'Omega Human Armor');
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638301, 63803, 7189, 1, 500); -- Aramid Fiber Ballistic Boots tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638302, 63803, 7114, 1, 500); -- Aramid Fiber Ballistic Gloves tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638303, 63803, 6888, 1, 500); -- Aramid Fiber Ballistic Helmet tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638304, 63803, 7039, 1, 500); -- Aramid Fiber Ballistic Pants tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638305, 63803, 6963, 1, 500); -- Aramid Fiber Ballistic Vest tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638306, 63803, 7190, 1, 800); -- Aramid Fiber Ballistic Boots tc8, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638307, 63803, 7115, 1, 800); -- Aramid Fiber Ballistic Gloves tc8, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638308, 63803, 6889, 1, 800); -- Aramid Fiber Ballistic Helmet tc8, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638309, 63803, 7040, 1, 800); -- Aramid Fiber Ballistic Pants tc8, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638310, 63803, 6964, 1, 800); -- Aramid Fiber Ballistic Vest tc8, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638311, 63803, 7214, 1, 500); -- Aramid Fiber Contamination Boots tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638312, 63803, 7139, 1, 500); -- Aramid Fiber Contamination Gloves tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638313, 63803, 6913, 1, 500); -- Aramid Fiber Contamination Helmet tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638314, 63803, 7064, 1, 500); -- Aramid Fiber Contamination Pants tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638315, 63803, 6988, 1, 500); -- Aramid Fiber Contamination Vest tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638316, 63803, 7089, 1, 500); -- Aramid Fiber Ablative Gloves tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638317, 63803, 6861, 1, 500); -- Aramid Fiber Ablative Helmet tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638318, 63803, 7014, 1, 500); -- Aramid Fiber Ablative Pants tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638319, 63803, 6938, 1, 500); -- Aramid Fiber Ablative Vest tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638320, 63803, 4330, 1, 100); -- BDU Boots tc1, sell-back none
```

### List 63804: Omega Human Crafting (Electronics) (16 rows)

```sql
-- CIMMERIA_AUTHORING (D-UO8 option A, pending owner)
INSERT INTO item_lists (item_list_id, name) VALUES (63804, 'Omega Human Crafting (Electronics)');
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638401, 63804, 5224, 1, 75); -- Integrated Circuit (Electronics) tc3, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638402, 63804, 5225, 1, 125); -- Integrated Psuedocore (Electronics) tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638403, 63804, 5373, 1, 125); -- IC-K-layer (Electronics Subcombine A) tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638404, 63804, 5374, 1, 125); -- Output Processor (Electronics Subcombine A) tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638405, 63804, 5375, 1, 125); -- Minimatrix (Electronics Subcombine A) tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638406, 63804, 5376, 1, 200); -- Signal Damp (Electronics Subcombine B) tc8, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638407, 63804, 5377, 1, 200); -- Optical Rectifier (Electronics Subcombine B) tc8, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638408, 63804, 5378, 1, 200); -- Cell Blade (Electronics Subcombine B) tc8, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638409, 63804, 5379, 1, 275); -- Spoke Pseudocore (Electronics Subcombine C) tc11, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638410, 63804, 5380, 1, 275); -- Server Protocore (Electronics Subcombine C) tc11, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638411, 63804, 5381, 1, 275); -- Terfenol-D Amplifier (Electronics Subcombine C) tc11, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638412, 63804, 5382, 1, 325); -- Wavelet Transformer (Electronics Subcombine D) tc13, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638413, 63804, 5383, 1, 325); -- Discrete Sampler (Electronics Subcombine D) tc13, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638414, 63804, 8887, 1, 125); -- Beta Electronics Research Kit tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638415, 63804, 8402, 1, 125); -- EAS-5 Field Crafting Tool tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638416, 63804, 8433, 1, 250); -- EAS-10 Field Crafting Tool tc10, sell-back none
```

### List 63805: Omega Common Crafting (Materials) (16 rows)

```sql
-- CIMMERIA_AUTHORING (D-UO8 option A, pending owner)
INSERT INTO item_lists (item_list_id, name) VALUES (63805, 'Omega Common Crafting (Materials)');
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638501, 63805, 5254, 1, 75); -- Steel Core (Materials) tc3, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638502, 63805, 5256, 1, 125); -- Titanium Core (Materials) tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638503, 63805, 5398, 1, 125); -- Steel Plating (Materials Subcombine A) tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638504, 63805, 5399, 1, 125); -- Steel Brushing (Materials Subcombine A) tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638505, 63805, 5400, 1, 125); -- Steel Shunt (Materials Subcombine A) tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638506, 63805, 5401, 1, 200); -- Titanium Plating (Materials Subcombine B) tc8, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638507, 63805, 5403, 1, 200); -- Titanium Scoring (Materials Subcombine B) tc8, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638508, 63805, 5404, 1, 200); -- Titanium Reinforcement (Materials Subcombine B) tc8, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638509, 63805, 5405, 1, 275); -- High-shear Brace (Materials Subcombine C) tc11, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638510, 63805, 5406, 1, 275); -- Ti,FeC SQUID (Materials Subcombine C) tc11, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638511, 63805, 5409, 1, 325); -- Hyperpure Cementite (Materials Subcombine D) tc13, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638512, 63805, 5410, 1, 325); -- Lamellar Steel (Materials Subcombine D) tc13, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638513, 63805, 5411, 1, 325); -- Lambda-Spalled Steel (Materials Subcombine D) tc13, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638514, 63805, 8889, 1, 125); -- Beta Materials Research Kit tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638515, 63805, 8406, 1, 125); -- MAS-5 Field Crafting Tool tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638516, 63805, 8443, 1, 250); -- MAS-10 Field Crafting Tool tc10, sell-back none
```

### List 63806: Omega Consumables (10 rows)

```sql
-- CIMMERIA_AUTHORING (D-UO8 option A, pending owner)
INSERT INTO item_lists (item_list_id, name) VALUES (63806, 'Omega Consumables');
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638601, 63806, 2893, 1, 25); -- Health Slappack TC1 tc1, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638602, 63806, 6657, 1, 50); -- Health Antidote tc10, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638603, 63806, 6659, 1, 50); -- Health Antidote tc10, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638604, 63806, 6656, 1, 50); -- Kinetic Antidote tc10, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638605, 63806, 6677, 1, 100); -- Mark III Stimpack: Coordination tc10, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638606, 63806, 6678, 1, 100); -- Mark III Stimpack: Engagement tc10, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638607, 63806, 6679, 1, 100); -- Mark III Stimpack: Fortitude tc10, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638608, 63806, 6680, 1, 100); -- Mark III Stimpack: Intellect tc10, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638609, 63806, 6681, 1, 100); -- Mark III Stimpack: Morale tc10, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638610, 63806, 6682, 1, 100); -- Mark III Stimpack: Perception tc10, sell-back none
```

### List 63807: Omega Jaffa Weapons (20 rows)

```sql
-- CIMMERIA_AUTHORING (D-UO8 option A, pending owner)
INSERT INTO item_lists (item_list_id, name) VALUES (63807, 'Omega Jaffa Weapons');
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638701, 63807, 3849, 1, 1000); -- Boreal Staff tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638702, 63807, 3744, 1, 1000); -- Decay Staff tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638703, 63807, 3773, 1, 1000); -- Discharge Staff tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638704, 63807, 3852, 1, 1000); -- Pilgrim Staff tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638705, 63807, 3818, 1, 1000); -- Radiant Staff tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638706, 63807, 3792, 1, 1000); -- Shriek Staff tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638707, 63807, 2797, 1, 200); -- Serpent Staff tc1, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638708, 63807, 3965, 1, 1000); -- Boreal Zat'ni'katel tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638709, 63807, 3977, 1, 1000); -- Decay Zat'ni'katel tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638710, 63807, 5141, 1, 200); -- Discharge Zat'ni'katel tc1, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638711, 63807, 4015, 1, 1000); -- Pilgrim Zat'ni'katel tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638712, 63807, 3940, 1, 1000); -- Radiant Zat'ni'katel tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638713, 63807, 4016, 1, 1000); -- Serpent Zat'ni'katel tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638714, 63807, 3904, 1, 1000); -- Shriek Zat'ni'katel tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638715, 63807, 4513, 1, 1000); -- Boreal Ribbon Device tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638716, 63807, 4654, 1, 1000); -- Decay Ribbon Device tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638717, 63807, 4458, 1, 1000); -- Discharge Ribbon Device tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638718, 63807, 4540, 1, 1000); -- Pilgrim Ribbon Device tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638719, 63807, 4487, 1, 1000); -- Radiant Ribbon Device tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638720, 63807, 4576, 1, 1000); -- Shriek Ribbon Device tc5, sell-back none
```

### List 63808: Omega Jaffa Armor (20 rows)

```sql
-- CIMMERIA_AUTHORING (D-UO8 option A, pending owner)
INSERT INTO item_lists (item_list_id, name) VALUES (63808, 'Omega Jaffa Armor');
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638801, 63808, 6066, 1, 500); -- Standard Defensive Boots tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638802, 63808, 6001, 1, 500); -- Standard Defensive Breastplate tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638803, 63808, 6114, 1, 500); -- Standard Defensive Gauntlets tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638804, 63808, 6147, 1, 500); -- Standard Defensive Helmet tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638805, 63808, 4341, 1, 100); -- Standard Leggings tc1, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638806, 63808, 6235, 1, 500); -- Jackal Melee Boots tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638807, 63808, 6208, 1, 500); -- Jackal Melee Breastplate tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638808, 63808, 6270, 1, 500); -- Jackal Melee Gauntlets tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638809, 63808, 6297, 1, 500); -- Jackal Melee Helmet tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638810, 63808, 6323, 1, 500); -- Jackal Melee Leggings tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638811, 63808, 6375, 1, 500); -- Falcon Ranged Boots tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638812, 63808, 6349, 1, 500); -- Falcon Ranged Breastplate tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638813, 63808, 6401, 1, 500); -- Falcon Ranged Gauntlets tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638814, 63808, 6427, 1, 500); -- Falcon Ranged Helmet tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638815, 63808, 6454, 1, 500); -- Falcon Ranged Leggings tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638816, 63808, 6059, 1, 500); -- Praxis Defensive Boots tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638817, 63808, 5976, 1, 500); -- Praxis Defensive Breastplate tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638818, 63808, 6107, 1, 500); -- Praxis Defensive Gauntlets tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638819, 63808, 6140, 1, 500); -- Praxis Defensive Helmet tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638820, 63808, 6172, 1, 500); -- Praxis Defensive Leggings tc5, sell-back none
```

### List 63809: Omega Jaffa Crafting (Biomedical) (16 rows)

```sql
-- CIMMERIA_AUTHORING (D-UO8 option A, pending owner)
INSERT INTO item_lists (item_list_id, name) VALUES (63809, 'Omega Jaffa Crafting (Biomedical)');
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638901, 63809, 5188, 1, 75); -- Protein Complex (Bio-Medical) tc3, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638902, 63809, 5189, 1, 125); -- T1 Cell (Bio-Medical) tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638903, 63809, 5360, 1, 125); -- Rich Substrate (Bio-Medical Subcombine A) tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638904, 63809, 5362, 1, 125); -- Strombine (Bio-Medical Subcombine A) tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638905, 63809, 5363, 1, 200); -- Xeno-bacterium (Bio-Medical Subcombine B) tc8, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638906, 63809, 5364, 1, 200); -- Chiral Lysine (Bio-Medical Subcombine B) tc8, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638907, 63809, 5366, 1, 275); -- X-Cosmid (Bio-Medical Subcombine C) tc11, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638908, 63809, 5367, 1, 275); -- Mimic Rhinovirus (Bio-Medical Subcombine C) tc11, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638909, 63809, 5368, 1, 275); -- Hypnotic Fungus (Bio-Medical Subcombine C) tc11, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638910, 63809, 5370, 1, 325); -- Immunosuppressant (Bio-Medical Subcombine D) tc13, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638911, 63809, 5371, 1, 325); -- Protein Simplex (Bio-Medical Subcombine D) tc13, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638912, 63809, 5372, 1, 325); -- 3,4-COLA (Bio-Medical Subcombine D) tc13, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638913, 63809, 5447, 1, 125); -- Beta Biomedical Research Kit tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638914, 63809, 5369, 1, 125); -- BMAS-5 Field Crafting Tool tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638915, 63809, 8407, 1, 125); -- BMAS-10 Field Crafting Tool tc5, sell-back none
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (638916, 63809, 8408, 1, 375); -- BMAS-15 Field Crafting Tool tc15, sell-back none
```

## Decision or owner question

### Decisions (high confidence, structural)

1. One vendor bit per template, the bit matching the moniker (table above). `interaction_type.count_ones() == 1`.
2. Monikers: Tier 1 for weapons and armor, the one named specialty for crafting, 27264 for consumables.
3. `sell_item_list`, `repair_item_list` and `recharge_item_list` = 2, as template 1302 (reuses the existing arbitrage-guarded sell list).
4. Stock is limited to `flags & 1024` designs and excludes 21, 55 and 3437, so the guard passes without a list-2 change.
5. List and row id layout, at most 20 rows per list.

### Owner questions (recommended option first)

- **Q1 (D-UO8).** Stock and prices. Recommended: option A below.
- **Q2.** Price formula (200/100/25 x tc). Recommended: keep. It puts a tc5 rifle at 1000, the same as the Debug Area SMG, and a tc5 armor piece at 500. Alternative: copy the Debug Area scale (pistol 300, SMG 1000) per weapon class, or price everything at 1 as the hub fixtures do.
- **Q3.** Crafting specialty per vendor. Recommended: Human = Electronics (27249), Common = Materials (27239), Jaffa = Biomedical (27232). Reason: Materials is the shared base input, and the Jaffa Biomedical moniker is the only crafting moniker with a Jaffa-flavoured component set. Power Systems goes unsold in option A.
- **Q4.** Heavy weapons left out of Human Weapons (GL 21 Grenade Launcher 4990, GAU-2/A Minigun 6491, Baby Mongoose Flamethrower 6480) and Jaffa Rak'tel 5094 and Serpent Jaffa Cannon 6617, Serpent Ribbon Device 4565. Recommended: leave out of Tier 1.
- **Q5.** The 20-row cap per list is a packet limit, not an engine limit. Recommended: keep.

## D-UO8

> May Omega's vendors sell the stock and prices in UO-06-services.md (option A: one Tier-1 / one-specialty vendor per role), or option B (every tier and specialty the client names, which needs a new template block), or none?

| Option | UO-11 (Tau'ri) | UO-12 (Jaffa) |
|---|---|---|
| **A** (recommended) | Copies the five Tau'ri rows above: templates 63812-63816, lists 63802-63806. No new template ids. | Copies the three Jaffa rows: templates 63817-63819, lists 63807-63809. |
| **B** | Needs a new template block: 4 Human weapons tiers + 4 Human armor tiers + 4 Human crafting + 4 Common crafting + consumables = 17 Tau'ri templates, instead of 5, and a list per template (more than 63802-63806). The block must come from Omega's 63800-63999 reservation (an ID-plan change for the architect), and the Tier 2-4 stock must be authored (this worknote surveyed Tier 1 only). | 4 + 4 + 4 = 12 Jaffa templates instead of 3, same note. UO-12 re-sliced. |
| **None** | UO-11/12 seed no vendor templates or lists; vendor spawns drop out of UO-01's `svc.*` placements. | Same. |

## Concept art

Source: [omega-concept-art.md](../../omega-concept-art.md) (art counts below), UO-01 "Art agrees" column. The art has no crafting vendor and no Omega moniker for General, Adv. Weapon or Adv. Armor.

| Art kind | Art area | Spots | Template that would serve it | Closest shipped moniker | UO-01 key area (Art agrees) |
|---|---|---|---|---|---|
| General | Common Area | 3 | none | no Omega moniker (Harset 8006 `DN_npc_ven_TBD_Harset_ConsumablesGeneral` is the only general-goods moniker, not Omega's) | none |
| General | SGC wing | 3 | none | no Omega moniker | none |
| General | Free Jaffa wing | 2 | none | no Omega moniker | none |
| Consumable | Common Area | 3 | 63816 | 27264 Consumables | `svc.consumables` is in sgc (n/a) |
| Consumable | SGC wing | 3 | 63816 | 27264 Consumables | sgc (n/a) |
| Consumable | Free Jaffa wing | 2 | 63816 | 27264 Consumables | sgc, not free_jaffa (n/a) |
| Weapon | Common Area (hub, east pod) | 2 | 63812 or 63817 | 27240 Human Weapons Tier01, 27224 Jaffa Weapons Tier01 | `svc.human_weapons` sgc, `svc.jaffa_weapons` free_jaffa; neither is common_area |
| Weapon | SGC wing | 2 | 63812 | 27240 Human Weapons Tier01 | sgc (yes) |
| Weapon | Free Jaffa wing | 1 | 63817 | 27224 Jaffa Weapons Tier01 | free_jaffa (yes) |
| Armor | Common Area (hub, east pod) | 2 | 63813 or 63818 | 27244 Human Armor Tier01, 27228 Jaffa Armor Tier01 | neither is common_area |
| Armor | SGC wing | 2 | 63813 | 27244 Human Armor Tier01 | sgc (yes) |
| Armor | Free Jaffa wing | 1 | 63818 | 27228 Jaffa Armor Tier01 | free_jaffa (yes) |
| Adv. Weapon | Common Area (north-west pod) | 1 | none | no Omega moniker (closest 27241-27243 Human Weapons Tier02-04) | none |
| Adv. Armor | Common Area (north-west pod) | 1 | none | no Omega moniker (closest 27245-27247 Human Armor Tier02-04) | none |
| Crafting (Human, Common, Jaffa) | none | 0 | 63814, 63815, 63819 | 27248-27251, 27236-27239, 27232-27235 | no art spot |

Finding: UO-01 places `svc.consumables` and `svc.common_crafting` in the SGC wing (Art agrees n/a), while D-UO16 option A as worded stands Consumables and Common Crafting in the Common Area. Option A needs those two coordinates re-picked in the Common Area, or the wording read as "as UO-01 placed them". `svc.banker` (not a vendor) is UO-01's only disagreement.

## D-UO16

> The [concept art](omega-concept-art.md) draws General, Consumable, Weapon and Armor vendors in every area (8/8/5/5), Adv. Weapon and Adv. Armor, and no crafting vendor. Which roster does Omega get? **A:** the plan as written, eight moniker-led templates with one spawn each, standing in their faction's art area (Human in the SGC wing, Jaffa in the Free Jaffa wing, Common Crafting and Consumables in the Common Area). **B:** A, plus extra spawns of the same templates at the art's other spots of that kind; same templates and lists, new spawn ids, and the architect re-slices UO-11/12. **C:** B, plus authored General, Adv. Weapon and Adv. Armor vendors, which have no Omega moniker: new templates, names and stock, CIMMERIA_AUTHORING, scaled under U-D07. In every option: the art draws no crafting vendor, so do the three crafting templates stay, and where?

No recommendation. Impact on UO-11/12:

| Option | New spawn rows | New templates | UO-11/12 change |
|---|---|---|---|
| **A** | none beyond UO-01's eight `svc.*` spawns (one per template) | none | UO-01's `svc.consumables` and `svc.common_crafting` coordinates must move to the Common Area, or the owner reads A as "where UO-01 put them". |
| **B** | extra spawns of templates 63812-63819 at the art's other spots: Weapon 3 more (5 art spots, 2 served by A), Armor 3 more (5, 2 served), Consumable 7 more (8 art spots, 1 served). New spawn ids, same templates and lists. Spawn ids come from UO-01's block, so UO-01 is reopened. | none | Architect re-slices UO-11/12 (spawns are a W1/W2 placement concern, not template rows). |
| **C** | B's spawns, plus General 8, Adv. Weapon 1, Adv. Armor 1 | at least three new templates (General, Adv. Weapon, Adv. Armor), each with a new name, a new list and new stock; ids from Omega's 63800-63999 reservation, so an ID-plan change | New templates scaled under U-D07; all CIMMERIA_AUTHORING; UO-06 would need to author three more lists. |
| Crafting (all options) | If kept: the three crafting spawns stay where UO-01 put them (SGC and Free Jaffa wings; Common Crafting is also in the SGC wing). If dropped: remove `svc.human_crafting`, `svc.common_crafting`, `svc.jaffa_crafting`. | none | If dropped, lists 63804, 63805, 63809 and templates 63814, 63815, 63819 drop out of UO-11/12. |

## Telemetry

Each file:line still holds (verified in this worktree). The spans carry only identity fields (`entity_id`, `player_id`, `vendor_entity_id`, `vendor_template_id`; `items_len` on purchase and sell). They record no outcome field. The success outcome is a separate INFO log event, target `vendor`, from `vendor/telemetry.rs`:

| Span | Location | Success log (`event`, `action`) | Emitted at |
|---|---|---|---|
| `vendor.open_store` | `crates/base-methods/src/base/world_entry/methods/vendor/store.rs:32` | `store_opened`, action `open`; fields `buy_count`, `sell_count`, `buyback_count`, `repair_count`, `recharge_count`, `vendor_template_id`, `vendor_template_name` | `store.rs:156` (`tel.opened`), defined `telemetry.rs:260` |
| `vendor.purchase` | `purchase/mod.rs:37` | `transaction`, action `buy`; fields `item_type_id`, `item_name`, `item_id`, `quantity`, `price`, `lines`, `cash_before`, `cash_after` | `purchase/mod.rs:328` (`tel.completed`) |
| `vendor.sell` | `sell/mod.rs:38` | `transaction`, action `sell`; same fields, `price` is the total cash gained | `sell/mod.rs:400` (`tel.completed`) |

Failure lines: `refused` (a player-caused reason, nothing written) and `failed` (WARN with error), for example `vendor_buys_nothing`, `empty_request`, `buyback_full`, `db_error`, `template_lookup_failed`, `no_database`, `inventory_lock_failed`, `commit_failed`. For Omega a `store_opened` with `buy_count` = the list's row count (17, 20, 16, 16, 10, 20, 20, 16) is the visible proof the list loaded.

## Required tests (for UO-11/12)

- The existing live-DB `live_db_vendor_arbitrage` guard must stay green with the new lists loaded.
- A live-DB seed test (UO-11/12 own it): each of the eight templates has exactly one vendor bit, class `mob`, faction not 10, sell/repair/recharge lists = 2, and every list row's `item_id` is in its range and has `naquadah` at least 1.
- Revert proof is theirs; this packet changes no behaviour.

## Handoffs

- Architect: D-UO16 options B and C re-slice UO-01, UO-11 and UO-12.
- database-persistence: nothing now; UO-11/12 land the seed rows.
- combat-systems-advisor: the `tc`-as-level question (Q2 note) matters if armor/weapons are to be level-gated.
