<!-- markdownlint-disable MD029 -- step numbers run on across sections and are cited by number -->

# BANK-C4 - relog persistence with .bankdump (step 6) and paid expansion 40 to 100 (step 11, GM part)

Track: Bank and vault, personal bank. Chunk 4 of 4.
Instance: `default` (account `lab`). Lease: `<LEASE_ID>` (lent; pass as `lease_id` on every driving tool; never acquire/status/release).
Server: COLO. Touch only `Bankone`. This chunk permanently grows Bankone's vault to 100 slots and spends about 600 naquadah (harmless: lab character only).
Location does not matter in this chunk: every command (`.bank`, `.bankdump`, `.bankexpand`, `.givecash`) works anywhere, so no teleport is needed, wherever Bankone starts (SGC_W1 or the hub) and wherever the relog puts it. Tool note: `client_window_read` rejects fields title/buttons/text (use `state`, `open`, `primary`, or omit); read chat replies ONLY with `client_chat_log`, a Lua read of chat output returns nil.
Preconditions (left by BANK-C3, or by C1 if C2/C3 were not run): `Bankone` online; vault is 40 slots; vault contains zero or one Steel Core; naquadah above 0 (it is topped up here).
Postcondition: end state, vault 100 slots. Nothing else depends on this chunk.

## Standing autonomy

Lua reads, listed GM commands, logout/login through the listed tools. Stop only on a real block.

## CHAT(line) macro

1. Close any dialog (Lua `DialogMod.onDialogDoneClicked(DialogWin, DialogWin)` if `DialogWin:isVisible()`).
2. `client_ui_sequence` {"lease_id":"<LEASE_ID>","actions":[{"do":"click","window":"Inst1Chat_Input"},{"do":"type","text":"<line>","into":"Inst1Chat_Input"}]}
3. `client_batch` Lua `return Inst1Chat_Input:getText()` must equal `<line>`; else `Inst1Chat_Input:setText('')`, retype once. (For the repeated `.bankexpand` lines after the first success you may skip this readback and just Enter.)
4. `client_ui_sequence` {"actions":[{"do":"key","key":"Enter"},{"do":"wait","ms":2000}]}
5. `client_chat_log` {"lease_id":"<LEASE_ID>","cursor":"bank","max":20}

## Actions

**D1. Step 6, before logout: make sure exactly one item is in the vault.**
 a. CHAT(`.bank`), wait 2500 ms. CHAT(`.bankdump Bankone`). Record every chat line (item slot, name, type id, stack size, item id, and the vault size line).
 b. If no item is listed: `client_inventory` {"lease_id":"<LEASE_ID>","containers":["Crafting","Main","Vault"],"include_empty":true} and `client_drag_drop` {"from":{"container":"<C>","slot":<N>},"to":{"container":"Vault","slot":<first empty vault slot>},"split":true,"wait_ms":4000} (open the bag with key b first if InventoryWin is hidden; see C2 for tab notes). Repeat CHAT(`.bankdump Bankone`).
 Record: ITEM_NAME, ITEM_TYPE_ID, VAULT_SLOT, ITEM_ID from the dump line.

**D2. Log out and back in.**
 a. `lab_logout` {"lease_id":"<LEASE_ID>"}. Record the result.
 b. `lab_ensure_in_world` {"lease_id":"<LEASE_ID>","character":"Bankone"} (no create). Record `in_world`, `world_id`. Then `client_batch` {"steps":[{"op":"wait","ms":4000}]}.
 If the client is stuck at character select for over 90 s, take a `lab_screenshot`, report the path, BLOCKED.

**D3. Step 6, after login: the item is still there.**
 a. CHAT(`.bank`), wait 2500 ms.
 b. `client_inventory` {"lease_id":"<LEASE_ID>","containers":["Vault"]} - the same item (name, type id, slot) as in D1.
 c. CHAT(`.bankdump Bankone`) - same line as D1.
 d. `lab_screenshot` ("step6-after-relog").
 PASS: item and slot equal D1's; vault size 40.

**D4. Step 11, refusal when no vault is open.** The logout cleared the vault session, so right after login, BEFORE any `.bank`, the expected refusal is "bankexpand: open your vault first". Because D3 already opened it, redo this by relogging once more, cheaply:
 `lab_logout`, then `lab_ensure_in_world` {"character":"Bankone"}, wait 4000 ms, then CHAT(`.bankexpand`) with NO `.bank`.
 Expected chat: "bankexpand: open your vault first (.bank, or a Banker in range). Nothing was charged."
 Record verbatim; vault size must still be 40 later (read it in D5).

