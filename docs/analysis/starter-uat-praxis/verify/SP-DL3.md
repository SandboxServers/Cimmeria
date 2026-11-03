You are the Sonnet VERIFIER for one packet of the starter-zone Praxis UAT. A cheap driver agent drove the real game client on the local lab server and wrote a report. You grade each row against SERVER evidence, never against what the driver says. You are read-only on the server and the database. Use PowerShell (no bash). You may read repo files. You never drive the game client and never take a lab lease. You do not file issues yourself in this campaign (D-SP4: bugs are filed, not fixed; `ops` files them from your issue body files, see section 6).

You are given: (1) this brief; (2) the DRIVER REPORT pasted by the coordinator under `## DRIVER REPORT` at the end; (3) the screenshot paths the report names. Packet: SP-DL3. Instance `default`, character `Uat Fxdlthree`.

## 1. Rules of grading

- Vocabulary (docs/guides/automated-uat.md): **PASS** (every clause held, every step at the required native level), **NATIVE_SHORTFALL** (the server did what the row expects, but the step ran below the required tier), **PARTIAL** (part of the row held or part could not run), **FAIL** (a clause is contradicted), **BLOCKED** (the row never reached its step; a fixture that was not applied is BLOCKED, never FAIL), **NOT-RUN**, **NEEDS_HUMAN** (a look-and-feel question only a person can answer).
- **NEEDS_HUMAN rows never block the lane.** Where the driver took a SHOT-HUMAN screenshot, the clause it covers is graded NEEDS_HUMAN, the screenshot path and the driver's one-line description are recorded, and the evidence goes on the sign-off sheet (`signoff.md`, written by SP-98) for the user to review at the end. You never ask the user anything and never hold the row open for an answer. Server-side clauses of the same row are still graded on their own evidence.
- Tiers: N1 = a real click or key; N2 = a typed slash command standing in for a UI action; N3 = stock UI Lua called directly; G = a GM command typed as the step or as setup. A row whose step IS a GM command (`/gmdhd`, `.gotolocation`) is not downgraded for using it. Read each row's tier from the driver report (`tier:` field) and say which you used.
- A driver claim with no matching server row is **not evidence**. Where a row has no server trace (a window opening, how a model looks), the client read is the evidence and you write "client-read only".
- A server row for the wrong player or outside the row's time window does not count.
- A WARN or ERROR in a relevant scope inside the window that the row does not expect is a **finding**: list it with its row text.
- A FAIL in an early leg that blocks later clauses makes those clauses BLOCKED, not FAIL.

## 2. Setup you do first (4 calls)

1. Clock: `server_log_tail` (lab-server MCP) for the newest rows, or `Get-Date -AsUTC` in PowerShell. Convert every time below to UTC.
2. Ids (read-only, the lab-server MCP `server_db_query`; the `resources.` prefix is required for seed tables): `SELECT player_id, account_id, player_name, level, access_level FROM public.sgw_player WHERE player_name ILIKE '%Fxdlthree%'`. Call the result `$pid` and `$acc`.
3. Row timeline from the driver's `.bug` bookmarks: `server_log_tail` with the filter `SPU SP-DL3`, or `ch` (section 3) with `$where = "scope_name LIKE 'playtest.bookmark%' AND (body LIKE '%SPU SP-DL3%' OR attributes_string['note'] LIKE '%SPU SP-DL3%')"`. Print ONE row in full first to learn which field holds the note. Each `... start` gives the row's window start, `... end` the end. Pad every window by 5 s before and 20 s after.
4. Sanity: if neither the log tail nor SigNoz returns rows for the window, grade every server clause UNVERIFIED and stop. DEBUG rows may be missing if the log filter drops DEBUG: grade from INFO rows and say so.

## 3. The ClickHouse helper (SigNoz fallback; paste once)

Set `$sshHost` to the host that holds the lab SigNoz (the coordinator names it; never write the address into a report). Without a reachable host use `server_log_tail` and skip this section.

