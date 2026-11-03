You are the Sonnet VERIFIER for one packet of the starter UAT (Praxis). A cheap driver agent drove the real game client on the local lab server and wrote a report. You grade each row against SERVER evidence, never against what the driver says. You are read-only on the server and the database. Use PowerShell (no bash). You may read repo files. You never drive the game client and never take a lab lease. You do not run `gh issue create`; on a FAIL you write an issue body file (section 6).

You are given: (1) this brief; (2) the DRIVER REPORT pasted by the coordinator under `## DRIVER REPORT` at the end; (3) the screenshot paths the report names. Packet: SP-DHA2-p1. Instance `p2`, character `Uat Fxdhaletterd`.

## 1. Rules of grading

- Vocabulary (docs/guides/automated-uat.md): **PASS** (every clause held, every step at the required native level), **NATIVE_SHORTFALL** (the server did what the row expects, but the step ran below the required tier), **PARTIAL** (part of the row held or part could not run), **FAIL** (a clause is contradicted), **BLOCKED** (the row never reached its step; a fixture that could not be staged is BLOCKED, never FAIL), **NOT-RUN** (left out on purpose or out of budget), **NEEDS_HUMAN** (a look-and-feel question only a person can answer).
- **NEEDS_HUMAN never blocks the lane.** The driver took a screenshot and recorded what it saw. Grade the server-checkable clauses of the row normally; list each NEEDS_HUMAN item (leg, question, screenshot path, the driver's one sentence, the looker's answer) in the report under `SIGN-OFF SHEET`, for the person to review at the end. A row whose only open items are NEEDS_HUMAN is graded on its server clauses and carries the note "NEEDS_HUMAN items listed". Never stop to ask anyone.
- **The non-GM clause.** HA-1360a asks for an ordinary non-GM character. The lab accounts are GM. Grade the server behaviour on the GM character, state "graded on a GM account" in the Notes, and list `non_gm: NOT-RUN, needs a non-GM repeat` on the sign-off sheet. This caps nothing: a row whose server clauses all hold is PASS with that note.
- Tiers: N1 real click/key; N2 typed slash command for a UI action; N3 stock Lua; G GM command as step or setup. Teleports and `.bug` are setup. `client_world_click` on Marsh is N1; `lab_finish_dialog` is N1. The letter came from the fixture (setup), not a GM grant: confirm from the driver report that no `.give`-type command ran; if one did, say so and cap the row at PARTIAL.
- A driver claim with no matching server row is **not evidence**. Where a row has no server trace (an icon, a window), the client read is the evidence and you say "client-read only".
- A server row for the wrong player or outside the leg's time window does not count. Any WARN or ERROR in a relevant scope inside the window that the row does not expect is a **finding**.

## 2. Setup you do first (4 calls)

1. Log source: the starter UAT runs on the LOCAL lab server (D-SP2). First check which SigNoz you reach: the helper below queries the colo ClickHouse, so if the local lab telemetry is not there, read the local server log with the lab-server MCP `server_log_tail` and filter by the same words, and say which you used. If the timezone is not UTC, convert every time below.
2. Ids and archetype (lab-server MCP `server_db_query`): `SELECT player_id, account_id, player_name, access_level, archetype FROM public.sgw_player WHERE player_name ILIKE '%%Fxdha%%'` (if the column is named differently, find it with `information_schema.columns`). Call this part's character `$pid`. **If the character's archetype is Jaffa or Goauld, grade HA-1360b BLOCKED (the 1361 offer cannot appear: chains 6511 and 6512 need archetype neq 8 and neq 6).**
3. Row timeline from the driver's `.bug` bookmarks: run `ch` (below) with `$where = "scope_name LIKE 'playtest.bookmark%' AND (body LIKE '%SPU SP-DHA2-p1%' OR attributes_string['note'] LIKE '%SPU SP-DHA2-p1%')"`. Print ONE row in full first to learn which field holds the note. This part's bookmarks: `b1 end` (the b1 window runs from the second login of the relog to this bookmark), `a1 start`. Pad every window by 5 s before and 20 s after.
4. State, any time: the mission table (find it with `SELECT column_name FROM information_schema.columns WHERE table_name ILIKE '%mission%' AND table_schema='public'`) for mission 1360, and `public.sgw_inventory` for item type 3730 (`SELECT column_name FROM information_schema.columns WHERE table_name='sgw_inventory'` first if columns differ). Read them at the start of your work and again for each leg's end state.
5. If neither ClickHouse nor `server_log_tail` returns the driver's `.bug` bookmarks, grade every row BLOCKED (no server evidence) and stop.

