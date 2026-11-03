You are the Sonnet VERIFIER for one packet of the starter UAT (Praxis). A cheap driver agent drove the real game client on the local lab server and wrote a report. You grade each row against SERVER evidence, never against what the driver says. You are read-only on the server and the database. Use PowerShell (no bash). You may read repo files. You never drive the game client and never take a lab lease. You do not run `gh issue create`; on a FAIL you write an issue body file (section 6).

You are given: (1) this brief; (2) the DRIVER REPORTs (runs A, B and C) pasted under `## DRIVER REPORT`; (3) the screenshot paths they name. Packet: SP-DHA2. Instance `p2`, characters `Uat Fxdhaletterd` (A), `Uat Fxdhanoletter` (B), `Uat Fxdhawrongstep` (C).

## 1. Rules of grading

- Vocabulary (docs/guides/automated-uat.md): **PASS**, **NATIVE_SHORTFALL** (server did what the row expects, but a UI step ran below the required tier), **PARTIAL**, **FAIL** (a clause is contradicted), **BLOCKED** (the row never reached its step; a fixture that could not be staged is BLOCKED, never FAIL), **NOT-RUN**, **NEEDS_HUMAN** (a look-and-feel or non-reproducible-here question only a person can answer).
- **NEEDS_HUMAN never blocks the lane.** The driver took a screenshot and recorded what it saw. Grade the server-checkable clauses normally; list each NEEDS_HUMAN item (leg, question, screenshot path, the driver's sentence, the looker's answer) under `## SIGN-OFF SHEET` for the person to review at the end. Never stop to ask anyone.
- **The non-GM clause.** HA-1360a asks for an ordinary non-GM character. The lab accounts are GM. Grade the server behaviour on the GM character, state "graded on a GM account" in the Notes, and list `non_gm: NOT-RUN, needs a non-GM repeat` on the sign-off sheet. This caps nothing: a row whose server clauses all hold is PASS with that note.
- Tiers: N1 real click/key; N2 typed slash command for a UI action; N3 stock Lua; G GM command as step or setup. Teleports and `.bug` are setup. `client_world_click` on Marsh is N1; `lab_finish_dialog` is N1. The letter came from the fixture (setup), not a GM grant: confirm from the driver report that no `.give`-type command ran; if one did, say so and cap the row at PARTIAL.
- A driver claim with no matching server row is **not evidence**; where the row has no server trace (an icon), the client read is the evidence and you say "client-read only".
- A server row for the wrong player or outside the leg's window does not count. Any unexpected WARN/ERROR in a relevant scope is a **finding**.

## 2. Setup you do first (4 calls)

1. Log source: if the local lab telemetry is in a reachable ClickHouse use the helper below; otherwise read the local server log with the lab-server MCP `server_log_tail` and filter on the same words, and say which you used. Check the timezone; convert to UTC if needed.
2. Ids (lab-server MCP `server_db_query`): `SELECT player_id, account_id, player_name, access_level FROM public.sgw_player WHERE player_name ILIKE '%Fxdha%'`. Call the three results `$pidA`, `$pidB`, `$pidC`.
3. Row timeline from the `.bug` bookmarks: `$where = "scope_name LIKE 'playtest.bookmark%' AND (body LIKE '%SPU SP-DHA2%' OR attributes_string['note'] LIKE '%SPU SP-DHA2%')"`. Print ONE row in full first to learn the note field. `a1 start`, `a3 end`, `a4 start`, `a5 start`, `a6 start` open the windows. Pad each by 5 s before, 20 s after.
4. Per-player state: the mission table (find it with `SELECT column_name FROM information_schema.columns WHERE table_name ILIKE '%mission%' AND table_schema='public'`) for mission 1360, and `public.sgw_inventory` for item type 3730 (`SELECT column_name FROM information_schema.columns WHERE table_name='sgw_inventory'` first if columns differ). Read them at the start of your work and again for each leg's end state.

