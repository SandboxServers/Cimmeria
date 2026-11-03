You are the Sonnet VERIFIER for one packet of the starter UAT (Praxis). A cheap driver agent drove the real game client on the local lab server and wrote a report. You grade each row against SERVER evidence, never against what the driver says. You are read-only on the server and the database. Use PowerShell (no bash). You may read repo files. You never drive the game client and never take a lab lease. You do not run `gh issue create`; on a FAIL you write an issue body file (section 6).

You are given: (1) this brief; (2) the DRIVER REPORT(s) pasted by the coordinator under `## DRIVER REPORT` at the end (one per row run); (3) the screenshot paths the reports name. Packet: SP-DHA1. Instance `default`, characters `Uat Fxdhajaffa`, `Uat Fxdhapraxis`, `Uat Fxdhagoauld`, `Uat Fxdhagoauldb`.

## 1. Rules of grading

- Vocabulary (docs/guides/automated-uat.md): **PASS** (every clause held, every step at the required native level), **NATIVE_SHORTFALL** (the server did what the row expects, but the step ran below the required tier), **PARTIAL** (part of the row held or part could not run), **FAIL** (a clause is contradicted), **BLOCKED** (the row never reached its step; a fixture that could not be staged is BLOCKED, never FAIL), **NOT-RUN** (left out on purpose or out of budget), **NEEDS_HUMAN** (a look-and-feel question only a person can answer).
- **NEEDS_HUMAN never blocks the lane.** The driver took a screenshot and recorded what it saw. Grade the server-checkable clauses of the row normally; list each NEEDS_HUMAN item (leg, question, screenshot path, the driver's one sentence, the looker's answer) in the report under `SIGN-OFF SHEET`, for the person to review at the end. A row whose only open items are NEEDS_HUMAN is graded on its server clauses and carries the note "NEEDS_HUMAN items listed". Never stop to ask anyone.
- Tiers: N1 = a real click/key; N2 = a typed slash command that stands in for a UI action; N3 = the stock UI's Lua called directly; G = a GM command typed as the step itself or as setup. Teleports (`.gotolocation`) and `.bug` are setup (G) and do not downgrade a row. A right-click on an NPC through `client_world_click` is N1. Dialog paging through `lab_finish_dialog` is N1. Using the disguise through `client_item_action` (H41 leg 2) is N3: NATIVE_SHORTFALL for that leg unless the driver clicked in the bag. Read each leg's `tier:` from the driver report.
- A driver claim with no matching server row is **not evidence**. Where a row has no server trace (an icon, a window), the client read is the evidence and you say "client-read only".
- A server row for the wrong player or outside the leg's time window does not count.
- Any WARN or ERROR in a relevant scope inside the window that the row does not expect is a **finding**.

## 2. Setup you do first (4 calls)

1. Clock: `ssh cimmeria-colo 'docker exec -i signoz-clickhouse clickhouse-client -q "SELECT timezone(), now()"'` is the colo; the starter UAT runs on the LOCAL lab server, so first check which SigNoz you reach: if the local lab telemetry is not in that ClickHouse, read the local server log with the lab-server MCP `server_log_tail` and filter by the same words, and say so in the report. If the timezone is not UTC, convert every time below.
2. Ids (lab-server MCP `server_db_query`; `resources.` prefix for seed tables): `SELECT player_id, account_id, player_name, access_level FROM public.sgw_player WHERE player_name ILIKE '%Fxdha%'`. Call each result `$pid` for its row's character.
3. Row timeline from the driver's `.bug` bookmarks: run `ch` (below) with `$where = "scope_name LIKE 'playtest.bookmark%' AND (body LIKE '%SPU SP-DHA1%' OR attributes_string['note'] LIKE '%SPU SP-DHA1%')"`. Print ONE row in full first to learn which field holds the note. `SPU SP-DHA1 <row> start` opens a window, `<row> end` closes it. Pad every window by 5 s before and 20 s after.
4. Mission state, any time: `SELECT column_name FROM information_schema.columns WHERE table_name ILIKE '%mission%' AND table_schema='public'` to find the per-player mission table, then read the row for `$pid` and the mission id. Use it to confirm the step and status at the end of each leg.
5. If the log source returns nothing at all, grade every server clause UNVERIFIED and stop.

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

