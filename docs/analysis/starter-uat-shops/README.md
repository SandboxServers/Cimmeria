# Starter-world UAT: shops, vendors and trainers (SS)

> **Type:** campaign ledger (explanation and status). **Audience:** the orchestrator that runs it, the owner and lomiada who answer its decisions.
> **Planned:** 2026-10-27 against `origin/main` `43a424588`.
> **Companions:** [work-packets.md](work-packets.md), [RESUME.md](RESUME.md), the [shared UAT contract](../starter-uat-praxis/work-packets.md#shared-uat-contract) (Praxis campaign), [unified-uat.md](../../guides/unified-uat.md), [debug-hub.md](../../content/debug-hub.md), [debug-area.md](../../content/debug-area.md).

## Sibling campaigns (lomiada's starter-world focus, 2026-10-27)

| Campaign | What it covers |
|---|---|
| [starter-uat-praxis](../starter-uat-praxis/README.md) | Lab UAT of the Praxis chain: Castle_CellBlock (12), Castle (8), Harset (57, 68-70), Tollana (19), and the hand-offs between them |
| [starter-uat-union](../starter-uat-union/README.md) | Lab UAT of the SGU chain: SGC_W1 (58), Dakara_E1 (61/62), Omega (18/80), Beta Site E1 (23) |
| **starter-uat-shops** (this one) | Vendor behaviour no zone ledger covers: sell, buyback, repair, recharge, refusal lines, trainers, the stasis-room and Debug Area vendors, and the worlds that have no vendor |
| [debug-armory](../debug-armory/README.md) | Debug Area world 1300: wear and fire any weapon, wear any armor, cast every archetype ability, with generated lab specs |

The four campaigns share no files except `docs/guides/unified-uat.md`, which only each close-out packet edits. They can run in any order. The lab takes one campaign's wave at a time.

## Purpose

lomiada, 2026-10-27, asked for development effort on the starting worlds up to Tollana and Beta Site: *"the worlds, gameplay, shops and so on"*. Zone rows that already name a vendor run in their zone campaign (D-SS4). This campaign covers the rest:

- the shop mechanics every vendor shares (buy, sell, buyback, repair, recharge, the refusal lines, the distance check);
- the vendors and trainers a new character meets first (the stasis-room debug hub in Castle_CellBlock and the Debug Area plaza);
- a seed guard that no starter-world vendor opens an empty store;
- the owner's call on the starter worlds that have no vendor at all.

It drives the real client through the lab, grades every row against server evidence, files an issue for each failure and turns each pass into a TOML spec.

## Vendors in scope and what is known today

"Tested" means a person or the lab saw it in a real client and the server agreed. "Server-only" means unit, replay or live-DB tests. Anything else is untested.

| World (id) | Vendor or trainer (template, spawn) | Lists (buy / sell / repair / recharge) | Tested in a client | Owned by |
|---|---|---|---|---|
| Castle_CellBlock (12), stasis-room hub | Basic Equipment Quartermaster (300, spawn 400) | 1 / 2 / 2 / 2 (`entity_templates.sql:1011`) | **Failed to open** in the lab on 2026-10-12: the "Vendor" popup item opened nothing ([haiku-lab-uat bank step 10](../haiku-lab-uat/README.md), [BANK-R10](../haiku-lab-uat/briefs/bank/BANK-R10.md)) | this campaign |
| Castle_CellBlock (12) | Common Materials Components supplies vendor (314, spawn 414) | 310 / none / none / none (`entity_templates.sql:1125`); selling to it must give `vendor_buys_nothing` | **Buy confirmed live** 2026-10-12 (Steel Core); `client_vendor_buy` proven at tier N1. Sell was a client read only | this campaign |
| Castle_CellBlock (12) | Archetype Skills Trainer (301, spawn 401); Goa'uld Advanced Skills pet trainer (360, spawn 450) | trainer list 1 (`entity_templates.sql:1019`); pet trainer list per `entity_templates.sql:1194` | never | this campaign |
| Debug Area (1300) plaza | Quartermaster 300 (13001), trainer 301 (13002), pet trainer 360 (13007), Consumables munitions vendor 1302 (13006), supplies 314 (13019) | same templates as the hub; 1302's store is list 1300 (19 rows) | DA-06: DA-U6 store open and DA-U8 munitions rows have specs; **DA-U7 (buy, sell, repair, recharge) never run**, blocked on the lab's row clicks (L9) | this campaign (SS-03b re-specs DA-U7) |
| Castle (8) | Castle_HubQuartermaster (415, spawn 13804, (440.0, 70.22, 992.0)) | 1300 / 2 / 2 / 2 (the Debug Area munitions list) | store-open seen **server-side only** 2026-10-21 | [starter-uat-praxis](../starter-uat-praxis/README.md) SP-DCA6 (Castle end-uat "hub S1" to "hub S5") |
| Omega (18/80) | 30+ vendors and trainers (UO-11/12, UO-27 to UO-30, UO-39, UO-40) | 63800-63828 | **server-only** (live-DB guards) | [starter-uat-union](../starter-uat-union/README.md) (UO-R5 rows, D-SU6) |
| Harset (57, 68-70) | none built | | | [harset-continuation](../harset-continuation/README.md) HA-15b/c (stock draft PU-08, provisional) |
| Tollana (19), SGC_W1 (58), Dakara_E1 (61/62), Beta Site E1 (23) | **none** | | | D-SS1 (owner) |

Server facts the rows rely on (all server-only so far):

- The vendor log events are `vendor` `store_opened` (with `buy_count`, `sell_count`, `buyback_count`, `repair_count`, `recharge_count`), `transaction`, `refused` (with `reason`) and `failed` (`crates/base-methods/src/base/world_entry/methods/vendor/telemetry.rs`).
- Refusal reasons and their chat lines: `insufficient_cash`, `missing_item_cost`, `bags_full`, `not_on_vendor_list`, `vendor_buys_nothing`, `not_sellable`, `buyback_full`, `quantity_exceeds_stack`, `not_in_buyback`, `vendor_does_not_repair`, `not_repairable`, `vendor_does_not_recharge`, `not_rechargeable`, `cash_overflow` (same file, lines 61-74).
- Sell list 2 buys back only designs 21, 55 and 3437. `live_db_vendor_arbitrage` keeps every buy row at or above its sell-back price.
- `/gmrechargeitem` exists. No command lowers an item's durability or charge (`docs/commands.md`).

## Findings

| # | Finding | Packet |
|---|---|---|
| F1 | The first vendor a new Praxis character can use, the stasis-room Quartermaster, failed to open its store in the only lab attempt. Its Debug Area twin (same template 300) has never been opened in a client. | SS-01, SS-D2 |
| F2 | Only *buy* has ever been confirmed in a client. Sell, buyback, repair, recharge and every refusal line are server-only. | SS-03a, SS-03b, SS-D1 |
| F3 | The lab has `client_vendor_buy` but no sell, buyback, repair or recharge tool. Rows must use `client_window_click` on the store window, `client_drag_drop` (lands at N3), or a stock Lua call (N3, graded NATIVE_SHORTFALL). | SS-01 |
| F4 | No in-game way exists to wear an item down or drain its charge, so repair and recharge may not be testable without a new GM command. | SS-01 decides; D-SS3 |
| F5 | A store opens empty when its template points at a missing or empty list, which is silent. Only Omega has a live-DB guard for its lists. | SS-02 |
| F6 | Four of the eight starter worlds sell nothing, and Harset's vendors are only a provisional draft. | SS-04, D-SS1, wave 3 |
| F7 | The lab-server MCP refused connections when this plan was written (2026-10-27). Verifiers need either it or ClickHouse over ssh. | contract § C3 |

## Decisions

| ID | Decision | Status | Reason |
|---|---|---|---|
| D-SS1 | Should the starter worlds that sell nothing (Tollana, SGC_W1, Dakara_E1, Beta Site E1) get story vendors and trainers? (a) no, players use the Castle hub and Omega; (b) one general-goods vendor and one archetype trainer per world, tier-1/2 stock priced by Omega's D-UO29 rule, no new currency or high-tier gear; (c) trainers only. | **BlockedDecision** (owner) | New content in four worlds is a product call. The four-zone standing approval O14-12 (coordinators pick vendor stock) covered the closed four-zone sessions, not new vendors. Architect recommendation: (b), because lomiada listed shops as part of the starter worlds. Only wave 3 waits on it. |
| D-SS2 | The Castle hub Quartermaster (415) sells Debug Area list 1300 at 1 naquadah per ammo stack. | Decided: castle-continuation OD-5 ("accepted for now; flag for an economy pass before a public release") | Already decided; this campaign does not reopen it. SS-04 lists it in the coverage map. |
| D-SS3 | If SS-01 finds no in-game way to wear or drain an item, the repair and recharge rows are graded BLOCKED and one issue proposes a GM command for it. This campaign does not build the command. | Decided (architect) | Same rule as D-SP4: issues, not fixes. |
| D-SS4 | Vendor rows that a zone ledger already lists run in that zone's campaign: Castle hub S1-S5 in SP-DCA6, Omega UO-R5 in starter-uat-union, Harset HA-15b/c in harset-continuation. | Decided (architect) | No row runs twice. |
| D-SS5 | Every lab wave asks the user first (contract § C1). | Decided (architect) | `feedback_ask_before_lab`; O14-08 does not cover this campaign. |
| D-SP2 | Which server the lab plays against (colo after a release, or a local server at a recorded SHA). Shared with starter-uat-praxis. | **BlockedDecision** (owner) | Waves 2 and 3b wait on it. |

D-SP1 (owner-reserved zone rows) does not apply here: none of this campaign's rows are reserved for the owner.

## Packet status

Statuses: Ready / BlockedDependency / BlockedDecision / Writing / Review / Integrated / UATPending / Done.

| Wave | Packet | What | Agent | Lab | Status |
|---|---|---|---|---|---|
| 1 | SS-01 | Store-open failure and the sell, buyback, repair, recharge client paths: `evidence/store-paths.md` | items-systems-advisor | no | Ready |
| 1 | SS-02 | Live-DB guard: no starter-world vendor or trainer has an empty or dangling list | packet-coder | no | Ready |
| 1 | SS-04 | Vendor coverage map for the 14 starter worlds, with the D-SS1 evidence | documentation-writer | no | Ready |
| 1 | SS-03a | `vendors.toml` rows V1-V7: open, buy, sell, buyback, three refusals | packet-coder | no | BlockedDependency (SS-01) |
| 1 | SS-03b | `vendors.toml` rows V8-V12 (repair, recharge, trainer, distance) and the DA-U7 re-spec | packet-coder | no | BlockedDependency (SS-01, SS-03a) |
| 1 | SS-05 | Driver and verify briefs for SS-D2 and SS-D3 | documentation-writer | no | BlockedDependency (SS-01) |
| 2 | SS-D1 | Run `vendors.toml` at the Debug Area plaza | lab-driver + verifier | **ASK-FIRST** | BlockedDecision (D-SP2) |
| 2 | SS-D2 | Stasis-room hub: Quartermaster store open (F1 repro), supplies vendor buy and sell, both trainers | lab-driver + verifier | **ASK-FIRST** | BlockedDecision (D-SP2) |
| 2 | SS-D3 | Bank step 10: sell, then try to drag the item out of buyback without paying | lab-driver + verifier | **ASK-FIRST** | BlockedDecision (D-SP2) |
| 3a | SS-20 | Tollana vendor and trainer seed | database-persistence | no | BlockedDecision (D-SS1) |
| 3a | SS-21 | SGC_W1 vendor and trainer seed | database-persistence | no | BlockedDecision (D-SS1) |
| 3a | SS-22 | Dakara_E1 vendor and trainer seed | database-persistence | no | BlockedDecision (D-SS1) |
| 3a | SS-23 | Beta Site E1 vendor and trainer seed | database-persistence | no | BlockedDecision (D-SS1) |
| 3b | SS-D4 | Lab check of what wave 3a seeded: per world, vendor rows `V-<world>-1/2` and trainer row `V-<world>-T1` under D-SS1 (b), the trainer row only under (c) | lab-driver + verifier | **ASK-FIRST** | BlockedDecision (D-SS1, D-SP2) |
| 4 | SS-98 | Results summary, issue roll-up, sign-off sheet | documentation-writer | no | BlockedDependency (wave 2; wave 3b if run) |
| 4 | SS-99 | Close-out: unified-uat hub and DA-U7 rows, debug-hub.md, status docs once | documentation-writer | no | BlockedDependency (SS-98) |

## Shipping

- **Wave 1** ships as **one PR** from the integration worktree `ss-integration` (branch `starter-uat-shops/integration`). It carries docs, one test file and one spec file. Merge order into it: SS-01, SS-02, SS-04 (any order among them), then SS-03a, SS-03b, SS-05. Before the PR: the wave review loop, then `pwsh tools/build-lane/live-db-test.ps1 live_db_starter_vendor` and the `cimmeria-lab` spec check from `ss-integration`.
- **Wave 2** ends with one docs PR from `ss-integration`: the `results/` files and the README status rows.
- **Wave 3a** (only if D-SS1 is (b) or (c)) ships as one seed PR from `ss-integration`: SS-20, SS-21, SS-22, SS-23 in that order, each with its live-DB guard green. Wave 3b's results ride on the wave 4 PR.
- **Wave 4** ships as one PR: SS-98 then SS-99.

## Not in this campaign

- Castle hub S1-S5, Omega vendor rows and Harset vendors (D-SS4).
- Building a durability or drain GM command (D-SS3), any vendor economy rebalance (OD-5), black market, auctions and player trade.
- The Debug Area armory vendors: [debug-armory](../debug-armory/README.md).
