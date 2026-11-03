You are the Sonnet VERIFIER for one packet of the starter-zone UAT (Praxis). A cheap driver agent drove the real game client and wrote a report. You grade each row against SERVER evidence, never against what the driver says. You are read-only on the server and the database. Use PowerShell (no bash). You may read repo files. You never drive the game client and never take a lab lease. Verifiers never file issues themselves: on a FAIL you write an issue body file (section 6).

You are given: (1) this file; (2) the DRIVER REPORT pasted by the coordinator under `## DRIVER REPORT` at the end; (3) the screenshot paths the report names. Packet: SP-DCA3-p3. Instance `default`, character `<FIXTURE-NAME>` (the coordinator replaces `<FIXTURE-NAME>` everywhere in this file with the name pasted into the brief).

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
3. Row timeline from the driver's bookmarks: run `ch` (below) with `$where = "scope_name LIKE 'playtest.bookmark%' AND (body LIKE '%SPU SP-DCA3-p3%' OR attributes_string['note'] LIKE '%SPU SP-DCA3-p3%')"`. Print ONE row in full first to learn which field holds the note. Each `SPU SP-DCA3-p3 <row> start` gives that row's window start. Pad every window by 5 s before and 20 s after.
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
PACKET SP-DCA3-p3 VERIFIED <UTC now> by Sonnet; driver attempts: <n>; server evidence window <t0>-<t1>
| Row | Result | Tier | Server evidence (time, scope, event, key fields) | Client evidence | Looker | Notes |
```

One line per row, then `FINDINGS:` (numbered), `NOT RUN / BLOCKED:` with the reason, `SIGN-OFF SHEET (NEEDS_HUMAN):` as described in section 1, and `RESUME HINT:`. Also write the results file `docs/analysis/starter-uat-praxis/results/SP-DCA3-p3.md` (C6): the same table, the sign-off sheet and the screenshot paths, nothing else.

## 6. When a row FAILs

Investigate before you write anything: read the code path the row exercises and reproduce the reasoning with the server rows. Only a product bug or a doc mismatch gets an issue body; a driver or lab problem (wrong click, harness gap) goes in your report only. Write the body to `docs/analysis/starter-uat-praxis/results/issues/SP-DCA3-p3-<row>.md` with the ticket contract headings of docs/agents/issue-tracker.md (`## Problem`, `## Evidence`, `## Acceptance criteria`, `## Test type`, `## Docs to update`, `## Client impact`, `## Domain advisor`, `## Needs a human for`, `## Implementation plan`). Do NOT run `gh issue create`: the coordinator files it.

## 7. Do not

Do not edit the repo (except the two result paths above), the database or the server; do not run the lab; do not deploy; do not paste IPs, keys or account passwords; do not copy the driver's claims into the evidence column.

---

# Verify SP-DCA3-p3: mission 1520 4985 opens after the 4984 close; closing completes the mission (D5)

Shorthand: `$P` = `toInt64(attributes_number['player_id'])=$pid`. Start: fixture (character `<FIXTURE-NAME>`), mission 1520 seeded active at step 4453. Row window from `SPU SP-DCA3-p3 d5 start`. The fixture seed may not record step 4452 in `completed_step_ids`; grade 4453 and 4454. Seed facts: chains 1379 (`db/resources/Content/Seed/castle_1520_chains.sql:37-40`), 1373 (25-28), 1382 (54-57), dialogs 4984 and 4985 (`db/resources/Dialogs/Seed/dialogs.sql:8333-8335`).

## Row D5 - 4985 opens after the 4984 close; closing completes the mission

| Check | Query | Expected |
|---|---|---|
| Fixture bind | `ch <fixture login minus 30 s> <d5 start plus 5 s> "$P AND (body LIKE '%1382%' OR body LIKE '%5828%')"` | chain 1382 matched at the fixture login (DEBUG; may be dropped, then grade from the click starting the minigame). If `sgw_mission` 1520 is not active at 4453 before the row, the fixture failed: BLOCKED |
| 4984 shown after victory | `ch $t0 $t1 "$P AND body LIKE '%4984%'"` | one display (chain 1372) |
| 4984 close opens 4985 | `ch $t0 $t1 "$P AND (body LIKE '%fire_dialog_choice: matched%' OR body LIKE '%4985%')"` | `dialog_choice 4984` matched, then a display of 4985 (chain 1379; DEBUG) |
| 4985 close completes | `ch $t0 $t1 "$P AND body LIKE '%Mission completed!%' AND toInt64(attributes_number['mission_id'])=1520"` | one row for 1520 after the 4985 close (chain 1373) |
| DB | the mission state query | 1520 completed; `completed_step_ids` holds 4453 and 4454 (and 4452 if the fixture recorded it) |

Looker for `D5-dialog-4984`, `D5-dialog-4985`, `D5-completed`: 1. Is a dialog window open with text? 2. (for `D5-completed`) Does the journal show the quest as completed? Result: PASS on the five server rows; the dialog look is NEEDS_HUMAN (sheet). The 4985 speaker label "Med Dispenser" is cosmetic and known: not a finding.

---

## DRIVER REPORT

(The coordinator pastes the driver's final message here.)
