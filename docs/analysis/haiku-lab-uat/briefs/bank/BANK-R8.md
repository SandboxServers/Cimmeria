<!-- markdownlint-disable MD029 -- step numbers run on across sections and are cited by number -->

# BANK-R8 - step 8 (mission item refused) and step 9 (banked item cannot be mailed or sold)

Track: Bank and vault, personal bank. Re-brief of steps 8 and 9 using the recipes proven in the live run.
Instance: `default`. Lease: `<LEASE_ID>` (lent; pass as `lease_id` on every driving tool; never acquire/status/release).
Server: COLO. Touch only `Bankone`.
Tools you have: client_batch (lua), client_ui_sequence, client_chat_log, client_inventory, client_drag_drop, client_window_read/click, client_world_click, client_camera, client_entity_find, lab_screenshot. You do NOT have client_lua_eval, client_item_action or client_move_to. Never walk with client_move_to; teleport instead.
Preconditions: `Bankone` in Castle_CellBlock near (-330, 73.47, -229.2); vault 100 slots holding 1 Steel Core (wire slot 0 = Lua slot 1); Crafting bag has Steel Cores (stacks of 1 and 4); Main has Accessory (slot 1) and 4 Health Slappack TC1 (slot 2); cash 894. No dialog open.
Postcondition: all Steel Cores in the vault; the Mission item (if it landed there) still in the Mission bag; vault session closed by the vendor click.
Coordinates are server metres (x, y up, z).

## Rules

- Read chat replies ONLY with `client_chat_log` {"lease_id":"<LEASE_ID>","cursor":"r8","max":20}. It sometimes misses a reply: if nothing shows after a command, send that command once more.
- `client_window_read` rejects fields title/buttons/text: use `fields:["state","open","primary"]` or none.
- Item moves use the stock Lua `moveItem(srcContainer, srcSlot, dstContainer, dstSlot, qty)` with 1-based slots, through `client_batch` {"op":"lua","chunk":"..."}. Container ids: Main 1, Crafting 15, Vault 17, Mission = whatever Lua `return Container.Mission` answers.
- CHAT(line): (1) close any dialog (Lua `if DialogWin:isVisible() then DialogMod.onDialogDoneClicked(DialogWin, DialogWin) end`); (2) `client_ui_sequence` actions [{"do":"click","window":"Inst1Chat_Input"},{"do":"type","text":"<line>","into":"Inst1Chat_Input"}]; (3) `client_batch` Lua `return Inst1Chat_Input:getText()` must equal the line (else `Inst1Chat_Input:setText('')` and retype once); (4) `client_ui_sequence` [{"do":"key","key":"Enter"},{"do":"wait","ms":2000}]; (5) `client_chat_log`.

## Part 1 - step 8: a mission item cannot be banked

