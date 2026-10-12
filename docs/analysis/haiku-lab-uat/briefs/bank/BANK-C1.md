<!-- markdownlint-disable MD029 -- step numbers run on across sections and are cited by number -->

# BANK-C1 - enter world, refused expand (step 12), .bank opens the 40-slot vault (steps 1 and 13 GM half)

Track: Bank and vault, personal bank. Chunk 1 of 4. Order matters: step 12 needs under 100 naquadah, so this chunk runs FIRST on a brand-new character, before any purchase or cash grant.
Instance: `default` (account `lab`). Lease: `<LEASE_ID>` (lent to you; pass it as `lease_id` on every driving tool; never call lease_acquire/status/release).
Server: COLO. Colo rule: touch only your own lab character. Do not target, select or message anyone else. Do not use `.reloadmap`, `.respawnall` or anything world-wide.
Character: a NEW character `Bankone` (ASSUMPTION: a free slot exists and the lab account may create; the spec `docs/guides/uat-specs/bank.toml` uses alignment "sgu", Soldier, male). Every later chunk plays `Bankone`.
Preconditions: none. Postcondition for C2: `Bankone` is in Castle_CellBlock stasis room, vault session closed or open (irrelevant), naquadah still under 100, no dialog open, vault still 40 slots.

## Standing autonomy

Lua reads, GM `.` commands listed here, and the GM teleport are fine. Stop only on a real block. Report raw values.

## Chat macro CHAT(line) - use for every chat/GM line below

1. `client_batch` {"lease_id":"<LEASE_ID>","steps":[{"op":"lua","id":"dlg","chunk":"return DialogWin ~= nil and DialogWin:isVisible()"}]}  - if true, close with Lua `DialogMod.onDialogDoneClicked(DialogWin, DialogWin)` and re-read. (Only needed once per command batch.)
2. `client_ui_sequence` {"lease_id":"<LEASE_ID>","actions":[{"do":"click","window":"Inst1Chat_Input"},{"do":"type","text":"<line>","into":"Inst1Chat_Input"}]}
3. `client_batch` {"lease_id":"<LEASE_ID>","steps":[{"op":"lua","id":"t","chunk":"return Inst1Chat_Input:getText()"}]} - must equal `<line>` exactly. If not, clear the box with Lua `Inst1Chat_Input:setText('')` and redo step 2 (this counts as the one retry).
4. `client_ui_sequence` {"lease_id":"<LEASE_ID>","actions":[{"do":"key","key":"Enter"},{"do":"wait","ms":2000}]}
5. `client_chat_log` {"lease_id":"<LEASE_ID>","cursor":"bank","max":20} - the reply lines are the evidence. Record them verbatim.

## Actions

**A1. Get in the world (1 call).**
`lab_ensure_in_world` {"lease_id":"<LEASE_ID>","character":"Bankone","create":{"alignment":"sgu","archetype":"Soldier","gender":"male","first":"Uat"}}
Expect `in_world: true`, `character` Bankone, `world_id` for Castle_CellBlock (ASSUMPTION: world 12). Record the whole compact result.
If it fails at character creation (slot limit, name taken), that is BLOCKED: report the error text and stop.

**A2. Wait out the new-character hold (1 call).**
`client_batch` {"lease_id":"<LEASE_ID>","steps":[{"op":"wait","ms":17000},{"op":"player_state","fields":["position","world_id"]}]}
Record the position. Expected near the respawner (-334.23, 73.47, -228.03) in server metres.

**A3. Baseline reads (1 call).**
`client_inventory` {"lease_id":"<LEASE_ID>","containers":["Main","Mission","Crafting","Vault"],"fields":["cash","containers"]}
Record `cash`. If cash >= 100: record it, mark step 12 BLOCKED ("character not fresh: cash >= 100"), do NOT spend or give cash, and still do A5 onward.
Also run `lab_screenshot` {"lease_id":"<LEASE_ID>"} once; record the path as the "hub baseline".

