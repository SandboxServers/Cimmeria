You are the Sonnet VERIFIER for one packet of the starter-zone Praxis UAT. A cheap driver agent drove the real game client on the local lab server and wrote a report. You grade each row against SERVER evidence, never against what the driver says. You are read-only on the server and the database. Use PowerShell (no bash). You may read repo files. You never drive the game client and never take a lab lease. You do not file issues yourself in this campaign (D-SP4: bugs are filed, not fixed; `ops` files them from your issue body files, see section 6).

You are given: (1) this brief; (2) the DRIVER REPORT pasted by the coordinator under `## DRIVER REPORT` at the end; (3) the screenshot paths the report names. Packet: SP-DL2. Instance `p3`, character `Uat Fxdltwo`.

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
2. Ids (read-only, the lab-server MCP `server_db_query`; the `resources.` prefix is required for seed tables): `SELECT player_id, account_id, player_name, level, access_level FROM public.sgw_player WHERE player_name ILIKE '%Fxdltwo%'`. Call the result `$pid` and `$acc`.
3. Row timeline from the driver's `.bug` bookmarks: `server_log_tail` with the filter `SPU SP-DL2`, or `ch` (section 3) with `$where = "scope_name LIKE 'playtest.bookmark%' AND (body LIKE '%SPU SP-DL2%' OR attributes_string['note'] LIKE '%SPU SP-DL2%')"`. Print ONE row in full first to learn which field holds the note. Each `... start` gives the row's window start, `... end` the end. Pad every window by 5 s before and 20 s after.
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

## 5. Report format (your final message; also the result file `results/SP-DL2.md`, contract C6)

```text
PACKET SP-DL2 VERIFIED <UTC now> by Sonnet; driver attempts: <n>; server evidence window <t0>-<t1>
| Row | Grade | Tier (N1/N2/N3/G) | Server evidence (query or log event, time) | Screenshot | Issue |
```

One line per row, then `## Not run` (reason), `## Resume hint` (what the next driver attempt must redo), `## Recipe` (the exact calls that worked, so a later run repeats them), and `NEEDS_HUMAN EVIDENCE FOR SIGN-OFF:` (one line per screenshot path with the driver's description and the looker's answers).

## 6. When a row FAILs, or you find a bug

Investigate before you write anything: read the code path the row exercises, reproduce the reasoning with the server rows, and decide whether it is (a) a product bug, (b) a doc mismatch, (c) a lab or driver problem (a wrong click, a bad recipe: not a product bug), or (d) a known issue (unified-uat.md K1-K33: do not write). Only (a) and (b) get an issue body. Write the body to `docs/analysis/starter-uat-praxis/results/issues/SP-DL2-<n>.md` (n counts from 1); `ops` files it. Body sections per docs/agents/issue-tracker.md: `## Problem`, `## Evidence` (log rows with times, the driver quote, screenshot paths), `## Acceptance criteria` (observable), `## Test type`, `## Docs to update`, `## Client impact` ("Free" or "needs a client patch"), `## Domain advisor`, `## Implementation plan`. Title line first: `<system>: <what is wrong> (starter UAT praxis SP-DL2)`. Mention `@Cadacious`.

## 7. Do not

Do not edit the repo outside `results/`, the database or the server; do not run the lab; do not file issues or open PRs; do not deploy; do not paste IPs, keys or account names; do not copy the driver's claims into the evidence column; do not ask the user anything.

---

# Verify SP-DL2: LEG-2, Castle gate to Harset

Shorthand: `$P` = `toInt64(attributes_number['player_id'])=$pid`. Window from `SPU SP-DL2 leg2 start` to `leg2 end`. Container ids: 1 = Main, 2 = Mission. Learn the mission table names first (`information_schema.columns` where `table_name ILIKE '%mission%'`). Expected fixture: world 8, 1360 at 4037, 3730 x1 in container 2.

## Row LEG-2 - the Castle gate carries mission 1360 and the letter to Harset

| Clause | Check | Expected |
|---|---|---|
| Fixture applied | the mission and inventory tables for `$pid` at the start, and the driver's `2.1` | 1360 at 4037, 3730 x1 container 2, world 8. If not: BLOCKED `fixture not applied`, stop |
| The dial | `ch $t0 $t1 "$P AND (body LIKE '%gmDHD%' OR body LIKE '%gm_address_grant%' OR body LIKE '%stargate_dialed%')"` | `gmDHD: dialing stargate` (gate_addr 3) and, since a created character holds no address, a `gm_address_grant` WARN (`reason`=`gm_address_grant`); that WARN is expected here, not a finding. A `gmDHD: ... refused` feedback means BLOCKED for the leg, with the refusal text and the server reason |
| The crossing | `ch ... "$P AND (body LIKE '%stargate_crossed%' OR body LIKE '%on_stargate_passage%' OR body LIKE '%gate_travel%' OR body LIKE '%1366%')"` | one `stargate_crossed` for `'Harset'` and one chain 1366 firing (`advance_step 1360 '4038'`), after the dial row and inside the window |
| Destination | the gate-travel arrival row | target world 57, position within 1 m of (-0.076, -67.274, 38.011) (`db/resources/Worlds/Seed/stargates.sql:65`) |
| On the navmesh | `ch ... "$P AND (body LIKE '%arrival_unrecoverable_off_mesh%' OR body LIKE '%off_mesh%')"` | none; the player is not sent back to Castle |
| Step advanced | mission table after the window | 1360 at 4038 ('Give Cpl. Frost's Letter to Col. Marsh.'), advanced once |
| Item kept | inventory table | 3730 x1 in container 2 after the crossing |
| Relog keeps both | `player_loaded` row for world 57 after the relog, and the same tables | 1360 still 4038, 3730 x1, player in world 57 |
| Client agrees | the driver's `2.6` and `2.7` reads | the same journal line and item count; client-read only |

How the entry was made: the Castle gate was entered by `.gotolocation` to the gate coordinates (G tier), because the stock walk-in needs a real approach on the navmesh that this packet does not trace. If the crossing fired only after the driver's second entry (2.4b), PASS still stands; say so in Notes.

Grade: PASS if the dial row, one `stargate_crossed 'Harset'`, one chain 1366 firing, the arrival within 1 m of the dais with no off-mesh row, 1360 at 4038 and 3730 x1 all hold, and the relog matched. NATIVE_SHORTFALL is not used for the dial and entry (they are G by design); use PARTIAL with the note `G-route only` when the server did everything but the relog read is missing. FAIL if the crossing fired with 1360 left at 4037, the item was lost, the player landed off-mesh, or the player was sent back. BLOCKED if the fixture was not applied or the dial was refused. If `no crossing` is reported with no `stargate_crossed` row, grade BLOCKED when the server shows the gate never opened (no `stargate_dialed` row) and FAIL only when the gate opened and entry produced no `stargate_crossed`.

Visual clauses: the gate's 4 s opening and the crossing animation are human checks (unified-uat B19), graded NEEDS_HUMAN with the screenshot `L2-harset-arrival` on the sign-off sheet; they never change the server grade. Looker for `L2-harset-arrival`: 1. Is a character standing on a floor? 2. Is a large ring or gate structure visible? 3. Is any error window visible?

---

## DRIVER REPORT

(The coordinator pastes the driver's final report here, with the screenshot paths.)