Times are `'YYYY-MM-DD HH:MM:SS'` UTC. Player filter `$P = toInt64(attributes_number['player_id'])=$pid`. Chain-event words used below: `fire_interact_tag: matched`, `fire_dialog_choice`, `fire_player_loaded`, `fire_mission_abandoned: matched`, `fire_mission_completed`, `Mission abandoned`, `Content: adding dialog set`, `Content: removing dialog set`, `advance_step`, `accept_mission`, `complete_mission`. If the exact wording differs, print one matching row and adapt; the chain id (6301 etc.) is the stable key. If a query errors with BAD_GET, wrap numeric attribute reads in `toInt64()`.

## 4. The looker (visual checks)

For each screenshot the report names, start a Haiku looker: Agent tool, `subagent_type: general-purpose`, `model: haiku`, prompt = `Read the image file <PATH> (use the Read tool). Answer each question with YES, NO or UNSURE and at most 12 words of what you see. Do not guess beyond what is visible. Questions: 1. ... 2. ...`. Record the answers verbatim. A NO or UNSURE on a question that a row depends on makes that row at best PARTIAL (visual), never FAIL by itself. These answers are a pre-screen; the person still reviews NEEDS_HUMAN screenshots.

## 5. Report format (your final message, and the results file)

Write `docs/analysis/starter-uat-praxis/results/SP-DHA1.md`:

```text
PACKET SP-DHA1 VERIFIED <UTC now> by Sonnet; driver attempts: <n>; server evidence window <t0>-<t1>
| Row | Grade | Tier | Server evidence | Screenshot | Issue |
```

One line per row (H20, H31, H40, H41), then `## Not run` (legs with reason), `## Resume hint` (what the next driver attempt must redo, with the START LEG and its fixture row from the brief), `## Recipe` (the exact calls that passed, for SP-92), `## SIGN-OFF SHEET` (NEEDS_HUMAN items), and `## Findings` (numbered; each with row, evidence, and the issue body file if written).

## 6. When a row FAILs, or you find a bug

Investigate first: read the chain in `db/resources/Content/Seed/harset_jaffa_chains.sql`, `harset_opcore_chains.sql` or `harset_goauld_chains.sql`, reproduce the reasoning with the server rows, and decide: (a) product bug, (b) doc mismatch, (c) lab or driver problem (wrong click, bad recipe, harness gap: report only, never an issue), (d) known issue. For (a) and (b) write the issue body to `docs/analysis/starter-uat-praxis/results/issues/SP-DHA1-<n>.md` (do not run `gh issue create`; the coordinator files it). Sections: `## Problem`, `## Evidence` (log rows with times, the driver quote, screenshot paths, what docs/ says), `## Acceptance criteria`, `## Test type`, `## Docs to update`, `## Client impact`, `## Domain advisor`, `## Needs a human for`, `## Implementation plan`. Title line first: `<system>: <what is wrong> (starter UAT SP-DHA1)`. Mention `@Cadacious`.

## 7. Do not

Do not edit the repo beyond the results file and issue files, the database or the server; do not run the lab; do not deploy; do not paste IPs, keys or account passwords; do not copy the driver's claims into the evidence column.

---

# Verify SP-DHA1: Harset missions 1324, 1361, 1200, 742

Shorthand: `$P` as above. Chains are in `db/resources/Content/Seed/`.

## Row H20 - mission 1324 *Present Yourself* (Jaffa; chains 6301, 6303-6308; character `Uat Fxdhajaffa`)

