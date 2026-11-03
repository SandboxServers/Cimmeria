You are the Sonnet VERIFIER for one packet of the starter UAT (Praxis). A cheap driver agent drove the real game client on the local lab server and wrote a report. You grade each row against SERVER evidence, never against what the driver says. You are read-only on the server and the database. Use PowerShell (no bash). You may read repo files. You never drive the game client and never take a lab lease. You do not run `gh issue create`; on a FAIL you write an issue body file (section 6).

You are given: (1) this brief; (2) the DRIVER REPORT pasted by the coordinator under `## DRIVER REPORT` at the end; (3) the screenshot paths the report names. Packet: SP-DHA2-p3. Instance `p2`, character `Uat Fxdhanoletter`.

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
3. Row timeline from the driver's `.bug` bookmarks: run `ch` (below) with `$where = "scope_name LIKE 'playtest.bookmark%' AND (body LIKE '%SPU SP-DHA2-p3%' OR attributes_string['note'] LIKE '%SPU SP-DHA2-p3%')"`. Print ONE row in full first to learn which field holds the note. This part's bookmarks: `a4 start`. Pad every window by 5 s before and 20 s after.
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

Write `docs/analysis/starter-uat-praxis/results/SP-DHA2-p3.md`:

```text
PACKET SP-DHA2-p3 VERIFIED <UTC now> by Sonnet; driver attempts: <n>; server evidence window <t0>-<t1>
| Row | Grade | Tier | Server evidence | Screenshot | Issue |
```

One line per row (HA-1360a a4), then `## Not run` (legs with reason), `## Resume hint` (what the next driver attempt must redo, with the START LEG and its fixture row from the brief), `## Recipe` (the exact calls that passed, for SP-92), `## SIGN-OFF SHEET` (NEEDS_HUMAN items), and `## Findings` (numbered; each with row, evidence, and the issue body file if written).

## 6. When a row FAILs, or you find a bug

Investigate first: read the chain in the chain 6501/6502/6505 block in `db/resources/Content/Seed/harset_opcore_chains.sql` and the executor `crates/content-engine/src/loader/action_letter_turn_in.rs`, reproduce the reasoning with the server rows, and decide: (a) product bug, (b) doc mismatch, (c) lab or driver problem (wrong click, bad recipe, harness gap: report only, never an issue), (d) known issue. For (a) and (b) write the issue body to `docs/analysis/starter-uat-praxis/results/issues/SP-DHA2-p3-<n>.md` (do not run `gh issue create`; the coordinator files it). Sections: `## Problem`, `## Evidence` (log rows with times, the driver quote, screenshot paths, what docs/ says), `## Acceptance criteria`, `## Test type`, `## Docs to update`, `## Client impact`, `## Domain advisor`, `## Needs a human for`, `## Implementation plan`. Title line first: `<system>: <what is wrong> (starter UAT SP-DHA2-p3)`. Mention `@Cadacious`.

## 7. Do not

Do not edit the repo beyond the results file and issue files, the database or the server; do not run the lab; do not deploy; do not paste IPs, keys or account passwords; do not copy the driver's claims into the evidence column.

---

# Verify SP-DHA2-p3: the Marsh letter hand-in, run B

Seed facts: chain 6501 (`interact_tag CmdCenter_Marsh`; conditions world 68, 1360 `active`, step 4038 `active`, 1361 step 4694 not active) runs one action, `complete_mission_with_item` with `{"expected_step_id":4038,"item_id":3730,"quantity":1,"dialog_id":4576,"dialog_set_id":120101,"slot":10}` (the committed receipt `commit_letter_turn_in`). Chain 6502 re-binds DSM 120101 slot 10 on `player_loaded 'Harset_CmdCenter'`. Chain 6505 re-binds it when 1361 completes. Completion offers 1361 through chain 6511 (DSM 5254 slot 10). One-shot rule: O14-19; Harset side only: O14-17 (SP-DL2 grades the crossing). Start: fixture (1360 at step 4038, no letter).

## Row HA-1360a (a4) - refusal without the letter

| Leg | Check | Expected |
|---|---|---|
| a4 no letter (A1.9) | a4 window | the receipt refuses (a refusal reason in the log, or no commit); 1360 still `active` at 4038; no completion; no item change |

Grade: PASS if the a4 server clause holds ("graded on a GM account"). FAIL if the no-letter click completes or consumes anything. BLOCKED if the fixture could not be staged.
Looker `HA1360-no-letter`: 1. Is a dialog window open? 2. Is any error or refusal text visible?


---

## DRIVER REPORT

(The coordinator pastes the driver's final report here, with the screenshot paths.)
