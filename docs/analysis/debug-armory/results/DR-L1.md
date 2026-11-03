# DR-L1 results: Armory walk

> **Type:** reference (verifier result). **Verifier:** DR-L1 verifier, 2026-11-02. **Brief:** [verify/DR-L1.md](../verify/DR-L1.md). **Driver evidence:** [DR-L1-driver-reports.md](DR-L1-driver-reports.md).

Character Celltest (player_id 71), local server on DB sgw_dr_lab, world 1300 (DebugArea, space 65552). Window 16:25-16:39 UTC, 2026-11-02.

**Evidence source note.** The brief's rule 2 (no SigNoz rows for a local server) did not hold this run: the local server exported to SigNoz with resource attribute `cimmeria.lab_run = dr-lab` (service.version 6b50d9131). All vendor, inventory and ability rows below come from those SigNoz logs (scope `vendor`, `inventory`, `cimmeria_cell_methods::...::item_ops`, `abilities`), plus read-only `server_db_query` and `server_entity_query`. The `server_log_tail` buffer held 0 vendor rows (empty buffer), so it gave nothing. No `.bug DRL1` line was needed: rows were located by UTC time. Account names are omitted.

## Runs

| Attempts | run_dir | Calls used |
|---|---|---|
| 4 driver attempts plus a vendor 4 retry (A 45/56, B ~31, retry, C ~37, D 41) | n/a | about 154 plus the retry |

## Grades

| Row | Grade | Tier | Server evidence (query or log event, time) | Screenshot | Issue |
|---|---|---|---|---|---|
| L1-1 | PASS (server clause); human clause NEEDS_HUMAN | N1 | `server_entity_query` template 1610..1616: seven live entities 100423..100429, spawn_id 14110..14116, tags DebugArea_Armory0..6, space 65552, positions (268,7.05,-915) to (266,7.15,-933). Looker not run. | L1-1-vendor-line `...102521.043.png` | none |
| L1-2.0 | PASS | N1 | `store_opened` template 1610 buy_count 19 (want 19), 16:26:32 | n/a | none |
| L1-2.1 | PASS | N1 | `store_opened` 1611 buy_count 19 (want 19), 16:27:09 | n/a | none |
| L1-2.2 | PASS | N1 | `store_opened` 1612 buy_count 19 (want 19), 16:27:45 | n/a | none |
| L1-2.3 | PASS | N1 | `store_opened` 1613 buy_count 16 (want 16), 16:28:17 | n/a | none |
| L1-2.4 | PASS (retry window) | N1 | `store_opened` 1614 buy_count 19 (want 19), 16:32:33 (retry bug 16:32:01). No `store_opened` between 16:28:17 and 16:30:31, matching the two failed hovers at the old stand point. | n/a | brief fix, not a server defect |
| L1-2.5 | PASS | N1 | `store_opened` 1615 buy_count 19 (want 19), 16:30:31 | n/a | none |
| L1-2.6 | PASS | N1 | `store_opened` 1616 buy_count 3 (want 3), 16:31:15 | n/a | none |
| L1-3 | PASS | Tier G setup, N1 buys (driver says N3 for the first buy, a click path; the server does not see the tier) | `transaction` buy, item_type_id 55, vendor 1611, price 300, cash_before 100000, cash_after 99700, 16:34:42. `transaction` buy, item_type_id 33, vendor 1614, price 1, cash_before 99700, cash_after 99699, 16:35:29. Both granted to MAIN (`grant_container_chosen`, source vendor_purchase). First buy `cash_before` 100000 confirms the `/gmgivecash 100000` setup. | n/a | none |
| L1-4 | PASS (server clause); human clause NEEDS_HUMAN | N1 | `moveItem` 10294 (design 55) MAIN to INV_Bandolier, 16:35:33; `moveItem` 10295 (design 33) MAIN to INV_Head, 16:35:35; both "Inventory move persisted". Item_Equip sequence 1872 sent. The bag query after the L1-6 sales cannot show the equipped state (items are sold), so the move rows are the evidence. | L1-4-equipped `...103546.358.png` | none |
| L1-5 | PARTIAL | N1 | `hostile_attack` "targeting hostile NPC for combat" Celltest to 100246 (Jaffa, template 1310), 16:37:34; `weapon_draw_queued` ability 579 Pistol Auto Attack; then `use_ability_no_ammo` reason no_ammo, ammo_current 0, ammo_required 1, 16:37:35, "the press is dropped, no cooldown". No `weapon_unbound` row, so no RANGED-binding defect. No attack, hit or damage row: the empty debug-kit pistol refused the shot, as the brief predicts. | before `...103722.212.png`, after `...103742.224.png` | not an Armory defect |
| L1-6 | PASS (sale clause); cash-after clause UNVERIFIED | N1 | `transaction` sell item_type_id 55, vendor 1611, price 0, 16:38:34; `transaction` sell item_type_id 33, vendor 1614, price 0, 16:39:33. Unequips first: BANDOLIER to MAIN 10294 at 16:37:44, HEAD to MAIN 10295 at 16:38:41. No `refused` row (zero `refused` events in the run). `sgw_inventory` for player 71 after the run: items 10294 and 10295 sit only in container 16 (INV_Buyback), nothing in MAIN, container 3 or container 4. | n/a | none |

