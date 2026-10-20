# HA-15f: Omega Site as the population, vendor and trainer reference for Harset

> Type: research worknote. Audience: HA-15b (vendors), HA-15c (trainers), HA-15g (trainer design), HA-16 and HA-01a (placements). Updated: 2026-10-18.
> Status: everything below is PROVISIONAL and `CIMMERIA_RECONSTRUCTION` or `CIMMERIA_AUTHORING` unless a row says `RESTORED_SOURCE`. Nothing here sets a price, allocates an ID or edits a seed.
> Decision: [O18-04](owner-decisions-2026-10-18.md) (Omega as layout reference, setting-coherent vendors, record the reasoning), bounded by O14-12 (tier-comparable stock, no new currency, no high-tier gear).
> Gates: [HA-15 audit](HA-15-audit.md), [ID candidates](../id-allocation.md), [work packets](../work-packets.md).

Label key. FACT cites a file and line or row. INFERENCE carries high, medium or low confidence with its reasoning. DECISION cites an O14 or O18 ID. PROVISIONAL means a working draft for in-game review.

## 1. Headline findings

1. FACT: the seed holds almost no Omega population. Worlds 18 and 80 carry 8 spawn rows and not one vendor, trainer, banker or registrar (section 2). "Extracting the Omega layout" therefore cannot mean reading placements from the seed.
2. FACT: the client's own text table names the intended Omega and Harset service roster, vendor by vendor and trainer by trainer (section 3). That roster is the strongest layout evidence available, and it is richer than the seed.
3. FACT: the client already names who serves Goa'uld-facing trade at Harset: the **Lo'taur**, a Goa'uld servant title (section 5). The answer to the owner's question is therefore Lo'taur for Goa'uld goods and Jaffa quartermasters for Jaffa goods, not a Jaffa serving Goa'uld goods.
4. INFERENCE (high): Omega is a three-resident-faction hub (SGU Human, Free Jaffa, Asgard) and Harset a three-resident-faction hub (OP-Core Human, Loyalist Jaffa, Praxis Goa'uld). The per-faction vendor pattern transfers one for one, with Asgard programs mapping to Goa'uld goods.
5. FACT: Harset has 46 spawns and 0 service spawns, and the Market (69) and Storage Room (70) have no spawn at all ([HA-15 audit](HA-15-audit.md) lines 38-43). Every placement below is gated on HA-01a doors and HA-16 placement evidence.
6. Not done: no map-actor extraction was run over the Omega_Site tiles (section 8). The monikers and the Dev Master supply the layout; coordinates stay unproven.

## 2. Census of the Omega worlds in the seed

| World | Name | Spawn rows | What they are | Evidence |
|---|---|---:|---|---|
| 18 | Omega_Site | 7 | spawn 78 Modi (template 65), spawns 132-136 beam transporters (template 3), spawn 41 DHD (template 1) | FACT `db/resources/Worlds/Seed/spawnlist.sql`; [zone audit](../../../content/zone-audit.md) lines 498-521 |
| 80 | Omega_Site_CmdCenter | 1 | spawn 137 beam (template 3) | FACT `spawnlist.sql` |
| 81, 82 | Ruins, StorageRm | 0 | none | FACT `spawnlist.sql` (no rows) |

- FACT: worlds 18 and 80 are `navmesh_mode = 'advisory'` with NA26 meshes `data/spaces/omega_site.nav` and `omega_site_cmdcenter.nav` (`db/resources/Worlds/Seed/worlds.sql` lines 123-127 and 275-279). A service NPC placed there would be walk-checked advisory only.
- FACT: the seed vendor, banker, registrar and trainer templates were written for the Debug Area and carry Omega moniker provenance in their comments, not Omega placements: template 314 (crafting supplies, moniker 27239, buy list 310), templates 370/371/372 (bankers, moniker 29462, vault scope personal/team/command), 330/331 (registrars, moniker 29068), 360 (pet trainer, moniker 8000, trainer list 350), 305 (auctioneer), 390 (mail clerk). `db/resources/Entities/Seed/entity_templates.sql`.
- FACT: the Dev Master lists no vendor or trainer placement. Sheet 08 (leadership) names Jackson, Bra'tac, Teal'c, Thor, Heimdall, Hammond, Carter, O'Neill, Modi, Ormak, Markak, Gru'blak and Grant; sheet 12 carries area coordinates the sheet itself labels reconstruction. `docs/analysis/sgw-handoff-pack-v1.2/pack/references/world_content/SGW_Omega_Site_Dev_Master_v1.md`.
- INFERENCE (medium): the Dev Master leadership list is the Omega quest-and-lore cast. The service roster of section 3 is a separate layer that sat beside it, which is why no sheet places it. Reasoning: the monikers use a `DN_npc_ven_` / `DN_npc_trn_` prefix, distinct from the named-character `DN_NPC_` form.

## 3. The service roster the client names

All rows FACT from `db/resources/Texts/Seed/texts.sql`; the line numbers are the first and last rows of each block.

### 3.1 Omega (lines 37034-37137)

| Role | Pattern | Count |
|---|---|---:|
| Weapons vendor | Jaffa / Human weapons tiers 1-4; Asgard "programs" tiers 1-4 | 12 |
| Armor vendor | Jaffa / Human / Asgard armor tiers 1-4 | 12 |
| Crafting-component vendor | one per faction per discipline (4 disciplines) | 12 |
| Common crafting vendor | one per discipline, faction-neutral | 4 |
| Consumables vendor | moniker 27264 | 1 |
| Trainer | Asgard Basic Skills 27218, Asgard Advanced Skills 27219 | 2 |
| Registrar | TeamCommandRegistrar (line 41036) | 1 |

### 3.2 Harset, shipped tier layout (lines 39042-39140, 41038, 41534)

| Role | Pattern | Count | Text state |
|---|---|---:|---|
| Armor vendor | Human / Goauld / Jaffa tiers 1-4 | 12 | Goauld rows have English text; Human and Jaffa rows are empty |
| Weapons vendor | Human / Goauld / Jaffa tiers 1-4 | 12 | same |
| Crafting-component vendor | Jaffa / Human / Goauld x 4 disciplines | 12 | all have text |
| Common crafting vendor | Biomedical, PowerSystems, Materials, Electronics | 4 | all have text |
| Mission-reward vendor | Human ("Op-Core Mission Reward"), Goauld, Jaffa | 3 | Goauld empty |
| Registrar | PvPRegistrar (41038) | 1 | present |
| Test vendor | Harset_BetaCompVendor (41534) | 1 | present |

### 3.3 Harset, earlier "TBD" layout (lines 55020-55132)

- Basic and Rare weapons and armor vendors for Tau'ri, Jaffa and Goa'uld: 12 monikers (7968-7979), all with text.
- ConsumablesGeneral (8006) and ConsumablesMinigame (8007).
- Six Global basic trainers: Soldier, Scientist, Jaffa, Goa'uld, Commando, Archaeologist (7988-7993).
- Eleven advanced trainers: Soldier Advanced; Scientist Advanced and Support; Jaffa Tau'ri and Jaffa Advanced; Goa'uld Ashrak and Goa'uld Advanced; Commando Advanced and Infiltration; Archaeologist Advanced and Support (7994-8004).
- Nine mission-token vendors (8009-8017), see section 5.

INFERENCE (medium): the "TBD Basic/Rare" vendors are an earlier design, superseded by the tier 1-4 vendors of 3.2, and the mission-token vendors are a third, mission-driven layer. Reasoning: three naming generations coexist, the TBD set carries the literal word TBD, and only the tier set matches Omega's pattern. HA-15b should treat 3.2 as the stock model and 3.3 trainers and mission-token vendors as the surviving layers.

## 4. The service-layout pattern

PROVISIONAL, `CIMMERIA_RECONSTRUCTION`.

1. INFERENCE (high): one commerce cluster per hub, grouped by faction side. The roster is faction-first (Jaffa, Human, Asgard blocks) and discipline-second. Reasoning: every vendor moniker is `<faction><kind>Tier<n>`.
2. INFERENCE (medium): the command building holds the organization services. The Omega registrar is a TeamCommand registrar, and `Omega_Site_CmdCenter` (world 80) exists. Reasoning: the name pairs a command function with that interior; the placement itself is not recorded.
3. INFERENCE (medium): trainers are faction-and-archetype specific, a basic and an advanced trainer per resident faction. Omega shows this at its smallest (Asgard 27218 and 27219); Harset's TBD list shows it at full width.
4. INFERENCE (low): bankers and the auctioneer sit at the hub next to the commerce cluster. Reasoning: only the debug-area templates exist; no Omega moniker confirms placement.
5. FACT: vendors and trainers are stationary service NPCs. The seed service templates carry no wander, patrol or leash columns (templates 314, 330, 331, 360, 370-372, `entity_templates.sql`).

## 5. Setting-coherent vendor identity and the Goa'uld-facing question

This section is the reasoning the owner asked to have on record for in-game review. DECISION O18-04. All PROVISIONAL, `CIMMERIA_AUTHORING` unless stated.

### 5.1 Who serves what

| Goods | Server NPC | Basis |
|---|---|---|
| Jaffa weapons and armor | Jaffa quartermaster | FACT moniker 8016 "Jaffa Armor Quartermaster", 8017 "Jaffa Weapons Quartermaster", 8010 "Basic Equipment Quartermaster" (Jaffa tier-0) |
| Human (Tau'ri) weapons and armor | OP-Core procurement officer | FACT monikers 8014, 8015 "Weapons/Armor Procurement Officer", 8009 "Basic Equipment Procurement Officer", 28199 "Op-Core Mission Reward" |
| Goa'uld weapons and armor | **Lo'taur** | FACT monikers 8011 "Basic Equipment Lo'taur", 8012 "Goa'uld Weapons Lo'taur", 8013 "Goa'uld Armor Lo'taur" |
| Common crafting components | neutral merchant | FACT four "Common" crafting monikers 28195-28198; no faction in the name |
| Faction crafting components | the faction's own merchant, same pattern | INFERENCE (medium): crafting monikers are faction-first like the equipment monikers |

Answer to the owner's question (INFERENCE, high): a Jaffa should not serve Goa'uld-facing trade at Harset. Reasoning: the client names a Lo'taur for that role, in the same mission-token layer that names Jaffa quartermasters for Jaffa goods, so the intent is one server per faction. It also fits the setting: Harset's Jaffa are Loyalist (serving the Goa'uld) in the Praxis storyline, and a Lo'taur is the Goa'uld's own household servant, so Goa'uld trade stays in Goa'uld hands even where Jaffa guard the hub. Where a Goa'uld vendor needs a face and no Lo'taur body_set exists, reuse a Loyalist-Jaffa body set with a Lo'taur name; that is a visual compromise, not a role change (open question 2).

### 5.2 Faction versus Human for Omega-style shared goods

INFERENCE (medium): Omega has no Goa'uld vendors (resident factions are Human, Free Jaffa, Asgard), so its Asgard program vendors have no direct Harset analogue. Harset's Goa'uld vendors replace them. Consumables and common crafting stay faction-neutral in both hubs.

### 5.3 Isolation is cosmetic

FACT (engine contract, [work packets](../work-packets.md) line 223 and the [HA-15 audit](HA-15-audit.md) lines 117-131): vendors have no faction gate. Any player can open any vendor.

- PROVISIONAL consequence: Jaffa, Human and Goa'uld stock is faction-flavoured, not faction-locked. A Human player can buy from a Lo'taur. Stock must therefore be tier-comparable across factions (O14-12), so cross-faction access is harmless.
- MAINTAINER DECISION (not mine): a server-side faction gate on vendors. Until decided, do not ship stock that is only safe if isolated.

## 6. Proposed Harset distribution

PROVISIONAL, `CIMMERIA_AUTHORING`. Every ID is `ID needed`; candidate pools are named only where [id-allocation.md](../id-allocation.md) holds one (templates 261-269 are unassigned capacity, spawns 360-379 are all spoken for). Density follows DECISION O14-12: conservative.

### 6.1 District map

| World | District | Role | Reasoning |
|---|---|---|---|
| 57 Harset | hub (exterior) | one weapons+armor vendor per faction, consumables, faction trainers, auctioneer, mail | INFERENCE (medium): mirrors Omega's hub commerce cluster (section 4 point 1). Placement needs HA-16 evidence |
| 68 Harset_CmdCenter | command interior | registrar, command banker, mission-reward vendors | INFERENCE (medium): Omega's registrar pairs with its command interior (section 4 point 2). World 68 already has 11 spawns, 0 service |
| 69 Harset_Market | market | crafting-component vendors, common crafting, faction crafting | FACT the audit puts the vendors of missions 1353 and 1247 in the Market ([HA-15 audit](HA-15-audit.md) line 43); INFERENCE (medium) that crafting belongs with them |
| 70 Harset_StorageRm | storage | personal-vault banker | INFERENCE (low): the room name suggests storage; no source places a banker there |

### 6.2 Roster by role and faction

Template and list IDs are `ID needed: entity template` / `ID needed: item list` unless stated. One template per row; one spawn per template at first.

| # | Role | Faction side | World | Count | ID |
|---|---|---|---|---:|---|
| 1 | Weapons vendor, tiers 1-2 | Human (OP-Core officer) | 57 | 1 | ID needed: entity template, item list, spawn |
| 2 | Armor vendor, tiers 1-2 | Human | 57 | 1 | same |
| 3 | Weapons vendor, tiers 1-2 | Jaffa (quartermaster) | 57 | 1 | same |
| 4 | Armor vendor, tiers 1-2 | Jaffa | 57 | 1 | same |
| 5 | Weapons vendor, tiers 1-2 | Goa'uld (Lo'taur) | 57 | 1 | same |
| 6 | Armor vendor, tiers 1-2 | Goa'uld (Lo'taur) | 57 | 1 | same |
| 7 | Consumables vendor | neutral | 57 | 1 | same |
| 8 | Crafting components, faction x 4 disciplines | Human / Jaffa / Goa'uld | 69 | 12 at full; 3 at first | same |
| 9 | Common crafting | neutral | 69 | 4 at full; 1 at first | same |
| 10 | Mission-reward vendors | Human / Jaffa / Goa'uld | 68 | 3 | same |
| 11 | Registrar | neutral | 68 | 1 | reuse template 330 or 331 if it fits, else ID needed: entity template |
| 12 | Banker, personal vault | neutral | 70 | 1 | reuse template 370; spawn ID needed |
| 13 | Basic trainers | Soldier, Scientist, Commando, Archaeologist, Jaffa, Goa'uld | 57 | 6 | ID needed: entity template, trainer ability list (generator-owned) |
| 14 | Advanced trainers | per archetype and faction | 57 | up to 11 | HA-15g decides; ID needed |

Density reasoning (INFERENCE, medium): the shipped tier layout names 43 vendor monikers (3.2). That is too many bodies for the traffic one hub sees, so row 1-6 fold four tiers into two (tiers 1-2, matching O14-12's "no high-tier gear"), and row 8-9 start with a representative subset (one per faction, plus one common) and grow only if crafting content needs the rest. The full 12+4 crafting set stays the long-run target because the monikers and the crafting campaign both enumerate it.

Mapping Omega to Harset (INFERENCE, high):

| Omega role | Harset counterpart |
|---|---|
| Human vendors | OP-Core Human vendors |
| Jaffa vendors (Free Jaffa) | Jaffa vendors (Loyalist) |
| Asgard program vendors | Goa'uld vendors (Lo'taur) |
| Asgard Basic/Advanced trainers | Goa'uld Basic/Advanced trainers (7991, 8000) |
| TeamCommandRegistrar | PvPRegistrar (41038) plus team/command registrar |

## 7. Price and stock envelope

PROVISIONAL. This proposes; HA-15b and the maintainer set. O14-12 forbids a new currency and high-tier gear.

- FACT (anchors are thin): the only seeded prices are debug values. `item_list_items.sql` rows 7-17 put test items at 50 to 1,000 naquadah, the debug-hub crafting list 310 sells every row at 1 naquadah, and `item_list_prices.sql` holds a single row (item 3, design 55, quantity 1). There is no tier-priced list in the seed to copy.
- INFERENCE (low): absolute naquadah values cannot be derived from the seed. Set them from the naquadah that Harset missions pay (read in HA-15b), not from these debug rows.
- PROPOSED ratios, ladder only: tier 1 at 1.0x, tier 2 at 2.5x. A tier-3 or higher row is out of scope under O14-12.
- Stock rules (all FACT hazards): one `quantity` per row, because vendors ignore `max_stack_size`; purchases land in bag 1; no mission-only items on a general vendor; mission-token vendors sell mission-reward items only.
- Faction parity: the Human, Jaffa and Goa'uld tier lists must carry the same count of rows and the same price at each tier. INFERENCE (medium): with no faction gate, price parity is the only thing that stops one faction's vendor being strictly cheaper.

## 8. Method limits and open questions

1. Map extraction not run. `extract_actors` ([tools catalog](../../../../tools/README.md)) over `Maps\Omega_Site\Omega_Site-*.umap` (160 files) could give actor placement. I judged it low value: the seed census is 8 rows, the Dev Master gives no service coordinates, and the monikers are the layout. Revisit if HA-16 needs coordinates; the run is lane-only, output to scratchpad, never into the repo.
2. Is there a dedicated Lo'taur body_set? FACT: none is named in the seed, but template 219 "Storage Lo'taur" (moniker 29463, faction 1) already exists in `entity_templates.sql` as a human-female body set with a pistol, so a Lo'taur look has a seed precedent. Whether it suits a vendor, or a Loyalist-Jaffa body set with a Lo'taur name (section 5.1) is better, is a visual call to review in game.
3. Which faction vendor texts exist? The Human and Jaffa tier vendors in 3.2 have empty text rows; HA-15b must supply names (CIMMERIA_AUTHORING) rather than assume the empty row is the intended text.
4. MAINTAINER DECISION: vendor faction gate (section 5.3).
5. Level bands per tier are not recorded in this packet; the tier-1/2 cap in section 6 is a placeholder until HA-15b reads the Harset mission reward levels.

## 9. Engine hazards to carry into HA-15b, HA-15c and HA-15g

All FACT from [work packets](../work-packets.md) line 223, the [HA-15 audit](HA-15-audit.md) and `.claude/reports/harset-continuation/research-items-advisor.md`.

1. No vendor faction gate (section 5.3).
2. Vendors ignore `max_stack_size`; one unit per row.
3. Purchases land in bag 1.
4. A quest speaker must carry no vendor or trainer fields: the trainer list answers the click before the dialog does. A talk-then-trade NPC needs two templates or a server change (MAINTAINER DECISION).
5. A service NPC must not be `faction = 10`: faction 10 is the only faction a player can damage and the one that auto-aggros on sight, so a vendor at 10 is a killable hostile.
6. Trainers gate on archetype and level only; trainer data comes from `tools/ability_trees/generate_seed.py` and its workbook, never a hand edit.
7. `item_lists` ids are FK-checked; the list sequence floor is 329, so new lists start above it.
8. `trainer_ability_list_id` references are FK-checked (existing lists 1-6 and 350-353).

## Tests the implementation owes

Each test names the revert that must make it fail. All are seed lints or live-db tests; none needs the client.

| Test (proposed name) | Asserts | Revert that must fail it |
|---|---|---|
| `harset_service_templates_not_hostile` | no vendor, trainer, banker or registrar template has `faction = 10` | set one new service template to faction 10 |
| `harset_quest_speakers_carry_no_service_fields` | no template with a `speaker_id` or quest dialog has a buy list or trainer list | add a buy list to a quest-speaker template |
| `harset_vendor_lists_resolve` | every vendor template's list ids exist in `item_lists` and every row's `design_id` exists in `items` | point a list at a deleted item list |
| `harset_vendor_rows_single_quantity` | every row in a Harset vendor list has `quantity = 1` | set one row to quantity 20 |
| `harset_vendor_faction_parity` | the Human, Jaffa and Goa'uld lists at each tier have equal row counts and equal total naquadah | delete one Jaffa row |
| `harset_vendor_tier_cap` | no Harset vendor list carries an item above the O14-12 tier cap | add a tier-3 design to a list |
| `harset_service_spawns_on_mesh` | every service spawn's coordinates sit on the world's navmesh and in a walkable component | move one spawn 40 units off the mesh |
| `harset_trainer_seed_has_no_drift` | `generate_seed.py --check` reports no diff for the trainer seed | hand-edit one `trainer_abilities` row |
| `harset_vendor_buy_lands_in_bag1_and_charges` (live db) | buying one row removes the price and places the item in bag 1 | change the buy path to fill bag 2 |

## Telemetry the implementation owes

Names are proposals, to be matched to the existing vendor and trainer instrumentation.

- Span or log row `vendor.open` with `template_id`, `item_list_id`, `world_id`, `player_faction` and `result`, so a wrong-list click is visible without a client.
- Span or log row `vendor.buy` with `template_id`, `design_id`, `price`, `bag`, `result` (success, insufficient funds, bag full, unknown item). A counter `vendor.buy.rejected` by reason.
- Span or log row `trainer.learn` with `template_id`, `trainer_ability_list_id`, `ability_id`, `result` (learned, level gate, archetype gate).
- One startup log row per service template that has a buy list or trainer list but no spawn, and one per spawn whose template has neither, so a placed-but-empty NPC is found in the logs.
- A `spawn.off_mesh` warning for any service spawn that fails the on-mesh check at load, tagged with `world_id`.