**D5. Step 11, the expansion run (GM).**
 a. CHAT(`.givecash 1000`) (ASSUMPTION: `.givecash` exists; fallback `/gmgivecash 1000`). `client_inventory` {"containers":["Vault"],"fields":["cash"]} -> CASH0 (must be at least 600).
 b. CHAT(`.bank`), wait 2500 ms. Keep the vault window OPEN for the whole run (to check live resize). Read once: `client_batch` {"steps":[{"op":"lua","id":"size","chunk":"return getContainerSize(Container.Vault)"},{"op":"lua","id":"vis","chunk":"return VaultWin:isVisible()"}]}. Expect 40 and true.
 c. Expansion 1: CHAT(`.bankexpand`). Expect chat "Your vault now has 50 slots. You paid 100 naquadah." Read immediately: `client_batch` Lua `return getContainerSize(Container.Vault)` (expect 50 while the window is still open: this is the "live resize" question; record the raw number) and `client_inventory` fields ["cash"] (CASH0 - 100). `lab_screenshot` ("expand-50", look for a larger scroll range). `client_window_read` {"kind":"vault","fields":["state","open","primary"]} record.
 d. Expansions 2 to 6: CHAT(`.bankexpand`) five more times. After each: chat line must read "Your vault now has N slots. You paid 100 naquadah." for N = 60, 70, 80, 90, 100. One `client_batch` size read after the last (expect 100) and one `client_inventory` cash read (CASH0 - 600).
 e. At the ceiling: CHAT(`.bankexpand`) once more. Expect "Your vault is already at its full size of 100 slots." Cash and size unchanged (read both).
 f. `lab_screenshot` ("expand-100").
 PASS: all six lines exact, size 100, cash fell by exactly 100 per step, ceiling refusal exact and nothing charged.
 The non-GM `.bankexpand` refusal is EXCLUDED (needs a non-GM account).

## Stop conditions

- Real block: `.givecash` and `/gmgivecash` both unknown or refused (record answers; then you cannot do 6 expansions: do as many as cash allows and mark the rest BLOCKED); `.bankexpand` answered ".bankexpand needs GM access" (the lab account is not a GM; stop); relog stuck (screenshot, stop); unexpected dialog (one `client_ui_state`, report, stop).
- Max 2 attempts per step. Do not restart the client. Do not release the lease.

## Report format

| step_id | action done | raw observation | PASS/FAIL/BLOCKED claim | screenshot path | UTC time |

Rows: 6, 11a (no vault open), 11b (expansions 1..6, one line each with the chat text, size read and cash), 11c (ceiling).

---

## Orchestrator only: verify list

- Step 6: SigNoz `scope_name = 'bank' AND event = 'vault_session_closed' AND reason = 'logout'` with `open_ms`, then a new `vault_session_opened`; `event = 'gm_action' AND action = 'bankdump' AND result = 'ok'` with `item_count`. Cross-check the item_id with `server_db_query` (ASSUMPTION: `sgw.sgw_inventory` rows for the character in container 17; probe columns with `SELECT * FROM sgw.sgw_inventory LIMIT 1`).
- Step 11 refusals: `event = 'expand_rejected' AND reason = 'no_vault_session' AND trigger = 'gm_console'`; `reason = 'at_ceiling'` with `bank_slots = 100`.
- Step 11 purchases: six rows `event = 'expand' AND trigger = 'gm_console' AND gm_override = true` with `bank_slots_before/after` 40->50 .. 90->100, `price = 100`, `cash_before - cash_after = 100`; spans `bank.console_expand` (cell) and `bank.expand_purchase` (base).
- Final DB state: `SELECT bank_slots FROM sgw.sgw_player WHERE <name col> = 'Bankone'` = 100 (name column ASSUMPTION).
- Live resize: compare the driver's size reads taken with the window open; whether the scroll range grew on screen is a human screenshot check (screenshots "expand-50" and "expand-100"). Per the guide this is a "things only a human can check" item: report the numbers and let the owner judge the pictures.
- `server_log_tail` for WARNs under `bank`; none expected.
