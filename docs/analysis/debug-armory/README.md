# Debug Area armory: wear and test every weapon, armor piece and ability (DR)

> **Type:** campaign ledger (explanation and status). **Audience:** the orchestrator that runs it, the owner and lomiada who answer its decisions.
> **Planned:** 2026-10-27 against `origin/main` `43a424588`.
> **Companions:** [work-packets.md](work-packets.md), [RESUME.md](RESUME.md), the [shared UAT contract](../starter-uat-praxis/work-packets.md#shared-uat-contract) (Praxis campaign), [debug-area.md](../../content/debug-area.md), the [debug-area campaign](../debug-area/README.md) (#1210), [automated-uat.md](../../guides/automated-uat.md).

## Sibling campaigns (lomiada's starter-world focus, 2026-10-27)

| Campaign | What it covers |
|---|---|
| [starter-uat-praxis](../starter-uat-praxis/README.md) | Lab UAT of the Praxis chain: Castle_CellBlock (12), Castle (8), Harset (57, 68-70), Tollana (19), and the hand-offs between them |
| [starter-uat-union](../starter-uat-union/README.md) | Lab UAT of the SGU chain: SGC_W1 (58), Dakara_E1 (61/62), Omega (18/80), Beta Site E1 (23) |
| [starter-uat-shops](../starter-uat-shops/README.md) | Vendor behaviour no zone ledger covers: sell, buyback, repair, recharge, refusal lines, trainers, the stasis-room and Debug Area vendors, and the worlds that have no vendor |
| **debug-armory** (this one) | Debug Area world 1300: wear and fire any weapon, wear any armor, cast every archetype ability, with generated lab specs |

The four campaigns share no files except `docs/guides/unified-uat.md`, which only each close-out packet edits. They can run in any order. The lab takes one campaign's wave at a time.

## Purpose

lomiada, 2026-10-27: *"When possible the dev map should have a function for us to wear and test every weapon and armor as well as abilities."*

This campaign gives the Debug Area (world 1300) an **Armory**: a row of plaza vendors that sell one representative of every weapon family and every armor slot at every tier and quality. The full catalog stays one `/gmgiveitem` away. It then generates lab specs that equip each representative and fire it at the existing training dummies, and that cast every ability of every playable archetype tree with the right weapon in hand. The lab runs those specs. Each failure gets an issue.

It needs no new opcode, no client patch and no new server code: seed rows, one Python generator, a live-DB guard, TOML specs and lab runs (D-DR1).

## What exists today

"Tested" means a person or the lab saw it in a real client and the server agreed. "Server-only" means unit, replay or live-DB tests. Anything else is untested.

| Piece | State | Evidence |
|---|---|---|
| World 1300 itself, the plaza, the GM-only entry (`.gotolocation DebugArea x y z`) | **Tested in the lab**: DA-06 graded 18 PASS, 12 PARTIAL, 2 BLOCKED, 2 NOT RUN, no open FAIL | [debug-area README:270](../debug-area/README.md) |
| Training dummies (templates 1310-1314, spawns 13100-13104, z -872) and `.dummy` | Tested in the lab (DA-06, ability-mechanics AB-U rows) | [debug-area.md:997-1001, 1152](../../content/debug-area.md) |
| Plaza vendors 300, 301, 360, 1302, 314 | Store open has a spec (DA-U6); **DA-U7 (buy, sell, repair, recharge) never run** | [starter-uat-shops](../starter-uat-shops/README.md) owns these |
| `/gmgiveitem <designId> <qty>` (to the caller only, quantity 1-1000), `/gmgiveability <id>`, `/gmgiveallabilities` (own archetype tree only), `/gmresetabilities` | Server-only; `/gmgiveability` is used in the ability-mechanics lab rows | [commands.md:257-267](../../commands.md) |
| A way to remove items (legacy `.removeitem`), set level (`/gmsetlevel`) or grant another archetype's abilities | **Missing** (legacy parity rows P06/P07) | D-DR4, D-DR5 |
| The debug kit (`char_creation.debug_kit`) | Five abilities and one empty pistol (55); no profile sets it | [character-creation.md:146](../../gameplay/character-creation.md) |
| Weapon-requirement audit: every player ability's `item_monikers` against every bandolier weapon | **Generated, server-only**: 596 player abilities, 126 need a weapon, 4 FAIL (no shipped weapon fits) | [weapon-requirement-audit.md:27-35](../class-start-v6/weapon-requirement-audit.md), `tools/ability_mechanics/weapon_requirement_audit.py` |
| Equip | Server checks the item's `container_sets` only; no level, archetype or discipline gate (`item_allows_container`) | [inventory-system.md:132](../../gameplay/inventory-system.md) |
| Spec macro to equip by name | `@item_action { action = "equip", name }` | `first-session.toml:492` |
| A lab run of "equip every weapon / armor piece" or "cast every tree ability" | **Never done** | none |

## Findings

- **F1. Most of the mechanism exists.** GM grants cover every item and the caller's own tree. What is missing is a fast, browsable way to *get* gear in the world without looking up ids, and a run that *proves* each piece works in the client.
- **F2. "Every" item is too many to stock.** The seed has thousands of items; a vendor window has a row limit (DR-02 finds it). The Armory stocks one representative per weapon family, tier and quality, and per armor slot, tier and quality (D-DR2). `/gmgiveitem` reaches the rest.
- **F3. The weapon-requirement audit is the right base.** `weapon_requirement_audit.py`'s `load()` already parses abilities, items, monikers, trees, trainer lists and content grants with `seed_sql.py`. The new generator imports it rather than writing a second parser.
- **F4. Id blocks.** The obvious blocks are taken: templates 1300-1609 and spawns 13000-14109 belong to DA-02 to DA-10 and MG-74 ([debug-area.md](../../content/debug-area.md) block list). The free blocks found on 2026-10-27 are **templates 1610-1659, spawns 14110-14299, buy lists 1310-1349, `item_list_items` ids 131000-134999**. DR-01's audit re-proves them at dispatch.
- **F5. Pricing must keep the arbitrage guard green.** `live_db_vendor_arbitrage.rs` fails if any design can be bought below what some vendor pays for it. Armory buy price = max(1, the highest unit price any sell list pays for that design).
- **F6. Abilities by archetype.** `archetype_ability_tree` has 439 rows in **7 trees** on 2026-10-27: Soldier 72, Asgard 66, Archeologist 65, Commando 64, Goa'uld 62, Scientist 59, Sholva 51. `ARCHETYPE_Jaffa` can be created (2 `char_creation` rows) but has no tree and no trainer rows by design: the Free Jaffa tree is Sholva's ([ability-trees D-AT04](../ability-trees/README.md)). A fresh character of each archetype takes `/gmgiveallabilities`, so one spec per archetype needs no cross-archetype grant. Asgard may be unplayable (F42 / HELD-B16, as Union D-SU3); its rows are BLOCKED if creation fails.
- **F7. No cleanup.** With no `.removeitem`, a tester's bags fill. Selling back to the Armory is the cleanup (sell list = the same designs, D-DR5).
- **F8. Many weapons cannot fire.** The audit counts 269 bandolier weapons with no RANGED binding (`items_event_sets` event 7): right-clicking a hostile with one logs `weapon_unbound` `reason = "no_ranged_attack"` and shows "This weapon has no ranged attack." (`crates/cell-methods/src/cell/cell_methods/player/interaction/hostile_attack.rs:138-147`). The representative rule prefers an item with a RANGED binding, and the gear spec grades an unbound representative on that line, so the run measures the gap instead of failing on it.

## Decisions

| ID | Decision | Status |
|---|---|---|
| D-DR1 | The Armory is **seed-only**: vendors in the Debug Area plaza with representative stock. No new GM command and no bulk-give command; the full catalog stays reachable through `/gmgiveitem`. | Decided (architect; lomiada may overrule) |
| D-DR2 | Representative rule. Weapons: per (family, `tier`, `quality_id`), the lowest `item_id` with a RANGED binding, else the lowest `item_id` (F8); family = the item's `moniker_ids` ∩ the monikers any `abilities.item_monikers` names, and the item can sit in the bandolier (container 3). Armor: the lowest `item_id` per (equipment slot from `container_sets`, `tier`, `quality_id`). Rows per vendor are capped at DR-02's store-window limit; families are split across vendors in id order. | Decided |
| D-DR3 | Ability scope: the 7 archetype trees (F6; Jaffa has none by D-AT04, and a Jaffa character is not run). NPC abilities, item-event and content-grant abilities are out (the weapon-requirement audit already covers them server-side). | Decided |
| D-DR4 | Cross-archetype grant (a GM line that gives another archetype's tree, ties to D-AT07). Not needed for this campaign's specs (F6). A DR-40 stub waits on it. | **BlockedDecision** (owner) |
| D-DR5 | Cleanup. Armory sell list = the same designs at price 0 if the sell path accepts 0, else 1, and a design that another buy list sells at a lower price is left off the sell list. Question for the owner: also pull legacy P07 `.removeitem` into a later campaign? | **BlockedDecision** (owner; the seed half is decided, only the `.removeitem` question is open) |
| D-DR6 | No level gating: the Armory sells every tier to any level. Equip has no level gate today (inventory-system.md:132). | Decided |
| D-DR7 | Where the lab sweeps run: the local server, the colo after deploy, or both. The colo DB is rebuilt from seeds on each deploy, so the Armory reaches it only after a release. | **BlockedDecision** (owner) |
| D-SP2 | Server target for every lab run (shared with the Praxis campaign). | **BlockedDecision** (shared) |

## Packets

Full specs in [work-packets.md](work-packets.md). Status values: Ready, BlockedDependency, BlockedDecision, Writing, Review, Integrated, UATPending, Done.

| Wave | Packet | Agent | What | Depends on | Status |
|---|---|---|---|---|---|
| 1 | DR-01 | packet-coder | `tools/ability_mechanics/debug_armory.py survey` and `audit`: the representative catalog and the id-block audit, with unit tests | none | Ready |
| 1 | DR-02 | game-archaeology-specialist | RE: the store window's row limit and paging, client-side equip checks, the Lua equip path for armor slots | none | Ready |
| 1 | DR-03a | packet-coder | `debug_armory.py seed`: emits the three seed files, with tests (no seed written yet) | DR-01, DR-02 | BlockedDependency |
| 2 | DR-03b | database-persistence | Run the generator, wire the seed into `db/database.sql`, place and nav-check the vendors | DR-03a | BlockedDependency |
| 2 | DR-04 | packet-coder | `live_db_debug_area_armory.rs`: stock, prices and ability-weapon coverage | DR-03b | BlockedDependency |
| 2 | DR-05 | documentation-writer | Armory section in `debug-area.md`, unified-uat rows | DR-03b | BlockedDependency |
| 3 | DR-06 | packet-coder | `debug_armory.py specs abilities`: one `armory-abilities-<archetype>.toml` per archetype | DR-04 | BlockedDependency |
| 3 | DR-07 | packet-coder | `debug_armory.py specs gear`: `armory-gear-weapons.toml`, `armory-gear-armor.toml` | DR-04 | BlockedDependency |
| 3 | DR-08 | documentation-writer | Brief and verify file for DR-L1 (a human-style walk of the Armory) | DR-05 | BlockedDependency |
| 4 | DR-L1 | lab-driver + verifier | Walk the Armory: open each vendor, buy, equip, sell back. **ASK-FIRST** | wave 3 merged | BlockedDependency |
| 4 | DR-L2 to DR-L5 | lab-driver (spec run) + verifier | Ability specs: Soldier, Commando, Scientist, Archeologist. **ASK-FIRST** | DR-L1 | BlockedDependency |
| 5 | DR-L6 to DR-L8 | lab-driver (spec run) + verifier | Ability specs: Asgard, Goa'uld, Sholva. **ASK-FIRST** | wave 4 | BlockedDependency |
| 5 | DR-L9, DR-L10 | lab-driver (spec run) + verifier | Gear specs: weapons, armor. **ASK-FIRST** | wave 4 | BlockedDependency |
| 6 | DR-30 | orchestrator + ops | File the issues the lab waves wrote | waves 4-5 | BlockedDependency |
| 6 | DR-99 | documentation-writer | Close-out: results summary, debug-area.md and unified-uat statuses, status docs once | DR-30 | BlockedDependency |
| - | DR-40 | (stub) | Cross-archetype grant command | D-DR4 | BlockedDecision |

Shipping: waves 1 and 2 ship as **one PR** from `dr-integration` (the Armory itself), merge order DR-01, DR-02, DR-03a, DR-03b, DR-04, DR-05. Wave 3 ships as a second PR from the same integration branch after the first merges, order DR-06, DR-07, DR-08. Waves 4 and 5 ship their results as one docs PR each. Wave 6 is one PR, DR-30 then DR-99.

## Not in this campaign

- Vendor mechanics (sell, buyback, repair, recharge, refusals): [starter-uat-shops](../starter-uat-shops/README.md).
- Ability *mechanics* (damage numbers, effects, QR): the [ability-mechanics](../ability-mechanics/) AB-U rows. These specs only prove each ability casts with the right weapon.
- A cross-archetype grant command (DR-40, D-DR4), a `.removeitem` command (D-DR5), or a `/gmsetlevel`.
- Abilities that come only from items or content grants (D-DR3).

## Status

Planned 2026-10-27; nothing dispatched. The lab-server MCP refused connections when this plan was written.