1. CHAT(`/gmgiveitem 1893 1`) (Opheltes's Injection, a mission item). Then one call: `client_batch` steps [{"op":"lua","id":"mc","chunk":"return Container.Mission"}] and `client_inventory` {"lease_id":"<LEASE_ID>","containers":["Mission","Main","Vault"],"snapshot":"r8a"}. Record `Container.Mission` and where item 1893 landed.
   - If it is NOT in the Mission bag: step 8 is BLOCKED ("1893 did not land in Mission"). Report the inventory and skip to Part 2.
   - Note its slot M (the slot number the tool prints; Lua uses 1-based, the tool says "wire" 0-based: use M+1 in Lua if the tool prints 0-based).
2. CHAT(`.bank`) to open the GM vault session. Verify: `client_batch` [{"op":"lua","chunk":"return VaultWin:isVisible()"},{"op":"lua","chunk":"return getContainerSize(Container.Vault)"}] expect true, 100.
3. Try to bank it: `client_batch` [{"op":"lua","id":"mv","chunk":"moveItem(Container.Mission, <M+1>, Container.Vault, 2, 1) return true"},{"op":"wait","ms":2500}] (vault Lua slot 2 is empty; slot 1 holds the Steel Core).
4. `client_chat_log` (record any refusal line verbatim; expected: a visible refusal message, text unknown) and `client_inventory` {"containers":["Mission","Vault"],"diff_against":"r8a"}; `lab_screenshot` ("step8-refused").
   PASS: refusal line seen AND the mission item is still in the Mission bag, vault slot 2 empty. FAIL if the item reached the vault. If no refusal line but also nothing moved: claim FAIL "silent refusal" (the guide wants a message) and say so.
   One retry max (repeat 3-4 once if the chat shows nothing).

## Part 2 - step 9: a banked item cannot be mailed or sold

5. Put EVERY Steel Core into the vault so no carried copy of the type exists: `client_inventory` {"containers":["Crafting","Main"],"include_empty":false} -> note each Steel Core stack (Lua slot s_i = tool slot +1 if the tool prints 0-based, quantity q_i). For each stack, one Lua step in a single `client_batch`: `moveItem(15, <s_i>, 17, <free vault slot, start at 3>, <q_i>)`, then {"op":"wait","ms":1500} between them. Then `client_inventory` {"containers":["Crafting","Main","Vault"]}: expect zero Steel Cores carried, all of them in the vault. Record. (If a move is refused, record the chat line and stop this part: BLOCKED.)
6. Mail: CHAT(`.mail to Bankone item 5254 1 vaulttest`). ASSUMPTION: syntax from unified-uat.md; Steel Core type id 5254. Read `client_chat_log`. Expected: a refusal because the item exists only in the vault (no "sent" line). If the reply says you cannot mail yourself, record it and mark the mail half BLOCKED. Retry the command once if nothing shows.
   PASS: refusal, and no "mail sent" line.
7. Vendor sell list. Teleport: CHAT(`.gotolocation Castle_CellBlock -337.0 73.47 -227.5`) (4.8 m from the Common Materials Components vendor; no walking). Wait 4 s. `client_entity_find` {"lease_id":"<LEASE_ID>","name":"Common Materials Components","fields":["matches"]} -> id V (expected 100485), record distance_m (must be 5.0 or less; if more, teleport again to (-338.0, 73.47, -227.0)).
8. `client_camera` {"lease_id":"<LEASE_ID>","face_entity_id":<V>,"zoom_to":400}. If the vendor is not centred, `client_camera` {"pitch_counts":-650} then face again. `lab_screenshot` once.
9. `client_world_click` {"lease_id":"<LEASE_ID>","entity_id":<V>,"button":"right","expect":"window","settle_ms":4000,"rotate_camera":false}. This also closes the vault session (expected).
10. `client_window_read` {"lease_id":"<LEASE_ID>","kind":"vendor","include_nodes":true,"max_nodes":80,"fields":["state","open","primary","nodes"]}. Record the vendor's rows and whether a Sell/Buyback tab lists anything named "Steel Core". Then `client_inventory` {"containers":["Crafting","Main","Vault"]} (vault listed only while a window is open: record what it prints).
    PASS (vendor half): there is no carried Steel Core to sell and the vendor window offers none from the vault; try ONE sell attempt of the vaulted item: `client_drag_drop` is impossible (vault not visible), so just record that the vendor's sell area shows no Steel Core. Close the vendor: `client_window_click` {"target":"VendorWin__auto_closebutton__"}.
    (Trade is EXCLUDED: needs a second player.)

## Stop conditions

Real block: `/gmgiveitem` unknown (record the answer); 1893 lands somewhere other than Mission (BLOCKED step 8 only); a moveItem refusal in step 5; unexpected dialog or window (one `client_ui_state`, report, stop); lease expired. Max 2 attempts per step. Do not restart the client. Do not release the lease.

## Report

| step_id | action done | raw observation | PASS/FAIL/BLOCKED claim | screenshot path | UTC time |

Rows: 8, 9(mail), 9(vendor). Include the raw Container.Mission value, the chat refusal lines verbatim, and the inventory states.

---

## Orchestrator only: verify

- Step 8: SigNoz `scope_name = 'bank' AND event = 'move_rejected' AND reason = 'mission_item_not_bankable' AND vault_end = 'target'` with `item_id`, `type_id = 1893`; `server_log_tail` search `mission_item_not_bankable`. No `move_accepted` for that item_id. Check `server_db_query` (probe columns first with `SELECT * FROM sgw.sgw_inventory LIMIT 1`; names are an ASSUMPTION) that 1893 is still in container 2 (Mission bag, per the container_sets = {2} note in bv-03.md; confirm id).
- Step 9 mail: `scope_name = 'mail' AND event = 'mail.attachment_refused' AND reason = 'item_in_vault'`; also no new mail row for Bankone (mail table name ASSUMPTION `sgw.sgw_mail`). A mail sent = FAIL.
- Step 9 moves (step 5): `event = 'move_accepted' AND kind = 'deposit'` rows; any `move_rejected` there is a finding.
- Vendor: no bank row by design. WARNs under `bank` not expected by a step are findings.
