# Haiku lab UAT, round 1 (2026-10-12)

> Type: explanation (round report). Audience: the coordinator who runs the next lab UAT round, the owners of the bank, consumables and ability-tree code, and anyone turning these rows into `uat-specs` step files.
> Run on 2026-10-12 against the colo, 07:54Z to 08:40Z (the 01:57Z to 02:28Z start-up attempts failed, see [Setup](#setup)). All times are UTC.
> Companions: [ledger.md](ledger.md) (every graded row with its evidence; the canonical record), [briefs/](briefs/README.md) (the driver briefs), [unified-uat.md](../../guides/unified-uat.md) (the steps), [automated-uat.md](../../guides/automated-uat.md) (result and native-level vocabulary), [live-research-lab.md](../../guides/live-research-lab.md), lab-harness issue [#1660](https://github.com/SandboxServers/Cimmeria/issues/1660).

## Purpose

This round tested whether cheap agents can drive the real game client through the [live research lab](../../guides/live-research-lab.md) well enough to grade unified-UAT rows. Haiku `lab-driver` agents drove one chunk of steps each from a written brief. When a chunk stalled, the coordinator escalated it to a Sonnet agent, which diagnosed the problem, found a recipe and re-briefed the driver. Opus was never needed for driving or diagnosis. The coordinator checked each driver claim before grading it: against server rows in ClickHouse and the database wherever the step has a server-side trace, and against client reads (chat log, inventory, attribute values) where it does not. Four rows rest on client reads only; the Results tables mark them "client-read only".

The pipeline this round feeds:

1. Grade the unified-UAT rows by hand, with a driver, and record how each one was driven.
2. Turn the rows with a proven recipe into `uat-specs` step files ([automated-uat.md](../../guides/automated-uat.md)).
3. Run those files through `lab_uat_run` several times.
4. Mark a feature validated once its rows pass repeatably.

Round 1 covers step 1. The [Ready for step files](#ready-for-step-files) list is the input to step 2.

## Setup

- **Server:** the colo. Its database resets on every deploy (K4), so every character here was created fresh.
- **Three lab clients in parallel**, one lease and one instance each ([lab parallel clients](../lab-parallel-clients/README.md)):

  | Instance | Account | Character | Track | Where |
  |---|---|---|---|---|
  | `default` | `lab` | Bankone (SGU Soldier, player 71) | Bank and vault, personal (steps 1-14) | SGC_W1, then the Castle_CellBlock stasis-room hub |
  | `p2` | `lab2` | Labtwo (player 72) | Consumables (I1-I10b) | SGC_W1 |
  | `p3` | `lab3` | Labthree (SGU Soldier, player 73) | Ability trees AT-06 (1-7) | Debug Area plaza (world 1300), trainer 100210 |

- **Stagger the starts.** Start one client, let it reach the world, then start the next.
- **The build lane must be idle.** Between 01:57Z and 02:28Z the `default` client hung before its window appeared, three times out of three. Its main thread sat in page-in waits while four other sessions' cargo jobs saturated C:. Once those builds had drained, the client started and logged in within about 2 s (07:54Z). Treat this as correlated rather than proven: no controlled run was made. Start lab clients only when `lane_stats.py` shows no running jobs (#1660).
- **Kill the screensaver** before the first start. After 5.5 h idle, `Ribbons.scr` was running and the lab's own D3D preflight refused to start the client.
- **Verification:** the SigNoz MCP returned 401 for this session, and `server_log_tail` (500 lines) fills with forwarded client telemetry within seconds. Server-side checks therefore went to ClickHouse over ssh, using [briefs/verify-template.md](briefs/verify-template.md), plus read-only database queries.
- **New SGU characters spawn in SGC_W1** (world 58), not in the stasis room. A brief that assumes the hub must teleport first.

## Results

`Tier` is the [native level](../../guides/automated-uat.md#native-levels) the step itself ran at; setup tiers are in the notes. Results use the [automated-uat vocabulary](../../guides/automated-uat.md#results). NATIVE_SHORTFALL means the server behaved as the step expects, but the step's action went through a stock Lua call (N3) because the lab's real input failed; it is not a pass until the lab gap that forced it is fixed. "Client-read only" means the evidence is what the client showed, with no server row pulled. "Typed" means a chat command typed by real input, where the step itself *is* the command.

**Totals:** 32 rows attempted: 11 PASS, 12 NATIVE_SHORTFALL (server behaviour verified through a stock Lua call, N3), 5 PARTIAL, 2 FAIL, 2 BLOCKED. Not attempted: bank 15-25, I11 and ability-tree steps 8-10.

### Bank and vault

| Step | Result | Tier | Evidence |
|---|---|---|---|
| 1 `.bank` | PASS | typed | `vault_session_opened gm_override=true scope=personal` 07:55:57; VaultWin visible, 40 slots |
| 2 Storage Officer | PASS | N1 | first right-click opened the vault: `vault_session_opened banker_id=100489` at 4.54 m (08:08:02); `expand_offer_suppressed dialog_quarantined` (K3) |
| 3 deposit | NATIVE_SHORTFALL | N3 | `move_accepted kind=deposit` 08:04:59; reopen shows the item. The drag needed the N3 drop fallback |
| 4 withdraw | NATIVE_SHORTFALL | N3 | `move_accepted kind=withdraw` 08:05:11; database shows the stack back in the bag |
| 5 split / merge | NATIVE_SHORTFALL | N3 | `kind=split` 08:09:39 and 08:12:45, `kind=merge` 08:12:58, totals unchanged. Lua `moveItem` because split drags never start (#1660) |
| 6 relog | PASS | N1 relog, G reads | `.bankdump` identical before and after; `vault_session_closed reason=logout` 08:18:07 |
| 7 walk away | PARTIAL | N1 walk, N3 move | Server guard works (client-read only: chat line and inventory diff, no `move_rejected` row pulled): at 6.64 m, a Lua `moveItem` got "You are too far from the Banker…" (08:16:20). The step as written cannot be done: the client hides VaultWin itself ([D1](#doc-mismatches)) |
| 8 mission item | BLOCKED | — | `/gmgiveitem 1893` put the mission item in Main, not the Mission bag ([P5](#product-findings)) |
| 9 mail / sell | PARTIAL | N1 (sell half) | Sell half PASS (client-read only: a window read): the materials vendor's Sell list is empty with six Steel Cores vaulted. Mail half invalid as written ([D2](#doc-mismatches)) |
| 10 buyback | BLOCKED | — | The Quartermaster's "Vendor" popup item opened nothing; carried to round 2 |
| 11 `.bankexpand` | PASS (GM half) | typed | five `expand` rows 50→100, cash −100 each; `expand_rejected at_ceiling` 08:19:43; `no_vault_session` after a relog. The non-GM half needs a second account |
| 12 too poor | PASS | typed | `expand_rejected reason=insufficient_cash` 07:56:06; `bank_slots` still 40 |
| 13 GM away from Banker | PARTIAL | typed | GM half vacuous: no Banker in SGC_W1, so the session opened without `banker_id`, but moving items was not exercised. The non-GM half needs a second account |
| 14 backpack merge | NATIVE_SHORTFALL | N3 | two `Inventory move persisted` rows 08:13:02 and 08:13:12; one stack of 4 in the database |

Steps 15-25 (organization vaults) were not attempted: they need two characters in one Team.

### Consumables

| Step | Result | Tier | Evidence |
|---|---|---|---|
| I1 grant + set health | FAIL | typed | The grant works. `/gmsethealth` answers "target not found": `GM cmd: target id out of u32 range target_raw=-1`, four times ([P1](#product-findings)) |
| I2 heal | NATIVE_SHORTFALL | N3 | `item used: one unit consumed` health 210→710, stack 3→2 (08:09). HP staged by an X-tier teleport next to hostile Jaffa |
| I3 full-health refusal | NATIVE_SHORTFALL | N3 | "You are already at full health." and the stack stays (refusal row 08:07:47). Also prints a raw `CONDITION_FEEDBACK_` line ([P7](#product-findings)) |
| I4 double-click guard | NATIVE_SHORTFALL | N3 | two `consumable_consume_requested`, one `consumable_consumed`, one `consume_refused reason=not_removed` (08:25) |
| I5 focus heal | PARTIAL | N3 | Low-focus half PASS (client-read only: FP and bag reads): FP 108→499, item gone. The full-focus refusal half was not run, because focus never read full |
| I6 Mark III | NATIVE_SHORTFALL | N3 | `stat_buff_applied` Coordination +5, effect 3950, 3600 s (08:10:54) |
| I7 Mark V | NATIVE_SHORTFALL | N3 | `stat_buff_removed reason=replaced` 3950, then Coordination +7 and Engagement +3 (08:11:10). The client lists the old buff at 0 s for up to about 12 s ([P7](#product-findings)) |
| I8 Mark III Fortitude | NATIVE_SHORTFALL | N3 | Fortitude +5; the Coordination and Engagement buffs remain |
| I9 death and relog | PASS | X death, N1 relog | Buffs survive death and respawn, and are back at base after a relog. Client-read only (attribute reads; no `stat_buff_removed` rows pulled); buff-icon survival was not observed |
| I10 mission item | NATIVE_SHORTFALL | N3 | no heal, item stays; `consumable_skipped reason=placeholder_ability` ([D3](#doc-mismatches)) |
| I10b no-effect item | NATIVE_SHORTFALL | N3 | "This item has no effect yet."; the item stays (K21) |

I11 (wait out the hour) was not run.

### Ability trees (AT-06, Soldier)

| Step | Result | Tier | Evidence |
|---|---|---|---|
| 1 open trainer | PASS | N1 | three tabs, roots trainable (598, 612, 1642), the rest locked; `interact` 08:12:22. Teleport to the trainer was X-tier setup |
| 2 buy root | PASS | N1 | `train_requested`/`train_persisted`/`granted` 612 (08:12:58); points 1→0 at once. The seed row's cost is 0 ([P6](#product-findings)) |
| 3 locked node | PARTIAL | N3 | nothing spent, but the client drops the press itself: no `train_requested` reached the server, so the server's refusal path was not exercised |
| 4 spend gate | PASS | N1 | 1642, 1454 and 632 bought, then 1452 turns trainable at spend 4 (08:15:21-30). Needed a window reopen first ([P2](#product-findings)) |
| 5 relog | PASS | N1 | known list and points identical; database: trained [612, 1642, 1454, 632, 1452], 5 points |
| 6 walk away | FAIL | N3 | At 22 m the window stays open but the client disables the buy button, so a player cannot press it ([D4](#doc-mismatches)). A stock Lua `buyTrainable` got `train_rejected reason=trainer_out_of_range` (08:19:29), nothing granted, and no message ([P3](#product-findings)) |
| 7 double-click | PASS | N1 | exactly one `train_requested`/`granted` 1452 (08:17:24) |

Steps 8-10 (level 50, respec, charge-up) and the other four archetypes were not attempted.

## Product findings

Each finding is confirmed against server rows unless it is marked *possible*. None has its own issue yet. The coordinator files each one, and K30-K33 in the [unified-UAT known issues](../../guides/unified-uat.md#current-known-issues) point back here until it is filed.

| # | Finding | Evidence | Suggested owner area |
|---|---|---|---|
| P1 | **`/gmsethealth` (and any GM command using `resolve_self_or_target`) cannot target yourself.** The client sends target `-1` when nothing is selected, and still sends `-1` with yourself selected. The server accepts only `0` or your own id. `-1` should mean "self". | `GM cmd: target id out of u32 range target_raw=-1` at 07:58:37, 07:58:50, 08:02:02 and 08:02:18, and again at 08:09 with Labtwo selected. Code: `crates/cell-console/src/cell/console/gm/mod.rs:418`. No `.`-command sets health, and `server_console_exec` rejects `gmsethealth` | GM console (`cell-console`) |
| P2 | **An open trainer window goes stale after a level-up.** The server resends trainer info after each purchase, but not on `progression_changed`, so nodes that the new points unlock stay locked until the window is reopened. | `progression_changed level=10 training_points=9` 08:13:41 with no `trainer_resend` afterwards. 1642 stayed locked; close and reopen showed it trainable at once | Ability trees: trainer session |
| P3 | **No feedback on `trainer_out_of_range`.** The guard refuses correctly, but sends no message. The stock client disables the buy button out of range, so only a send that bypasses the button (a modified client, or a stock Lua call) reaches this path; low priority. | `train_rejected reason=trainer_out_of_range` 851 at 08:19:29 after an N3 `buyTrainable`; no chat line or error code in the client | Ability trees: trainer refusals |
| P4 | **The server vault session outlives the client window.** A same-world teleport and a walk-away both hide VaultWin, but the server logs no `vault_session_closed`. A later `.bankexpand`, with no `.bank` first, charged 100 naquadah and grew the vault with no window open. The expected answer was "open your vault first". | no `vault_session_closed` 08:11:30-08:13:20 after the teleport, or on the 08:15 walk-away; then "Your vault now has 50 slots. You paid 100 naquadah." (08:17) | Bank: vault session lifecycle |
| P5 | *Possible:* **`/gmgiveitem` ignores `container_sets`.** Item 1893 (`container_sets [2]`, the Mission bag) landed in Main slot 3. Check whether the grant path should route by `container_sets` before filing. | `resources.items` 1893 `container_sets [2]`, `flags 32768`; the item is in container 1 (08:24) | Items: GM grant routing |
| P6 | **Seed: ability 612's `training_cost` is 0.** The server falls back to `skill_point_cost` 1, so players notice nothing, but the row is wrong. | WARN `train_raw_cost_zero` at the 08:12:58 purchase | Seeds (`db/resources/`), ability trees |
| P7 | **Consumable cosmetics.** A full-health refusal also prints the raw key `CONDITION_FEEDBACK_StatValueGreaterThanOrEqual` as a chat line. A replaced stimpack buff stays listed in the client at 0 s for up to about 12 s. | I3 chat log 08:18; I7 effect read 08:11 against `stat_buff_removed reason=replaced` at 08:11:10.028 | Consumables (condition feedback text, buff removal message) |

## Doc mismatches

| # | Where | Mismatch | Fix |
|---|---|---|---|
| D1 | Bank step 7 ([unified-uat.md](../../guides/unified-uat.md#bank-and-vault), and the canonical [bank checklist](../bank-vault/handoffs/session-resume.md#uat-checklist) item 7) | The step assumes the vault window stays open past 5 m. The stock client hides it by itself (real S-key walk, 4.5 m to 12 m), so a player cannot drag. | Rewrite the step around what a player sees: the window closes on walk-away. The server guard can only be reached by an N3 move. Both the bank checklist item 7 and the guide's step 7 now carry a lab note; rewriting the step itself is open. |
| D2 | Bank step 9, mail half | The GM `.mail … item <type>` mints its attachment and never takes an item from your bags, so it cannot reach the `item_in_vault` guard (`sgw_gate_mail_item source_character_id 0`, mail `item_id null`). That is not a bypass. | Test with the player mail window. The Gate Mail Clerk is quarantined (K2), so this needs another way to open it. |
| D3 | Consumables I10 | The server logs `consumable_skipped reason=placeholder_ability` for a mission item. The round's grading expected a `consumable_refused` row, and the guide's SigNoz table gave no filter for I10. | Name the `consumable_skipped` filter for I10 in the guide (done in this change). |
| D4 | Ability trees step 6 ([unified-uat.md](../../guides/unified-uat.md#ability-trees)) | The step expects "rejected with feedback" when you buy after walking away. The stock client keeps the window open at 22 m but disables the buy button, so a player cannot send the purchase; the server's refusal is only reachable by a modified-client send. | Expect a disabled button; the server-side refusal needs a send that bypasses it (N3 in the lab). Done in this change. |

## Lab harness findings

The lab bugs and gaps found in this round are collected in one issue, [#1660](https://github.com/SandboxServers/Cimmeria/issues/1660), which holds the symptoms, evidence and workarounds. In short:

- **Input:** split drags never start; Main-bag drags fail while another inventory tab is active; `client_item_action` reports success but sends nothing; `__auto_*` CEGUI children cannot be addressed by name; `client_input_mouse` ignores x/y.
- **Camera and movement:** pitch sticks at the clamp after failed face attempts; the face loop ignores your own avatar in the line of sight; `client_move_to` gets stuck in the hub; a right-click at 6.4 m opens nothing.
- **Reads:** `client_player_state` returns empty `stats`; `client_chat_log` can miss a reply; `server_log_tail` is flooded.
- **Supervisor and agents:** start-up hangs under build load; the display keep-awake was not logged; `lab-driver.md` lacks `client_item_action`, `client_chat_log`, `client_die_and_respawn` and `client_type_text`.

## How the Haiku drivers did

- **Escalations:** about 7 Sonnet escalations over about 20 chunks, and no Opus. Each escalation was a lab gap rather than a Haiku reasoning failure: the start-up hang (two levels), the vendor quantity spinner, the p3 camera clamp, split drags, item use, and HP and focus staging.
- **What worked:** short chunks of 2-4 steps with a stated precondition and postcondition; exact tool calls written out in the brief; a "record raw, stop on a real block" rule; a chat macro that checks `Inst1Chat_Input:getText()` before pressing Enter. Drivers reported raw values faithfully. The coordinator's ClickHouse check found a driver's claim wrong only where the lab itself lied (`native_pass` on an item use that never reached the server).
- **What needed escalation:** anything that needed a new recipe rather than a new attempt. A Haiku driver retries the same failing call, or concludes BLOCKED. That is the right outcome, but finding the workaround needed Sonnet.
- **Prefer the `lab-driver` agent over a general-purpose Haiku.** A general-purpose Haiku loads the user's "ask before any lab use" memory and refused a lent lease until told the run was approved. `lab-driver` has no memory, so its briefs do not need an approval line. It does lack four tools (#1660), which is why some consumables chunks ran on general-purpose agents.
- **Briefs go stale fast.** Three of the five consumables briefs were replaced by one re-brief once item use and HP staging had recipes ([briefs/README.md](briefs/README.md)).

## Ready for step files

These rows have a recipe that worked live. Each can become a `uat-specs` step file now, at the tier shown in [Results](#results).

- **Bank:** 1, 2, 3, 4, 5, 6, 11 (GM half), 12, 14. Add a server clause first (the `move_rejected banker_out_of_range` row; a sell-list check the server logs) for 7 (server guard: walk away, then an N3 move) and 9 (sell half), which were graded on client reads only. 13 is not ready: moving items was never exercised.
- **Consumables:** I2, I3, I4, I6, I7, I8, I10, I10b. I5 (low-focus half) and I9 were graded on client reads only: add their server clauses (`consumable_used` focus before and after; `stat_buff_removed` on death and relog) first.
- **Ability trees:** 1, 2, 4 (reopen the window after the level-up until P2 is fixed), 5, 7.

## Open for round 2

- **Bank 8:** needs a mission item in the Mission bag. Get one from a real mission, or wait for P5.
- **Bank 9, mail half:** needs the player mail window (D2).
- **Bank 10:** open the Quartermaster's vendor window. In round 1 the popup's "Vendor" item did nothing. That is possibly a product finding; confirm it with a human click.
- **Bank 11 and 13, non-GM halves; bank 15-25:** need a second account online.
- **Consumables I1:** after P1 is fixed. **I5**, full-focus half: needs a full-focus read first. **I9:** observe the buff icons. **I11:** the one-hour expiry.
- **Ability trees 3:** redesign the step, because this client filters a locked press. **6:** rewrite to expect a disabled button (D4); the server refusal stays an N3 check, with its message after P3 is fixed. **8-10:** not reached. The Commando, Scientist, Archaeologist and Free Jaffa trees: not reached.

## Proven driving recipes

Each recipe worked live in this round. The briefs in [briefs/](briefs/README.md) show them in full.

- **Teleport, then click; never walk to a target.** `.gotolocation Castle_CellBlock -330.0 73.47 -229.2` puts you 4.5 m from the personal Banker, and `-337.0 73.47 -227.5` puts you 4.8 m from the Common Materials Components vendor. Coordinates are server metres (x, y up, z); the client reports `(z, x, y) × 100`. Stand 3.5-4.5 m away: at 6.4 m a right-click hovered the trainer and opened nothing, and at 3.4 m your own avatar blocks the camera line.
- **Camera recovery:** `client_camera {"pitch_counts": -650}` (about 0.11° per count) un-pins a pitch stuck at the clamp. Then face the entity again. Reopening the trainer after a relog needed it again.
- **Clicking at a screen point:** `client_cursor_move` first, then `client_input_mouse`, which ignores its own x/y.
- **Walking:** held keys, for example `{"do":"key","key":"s","hold_ms":2500}` in `client_ui_sequence` (N1). Do not use `client_move_to` in the hub.
- **Stock Lua at N3 when the lab input fails:**
  - `moveItem(srcContainer, srcSlot, dstContainer, dstSlot, qty)` for moves, splits and merges. Slots are 1-based; containers are Main 1, Crafting 15, Vault 17 (`Inventory.lua:704`).
  - `useItem(1, <slot>)` to use a Main-bag item, with the slot as `client_inventory` prints it.
  - `setVendorSelectedQuantity(<row, 0-based>, n)`, then click `Vendor_BuyButton`. Each row's quantity starts at 0.
  - `getUnitStat(Unit.Player, Stat.X).current` for stats (`Stat.Intellect` does not exist).
  - `buyTrainable(<abilityId>)` presses a trainer node.
- **Inventory tabs:** Main-bag slots are only "on screen" while the General tab is active. A lab click on `Inventory_GeneralTabInactive` does not switch the tab, so use `moveItem` instead.
- **Lowering your own HP:**
  - X-tier, proven: `server_console_exec gotoxyz 178 -8.4 9` next to Ba'al's Jaffa in SGC_W1. Focus drains first, then health at about 100/s. Teleport out (`gotoxyz 201.5 1.4 49.7`) at 150-300 HP; staying about 12 s kills you, and you respawn at full HP.
  - Typed alternative: `.dummy caster 1354 [interval ≥ 5]`. The ledger records it working at 08:12, but one driver saw no HP change in 10 s (Disabling Shot is a debuff), so check that HP actually drops.
  - `/gmsethealth` does not work (P1).
- **Chat:**
  - Type into `Inst1Chat_Input`, check `Inst1Chat_Input:getText()` equals the line, then press Enter.
  - Read replies only with `client_chat_log` and a named cursor, and repeat a command once if no reply shows. A Lua read of the chat output returns nil.
  - `.givecash` needs a target; `/gmgivecash 500` works on yourself.
- **Dialogs and new characters:**
  - Close a dialog with `DialogMod.onDialogDoneClicked(DialogWin, DialogWin)`.
  - A brand-new character holds entity introductions for about 16 s, so wait 17 s before looking for NPCs.
- **Verification:** use the ClickHouse template in [briefs/verify-template.md](briefs/verify-template.md). Filter on `player_id` with `toInt64(attributes_number['player_id'])`. Put the `resources.` schema prefix on seed tables. GM reply text is only in the client chat log, not in SigNoz.
