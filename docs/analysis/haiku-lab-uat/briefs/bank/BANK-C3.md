<!-- markdownlint-disable MD029 -- step numbers run on across sections and are cited by number -->

# BANK-C3 - Storage Officer click, walk-away refusal, mission item, banked-item guards, buyback (steps 2, 7, 8, 9, 10)

Track: Bank and vault, personal bank. Chunk 3 of 4.
Instance: `default` (account `lab`). Lease: `<LEASE_ID>` (lent; pass as `lease_id` on every driving tool; never acquire/status/release).
Server: COLO. Touch only `Bankone`. Do not click any entity except the ones named below.
Preconditions (left by BANK-C2): `Bankone` in Castle_CellBlock (world 12; if C2 was cut short it may still be in SGC_W1 world 58, then A0 below covers it), vault empty, some Steel Cores (type 5254) in a carried bag, naquadah roughly 450 or more, no dialog open.
Postcondition for C4: vault holds exactly the one item deposited in A6 (or is empty if you withdrew it), `Bankone` still online in the stasis room.

## Hub facts (server metres, x / y(up) / z; verified from docs/content/debug-hub.md and db/resources spawn rows)

| Name shown | Where | Note |
|---|---|---|
| Storage Officer, PERSONAL Banker (spawn 470) | (-325.92, 73.47, -231.18) | woman in SGC uniform |
| Storage Officer, TEAM Banker (471) | (-327.65, 73.47, -228.09) | Cellblock-guard uniform; 3.55 from the personal one |
| Storage Officer, COMMAND Banker (472) | (-325.84, 73.47, -224.74) | plain crew clothes |
| Basic Equipment Quartermaster (vendor, spawn 400) | (-343.90, 73.47, -228.49) | |
| Respawner / arrival spot | (-334.23, 73.47, -228.03) | 8.9 m from the personal Banker |

All three Bankers are named "Storage Officer" (K17). Never click by name. Identify the personal Banker by POSITION: the `client_entity_find` match whose `position.server` is within 0.6 m of (-325.92, -231.18) in x and z. Clicking the Team or Command Banker is NOT a failure of the server, but it is a wrong click for this chunk: record which one answered. "Expand vault" dialog is quarantined (K3): its absence is the expected result.

## Standing autonomy

Lua reads, listed GM commands, walking with client_move_to. Stop only on a real block.

## CHAT(line) macro

1. Close any dialog (Lua `DialogMod.onDialogDoneClicked(DialogWin, DialogWin)` if `DialogWin:isVisible()`).
2. `client_ui_sequence` {"lease_id":"<LEASE_ID>","actions":[{"do":"click","window":"Inst1Chat_Input"},{"do":"type","text":"<line>","into":"Inst1Chat_Input"}]}
3. `client_batch` Lua `return Inst1Chat_Input:getText()` must equal `<line>`; else `Inst1Chat_Input:setText('')`, retype once.
4. `client_ui_sequence` {"actions":[{"do":"key","key":"Enter"},{"do":"wait","ms":2000}]}
5. `client_chat_log` {"lease_id":"<LEASE_ID>","cursor":"bank","max":20}

## Actions

Coordinates: server metres (x, y up, z) in every command here. Client units = (z, x, y) * 100, so server (-330.0, 73.47, -229.2) reads about [-22920, -33000, 7347] in the client. Tool note: `client_window_read` rejects fields title/buttons/text; use `fields:["state","open","primary"]` or omit. Read chat replies ONLY with `client_chat_log` (a Lua read of the chat output returns nil).

**A0. Make sure you are in the hub (`Bankone` spawned in SGC_W1, which has no Storage Officer).**
 `client_batch` {"lease_id":"<LEASE_ID>","steps":[{"op":"player_state","fields":["world_id","position"]}]}. If world_id is already 12 (Castle_CellBlock; ASSUMPTION per debug-hub.md) skip to A1. Else CHAT(`.gotolocation Castle_CellBlock -330.0 73.47 -229.2`), then `client_wait_for` {"lease_id":"<LEASE_ID>","lua_condition":"getCurrentWorldID() == 12","timeout_ms":60000}, wait 17 s, re-read player_state.

