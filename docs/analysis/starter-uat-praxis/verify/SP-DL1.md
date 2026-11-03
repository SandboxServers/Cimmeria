You are the Sonnet VERIFIER for one packet of the starter-zone Praxis UAT. A cheap driver agent drove the real game client on the local lab server and wrote a report. You grade each row against SERVER evidence, never against what the driver says. You are read-only on the server and the database. Use PowerShell (no bash). You may read repo files. You never drive the game client and never take a lab lease. You do not file issues yourself in this campaign (D-SP4: bugs are filed, not fixed; `ops` files them from your issue body files, see section 6).

You are given: (1) this brief; (2) the DRIVER REPORT pasted by the coordinator under `## DRIVER REPORT` at the end; (3) the screenshot paths the report names. Packet: SP-DL1. Instance `default`, character `Uat Fxdlone`.

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
2. Ids (read-only, the lab-server MCP `server_db_query`; the `resources.` prefix is required for seed tables): `SELECT player_id, account_id, player_name, level, access_level FROM public.sgw_player WHERE player_name ILIKE '%Fxdlone%'`. Call the result `$pid` and `$acc`.
3. Row timeline from the driver's `.bug` bookmarks: `server_log_tail` with the filter `SPU SP-DL1`, or `ch` (section 3) with `$where = "scope_name LIKE 'playtest.bookmark%' AND (body LIKE '%SPU SP-DL1%' OR attributes_string['note'] LIKE '%SPU SP-DL1%')"`. Print ONE row in full first to learn which field holds the note. Each `... start` gives the row's window start, `... end` the end. Pad every window by 5 s before and 20 s after.
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

## 5. Report format (your final message; also the result file `results/SP-DL1.md`, contract C6)

```text
PACKET SP-DL1 VERIFIED <UTC now> by Sonnet; driver attempts: <n>; server evidence window <t0>-<t1>
| Row | Grade | Tier (N1/N2/N3/G) | Server evidence (query or log event, time) | Screenshot | Issue |
```

One line per row, then `## Not run` (reason), `## Resume hint` (what the next driver attempt must redo), `## Recipe` (the exact calls that worked, so a later run repeats them), and `NEEDS_HUMAN EVIDENCE FOR SIGN-OFF:` (one line per screenshot path with the driver's description and the looker's answers).

## 6. When a row FAILs, or you find a bug

Investigate before you write anything: read the code path the row exercises, reproduce the reasoning with the server rows, and decide whether it is (a) a product bug, (b) a doc mismatch, (c) a lab or driver problem (a wrong click, a bad recipe: not a product bug), or (d) a known issue (unified-uat.md K1-K33: do not write). Only (a) and (b) get an issue body. Write the body to `docs/analysis/starter-uat-praxis/results/issues/SP-DL1-<n>.md` (n counts from 1); `ops` files it. Body sections per docs/agents/issue-tracker.md: `## Problem`, `## Evidence` (log rows with times, the driver quote, screenshot paths), `## Acceptance criteria` (observable), `## Test type`, `## Docs to update`, `## Client impact` ("Free" or "needs a client patch"), `## Domain advisor`, `## Implementation plan`. Title line first: `<system>: <what is wrong> (starter UAT praxis SP-DL1)`. Mention `@Cadacious`.

## 7. Do not

Do not edit the repo outside `results/`, the database or the server; do not run the lab; do not file issues or open PRs; do not deploy; do not paste IPs, keys or account names; do not copy the driver's claims into the evidence column; do not ask the user anything.

---

# Verify SP-DL1: LEG-1, Cellblock exit ring to Castle

Shorthand: `$P` = `toInt64(attributes_number['player_id'])=$pid`. Window from `SPU SP-DL1 leg1 start` to `leg1 end`. Inventory container ids: 1 = Main, 2 = Mission (`crates/lab/src/uat/spec_fixture.rs`). Mission progress lives in the per-player mission tables: learn the exact names first with `SELECT table_name, column_name FROM information_schema.columns WHERE table_schema='public' AND table_name ILIKE '%mission%'` and use the one that holds `player_id`, mission id and step. Expected fixture: 688 at 80688, 1360 at 4037, item 3730 x1 in container 2.

