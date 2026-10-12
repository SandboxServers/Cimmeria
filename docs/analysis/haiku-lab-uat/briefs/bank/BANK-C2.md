<!-- markdownlint-disable MD029 -- step numbers run on across sections and are cited by number -->

# BANK-C2 - deposit, withdraw, split/merge, backpack merge (steps 3, 4, 5, 14)

Track: Bank and vault, personal bank. Chunk 2 of 4.
Instance: `default` (account `lab`). Lease: `<LEASE_ID>` (lent; pass as `lease_id` on every driving tool; never acquire/status/release).
Server: COLO. Touch only your own character `Bankone`. Do not click or target other players.
Preconditions (left by BANK-C1, measured live): `Bankone` (player 71) stands in SGC_W1 (world 58, about [4972, 20150, 131] in client units), NOT in the stasis room. GM, `.bank` opens 40 slots, cash 0, no dialog open. Step 12 is already done, so cash may now be granted and spent.
Coordinates: every `.gotolocation` and `point` below is SERVER metres (x, y up, z), as in db/resources and docs/content/debug-hub.md. The client reports UE3 units: client = (z, x, y) * 100, so server (-337.0, 73.47, -227.5) reads about [-22750, -33700, 7347] in client units. Do not mix the two.
Postcondition for C3: vault EMPTY (everything withdrawn at the end), `Bankone` holds several Steel Cores (type 5254) in the crafting bag, vault window may be open or closed, `Bankone` stands in Castle_CellBlock (world 12) near the Common Materials Components vendor.
Tool note: `client_window_read` rejects the field names title/buttons/text. For `kind:"vault"` or `kind:"vendor"` use `fields:["state","open","primary"]` (or omit `fields`). Replies to chat commands are read ONLY with `client_chat_log`; a Lua read of the chat output returns nil.

## Standing autonomy

Lua reads, listed GM commands, GM teleport, buying from the Common Materials Components vendor. Stop only on a real block.

## CHAT(line) macro (same as C1)

1. Close any dialog: Lua `DialogMod.onDialogDoneClicked(DialogWin, DialogWin)` if `DialogWin:isVisible()`.
2. `client_ui_sequence` {"lease_id":"<LEASE_ID>","actions":[{"do":"click","window":"Inst1Chat_Input"},{"do":"type","text":"<line>","into":"Inst1Chat_Input"}]}
3. `client_batch` Lua `return Inst1Chat_Input:getText()` must equal `<line>`; else `Inst1Chat_Input:setText('')` and retype once.
4. `client_ui_sequence` {"actions":[{"do":"key","key":"Enter"},{"do":"wait","ms":2000}]}
5. `client_chat_log` {"lease_id":"<LEASE_ID>","cursor":"bank","max":20}

## Actions

**B0. Teleport from SGC_W1 into the Castle_CellBlock stasis room (steps 3-5 and 14 themselves work anywhere with `.bank`, but the vendor is in the hub).**
 a. CHAT(`.gotolocation Castle_CellBlock -337.0 73.47 -227.5`) - open floor in the hub, about 3.6 m from the Common Materials Components vendor at (-341.67, 73.47, -226.29) and 3.3 m from Airman Lance. ASSUMPTION: the hub placement is "unchecked in the client" per debug-hub.md; if the avatar lands inside an NPC or wall, retry once at (-336.0, 73.47, -226.0).
 b. Wait 6000 ms (world load; the map change can take 20-40 s: use `client_wait_for` {"lease_id":"<LEASE_ID>","lua_condition":"getCurrentWorldID() == 12","timeout_ms":60000} ASSUMPTION world id 12 per debug-hub.md).
 c. `client_batch` {"lease_id":"<LEASE_ID>","steps":[{"op":"player_state","fields":["position","world_id"]}]}. Expect world 12 and position near client [-22750, -33700, 7347]. Record raw. Close any dialog (Lua `DialogMod.onDialogDoneClicked(DialogWin, DialogWin)`).
 d. `client_entity_find` {"lease_id":"<LEASE_ID>","name":"Common Materials Components","fields":["matches"]} must return one match, else the teleport failed: BLOCKED.
 If the change of world swallows chat (the teleport is a world change from 58 to 12), repeat the macro once after waiting 17 s.