**A1. Close the vault, find the personal Banker, approach.**
 a. `client_batch` {"lease_id":"<LEASE_ID>","steps":[{"op":"lua","id":"v","chunk":"return VaultWin:isVisible()"}]}; if true Lua `VaultWin:setVisible(false)`.
 b. `client_entity_find` {"lease_id":"<LEASE_ID>","name":"Storage Officer","limit":5,"fields":["matches"]}. Pick the match by position (see above). Record all three matches (id, position.server, distance_m). Call the picked id BANKER_ID.
 c. `client_move_to` {"lease_id":"<LEASE_ID>","entity_id":<BANKER_ID>,"arrival_m":3.5,"timeout_ms":30000}. The room is crowded (registrars and the pet trainer stand near it). If it fails, retry once; if it fails again, teleport with CHAT(`.gotolocation Castle_CellBlock -330.0 73.47 -229.2`) (open floor 4.5 m from the personal Banker at (-325.92, 73.47, -231.18); the Team Banker is 2.6 m and the Command registrar 3.4 m from it, so expect a crowded spot; ASSUMPTION: the hub placement is unchecked in the client) and then `client_move_to` the Banker with arrival_m 3.5 again. The first approach should already start from a hub teleport (A0) when coming from SGC_W1.
 d. `client_entity_find` {"entity_id":<BANKER_ID>,"fields":["matches"]}: `distance_m` must be about 3 to 4.5 (server range is 5.0 m). Record it.
 e. `client_camera` {"lease_id":"<LEASE_ID>","face_entity_id":<BANKER_ID>,"pitch_deg":-15,"yaw_offset_deg":25,"zoom":250}. Read `view_after.pitch_offset_deg`.
 f. `lab_screenshot` {"lease_id":"<LEASE_ID>"} - confirm the Banker is visible and not under your avatar; record the path.

**A2. Step 2 - right-click the personal Banker; the vault opens on the FIRST click.**
 `client_world_click` {"lease_id":"<LEASE_ID>","entity_id":<BANKER_ID>,"button":"right","expect":"window","settle_ms":4000,"rotate_camera":false}
 Then `client_batch` {"lease_id":"<LEASE_ID>","steps":[{"op":"lua","id":"size","chunk":"return getContainerSize(Container.Vault)"},{"op":"lua","id":"vis","chunk":"return VaultWin:isVisible()"},{"op":"lua","id":"dlg","chunk":"return DialogWin ~= nil and DialogWin:isVisible()"}]} and `client_ui_state` {"lease_id":"<LEASE_ID>","fields":["windows","dialog"]}.
 PASS: size 40, VaultWin visible, DialogWin NOT visible and no Expand-vault dialog anywhere in the window list, on the first click (count clicks; the click count must be 1).
 Failure handling: on a click error take `lab_screenshot`, report the path, retry once. If a click opened a different window (a Team/Command refusal in chat), record `client_chat_log` and that you hit the wrong Banker.
 `lab_screenshot` ("step2-vault-open") and record.

**A3. Deposit one item so step 7 has something to move.**
 `client_inventory` {"lease_id":"<LEASE_ID>","containers":["Crafting","Main","Vault"],"include_empty":true,"snapshot":"c3a"}. Choose a Steel Core slot (container, slot) and the first empty Vault slot V1 (use the slot numbering the tool prints). `client_drag_drop` {"from":{"container":"<C>","slot":<N>},"to":{"container":"Vault","slot":<V1>},"split":true,"wait_ms":4000}. Expect moved true (one core in the vault). Record.