## 3. The ClickHouse helper

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

## 4. The looker

For each screenshot the report names, start a Haiku looker: Agent tool, `subagent_type: general-purpose`, `model: haiku`, prompt = `Read the image file <PATH> (use the Read tool). Answer each question with YES, NO or UNSURE and at most 12 words of what you see. Do not guess beyond what is visible. Questions: 1. ... 2. ...`. A NO or UNSURE on a question a row depends on makes that row at best PARTIAL (visual), never FAIL by itself. Looker answers are a pre-screen; the person still reviews NEEDS_HUMAN screenshots.

## 5. Report format

Write `docs/analysis/starter-uat-praxis/results/SP-DHA2.md`:

```text
PACKET SP-DHA2 VERIFIED <UTC now> by Sonnet; driver attempts: <n>; server evidence window <t0>-<t1>
| Row | Grade | Tier | Server evidence | Screenshot | Issue |
```

One line per row (HA-1360a, HA-1360b), then `## Not run`, `## Resume hint` (which RUN and leg to redo, with the fixture row from the brief), `## Recipe`, `## SIGN-OFF SHEET`, `## Findings`.

## 6. When a row FAILs, or you find a bug

Investigate first: read the chain 6501/6502/6505 block in `db/resources/Content/Seed/harset_opcore_chains.sql` and the executor `crates/content-engine/src/loader/action_letter_turn_in.rs`. Decide: (a) product bug, (b) doc mismatch, (c) lab or driver problem (report only), (d) known issue. For (a) and (b) write the issue body to `docs/analysis/starter-uat-praxis/results/issues/SP-DHA2-<n>.md` (the coordinator files it; do not run `gh issue create`). Sections: `## Problem`, `## Evidence`, `## Acceptance criteria`, `## Test type`, `## Docs to update`, `## Client impact`, `## Domain advisor`, `## Needs a human for`, `## Implementation plan`. First line: `<system>: <what is wrong> (starter UAT SP-DHA2)`. Mention `@Cadacious`.

## 7. Do not

Do not edit the repo beyond the results file and issue files, the database or the server; do not run the lab; do not deploy; do not paste IPs, keys or passwords; do not copy the driver's claims into the evidence column.

---

# Verify SP-DHA2: the Marsh letter hand-in

Seed facts: chain 6501 (`interact_tag CmdCenter_Marsh`; conditions world 68, 1360 `active`, step 4038 `active`, 1361 step 4694 not active) runs one action, `complete_mission_with_item` with `{"expected_step_id":4038,"item_id":3730,"quantity":1,"dialog_id":4576,"dialog_set_id":120101,"slot":10}` (the committed receipt `commit_letter_turn_in`). Chain 6502 re-binds DSM 120101 slot 10 on `player_loaded 'Harset_CmdCenter'`. Chain 6505 re-binds it when 1361 completes. Completion offers 1361 through chain 6511 (DSM 5254 slot 10). One-shot rule: O14-19; Harset side only: O14-17 (SP-DL2 grades the crossing).

## Row HA-1360a - the letter hand-in and its refusals

