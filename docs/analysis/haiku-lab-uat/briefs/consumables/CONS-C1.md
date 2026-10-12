<!-- markdownlint-disable MD029 -- step numbers run on across sections and are cited by number -->

# CONS-C1 - Consumables: setup, grant, first heal (UAT I1, I2)

Track: Consumables (unified-uat.md "## Consumables"). Chunk: CONS-C1 of 5. Instance: `p2` (account lab2, character Labtwo; created fresh if missing). Lease: `<LEASE_ID>` (lent to you: use it on every driving tool, never call lease tools). Budget: about 20 tool calls.

Preconditions: p2 is Labtwo, already in world 58 (SGC_W1), GM, with 3 Health Slappack TC1 in Main slot 2. This chunk leaves: health healed once from 100, 2 slappacks in the bag.

Why SGC_W1: another lab client is working in the Castle_CellBlock stasis room; stay out of it. Do not touch anything except your own character.

## Reusable calls (used by name below)

READ = one `client_batch`:

```json
{"lease_id":"<LEASE_ID>","instance":"p2","steps":[{"id":"w","op":"player_state","fields":["world_id"]},{"id":"s","op":"lua","chunk":"local h=getUnitStat(Unit.Player,Stat.Health);local f=getUnitStat(Unit.Player,Stat.Focus);local o={};for _,c in ipairs({'Main','Mission'}) do local cid=Container[c];if cid then for s=0,getContainerSize(cid)-1 do local n=getNameForSlot(cid,s);if n~=nil and n~='' then o[#o+1]=c..s..':'..n..'x'..getQuantityForSlot(cid,s) end end end end;return 'HP='..h.current..'/'..h.max..' FP='..f.current..'/'..f.max..' ITEMS='..table.concat(o,'|')"}]}
```

Record the `s` string raw. (ASSUMPTION: `Container.Main` and `getNameForSlot` etc. work as in docs/guides/uat-specs/consumables.toml; `Container.Mission` is a guess, a nil is skipped.) If `s` is an error, record it and use `client_inventory {"lease_id":"<LEASE_ID>","instance":"p2","fields":["containers","cash"]}` plus `client_player_state {"lease_id":"<LEASE_ID>","instance":"p2","stats":false,"effects":false,"fields":["health","focus","world_id"]}` instead.

CHAT(text) = type a chat line. Close every dialog first. First chat line of the chunk, the safe way (3 calls):

1. `client_ui_sequence {"lease_id":"<LEASE_ID>","instance":"p2","actions":[{"do":"type","text":"<text>","into":"Inst1Chat_Input"}]}`
2. `client_batch {"lease_id":"<LEASE_ID>","instance":"p2","steps":[{"id":"t","op":"lua","chunk":"return Inst1Chat_Input:getText()"}]}` - must equal `<text>` exactly. If not, STOP this step and report (letters typed without focus hit keybinds).
3. `client_ui_sequence {"lease_id":"<LEASE_ID>","instance":"p2","actions":[{"do":"key","key":"Enter"},{"do":"wait","ms":2500}]}`

If the first line worked, later lines may be one call: `actions:[{"do":"type","text":"<text>","into":"Inst1Chat_Input"},{"do":"key","key":"Enter"},{"do":"wait","ms":2500}]` (ASSUMPTION: the `into` click focuses the box). If any line behaves oddly, go back to the 3-call form.

CHATLOG = `client_chat_log {"lease_id":"<LEASE_ID>","instance":"p2","cursor":"cons","max":30}` (returns only lines since its last read).

SETSTAT(cmd, N) = set your own health/focus. The command is `/gmsethealth N 0` or `/gmsetfocus N 0` (cmd = gmsethealth / gmsetfocus; Amount first, then TargetId, 0 = yourself). The client may fill the target from whatever is selected, so a stale selection gives the reply "gmSetHealth: target not found" and changes nothing. Procedure:
  a. First clear your selection: `client_ui_sequence {"lease_id":"<LEASE_ID>","instance":"p2","actions":[{"do":"key","key":"Escape"},{"do":"wait","ms":500}]}` (once per chunk is enough; skip if a dialog is closed already).
  b. CHAT(`/<cmd> N 0`) then CHATLOG. If the reply is "target not found" (or the stat did not change after READ), get your own entity id with `client_player_state {"lease_id":"<LEASE_ID>","instance":"p2","stats":false,"effects":false,"fields":["entity_id","id","name"]}` (record raw; use the numeric id of Labtwo; ASSUMPTION: the field is named entity_id or id) and CHAT(`/<cmd> N <ID>`). The server accepts your own id as well as 0.
  c. If both fail, record both raw replies and BLOCK that step (do not guess further).