**B1. Get cash and find the vendor.**
 a. CHAT(`.givecash 500`). ASSUMPTION: `.givecash <amount>` exists for GMs (commands.md lists `.givecash`; the slash form `/gmgivecash <amount>` is the documented alternative - if the `.` form answers "unknown command", type `/gmgivecash 500` the same way). Then `client_inventory` fields ["cash"]; record cash before (from C1) and after.
 b. `client_entity_find` {"lease_id":"<LEASE_ID>","name":"Common Materials Components","fields":["matches"]} - expect one entity near server (-341.67, 73.47, -226.29). Record its `id` and `distance_m`.
 c. `client_move_to` {"lease_id":"<LEASE_ID>","entity_id":<id from b>,"arrival_m":3.5}. Then `client_entity_find` again with the same name; `distance_m` must be 5.0 or less (target 3-4).
 d. `client_world_click` {"lease_id":"<LEASE_ID>","entity_id":<id>,"button":"right","expect":"window","settle_ms":3000}. If it fails: `lab_screenshot`, report the path, retry once.
 e. `client_window_read` {"lease_id":"<LEASE_ID>","kind":"vendor","fields":["state","open","primary"]} - record the store window name and the rows. You need the row for Steel Core (design 5254, 1 naquadah).

**B2. Buy a stack of Steel Cores.** The vendor sells one per purchase (quantity 1, 1 naquadah). Buy 6. Use `client_window_click` {"lease_id":"<LEASE_ID>","list":"<the vendor list name from B1e>","row_text":"Steel Core","double":true} six times (a double-click buys one: ASSUMPTION - if nothing changes, take one `lab_screenshot`, look for a Buy/Accept button in the window nodes via `client_window_read` {"window":"<vendor window>","include_nodes":true} and click it once per unit). After each purchase you need not read; after the sixth: `client_inventory` {"lease_id":"<LEASE_ID>","containers":["Main","Crafting"],"include_empty":false}. Record the item rows. PASS for the setup: a stack (quantity > 1) or several Steel Core rows exist. Cash must have fallen by about 6.
Then close the store window: Lua `VendorWin:setVisible(false)` ASSUMPTION about the name - use the name from B1e; if it does not close, `client_window_click` target `<window>__auto_closebutton__`.
(ASSUMPTION: Steel Cores land in the crafting bag, container 15, per debug-hub.md. If they landed in Main, that is fine; use whichever container `client_inventory` shows.)

**B3. Step 3 - drag an item into the vault, close, reopen.**
 a. CHAT(`.bank`). Wait 2500 ms. Read `client_inventory` {"lease_id":"<LEASE_ID>","containers":["Vault"],"include_empty":true,"snapshot":"v0"} - note the first vault slot number the tool prints (the numbering base: 0 or 1) and call it V1.
 b. Open the bag with key B if InventoryWin is not visible: `client_ui_sequence` {"lease_id":"<LEASE_ID>","actions":[{"do":"key","key":"b"},{"do":"wait","ms":1000}]} (check Lua `return InventoryWin:isVisible()` first so you do not toggle it closed). If the item is in the crafting bag you may need its tab; find the tab names with `client_window_read` {"window":"InventoryWin","include_nodes":true,"max_nodes":120} (ASSUMPTION: tabs are windows named like Inventory_*Tab) and `client_window_click` the Crafting tab once.
 c. `client_inventory` {"lease_id":"<LEASE_ID>","containers":["Crafting","Main"],"snapshot":"b0"} - choose a Steel Core stack S: container name C_S and slot number N_S as printed.
 d. `client_drag_drop` {"lease_id":"<LEASE_ID>","from":{"container":"<C_S>","slot":<N_S>},"to":{"container":"Vault","slot":<V1>},"wait_ms":4000}. Record `moved`, `snap_back`, `drag_started`, `drop_notified`, `effect_ok`, `native_level`.
 e. Close and reopen: Lua `VaultWin:setVisible(false)`; then CHAT(`.bank`); then `client_inventory` {"containers":["Vault"],"diff_against":"v0"}.
PASS: the drag reports moved true; after reopening, the vault shows the item in slot V1.
Screenshot: `lab_screenshot` once after e (name "step3-after-reopen").

