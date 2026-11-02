# DR-L1 verify brief: grade the Armory walk

> **Type:** how-to (verifier brief). **Audience:** the `general-purpose` verifier (Sonnet) that grades DR-L1. **Updated:** 2026-11-02
> **Companions:** [briefs/DR-L1.md](../briefs/DR-L1.md), [work packet DR-08](../work-packets.md), [Armory ledger](../README.md), [shared UAT contract §C5, §C6](../../starter-uat-praxis/work-packets.md#c5-grades), [shops store-paths](../../starter-uat-shops/evidence/store-paths.md).

You grade one lab run. You read evidence; you do not drive the client.

## Rules

1. **Grade only on server evidence**: a vendor log row (`target "vendor"`, events `store_opened`, `transaction`, `refused`), a read-only database query, or a packet-tap row. A clause seen only in a screenshot, the client UI or the driver's words is PARTIAL or NEEDS_HUMAN (contract § C5).
2. **The local server can export to SigNoz.** When it was started with the OTLP env (service attribute `cimmeria.lab_run=dr-lab`), SigNoz rows may exist: read them first and filter on `cimmeria.lab_run`. A clause that needs SigNoz is **UNVERIFIED** (write that word, not FAIL) only when no SigNoz rows exist. The local evidence sources are `server_log_tail`, `server_db_query` (read-only SQL) and the packet tap of the lab-server MCP, plus the server's own log file.
3. **Chat clauses cannot be attested.** A reply seen only in chat (for example "You do not have enough naquadah.") is recorded as a fact in `## Not run` with the reason "chat only"; it never carries a grade.
4. **The driver is not evidence.** Read its report for times (the BUG lines) and screenshot paths, then check every claim against the server.
5. **A fixture that cannot be set up makes the row BLOCKED, never FAIL.** A FAIL in L1-2 for a vendor that never opened does not turn L1-3 to L1-6 into FAIL: if the vendor needed by a later row did not open, those rows are BLOCKED.

## Setup

1. Read the driver report. Note the character name, the UTC time of each `.bug DRL1 DR-L1 ...` line (the row windows) and the screenshot paths. The window of a row runs from its `L1-N start` bug to the next row's bug.
2. Find the character's `player_id` and `account_id`: `server_db_query` `SELECT ... FROM characters WHERE name = '<name>'` (read-only; look up the exact column names with a first `\d`-style query on `information_schema.columns` if they differ). Never write the account name in the result file.
3. Pull the vendor rows for the run window: `server_log_tail` filtered to `vendor` and the player id, or the server log file for the same window. Each row carries `event`, `action`, `vendor_template_id`, `design_id`, `item_id`, `price`, `cash_before`, `cash_after`, `buy_count`, `sell_count`.
4. Seed facts for cross-checks, read from `db/resources/` in the repo: buy-list sizes (debug-area.md Armory table) and prices (`Items/Seed/item_lists_debug_area_armory.sql`: design 55 at line 48 costs 300; design 33 at line 84 costs 1; sell list 1349 pays 0).
5. The Haiku looker step: for each saved screenshot path (L1-1, L1-4), ask a Haiku `general-purpose` agent the yes/no question below with the image; the answer is NEEDS_HUMAN-grade evidence, never PASS on its own.

## Grading sections

### ROW L1-1 - the vendor line

- Server clause: seven Armory vendors exist in the world. Query `server_entity_query` (lab-server MCP) for entities named "Armory 0" to "Armory 6" in world 1300, or `server_db_query` `SELECT spawn_id, template_id FROM spawnlist WHERE spawn_id BETWEEN 14110 AND 14116` (confirms the seed). PASS needs seven live entities from `server_entity_query`; seven spawn rows only is PARTIAL ("seed present, entities not read").
- Human clause (looker): "A line of vendors east of the plaza vendors, none inside a wall." Looker answer yes -> record `looker yes`; the clause stays NEEDS_HUMAN until a person confirms. The row is PASS when the server clause is PASS and the looker said yes, with NEEDS_HUMAN written beside the human clause.
- The driver's `client_entity_find` match count is a fact for the table, not a grade.

### ROW L1-2 - open each vendor in turn

For N = 0 to 6, one `store_opened` row with `vendor_template_id` = 1610 + N inside the L1-2 window of the driver's report, with `buy_count` equal to the list size: 19, 19, 19, 16, 19, 19, 3 (list 1310 to 1316, `item_lists_debug_area_armory.sql`).

- PASS for the row: all seven exist with the right `buy_count`.
- PARTIAL: at least one exists and the rest show no row (note which; a driver FAIL on a click is the cause to cite, not a verdict).
- A `store_opened` with a wrong `buy_count` is a FAIL against the seed: write the seen and expected value. The window's row cap or paging is not a FAIL here (it is the DR-02 finding).
- `sell_count` counts the player's own items the sell list prices (the shops SS-D1 finding, `store.rs:125,141`), so a fresh bag logs 0: it is not graded.
- One failed vendor does not fail the others; grade each in its own line of the table.

### ROW L1-3 - buy a weapon and an armor piece

- Two `transaction` rows with `action` `buy` in the L1-3 window: one `design_id` 55 with `price` 300 at `vendor_template_id` 1611, one `design_id` 33 with `price` 1 at `vendor_template_id` 1614. `cash_after` = `cash_before` minus `price` in each.
- Cross-check the bag: `server_db_query` for the inventory rows of the character with those designs (look up the inventory table and column names first; do not guess). Both items must be there after the buys.
- PASS: both transactions and both bag rows. PARTIAL: one of the two. FAIL: both buys were attempted (the driver reports a `client_vendor_buy` call) and a `refused` row names the reason; cite it. If the driver reports `no_effect` and the server has no `transaction`, grade FAIL with the note "no_effect, tool known to treat a 100-stack as no_effect; this row is quantity 1, so not the stack case".
- The G-tier setup (`/gmgivecash 100000`) is graded by the `cash_before` of the first buy; record `Tier G` for the setup and `N1` for the buys.

### ROW L1-4 - equip both

- The item moves: packet-tap rows or inventory rows showing design 55 in container 3 (Bandolier) and design 33 in container 4 (INV_Head). Use `server_packet_tap_read` if a tap ran, else the bag query from L1-3 repeated (container ids from [inventory-system.md:104](../../../gameplay/inventory-system.md)).
- PASS: both designs sit in their equipment containers after the L1-4 window. PARTIAL: one. FAIL: neither, with the driver's `client_item_action` results quoted.
- The screenshot (looker: "the character wears a helmet and holds a pistol") is the human clause; NEEDS_HUMAN, never PASS on its own.
- Record the tier: `@item_action` equip is N1/N2 by the tool; if the driver reports it used Lua, mark NATIVE_SHORTFALL only when the server is correct.

### ROW L1-5 - fire at the L1 dummy

- Server clause: an attack by the character on the dummy in the L1-5 window. Look for the attack or combat event of the character against entity 13100's live id (`server_entity_query` for tag `DebugArea_Dummy_L1`), or a `weapon_unbound` row with `reason = "no_ranged_attack"` (`hostile_attack.rs:138-147`).
- PASS: an attack row (damage, miss or hit) naming the dummy. A `weapon_unbound` row is a FAIL for design 55 (it has a RANGED binding, `catalog.md:67`): cite both. An out-of-ammo refusal is PARTIAL with the sentence quoted (the debug kit's pistol is empty, `character-creation.md:146`); it is not a defect of the Armory.
- No row at all and the driver reports the click opened nothing: BLOCKED with the click result as the cause.
- The combat log text is chat-class evidence: not graded alone.

### ROW L1-6 - sell both back

- Two `transaction` rows with `action` `sell`: `design_id` 55 and `design_id` 33, each `price` 0 (list 1349 pays 0), `cash_after` = `cash_before`, at an Armory vendor (`vendor_template_id` 1610 to 1616).
- Cross-check: neither design is in the character's bag or equipment after the L1-6 window.
- PASS: both sales and both items gone. PARTIAL: one. If the sell path refused with `reason = "not_sellable"` for either design, FAIL, because neither is Fists (2699, the one skipped design). If the driver could not start the sale (the Sell tab or the Sell button did nothing) and the server shows no row: FAIL for the tool path, note "sell recipe from the shops SS-D1 rerun (Sell tab, spinner, Vendor_SellButton)".
- Sell-back at price 0 is the cleanup contract (F7): the cash must not change.

## Result file

Write `docs/analysis/debug-armory/results/DR-L1.md` in CRLF, following contract § C6 with the contract § C8 `## Runs` table first (this packet is a driver brief, not a spec run, so the table has one row: driver attempts, `run_dir` `n/a`, calls used).

```
| Row | Grade | Tier (N1/N2/N3/G) | Server evidence (query or log event, time) | Screenshot | Issue |
```

Six rows to grade (L1-1 to L1-6), with L1-2 as seven sub-rows 2.0 to 2.6. Then `## Not run` (each clause and why: chat-only clauses, SigNoz-only clauses as UNVERIFIED, the human screenshot questions), `## Resume hint` (what state a re-run starts from: the bag, the cash and the vendor last opened), `## Recipe` (the exact tool calls that worked, for the later spec runs).

On any FAIL write an issue body to `docs/analysis/debug-armory/results/issues/DR-L1-<row>.md` with the failing step, the evidence row and the seed lines; do not file it (the orchestrator has `ops` file it, DR-30).

## Do not

- Edit any file outside `docs/analysis/debug-armory/results/`.
- Call any `cimmeria-lab` tool, or `lab-server` write tools (`server_console_exec`, `server_stage_fixture`, `server_content_reload`). Reads only.
- Fix, patch or re-seed anything.
- Grade a clause the evidence cannot show: write UNVERIFIED or NEEDS_HUMAN.
- Write IPs, credentials or account names.

## DRIVER REPORT

(The coordinator pastes the driver's full report here before dispatch.)
