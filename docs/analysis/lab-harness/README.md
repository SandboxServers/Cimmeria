# Lab harness fixes (LH campaign)

Campaign ledger for [#1660](https://github.com/SandboxServers/Cimmeria/issues/1660): the sixteen lab-harness defects found by the Haiku-driven colo UAT round 1 ([haiku-lab-uat](../haiku-lab-uat/README.md)). The goal is that the UAT steps that graded NATIVE_SHORTFALL, PARTIAL or BLOCKED because of the harness run natively in the lab, or the guide says exactly why they can't.

Lab and supervisor only. One product-crate change, LH-13 (`crates/admin-api` log ring), is additive and serves the lab control endpoint. No client patch, no new opcode, no wire change.

- Packets: [work-packets.md](work-packets.md) (dispatch rules and order), then the four packet files it indexes.
- Live probes run before coding: [probes.md](probes.md).
- Close-out (LH-18): [closeout.md](closeout.md), the live result, server evidence and status of every item, the native tiers against the acceptance, and what is still owed.
- Per-packet notes: `worknotes/`. Session handoffs: `handoffs/`.

## Findings

| # | Issue item | Root cause (short) | Packet |
|---|---|---|---|
| 1 | Split drag never starts | A Ctrl-drag hands off to `InventoryDragItemWin`; the tool watches only the slot's container | LH-01 |
| 2 | Main-bag drag "not on screen"; tab click doesn't switch | `reveal_slot` clicks the tab once without reading it back, then retries the same lost click | LH-02 |
| 3 | `client_item_action` reports native_pass, sends nothing | Stock inventory has no double-click handler; `effect_ok` is never set, so any outcome grades native_pass | LH-03 |
| 4 | `__auto_*` children unreachable; vendor quantity | `ui_click` resolves `_G` names only (no `getWindow`); no vendor helper | LH-04 |
| 5 | `client_input_mouse` ignores x/y | The tool has no x/y arguments | LH-05 |
| 6 | Pitch pinned at the clamp; p3 zero pose | `native_look` clamps silently; face gains are not seeded from the camera readout; a zero pose is treated as real | LH-06 |
| 7 | Own avatar occludes the target | Retries pitch before yaw, and a `Nothing` hover ends the retries | LH-07 |
| 8 | `client_move_to` stuck and drifting | 2.5 m arrival radius under the interact range; no walked-away check | LH-10 |
| 9 | Right-click at 6.4 m opens nothing | Server `MAX_INTERACT_DISTANCE` = 5.0 m (3D); the lab never reports distance | LH-08 |
| 10 | `client_player_state` empty stats | Stat read guard and name list (confirmed by probe C-P1) | LH-09 |
| 11 | `client_chat_log` misses a reply; `bridge_dropped` thousands | Chat wrapper armed only by a reader tool; the DLL event ring is never drained between tool calls | LH-11 |
| 11b | Click ring install `failed` | `subscribeGlobal` error swallowed by `pcall` | LH-12 |
| 12 | `server_log_tail` flooded | The shared 500-entry `LogBuffer` evicts server lines on ingest of replayed client telemetry | LH-13 |
| 13 | Startup hangs under build IO | 120 s watchdog kills a client paging in from a saturated disk; every kill counts as a crash | LH-14, LH-15 |
| 14 | Keep-awake never held | It counts only `SGW.exe` with a top-level window | LH-16 |
| 15 | `server:"colo"` fails at `pick_server` | "colo" is a launcher label; the list has one shard named "Test" | LH-16 |
| 16 | `lab-driver` lacks tools | Agent `tools:` line is behind the lab tool set | LH-17 |
| LH-19 | `client_move_to` never turns the pawn (close-out run) | Mouse-look turns only the camera; the pawn's heading answers only to the bound TurnLeft / TurnRight keys (Q / E, about 110 deg/s) | LH-19 |
| LH-20 | Vault-to-bag drag never starts (close-out run) | The bag window opens exactly over the vault, so the press lands on the bag's slot; raise the source window first | LH-20 |

## Decisions

Coordinator defaults, taken from the expanders' recommendations; the owner can reverse any of them in review.

| ID | Decision | Status | Reason |
|---|---|---|---|
| D-LH1 | `effect_ok` is checked by default for item and vendor actions; `expect_change:false` for negative checks | Accepted | A native_pass that changed nothing is the item-3 bug |
| D-LH2 | `doubleclick` on an inventory slot is performed as a right-click and reported as `requested`/`performed` | Accepted | Stock Lua has no double-click handler |
| D-LH3 | Splits and plain drags stay N3 until the drop-target question is settled; the guide states why | Accepted | No honest N1 landing is known |
| D-LH4 | `client_world_click` reports interact range, does not auto-approach | Accepted | `client_move_to` is unreliable in the hub; revisit later |
| D-LH5 | Default entity arrival radius 2.5 m to 4.0 m | Accepted | Inside the server's 5.0 m interact range |
| D-LH6 | Event keeper on by default; `CIMMERIA_LAB_EVENT_KEEPER=0` turns it off | Accepted | Drains the DLL ring and re-arms capture after relog |
| D-LH7 | No path-finding, teleport or held-key fallback inside `client_move_to` | Accepted | Failure text and guide point at waypoints, held keys, teleport |
| D-LH8 | The DLL's drop-newest ring stays; the supervisor keeper covers the loss | Accepted | No client-telemetry redeploy |
| D-LH9 | LH-13 adds a second ring to `admin-api`'s `LogBuffer` | Accepted | Additive; alternative is a larger lab-only subscriber |
| D-LH10 | Load-phase grace 300 s while IO progresses; load stalls have their own budget (6/h) | Accepted | Kills during page-in are not crashes |
| D-LH11 | Client launch waits up to 90 s for the build lane to go idle, never refuses | Accepted | Moving the client to the Dev Drive is an environment choice, not code |
| D-LH12 | A server list with one row selects it when the requested name is missing | Accepted | Several rows with no match stay an error |
| D-LH13 | With `expect_change:false` the item-action escalation stops after the first native click | Accepted | A negative check must not send extra use attempts |
| D-LH14 | `client_vendor_buy` reports `double_buy_risk` when a Buy retry ran after a silent click | Accepted | Same as item use (D-LH1); a slow server can land the first click late |
| D-LH15 | The event keeper ticks unless the client is known to be busy: logging in, at character select, crashed, no heartbeat yet, or a heartbeat stalled for 3 s (a load) | Accepted | A tick holds the bridge mutex up to 3 s; during load it would contend with the watchdog heartbeat. Gating on `in_world` left a client logged in by `ui_sequence`, already in world, or `failed` after a timeout without a keeper |

## Packets

| Packet | Title | Size | Depends on | Status |
|---|---|---|---|---|
| LH-01 | Split drag follows the stand-in window; fake client extended | M | | Integrated |
| LH-02 | Verified tab and filter clicks | M | LH-01 | Integrated |
| LH-03 | `client_item_action` honest use | M | LH-02 | Integrated |
| LH-04 | `ui_click` name resolution and `client_vendor_buy` | L | LH-03 | Integrated |
| LH-05 | `client_input_mouse` takes x/y | S | | Integrated |
| LH-06 | Pitch clamp detected and recovered; zero pose rejected | M | | Integrated |
| LH-07 | `world_click` dodges the own avatar (yaw retries first) | L | LH-06, probe B-P2 | Integrated |
| LH-08 | `world_click` reports interact range | S | LH-07 | Integrated |
| LH-09 | `client_player_state` stats | S | probe C-P1 | Integrated |
| LH-10 | `client_move_to` stops at range, aborts on walk-away | M | LH-06 (steer.rs) | Integrated |
| LH-11 | Event keeper: capture armed, ring drained, honest drops | M | probes C-P2, C-P3 | Integrated |
| LH-12 | Click ring install error text | S | LH-11, probe C-P4 | Integrated |
| LH-13 | `server_log_tail` keeps server lines | M | | Integrated |
| LH-14 | Watchdog IO-progress load grace and stall budget | M | | Integrated |
| LH-15 | Wait for an idle build lane before a launch | S | LH-14 | Integrated |
| LH-16 | Keep-awake sees windowless clients; single-row server pick | S | | Integrated |
| LH-17 | `lab-driver` agent tools and docs | S | all tool packets | Integrated |
| LH-19 | `client_move_to` steers with the turn keys (mouse-look turns only the camera) | L | close-out finding | Integrated |
| LH-20 | Drags raise the source slot's window first (bag covers the vault) | S | close-out finding | Integrated |
| LH-18 | Close-out: live lab UAT of every fix, guide, unified-uat, status ([closeout.md](closeout.md)) | M | all | Done |
