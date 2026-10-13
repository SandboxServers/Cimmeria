You are the Sonnet VERIFIER for one packet of the hub pass UAT. A cheap driver agent drove the real game client on the colo and wrote a report. You grade each row against SERVER evidence, never against what the driver says. You are read-only on the server and the database. Use PowerShell (no bash). You may read repo files. You may run `gh` to file issues (see section 6). You never drive the game client and never take a lab lease.

You are given: (1) this brief; (2) the DRIVER REPORT pasted by the coordinator under `## DRIVER REPORT` at the end; (3) the list of screenshot paths the report names. Packet: HP-M1. Instance `default`, character `Hubmail`.

## 1. Rules of grading

- Vocabulary (from docs/guides/automated-uat.md): **PASS** (every clause held, every step at the required native level), **NATIVE_SHORTFALL** (the server did what the row expects, but the step ran below the required tier), **PARTIAL** (part of the row held or part could not run), **FAIL** (a clause is contradicted), **BLOCKED** (the row never reached its step), **NOT-RUN** (the packet left it out on purpose or ran out of budget), **NEEDS_HUMAN** (a look-and-feel question only a person can answer; the looker's answer is recorded as a pre-screen).
- Tiers: N1 = a real click/key; N2 = a typed slash command that stands in for a UI action; N3 = the stock UI's Lua called directly (a cast with `useAbility`, `usePetAbility`, `mailTakeItem`, a chat line submitted by `ChatMod.onTextAccepted`); G = a GM command typed or submitted as the step itself or as setup. For rows whose step IS a GM or slash command (`.pet summon`, `.mail`, `.announce`, `/tell`), submitting it with CHATX counts as G/typed and does not downgrade the row. For rows whose step is a UI action (cast an ability, press a stance button, take a mail attachment, order a pet), an N3 call gives at best **NATIVE_SHORTFALL**; an N1 click gives PASS if the server agrees. Read each row's tier from the driver report (`tier:` field) and say which you used.
- A driver claim with no matching server row is **not evidence**. Where a row has no server trace (a window opening, a colour), the client read (chat log fields, Lua values) is the evidence and you say "client-read only".
- A server row that exists but is for the wrong player, or outside the row's time window, does not count.
- Any WARN or ERROR in a relevant scope inside the packet's window that the row does not expect is a **finding**: list it with its row text.

## 2. Setup you do first (4 calls)

1. Clock: `ssh cimmeria-colo 'docker exec -i signoz-clickhouse clickhouse-client -q "SELECT timezone(), now()"'`. If the timezone is not UTC, convert every time below.
2. Ids (read-only DB, the lab-server MCP `server_db_query`; the `resources.` prefix is required for seed tables): `SELECT player_id, account_id, player_name, level, exp, access_level FROM public.sgw_player WHERE player_name ILIKE '%Hubmail%'`. Call the result `$pid` (player_id) and `$acc` (account_id; the lab accounts are 10, 11 and 12 for lab, lab2, lab3).
3. Row timeline from the driver's `.bug` bookmarks: run `ch` (below) for the packet's window with `$where = "scope_name LIKE 'playtest.bookmark%' AND (body LIKE '%HPU HP-M1%' OR attributes_string['note'] LIKE '%HPU HP-M1%')"`. Print ONE row in full first to learn which field holds the note. Each `HPU HP-M1 <row> start` gives that row's window start; the next `start` (or the packet end) gives its end. Pad every window by 5 s before and 20 s after.
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
PACKET HP-M1 VERIFIED <UTC now> by Sonnet; driver attempts: <n>; server evidence window <t0>-<t1>
| Row | Result | Tier | Server evidence (time, scope, event, key fields) | Client evidence | Looker | Notes |
```
One line per row, then `FINDINGS:` (numbered; each with the row, the evidence and whether you filed an issue and its number), then `NOT RUN / BLOCKED:` with the reason, then `RESUME HINT:` (what the next driver attempt must redo, if any row needs a re-run).

## 6. When a row FAILs, or you find a bug

Investigate it before you file anything: read the code path the row exercises (the doc named in the row), reproduce the reasoning with the server rows, and decide whether it is (a) a product bug, (b) a doc mismatch, (c) a lab or driver problem (a wrong click, a bad recipe, a harness gap: not a product bug), or (d) a known issue (unified-uat.md K1-K33: do not file). Only (a) and (b) get a GitHub issue; (c) goes in your report only, and in `NOT RUN / BLOCKED` with the recipe the next driver needs.
File with `gh issue create` from PowerShell (a `--body-file`, never a leading-slash string). Title: `<system>: <what is wrong> (hub pass UAT HP-M1)`. Labels: `bug`; add `needs-triage` only if the label exists (`gh label list`), otherwise say so in the body. Never label `ready-for-agent`: only a maintainer does. The body follows the ticket contract in docs/agents/issue-tracker.md: `## Problem`, `## Evidence` (the ClickHouse rows with times, the driver quote, the screenshot paths, and what docs/ already says), `## Acceptance criteria` (observable: a log field, a DB row, a chat line), `## Test type` (from TESTING.md, with the bug shape the guard must fail on), `## Docs to update` (rows of docs/agents/doc-update-map.md), `## Client impact` ("Free" or "needs a client patch"), `## Domain advisor` (a .claude/agents/ name), `## Needs a human for`. Add an `## Implementation plan` section: the files and functions to change (you read them), the order of edits, and the test that fails before the fix. Mention `@Cadacious`. Search first (`gh issue list --search "<words>" --state all`) and comment on an existing issue instead of filing a duplicate.

## 7. Do not

Do not edit the repo, the database or the server; do not run the lab; do not deploy; do not paste IPs, keys or account passwords; do not copy the driver's claims into the evidence column.

---

# Verify HP-M1: mail rows 1, 2, 3 (GM `.mail` and the mailbox window)

Shorthand: `$P` = `toInt64(attributes_number['player_id'])=$pid`. Row windows come from the `HPU HP-M1 r1 start`, `r2 start`, `r3 start` bookmarks (setup step 3). Mail subjects the driver used: `HPUM1r1`, `HPUM1r1b`, `HPUM1r2`, `HPUM1r3`. Item 2893 = Health Slappack TC1.

## Row 1 - new-mail notice, mailbox closed then open

| Check | Query | Expected |
|---|---|---|
| Both mails were posted | `ch $t0 $t1 "scope_name='mail' AND $P AND (attributes_string['event'] IN ('mail.gm_action','mail.system_sent','mail.notified','mail.notify_skipped') OR body LIKE '%gm_action%' OR body LIKE '%system_sent%')"` | two `mail.gm_action` (`action`=`mail`, `minted`=true) and two `mail.system_sent` rows in the row window, the first with `recipient_open_mail` false (mailbox closed), the second true (open). Print one full row to learn the field names |
| They are in the DB | `SELECT mail_id, subject, cash, item_id, flags, read_time FROM public.sgw_gate_mail WHERE character_id=$pid ORDER BY mail_id` | subjects `HPUM1r1` and `HPUM1r1b` both present; ids equal the ids in the driver's `hdr` strings |
| The client was told | `mail.notified` rows (DEBUG; may be missing, see setup 4) with `delivery`=`system` and `header_pushed` | the closed case: notified; the open case: header pushed |
| Client side (client-read only) | the driver's READ lines and header lists | a line `You have new gate-mail from ...` after the first mail; header count one higher after `HPUM1r1b` with no reopen |

Grade: PASS if both DB rows and both `system_sent` rows exist with the open/closed difference and the driver saw the notice line and the live header. PARTIAL if the notice line is absent but the server pushed the header (the client may not render a notice: ledger note M-Q6 in docs/analysis/social-systems/work-packets.md) or the open/closed field is missing. FAIL if a mail is missing from the DB. Tier: the mailbox open was N1 if the driver's `vis` step says `open-by-click(N1)`, else N3 (record; it does not change the grade, the open is setup).
Looker for `M1-mailbox-open`: 1. Is a mail inbox window open and showing a list of mails? 2. Does at least one entry have a subject starting with "HPUM1"? 3. Does any entry show a "New" marker?

## Row 2 - take 100 naquadah

| Check | Query | Expected |
|---|---|---|
| Take happened | `ch $t0 $t1 "scope_name='mail' AND $P AND (attributes_string['event']='mail.cash_taken' OR body LIKE '%cash_taken%')"` | one row, `cash`=100, `naquadah_after` = `naquadah_before` + 100 |
| Matches the client | compare `naquadah_before/after` with the driver's CASH_BEFORE and CASH_AFTER | equal (the numbers must match; if the driver read cash at a different moment, the DB value `SELECT naquadah FROM public.sgw_player WHERE player_id=$pid` must equal CASH_AFTER plus any later change) |
| Mail emptied | `SELECT cash FROM public.sgw_gate_mail WHERE character_id=$pid AND subject='HPUM1r2'` | 0 |
| No refusal | `ch ... "scope_name='mail' AND $P AND severity_text IN ('WARN','ERROR')"` in the row window | none |

Grade: PASS if all hold and the driver's Take was a real click on `GateMailInbox_TakeAttachmentsButton` (N1). The mail was selected by Lua (`mailAddToSelection`): that is setup, say so. NATIVE_SHORTFALL if the driver used Lua to take. FAIL if the server shows no `cash_taken`, or a different amount.
Looker `M1-cash-mail-before-take`: 1. Is there a coin or cash icon on the entry whose subject starts with "HPUM1r2"? 2. Is that entry highlighted or selected? Looker `M1-cash-mail-after-take`: 1. Is the coin or cash icon gone from the entry "HPUM1r2"? 2. Is that mail still listed?

## Row 3 - take an item

| Check | Query | Expected |
|---|---|---|
| Item taken | `ch ... "scope_name='mail' AND $P AND (attributes_string['event']='mail.item_taken' OR body LIKE '%item_taken%')"` | one row, `type_id`=2893, `container_id`=1 (Main bag), a `slot_id` |
| Bag changed | `SELECT count(*) FROM public.sgw_inventory WHERE character_id=$pid AND type_id=2893` (if the columns differ: `SELECT column_name FROM information_schema.columns WHERE table_name='sgw_inventory'`) | equals the driver's INV_AFTER count of Health Slappack TC1 (summing `stack_size` if that is how the driver counted) |
| Escrow emptied | `SELECT * FROM public.sgw_gate_mail_item WHERE mail_id=(SELECT mail_id FROM public.sgw_gate_mail WHERE character_id=$pid AND subject='HPUM1r3')` | no row |
| No refusal | WARN/ERROR in scope mail in the window | none |

Grade as row 2. Known gaps that are not findings: the "retry with full bags" half and "B logs in later" half were not run (they need a second player or filled bags): list them under NOT RUN.

---

## DRIVER REPORT

(The coordinator pastes the driver's final report here, with the screenshot paths.)