| Check | Query | Expected |
|---|---|---|
| Offer bound at load | `ch ... "$P AND body LIKE '%6301%'"` or `Content: adding dialog set` with dialog set 5149 | chain 6301 fired on the world-68 load; `add_dialog_set 5149` for slot 54 (Moh'katan) |
| Accept | chain 6303: `accept_mission 1324`, `display_dialog 4358`, `remove_dialog_set 5149`, `add_dialog_set 5151` slot 42 | one pass at the first accept; mission 1324 `active` at step 3953 |
| Click range | the driver's `RCLICK` `range` | at most 5.0 m, and no `interaction.out_of_range` WARN for the leg |
| Abandon | chain 6308 `mission_abandoned '1324'`: removes 5151 and 120001, `add_dialog_set 5149` slot 54 | one pass; mission no longer active; the next Moh'katan click shows the offer again (client-read: dialog title) |
| Re-accept | chain 6303 a second time | again step 3953 active |
| Ba'al | chain 6304 `interact_tag CmdCenter_Baal` at step 3953: `display_dialog 4363`, `advance_step 1324 3954`, remove 5151, add dsm 120001 slot 54 | one pass, step 3954 |
| Turn in | chain 6305 `interact_tag CmdCenter_Mohkatan` at 3954: `display_dialog 4365`, remove 120001, `complete_mission 1324` | one pass; mission completed (mission table) |
| Positions | `server_entity_query` or `server_entity_get` for template 54 and 42 in world 68 | Moh'katan at about (-5, 0.37, -8), Ba'al at about (2, 1.95, 88) (placements.md spawns 343 and 340) |
| No stray chain | WARN/ERROR in scopes `content` or `interaction` in the window | none |

Grade: PASS if every row above holds and the clicks were N1. FAIL if a chain does not fire for an in-range click, or the mission state contradicts. BLOCKED if the fixture could not be staged. Known gap, not a finding: the 1324 level gate (level 6) is not in the chains; the fixture does not set a level, and the verifier does not fail the row for it. Chain 6302 is intentionally absent. The completion follow-up 1326 (chain 6331) needs 1325 and is not offered.
Looker `H20-moh-offer`: 1. Is there a "?" or yellow icon over an NPC? 2. Is an NPC standing near the player? Looker `H20-complete`: 1. Is a dialog window closed? 2. Is an NPC visible?

## Row H31 - mission 1361 *Meet The Praxis* (chains 6511-6528; character `Uat Fxdhapraxis`)

| Leg | Chain evidence | Mission step after |
|---|---|---|
| 1 offer and accept | 6511 at load (`add_dialog_set 5254` slot 10, mission 1361); 6512 shows dialog 4457; 6513 `dialog_choice 4457` accepts 1361 | 4040 |
| 2 Moh'katan | 6515 `interact_tag CmdCenter_Mohkatan` at 4040 | 4041 |
| 3 Hansen | 6516 `player_loaded 'Harset'` at 4041 binds 6399 slot 212; 6518 `dialog_choice 4459`, display 4460 | 4042 |
| 4 Moh'katan | 6520 (dialog 4466) | 4043 |
| 5 Ba'al | 6522 (dialog 4461) | 4693 |
| 6 Anat | 6525 `dialog_choice 4462`, display 4463 | 4694 |
| 7 Marsh | 6527 (dialog 4465), `complete_mission 1361` | completed |

Also check: (a) **enablement tied to 6007**: the 6511 offer exists for a non-Jaffa, non-Goa'uld Praxis character in world 68 with 1360 not at 4038 and 1664 not at 5226 (the fixture); the chain conditions are `archetype neq 8`, `neq 6`; (b) **binds**: each restore bind (6514, 6519, 6521, 6523, 6526) is only checked if the driver relogged, which this packet does not; list them under NOT RUN; (c) **disjointness from 1360**: while the character has 1360 at 4038 the 6511 offer must not appear. This packet's character has 1360 `not_active`, so check only that chain 6501/6502 did NOT fire for it (`ch ... "$P AND (body LIKE '%6501%' OR body LIKE '%6502%')"` returns nothing) and say the 4038 half is graded by SP-DHA2. Steps 6517 and 6524 are intentionally absent. A relog-restore half is NOT-RUN by design.

Grade: PASS if every leg the driver ran shows its chain and the step in the mission table; PARTIAL if some legs are NOT-RUN (budget) but every run leg held; FAIL on a missing chain pass or a wrong step. NEEDS_HUMAN: none required.
Looker `H31-marsh-offer`: 1. Is there a "?" icon over an NPC? 2. Is the player close to an NPC?

## Row H40 - mission 1200 *Meet Your Queen* (Goa'uld; chains 6121-6127; character `Uat Fxdhagoauld`)

| Check | Expected |
|---|---|
| Arrival accept | chain 6121 `player_loaded 'Harset'` for archetype 6 with 1200 `not_active`: `accept_mission 1200`; mission table shows 1200 active at step 3584 |
| Abandon and re-entry (policy open) | after the driver's abandon, `Mission abandoned` for 1200 and, on re-entering world 57, whether chain 6121 fired again. **Record both outcomes; either is PASS for this clause** (the re-accept policy is an open decision). Say which happened. |
| Royal Guard | chain 6122 `dialog_choice 4452` at 3584: removes dsm 4817 slot 209, `advance_step 1200 3585`, adds dsm 4751 slot 43 and 6360 slot 42 |
| Anat | chain 6123 `dialog_choice 5435` at 3585: removes 4751 and 6360, `complete_mission 1200` |
| Optional Ba'al | chain 6124 `dialog_open 5436`: objective 5399 completed (only if the driver ran it; else NOT-RUN) |
| Restore | chain 6125 `player_loaded 'Harset_CmdCenter'` bound the guard topic at 3584 on each Command Center entry (check once) |

Grade as H31. Note the Royal Guard (template 209, spawn 341) and the Anat spawn (222) must be present in world 68 (`server_entity_query`). Looker `H40-arrival`: 1. Is any mission notice or text visible? 2. Is the player outdoors in a stone city?

## Row H41 - mission 742 *Giving the Walls Ears* (Goa'uld; chains 6101-6120; character `Uat Fxdhagoauldb`)

| Leg | Evidence |
|---|---|
| 1 Petbe | 6112 bound Petbe's topic at load (step 2502); 6102 `dialog_choice 2638`: `add_item 2819` (disguise), remove dsm 3129 slot 163, `advance_step 742 2503` |
| 2 disguise | 6103 `item_use 2819`: `advance_step 742 2504`, `display_dialog 2637`, `set_interaction_type` on tags FirstBug, SecondBug, ThirdBug (mask `INT_MissionWorldObject`). Tier N3 unless the driver clicked in the bag (NATIVE_SHORTFALL). |
| 3 baskets | Entities with tag `FirstBug`, `SecondBug`, `ThirdBug` exist in world 57 (`server_entity_query`), and each `interact_tag` chain (6104, 6105, 6106) fired once: `complete_objective` 2913, 2914, 2915 and `remove_item 2820`. The third pass fires 6107, 6108 or 6109 (whichever basket is last): `advance_step 742 2505` and `add_dialog_set 3130` slot 43 |
| 4 abandon | `Mission abandoned` for 742, then chain 6120 (`fire_mission_abandoned: matched`): removes dsm 3130 slot 43, adds dsm 3127 slot 43 (the offer). The driver's next Anat click shows the offer (dialog 2636, Accept). |

Spawns: SecondBug (306) and ThirdBug (307) are Inferred in placements.md; if `server_entity_query` finds no entity with those tags, grade leg 3 BLOCKED (not FAIL) and say which tag is missing. Chain 6111 (Nerus) is not run.
Grade: PASS if all four legs hold; PARTIAL if a basket is missing or the abandon route was NEEDS_HUMAN; FAIL on a contradicted chain. Looker `H41-after-baskets`: 1. Is the player in a cave or tunnel area? 2. Is any dialog open? Looker `H41-offer-back`: 1. Is a dialog or offer window open? 2. Is there an Accept button?

---

## DRIVER REPORT

(The coordinator pastes each driver's final report here, with the screenshot paths.)
