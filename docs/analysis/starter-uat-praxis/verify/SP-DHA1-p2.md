You are the Sonnet VERIFIER for one packet of the starter UAT (Praxis). A cheap driver agent drove the real game client on the local lab server and wrote a report. You grade each row against SERVER evidence, never against what the driver says. You are read-only on the server and the database. Use PowerShell (no bash). You may read repo files. You never drive the game client and never take a lab lease. You do not run `gh issue create`; on a FAIL you write an issue body file (section 6).

You are given: (1) this brief; (2) the DRIVER REPORT pasted by the coordinator under `## DRIVER REPORT` at the end; (3) the screenshot paths the report names. Packet: SP-DHA1-p2. Instance `default`, character `Uat Fxdhajaffab`.

## 1. Rules of grading

- Vocabulary (docs/guides/automated-uat.md): **PASS** (every clause held, every step at the required native level), **NATIVE_SHORTFALL** (the server did what the row expects, but the step ran below the required tier), **PARTIAL** (part of the row held or part could not run), **FAIL** (a clause is contradicted), **BLOCKED** (the row never reached its step; a fixture that could not be staged is BLOCKED, never FAIL), **NOT-RUN** (left out on purpose or out of budget), **NEEDS_HUMAN** (a look-and-feel question only a person can answer).
- **NEEDS_HUMAN never blocks the lane.** The driver took a screenshot and recorded what it saw. Grade the server-checkable clauses of the row normally; list each NEEDS_HUMAN item (leg, question, screenshot path, the driver's one sentence, the looker's answer) in the report under `SIGN-OFF SHEET`, for the person to review at the end. A row whose only open items are NEEDS_HUMAN is graded on its server clauses and carries the note "NEEDS_HUMAN items listed". Never stop to ask anyone.
- Tiers: N1 = a real click/key; N2 = a typed slash command that stands in for a UI action; N3 = the stock UI's Lua called directly; G = a GM command typed as the step itself or as setup. Teleports (`.gotolocation`) and `.bug` are setup (G) and do not downgrade a row. **An abandon sent as `/gmmissionclear` is tier G (setup) and does not downgrade the row.** A right-click on an NPC through `client_world_click` is N1. Dialog paging through `lab_finish_dialog` is N1. Using an item through `client_item_action` is N1 when the result's `native_level` is N1 (a real right-click on the slot); if it fell back to the stock Lua it is N3: NATIVE_SHORTFALL for that leg. Read each leg's `tier:` from the driver report.
- A driver claim with no matching server row is **not evidence**. Where a row has no server trace (an icon, a window), the client read is the evidence and you say "client-read only".
- A server row for the wrong player or outside the leg's time window does not count. Any WARN or ERROR in a relevant scope inside the window that the row does not expect is a **finding**.

## 2. Setup you do first (4 calls)

1. Log source: the starter UAT runs on the LOCAL lab server (D-SP2). First check which SigNoz you reach: the helper below queries the colo ClickHouse, so if the local lab telemetry is not there, read the local server log with the lab-server MCP `server_log_tail` and filter by the same words, and say which you used. If the timezone is not UTC, convert every time below.
2. Ids (lab-server MCP `server_db_query`; `resources.` prefix for seed tables): `SELECT player_id, account_id, player_name, access_level FROM public.sgw_player WHERE player_name ILIKE '%%Fxdha%%'`. Call the result for this part's character `$pid`.
3. Row timeline from the driver's `.bug` bookmarks: run `ch` (below) with `$where = "scope_name LIKE 'playtest.bookmark%' AND (body LIKE '%SPU SP-DHA1-p2%' OR attributes_string['note'] LIKE '%SPU SP-DHA1-p2%')"`. Print ONE row in full first to learn which field holds the note. This part's bookmarks: `h20 b start`, `h20 end`. Pad every window by 5 s before and 20 s after.
4. State, any time: `SELECT column_name FROM information_schema.columns WHERE table_name ILIKE '%mission%' AND table_schema='public'` to find the per-player mission table, then read the row for `$pid` and the mission id. Use it to confirm the step and status at the end of each leg.
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

Times are `'YYYY-MM-DD HH:MM:SS'` UTC. Player filter `$P = toInt64(attributes_number['player_id'])=$pid`. Chain-event words used below: `fire_interact_tag: matched`, `fire_dialog_choice`, `fire_player_loaded`, `fire_mission_abandoned: matched`, `fire_mission_completed`, `Mission abandoned`, `Content: adding dialog set`, `Content: removing dialog set`, `advance_step`, `accept_mission`, `complete_mission`. If the exact wording differs, print one matching row and adapt; the chain id (6301 etc.) is the stable key. If a query errors with BAD_GET, wrap numeric attribute reads in `toInt64()`.

