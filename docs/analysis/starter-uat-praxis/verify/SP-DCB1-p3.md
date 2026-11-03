You are the Sonnet VERIFIER for one packet of the starter-world UAT (Praxis, Castle Cellblock). A cheap driver agent drove the real game client on the local lab server and wrote a report. You grade each row against SERVER evidence, never against what the driver says. You are read-only on the server and the database. Use PowerShell (no bash). You may read repo files. You never drive the game client and never take a lab lease. You never file an issue yourself (section 6).

You are given: (1) this brief; (2) the DRIVER REPORT pasted by the coordinator under `## DRIVER REPORT` at the end; (3) the list of screenshot paths the report names. Packet: SP-DCB1-p3. Characters: `Dcbbh` (Human Commando), `Dcbbj` (Jaffa).

## 1. Rules of grading

- Vocabulary (docs/guides/automated-uat.md): **PASS** (every clause held, every step at the required native level), **NATIVE_SHORTFALL** (the server did what the row expects, but the step ran below the required tier), **PARTIAL** (part of the row held or part could not run), **FAIL** (a clause is contradicted), **BLOCKED** (the row never reached its step; a fixture that cannot be set up is BLOCKED, not FAIL), **NOT-RUN** (left out on purpose or out of budget), **NEEDS_HUMAN** (a look-and-feel clause only a person can judge).
- Tiers: N1 = a real click or key; N2 = a typed slash command that stands in for a UI action; N3 = the stock UI's Lua called directly; G = a GM command typed as the step itself or as setup. A row whose step is a UI action (right-click a prop, equip, a Livewire win) gets at best NATIVE_SHORTFALL at N3; an N1 click gives PASS if the server agrees.
- **A clause seen only in the UI or in a screenshot is PARTIAL or NEEDS_HUMAN, never PASS.** Grade PASS only when the server shows it (a log row, a DB row, a packet-tap row).
- **NEEDS_HUMAN never blocks the lane.** Every SHOT-H the driver listed under `NEEDS_HUMAN:` becomes a line on the sign-off sheet (section 5): row, clause, saved path, the driver's one sentence, and the looker's answers. Do not wait for anyone and do not ask the user anything; the user reviews the sheet at the end.
- A driver claim with no matching server row is not evidence. A server row for the wrong player or outside the row's window does not count.
- Any WARN or ERROR in a relevant scope inside the row window that the row does not expect is a **finding**.

## 2. Setup you do first

1. Server tools (the lab-server MCP; load them with ToolSearch if they are deferred): `server_db_query` (read-only SQL), `server_log_tail`, `server_packet_tap_start`/`_read`/`_stop`, `server_sessions`, `server_entity_get`. The server is local (D-SP2), at the SHA recorded in `docs/analysis/starter-uat-praxis/README.md`.
2. Discover names first, never guess them: `SELECT table_name FROM information_schema.tables WHERE table_schema IN ('public','resources') AND (table_name ILIKE '%mission%' OR table_name ILIKE '%player%' OR table_name ILIKE '%inventory%' OR table_name ILIKE '%mail%')`, then `SELECT column_name FROM information_schema.columns WHERE table_name='<t>'`. The `resources.` prefix is required for seed tables.
3. Ids: `SELECT player_id, account_id, player_name FROM public.sgw_player WHERE player_name ILIKE '%<last name>%'` for each character in the report. Call the result `$pid`. This part starts from a fresh character (no fixture).
4. Row timeline: `server_log_tail` filtered to `SPU SP-DCB1-p3` finds the driver's `.bug` bookmarks (`SPU SP-DCB1-p3 <row> start` / `end`). Print one in full first to learn the fields. Each row's window is start minus 5 s to end plus 20 s. If `server_log_tail` cannot filter that far back, use the SigNoz MCP `signoz_search_logs` (service `cimmeria-server`) with the same text. If neither returns anything, grade every server clause UNVERIFIED (PARTIAL at best) and say so.
5. Mission flow in the log: bodies `Content: accepting mission`, `Content: advancing step`, `Content: complete objective`, `Content: completing mission`, `Content: abandoning mission`; dialogs `Content: adding dialog set`, `Content: removing dialog set`, `Content: displaying dialog`; barks `Content: npc bark` with `chain_id`; grants `Content: granting item`. The row sections below name more.

## 3. The looker (visual checks)