```powershell
function ch([string]$t0,[string]$t1,[string]$where){
$sql = @"
SELECT toDateTime(intDiv(timestamp,1000000000)) t, severity_text sev, scope_name sc, substring(body,1,200) b, attributes_string a, attributes_number n, attributes_bool f
FROM signoz_logs.distributed_logs_v2
WHERE timestamp BETWEEN toUnixTimestamp('$t0')*1000000000 AND toUnixTimestamp('$t1')*1000000000
  AND resources_string['service.name']='cimmeria-server' AND ($where)
ORDER BY timestamp LIMIT 120 FORMAT Vertical
"@
$sql | ssh $sshHost 'docker exec -i signoz-clickhouse clickhouse-client -n'
}
```

Times are `'YYYY-MM-DD HH:MM:SS'` UTC. Player filter: `toInt64(attributes_number['player_id'])=$pid`. Event filter: `(attributes_string['event']='<ev>' OR body LIKE '%<ev>%')`. If a query errors with BAD_GET, wrap numeric attribute reads in `toInt64()`. Field names below are not all traced: print one full row first and use what it shows.

## 4. The looker (visual checks)

For each screenshot the report names, start a Haiku looker: Agent tool, `subagent_type: general-purpose`, `model: haiku`, prompt = `Read the image file <PATH> (use the Read tool). Answer each question with YES, NO or UNSURE and at most 12 words of what you see. Do not guess beyond what is visible. Questions: 1. ... 2. ...` using the questions listed per screenshot below. Record the answers verbatim as a pre-screen. A looker answer never turns a NEEDS_HUMAN clause into PASS; a NO or UNSURE on a clause a server row already proves changes nothing. If the PNG is black or missing, say so.

## 5. Report format (your final message; also the result file `results/SP-DL3.md`, contract C6)

```text
PACKET SP-DL3 VERIFIED <UTC now> by Sonnet; driver attempts: <n>; server evidence window <t0>-<t1>
| Row | Grade | Tier (N1/N2/N3/G) | Server evidence (query or log event, time) | Screenshot | Issue |
```

One line per row, then `## Not run` (reason), `## Resume hint` (what the next driver attempt must redo), `## Recipe` (the exact calls that worked, so a later run repeats them), and `NEEDS_HUMAN EVIDENCE FOR SIGN-OFF:` (one line per screenshot path with the driver's description and the looker's answers).

## 6. When a row FAILs, or you find a bug

Investigate before you write anything: read the code path the row exercises, reproduce the reasoning with the server rows, and decide whether it is (a) a product bug, (b) a doc mismatch, (c) a lab or driver problem (a wrong click, a bad recipe: not a product bug), or (d) a known issue (unified-uat.md K1-K33: do not write). Only (a) and (b) get an issue body. Write the body to `docs/analysis/starter-uat-praxis/results/issues/SP-DL3-<n>.md` (n counts from 1); `ops` files it. Body sections per docs/agents/issue-tracker.md: `## Problem`, `## Evidence` (log rows with times, the driver quote, screenshot paths), `## Acceptance criteria` (observable), `## Test type`, `## Docs to update`, `## Client impact` ("Free" or "needs a client patch"), `## Domain advisor`, `## Implementation plan`. Title line first: `<system>: <what is wrong> (starter UAT praxis SP-DL3)`. Mention `@Cadacious`.

## 7. Do not

Do not edit the repo outside `results/`, the database or the server; do not run the lab; do not file issues or open PRs; do not deploy; do not paste IPs, keys or account names; do not copy the driver's claims into the evidence column; do not ask the user anything.

---

# Verify SP-DL3: LEG-3, Harset gate to Tollana

Shorthand: `$P` = `toInt64(attributes_number['player_id'])=$pid`. Window from `SPU SP-DL3 leg3 start` to `leg3 end`. Container ids: 1 = Main, 2 = Mission. Expected fixture: world 57, 1360 at 4038, 3730 x1 in container 2.