**B4. Step 4 - drag it back.**
`client_drag_drop` from Vault V1 to the first EMPTY slot of the container it came from (read `client_inventory` {"containers":["Crafting","Main"],"include_empty":true} to find an empty slot number and its container). Then `client_inventory` {"containers":["Vault","Crafting","Main"],"diff_against":"b0"}.
PASS: the item is back in the carried bag, Vault V1 is empty, quantities unchanged versus snapshot b0.

**B5. Step 5 - split a stack into the vault, then merge it back.** Needs a stack of at least 2. If the Steel Core rows from B2 are separate single items, first merge them by dragging one onto another (if that works it also covers step 14 for the crafting bag; record it) and use the resulting stack.
 a. `client_inventory` {"containers":["Crafting","Main","Vault"],"snapshot":"s0"}; record the stack quantity Q (expect Q >= 2).
 b. `client_drag_drop` {"from":{"container":"<C>","slot":<N>},"to":{"container":"Vault","slot":<V1>},"split":true,"wait_ms":4000}. Expect exactly one item in the vault slot V1 and Q-1 left in the source.
 c. Record `client_inventory` {"containers":["Crafting","Main","Vault"],"diff_against":"s0"}: the per-item quantity delta must sum to zero across containers.
 d. Merge back: `client_drag_drop` from Vault V1 to the source stack slot ({"container":"<C>","slot":<N>}).
 e. `client_inventory` {"diff_against":"s0"}: totals identical to s0, Vault V1 empty.
PASS: after b, total across Vault+bag equals Q; after d, source stack is back at Q and the vault is empty.

**B6. Step 14 - backpack merge (Main bag).** Needs two same-type stacks in the carried bags. Use Steel Cores if they landed as two stacks; otherwise skip B6 as BLOCKED: "no second stack available". Try: split one core off with `split:true` into an empty slot of the same container (`to` = an empty slot number), then `client_drag_drop` that single core back onto the original stack. Compare totals via snapshot diff before and after. PASS: totals equal, one stack remains. Note: the guide says Main bag; if Steel Cores live in the crafting bag, record that the merge ran in the crafting bag (ASSUMPTION: still counts, BV-03 turned the merge on for every container).

**B7. Leave the vault empty.** `client_inventory` {"containers":["Vault"]} must show no items. If any item remains in the vault, drag it back and record. Optionally run CHAT(`.bankdump Bankone`) and record the lines (expect an empty vault and `bank_slots` 40).
Then `lab_screenshot` once ("c2-end").

## Stop conditions

- Real block: the vendor window never opens after two attempts; no purchase possible (cash not granted: record `.givecash` and `/gmgivecash` answers); `client_drag_drop` returns `effect_ok:false` twice for the same drag (record the full verbose result of one attempt: add "verbose": true on the retry); unexpected window or dialog (read one `client_ui_state`, report, stop).
- Max 2 attempts per step, then report and move on.
- Do not restart the client. Do not release the lease.

## Report format

| step_id | action done | raw observation | PASS/FAIL/BLOCKED claim | screenshot path | UTC time |

Rows: B1, B2, 3, 4, 5, 14 (and B7 as a note). Include in each drag row: moved, snap_back, drop_target_resolved, native_level.

---

## Orchestrator only: verify list

- Step 3: SigNoz `scope_name = 'bank' AND event = 'move_accepted' AND kind = 'deposit' AND target_container_id = 17` with `item_id`, `target_slot_id` < 40 (span `bank.move_item`); the reopen is a second `vault_session_opened`.
- Step 4: `event = 'move_accepted' AND kind = 'withdraw' AND source_container_id = 17` with the same `item_id`.
- Step 5: `move_accepted` with `kind = 'split'` then `kind = 'merge'`; on each row `source_stack_before + target_stack_before = source_stack_after + target_stack_after`.
- Step 14: expect NO bank row (design). Unstructured DEBUG "Inventory move persisted" via `server_log_tail` search `Inventory move persisted`.
- Server state: after B3, `CHAT .bankdump Bankone` output (or `server_db_query` on `sgw.sgw_inventory` for container 17; ASSUMPTION column names `container_id`, `slot_id`, `item_id`, `stack_size`; confirm with `server_db_query` `SELECT * FROM sgw.sgw_inventory LIMIT 1` first).
- Cash: `server_entity_get` for Bankone or DB; compare to the driver's cash numbers.
- Any `scope_name = 'bank'` WARN not expected by a step is a finding. A `move_resync_skipped` row means a snap-back did not happen.