## Row LEG-1 - the exit ring carries the state to Castle

| Clause | Check | Expected |
|---|---|---|
| Fixture applied | `server_db_query` on the mission and inventory tables before the press (the driver's STATE-before is client-read only) and the driver's `1.1` values | 688 at 80688, 1360 at 4037, 3730 x1 in container 2, `world_id` 12. If not: BLOCKED `fixture not applied`, stop |
| Transfer on the first press | `ch $t0 $t1 "$P AND (body LIKE '%cross_world%' OR body LIKE '%1109%' OR body LIKE '%Cellblock_ArmoryRingSwitch%' OR body LIKE '%complete_mission%')"` | exactly one chain 1109 firing (`complete_mission 688` then `cross_world_teleport 'Castle'`), one press, no second firing; time inside the driver's `1.4` window |
| Destination | the cross-world travel row for the player (unified-uat T21/T22 name it: cross-world travel to Castle, the old space destroyed) | target world 8, position (466.365, 70.397, 991.466) within 0.5 m; and a region row `Castle.ArmoryRingDropZone` fired (unified-uat T21/T22 telemetry note) |
| On the navmesh | `ch ... "$P AND (body LIKE '%arrival_unrecoverable_off_mesh%' OR body LIKE '%off_mesh%' OR body LIKE '%fall%')"` and `server_nav_path` or `server_entity_get` for the player's position in Castle | no off-mesh row; the player's server position is 70.4 +/- 1 m on y, not falling. Client-read: the driver's position after WAITW equals the server position within 1 m |
| 688 completed | mission table read after the window | 688 completed once (a single completion row, no double reward); the exit did not complete it before the press (compare the step before: 80688 active) |
| State survives | mission and inventory tables after the window | 1360 still active at 4037; 3730 x1 in container 2; character appearance rows unchanged (`SELECT * FROM public.sgw_player WHERE player_id=$pid` before and after has the same appearance columns) |
| Relog keeps it | `ch ... "$P AND (body LIKE '%player_loaded%' OR body LIKE '%login%')"` after the press, plus the same tables | one load of world 8 after the relog; the same 1360 step, 3730 count and `688` completed; no second reward, no second chain 1109 firing |
| Client agrees | the driver's `1.7` and `1.9` reads | the same items and journal lines as the server tables; client-read only |

Grade: PASS if the fixture held, one chain 1109 firing and one cross-world travel to Castle landed within 0.5 m of the platform with no off-mesh row, 688 completed once, 1360 at 4037 and 3730 x1 survived, and the relog read matched. NATIVE_SHORTFALL if the server did all of that but the switch press was not an N1 world click (read the `tier:`). PARTIAL if the transfer held and the relog read was missing or the state check could not be read. FAIL if the press did not move the player, moved them to another place, completed 688 twice, lost 1360 or 3730, or the player fell (an off-mesh row). BLOCKED if the fixture was not applied or the switch entity was never found. The ring animation is not a clause: it does not exist without the client patch (ring-transport README), so its absence is not a finding.

Appearance and the look of the landing: client-read only. Clause `appearance after the transfer is intact` is graded NEEDS_HUMAN from the screenshot `L1-arrival-castle`, and the evidence goes on the sign-off sheet; it never changes the server grade.
Looker for `L1-before-press`: 1. Is a prison-like corridor or room visible? 2. Is there a glowing object near the middle of the view? Looker for `L1-arrival-castle`: 1. Is a character standing on a floor (not in the sky)? 2. Does the character model look normal (two arms, two legs, no stretching)? 3. Is any error window visible?

---

## DRIVER REPORT

(The coordinator pastes the driver's final report here, with the screenshot paths.)