Below, `CHAT(/gmsethealth N 0)` and `CHAT(/gmsetfocus N 0)` always mean SETSTAT.

## Actions

Starting state (already done by someone else): Labtwo (p2) is in the world in SGC_W1 (world 58), a GM, with 3 Health Slappack TC1 in Main slot 2 (item 2893, stack 3). Do NOT run /gmgiveitem again.

1. READ -> baseline. Record HPMAX, FPMAX and the slappack stack (expect 3). If world_id is not 58 or the stack is not 3, record it and continue only if a stack of at least 1 exists.
2. Clear selection and close dialogs: `client_ui_sequence {"lease_id":"<LEASE_ID>","instance":"p2","actions":[{"do":"key","key":"Escape"},{"do":"wait","ms":500}]}`. CHATLOG once (discard: advances the cursor).

### Step I1 - drop health to 100

3. SETSTAT(gmsethealth, 100) (first chat line of the chunk: use the 3-call safe CHAT form). Then `client_batch [{"op":"wait","ms":2000}]`.
4. READ. `lab_screenshot` -> S1 (health bar visible). CHATLOG (record all lines).

- Expect (PASS): HP current = 100/HPMAX and the slappack stack is unchanged (3). Record the raw `s` string.
- FAIL: HP not 100 after one more READ following 2 s, with SETSTAT's fallback also tried. (Known earlier failure: reply "gmSetHealth: target not found" with `/gmsethealth 100 0`; that is why SETSTAT retries with your own id.)

### Step I2 - double-click a slappack while hurt

5. `client_item_action {"lease_id":"<LEASE_ID>","instance":"p2","action":"doubleclick","name":"Health Slappack","wait_ms":4000}`. Record `native_level`, the `diff` and any error. If it errors with "could not be put on screen"/no slot, retry once with `"action":"use"`. Then `client_batch [{"op":"wait","ms":1500}]`, READ, CHATLOG, `lab_screenshot` -> S2.

- Expect (PASS): stack 3 -> 2 (exactly one taken) and HP current = min(100+500, HPMAX) (600 if HPMAX >= 600, else HPMAX). Record exact before/after numbers.
- FAIL: stack still 3 (no use), stack 1 or 0 (double consumption), or HP unchanged. Record raw. If HP was NOT 100 when you used it (I1 failed), the use is refused at full health with the chat line "You are already at full health." and nothing is consumed: report that as BLOCKED (I1), not an I2 fail.

## Stop conditions

Another lease holder/unknown window/dialog (read once with `client_ui_state`, report, stop); character not Labtwo; HP max read as 0 or nil. Max 2 tries per step, then stop and report. Do not log out; leave p2 in the world.

## Report (one row per step)

| step_id | action done | raw observation | PASS/FAIL/BLOCKED claim | screenshot path | UTC time |

Rows: baseline (steps 1-2), I1, I2. Include HPMAX and the char name/world ids.

---

## VERIFY (orchestrator only, not for the driver)

Player id: from `server_sessions` or `sgw_player` for Labtwo (ASSUMPTION: column names; use `server_db_query` `SELECT * FROM sgw_player WHERE ...`), then:

- I1 grant: `server_log_tail` search `2893` / `gmgiveitem`; DB: `SELECT type_id, stack_size, container_id, slot_id FROM sgw_inventory WHERE character_id = <ID> AND type_id = 2893;` expect one row stack_size 3 after I1, 2 after I2. Item facts: `SELECT item_id, name, max_stack_size FROM resources.items WHERE item_id = 2893;` (Health Slappack TC1, stack 10).
- Health set: `server_entity_get` for the player: health cur 100 then 600/HPMAX.
- I2 SigNoz (service.name = 'cimmeria-server'): `event = 'consumable_used' AND player_id = <ID>` expect 1 row, `item_type_id = 2893`, `health_before` ~100, `health_after` ~600, `heals` includes 500; `event IN ('consumable_consume_requested')` (debug level, may be absent). Also `event IN ('consumable_refused','consumable_skipped','consumable_apply_skipped') AND player_id = <ID>` must be 0 rows.
- Server log strings: `consumable_used`, `consume`.
- Console/teleport: `Moving entity .* to SGC_W1`.
