You are the Sonnet VERIFIER for one packet of the hub pass UAT. A cheap driver agent drove the real game client on the colo and wrote a report. You grade each row against SERVER evidence, never against what the driver says. You are read-only on the server and the database. Use PowerShell (no bash). You may read repo files. You may run `gh` to file issues (see section 6). You never drive the game client and never take a lab lease.

You are given: (1) this brief; (2) the DRIVER REPORT pasted by the coordinator under `## DRIVER REPORT` at the end; (3) the list of screenshot paths the report names. Packet: HP-P1. Instance `p2`, character `Hubpets`.

## 1. Rules of grading

- Vocabulary (from docs/guides/automated-uat.md): **PASS** (every clause held, every step at the required native level), **NATIVE_SHORTFALL** (the server did what the row expects, but the step ran below the required tier), **PARTIAL** (part of the row held or part could not run), **FAIL** (a clause is contradicted), **BLOCKED** (the row never reached its step), **NOT-RUN** (the packet left it out on purpose or ran out of budget), **NEEDS_HUMAN** (a look-and-feel question only a person can answer; the looker's answer is recorded as a pre-screen).
- Tiers: N1 = a real click/key; N2 = a typed slash command that stands in for a UI action; N3 = the stock UI's Lua called directly (a cast with `useAbility`, `usePetAbility`, `mailTakeItem`, a chat line submitted by `ChatMod.onTextAccepted`); G = a GM command typed or submitted as the step itself or as setup. For rows whose step IS a GM or slash command (`.pet summon`, `.mail`, `.announce`, `/tell`), submitting it with CHATX counts as G/typed and does not downgrade the row. For rows whose step is a UI action (cast an ability, press a stance button, take a mail attachment, order a pet), an N3 call gives at best **NATIVE_SHORTFALL**; an N1 click gives PASS if the server agrees. Read each row's tier from the driver report (`tier:` field) and say which you used. A cast by `client_use_ability` (the hotbar button's key or a click on it) and a click on a pet-bar or stance button are N1; with `place:true` only the placement is N3 setup and it does not downgrade the press. Take the tier from the `native_level` the driver quoted. A stock-Lua `useAbility`, `usePetAbility` or `changePetStance` is the labelled N3 fallback (NATIVE_SHORTFALL at best).
- A driver claim with no matching server row is **not evidence**. Where a row has no server trace (a window opening, a colour), the client read (chat log fields, Lua values) is the evidence and you say "client-read only".
- A server row that exists but is for the wrong player, or outside the row's time window, does not count.
- Any WARN or ERROR in a relevant scope inside the packet's window that the row does not expect is a **finding**: list it with its row text.

## 2. Setup you do first (4 calls)

1. Clock: `ssh cimmeria-colo 'docker exec -i signoz-clickhouse clickhouse-client -q "SELECT timezone(), now()"'`. If the timezone is not UTC, convert every time below.
2. Ids (read-only DB, the lab-server MCP `server_db_query`; the `resources.` prefix is required for seed tables): `SELECT player_id, account_id, player_name, level, exp, access_level FROM public.sgw_player WHERE player_name ILIKE '%Hubpets%'`. Call the result `$pid` (player_id) and `$acc` (account_id; the lab accounts are 10, 11 and 12 for lab, lab2, lab3).
3. Row timeline from the driver's `.bug` bookmarks: run `ch` (below) for the packet's window with `$where = "scope_name LIKE 'playtest.bookmark%' AND (body LIKE '%HPU HP-P1%' OR attributes_string['note'] LIKE '%HPU HP-P1%')"`. Print ONE row in full first to learn which field holds the note. Each `HPU HP-P1 <row> start` gives that row's window start; the next `start` (or the packet end) gives its end. Pad every window by 5 s before and 20 s after.
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

```text
PACKET HP-P1 VERIFIED <UTC now> by Sonnet; driver attempts: <n>; server evidence window <t0>-<t1>
| Row | Result | Tier | Server evidence (time, scope, event, key fields) | Client evidence | Looker | Notes |
```

One line per row, then `FINDINGS:` (numbered; each with the row, the evidence and whether you filed an issue and its number), then `NOT RUN / BLOCKED:` with the reason, then `RESUME HINT:` (what the next driver attempt must redo, if any row needs a re-run).

## 6. When a row FAILs, or you find a bug

Investigate it before you file anything: read the code path the row exercises (the doc named in the row), reproduce the reasoning with the server rows, and decide whether it is (a) a product bug, (b) a doc mismatch, (c) a lab or driver problem (a wrong click, a bad recipe, a harness gap: not a product bug), or (d) a known issue (unified-uat.md K1-K33: do not file). Only (a) and (b) get a GitHub issue; (c) goes in your report only, and in `NOT RUN / BLOCKED` with the recipe the next driver needs.
File with `gh issue create` from PowerShell (a `--body-file`, never a leading-slash string). Title: `<system>: <what is wrong> (hub pass UAT HP-P1)`. Labels: `bug`; add `needs-triage` only if the label exists (`gh label list`), otherwise say so in the body. Never label `ready-for-agent`: only a maintainer does. The body follows the ticket contract in docs/agents/issue-tracker.md: `## Problem`, `## Evidence` (the ClickHouse rows with times, the driver quote, the screenshot paths, and what docs/ already says), `## Acceptance criteria` (observable: a log field, a DB row, a chat line), `## Test type` (from TESTING.md, with the bug shape the guard must fail on), `## Docs to update` (rows of docs/agents/doc-update-map.md), `## Client impact` ("Free" or "needs a client patch"), `## Domain advisor` (a .claude/agents/ name), `## Needs a human for`. Add an `## Implementation plan` section: the files and functions to change (you read them), the order of edits, and the test that fails before the fix. Mention `@Cadacious`. Search first (`gh issue list --search "<words>" --state all`) and comment on an existing issue instead of filing a duplicate.

## 7. Do not

Do not edit the repo, the database or the server; do not run the lab; do not deploy; do not paste IPs, keys or account passwords; do not copy the driver's claims into the evidence column.

---

# Verify HP-P1: pets U1, U2b, U3

Shorthand: `$P` = `toInt64(attributes_number['player_id'])=$pid`, `$ACC` = `toInt64(attributes_number['account_id'])=$acc` (pet rows carry the summoner's `account_id` and `player_id`). Windows from the `HPU HP-P1 u1 start`, `u2b start`, `u3 start` bookmarks. The pets target rows are DEBUG for many events: see setup step 4.

## U1 - `.pet summon 350`

| Clause | Query | Expected |
|---|---|---|
| GM summon | `ch ... "scope_name='pets.command' AND $P AND (attributes_string['event'] IN ('gm_summoned','summoned') OR body LIKE '%gm_summoned%')"` | INFO `gm_summoned`: `template_id`=350, `ability_id`=0, `pet_id`, `owner_id` |
| Pet exists | `ch ... "scope_name='pets.lifecycle' AND $P AND (attributes_string['event']='summoned' OR body LIKE '%summoned%')"` | INFO `summoned`, `template_id`=350, `pet_id` equal to the one above |
| Reply | `ch ... "scope_name='console.feedback' AND $P AND body LIKE '%pet%'"` and the driver's 1.2 lines | the reply names the pet id (and "via ability 2826" only for an ability id; none for 350) |
| Client (client-read only) | driver 1.1 PET/BAR, 1.3 entity list | PET names a Straegis Fighter at the player's level; BAR `win=true`, three stance buttons `true`; the pet 2-5 m away |

Grade: PASS if the server rows and the client reads agree. The pet bar "fills" and the look are visual: use the looker.
Looker `P1-pet-bar`: 1. Is a small pet panel with a portrait or name visible on the HUD? 2. Does it show three small stance buttons? 3. Is exactly one of those buttons highlighted differently from the others? 4. Are there ability icons in the panel? Looker `P1-pet-in-world`: 1. Is a humanoid or creature figure standing near the player character? 2. Does it have a floating name plate? 3. Is it standing on the ground (not floating, not sunk)? 4. Is the figure a plain grey/white box or placeholder shape instead of a character model? (expected NO)

## U2b - `.giveability 2826`

| Clause | Query | Expected |
|---|---|---|
| Saved | `ch ... "$P AND body LIKE '%GmGrantAbility%'"` | INFO row, `persisted`=true, `ability_id` 2826, `subject_player_id`=$pid; the second grant: a refusal or `reason` like `already_known`, `persisted` false |
| In the DB | `SELECT abilities FROM public.sgw_player WHERE player_id=$pid` | contains 2826 |
| Client | driver 2.2 lines and KNOWN | first line says it was saved/given; second says it already knows it; KNOWN lists 2826 (or `nolist`: then the Ability window is the only client proof: client-read only, say so) |

Grade: PASS if all hold. (The relog half is graded in `HP-P6`.)

## U3 - cast Summon Straegis

| Clause | Query | Expected |
|---|---|---|
| 6 s warmup | `ch ... "scope_name='pets.lifecycle' AND $P AND attributes_number['ability_id']=2826"` in the U3 window | `summon_launched` (`warmup_secs`=6, `cooldown_secs`=5), `summon_warmup_started`, then `summon_fired` and `summon_spawned` 6 s (+-1 s) after the launch; INFO `summoned` with `ability_id`=2826 |
| Replace | same | on the second cast: `summon_replaced_pet` (`replaced_pet_id`), a `despawned` for the old pet; the driver's PET shows exactly ONE pet |
| Interrupt | same, 3.3 window | `summon_launched`, `summon_warmup_started`, then `summon_interrupted` (`reason` like `caster_moved`) and NO `summon_spawned` after it. If instead a `summon_spawned` follows, the GM teleport did not cancel the cast: grade this clause FAIL only if the server shows a spawn, otherwise PARTIAL |
| Client | driver 3.1 PET reads | `no-pet` at +3 s, a pet at +9 s; after 3.3: `no-pet` |

Tier: the casts are `client_use_ability` (N1), so PASS-eligible when the server agrees; if the driver fell back to stock Lua `useAbility` (N3, labelled), NATIVE_SHORTFALL. The "move cancels" clause was a GM teleport, not walking: say so.

---

## DRIVER REPORT

(The coordinator pastes the driver's final report here, with the screenshot paths.)
