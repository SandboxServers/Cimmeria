You are the Sonnet VERIFIER for one packet of the starter-zone UAT (Praxis). A cheap driver agent drove the real game client and wrote a report. You grade each row against SERVER evidence, never against what the driver says. You are read-only on the server and the database. Use PowerShell (no bash). You may read repo files. You never drive the game client and never take a lab lease. Verifiers never file issues themselves: on a FAIL you write an issue body file (section 6).

You are given: (1) this file; (2) the DRIVER REPORT pasted by the coordinator under `## DRIVER REPORT` at the end; (3) the screenshot paths the report names. Packet: SP-DCA2-p2. Instance `p2` (any available client is fine, one at a time), character `<FIXTURE-NAME>` (the coordinator replaces `<FIXTURE-NAME>` everywhere in this file with the name pasted into the brief).

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
3. Row timeline from the driver's bookmarks: run `ch` (below) with `$where = "scope_name LIKE 'playtest.bookmark%' AND (body LIKE '%SPU SP-DCA2-p2%' OR attributes_string['note'] LIKE '%SPU SP-DCA2-p2%')"`. Print ONE row in full first to learn which field holds the note. Each `SPU SP-DCA2-p2 <row> start` gives that row's window start. Pad every window by 5 s before and 20 s after.
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
PACKET SP-DCA2-p2 VERIFIED <UTC now> by Sonnet; driver attempts: <n>; server evidence window <t0>-<t1>
| Row | Result | Tier | Server evidence (time, scope, event, key fields) | Client evidence | Looker | Notes |
```

One line per row, then `FINDINGS:` (numbered), `NOT RUN / BLOCKED:` with the reason, `SIGN-OFF SHEET (NEEDS_HUMAN):` as described in section 1, and `RESUME HINT:`. Also write the results file `docs/analysis/starter-uat-praxis/results/SP-DCA2-p2.md` (C6): the same table, the sign-off sheet and the screenshot paths, nothing else.

## 6. When a row FAILs

Investigate before you write anything: read the code path the row exercises and reproduce the reasoning with the server rows. Only a product bug or a doc mismatch gets an issue body; a driver or lab problem (wrong click, harness gap) goes in your report only. Write the body to `docs/analysis/starter-uat-praxis/results/issues/SP-DCA2-p2-<row>.md` with the ticket contract headings of docs/agents/issue-tracker.md (`## Problem`, `## Evidence`, `## Acceptance criteria`, `## Test type`, `## Docs to update`, `## Client impact`, `## Domain advisor`, `## Needs a human for`, `## Implementation plan`). Do NOT run `gh issue create`: the coordinator files it.

## 7. Do not

Do not edit the repo (except the two result paths above), the database or the server; do not run the lab; do not deploy; do not paste IPs, keys or account passwords; do not copy the driver's claims into the evidence column.

---

# Verify SP-DCA2-p2: mission 1520 "Med Dispenser", Livewire (D4)

Shorthand: `$P` = `toInt64(attributes_number['player_id'])=$pid`. Start: fixture (character `<FIXTURE-NAME>`, a different character from part 1), with mission 1520 seeded active at step 4453. The row window comes from `SPU SP-DCA2-p2 d4 start`. Seed facts: steps 4452-4454 (`db/resources/Missions/Seed/mission_steps.sql:2052-2056`), chains 1371, 1372, 1378 and 1382 (`db/resources/Content/Seed/castle_1520_chains.sql`, lines cited per row), dialogs 4983-4985 (`db/resources/Dialogs/Seed/dialogs.sql:8331-8335`), dispenser spawn (`db/resources/Worlds/Seed/spawnlist.sql:628`).

## Row D4 - right-click starts Livewire; defeat allows retry; victory advances to 4454 and shows 4984

| Check | Query | Expected |
|---|---|---|
| Gate: chains enabled | `server_db_query` `SELECT chain_id, enabled FROM resources.content_chains WHERE chain_id IN (1371,1372,1382)` | all `true`. If any is `false` the row is BLOCKED (Goa'uld-branch, BlockedEvidence CS-02) |
| Server step vs tracker text | `missions_restored` row and the DB | server step is 4453. The client tracker may show step 4452's display text ("Secure a supply of Ambernol..."); grade the step from the server, never from the tracker text |
| Fixture bind | `ch <fixture login time minus 30 s> <d4 start plus 5 s> "$P AND (body LIKE '%1382%' OR body LIKE '%5828%' OR body LIKE '%add_dialog_set%')"` | chain 1382 (`player_loaded Castle` at step 4453) added dialog set 5828 slot 412 at the fixture login (DEBUG; may be dropped, then grade from the click starting the minigame). If the DB shows `sgw_mission` 1520 not active at 4453 the fixture failed: BLOCKED |
| Minigame started | `server_minigame_state` / `server_minigame_tap_read`, or `ch $t0 $t1 "$P AND (body LIKE '%minigame%' OR body LIKE '%Livewire%')"` | a Livewire session ticket for `$pid`, difficulty 1 (chain 1371, lines 14-17) |
| Abort then retry (the defeat clause) | the same query across the window | `Minigame aborted without a result` (player_canceled) followed by a second start. Closing the window aborts the game; it does not lose a board, so this clause is PARTIAL by design (a WARN `result_deposit_refused` on the abort is expected). A true defeat is not exercised; no brief step can lose a board. Else the retry clause is NOT-RUN |
| Victory chain | `ch $t0 $t1 "$P AND (body LIKE '%1372%' OR (body LIKE '%Mission step advanced%' AND toInt64(attributes_number['mission_id'])=1520 AND toInt64(attributes_number['new_step_id'])=4454))"` | chain 1372: step 4454, dialog 4984 displayed (lines 20-22); DB `current_step_id`=4454 |
| Win result | the driver's `client_minigame_play` result | `effect_ok` and a win outcome recorded verbatim |
| Lighting | `server_entity_get` the dispenser | dialog set 5828 removed after victory |

Looker for `D4-livewire-open` / `D4-after-victory`: 1. Is a minigame window (wires or circuit puzzle) open? 2. Is a dialog window open? Result: PASS on the start and victory rows; the defeat clause is PARTIAL by design (abort, not defeat) and a `no close control found` report makes it NOT-RUN. The driver's closing of 4984 and 4985 at the end is not graded here (row D5 of SP-DCA3-p3).

---

## DRIVER REPORT

(The coordinator pastes the driver's final message here.)
