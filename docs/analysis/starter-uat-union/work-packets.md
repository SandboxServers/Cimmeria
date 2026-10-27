# Starter-world UAT, Union: work packets

> **Type:** runbook. **Audience:** the orchestrator, the brief authors, the lab drivers and the verifiers.
> **Companions:** [README.md](README.md), [RESUME.md](RESUME.md). **Contract:** every drive packet follows [starter-uat-praxis § Shared UAT contract](../starter-uat-praxis/work-packets.md#shared-uat-contract) (C1 ask first, C2 brief, C3 verify, C4 fixtures, C5 grades, C6 result file, C7 repo rules). Read it before any packet here.

## Dispatch rules

1. **Wave 0.** `ops` creates `su-integration` (`pwsh tools/build-lane/mk-worktree.ps1 starter-uat-union/integration su-integration`) and commits the three campaign files into it. SU-00 runs alone there.
2. **Wave 1a, then 1b.** When SU-00 is reviewed and committed, `ops` creates one worktree per packet branched at the `su-integration` head (branch at SHA), named `su-<packet>` on branch `starter-uat-union/<packet>-briefs`. Dispatch SU-01, SU-02a, SU-02b, SU-02c, SU-02d and SU-03 in one message, in the background; then SU-04 and SU-05 the same way. `documentation-writer` as defined, no model override. Packet review loop each; merge back `--no-ff`; one wave-1 review loop over all eight; one docs PR.
3. **Lab waves 2 to 6 are ASK-FIRST** (contract C1), and open only when D-SU1 and D-SP2 are answered. Instances `default`, `p2`, `p3`; at most three drive packets at once; two lanes never share a plaza, a room or a tent instance.
4. **Per drive packet:** `lab-driver` as defined with the brief pasted verbatim and `ATTEMPT:` set; then the verifier (`general-purpose`, no model override) with the verify file and the driver's report. Three Haiku attempts, then one Sonnet takeover (`ATTEMPT: 4`). The verifier writes `results/<packet>.md` in `su-integration`; issue bodies go to `results/issues/`; `ops` files them.
5. **Stager.** Fixtures are staged by a `general-purpose` agent with the lab-server MCP (`server_stage_fixture`), never by the Haiku driver, as the Beta step files require. The stager runs right before the driver, inside the same lease window.
6. **Results ship per lab wave** as one docs PR from `su-integration`.

## Repo rules every authoring packet carries

The seven rules of contract C7, plus:

8. A brief that converts an owner step file (SGC `uat-U*.md`, Omega `UO-U1.md`) keeps the source step id in each `### ROW` header (`### ROW U1-07`), so the verifier and SU-99 can map grades back.
9. Never copy a coordinate from a step file without checking it against the seed it cites (`db/resources/Worlds/Seed/spawnlist_*.sql`, the chain files). If they differ, the brief uses the seed and notes the difference in § 0.

## Wave 0

### SU-00: row inventory and stale status lines

- **Agent:** documentation-writer. **Host:** windows. **Worktree:** `su-integration`. **Commit:** `docs(starter-uat-union): SU-00 row inventory`.
- **Read:** [uat-U1.md](../union-sgc-continuation/uat-U1.md), [uat-U2.md](../union-sgc-continuation/uat-U2.md), [uat-U3.md](../union-sgc-continuation/uat-U3.md); [unified-uat § Dakara_E1](../../guides/unified-uat.md#dakara_e1); [UO-U1.md](../union-zone-planning/omega-packets/UO-U1.md); [Beta uat/README.md](../union-beta-e1-continuation/uat/README.md) and [fixtures.md](../union-beta-e1-continuation/uat/fixtures.md).
- **Write** `docs/analysis/starter-uat-union/rows.md`: `| Row id | World | Race | Source (file#anchor) | What it checks | Fixture or setup | Drive packet | Grade |`, grade empty. Assign rows by the table below; at most 6 rows and about 28 tool calls per driver dispatch (Beta runs keep the budgets their step files give). Asgard rows get drive packet `-` and grade `BLOCKED (F42)` now (D-SU3).

  | Packet | Rows |
  |---|---|
  | SU-DSG1 | U1 Run A (U1-01 to U1-06) and the escort judgement |
  | SU-DSG2 | U1 Run B (U1-07 to U1-12) |
  | SU-DSG3 | U1 Run C (U1-13 to the Dakara arrival) and the "placement lab checks owed" section |
  | SU-DDK1 | DK-U1 to DK-U5 |
  | SU-DDK2 | DK-U6 to DK-U10 |
  | SU-DDK3 | DK-U11 to DK-U17 |
  | SU-DDK4 | DK-U18 to DK-U21 (1647 fixture, D-SU2; Livewire needs the minigame service) |
  | SU-DDK5 | DK-U22 to DK-U25 |
  | SU-DDK6 | DK-U26 to DK-U30 |
  | SU-DDK7 | DK-U31 to DK-U34 |
  | SU-DDK8 | DK-U35 to DK-U37 |
  | SU-DDK9 | DK-P1 to DK-P5 |
  | SU-DDK10 | DK-P6 to DK-P9 |
  | SU-DL1 | U2 Run A (U2-01 to U2-07): Human 1651 round trip |
  | SU-DL2 | U2 Run B (U2-08 to U2-14): Human 1654, 1655 to Omega |
  | SU-DL3 | U2 Run C (U2-15 to U2-20): Free Jaffa 1654, 1655 |
  | SU-DL4 | U3-01 to U3-06: Omega arrival, both races |
  | SU-DOM1 | UO-H1 to UO-H9 |
  | SU-DOM2 | UO-J1, UO-J2, UO-O1 to UO-O3 |
  | SU-DOM3 | UO-S1 to UO-S5 |
  | SU-DOM4 | UO-R5-S6 to UO-R5-S11 |
  | SU-DOM5 | UO-R5-S12 to UO-R5-S14 |
  | SU-DBE1 | Beta run order 1 (UB-U1, Human half of UB-U1b) |
  | SU-DBE2 | run order 2 (UB-U3 runs A to C; grades UB-U3c, UB-U3d, UR1, UB-U6b 247) |
  | SU-DBE3 | run orders 3 and 4 (UB-U3b, UB-U7b) |
  | SU-DBE4 | run order 5 (UB-U4 runs A, B; grades UB-U4b, UB-U2b 4663, UB-U6b 456 to 458) |
  | SU-DBE5 | run orders 6 and 7 (UB-U6, UB-U8; Free Jaffa) |
  | SU-DBE6 | run orders 8 to 10 (UB-U7, UB-U2, UB-U2c) |
  | SU-DBE7 | run orders 11 and 12 (UB-U9 two clients, UB-U6b fallbacks) |

- **Also check and record** in a `## Preconditions found` section: whether UB-50b2 (Beta naquadah rewards) is on `main` (grep `db/resources/Content/Seed/` for the Beta naq chains; #1944 says "XP + naq"), so UR1's naq half is runnable or not; whether `lab_create_character` can make archetype 7 (the CS-08 worknote); whether any self-damage GM command is registered (grep `crates/cell-console/src/` for `kill`, `damage`, `hurt`) (D-SU4).
- **Stale lines (F5), one line each, nothing else in those files:**
  - [union-zone-planning/README.md](../union-zone-planning/README.md): the Union Omega status cell: "merged in #1950; UO-U1 runs in starter-uat-union".
  - [dakara-e1-rebuild/RESUME.md](../dakara-e1-rebuild/RESUME.md) state bullet: "merged in #1945; the UAT runs in starter-uat-union".
  - [union-sgc-continuation/uat-U1.md](../union-sgc-continuation/uat-U1.md) and uat-U2/U3 preconditions line about the build: "a local server built from `main` at or after #1946".
- **Checks:** CRLF; every row id in `rows.md` exists in its source (`Select-String`).

## Wave 1a: briefs (docs only)

Each packet: **Agent** documentation-writer, **Host** windows, worktree `su-<packet>`, commit `docs(starter-uat-union): <packet> briefs`. For each drive packet it owns it writes `docs/analysis/starter-uat-union/briefs/<drive>.md` (contract C2) and `verify/<drive>.md` (C3) from `rows.md`. Rules C7 plus rules 8 and 9 above. Checks: CRLF; each file ≤ 500 lines; each `rows.md` row of its packets has one `### ROW` block and one grading section.

### SU-01: SGC briefs (SU-DSG1 to DSG3)

- **Source:** uat-U1.md runs A to C, its "escort walk", "stop conditions" and "placement lab checks owed" sections; [placement.md](../union-sgc-continuation/placement.md); [crossings.md](../union-sgc-continuation/crossings.md); `.claude/agent-memory/movement-teleport-advisor/` notes on SGC_W1 crossings.
- **Character:** a fresh Human Soldier per lane (archetype 1). SU-DSG1 starts at 1562 active: stage a fixture with 1559 to 1561 completed and 1562 active at its first step (trace the step ids from the mission seed), in SGC_W1 at the 1562 start point. SU-DSG2 and DSG3 stage the state after the previous run (U1 preconditions column).
- **Gotchas to put in § 0:** teleport only with `.gotolocation SGC_W1 x y z`; the escort stall is judged (record where Carter stopped), not failed; the Gate Room Jaffa may aggro on arrival (record it); combat rows need `/gmsetgodmode 1` so the driver survives, and say so in the row.
- **Evidence for the verify files:** chain ids 3051 to 3199 (the step file's SigNoz filter), `Content: accepting mission`, `final required objective completed the mission`.

### SU-02a: Dakara M1-M2 briefs (SU-DDK1 to DDK3)

- **Source:** unified-uat § Dakara_E1 rows DK-U1 to DK-U17, its prerequisites and logs line; [dakara-e1-rebuild README § the dead-end fix](../dakara-e1-rebuild/README.md#the-dead-end-fix-exactly); [CS-08 worknote](../class-start-v6/worknotes/CS-08.md) (the Free Jaffa creation that passed).
- **Character:** a new Free Jaffa per row group (D-SU5). DK-U10 and DK-U16 need a second, non-Jaffa GM character on another instance, or on the same lane after the Jaffa rows; say which.
- **Gotchas:** mail is read through the mail window (`client_mail_*` tools if present; else NEEDS_HUMAN for the text and the server's `content.send_system_mail` row for the grade); DK-U9 per D-SU4; DK-U11 is a one-time post-deploy check, so it is the first row of the first run after the server starts; DK-U17's portrait and speaker label are expected empty.

### SU-02b: Dakara M3-M5 briefs part 1 (SU-DDK4 to DDK6)

- **Source:** unified-uat DK-U18 to DK-U30, the "M3 to M5 setup (1647)" paragraph, [wave-6b](../dakara-e1-rebuild/packets/wave-6b.md) for DK-U27.
- **Character:** fixture: Free Jaffa, 1646 `completed`, 1647 `completed` (D-SU2), world 62 at Bra'tac. The Livewire rows (DK-U21, U24) need the minigame service; the brief's S1 checks it and marks those rows BLOCKED if it is down.
- **Gotchas:** DK-U19 has no decline (record, D-SU7); DK-U23 moves on region entry, not a click; DK-U26 lands on gate 27's pin, about 13 m in front of the gate.

### SU-02c: Dakara briefs part 2 (SU-DDK7, DDK8)

- **Source:** unified-uat DK-U31 to DK-U37.
- **Character:** DDK7 from a fixture at 1653 accepted (DK-U31 needs it accepted while standing in the courtyard: stage outside, then walk in). DDK8 needs any character at the SGC and a Tau'ri at 1568 step 4638 (stage that fixture in SGC_W1).
- **Gotchas:** DK-U34's pad is LOW-MEDIUM confidence (record where it stands); DK-U35's prop cursor is unchecked (record what shows).

### SU-02d: Dakara placement briefs (SU-DDK9, DDK10)

- **Source:** unified-uat DK-P1 to DK-P9; [dakara-e1-rebuild placements](../dakara-e1-rebuild/README.md) (PL-DK-A and PL-DK-B rows).
- **Character:** a GM Human visitor (placement rows need no mission); `.gotoxyz` is allowed here because the unified rows use it.
- **Output detail:** every placement row records the measured `.location` beside the estimate, so a correction packet can use it.

### SU-03: leg briefs (SU-DL1 to DL4)

- **Source:** uat-U2.md runs A to C, uat-U3.md, [crossings.md](../union-sgc-continuation/crossings.md) (C1 to C4), [omega handoff-sgc-beta.md](../union-zone-planning/omega-packets/handoff-sgc-beta.md).
- **Characters:** fixtures at each run's precondition (Human on Dakara with 1651 offered; Human at the 1654 hand-off; Free Jaffa at the 1654 hand-off; Human and Free Jaffa at 1655 step 4944, the Human holding item 6786).
- **Gotchas:** U2-07 needs a second staged Human and a press inside a 3 s window (one `client_batch` with two presses); heading is a record line in U3-03.

## Wave 1b: briefs (docs only)

### SU-04: Omega briefs (SU-DOM1 to DOM5)

- **Agent:** documentation-writer. Worktree `su-04`. **Source:** [UO-U1.md](../union-zone-planning/omega-packets/UO-U1.md) preparation table (fixtures R1 to R5), steps R1 to R5, its SigNoz section; unified-uat § Union Omega (row texts and exact log lines).
- **Rewrite rule:** UO-U1 is an owner script ("agents never use the lab"). The briefs keep its rows and checks but are driven by the lab driver under D-SU1. They do not edit UO-U1.md.
- **Gotchas:** H9, S8 refill half, S9 per D-SU4; S11 is N/A (not built), write it as a record row; S13 needs at least 1 500 naquadah (`/gmgivecash` in setup) and six free bag slots; S14 is a refusal check (`reason=not_sellable`); Asgard rows A1 to A4 are not in any brief (D-SU3).

### SU-05: Beta run headers and verify files (SU-DBE1 to DBE7)

- **Agent:** documentation-writer. Worktree `su-05`. **Source:** [Beta uat/README.md](../union-beta-e1-continuation/uat/README.md), [_driver-common.md](../union-beta-e1-continuation/uat/_driver-common.md), [fixtures.md](../union-beta-e1-continuation/uat/fixtures.md), every step file named by the drive packet.
- **Write, per drive packet:** `briefs/SU-DBEn.md` is **a header only**: the contract C2 header lines, § 0 gotchas (the hub-pass lessons not already in `_driver-common.md`), then "Paste `_driver-common.md`, then the step file `<name>.md`, run by run" with the run list and the fixture id for each run, and the C2 § 2 and § 3 sections. Do not copy or edit the step files. `verify/SU-DBEn.md` follows C3 and takes its checks from each step file's verify list.
- **Also:** `rows.md` already notes the HELD Asgard rows; DBE5 states the Free Jaffa character rule (D-SU5) in its header; DBE7 is a two-instance packet (UB-U9: one driver at a time, one lease per instance, per the Beta README).
- **Results:** the verifier writes this campaign's `results/` and **also** fills the matching rows of [uat-results.md](../union-beta-e1-continuation/uat-results.md) (result, evidence, date), because that is the Beta ledger's results table.

## Waves 2 to 6: lab drive packets (ASK-FIRST)

Every row below: **LAB: ASK-FIRST** (contract C1). Host windows (the lab machine). Lease owner `su-<packet>`. Depends on its brief being integrated, D-SU1 and D-SP2.

| Wave | Packets (instance) | Notes |
|---|---|---|
| 2 | SU-DSG1 (default), SU-DSG2 (p2), SU-DDK1 (p3); then SU-DSG3 (default), SU-DDK2 (p2), SU-DDK3 (p3) | DSG2 and DSG3 use their own staged characters, so they need not wait for DSG1 |
| 3 | SU-DDK4 (default), SU-DDK5 (p2), SU-DL1 (p3); then SU-DDK6 (default), SU-DDK7 (p2), SU-DDK8 (p3) | DDK4 and DDK5 both touch Bra'tac's tent: world 62 is instanced per player, so two lanes may run there; if the brief finds it shared, run them one after the other |
| 4 | SU-DDK9 (default), SU-DL2 (p2), SU-DL3 (p3); then SU-DDK10 (default), SU-DL4 (p2), SU-DOM1 (p3) | DL4 needs a Human and a Free Jaffa: run it on p2 after DL2 and DL3 finish |
| 5 | SU-DOM2 (default), SU-DOM3 (p2), SU-DBE1 (p3); then SU-DOM4 (default), SU-DOM5 (p2), SU-DBE2 (p3) | DOM3 to DOM5 share the Omega hub: never two of them at once |
| 6 | SU-DBE3 to DBE6 (two lanes), then SU-DBE7 (default + p2), then attempt-4 takeovers | DBE7 runs alone on the lab |

## Wave 7: specs, roll-up, close-out

### SU-91: TOML specs for passed SGC and Dakara rows

- **Agent:** packet-coder. **Host:** windows. Worktree `su-91`, branch `starter-uat-union/SU-91-specs`. **Commit:** `docs(uat-specs): Union SGC and Dakara rows from starter-uat-union`.
- **Input:** `results/SU-DSG*.md`, `SU-DDK*.md`, `SU-DL1.md` § Recipe; rows graded PASS or NATIVE_SHORTFALL only.
- **Write:** new `docs/guides/uat-specs/sgc-w1.toml` (section `sgc-w1`) and `dakara-e1.toml` (section `dakara-e1`), `account = "gm"`, `character = "fresh"`, fixtures per contract C4 (`state = "completed"` for 1647). Row shape from [abilities.toml](../../guides/uat-specs/abilities.toml). Row ids prefixed `SU-`. Tags named in a spec must exist in `db/resources/Worlds/Seed/` (the loader checks it).
- **Check (the only cargo):** `Set-Location <worktree>; pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-lab committed_specs_parse_and_validate every_committed_spec_tag_is_seeded`. Both pass. Fix rounds ≤ 5. Foreground, from the worktree. TOML files stay LF.

### SU-92: TOML specs for passed Omega and Beta rows

As SU-91 for `results/SU-DOM*.md`, `SU-DBE*.md`, `SU-DL2..4.md`, into `omega-site.toml` and `beta-site-e1.toml`. Worktree `su-92`. Commit `docs(uat-specs): Union Omega and Beta rows from starter-uat-union`.

### SU-98: issue roll-up and sign-off sheet

documentation-writer, worktree `su-integration`, commit `docs(starter-uat-union): SU-98 results summary`. A `## Results summary` at the top of README.md (grade counts, per-world sign-off, issues, lessons; the [hub-pass shape](../hub-pass-uat/README.md#results-summary-final-2026-10-13)) and `signoff.md` with every NEEDS_HUMAN row (D-SU7) and its screenshot.

### SU-99: close-out

documentation-writer, worktree `su-integration`, commit `docs(starter-uat-union): SU-99 close-out`. Fill the result columns of the SGC step files, the Omega `worknotes/UO-U1-results.md` (create it, the path UO-U1 names), Beta `uat-results.md` (already filled per run, check it) and the Dakara ledger's UAT rows, each citing `results/<packet>.md`. In unified-uat.md: the SG-1 to SG-10 "Lab result" column, the Dakara, Omega and Beta status lines (including F5's stale Omega line). Then the status docs once (`docs/project-status.md`, `docs/gap-analysis*`; walk [doc-update-map.md](../../agents/doc-update-map.md); list the rows in the PR body). RESUME.md says "closed".