**A4. Step 12 - `.bankexpand` with no vault open and under 100 naquadah.**
Careful: the vault must be OPEN for the refusal to be the insufficient-cash one (with no vault open, the answer is a different refusal, "open your vault first"). So:
 a. CHAT(`.bank`) - expect the vault window to open (see A5 reads; do them now): `client_batch` {"lease_id":"<LEASE_ID>","steps":[{"op":"lua","id":"vis","chunk":"return VaultWin:isVisible()"},{"op":"lua","id":"size","chunk":"return getContainerSize(Container.Vault)"}]}
 b. CHAT(`.bankexpand`)
 c. `client_chat_log` result must contain exactly: "You need 100 naquadah to expand your vault. Nothing was charged."
 d. `client_batch` {"lease_id":"<LEASE_ID>","steps":[{"op":"lua","id":"size","chunk":"return getContainerSize(Container.Vault)"}]} - expect 40. `client_inventory` fields ["cash"] - same cash as A3.
PASS: the exact line appears, size is 40 after, cash unchanged.
Record raw: the chat line, size before/after, cash before/after.

**A5. Step 1 and step 13 (GM half) - `.bank` opens a 40-slot vault away from any Banker.**
You are at the respawn spot, about 8.9 m from the Banker (more than the 5 m interact range), so this is also step 13's "GM away from any Banker".
 a. If the vault is still open from A4, close it by CHAT-free means: `client_ui_sequence` {"lease_id":"<LEASE_ID>","actions":[{"do":"key","key":"Escape"}]} then Lua `return VaultWin:isVisible()`. If still true, leave it; skip to b.
 b. If VaultWin is hidden, CHAT(`.bank`); wait 2500 ms.
 c. `client_batch` {"lease_id":"<LEASE_ID>","steps":[{"op":"lua","id":"size","chunk":"return getContainerSize(Container.Vault)"},{"op":"lua","id":"vis","chunk":"return VaultWin:isVisible()"},{"op":"lua","id":"scroll","chunk":"return VaultMod.ValidateScrollbar ~= nil"}]}
 d. `client_window_read` {"lease_id":"<LEASE_ID>","kind":"vault","fields":["items","buttons","text","title"]} - record raw.
 e. `lab_screenshot` {"lease_id":"<LEASE_ID>"} - record the path (name it "vault-open-40").
PASS step 1: size == 40, VaultWin visible, screenshot shows a vault window (rows of 10, 4 rows, no extra empty scroll area).
PASS step 13 (GM half): the same window opened while you are more than 5 m from every Storage Officer (use `client_entity_find` {"lease_id":"<LEASE_ID>","name":"Storage Officer","fields":["matches"]} and record each `distance_m`; all must be greater than 5; if any is 5 or less, report it, still call step 1 PASS).
The non-GM half of step 13 is EXCLUDED (needs a second, non-GM account).

## Stop conditions

- Real block: lab_ensure_in_world fails twice; `.bank` is answered with "needs GM access" (the `lab` account is not a GM: ASSUMPTION it is; record the line, mark steps 1, 12, 13 BLOCKED, stop); an unexpected window or dialog (read one `client_ui_state`, report, stop).
- Max 2 attempts per step (one unchanged retry), then report and continue with the next independent step.
- Do not restart the client.

## Report format (compact table)

| step_id | action done | raw observation | PASS/FAIL/BLOCKED claim | screenshot path | UTC time |

Rows: A1, 12, 1, 13(GM). Add the cash value and the position as extra lines.
Do NOT release the lease (the orchestrator owns it).

---

## Orchestrator only: verify list (the driver must not do these)

- Step 12: `server_log_tail` search `expand_rejected` / `insufficient_cash`; SigNoz `service.name = 'cimmeria-server' AND scope_name = 'bank' AND event = 'expand_rejected' AND reason = 'insufficient_cash'` plus the character's player_id, expect `cash` < `price`, no `event = 'expand'` row.
- Step 12 DB: `server_db_query` `SELECT bank_slots FROM sgw.sgw_player WHERE <name column> = 'Bankone'` (ASSUMPTION: table sgw.sgw_player has bank_slots, per debug-hub.md; the name column is unconfirmed) - expect 40.
- Step 1/13: SigNoz `scope_name = 'bank' AND event = 'vault_session_opened' AND gm_override = true` (no `banker_id`), span `bank.console_open`; also `expand_quote` row `offered=true bank_slots=40 price=100`.
- `server_sessions` / `server_entity_get` for Bankone: confirm world and position match the driver's report.
- Any WARN under `scope_name = 'bank'` in this session that the steps do not expect is a finding.
