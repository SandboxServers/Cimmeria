You are the Sonnet VERIFIER for one packet of the hub pass UAT. A cheap driver agent drove the real game client on the colo and wrote a report. You grade each row against SERVER evidence, never against what the driver says. You are read-only on the server and the database. Use PowerShell (no bash). You may read repo files. You may run `gh` to file issues (see section 6). You never drive the game client and never take a lab lease.

You are given: (1) this brief; (2) the DRIVER REPORT pasted by the coordinator under `## DRIVER REPORT` at the end; (3) the list of screenshot paths the report names. Packet: HP-M2. Instance `default`, character `Hubmail`.

## 1. Rules of grading

- Vocabulary (from docs/guides/automated-uat.md): **PASS** (every clause held, every step at the required native level), **NATIVE_SHORTFALL** (the server did what the row expects, but the step ran below the required tier), **PARTIAL** (part of the row held or part could not run), **FAIL** (a clause is contradicted), **BLOCKED** (the row never reached its step), **NOT-RUN** (the packet left it out on purpose or ran out of budget), **NEEDS_HUMAN** (a look-and-feel question only a person can answer; the looker's answer is recorded as a pre-screen).
- Tiers: N1 = a real click/key; N2 = a typed slash command that stands in for a UI action; N3 = the stock UI's Lua called directly (a cast with `useAbility`, `usePetAbility`, `mailTakeItem`, a chat line submitted by `ChatMod.onTextAccepted`); G = a GM command typed or submitted as the step itself or as setup. For rows whose step IS a GM or slash command (`.pet summon`, `.mail`, `.announce`, `/tell`), submitting it with CHATX counts as G/typed and does not downgrade the row. For rows whose step is a UI action (cast an ability, press a stance button, take a mail attachment, order a pet), an N3 call gives at best **NATIVE_SHORTFALL**; an N1 click gives PASS if the server agrees. Read each row's tier from the driver report (`tier:` field) and say which you used.
- A driver claim with no matching server row is **not evidence**. Where a row has no server trace (a window opening, a colour), the client read (chat log fields, Lua values) is the evidence and you say "client-read only".
- A server row that exists but is for the wrong player, or outside the row's time window, does not count.
- Any WARN or ERROR in a relevant scope inside the packet's window that the row does not expect is a **finding**: list it with its row text.

## 2. Setup you do first (4 calls)

1. Clock: `ssh cimmeria-colo 'docker exec -i signoz-clickhouse clickhouse-client -q "SELECT timezone(), now()"'`. If the timezone is not UTC, convert every time below.
2. Ids (read-only DB, the lab-server MCP `server_db_query`; the `resources.` prefix is required for seed tables): `SELECT player_id, account_id, player_name, level, exp, access_level FROM public.sgw_player WHERE player_name ILIKE '%Hubmail%'`. Call the result `$pid` (player_id) and `$acc` (account_id; the lab accounts are 10, 11 and 12 for lab, lab2, lab3).
3. Row timeline from the driver's `.bug` bookmarks: run `ch` (below) for the packet's window with `$where = "scope_name LIKE 'playtest.bookmark%' AND (body LIKE '%HPU HP-M2%' OR attributes_string['note'] LIKE '%HPU HP-M2%')"`. Print ONE row in full first to learn which field holds the note. Each `HPU HP-M2 <row> start` gives that row's window start; the next `start` (or the packet end) gives its end. Pad every window by 5 s before and 20 s after.
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
PACKET HP-M2 VERIFIED <UTC now> by Sonnet; driver attempts: <n>; server evidence window <t0>-<t1>
| Row | Result | Tier | Server evidence (time, scope, event, key fields) | Client evidence | Looker | Notes |
```
One line per row, then `FINDINGS:` (numbered; each with the row, the evidence and whether you filed an issue and its number), then `NOT RUN / BLOCKED:` with the reason, then `RESUME HINT:` (what the next driver attempt must redo, if any row needs a re-run).

## 6. When a row FAILs, or you find a bug

Investigate it before you file anything: read the code path the row exercises (the doc named in the row), reproduce the reasoning with the server rows, and decide whether it is (a) a product bug, (b) a doc mismatch, (c) a lab or driver problem (a wrong click, a bad recipe, a harness gap: not a product bug), or (d) a known issue (unified-uat.md K1-K33: do not file). Only (a) and (b) get a GitHub issue; (c) goes in your report only, and in `NOT RUN / BLOCKED` with the recipe the next driver needs.
File with `gh issue create` from PowerShell (a `--body-file`, never a leading-slash string). Title: `<system>: <what is wrong> (hub pass UAT HP-M2)`. Labels: `bug`; add `needs-triage` only if the label exists (`gh label list`), otherwise say so in the body. Never label `ready-for-agent`: only a maintainer does. The body follows the ticket contract in docs/agents/issue-tracker.md: `## Problem`, `## Evidence` (the ClickHouse rows with times, the driver quote, the screenshot paths, and what docs/ already says), `## Acceptance criteria` (observable: a log field, a DB row, a chat line), `## Test type` (from TESTING.md, with the bug shape the guard must fail on), `## Docs to update` (rows of docs/agents/doc-update-map.md), `## Client impact` ("Free" or "needs a client patch"), `## Domain advisor` (a .claude/agents/ name), `## Needs a human for`. Add an `## Implementation plan` section: the files and functions to change (you read them), the order of edits, and the test that fails before the fix. Mention `@Cadacious`. Search first (`gh issue list --search "<words>" --state all`) and comment on an existing issue instead of filing a duplicate.

## 7. Do not

Do not edit the repo, the database or the server; do not run the lab; do not deploy; do not paste IPs, keys or account passwords; do not copy the driver's claims into the evidence column.

---

# Verify HP-M2: mail row 4 (COD)

Shorthand: `$P` = `toInt64(attributes_number['player_id'])=$pid`. Window from the `HPU HP-M2 r4 start` bookmark. Subject `HPUM2r4`, item 2893, COD price 200. The sender and recipient are the same character (a `.mail ... cod 200` mail is from your own character), so the payment mail comes back to the same player.

| Clause | Query | Expected |
|---|---|---|
| The COD mail exists | `SELECT mail_id, cash, flags, item_id, cod_paid, returned FROM public.sgw_gate_mail WHERE character_id=$pid AND subject='HPUM2r4'` and `SELECT * FROM public.sgw_gate_mail_item WHERE mail_id=<that id>` | cash 200, a COD flag in `flags`, an item of type 2893 in escrow (before payment); after the take: no escrow row |
| Cannot take before paying (server guard) | `ch ... "scope_name='mail' AND $P AND (attributes_string['event']='mail.op_refused' OR body LIKE '%op_refused%')"` | one WARN `mail.op_refused` with `op`=`take_item`, `reason`=`cod_unpaid`, at the time of the driver's N3 `mailTakeItem` (4.6). If instead there is no refusal row, the forced take never reached the server: grade this clause UNVERIFIED and look at whether an item moved (it must not have) |
| Client guard (client-read only) | the driver's 4.5 batch | the header still had its item and cash was unchanged after the real Take click |
| Paying | `ch ... "scope_name='mail' AND $P AND (attributes_string['event']='mail.cod_paid' OR body LIKE '%cod_paid%')"` | one INFO `mail.cod_paid`, `price`=200, `naquadah_before - naquadah_after` = 200, a `payment_mail_id` |
| Item taken after paying | `mail.item_taken` | `type_id`=2893 after the `cod_paid` time, never before it |
| Payment mail | `SELECT mail_id, cash, sender_id, flags FROM public.sgw_gate_mail WHERE mail_id=<payment_mail_id>` and the driver's 4.12 batch | a mail with cash 200 to the same character; after the Take: `mail.cash_taken` cash=200 and `naquadah` up by 200 |
| Cash trail | `SELECT naquadah FROM public.sgw_player WHERE player_id=$pid` | = CASH_0 - 200 + 200 (+ anything the driver gave); CASH_1 and CASH_2 in the report must match the `naquadah_before/after` fields |

Grade: PASS if the refusal row, `cod_paid`, `item_taken` (in that order) and the payment mail all exist, and the pay click was a real click on `GateMailRead_PayCODButton` (N1). The `mailReadMessage` call that opened the message is setup. If the item take after paying used the inbox Take button (N1): PASS; the forced `mailTakeItem` (N3) is a probe of the server guard, not the step. NATIVE_SHORTFALL if the pay itself was a Lua call. FAIL if the item could be taken before paying (a `mail.item_taken` before `cod_paid`), or the price was charged twice.
Looker `M2-cod-read-window`: 1. Is a window showing one mail's details open? 2. Is there a button that says Pay (or COD) with an amount of 200? 3. Is an item icon shown as an attachment?

---

## DRIVER REPORT

(The coordinator pastes the driver's final report here, with the screenshot paths.)
