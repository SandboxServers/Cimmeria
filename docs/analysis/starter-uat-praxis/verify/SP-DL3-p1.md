You are the Sonnet VERIFIER for one packet of the starter-zone Praxis UAT. A cheap driver agent drove the real game client on the local lab server and wrote a report. You grade each row against SERVER evidence, never against what the driver says. You are read-only on the server and the database. Use PowerShell (no bash). You may read repo files. You never drive the game client and never take a lab lease. You do not file issues yourself in this campaign (D-SP4: bugs are filed, not fixed; `ops` files them from your issue body files, see section 6).

You are given: (1) this brief; (2) the DRIVER REPORT pasted by the coordinator under `## DRIVER REPORT` at the end; (3) the screenshot paths the report names. Packet: SP-DL3-p1. Instance `default`, character `Uat Fxdlthree`.

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
3. Row timeline from the driver's `.bug` bookmarks: `server_log_tail` with the filter `SPU SP-DL3-p1`, or `ch` (section 3) with `$where = "scope_name LIKE 'playtest.bookmark%' AND (body LIKE '%SPU SP-DL3-p1%' OR attributes_string['note'] LIKE '%SPU SP-DL3-p1%')"`. Print ONE row in full first to learn which field holds the note. Each `... start` gives the row's window start, `... end` the end. Pad every window by 5 s before and 20 s after.
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

## 5. Report format (your final message; also the result file `results/SP-DL3-p1.md`, contract C6)

```text
PACKET SP-DL3-p1 VERIFIED <UTC now> by Sonnet; driver attempts: <n>; server evidence window <t0>-<t1>
| Row | Grade | Tier (N1/N2/N3/G) | Server evidence (query or log event, time) | Screenshot | Issue |
```

One line per row, then `## Not run` (reason), `## Resume hint` (what the next driver attempt must redo), `## Recipe` (the exact calls that worked, so a later run repeats them), and `NEEDS_HUMAN EVIDENCE FOR SIGN-OFF:` (one line per screenshot path with the driver's description and the looker's answers).

## 6. When a row FAILs, or you find a bug

Investigate before you write anything: read the code path the row exercises, reproduce the reasoning with the server rows, and decide whether it is (a) a product bug, (b) a doc mismatch, (c) a lab or driver problem (a wrong click, a bad recipe: not a product bug), or (d) a known issue (unified-uat.md K1-K33: do not write). Only (a) and (b) get an issue body. Write the body to `docs/analysis/starter-uat-praxis/results/issues/SP-DL3-p1-<n>.md` (n counts from 1); `ops` files it. Body sections per docs/agents/issue-tracker.md: `## Problem`, `## Evidence` (log rows with times, the driver quote, screenshot paths), `## Acceptance criteria` (observable), `## Test type`, `## Docs to update`, `## Client impact` ("Free" or "needs a client patch"), `## Domain advisor`, `## Implementation plan`. Title line first: `<system>: <what is wrong> (starter UAT praxis SP-DL3-p1)`. Mention `@Cadacious`.

## 7. Do not

Do not edit the repo outside `results/`, the database or the server; do not run the lab; do not file issues or open PRs; do not deploy; do not paste IPs, keys or account names; do not copy the driver's claims into the evidence column; do not ask the user anything.

---

# Verify SP-DL3-p1: LEG-3a, Harset DHD dial list

Shorthand: `$P` = `toInt64(attributes_number['player_id'])=$pid`. Window from `SPU SP-DL3-p1 leg3a start` to `leg3a end`. Start: fixture (world 57 only). This is Part A of the former LEG-3; Part B is [SP-DL3-p2](SP-DL3-p2.md).

Context for the grade: no hand-off mission is playable from Harset to Tollana (618 and 621 recipient graphs are BlockedEvidence; T-05 to T-10 held; [harset-continuation README](../../harset-continuation/README.md), [T-01](../../tollana-continuation/evidence/T-01-offline-placement.md)).

## Row LEG-3a - is Tollana on the real Harset DHD list

| Clause | Check | Expected |
|---|---|---|
| Fixture applied | the driver's S1 read; `SELECT` the player's world from the persisted state | world 57. If not: BLOCKED `fixture not applied`, stop |
| The DHD opens on a real click | the driver's `3.2` click and `tier:` field; client-read only | an N1 world click opened the dialling window (a `has no pose` point click is still N1) |
| Tollana dialable by a player | the driver's `client_ui_clickables` list from `3.2` (client-read only) and `SELECT known_stargates` for `$pid` from the player's persisted state if the column is readable | a created character holds no Tollana address, so Tollana is expected NOT listed; record listed or not listed |

Grade: `BLOCKED: no playable Harset to Tollana hand-off` when Tollana is not in the real DHD list (expected: the natural dial needs an address no live chain grants); PASS only if Tollana was listed and the click was N1, which would be a finding for the ledger; BLOCKED `no window` if the click opened nothing after the retry. The combined LEG-3 result on the sign-off sheet is composed from this result and `results/SP-DL3-p2.md`: `PARTIAL (A: <this grade>; B: <p2 grade>)` when B passes, because the hand-off itself cannot be played; it is not a FAIL and not a PASS.

The Harset DHD list screenshot `L3-harset-dhd-list` goes to the sign-off sheet as NEEDS_HUMAN (names, none Unknown); it does not change the server grade. Looker for `L3-harset-dhd-list`: 1. Is a list of gate names visible? 2. Does any entry read Unknown? 3. Is the name Tollana listed?

---

## DRIVER REPORT

(The coordinator pastes the driver's final report here, with the screenshot paths.)
