You are the Sonnet VERIFIER for one packet of the starter-zone UAT (Praxis). A cheap driver agent drove the real game client and wrote a report. You grade each row against SERVER evidence, never against what the driver says. You are read-only on the server and the database. Use PowerShell (no bash). You may read repo files. You never drive the game client and never take a lab lease. Verifiers never file issues themselves: on a FAIL you write an issue body file (section 6).

You are given: (1) this file; (2) the DRIVER REPORT pasted by the coordinator under `## DRIVER REPORT` at the end; (3) the screenshot paths the report names. Packet: SP-DCA1-p1. Instance `p2`, character `<FIXTURE-NAME>` (the coordinator replaces `<FIXTURE-NAME>` everywhere in this file with the name pasted into the brief).

## 1. Rules of grading

- Grades (C5): **PASS** (every clause held at the required tier), **PARTIAL** (part held or part could not run), **FAIL** (a clause is contradicted by server evidence), **BLOCKED** (the row never reached its step, or its fixture could not be staged), **NOT-RUN** (left out on purpose or out of budget), **NEEDS_HUMAN** (a look-and-feel question only a person can answer), **NATIVE_SHORTFALL** (the server did what the row expects but the step ran below the required tier).
- Tiers: N1 real click or key; N2 typed slash command; N3 stock Lua called directly; G GM command (a `.gotolocation` used as setup or as the step itself does not downgrade a row whose step is that move).
- **Server evidence only.** A clause seen only in the client UI or a screenshot is at best PARTIAL or NEEDS_HUMAN. A driver claim with no matching server row is not evidence; where a row has no server trace say "client-read only".
- **NEEDS_HUMAN rows never block the lane.** Grade a look-and-feel clause NEEDS_HUMAN when the server half passes, list the driver's screenshot path and its one-line "I saw" on the **sign-off sheet** at the end of your report (columns: row, clause, screenshot path, driver note, looker answer). The user reviews that sheet at the end. Do not ask anyone anything.
- A row whose server clauses pass and whose look clause is NEEDS_HUMAN is graded PASS (server) with the NEEDS_HUMAN entry on the sheet; a row with no server clause at all is NEEDS_HUMAN.
- A server row for the wrong player or outside the row window does not count. An unexpected WARN or ERROR in the window is a **finding**.
- Known issues (docs/guides/unified-uat.md K1-K33) are not filed. K20: friendly NPCs give no kill credit.

## 2. Setup you do first (4 calls)

1. Clock: `ssh cimmeria-colo 'docker exec -i signoz-clickhouse clickhouse-client -q "SELECT timezone(), now()"'`. Convert times to UTC if the zone is not UTC. For the local lab server use `server_log_tail` (lab-server MCP) in place of ClickHouse and say so.
2. Ids (`server_db_query`, read-only; seed tables need the `resources.` prefix): `SELECT player_id, account_id, player_name, level FROM public.sgw_player WHERE player_name ILIKE '%<FIXTURE-NAME>%'`. Call the result `$pid` / `$acc`.
3. Row timeline from the driver's bookmarks: run `ch` (below) with `$where = "scope_name LIKE 'playtest.bookmark%' AND (body LIKE '%SPU SP-DCA1-p1%' OR attributes_string['note'] LIKE '%SPU SP-DCA1-p1%')"`. Print ONE row in full first to learn which field holds the note. Each `SPU SP-DCA1-p1 <row> start` gives that row's window start. Pad every window by 5 s before and 20 s after.
4. If SigNoz returns nothing for the window: `SELECT count() FROM signoz_logs.distributed_logs_v2 WHERE timestamp > now() - INTERVAL 1 HOUR AND resources_string['service.name']='cimmeria-server'`. If zero, grade every server clause UNVERIFIED and use `server_log_tail`. DEBUG rows (`resolve_event: chain matched`, `fire_dialog_choice: matched`) may be dropped by the log filter: say so and grade from INFO rows (`Mission accepted`, `Mission step advanced`, `Mission completed!`) plus the DB.

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

Times are `'YYYY-MM-DD HH:MM:SS'` UTC. Player filter `toInt64(attributes_number['player_id'])=$pid`. Body filter `body LIKE '%<text>%'`. If a query errors with BAD_GET wrap numeric attribute reads in `toInt64()`.

Mission state query (read-only): `SELECT mission_id, current_step_id, status, completed_step_ids, completed_objective_ids, active_objective_ids FROM public.sgw_mission WHERE player_id=$pid ORDER BY mission_id`. Print the row and read what `status` values mean in `crates/` before grading (do not assume).

Chain and dialog ids may sit in attributes, not in body; if a body filter on an id returns nothing, rerun with only the player filter and the event text and read the id from the `a` / `n` columns. Mission events carry `mission_id`, `old_step_id` and `new_step_id` as numeric attributes (the body is only the constant text, `crates/cell-content/src/cell/missions/progression.rs` lines 238-245).

## 4. The looker (visual checks)

For each screenshot the report names, start a Haiku looker: Agent tool, `subagent_type: general-purpose`, `model: haiku`, prompt = `Read the image file <PATH> (use the Read tool). Answer each question with YES, NO or UNSURE and at most 12 words of what you see. Do not guess beyond what is visible. Questions: ...` using the questions listed for that screenshot below. Record the answers verbatim. A NO or UNSURE makes the row at best PARTIAL (visual) or NEEDS_HUMAN, never FAIL by itself. A black or missing PNG is reported as such.

## 5. Report format (your final message) and the results file (C6)

```text
PACKET SP-DCA1-p1 VERIFIED <UTC now> by Sonnet; driver attempts: <n>; server evidence window <t0>-<t1>
| Row | Result | Tier | Server evidence (time, scope, event, key fields) | Client evidence | Looker | Notes |
```