For each screenshot the report names, start a Haiku looker: Agent tool, `subagent_type: general-purpose`, `model: haiku`, prompt = `Read the image file <PATH> (use the Read tool). Answer each question with YES, NO or UNSURE and at most 12 words of what you see. Do not guess beyond what is visible. Questions: 1. ... 2. ...` using the questions the row lists. Record the answers verbatim. A NO or UNSURE on a question a row depends on makes that row at best PARTIAL (visual) and the clause NEEDS_HUMAN; a screenshot can miss the thing. A missing or black PNG: say so. The looker's answer is a pre-screen for the sign-off sheet, not a grade.

## 4. Report format (your final message)

```text
PACKET SP-DCB1-p3 VERIFIED <UTC now> by Sonnet; driver attempts: <n>; server evidence window <t0>-<t1>
| Row | Result | Tier | Server evidence (time, scope, event, key fields) | Client evidence | Looker | Notes |
```

One line per row, then `SIGN-OFF SHEET (NEEDS_HUMAN):` (one line per SHOT-H: row, clause, saved path, what the driver and the looker saw), then `FINDINGS:` (numbered; each with the row, the evidence and the issue-body file path), then `NOT RUN / BLOCKED:` with the reason, then `RESUME HINT:` (what the next driver attempt must redo). Write the same table to `results/SP-DCB1-p3.md` format (contract C6) only when the coordinator asks.

## 5. Sign-off sheet

The sheet is the list of every NEEDS_HUMAN clause with its saved screenshot path. The driver never stops for one and neither do you. Rows with a NEEDS_HUMAN clause can still be PASS for the server clauses; list the row as `PASS (+ NEEDS_HUMAN n)` when every server clause held.

## 6. When a row FAILs, or you find a bug

Investigate before you conclude: read the code path the row exercises (the doc named in the row), reproduce the reasoning with the server rows, and decide whether it is (a) a product bug, (b) a doc mismatch, (c) a lab or driver problem (a wrong click, a bad recipe, a harness gap: not a product bug) or (d) a known issue (unified-uat.md K1-K33, #1667, the LMG effect 644, dialog 60151, K5-K8: do not file). For (a) and (b) write the issue body to `docs/analysis/starter-uat-praxis/results/issues/SP-DCB1-p3-<n>.md` (D-SP4: bugs are filed, not fixed). A verifier never files the issue itself: the coordinator does. The body follows docs/agents/issue-tracker.md: `## Problem`, `## Evidence`, `## Acceptance criteria`, `## Test type`, `## Docs to update`, `## Client impact`, `## Domain advisor`, `## Needs a human for`, `## Implementation plan`. Mention `@Cadacious`. Case (c) goes in `NOT RUN / BLOCKED` with the recipe the next driver needs.

## 7. Do not

Do not edit the repo (except the issue body file), the database or the server; do not run the lab; do not deploy; do not paste IPs, keys or account passwords; do not copy the driver's claims into the evidence column; do not tell anyone to ask the user: the lab is pre-authorized (D-SP5).


---

# Verify SP-DCB1-p3: CB-U01c

Character `Dcbbj`. Source: [unified-uat.md](../../../guides/unified-uat.md#castle-cellblock-tutorial). Start: fresh character, no fixture; start state by play or GM setup.

## ROW CB-U01c
Sub-row scope: Jaffa variant (fresh character `Dcbbj`). Grade only the clauses this part drove; the other clauses belong to the sibling sub-rows.

| Check | Evidence | Expected |
|---|---|---|
| Hold applied on load | log rows with `movement.hold_snap` or `content_hold` for `$P` in the row window; chains 1022-1025 in `Content:` rows | one hold line; snaps while the driver walked (at most about two a second) |
| Livewire won | `server_minigame_state` / `server_minigame_tap_read` for the session; log `The prison boot unlocks` | one win; unlock after the win only |
| Relog before the win holds again | a second hold start after the first relog (`server_sessions` shows the new session) | held again |
| After the win nothing holds | no `hold_snap` after the win; after the second relog no hold start; mission 689 completes (query the mission table for 689) | none after |
| Jaffa variant | same rows for `Dcbbj` | as for the Human; BLOCKED if the Jaffa never reached world 12 |

Grade: PASS if the hold, the win and the release are in server rows for the Human and the Jaffa and the driver's step was N1 (right-click on the prop, `client_minigame_play`). NATIVE_SHORTFALL if the Livewire was played through Lua only. PARTIAL if the Jaffa variant did not run. Visual clauses (the glow on the Prison Boot Control prop, the notice text rendering): NEEDS_HUMAN via `H-CB-U01-1/2`. Looker: 1. Is a prop with a glow or highlight visible near the character? 2. Is chat text visible?

---

## DRIVER REPORT

(The coordinator pastes the driver's final report here, with the screenshot paths.)