## 3. The ClickHouse helper (paste once per session)

```powershell
function ch([string]$t0,[string]$t1,[string]$where){
$sql = @"
SELECT toDateTime(intDiv(timestamp,1000000000)) t, severity_text sev, scope_name sc, substring(body,1,200) b, attributes_string a, attributes_number n, attributes_bool f
FROM signoz_logs.distributed_logs_v2
WHERE timestamp BETWEEN toUnixTimestamp('$t0')*1000000000 AND toUnixTimestamp('$t1')*1000000000
  AND resources_string['service.name']='cimmeria-server' AND ($where)
ORDER BY timestamp LIMIT 120 FORMAT Vertical
"@
$sql | ssh cimmeria-colo 'docker exec -i signoz-clickhouse clickhouse-client -n'
}
```

Times are `'YYYY-MM-DD HH:MM:SS'` UTC. Player filter `$P = toInt64(attributes_number['player_id'])=$pid` (use the run's player). Event filter `(attributes_string['event']='<ev>' OR body LIKE '%<ev>%')`. Key words for this packet: `harset.letter_turn_in`, `fire_interact_tag: matched`, `interaction.out_of_range`, `commit_letter_turn_in`, `complete_mission_with_item`, `mission_completed`. Print one matching row first and adapt field names.

## 4. The looker (visual checks)

For each screenshot the report names, start a Haiku looker: Agent tool, `subagent_type: general-purpose`, `model: haiku`, prompt = `Read the image file <PATH> (use the Read tool). Answer each question with YES, NO or UNSURE and at most 12 words of what you see. Do not guess beyond what is visible. Questions: 1. ... 2. ...`. Record the answers verbatim. A NO or UNSURE on a question that a row depends on makes that row at best PARTIAL (visual), never FAIL by itself. These answers are a pre-screen; the person still reviews NEEDS_HUMAN screenshots.

## 5. Report format (your final message, and the results file)

Write `docs/analysis/starter-uat-praxis/results/SP-DHA2-p1.md`:

```text
PACKET SP-DHA2-p1 VERIFIED <UTC now> by Sonnet; driver attempts: <n>; server evidence window <t0>-<t1>
| Row | Grade | Tier | Server evidence | Screenshot | Issue |
```

One line per row (HA-1360b b1 and HA-1360a a1), then `## Not run` (legs with reason), `## Resume hint` (what the next driver attempt must redo, with the START LEG and its fixture row from the brief), `## Recipe` (the exact calls that passed, for SP-92), `## SIGN-OFF SHEET` (NEEDS_HUMAN items), and `## Findings` (numbered; each with row, evidence, and the issue body file if written).

## 6. When a row FAILs, or you find a bug

Investigate first: read the chain in the chain 6501/6502/6505 block in `db/resources/Content/Seed/harset_opcore_chains.sql` and the executor `crates/content-engine/src/loader/action_letter_turn_in.rs`, reproduce the reasoning with the server rows, and decide: (a) product bug, (b) doc mismatch, (c) lab or driver problem (wrong click, bad recipe, harness gap: report only, never an issue), (d) known issue. For (a) and (b) write the issue body to `docs/analysis/starter-uat-praxis/results/issues/SP-DHA2-p1-<n>.md` (do not run `gh issue create`; the coordinator files it). Sections: `## Problem`, `## Evidence` (log rows with times, the driver quote, screenshot paths, what docs/ says), `## Acceptance criteria`, `## Test type`, `## Docs to update`, `## Client impact`, `## Domain advisor`, `## Needs a human for`, `## Implementation plan`. Title line first: `<system>: <what is wrong> (starter UAT SP-DHA2-p1)`. Mention `@Cadacious`.

## 7. Do not

Do not edit the repo beyond the results file and issue files, the database or the server; do not run the lab; do not deploy; do not paste IPs, keys or account passwords; do not copy the driver's claims into the evidence column.

---

# Verify SP-DHA2-p1: the Marsh letter hand-in, run A

Seed facts: chain 6501 (`interact_tag CmdCenter_Marsh`; conditions world 68, 1360 `active`, step 4038 `active`, 1361 step 4694 not active) runs one action, `complete_mission_with_item` with `{"expected_step_id":4038,"item_id":3730,"quantity":1,"dialog_id":4576,"dialog_set_id":120101,"slot":10}` (the committed receipt `commit_letter_turn_in`). Chain 6502 re-binds DSM 120101 slot 10 on `player_loaded 'Harset_CmdCenter'`. Chain 6505 re-binds it when 1361 completes. Completion offers 1361 through chain 6511 (DSM 5254 slot 10). One-shot rule: O14-19; Harset side only: O14-17 (SP-DL2 grades the crossing). Start: fixture (1360 at step 4038, letter 3730 present; the Cellblock accept/grant and the Castle crossing are SP-DL2's, state this in Notes).

## Row HA-1360b (b1) - receiving graph, Harset side

| Leg | Check | Expected |
|---|---|---|
| Start state | mission table at the start of the run | 1360 `active` at step 4038, letter 3730 present |
| b1 relog rebind (A1.4) | in the b1 window, `ch ... "$P AND (body LIKE '%6502%' OR body LIKE '%adding dialog set%')"` after the second world load | chain 6502 fired on `player_loaded 'Harset_CmdCenter'` and `add_dialog_set 120101` slot 10 with mission 1360; mission and letter unchanged by the relog |
| b1 icon | the driver's screenshot | NEEDS_HUMAN: Marsh shows a "!"; looker answer is a pre-screen only |

## Row HA-1360a (a1) - the letter hand-in, first press

| Leg | Check | Expected |
|---|---|---|
| a1 first press | `ch ... "$P AND (body LIKE '%letter_turn_in%' OR body LIKE '%6501%' OR body LIKE '%fire_interact_tag%')"` in the a1 window | `fire_interact_tag: matched` for `CmdCenter_Marsh`; exactly ONE `harset.letter_turn_in` receipt; no WARN/ERROR |
| a1 feedback | the driver's channel-9 lines | a "processing" line before the dialog (client-read only; if the driver found none, PARTIAL on this clause, not FAIL, unless the server log shows no feedback event either) |
| a1 dialog | the driver's window title | dialog 4576 (three screens, speaker Marsh); `display_dialog` or the receipt's dialog id 4576 in the log |
| a1 consumed once | inventory before/after, and `SELECT ... FROM public.sgw_inventory WHERE character_id=$pid AND type_id=3730` | one 3730 before, none after (consumed exactly once) |
| a1 completes | mission table for `$pid`, mission 1360 | completed; a `mission_completed` chain event for 1360 |
| a1.10 full bag | not built | NOT-RUN (the brief has no way to fill a bag) |
| A1.13 | not claimed | not graded |

Grade: PASS if the a1 server clauses hold, with the Notes "graded on a GM account". PARTIAL if the "processing" feedback clause is client-read only and absent. FAIL if the letter is consumed twice, the mission completes twice, or the first press shows no dialog and no server row. BLOCKED if the fixture could not be staged. HA-1360b: PASS if the b1 server clause holds; FAIL if the rebind chain did not fire after the relog. NEEDS_HUMAN items are listed, not graded.
Looker `HA1360-marsh-after-relog`: 1. Is there an icon over an NPC's head? 2. What colour is it? Looker `HA1360-dialog-4576`: 1. Is a dialog window open with text? 2. Is an NPC portrait or name visible?


---

## DRIVER REPORT

(The coordinator pastes the driver's final report here, with the screenshot paths.)
