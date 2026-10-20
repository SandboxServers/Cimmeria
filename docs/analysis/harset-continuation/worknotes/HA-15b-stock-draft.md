# HA-15b stock draft: one stock per (faction, role, tier)

> Type: offline draft worknote (packet PU-08). Audience: the maintainer and owner reviewing D-PU2, then the HA-15b seal. Updated: 2026-10-19.
> Status: **PROVISIONAL** (O18-13). Every item pick, ratio and rule below is an AI-derived working draft; none was checked in game. Docs only: no seed is written, no ID is allocated, no price is set, no lab or client was used.
> Inputs: [HA-15f](HA-15f-omega-reference.md) (Omega pattern, sections 5, 6.2 and 7), [HA-15g](HA-15g-service-npc-placement.md) (placement plan, sections 6, 7 and 9), [HA-15 audit](HA-15-audit.md) (sections 4 and 7), [owner decisions](owner-decisions-2026-10-18.md) (O18-04, O18-13), and the item seed `db/resources/Items/Seed/items.sql`.
> Bounded by O14-12 (tier-comparable stock, no new currency, no high-tier gear). Prices are not set here (D-PU2).

Label key. FACT is a seed row or a document line. INFERENCE carries a confidence and its reasoning. PROVISIONAL is a draft for review. MAINTAINER DECISION and OWNER DECISION are not mine to make.

## 1. Summary

1. FACT: the seed can fill 11 of the 12 weapons and armor stocks (3 factions x weapons and armor x tiers 1 and 2) from existing items. The twelfth, the Goa'uld armor tier 2 stock, has no seeded item to fill it (section 4.3).
2. FACT: Goa'uld armor is thin even at tier 1: three clothing pieces (vest, pants, boots), no head and no hands piece, against five slots for Human and Jaffa. Count parity between factions is therefore broken for Goa'uld armor, not by design but by data (section 4.3).
3. PROVISIONAL: price ladder, tier 1 at 1.0x and tier 2 at 2.5x, applied per row. Two base units (weapon, armor) are left symbolic for the owner (D-PU2). No naquadah figure is proposed.
4. FACT: nothing here is purchasable in game yet. Harset has 46 spawns and 0 service spawns, and the Market and Storage Room have no spawns ([HA-15 audit](HA-15-audit.md) section 3).
5. Blocked: owner prices (D-PU2), faction gating (maintainer), placement probes, the treat item 4512 versus 2714 (HA-15a). Section 7.

## 2. Method and what it rests on

1. FACT: I parsed every `INSERT INTO items` statement in `db/resources/Items/Seed/items.sql` with a quote-aware tokenizer (multi-line descriptions included). 6,059 rows, all parsed, none skipped. Scratch only, nothing committed.
2. FACT: the items table has no faction column and no price column. The columns this draft relies on are `tier`, `tech_comp`, `container_sets`, `visual_component`, `discipline_ids`, `quality_id`, `max_stack_size`, `ammo_types` and `clip_size`.
3. INFERENCE (medium): faction is read from the `visual_component` folder prefix, the same convention HA-15f used for the client's vendor monikers. `WP-Human` and `AR_H_*` are Human, `WP-Jaffa` and `AR_J_*` are Jaffa, `WP-Goauld` and `AR_G_*` are Goa'uld. Reasoning: nothing else in the row names a faction.
4. FACT: all 6,059 item tiers fall in 1 to 5, and `max_stack_size` is 1 on every weapon and armor row checked, which fits the vendor engine ignoring `max_stack_size` (one unit per list row, [HA-15f](HA-15f-omega-reference.md) section 9).
5. FACT: prices are not in `items`. A vendor price is the `naquadah` column of an `item_list_items` row. The only seeded vendor lists are the debug ones: lists 1 and 2 (0 to 1,000), list 310 (every row 1 naquadah) and list 1300. There is no tier-priced list to copy ([HA-15f](HA-15f-omega-reference.md) section 7).

## 3. What tier means in the seed

FACT, from the parsed rows (weapons and armor of the three factions only):

| Tier | `tech_comp` range seen | Quality |
|---:|---|---|
| 1 | 1 to 23 (23 only in the Human blade family, item 3333) | all Normal |
| 2 | 5 to 33 (5 only in the Human SMG family, item 4739) | all Normal |
| 3 to 5 | 33 and above (not read further) | out of scope under O14-12 |