Notes on L1-6: the sell rows carry no `cash_before`/`cash_after` fields, so cash unchanged (99699) rests on the price 0 in the transaction row plus the driver's inventory read; the live cash property was not queried. The design id on the rows is `item_type_id` (55 and 33).

## Cross-check of the uncertain last-row names

Seed lists, `resources.item_lists` and `item_list_items` in the local DB (match `db/resources/Items/Seed/item_lists_debug_area_armory.sql`):

- Vendor 1 (list 1311): last row is design 55, "SI 3 9mm Pistol". The driver's "SI 9mm Pistol" was a misread; the server also logs the template as "Armory 1: Microblade Combat Knife to SI 3 9mm Pistol". Consistent.
- Vendor 2 (list 1312): last row is design 4933, "Assault Shotgun". The driver's reading matches.
- Other first/last names the driver reported (vendors 0, 3, 4, 5, 6) all match the lists 1310, 1313, 1314, 1315, 1316 and the template names.
- Vendor 1 list contains design 2699 (Fists) at row 8 of 19, still sold per the seed; not tested.

## Server-side defects

None found. Observations only:

1. Armory 4 (spawn 14114, (266,7.02,-927)) cannot be clicked from the brief's stand point because the Archetype Skills Trainer (100226) covers it; this is a placement/brief issue, not a server fault. Brief fix owed: stand at 261.5 7.02 -926.0.
2. All seven vendors render with the shared name_id 27264 ("Consumables") while the server entity names are "Armory N: ..."; name searches from the client fail (driver deviation 1).

## NEEDS_HUMAN

- L1-1: "a line of vendors east of the plaza vendors, none inside a wall" (looker not run).
- L1-4: "the character wears a helmet and holds a pistol" (looker not run).

## UNVERIFIED

- Cash after the sells (L1-6) from a live cash read; only price 0 on the transaction rows.
- The tier (N1/N2/N3) of each client action: the server does not log it; tiers are from the brief and driver.

## Not run

- Haiku looker questions for the L1-1 and L1-4 screenshots: not run (no Haiku image step available in this verifier session). Answers owed; they would be NEEDS_HUMAN-grade only.
- Chat-only clauses, recorded as facts with no grade ("chat only"): the driver's chat read of "Your SI 3 9mm Pistol is out of ammo." (matches the server `use_ability_no_ammo` row); the `.bug DRL1` lines never came back in chat.
- Combat log: the driver had no `client_combat_log`; `client_chat_log` was used.
- `server_packet_tap_read`: no tap was running for the run, so not used.
- Sell-tab click path: the driver's `ui_sequence` results were inconclusive (posted_only); the server shows both sales committed, so the path worked, but the exact recipe is only the driver's description.

## Resume hint

A re-run starts from: Celltest (player 71), cash 99699, bag Main with Accessory (10293) only, Bandolier and Head empty, SI 3 9mm Pistol (10294) and Aramid Fiber Ballistic Helmet (10295) in INV_Buyback (container 16), last vendor opened Armory 4 (100427) at 16:39:33. The debug-kit pistol has 0 ammo, so L1-5 needs ammo staged to see an attack row.

## Recipe

- Row timing: SigNoz logs filter `cimmeria.lab_run = 'dr-lab' AND attribute.player_id = 71 AND scope_name = 'vendor'` over the window, newest first; `attribute.event` is `store_opened` or `transaction`.
- Equip evidence: scopes `cimmeria_cell_methods::cell::cell_methods::inventory::item_ops` (`moveItem`) and `...::inventory::move_::after_commit`.
- Combat evidence: scope `abilities` (events `weapon_draw_queued`, `use_ability_no_ammo`, `ability_refused`) and `...::hostile_attack`.
- Entities: `server_entity_query` with `template_id` 1610..1616 (one call each).
- Inventory: `SELECT item_id,type_id,container_id,slot_id FROM sgw_inventory WHERE character_id = 71` (columns of `sgw_inventory`: type_id is the design; the table is `public.sgw_inventory`, not `characters`-keyed by name).
- Seed lists: `resources.item_lists` joined to `resources.item_list_items` and `resources.items`.
- Driver: use template_id 1610-1616 and stand point 261.5 7.02 -926.0 for Armory 4; dummy template 1310.
