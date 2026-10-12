<!-- markdownlint-disable MD029 -- step numbers run on across sections and are cited by number -->

# CONS-C4 - Consumables: stimpack buffs (UAT I6, I7, I8)

Track: Consumables. Chunk: CONS-C4 of 5. Instance: `p2` (Labtwo, GM). Lease: `<LEASE_ID>` (lent: use it on every driving tool, never call lease tools). Budget: about 26 tool calls.

Preconditions: p2 in the world as Labtwo (GM), alive, no stimpack buff active (a buff is lost on logout, zone change, gate trip; if the first ATTR read shows raised attributes or `effects` lists buffs from earlier, just record that and use the values as the new baseline). Stay in one zone for the whole chunk (SGC_W1): a zone change drops the buffs. Leaves: Coordination, Engagement and Fortitude buffs active (lost when CONS-C5 changes zone, which is fine).

## Reusable calls

ATTR = one `client_batch` (reads the six attributes, health and the bag):

```json
{"lease_id":"<LEASE_ID>","instance":"p2","steps":[{"id":"a","op":"lua","chunk":"local o={};for _,n in ipairs({'Coordination','Engagement','Fortitude','Intellect','Morale','Perception'}) do local sid=Stat[n];if sid then local v=getUnitStat(Unit.Player,sid);o[#o+1]=n..'='..tostring(v.current)..'/'..tostring(v.max) else o[#o+1]=n..'=NOSTAT' end end;local h=getUnitStat(Unit.Player,Stat.Health);local b={};local cid=Container.Main;for s=0,getContainerSize(cid)-1 do local nm=getNameForSlot(cid,s);if nm~=nil and nm~='' then b[#b+1]=nm..'x'..getQuantityForSlot(cid,s) end end;return table.concat(o,' ')..' HP='..h.current..'/'..h.max..' BAG='..table.concat(b,'|')"}]}
```

(ASSUMPTION: the stat names are `Stat.Coordination` etc.; if any reads NOSTAT, run once `client_player_state {"lease_id":"<LEASE_ID>","instance":"p2","effects":false,"fields":["stats"]}` and read the attribute names from it, then report the real names and use those in the Lua `ipairs` list for the rest of the chunk.) Record the string raw. Note which of `current` / `max` moves when a buff lands (record both).

EFFECTS = `client_player_state {"lease_id":"<LEASE_ID>","instance":"p2","stats":false,"fields":["effects"]}`. Record raw: each effect's name/icon/duration (the buff should show about 3600 s / 60 minutes remaining).

CHAT(text): close dialogs first. Safe form (3 calls): (1) `client_ui_sequence {"lease_id":"<LEASE_ID>","instance":"p2","actions":[{"do":"type","text":"<text>","into":"Inst1Chat_Input"}]}`; (2) `client_batch {"lease_id":"<LEASE_ID>","instance":"p2","steps":[{"id":"t","op":"lua","chunk":"return Inst1Chat_Input:getText()"}]}` must equal `<text>` exactly, else stop; (3) `client_ui_sequence {...,"actions":[{"do":"key","key":"Enter"},{"do":"wait","ms":2500}]}`. Safe form for the first line of the chunk; later lines one call `actions:[{"do":"type","text":"<text>","into":"Inst1Chat_Input"},{"do":"key","key":"Enter"},{"do":"wait","ms":2500}]` if the first worked (ASSUMPTION).

USE(name) = `client_item_action {"lease_id":"<LEASE_ID>","instance":"p2","action":"use","name":"<name>","wait_ms":4000}`; record `native_level`, `diff`, errors; on "cannot put on screen" retry once with `"action":"doubleclick"`. Use the FULL item names exactly as written below (substring match, first hit; "Stimpack: Coordination" would match two items).

CHATLOG = `client_chat_log {"lease_id":"<LEASE_ID>","instance":"p2","cursor":"cons","max":30}`.

## Actions

1. ATTR -> baseline B0 (record the six attributes and HP). EFFECTS -> E0. CHATLOG once, discard. Optionally open the character window (the key the client binds; `client_ui_state` shows windows) only if a screenshot of attributes is wanted; do not spend more than 2 calls on it.