- FACT: `tech_comp` 20 occurs in both tier 1 and tier 2 rows, so tier is not a pure function of `tech_comp`.
- INFERENCE (medium): `tech_comp` is the within-tier rung and the closest thing the seed has to an item level. Reasoning: within one visual family the same piece repeats at `tech_comp` 5, 10, 13, 15, 18, 20 for tier 1 and 20, 23, 25, 28, 30, 33 for tier 2.
- FACT: Harset level bands per tier are not recorded anywhere ([HA-15f](HA-15f-omega-reference.md) section 8, item 5). `tech_comp` is therefore a proxy only, and every rung below is a placeholder until HA-15b reads the Harset mission reward levels.
- FACT: `discipline_ids` differ by item and overlap across factions. For example the Human Standard Pistol carries disciplines 21 to 25, the same set as the Jaffa Serpent Staff, and the Human Standard SMG carries 59 to 65, the same set as the Jaffa Cat armor. INFERENCE (low): `discipline_ids` is the archetype or skill gate, so the visual-folder faction in section 2 point 3 is a look, not a usage restriction. I did not verify the discipline meaning against a `.def` or the client; do that before any claim about who can equip what.

## 4. The stocks

### 4.1 Rules that make a stock

PROVISIONAL, all applied below.

1. One stock is a set of `item_list_items` rows for one (faction, role, tier). A vendor template carries one `buy_item_list`, so a faction's weapons vendor list is its tier 1 stock followed by its tier 2 stock, and each vendor shows both. This matches the six weapons and armor vendors of [HA-15g](HA-15g-service-npc-placement.md) section 7.
2. Weapons stock is 3 rows per tier. Armor stock is one row per equipment slot at one rung, up to 5 rows.
3. Armor slots, by `container_sets` code: 4, 7, 8, 11 and 12. INFERENCE (medium): head, torso, hands, legs and feet. Reasoning: the Jaffa visual names end in H, T, G, L and B (`AR_JM_FH1`, `FT1`, `FG1`, `FL1`, `FB1`) and map one to one onto codes 4, 7, 8, 11 and 12. Verify against the item slot enum before the seal.
4. Every row `quantity = 1`.
5. Normal quality only. No Poor, Good, Great or Fantastic rows.
6. No placeholder or prisoner kit. Items 3437, 3438, 3440 and 5865 (`AR_Global.Prisoner_*`) are excluded even though they are tier 1; 3437 is already on the debug list 2.
7. Tier 1 and tier 2 only (O14-12). No tier 3 to 5 row.
8. Rung choice: tier 1 around `tech_comp` 10 to 18, tier 2 around 23 to 28. Placeholder, see section 3.

### 4.2 Weapons stocks

All item IDs are existing `items.sql` rows (FACT). Rung is `tech_comp`.

| Faction | Role | Tier | Rows (item id, name, rung) | Why these |
|---|---|---:|---|---|
| Human | Weapons | 1 | 3240 SI 3 9mm Pistol (15), 3156 MPX 77 SMG (15), 3166 AR 21 Assault Rifle (15) | Three families at one rung, picked from the nine tier 1 Human weapon families in the seed (AR, Dartgun, GL, HumanBlade, LMG, Pistol, Rifle, Shotgun, SMG): pistol (disciplines 21 to 25), SMG (59 to 65), assault rifle (40 to 49). Three families because the discipline sets differ, and one family at three rungs would serve one skill set only |
| Human | Weapons | 2 | 3244 SI 4 9mm Pistol (25), 3160 MPX 77a SMG (25), 3170 AR 21A Assault Rifle (25) | Same families, tier 2 rung 25 |
| Jaffa | Weapons | 1 | 3473 Serpent Staff (10), 3475 Serpent Staff (15), 3476 Serpent Staff (18) | One family, three rungs. The seed holds one Jaffa tier 1 family |
| Jaffa | Weapons | 2 | 3478 Venom Staff (23), 3479 Venom Staff (25), 3480 Venom Staff (28) | One family, three rungs |
| Goa'uld | Weapons | 1 | 5098 Rak'tel (10), 5100 Rak'tel (15), 5101 Rak'tel (18) | One family, three rungs |
| Goa'uld | Weapons | 2 | 5104 Rak'fa (23), 5105 Rak'fa (25), 5106 Rak'fa (28) | One family, three rungs |