**A4. Step 7 - walk more than 5 m away with the vault window still open, then drag.**
 a. Pick a target point on open floor about 9 m from the Banker, toward the respawner: `client_move_to` {"lease_id":"<LEASE_ID>","point":{"x":-334.2,"y":73.47,"z":-228.0},"arrival_m":1.5,"timeout_ms":30000}.
 b. `client_entity_find` {"entity_id":<BANKER_ID>,"fields":["matches"]}: `distance_m` must be greater than 6. Record. Also Lua `return VaultWin:isVisible()` (the client does not close the window by itself; if it is hidden, mark the step BLOCKED "window closed on its own" and stop this step).
 c. Snapshot `client_inventory` {"containers":["Crafting","Main","Vault"],"snapshot":"c3b"}.
 d. Try to withdraw: `client_drag_drop` {"from":{"container":"Vault","slot":<V1>},"to":{"container":"<C>","slot":<an empty slot in the carried bag>"},"wait_ms":4000,"allow_fallback":true}. Record `moved`, `snap_back`.
 e. `client_chat_log` {"cursor":"bank","max":20}: record any refusal line verbatim (expected: a visible message, text unknown: ASSUMPTION).
 f. `client_inventory` {"diff_against":"c3b"}: expect no change; the item is still in the vault.
 PASS: the move is refused with a chat/feedback line AND the inventory is unchanged (`moved` false or snap_back true). FAIL if the item moved. If `moved:false` and no line appears anywhere, claim FAIL ("no visible message") and attach a `lab_screenshot` path.
 Then walk back: `client_move_to` entity BANKER_ID arrival_m 3.5.

**A5. Step 8 - bank a mission item.**
 `client_inventory` {"containers":["Mission"]} - list Mission-bag items. If the Mission bag is EMPTY, mark step 8 BLOCKED ("no mission item on a fresh character"; do not invent one) and go to A6. Otherwise drag the first Mission item to an empty Vault slot (`client_drag_drop` from {"container":"Mission","slot":<n>} to {"container":"Vault","slot":<V2>}); if the tool refuses because the Mission tab is not on screen, click the Mission tab once (find its name via `client_window_read` {"window":"InventoryWin","include_nodes":true,"max_nodes":120}). Record `moved`, `snap_back`, and the chat line.
 PASS: refused with a message, item still in Mission bag, vault slot empty.

**A6. Step 9 - banked item cannot be sold, mailed or used (partial: trade is EXCLUDED, needs a second player).**
 a. With the Steel Core still in the vault, mail test: CHAT(`.mail to Bankone item 5254 1 vaulttest`). ASSUMPTION: syntax per unified-uat.md (`.mail [to <name>] [cash <n>] [item <typeId> [qty]] [cod <n>] [<subject>]`); the command attaches by type id from carried bags and should refuse an item that exists only in the vault ("item_in_vault"). Record the chat line verbatim. If it says you cannot mail yourself, record that and mark the mail half BLOCKED (do not mail anyone else).
 b. Vendor list: open the Quartermaster: `client_entity_find` name "Basic Equipment Quartermaster" -> `client_move_to` entity arrival_m 3.5 -> `client_world_click` {"entity_id":<id>,"button":"right","expect":"window","settle_ms":3000}. NOTE: any right-click on another NPC ends the vault session; that is expected. `client_window_read` {"kind":"vendor","fields":["state","open","primary"]}: record the sell/inventory rows. PASS (vendor half): the vaulted Steel Core is not offered for sale (it does not appear in the vendor's sell list). Leave the vendor window open for A7.

**A7. Step 10 - sell an item, then try to drag it out of the buyback tab without paying.**
 a. Choose a sellable item that is NOT the Steel Core in the vault and not an equipped item: `client_inventory` {"containers":["Main","Crafting"]}; pick any Main item, else a Steel Core from the crafting bag. Record `cash` before.
 b. Sell it: right-click the item's slot in the vendor/inventory (`client_item_action` {"lease_id":"<LEASE_ID>","action":"rightclick", ...} needs the item id from the read) or drag the slot onto the vendor window (`client_drag_drop` to {"window":"<vendor window name from client_window_read kind vendor>"}). ASSUMPTION: either sells the item. Record the method and cash after.
 c. Find the buyback tab: `client_window_read` {"window":"<vendor window>","include_nodes":true,"max_nodes":150}; click the tab whose text/name contains "Buy" and "Back" with `client_window_click` {"text":"Buyback","root":"<vendor window>"}. Take `lab_screenshot` ("buyback-tab").
 d. Drag the sold item out of buyback into an empty carried slot: `client_drag_drop` from {"container":"Buyback","slot":0} (ASSUMPTION about the container name; if the tool errors, read `client_inventory` with no container filter for the buyback container name) to {"container":"Main","slot":<empty>}. Record moved/snap_back/chat line. Cash must not change.
 PASS: refused (nothing moved, cash unchanged). If the container name cannot be resolved after two tries: BLOCKED.
 Close the vendor window (`<Win>__auto_closebutton__` via client_window_click).

## Stop conditions

- Real block: personal Banker cannot be identified by position; every click on the Banker fails twice with screenshots showing no occluder; the Basic Equipment Quartermaster does not exist; unexpected window or dialog (one `client_ui_state`, report, stop).
- A click on the wrong Banker: record and try the right one once.
- Max 2 attempts per step. Do not restart the client. Do not release the lease.

## Report format

| step_id | action done | raw observation | PASS/FAIL/BLOCKED claim | screenshot path | UTC time |

Rows: 2, 7, 8, 9(vendor), 9(mail), 10. Include BANKER_ID, its distance at click time, and click count in row 2.

---

## Orchestrator only: verify list

- Resolve BANKER_ID: `server_entity_query` / `server_entity_get` for the entity id the driver used; confirm its template is 370 (`DebugHub_Banker`, spawn 470) and not 371/372. The SigNoz `banker_id` on `vault_session_opened` must equal that entity.
- Step 2: SigNoz `scope_name = 'bank' AND event = 'vault_session_opened' AND scope = 'personal'` with `banker_id` and `distance` <= 5 (span `bank.banker_interact`); quarantine row `event = 'expand_offer_suppressed' AND reason = 'dialog_quarantined'` is EXPECTED; any `vault_open_rejected` is a finding. `server_log_tail` search `banker` / `interact` / `too far`. Count click events to confirm "first click".
- Step 7: `event = 'move_rejected' AND reason = 'banker_out_of_range'` with `vault_end`, `banker_id`, `distance` > 5; absence of `move_resync_skipped` for that item_id (snap-back sent). `server_packet_tap_start` before A4d (session = Bankone from `server_sessions`) and read after: look for the refusal line (chat) and the resync of container 17.
- Step 8: `event = 'move_rejected' AND reason = 'mission_item_not_bankable' AND vault_end = 'target'` with `item_id`, `type_id`.
- Step 9: mail `scope_name = 'mail' AND event = 'mail.attachment_refused' AND reason = 'item_in_vault'`; vendor sell list has no `bank` row by design.
- Step 10: `event = 'move_rejected' AND reason = 'source_container_not_player_movable' AND source_container_id = 16`. Also DB cash before/after via `server_entity_get` or `.bankdump` unaffected.
- The A6a mail may deposit a mail row if it succeeds: check `server_db_query` on the mail table (ASSUMPTION `sgw.sgw_mail`); a successful mail of a vaulted item is a FAIL for step 9.
