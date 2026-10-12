# Lab harness close-out (LH-18)

> Type: reference (campaign close-out record). Audience: whoever runs the next lab UAT round or picks up an owed row below, and reviewers of the lab-harness branch.
> Updated: 2026-10-12. Live run on the colo on 2026-10-12: instance `p3` 10:40Z to 11:35Z, `default` and `p2` 13:15Z to 13:32Z, a recheck of LH-19 and LH-20 on `default` about 14:00Z to 14:10Z. All times are UTC.
> Companions: [README.md](README.md) (ledger: findings, decisions, packets), [probes.md](probes.md) (the probes run before coding), [round-1 report](../haiku-lab-uat/README.md) (the grades this campaign set out to lift), [live-research-lab.md](../../guides/live-research-lab.md) and [automated-uat.md](../../guides/automated-uat.md) (the guides the packets changed), [unified-uat.md § Lab harness](../../guides/unified-uat.md#lab-harness) (the owed rows), issue [#1660](https://github.com/SandboxServers/Cimmeria/issues/1660).

This page records, for each of the sixteen items in #1660 and the two findings the close-out run added (LH-19, LH-20), which packet fixed it, what the live client did on 2026-10-12, the server row that backs it, and whether the fix is proven live. Read the [What was not tested live](#what-was-not-tested-live) list before you rely on a row marked anything but "Verified live".

## How the run worked

Three lab clients ran in parallel on the colo through one lab daemon built from the integration branch: `default` (Bankone, Castle_CellBlock, world 12), `p2` (Labtwo, SGC_W1, world 58) and `p3` (Labthree, Debug Area, world 1300). Haiku drivers followed written briefs and reported raw tool fields; the coordinator checked every server claim against ClickHouse (`signoz_logs.distributed_logs_v2`, `service.name = cimmeria-server`, read-only over ssh). Player ids this deploy: Bankone 71, Labtwo 72; the trainer rows carry entity names, not `player_id`.

Status values:

| Status | Meaning |
|---|---|
| Verified live | The fixed tool did the thing on the live client on 2026-10-12, and the server row agrees where there is one |
| Unit-tested only | Covered by `cimmeria-lab` tests against the fake bridge; the live run did not exercise the fixed path |
| Owed | Needs a live check that has not happened; listed in [unified-uat.md § Lab harness](../../guides/unified-uat.md#lab-harness) |

"Client-only" in the evidence column means the behaviour never reaches the server, so there is no row to find.

## Per-item results

| # | Issue item | Fix | Live result (2026-10-12) | Server evidence (UTC) | Status |
|---|---|---|---|---|---|
| 1 | Split drag never starts | LH-01 | Main-bag split Main 2 to Main 4 (D10.4): `drag_started: true`, `handed_off: true`, `stand_in: InventoryDragItemWin`, `mod_drag_type.inventory: 1`, `drop_target_resolved: false`, `moved: true`, `effect_ok: true`, `native_level: native_call` (N3); stack 8 to 7, new item 10321. The Crafting-to-vault "split" (D9.2) ran on a 1-stack, so it moved the whole item (a swap) | 13:23:16 `Inventory move persisted` MAIN to MAIN, item 10311; D9.2 is 13:22:52 bank `move_accepted kind=swap` | Verified live (lands at N3, see [Native tiers](#native-tiers-against-the-issues-acceptance)) |
| 2 | Main-bag drag "not on screen"; tab click does not switch | LH-02 | Every bag drag reached its slot: Crafting 1 (D7.4), Main 2 (D10.4), Main 4 (D10.6); no "still not on screen". The drivers did not return `native_steps`, so which tab-click rung worked is not recorded. The briefs' own preparatory clicks on `Inventory_GeneralTabInactive` and `Inventory_CraftingTabInactive` (D7.2, D10.2) failed with "is not visible" | 13:22:13 and 13:23:16 / 13:23:23 move rows for the drags | Verified live (slot reached); the tab-click escalation is unit-tested only |
| 3 | `client_item_action` reports native_pass, sends nothing | LH-03 | Heal while hurt (B2): `effect_ok: true`, `native_pass: true`, `native_level: native_cegui` (N1), no escalation, `double_use_risk: false`; Slappack 2 to 1, HP 14 to 520. Second unit (B3): same fields, 1 to 0. Coordination stimpack (B4): N1, item gone, Coordination 5 to 10. Full-health negative check (A4, `expect_change: false`): one injected right-click, `escalation: skipped_negative_check`, no change, chat "You are already at full health." | 13:23:27 `consumable_requested` / `consumed` / `consumable_used`, item 2893, ability 648, health 16 to 516. 13:23:41 the same triple, 530 to 760. 13:24:02 item 6677, `stat_buff_applied` Stimpack_Coordination effect 3950, 3600 s, stat 5 to 10. 13:20:51 `consumable_refused reason=already_at_max` 760/760, nothing consumed | Verified live (N1) |
| 4 | `__auto_*` children unreachable; vendor quantity | LH-04 | Over-max (D5.3): error "client_vendor_buy: row 1 allows at most 99, not 9999", nothing sent. Buy 6 (D5.4): `bought: true`, `effect_ok: true`, `native_level: real_input` (N1), `spinner_clicks: 6`, `spinner_final: 6`, `items_gained: 6`, `double_buy_risk: false`, cash 1394 to 1388. `InventoryWin__auto_closebutton__` resolved by name (D11.1). Vendor window closed with a real click (D5.6, N1) | 13:21:43 vendor `transaction` buy, Bankone, item type 5254, quantity 6, cash 1394 to 1388, template 314, one row; no transaction near 13:21:36 (the refused 9999) | Verified live (N1). The Lua fallback never ran (see [not tested](#what-was-not-tested-live)) |
| 5 | `client_input_mouse` ignores x/y | LH-05 | D11.3: `x: 814, y: 238` put the cursor at [814, 238] (`cursor_via: native_inject`, `native_cegui`) and the press closed `InventoryWin` | Client-only | Verified live |
| 6 | Pitch pinned at the clamp; zero pose | LH-06 | 900 pitch counts (P3.1): `pitch_clamped: true` and the warning "the pitch is pinned at the 78.75 deg limit ...". A face from there (P3.2): `face.pitch_clamped: true`, recovered 78.75 to 9.98 deg, `centred: true` in 3 looks. `camera_before` / `camera_after` were left out (the zero pose is rejected; use `view_after`) | Client-only | Verified live |
| 7 | Own avatar occludes the target | LH-07 | Trainer at 3.42 m straight behind the avatar (P4.3): first aim 100 units above the feet hovered the trainer (`hover_verified: true`), real right-click, `native_tier: N1`, `AbilityWin` opened. The yaw retries were not needed this time | 10:46:14 abilities `trainer_open`, trainer 100210, template 301, Labthree; exactly one row | Verified live |
| 8 | `client_move_to` stuck and drifting | LH-10, LH-19 | Before LH-19 (D12.4): the walk-away guard stopped it, "walking away from the goal: 18.6 m now, 11.7 m at the closest; stopped instead of drifting further", with 0 turn samples, which led to LH-19. After LH-19 (recheck): 12.03 m to the vendor, `arrived: true` at 3.45 m in 3560 ms, `turn_rate` 113.2 deg/s, 0 sign flips, `real_input` N1; a 17.7 m point walk arrived at 0.15 m in 8398 ms (107 deg/s). The walk back hit a wall: "stuck 17.3 m short after 3 unstick attempts", then `input_probe` "did not move for a 300 ms forward key hold" (no path-finding by design, D-LH7) | D12: only a `movement.speed_warning` for Bankone's walk; the recheck was not queried (client-only) | Verified live (arrive, abort on walk-away). The already-in-range case (brief D4) has no recorded result: unit-tested only |
| 9 | Right-click at 6.4 m opens nothing | LH-08 | 6.39 m (P5.3): failed at `observe` with "... the target is 6.4 m away, beyond the server's 5 m interact range: stand 3.5-4.5 m from it", `range: {distance_m: 6.4, interact_range_m: 5.0, in_interact_range: false}`; chat "You are too far away from Debug Hub - Trainer. Move closer to interact." 4.21 m (P5.8): `in_interact_range: true`, N1, `AbilityWin` opened | 10:46:55 INFO `interact: too far away`, Labthree to trainer 100210, dist 6.3957, max 5; no `trainer_open` between 10:46:14 and 10:47:41. 10:47:41 `trainer_open`, no too-far row after it | Verified live |
| 10 | `client_player_state` empty stats | LH-09 | `stats.Coordination` read 5 before the stimpack and 10 after (B4), matching the server. The brief's full stats check (A2: key count, Health against Lua) did not run, because `lab_ensure_in_world` failed twice first ([pre-existing](#pre-existing-lab-issues-seen-not-caused-by-this-branch)) | 13:24:02 `stat_buff_applied` stat 5 to 10 | Verified live (one stat); the 34-key read is unit-tested only |
| 11 | `client_chat_log` misses a reply | LH-11 | After `lab_logout` and `lab_ensure_in_world` (B5), with no chat read in between: the capture reported `lua_epoch_changed: true`, chat `ok`, `bridge_dropped: 0`, no `capture_warning`, and the reply "bankexpand: open your vault first ..." was in the log (seq 254139). A new cursor starts at the oldest kept line, so the first read returned 30 old Combat lines (`more: true`); the reply came on the next read with `since_seq`. After 120 s idle (COORD-1) `client_events_read` reported `dropped: 0` | 13:24:41 bank `expand_rejected reason=no_vault_session`, trigger gm_console, Labtwo (72) | Verified live |
| 11b | Click ring install `failed` | LH-12 | `lua_install.click` read `ok host=BackgroundWin` on p3 (P1.2, P6.1) and p2 (B5). COORD-2 (does the ring record the `Ability_Tab2` click?) was not run; probe C-P7 saw the global click event fire on that host | Client-only | Verified live (install); recording is probe-only |
| 12 | `server_log_tail` flooded | LH-13 | Not run: the fix is in the server (`crates/admin-api` log ring), and the colo runs `main` | — | Owed (after the colo deploy) |
| 13 | Startup hangs under build IO | LH-14, LH-15 | A cold start during a review build took about 38 s (start 971 ms, login 10111, play 6694, ready 20116) with `crash_count_10min: 0`. No page-in stall happened, so the load grace never engaged; whether the launch waited on a busy lane was not recorded | Client-only (`labd.log`) | LH-14: Owed. LH-15: Unit-tested only |
| 14 | Keep-awake never held | LH-16 part 1 | Not checked: nobody read `labd.log` for `lab display keep-awake held=true` | — | Owed |
| 15 | `server: "colo"` fails at `pick_server` | LH-16 part 2 | Not reached: `lab_logout` stops at character select, and `lab_ensure_in_world {server: "colo"}` from there ran only `play` and `ready` (D13.2, recheck R3). No call reached the server list | — | Owed |
| 16 | `lab-driver` lacks tools | LH-17 | Not exercised: this session's agent list was loaded before the LH-17 change, so it still had the old `lab-driver`, and the drivers were general-purpose Haiku agents | — | Owed (next session) |
| LH-19 | `client_move_to` never turns the pawn (close-out finding) | LH-19 | Mouse-look turns only the camera; the pawn turns with the bound TurnLeft / TurnRight keys (Q / E), about 110 deg/s. See item 8 for the live walks (113.2 and 107 deg/s learned, 0 sign flips) | Client-only | Verified live |
| LH-20 | Vault drag never starts (close-out finding) | LH-20 | Before (D8.2, D9.4): vault-to-bag drags pressed `Inventory_Slot1`, which the bag window covers exactly, so `drag_started: false`, `moved: false`, and nothing reached the server. After (recheck R1): Vault 1 to Crafting 2 `raised_window: VaultWin`, `drag_started: true`, `moved: true`, `effect_ok: true`, N3; back the other way `raised_window: InventoryWin`, the same | Before: no move row between 13:22:13 and 13:22:52, nor after 13:22:52 until D10.4. The probe that found it (manual `moveToFront`): 13:28:47 and 13:28:53 swaps BANK to CRAFTING, 13:28:34 `move_rejected` BANK to MAIN `not_allowed_in_container`. R1 itself was not queried | Verified live (N3) |

Two check-sheet expectations were wrong, not the tools: the deposit (D7.4) logged `kind=swap` because vault slot 1 still held a Steel Core from round 1, and the "split" on a 1-stack (D9.2) is a swap too. One field is unexplained: D5.4's diff reported `by_item` 6 to 12 for Steel Core while the inventory read and the server both say 6 were bought.

## Native tiers against the issue's acceptance

The issue asks that split, item use and vendor-quantity steps run at N1, or that the guide states why they can't.

| Step | Tier live | Acceptance |
|---|---|---|
| Item use | N1 (`native_cegui`, the injected right-click) every time | Met |
| Vendor quantity and buy | N1 (`real_input`, the spinner buttons and Buy) | Met |
| Split | Starts natively (CEGUI press and Ctrl-drag, stand-in window followed), lands at N3 | Not met; the guides say why |
| Plain drags (bag, vault) | Start natively, land at N3 | Not met; the guides say why |

Why drags and splits stay at N3 (D-LH3): in every live drag, `drop_target_resolved` stayed false. CEGUI never set the dragged container's `d_dropTarget` over a slot, so a native button release drops on nothing and the item snaps back. The tool then fires the target window's `DragDropItemDropped` itself (`notifyDragDropItemDropped`) before the release, the stock handler sends `moveItem`, and that step is reported `native_call` (N3). The leading suspect is that no window from the slot up to the GUI sheet carries the `DragDropTarget` flag ([cegui-mouse-input-feed.md § 8](../../reverse-engineering/findings/cegui-mouse-input-feed.md#8-follow-up-d_droptarget-stays-0-after-a-live-injected-drag-2026-10-10)); it is not proven. Once CEGUI resolves the target, the same tool reports `drop_target_resolved: true` and N1 with no code change. Both guides now state this: [live-research-lab.md § UI readers and item tools](../../guides/live-research-lab.md#ui-readers-and-item-tools) and [automated-uat.md § Native levels](../../guides/automated-uat.md#native-levels).

What that means for the round-1 grades ([round 1, Results](../haiku-lab-uat/README.md#results)):

- Consumables I2, I3 and I6, graded NATIVE_SHORTFALL (N3) in round 1, now run at N1 and can be regraded PASS. I4 (double use) needs the concurrent check below first.
- Bank 3 (deposit), 4 (withdraw), 5 (split / merge) and 14 (backpack merge) still land at N3 and stay NATIVE_SHORTFALL until the drop-target question is settled. Bank 4 now at least starts: before LH-20 it could not.

One result shape to follow up: the full-health negative check (A4) came back with `native_pass: false` and no `native_level` or `native_tier`, although its one injected right-click was N1 and nothing changing was the expected outcome. A runner that reads `native_pass` would grade a correct refusal as a shortfall. Fixed 2026-10-12 (follow-up commit "a negative item check reports its native click tier"): a negative check now stamps the native level and tier of its single click (`native_cegui`, N1) and `native_pass: true`, and `effect_ok` stays unset.

## What was not tested live

Each of these is a row in [unified-uat.md § Lab harness](../../guides/unified-uat.md#lab-harness).

1. **LH-13 `server_log_tail`.** The second log ring lives in the server's `admin-api`, and the colo runs `main`. It needs the colo deploy after this branch merges, then a lab run with the client telemetry flowing.
2. **LH-16 part 2, the single-row `pick_server`.** `lab_logout` stops at character select, so `lab_ensure_in_world {server: "colo"}` never went back to the server list and `pick_server` never ran. It needs a start from the login screen (a fresh `lab_client_start`).
3. **LH-16 part 1, keep-awake `held=true`.** Nobody read `labd.log` after a launch.
4. **LH-14, the load grace under a real IO stall.** The 38 s cold start under a review build had no stall and no crash, so the grace never engaged. LH-15's lane wait was not observed either.
5. **LH-04's Lua fallback.** The spinner buttons (N1) worked, so the `allow_fallback` path, and the handler name the packet assumed for it (`VendorMod.onBuyClicked`), never ran. That name is unconfirmed.
6. **I4, a concurrent double use.** The two use calls ran one after the other, not together, so the second failed client-side ("no item matching ...") and never reached the server; the server's `consume_refused reason=not_removed` guard was not exercised.
7. **LH-17, the `lab-driver` agent with its new tools.** The session's agent list predates the change.
8. **LH-10's already-in-range case** (brief D4) has no recorded result, and **LH-12's click recording** (COORD-2) was not read back.

## Pre-existing lab issues seen (not caused by this branch)

These showed up during the run; the analysis in [packets-closeout.md](packets-closeout.md#not-a-packet-the-two-small-supervisor-notes) finds that this branch does not cause them.

- **`lab_ensure_in_world`: "in the world but the player's name did not read for 15 s".** Twice in a row on p2 (sessions A and B), while a Lua HP read worked. `unitName(Unit.Player)` reads empty until the player entity is bound; a slow, CPU- or disk-starved load is the suspected cause, not shown.
- **`enter_world` dispatch timeouts on the first try.** p3's first `lab_ensure_in_world` failed at `enter_world` with "bridge error -32603: dispatch timeout" after 11927 ms; the unchanged retry worked.
- **`lab_logout` failures.** "An established connection was aborted by the software in your host machine. (os error 10053)" (default, D13.1) and "bridge error -32603: dispatch timeout" at `character_select` (p2 B5, recheck R3). A second call found the client at character select. CS-08's worknote records the same first-try timeouts before this branch.
- **New tool schemas need `/mcp`.** After a `labd` restart the session keeps the old tool schemas until you reconnect with `/mcp`.
- **General-purpose Haiku drivers run out of context.** They hit their context cap after about 15 to 30 lab calls. Use the lean `lab-driver` agent, which has no memory and only the lab tools; this session could not load the updated one, because its agent list predates LH-17.