### Step I6 - Mark III Stimpack: Coordination (item 6677)

2. CHAT(`/gmgiveitem 6677 1`), wait 1500 ms. Then USE(`Mark III Stimpack: Coordination`). Wait 2000 ms.
3. ATTR -> A1, EFFECTS -> E1, CHATLOG, `lab_screenshot {"lease_id":"<LEASE_ID>","instance":"p2"}` -> S6 (never `image:true`).

- Expect (PASS): Coordination rose by exactly 5 versus B0; one new buff effect with about 60 minutes (3600 s) on it; the stimpack gone from BAG; HP unchanged from B0 (if HP was full, unchanged). Record which icon name the effect shows (unverified in the client; the icon itself is not a pass criterion).

### Step I7 - Mark V Stimpack: Coordination/Engagement (item 6697) replaces, not adds

4. CHAT(`/gmgiveitem 6697 1`), wait 1500 ms. USE(`Mark V Stimpack: Coordination/Engagement`). Wait 2000 ms.
5. ATTR -> A2, EFFECTS -> E2, `lab_screenshot` -> S7.

- Expect (PASS): Coordination is B0 + 7 (NOT B0 + 12); Engagement is B0 + 3; the Mark III effect/icon is replaced by the Mark V one (E2 does not list two Coordination buffs); item gone.
- FAIL: Coordination at B0 + 12 (stacked), or Engagement unchanged.

### Step I8 - Mark III Stimpack: Fortitude (item 6679) adds alongside

6. CHAT(`/gmgiveitem 6679 1`), wait 1500 ms. USE(`Mark III Stimpack: Fortitude`). Wait 2000 ms.
7. ATTR -> A3, EFFECTS -> E3, `lab_screenshot` -> S8.

- Expect (PASS): Fortitude = B0 + 5; Coordination still B0 + 7 and Engagement still B0 + 3; E3 lists the Coordination/Engagement buffs and the Fortitude buff (3 attribute buffs in total, or as many as the client shows); item gone.
- FAIL: either earlier buff missing.

## Stop conditions

Unknown window/dialog (read once with `client_ui_state`, report, stop); an item not appearing after two grants; attribute names unresolvable (report the stats dump and stop); the character dies or changes zone (report, stop). Max 2 tries per step then stop and report. Do not log out.

## Report

| step_id | action done | raw observation | PASS/FAIL/BLOCKED claim | screenshot path | UTC time |

Rows: baseline, I6, I7, I8. Put B0/A1/A2/A3 attribute strings and the effects lists in the raw column.

---

## VERIFY (orchestrator only)

Player id as in CONS-C1. Item rows: `SELECT item_id, name, description FROM resources.items WHERE item_id IN (6677, 6697, 6679);` (6677 Coordination +5; 6697 Coordination +7 / Engagement +3; 6679 Fortitude +5; each max stack 1).

- SigNoz (service.name = 'cimmeria-server'): `event = 'consumable_used' AND player_id = <ID>` -> 3 rows in order, item_type_id 6677, 6697, 6679, each `stat_buff` true, `active_stat_buffs` 1 -> 2(Coordination replaced by the 6697 pair: expect 2 entries: Coordination+Engagement) -> 3.
- Buff ledger: `scope_name = 'abilities' AND event IN ('stat_buff_applied','stat_buff_removed') AND player_id = <ID>`: applied for Coordination (+5), then `stat_buff_removed reason = 'replaced'` for the Coordination +5 and applied Coordination +7 and Engagement +3; applied Fortitude +5; no `expired`.
- Live state: `server_entity_get` for the player: attribute stats and `stat_buffs` entries.
- DB: no 6677/6697/6679 rows left: `SELECT type_id, stack_size FROM sgw_inventory WHERE character_id = <ID> AND type_id IN (6677,6697,6679);` empty. `event IN ('consumable_refused','consumable_apply_skipped') AND player_id = <ID>` should be 0 rows.