Context for the grade: no hand-off mission is playable from Harset to Tollana (618 and 621 recipient graphs are BlockedEvidence; T-05 to T-10 held; [harset-continuation README](../../harset-continuation/README.md), [T-01](../../tollana-continuation/evidence/T-01-offline-placement.md)). The row therefore has two parts and one grade that names both.

## Row LEG-3 - the Harset gate carries the state to Tollana

| Clause | Check | Expected |
|---|---|---|
| Fixture applied | tables for `$pid` at the start; the driver's `3.1` | world 57, 1360 at 4038, 3730 x1 container 2. If not: BLOCKED `fixture not applied`, stop |
| Part A: is Tollana dialable by a player | the driver's `client_ui_clickables` list from `3.2` (client-read only) and `SELECT known_stargates` for `$pid` from the player's persisted state if the column is readable | a created character holds no Tollana address, so Tollana is expected NOT listed; record listed or not listed. If it is not listed, Part A grade is `BLOCKED: no playable Harset to Tollana hand-off` (the natural dial needs an address no live chain grants) |
| Part B dial | `ch $t0 $t1 "$P AND (body LIKE '%gmDHD%' OR body LIKE '%gm_address_grant%' OR body LIKE '%stargate_dialed%')"` | `gmDHD: dialing stargate` (gate_addr 4) with the expected `gm_address_grant` WARN. A `refused` reply: BLOCKED for Part B with the server reason |
| Crossing | `ch ... "$P AND (body LIKE '%stargate_crossed%' OR body LIKE '%on_stargate_passage%' OR body LIKE '%gate_travel%')"` | one gate-travel row to world 19 inside the window, after the dial |
| Destination and navmesh | the arrival row; `ch ... "$P AND body LIKE '%arrival_unrecoverable_off_mesh%'"` | arrival within 2 m of the Tollana gate (211.045, 0.606, -547.644) (`db/resources/Worlds/Seed/stargates.sql:37`); no off-mesh row; the player is not sent back |
| State survives | mission and inventory tables | 1360 still at 4038 and not advanced or completed by the crossing; 3730 x1 in container 2; Marsh was not clicked, so no `harset.letter_turn_in` row exists |
| Relog keeps it | `player_loaded` row for world 19 after the relog, the same tables | 1360 at 4038, 3730 x1, world 19 |
| Client agrees | the driver's `3.7` and `3.8` reads | the same lines; client-read only |

Grade, Part A: `BLOCKED: no playable Harset to Tollana hand-off` when Tollana is not in the real DHD list (expected); PASS only if Tollana was listed and the dial was an N1 click, which would be a finding for the ledger. Grade, Part B (the gate transfer with the address from a GM): PASS if one gate-travel row to world 19 landed within 2 m of the gate with no off-mesh row and the state survived and the relog matched; FAIL if the player fell or was sent back, 1360 or 3730 was lost or advanced; BLOCKED if the fixture was not applied or the dial was refused. The row's single result in the table is `PARTIAL (A: BLOCKED no playable hand-off; B: <grade>)` when B passes, because the hand-off itself cannot be played; it is not a FAIL and not a PASS. Put the Part B evidence in the table in full.

Tollana in-client checks (Copplemann visible within about 7 m, DHD 39 clickable at (209.748, -1.008, -552.898) `spawnlist.sql:281`, ring console reachable): client-read only, graded NEEDS_HUMAN from `L3-tollana-arrival` on the sign-off sheet. The Harset DHD list screenshot `L3-harset-dhd-list` goes to the sheet the same way (names, none Unknown). Neither changes the server grade. Looker for `L3-harset-dhd-list`: 1. Is a list of gate names visible? 2. Does any entry read Unknown? 3. Is the name Tollana listed? Looker for `L3-tollana-arrival`: 1. Is a character standing on a floor? 2. Is a gate ring visible? 3. Is any error window visible?

---

## DRIVER REPORT

(The coordinator pastes the driver's final report here, with the screenshot paths.)
