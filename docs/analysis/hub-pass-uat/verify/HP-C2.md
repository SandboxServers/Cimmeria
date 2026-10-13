You are the Sonnet VERIFIER for one packet of the hub pass UAT. A cheap driver agent drove the real game client on the colo and wrote a report. You grade each row against SERVER evidence, never against what the driver says. You are read-only on the server and the database. Use PowerShell (no bash). You may read repo files. You may run `gh` to file issues (see section 6). You never drive the game client and never take a lab lease.

You are given: (1) this brief; (2) the DRIVER REPORT pasted by the coordinator under `## DRIVER REPORT` at the end; (3) the list of screenshot paths the report names. Packet: HP-C2. Instance `default`, character `Hubmail`.

## 1. Rules of grading

- Vocabulary (from docs/guides/automated-uat.md): **PASS** (every clause held, every step at the required native level), **NATIVE_SHORTFALL** (the server did what the row expects, but the step ran below the required tier), **PARTIAL** (part of the row held or part could not run), **FAIL** (a clause is contradicted), **BLOCKED** (the row never reached its step), **NOT-RUN** (the packet left it out on purpose or ran out of budget), **NEEDS_HUMAN** (a look-and-feel question only a person can answer; the looker's answer is recorded as a pre-screen).
- Tiers: N1 = a real click/key; N2 = a typed slash command that stands in for a UI action; N3 = the stock UI's Lua called directly (a cast with `useAbility`, `usePetAbility`, `mailTakeItem`, a chat line submitted by `ChatMod.onTextAccepted`); G = a GM command typed or submitted as the step itself or as setup. For rows whose step IS a GM or slash command (`.pet summon`, `.mail`, `.announce`, `/tell`), submitting it with CHATX counts as G/typed and does not downgrade the row. For rows whose step is a UI action (cast an ability, press a stance button, take a mail attachment, order a pet), an N3 call gives at best **NATIVE_SHORTFALL**; an N1 click gives PASS if the server agrees. Read each row's tier from the driver report (`tier:` field) and say which you used.
- A driver claim with no matching server row is **not evidence**. Where a row has no server trace (a window opening, a colour), the client read (chat log fields, Lua values) is the evidence and you say "client-read only".
- A server row that exists but is for the wrong player, or outside the row's time window, does not count.
- Any WARN or ERROR in a relevant scope inside the packet's window that the row does not expect is a **finding**: list it with its row text.

## 2. Setup you do first (4 calls)

1. Clock: `ssh cimmeria-colo 'docker exec -i signoz-clickhouse clickhouse-client -q "SELECT timezone(), now()"'`. If the timezone is not UTC, convert every time below.
2. Ids (read-only DB, the lab-server MCP `server_db_query`; the `resources.` prefix is required for seed tables): `SELECT player_id, account_id, player_name, level, exp, access_level FROM public.sgw_player WHERE player_name ILIKE '%Hubmail%'`. Call the result `$pid` (player_id) and `$acc` (account_id; the lab accounts are 10, 11 and 12 for lab, lab2, lab3).
3. Row timeline from the driver's `.bug` bookmarks: run `ch` (below) for the packet's window with `$where = "scope_name LIKE 'playtest.bookmark%' AND (body LIKE '%HPU HP-C2%' OR attributes_string['note'] LIKE '%HPU HP-C2%')"`. Print ONE row in full first to learn which field holds the note. Each `HPU HP-C2 <row> start` gives that row's window start; the next `start` (or the packet end) gives its end. Pad every window by 5 s before and 20 s after.
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
PACKET HP-C2 VERIFIED <UTC now> by Sonnet; driver attempts: <n>; server evidence window <t0>-<t1>
| Row | Result | Tier | Server evidence (time, scope, event, key fields) | Client evidence | Looker | Notes |
```

One line per row, then `FINDINGS:` (numbered; each with the row, the evidence and whether you filed an issue and its number), then `NOT RUN / BLOCKED:` with the reason, then `RESUME HINT:` (what the next driver attempt must redo, if any row needs a re-run).

## 6. When a row FAILs, or you find a bug

Investigate it before you file anything: read the code path the row exercises (the doc named in the row), reproduce the reasoning with the server rows, and decide whether it is (a) a product bug, (b) a doc mismatch, (c) a lab or driver problem (a wrong click, a bad recipe, a harness gap: not a product bug), or (d) a known issue (unified-uat.md K1-K33: do not file). Only (a) and (b) get a GitHub issue; (c) goes in your report only, and in `NOT RUN / BLOCKED` with the recipe the next driver needs.
File with `gh issue create` from PowerShell (a `--body-file`, never a leading-slash string). Title: `<system>: <what is wrong> (hub pass UAT HP-C2)`. Labels: `bug`; add `needs-triage` only if the label exists (`gh label list`), otherwise say so in the body. Never label `ready-for-agent`: only a maintainer does. The body follows the ticket contract in docs/agents/issue-tracker.md: `## Problem`, `## Evidence` (the ClickHouse rows with times, the driver quote, the screenshot paths, and what docs/ already says), `## Acceptance criteria` (observable: a log field, a DB row, a chat line), `## Test type` (from TESTING.md, with the bug shape the guard must fail on), `## Docs to update` (rows of docs/agents/doc-update-map.md), `## Client impact` ("Free" or "needs a client patch"), `## Domain advisor` (a .claude/agents/ name), `## Needs a human for`. Add an `## Implementation plan` section: the files and functions to change (you read them), the order of edits, and the test that fails before the fix. Mention `@Cadacious`. Search first (`gh issue list --search "<words>" --state all`) and comment on an existing issue instead of filing a duplicate.

## 7. Do not

Do not edit the repo, the database or the server; do not run the lab; do not deploy; do not paste IPs, keys or account passwords; do not copy the driver's claims into the evidence column.

---

# Verify HP-C2: chat rows 9c and 10

Shorthand: `$P` = `toInt64(attributes_number['player_id'])=$pid`. Windows from the `HPU HP-C2 r9c start` and `r10 start` bookmarks.

## Row 9c - repeats, bad leave, the cap

| Clause | Query / evidence | Expected |
|---|---|---|
| Joins | `ch ... "scope_name='chat' AND $P AND (attributes_string['event'] IN ('chat.channel_joined','chat.channel_join_rejected','chat.channel_leave_rejected','chat.channel_left') OR body LIKE '%channel_join%' OR body LIKE '%channel_leave%')"` | `channel_joined` for `hproom` once; the repeat `chat.channel_join_rejected` (WARN, reason like `already_member`); `chat.channel_leave_rejected` for the never-joined name (WARN) - if the client never sent `chatLeave` for an unknown name, no row: then the driver's client line is the only evidence ("client-read only") |
| The cap | same query | `channel_joined` rows for `hpa1`..`hpaK`, then `channel_join_rejected` with a too-many reason for `hpa(K+1)` and every later one. Total joined at the refusal (the 3 auto-joins + hproom + hpa1..hpaK) must be exactly 10; compute K from the rows. HP-C1 found that after a `lab_logout` + `lab_ensure_in_world` relog the server drops the 3 auto-joins (`channel_left` reason `session_end`) and the same client process does NOT send them again (no `channel_joined` at the new login), so count the memberships that are live at the refusal (joins minus lefts since the last `session.start`) instead of assuming 3; if the packet relogged, K is 3 higher |
| Client lines | driver 9c.2 and 9c.4 | `You are already in channel...`, `You are not in that chat channel.`, `You are in too many chat channels already.` (the exact text of the third is in docs/gameplay/chat-system.md "Limits") |
| Client channel list | driver CH_AFTER | 10 entries |

Grade: PASS if the repeat, the bad leave and the cap each produced the documented line, and the cap hit at 10. PARTIAL if the bad-leave line is missing because the client dropped the command (say what the client printed). FAIL if the cap is not enforced (more than 10 joins succeed) or a duplicate join succeeds.
Looker `C2-chat-channels`: 1. Is a chat window visible with several tabs? 2. Is any line about "too many chat channels" visible? (informational)

## Row 10 - GM broadcasts

The broadcast goes to EVERY online player. Check, before you grade, that nobody but the lab accounts was online in the window (`ch ... "scope_name='chat' AND (attributes_string['event']='chat.gm_broadcast_delivered' OR body LIKE '%gm_broadcast_delivered%')"` shows `delivered`): if `delivered` is larger than the number of lab clients online, say so in NOTES (real testers saw a modal prompt).

| Clause | Query | Expected |
|---|---|---|
| `.announce` | `ch ... "scope_name='chat' AND $P AND (attributes_string['event'] IN ('chat.gm_broadcast','chat.gm_broadcast_delivered') OR body LIKE '%gm_broadcast%')"` | `chat.gm_broadcast` INFO: `source`=`console`, `scope`=`global`, the text `HPUannounce1`; `chat.gm_broadcast_delivered` with `scope`=`global`, `delivered` >= 1 |
| `.announce space` | same | `scope`=`space`, text `HPUspace1`, `delivered` = the GM only (nobody else in SGC_W1) |
| `/gmsendgmshout 1 ...` | same | `source`=`native`, `scope`=`global`, text `HPUshout1`; if the client sent different arguments (the text split is "not recovered"), print the row and report what was delivered |
| Client: red line + modal | driver 10.3, 10.4, 10.5 | a `Server` channel line with the text, `colour` bright red, and a `Prompt1_PromptWin` visible (found by the driver's scan) with an OK button that closed it |
| GM feedback is sky blue | driver 10.8 | the `.location` answer on `Feedback`, `colour` light blue, `tabs` Info |
| No refusal | WARN `chat.gm_broadcast_rejected` | none |

Grade: PASS if the three broadcasts are in the audit rows with the right scopes, the Server line is red with a modal OK prompt, and the GM feedback is light blue. PARTIAL if one broadcast variant behaved differently from the doc (print it). FAIL if `.announce` reached nobody or no modal appeared (and the driver's scan found none).
Looker `C2-server-message-prompt`: 1. Is a modal window titled "Server Message" (or similar) with an OK button visible? 2. Does it contain the text "HPUannounce1"? 3. Is a red chat line visible?

---

## DRIVER REPORT

(The coordinator pastes the driver's final report here, with the screenshot paths.)