- FACT: the Jaffa and Goa'uld folders each hold a single tier 1 and tier 2 family. Goa'uld has the Ashrak blade only. Jaffa has the plasma staff, plus item 3904 Shriek Zat'ni'katel (tier 1, rung 5), whose `discipline_ids` (78 to 85) match the Goa'uld blades, not the staffs. I left 3904 out of the Jaffa stock for that reason (INFERENCE, low).
- INFERENCE (medium): row counts are equal (3 per stock) but breadth is not. A Human buyer sees three different weapon types; a Jaffa or Goa'uld buyer sees one weapon at three rungs. That is a property of the seed, not a choice; widening the Jaffa and Goa'uld families needs new items and is out of scope here (no new items under O14-12's no-high-tier rule is not the blocker, authoring effort and an owner call are).
- FACT: ranged weapons here take `Bullet_Default` ammo with clip 15 to 50 (Human) or clip 0 (staff, blade). Ammo supply is not part of any stock in this draft. Clip size is DB-sourced and must not be hardcoded. Whether Harset sells ammo is open (section 7.5).

### 4.3 Armor stocks

| Faction | Role | Tier | Rows (item id by slot: head, torso, hands, legs, feet) | Family and rung |
|---|---|---:|---|---|
| Human | Armor | 1 | 2930, 2942, 2977, 3002, 2960 | Ballistic, rung 15 |
| Human | Armor | 2 | 2934, 2946, 2981, 3010, 2963 | Ballistic, rung 25 |
| Jaffa | Armor | 1 | 6431, 6353, 6405, 6458, 6379 | Falcon, rung 15 |
| Jaffa | Armor | 2 | 8404, 6218, 6279, 6332, 6249 | Cat, rung 25 |
| Goa'uld | Armor | 1 | torso 4339 Goa'Uld Vest, legs 4338 Goa'Uld Pants, feet 4337 Goa'Uld Boots (3 rows) | `AR_G_Clothing00`, rung 1 |
| Goa'uld | Armor | 2 | none | no seeded item |

- FACT: Human and Jaffa have complete five-slot sets at both tiers in the families above. I checked that each id sits in the intended slot code and rung.
- FACT: Jaffa changes family between tiers. Tier 1 is Falcon ranged (disciplines 21 to 25, 32) and tier 2 is Cat melee (disciplines 59, 63, 66, 68), so a buyer who wants ranged Jaffa armor at tier 2 finds none in this stock. The seed also holds Jackal, Naga, Praxis and Standard at tier 1, and Asian, Horse and Viking at tier 2. Keeping Falcon at tier 1 and Cat at tier 2 is a choice this draft makes by default; a same-family tier 2 or a different tier 1 family is open for the owner.
- FACT: Goa'uld tier 1 is three clothing pieces at `tech_comp` 1, with no head piece and no hands piece. There is no Goa'uld tier 2 armor of any kind in the seed (`AR_G_*` is 3 rows in total, 4337 to 4339, plus the four `AR_Global.Prisoner_*` rows excluded in section 4.1).
- PROVISIONAL consequence: Goa'uld armor is the single stock that cannot reach faction parity (3 rows against 5, and no tier 2). Options, none chosen. (a) Ship the three-row tier 1 stock and no tier 2 until items are authored. (b) Hold the Goa'uld armor vendor until an item packet authors a Goa'uld armor family. (c) Let the Lo'taur armor vendor sell a Jaffa or Human family, which breaks the one-server-per-faction identity of [HA-15f](HA-15f-omega-reference.md) section 5.1. I lean (a) with (b) as the fallback, because (c) undoes the reasoning O18-04 asked to have on record. This is an OWNER DECISION with a MAINTAINER view on (c).
- INFERENCE (low): because Goa'uld clothing is rung 1 while the other tier 1 armor is rung 15, the Goa'uld vendor is not tier-comparable even at tier 1. The rung gap is a data limit.

### 4.4 Roles not drafted

| Role (HA-15f section 6.2) | State |
|---|---|
| Consumables (row 7) | No tier axis; not read. Needs a list built from the consumables seed and the [native consumables](../../../../.claude/agent-memory/items-systems-advisor/native_consumables_and_stat_buffs.md) caveats (placeholder ability 597 gate). Out of this draft |
| Crafting components and common crafting (rows 8 and 9) | Faction-neutral or per discipline, no weapons-style tier. The debug crafting list 310 (24 rows, every row 1 naquadah) is the only precedent and is a debug price, not an anchor ([HA-15g](HA-15g-service-npc-placement.md) section 9) |
| Mission-reward vendors (row 10) | Moniker 28199 is shared by all three; reward-class stock cannot be chosen before the mission rewards are fixed (HA-15d) |
| Sell, repair and recharge lists | Debug list 2 is shared by every debug vendor. A Harset vendor either reuses list 2 or gets its own; not decided |

## 5. Price ladder (PROPOSAL only)

PROVISIONAL. This proposes a shape. It sets no naquadah figure. The owner sets the base units (D-PU2) inside O14-12.

### 5.1 Shape

| Symbol | Meaning |
|---|---|
| `Pw` | base price of one weapons row at tier 1 |
| `Pa` | base price of one armor row at tier 1 |
| tier 1 multiplier | 1.0x (proposed) |
| tier 2 multiplier | 2.5x (proposed) |

Per-row price, applied flat across every row of a stock:

| Stock | Per-row price |
|---|---|
| Any weapons stock, tier 1 | `Pw` x 1.0 |
| Any weapons stock, tier 2 | `Pw` x 2.5 |
| Any armor stock, tier 1 | `Pa` x 1.0 |
| Any armor stock, tier 2 | `Pa` x 2.5 |

### 5.2 What the shape gives

| Stock | Rows | Cost to buy every row |
|---|---:|---|
| Human, Jaffa or Goa'uld weapons, tier 1 | 3 | 3.0 `Pw` |
| Human, Jaffa or Goa'uld weapons, tier 2 | 3 | 7.5 `Pw` |
| Human or Jaffa armor, tier 1 | 5 | 5.0 `Pa` |
| Human or Jaffa armor, tier 2 | 5 | 12.5 `Pa` |
| Goa'uld armor, tier 1 | 3 | 3.0 `Pa` |

- PROVISIONAL reasoning for flat pricing: with no faction gate any player can open any vendor, so equal per-row prices at equal tier are what stop one faction's vendor from being strictly cheaper ([HA-15f](HA-15f-omega-reference.md) section 7). It also keeps the proposed `harset_vendor_faction_parity` test simple.
- INFERENCE (medium): 2.5x is inside O14-12 because it adds no currency and no tier above 2. Whether 2.5x is the right slope is not derivable from the seed (section 2 point 5); the owner may prefer a flatter or steeper ladder. A rung-by-rung gradient inside a tier (price rising with `tech_comp`) is possible and not drafted.
- Weakness to carry: the Goa'uld armor stock costs 3.0 `Pa` against 5.0 `Pa` for the others, so the parity test as worded in HA-15f ("equal row counts and equal total naquadah") fails by construction until the Goa'uld armor gap is closed or the test is relaxed for that stock.
- Open: `Pw` and `Pa` should come from the naquadah Harset missions pay, not from the debug rows. That figure has not been read (no Harset mission reward naquadah survey exists in the ledger). A survey is the first input for D-PU2.

## 6. Hazards carried into the seal

FACT, from [HA-15f](HA-15f-omega-reference.md) section 9, [HA-15g](HA-15g-service-npc-placement.md) section 2 and the [HA-15 audit](HA-15-audit.md).

1. No vendor faction gate in the engine: a Human can buy from the Lo'taur. Stock is flavour, not a lock.
2. Vendors ignore `max_stack_size`; one unit per list row.
3. Purchases land in bag 1. A full bag 1 rejects the buy; a five-row armor set needs five free slots.
4. A quest speaker must carry no vendor fields (talk-then-trade needs two templates).
5. A service NPC must not be `faction = 10`.
6. `item_lists` ids are FK-checked and the list sequence floor is 329, so new lists start above it. Every `design_id` must exist in `items`; all IDs in section 4 do (FACT, read from the parsed seed).
7. No mission-only item on a general vendor. None of the section 4 IDs is a mission item. The treat items are not in any stock here (section 7.4).
8. Double-consume trap: not triggered by a plain vendor stock, because buying grants through the vendor path and not through a `remove_item` chain. It becomes live only if a stocked item is also wired through a UseInventoryItem or `items_event_sets` chain. None of the section 4 items is a consumable; I did not check `items_event_sets` rows for them. INFERENCE (low), verify with a Grep of `items_event_sets.sql` for these IDs before the seal.

## 7. What is still blocked

### 7.1 Owner prices (D-PU2)

- `Pw`, `Pa`, the 2.5x slope, and whether prices rise with `tech_comp` inside a tier. Inside O14-12: no new currency, no high-tier gear.
- The input that should come first: a survey of what Harset missions pay in naquadah, per level band (not done).
- The sell, repair and recharge price side, which uses different lists and is not drafted.

### 7.2 Faction gating (maintainer call)

- Engine fact: no gate. Options on the table ([HA-15g](HA-15g-service-npc-placement.md) section 6): no gate with strict parity (recommended there), a dialog-side check on the speaker, or a future engine gate.
- Dependence of this draft on the answer: with no gate, the parity rule in section 5 is the only coherence tool, and the Goa'uld armor gap (section 4.3) matters. With a gate, parity matters less and the gap matters more, because a Goa'uld player could not buy armor elsewhere.
- Do not ship stock that is only safe if isolated.

### 7.3 Placement probes

None of these stocks can be sold until vendors exist. Open from [HA-15g](HA-15g-service-npc-placement.md) section 8:

- Market has two usable crafting sites; more probes needed. World 69 has zero spawns.
- No `find_path` run from any arrival to any candidate; no clearance check beyond two known hazards.
- The bank probe conflict (spawn 314) is unresolved.
- The Lo'taur clone (template 353 look) has never rendered; fallback is a Goa'uld Praxis look. The Goa'uld weapons and armor vendors depend on it.
- Template capacity: about 19 templates needed against 9 candidates (261 to 269). No ID is allocated here.

### 7.4 HA-15a treat: item 4512 versus 2714

- FACT ([HA-15 audit](HA-15-audit.md) section 4): 4512 is `"Death by Chocolate"` and 2714 is `Delicious Confection`. Both exist in `items.sql`; neither has a grant, reward, loot, vendor or `items_event_sets` source.
- This draft puts neither item in any stock. That is deliberate: choosing a vendor purchase for the treat is an owner decision (HA-15a), the weapons and armor vendors are the wrong shop for it, and O18-03 requires a real collectible item with no invisible credit.
- If the owner picks "bought", the treat needs its own stock (probably a consumables or mission vendor) priced and placed in the Market, with the same bag-1 and `max_stack_size` rules. If "granted", it is a chain grant and no stock changes. Either way, whichever item is picked, a visible grant must precede any equip or use signal.

### 7.5 Smaller open points

1. Ammo: whether the Human ranged stocks need an ammo row, and its quantity rule. Tied to the ammo campaign; not decided.
2. `discipline_ids` meaning (section 3) is unverified. Re-check against the defs before telling a player which class a vendor serves.
3. Slot code meaning (section 4.1, point 3) is inferred from visual names.
4. Rungs are placeholders until Harset level bands are read.
5. Vendor text: the Human and Jaffa tier vendor monikers have empty text rows in `texts.sql` ([HA-15f](HA-15f-omega-reference.md) section 8); names are authoring work.

## 8. Tests the seal would owe

Proposals, not written. Each must fail when its fix is reverted ([TESTING.md](../../../../TESTING.md)).

| Test (proposed) | Asserts | Revert that must fail it |
|---|---|---|
| `harset_vendor_lists_resolve` | every list id exists in `item_lists` and every `design_id` exists in `items` | point a row at a deleted item |
| `harset_vendor_rows_single_quantity` | every Harset vendor row has `quantity = 1` | set one row to 20 |
| `harset_vendor_tier_cap` | no Harset vendor row has `items.tier` above 2 | add a tier 3 weapon |
| `harset_vendor_no_placeholder_kit` | no Harset vendor row is in the `AR_Global.Prisoner_*` set (3437, 3438, 3440, 5865) | add 3437 to a list |
| `harset_vendor_faction_parity` | same-role same-tier stocks have equal row counts and equal total naquadah, with the Goa'uld armor stock listed as a known exception until the gap closes | delete one Jaffa weapons row |
| `harset_vendor_buy_lands_in_bag1_and_charges` (live db) | one purchase removes the price and places the item in bag 1 | change the buy path to fill bag 2 |

## 9. Hand-offs and next step

1. Owner: review D-PU2 (section 7.1) and the Goa'uld armor option (section 4.3); the treat choice (section 7.4).
2. Maintainer: faction gate (section 7.2).
3. items-systems-advisor, at the seal: re-verify slot codes and `discipline_ids`, grep `items_event_sets.sql` for the section 4 IDs, and turn this into the sealed stock and price note once D-PU2 returns.
4. npc-ai-spawn-advisor and HA-15g: placement probes (section 7.3).
5. This note allocates no IDs and edits no seed. A later seed packet takes IDs from [id-allocation](../id-allocation.md) once the coordinator widens the template range.