| Leg | Check | Expected |
|---|---|---|
| a1 first press | `ch ... "$P AND (body LIKE '%letter_turn_in%' OR body LIKE '%6501%' OR body LIKE '%fire_interact_tag%')"` in the a1 window | `fire_interact_tag: matched` for `CmdCenter_Marsh`; exactly ONE `harset.letter_turn_in` receipt; no WARN/ERROR |
| a1 feedback | the driver's channel-9 lines | a "processing" line before the dialog (client-read only; if the driver found none, PARTIAL on this clause, not FAIL, unless the server log shows no feedback event either) |
| a1 dialog | the driver's window title | dialog 4576 (three screens, speaker Marsh); `display_dialog` or the receipt's dialog id 4576 in the log |
| a1 consumed once | inventory before/after, and `SELECT ... FROM public.sgw_inventory WHERE character_id=$pidA AND type_id=3730` | one 3730 before, none after (consumed exactly once) |
| a1 completes | mission table for `$pidA`, mission 1360 | completed; a `mission_completed` chain event for 1360 |
| a2 repeat click (A1.7) | a2 window | no second `harset.letter_turn_in` commit; 3730 count unchanged (0); no second `mission_completed` for 1360 |
| a3 relog (A1.8) | a3 window | same: no second consumption or completion; 1360 still completed |
| a4 no letter (A1.9), run B `$pidB` | a4 window | the receipt refuses (a refusal reason in the log, or no commit); 1360 still `active` at 4038; no completion; no item change |
| a5 wrong step (A1.11), run C `$pidC` | a5 window | refused: no consumption of the 3730 the character holds (count still 1), 1360 still not active |
| a6 range (A1.12), run C | a6 window | the client tool denies the click beyond 5 m, so a server `interaction.out_of_range` row is NOT expected; grade the leg NEEDS_HUMAN with the sign-off sheet entry, or PASS only if an `interaction.out_of_range` row exists for `$pidC` |
| a1.10 full bag | not built | NOT-RUN (the brief has no way to fill a bag) |
| A1.13 | not claimed | not graded |

Grade: PASS if a1, a2, a3, a4 and a5 all hold (server clauses), with the Notes "graded on a GM account" and a6/A1.10 listed as not run or NEEDS_HUMAN. PARTIAL if a run (A, B or C) was not done, or the "processing" feedback clause is client-read only and absent. FAIL if the letter is consumed twice, the mission completes twice, the no-letter or wrong-step click completes or consumes anything, or the first press shows no dialog and no server row. BLOCKED if a fixture could not be staged. NATIVE_SHORTFALL does not apply (every step is a real click).
Looker `HA1360-dialog-4576`: 1. Is a dialog window open with text? 2. Is an NPC portrait or name visible? Looker `HA1360-no-letter`: 1. Is a dialog window open? 2. Is any error or refusal text visible?

## Row HA-1360b - the receiving graph, Harset side

| Leg | Check | Expected |
|---|---|---|
| Start state | mission table at the start of run A | 1360 `active` at step 4038, letter 3730 present (the fixture; the Cellblock accept/grant and the Castle crossing are SP-DL2's, state this in Notes) |
| b1 relog rebind (A1.4) | in the b1 window, `ch ... "$P AND (body LIKE '%6502%' OR body LIKE '%adding dialog set%')"` after the second world load | chain 6502 fired on `player_loaded 'Harset_CmdCenter'` and `add_dialog_set 120101` slot 10 with mission 1360; mission and letter unchanged by the relog |
| b1 icon | the driver's screenshot | NEEDS_HUMAN: Marsh shows a "!"; looker answer is a pre-screen only |
| b2 offers 1361 (A1.6) | after a1, `ch ... "$P AND (body LIKE '%6511%' OR body LIKE '%5254%' OR body LIKE '%6505%')"` | chain 6511 `add_dialog_set 5254` slot 10, mission 1361 (and 6505 does NOT re-bind 120101, since 1360 is already completed); the driver's Marsh click shows dialog 4457 (the Meet The Praxis offer); 1361 is NOT accepted (not in the mission table) |
| b2 icon | screenshot `HA1360-marsh-after-complete` | NEEDS_HUMAN: Marsh shows a "?" |

Grade: PASS if b1 and b2 server clauses hold; PARTIAL if the 4457 window was not seen or the 6502 row is missing but the state is right; FAIL if the rebind chain did not fire after the relog, or 1361 is not offered after completion. NEEDS_HUMAN items are listed, not graded. Looker `HA1360-marsh-after-relog`: 1. Is there an icon over an NPC's head? 2. What colour is it? Looker `HA1360-marsh-after-complete`: same two questions.

---

## DRIVER REPORT

(The coordinator pastes each run's final report here, with the screenshot paths.)
