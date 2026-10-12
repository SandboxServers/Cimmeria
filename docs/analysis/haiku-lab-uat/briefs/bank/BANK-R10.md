<!-- markdownlint-disable MD029 -- step numbers run on across sections and are cited by number -->

# BANK-R10 - step 10: an item sold to a vendor cannot be dragged out of the buyback tab for free

Track: Bank and vault, personal bank. Re-brief of step 10 using the live-run recipes.
Instance: `default`. Lease: `<LEASE_ID>` (lent; pass as `lease_id` on every driving tool; never acquire/status/release).
Server: COLO. Touch only `Bankone`.
Tools you have: client_batch (lua), client_ui_sequence, client_chat_log, client_inventory, client_drag_drop, client_window_read/click, client_world_click, client_camera, client_entity_find, lab_screenshot. You do NOT have client_lua_eval, client_item_action or client_move_to. Never walk; teleport.
Preconditions: `Bankone` in Castle_CellBlock; Main has Accessory (slot 1) and 4 Health Slappack TC1 (slot 2); cash about 894 (record the real value); no dialog open. Coordinates are server metres.
Postcondition: one item sold (cash up), still in the buyback tab; nothing bought back.

## Rules

- Read replies ONLY with `client_chat_log` {"lease_id":"<LEASE_ID>","cursor":"r10","max":20}; it sometimes misses a reply, so repeat a command once if nothing shows.
- `client_window_read` rejects fields title/buttons/text: use `fields:["state","open","primary"]` (+ "nodes" with include_nodes).
- Moves use stock Lua `moveItem(srcContainer, srcSlot, dstContainer, dstSlot, qty)`, 1-based slots, via `client_batch` {"op":"lua","chunk":"..."}. Main is container 1. Buyback is container 16 (ASSUMPTION from the bank telemetry `source_container_id=16`).
- client_drag_drop only works for slots on the visible inventory tab (and the Inventory tab buttons do not switch tabs).

## Steps (numbered; do in order)

1. Teleport to the Quartermaster: send chat `.gotolocation Castle_CellBlock -339.5 73.47 -226.5` (macro: close dialog; `client_ui_sequence` click `Inst1Chat_Input` + type; `client_batch` Lua `return Inst1Chat_Input:getText()` must equal the line; `client_ui_sequence` key Enter + wait 2000). Wait 4 s. NOTE: the Common Materials Components vendor (100485) is nearer than the Quartermaster from here; do not click it.
2. `client_entity_find` {"lease_id":"<LEASE_ID>","name":"Quartermaster","fields":["matches"]} -> id Q, `distance_m` (hub table: the Quartermaster is at (-343.90, 73.47, -228.49); expect about 4.8; it must be 5.0 or less, else teleport to (-340.5, 73.47, -226.8) and re-measure). Record the id and distance.
3. `client_camera` {"lease_id":"<LEASE_ID>","face_entity_id":<Q>,"zoom_to":400}. If Q is not centred: `client_camera` {"pitch_counts":-650} then face again. `lab_screenshot` ("r10-before-click"; confirm the Quartermaster is visible and unobstructed).
4. `client_world_click` {"lease_id":"<LEASE_ID>","entity_id":<Q>,"button":"right","expect":"window","settle_ms":4000,"rotate_camera":false}. Retry once on failure (screenshot, report path).
5. One call: `client_inventory` {"lease_id":"<LEASE_ID>","containers":["Main"],"fields":["cash","containers"],"snapshot":"r10a"} -> CASH_BEFORE; and `client_window_read` {"lease_id":"<LEASE_ID>","kind":"vendor","include_nodes":true,"max_nodes":80,"fields":["state","open","primary","nodes"]}. Record the window name (expected `VendorWin`) and the names of the sell/buyback tabs and the buyback list.
6. Sell one Health Slappack: `client_drag_drop` {"lease_id":"<LEASE_ID>","from":{"container":"Main","slot":<Slappack slot as the tool prints it; Main wire slot 1 if 0-based>},"to":{"window":"VendorWin"},"split":true,"wait_ms":4000}. If the vendor refuses or nothing changes, retry with the Accessory slot (wire slot 0) and no split. Then `client_inventory` {"containers":["Main"],"diff_against":"r10a","fields":["cash","diff"]} -> CASH_AFTER_SELL.
   PASS (setup): the item left Main and cash rose. If neither item can be sold: step 10 BLOCKED ("vendor will not buy"); record the chat line and stop.
7. Open the buyback tab: `client_window_click` {"lease_id":"<LEASE_ID>","text":"Buyback","root":"VendorWin"} (if no match, click the tab name read in step 5). `lab_screenshot` ("r10-buyback-tab"). Confirm the sold item appears in the buyback list (`client_window_read` same call as step 5).
8. Try to take it out for free, two ways, in this order:
   a. Lua: `client_batch` [{"op":"lua","id":"mv","chunk":"moveItem(16, 1, 1, <an EMPTY Main slot, Lua 1-based; read an empty one from client_inventory include_empty>, 1) return true"},{"op":"wait","ms":2500}]
   b. If the buyback slot is on screen, also `client_drag_drop` from {"container":"Buyback","slot":0} to {"container":"Main","slot":<same empty slot>} (ASSUMPTION about the container name; if the tool errors on the name, skip b and record the error text).
9. Verify: `client_chat_log` (record any refusal line verbatim), `client_inventory` {"containers":["Main"],"diff_against":"r10a","fields":["cash","diff"]} -> CASH_AFTER_DRAG. `lab_screenshot` ("r10-after-drag").
   PASS: the item did NOT arrive in Main and cash is unchanged versus CASH_AFTER_SELL (nothing charged and nothing given). FAIL if the item appears in Main without a purchase. A visible refusal message is expected but its text is unknown (record it; no line is a note, not a fail).
10. Close the vendor: `client_window_click` {"lease_id":"<LEASE_ID>","target":"VendorWin__auto_closebutton__"}.

## Stop conditions

Real block: Quartermaster never found or out of range after one re-teleport; the vendor window never opens after two clicks; vendor buys neither item; unexpected dialog or window (one `client_ui_state`, report, stop). Max 2 attempts per step. Do not restart the client. Do not release the lease.

## Report

| step_id | action done | raw observation | PASS/FAIL/BLOCKED claim | screenshot path | UTC time |

Rows: 10 (setup/sell), 10 (drag-out). Include Q id, distance_m, CASH_BEFORE / AFTER_SELL / AFTER_DRAG, the diff results, the chat lines.

---

## Orchestrator only: verify

- SigNoz `scope_name = 'bank' AND event = 'move_rejected' AND reason = 'source_container_not_player_movable' AND source_container_id = 16` with the sold `item_id`. The Lua moveItem is a real client send, so the row proves the server saw it.
- The sale itself: `server_log_tail` search `sell` / `buyback`; vendor sell has no bank row. Check cash via `server_entity_get` / DB (cash column name ASSUMPTION): CASH_AFTER_DRAG equals CASH_AFTER_SELL.
- `server_db_query` that the sold item is not in Bankone's Main bag afterwards (probe `sgw.sgw_inventory` columns first; names are an ASSUMPTION).
- Packet tap (optional): `server_packet_tap_start` before step 8, read after; look for the resync of the buyback container after the refusal. WARNs under `bank` or `inventory` not expected by the step are findings.
