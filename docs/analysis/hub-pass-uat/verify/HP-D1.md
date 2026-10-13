You are the Sonnet VERIFIER for one packet of the hub pass UAT. A cheap driver agent drove the real game client on the colo and wrote a report. You grade each row against SERVER evidence, never against what the driver says. You are read-only on the server and the database. Use PowerShell (no bash). You may read repo files. You may run `gh` to file issues (see section 6). You never drive the game client and never take a lab lease.

You are given: (1) this brief; (2) the DRIVER REPORT pasted by the coordinator under `## DRIVER REPORT` at the end; (3) the list of screenshot paths the report names. Packet: HP-D1. Instance `p3`, character `Hubdeploy`.

## 1. Rules of grading

- Vocabulary (from docs/guides/automated-uat.md): **PASS** (every clause held, every step at the required native level), **NATIVE_SHORTFALL** (the server did what the row expects, but the step ran below the required tier), **PARTIAL** (part of the row held or part could not run), **FAIL** (a clause is contradicted), **BLOCKED** (the row never reached its step), **NOT-RUN** (the packet left it out on purpose or ran out of budget), **NEEDS_HUMAN** (a look-and-feel question only a person can answer; the looker's answer is recorded as a pre-screen).
- Tiers: N1 = a real click/key; N2 = a typed slash command that stands in for a UI action; N3 = the stock UI's Lua called directly (a cast with `useAbility`, `usePetAbility`, `mailTakeItem`, a chat line submitted by `ChatMod.onTextAccepted`); G = a GM command typed or submitted as the step itself or as setup. For rows whose step IS a GM or slash command (`.pet summon`, `.mail`, `.announce`, `/tell`), submitting it with CHATX counts as G/typed and does not downgrade the row. For rows whose step is a UI action (cast an ability, press a stance button, take a mail attachment, order a pet), an N3 call gives at best **NATIVE_SHORTFALL**; an N1 click gives PASS if the server agrees. Read each row's tier from the driver report (`tier:` field) and say which you used.
- A driver claim with no matching server row is **not evidence**. Where a row has no server trace (a window opening, a colour), the client read (chat log fields, Lua values) is the evidence and you say "client-read only".
- A server row that exists but is for the wrong player, or outside the row's time window, does not count.
- Any WARN or ERROR in a relevant scope inside the packet's window that the row does not expect is a **finding**: list it with its row text.

## 2. Setup you do first (4 calls)

1. Clock: `ssh cimmeria-colo 'docker exec -i signoz-clickhouse clickhouse-client -q "SELECT timezone(), now()"'`. If the timezone is not UTC, convert every time below.
2. Ids (read-only DB, the lab-server MCP `server_db_query`; the `resources.` prefix is required for seed tables): `SELECT player_id, account_id, player_name, level, exp, access_level FROM public.sgw_player WHERE player_name ILIKE '%Hubdeploy%'`. Call the result `$pid` (player_id) and `$acc` (account_id; the lab accounts are 10, 11 and 12 for lab, lab2, lab3).
3. Row timeline from the driver's `.bug` bookmarks: run `ch` (below) for the packet's window with `$where = "scope_name LIKE 'playtest.bookmark%' AND (body LIKE '%HPU HP-D1%' OR attributes_string['note'] LIKE '%HPU HP-D1%')"`. Print ONE row in full first to learn which field holds the note. Each `HPU HP-D1 <row> start` gives that row's window start; the next `start` (or the packet end) gives its end. Pad every window by 5 s before and 20 s after.
4. If SigNoz returns nothing at all for the window, the log level or the deploy may differ: run `SELECT count() FROM signoz_logs.distributed_logs_v2 WHERE timestamp > now() - INTERVAL 1 HOUR AND resources_string['service.name']='cimmeria-server'`. If that is zero, grade every server clause UNVERIFIED and stop. DEBUG rows (pets.*, deployables.pulse, mail.notified) may be missing if the colo's log filter drops DEBUG: say so, grade from INFO rows, and mark the row PARTIAL if the DEBUG row was essential.

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
Times are `'YYYY-MM-DD HH:MM:SS'` UTC. Player filter: `toInt64(attributes_number['player_id'])=$pid`. Account filter: `toInt64(attributes_number['account_id'])=$acc`. Event filter: `(attributes_string['event']='<ev>' OR body LIKE '%<ev>%')`. `console.feedback` rows hold the GM command reply text only in `body` (first 400 chars). The SigNoz MCP returned 401 last round: use it only if it works; otherwise this helper is the method. If a query errors with BAD_GET, wrap numeric attribute reads in `toInt64()`.

## 4. The looker (visual checks)

For each screenshot the report names, start a Haiku looker: Agent tool, `subagent_type: general-purpose`, `model: haiku`, prompt = `Read the image file <PATH> (use the Read tool). Answer each question with YES, NO or UNSURE and at most 12 words of what you see. Do not guess beyond what is visible. Questions: 1. ... 2. ...` using the exact questions listed for that screenshot below. Record the answers verbatim in your report. A NO or UNSURE on a question that a row depends on makes that row at best PARTIAL (visual), never FAIL by itself: a screenshot can miss the thing. If the PNG is black or missing, say so.

## 5. Report format (your final message)

```
PACKET HP-D1 VERIFIED <UTC now> by Sonnet; driver attempts: <n>; server evidence window <t0>-<t1>
| Row | Result | Tier | Server evidence (time, scope, event, key fields) | Client evidence | Looker | Notes |
```
One line per row, then `FINDINGS:` (numbered; each with the row, the evidence and whether you filed an issue and its number), then `NOT RUN / BLOCKED:` with the reason, then `RESUME HINT:` (what the next driver attempt must redo, if any row needs a re-run).

## 6. When a row FAILs, or you find a bug

Investigate it before you file anything: read the code path the row exercises (the doc named in the row), reproduce the reasoning with the server rows, and decide whether it is (a) a product bug, (b) a doc mismatch, (c) a lab or driver problem (a wrong click, a bad recipe, a harness gap: not a product bug), or (d) a known issue (unified-uat.md K1-K33: do not file). Only (a) and (b) get a GitHub issue; (c) goes in your report only, and in `NOT RUN / BLOCKED` with the recipe the next driver needs.
File with `gh issue create` from PowerShell (a `--body-file`, never a leading-slash string). Title: `<system>: <what is wrong> (hub pass UAT HP-D1)`. Labels: `bug`; add `needs-triage` only if the label exists (`gh label list`), otherwise say so in the body. Never label `ready-for-agent`: only a maintainer does. The body follows the ticket contract in docs/agents/issue-tracker.md: `## Problem`, `## Evidence` (the ClickHouse rows with times, the driver quote, the screenshot paths, and what docs/ already says), `## Acceptance criteria` (observable: a log field, a DB row, a chat line), `## Test type` (from TESTING.md, with the bug shape the guard must fail on), `## Docs to update` (rows of docs/agents/doc-update-map.md), `## Client impact` ("Free" or "needs a client patch"), `## Domain advisor` (a .claude/agents/ name), `## Needs a human for`. Add an `## Implementation plan` section: the files and functions to change (you read them), the order of edits, and the test that fails before the fix. Mention `@Cadacious`. Search first (`gh issue list --search "<words>" --state all`) and comment on an existing issue instead of filing a duplicate.

## 7. Do not

Do not edit the repo, the database or the server; do not run the lab; do not deploy; do not paste IPs, keys or account passwords; do not copy the driver's claims into the evidence column.

---

# Verify HP-D1: deployables DP-U1, DP-U2, DP-U4

Shorthand: `$P` = `toInt64(attributes_number['player_id'])=$pid`, `$ACC` = account filter. Window from the `HPU HP-D1 dp1 start` bookmark. Ability 1012: ground target, range 5 m, warmup 2 s, cooldown 30 s, 30 pulses 1 s apart, then gone. Template 400. If the driver reports BLOCKED on the ground cast, grade all three rows BLOCKED, list exactly what was tried (the tool results and chat lines, the screenshot `D-cast-failed`), check the server for `deploy_refused` rows (the `reason` tells whether the press reached the server with a ground point: `no_ground_point` means the client sent a plain `useAbility`), and write the recipe gap in RESUME HINT. Do not file a product bug for a lab recipe gap.

## DP-U1 - place one

| Clause | Query | Expected |
|---|---|---|
| The ability is known | `SELECT abilities FROM public.sgw_player WHERE player_id=$pid` | contains 1012 |
| Press reached the server with a ground point | `ch ... "scope_name='deployables.lifecycle' AND $P AND (attributes_string['event'] IN ('deploy_launched','deploy_refused','spawned') OR body LIKE '%deploy_%' OR body LIKE '%spawned%')"` | `deploy_launched` (DEBUG), then `spawned` (INFO) about 2 s later (warmup), with `ability_id`=1012, `template_id`=400, `x/y/z` within 5 m of the caster, `radius`=10, `pulses_total`=30 |
| Cooldown/warmup shown | the client: the hotbar cooldown only if the ability is on the bar; the driver's `D1-warmup` shot | visual pre-screen |
| Object exists on the client | driver GC.3 entity list | an entity named "Deployable: Microwave Emitter" within 5 m of the avatar |

Grade: the cast was `useAbility` (N3) plus a real world click (N1): **NATIVE_SHORTFALL** if the server rows are right. FAIL if the server shows `deploy_refused` for a reason the recipe should not hit (`out_of_range` at 4 m, `no_line_of_sight` on open floor, `off_navmesh`): file a bug with the point and `caster_pos`.
Looker `D1-warmup`: 1. Is a cast bar, progress ring or cooldown sweep visible? 2. Is any new object visible near the avatar? Looker `D1-before-cast`: 1. Is the avatar standing in an open paved area? 2. Is the hotbar visible?

## DP-U2 - what it looks like

Evidence is visual. Looker `D1-object-front`: 1. Is there a small object standing on the ground in front of the camera, separate from the avatar? 2. Does it look like a device (a small machine, emitter, box on legs) rather than a character? 3. Does it appear to stand ON the ground (shadow or contact with the floor)? 4. Does it float visibly above the floor? (expected NO) 5. Does it appear sunk into the floor? (expected NO) 6. Is a name plate "Deployable: Microwave Emitter" (or similar) shown? 7. Is it a plain grey/white placeholder cube or a missing-texture shape? Looker `D1-object-side`: questions 1, 3, 4, 5 again from the side view, and 8. Describe its shape and size in one short sentence.
Grade: NEEDS_HUMAN (owner question Q-DP1: is this the right look?). Record the looker's description as the pre-screen: PASS(looker) only when questions 1, 3 are YES and 4, 5, 7 are NO.

## DP-U4 - it vanishes after 30 s

| Clause | Query | Expected |
|---|---|---|
| Lifetime | `ch ... "scope_name='deployables.lifecycle' AND $P AND (attributes_string['event']='despawned' OR body LIKE '%despawned%')"` | INFO `despawned`, `reason`=`expired`, `lifetime_secs` about 30 (29-31), `pulses`=30 |
| Pulses | `ch ... "scope_name='deployables.pulse' AND $P"` | 30 `pulse` rows, `pulse_index` 1..30 (DEBUG; may be missing) |
| Client (client-read) | driver 4.1 | listed at about +20 s, gone at about +35 s |

Grade: PASS (or NATIVE_SHORTFALL as above) if `despawned expired` is within 30 +-2 s of `spawned` and the client agrees. FAIL if it lasts more than 33 s or goes early without a reason.
Looker `D1-after-30s`: 1. Is the small object gone from where it stood? 2. Is the avatar still standing in the plaza?

---

## DRIVER REPORT

(The coordinator pastes the driver's final report here, with the screenshot paths.)