One line per row, then `FINDINGS:` (numbered), `NOT RUN / BLOCKED:` with the reason, `SIGN-OFF SHEET (NEEDS_HUMAN):` as described in section 1, and `RESUME HINT:`. Also write the results file `docs/analysis/starter-uat-praxis/results/SP-DCA1-p1.md` (C6): the same table, the sign-off sheet and the screenshot paths, nothing else.

## 6. When a row FAILs

Investigate before you write anything: read the code path the row exercises and reproduce the reasoning with the server rows. Only a product bug or a doc mismatch gets an issue body; a driver or lab problem (wrong click, harness gap) goes in your report only. Write the body to `docs/analysis/starter-uat-praxis/results/issues/SP-DCA1-p1-<row>.md` with the ticket contract headings of docs/agents/issue-tracker.md (`## Problem`, `## Evidence`, `## Acceptance criteria`, `## Test type`, `## Docs to update`, `## Client impact`, `## Domain advisor`, `## Needs a human for`, `## Implementation plan`). Do NOT run `gh issue create`: the coordinator files it.

## 7. Do not

Do not edit the repo (except the two result paths above), the database or the server; do not run the lab; do not deploy; do not paste IPs, keys or account passwords; do not copy the driver's claims into the evidence column.

---

# Verify SP-DCA1-p1: `medical_q1` (mission 10300), accept and journal (A2, A1, A5)

Shorthand: `$P` = `toInt64(attributes_number['player_id'])=$pid`. Start: fixture (character `<FIXTURE-NAME>`). The driver ran A2, A1, A5. Row windows come from the `SPU SP-DCA1-p1 a2 start` bookmark; A1 has no start bookmark (use the A2 end onward) and A5 has none (server clause only). Seed facts: mission 10300 (`db/resources/Missions/Seed/missions.sql:2118`), steps 10340 and 10341 (`mission_steps.sql:7329-7330`), objectives 10460 and 10461 (`mission_objectives.sql:8123-8124`), chains 10000-10006 (`db/resources/Content/Seed/castle_side_stories_chains.sql:2-35`). Step text expected on screen: 10341 "Check the Castle front courtyard for wounded.".

## Row A1 - the mission appears in the journal with history and step text

| Check | Query | Expected |
|---|---|---|
| Mission active in the DB | the mission state query, `mission_id=10300` | one row, `current_step_id`=10341 after the accept (10340 only before it) |
| Server accepted it | `ch $t0 $t1 "$P AND body LIKE '%Mission accepted%' AND toInt64(attributes_number['mission_id'])=10300"` | one row in the A2 window |
| Client read | driver `client_window_read kind=mission` | lists the mission name and the 10341 step text verbatim (client-read only) |

Looker for `A1-journal`: 1. Is a quest journal window open? 2. Does it list a quest about the Castle infirmary or medic? 3. Is the text "Check the Castle front courtyard for wounded" (or close) visible? Result: PASS if the DB and log hold; the on-screen history and wording go on the sign-off sheet (NEEDS_HUMAN).

## Row A2 - offer dialog with the authored screens; Accept starts the mission

| Check | Query | Expected |
|---|---|---|
| Offer chain bound the medic | `ch <fixture login time minus 30 s> <a2 start plus 5 s> "$P AND (body LIKE '%10001%' OR body LIKE '%add_dialog_set%' OR body LIKE '%resolve_event%')"` | chain 10001 (`player_loaded Castle`) matched at the fixture login, adding dialog set 1000001 slot 177 (DEBUG; may be dropped, then grade from chain 10003 firing on the click) |
| Click resolved | `ch $t0 $t1 "$P AND (body LIKE '%interact_tag%' OR body LIKE '%Castle_Pop_Infirmary_Medic1%')"` | chain 10003 matched |
| Accept | `ch $t0 $t1 "$P AND body LIKE '%Mission accepted%' AND toInt64(attributes_number['mission_id'])=10300"` | one row for mission 10300 |
| Post-accept chain | `ch $t0 $t1 "$P AND ((body LIKE '%Mission step advanced%' AND toInt64(attributes_number['mission_id'])=10300 AND toInt64(attributes_number['new_step_id'])=10341) OR body LIKE '%10700%')"` | step advanced to 10341; dialog 10700 displayed |

Looker for `A2-offer` / `A2-after-accept`: 1. Is a dialog window with an NPC and an Accept button visible? 2. Is there body text? Result: PASS when accept and advance rows exist; the offer screens' wording and look are NEEDS_HUMAN (sheet). If the driver could not find the medic, BLOCKED (the grade is never FAIL on a missing entity unless `ch` shows the bind chain firing).

## Row A5 - an old cached mission category resyncs (metadata bump)

| Check | Query | Expected |
|---|---|---|
| Metadata bump at startup | `ch <server start-5m> <server start+5m> "body LIKE '%Cimmeria mission overrides applied; metadata bumped%'"` | one INFO row; fields `bump` and `bumped_metadata` present (source `crates/resources/src/base/resources/apply_overrides.rs:324-330`) |
| Mission is in the catalog | `server_db_query` `SELECT mission_id FROM resources.missions WHERE mission_id=10300` | one row (it is also an in-memory authored entry, `MISSION_ADDITIONS`) |
| Client resync | the driver's `A1-journal` read and screenshot (A5 makes no new call) | the entry is listed |

Result: the driver cannot plant a stale cache, so the cache-staleness clause is NEEDS_HUMAN; grade PARTIAL on the server half only if the bump row is missing. List the `A1-journal` screenshot on the sign-off sheet for this row.

---

## DRIVER REPORT

(The coordinator pastes the driver's final message here.)