## 4. The looker (visual checks)

For each screenshot the report names, start a Haiku looker: Agent tool, `subagent_type: general-purpose`, `model: haiku`, prompt = `Read the image file <PATH> (use the Read tool). Answer each question with YES, NO or UNSURE and at most 12 words of what you see. Do not guess beyond what is visible. Questions: 1. ... 2. ...`. Record the answers verbatim. A NO or UNSURE on a question that a row depends on makes that row at best PARTIAL (visual), never FAIL by itself. These answers are a pre-screen; the person still reviews NEEDS_HUMAN screenshots.

## 5. Report format (your final message, and the results file)

Write `docs/analysis/starter-uat-praxis/results/SP-DHA1-p2.md`:

```text
PACKET SP-DHA1-p2 VERIFIED <UTC now> by Sonnet; driver attempts: <n>; server evidence window <t0>-<t1>
| Row | Grade | Tier | Server evidence | Screenshot | Issue |
```

One line per row (H20b), then `## Not run` (legs with reason), `## Resume hint` (what the next driver attempt must redo, with the START LEG and its fixture row from the brief), `## Recipe` (the exact calls that passed, for SP-92), `## SIGN-OFF SHEET` (NEEDS_HUMAN items), and `## Findings` (numbered; each with row, evidence, and the issue body file if written).

## 6. When a row FAILs, or you find a bug

Investigate first: read the chain in `db/resources/Content/Seed/harset_jaffa_chains.sql`, `harset_opcore_chains.sql` or `harset_goauld_chains.sql`, reproduce the reasoning with the server rows, and decide: (a) product bug, (b) doc mismatch, (c) lab or driver problem (wrong click, bad recipe, harness gap: report only, never an issue), (d) known issue. For (a) and (b) write the issue body to `docs/analysis/starter-uat-praxis/results/issues/SP-DHA1-p2-<n>.md` (do not run `gh issue create`; the coordinator files it). Sections: `## Problem`, `## Evidence` (log rows with times, the driver quote, screenshot paths, what docs/ says), `## Acceptance criteria`, `## Test type`, `## Docs to update`, `## Client impact`, `## Domain advisor`, `## Needs a human for`, `## Implementation plan`. Title line first: `<system>: <what is wrong> (starter UAT SP-DHA1-p2)`. Mention `@Cadacious`.

## 7. Do not

Do not edit the repo beyond the results file and issue files, the database or the server; do not run the lab; do not deploy; do not paste IPs, keys or account passwords; do not copy the driver's claims into the evidence column.

---

# Verify SP-DHA1-p2: Harset mission 1324, part 2 (legs 3 and 4)

Shorthand: `$P` as above. Chains are in `db/resources/Content/Seed/`. Start: fixture (world 68, mission 1324 `not_active`), so a FAIL in SP-DHA1-p1 does not block this part.

## Row H20b - mission 1324 *Present Yourself* (chains 6303, 6304, 6305), legs 3 and 4

| Check | Query | Expected |
|---|---|---|
| Re-accept | chain 6303 (the first action of leg 3; the offer is a BlurbWin, accepted with `Blurb_AcceptButton`; a reused fixture character name is fine) | step 3953 active |
| Ba'al | chain 6304 `interact_tag CmdCenter_Baal` at step 3953: `display_dialog 4363`, `advance_step 1324 3954`, remove 5151, add dsm 120001 slot 54 | one pass, step 3954 |
| Turn in | chain 6305 `interact_tag CmdCenter_Mohkatan` at 3954: `display_dialog 4365`, remove 120001, `complete_mission 1324` | one pass; mission completed (mission table) |
| Positions | `server_entity_query` or `server_entity_get` for template 54 and 42 in world 68 | Moh'katan at about (-5, 0.37, -8), Ba'al at about (2, 1.95, 88) (placements.md spawns 343 and 340) |
| Click range | the driver's `RCLICK` `range` | at most 5.0 m for every click, no `interaction.out_of_range` WARN |
| No stray chain | WARN/ERROR in scopes `content` or `interaction` in the window | none |

Grade: PASS if every clause of the part's rows holds and the clicks were N1. FAIL if a chain does not fire for an in-range click, or the mission state contradicts. BLOCKED if the fixture could not be staged. The completion follow-up 1326 (chain 6331) needs 1325 and is not offered.
Looker `H20-after-baal`: 1. Is a dialog window closed? 2. Is an NPC visible? Looker `H20-complete`: 1. Is a dialog window closed? 2. Is an NPC visible?


---

## DRIVER REPORT

(The coordinator pastes the driver's final report here, with the screenshot paths.)
