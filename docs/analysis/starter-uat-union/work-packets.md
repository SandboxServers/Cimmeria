# Starter-world UAT, Union: work packets

> **Type:** runbook. **Audience:** the orchestrator, the brief authors, the lab drivers and the verifiers.
> **Companions:** [README.md](README.md), [RESUME.md](RESUME.md). **Contract:** every drive packet follows [starter-uat-praxis § Shared UAT contract](../starter-uat-praxis/work-packets.md#shared-uat-contract) (C1 ask first, C2 brief, C3 verify, C4 fixtures, C5 grades, C6 result file, C7 repo rules, C8 spec runs). Read it before any packet here.

## Dispatch rules

1. **Wave 0.** `ops` creates `su-integration` (`pwsh tools/build-lane/mk-worktree.ps1 starter-uat-union/integration su-integration`) and commits the three campaign files into it. SU-00 runs alone there.
2. **Wave 1a, then 1b.** When SU-00 is reviewed and committed, `ops` creates one worktree per packet branched at the `su-integration` head (branch at SHA), named `su-<packet>` on branch `starter-uat-union/<packet>-briefs`. Dispatch SU-01, SU-02a, SU-02b, SU-02c, SU-02d, SU-03a and SU-03b in one message, in the background; then SU-04 and SU-05 the same way. `documentation-writer` as defined, no model override. Packet review loop each; merge back `--no-ff`; one wave-1 review loop over all nine; one docs PR.
3. **Lab waves 2 to 6 are ASK-FIRST** (contract C1), and open only when D-SU1 and D-SP2 are answered. Instances `default`, `p2`, `p3`; at most three drive packets at once; two lanes never share a plaza, a room or a tent instance. Each brief's § 0 names the area its rows use; if two packets the waves table runs at once name the same area, the orchestrator moves the later one to the end of the other's chain. "A → B" in the waves table means B starts on that instance only after A's verifier has returned.
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
  | SU-DSG1 | U1 Run A (U1-01 to U1-06); the escort judgement is part of U1-05, not a seventh row |
  | SU-DSG2 | U1 Run B (U1-07 to U1-12) |
  | SU-DSG3 | U1 Run C (U1-13 to U1-16, which ends in the Dakara arrival) and the "placement lab checks owed" section (notes read beside the steps they name, not rows) |
  | SU-DDK1 | DK-U1 to DK-U5 |
  | SU-DDK2 | DK-U6 to DK-U10 |
  | SU-DDK3a | DK-U11 to DK-U15: one new Free Jaffa from the first login after the deploy through the Rak'nor click and the journal after 6110 (DK-U14 and U15 are one sequence) |
  | SU-DDK3b | DK-U16 (relog, then a non-Jaffa GM visitor), DK-U17 (fixture at 1570 "Collect the dog tags") |
  | SU-DDK4 | DK-U18 to DK-U21 (1647 fixture, D-SU2; Livewire needs the minigame service) |
  | SU-DDK5 | DK-U22 to DK-U25 |
  | SU-DDK6 | DK-U26 to DK-U30 |
  | SU-DDK7 | DK-U31 to DK-U34 |
  | SU-DDK8 | DK-U35 to DK-U37 |
  | SU-DDK9 | DK-P1 to DK-P5 |
  | SU-DDK10 | DK-P6 to DK-P9 |
  | SU-DL1a | U2-01 to U2-04: Human 1651 from Dakara to the SGC, Hammond's briefing, the relog on step 4931 |
  | SU-DL1b | U2-05 to U2-07: Carter's item, Harriman's dial back to Dakara, the second Human's double press (fixture: Human in SGC_W1 on 1651 step 4931, plus a second Human staged to U2-05) |
  | SU-DL2a | U2-08 to U2-11: Human 1654 hand-off to the SGC, Hammond, Woolsey, the Woolsey relog |
  | SU-DL2b | U2-12 to U2-14: 1655 accept, item 6786, Harriman to Omega (fixture: Human in SGC_W1 with 1654 completed, so chain 3152 spawns Woolsey on load) |
  | SU-DL3 | U2 Run C (U2-15 to U2-20): Free Jaffa 1654, 1655 |
  | SU-DL4 | U3-01 to U3-06: Omega arrival, both races |
  | SU-DOM1a | UO-H1 to UO-H5: Human into Omega_Site, Modi, pad 14 to world 80, Jackson |
  | SU-DOM1b | UO-H6 to UO-H9: the relog in world 80, the 1423 accept, the dial list, the second Human's death (fixture: Human in world 80 at the Jackson step, the state H4 leaves) |
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
- **Checks:** CRLF; every row id in `rows.md` exists in its source (`Select-String`); no drive packet outside the Beta runs holds more than 6 rows.

## Wave 1a: briefs (docs only)

Each packet: **Agent** documentation-writer, **Host** windows, worktree `su-<packet>`, commit `docs(starter-uat-union): <packet> briefs`. For each drive packet it owns it writes `docs/analysis/starter-uat-union/briefs/<drive>.md` (contract C2) and `verify/<drive>.md` (C3) from `rows.md`. Rules C7 plus rules 8 and 9 above. Checks: CRLF; each file ≤ 500 lines; each `rows.md` row of its packets has one `### ROW` block and one grading section.

### SU-01: SGC briefs (SU-DSG1 to DSG3)

- **Source:** uat-U1.md runs A to C, its "escort walk", "stop conditions" and "placement lab checks owed" sections; [placement.md](../union-sgc-continuation/placement.md); [crossings.md](../union-sgc-continuation/crossings.md); `.claude/agent-memory/movement-teleport-advisor/` notes on SGC_W1 crossings.
- **Character:** a fresh Human Soldier per lane (archetype 1). SU-DSG1 starts at 1562 active: stage a fixture with 1559 to 1561 completed and 1562 active at its first step (trace the step ids from the mission seed), in SGC_W1 at the 1562 start point. SU-DSG2 and DSG3 stage the state after the previous run (U1 preconditions column).
- **Gotchas to put in § 0:** teleport only with `.gotolocation SGC_W1 x y z`; the escort stall is judged (record where Carter stopped), not failed; the Gate Room Jaffa may aggro on arrival (record it); combat rows need `/gmsetgodmode 1` so the driver survives, and say so in the row.
- **Evidence for the verify files:** chain ids 3051 to 3199 (the step file's SigNoz filter), `Content: accepting mission`, `final required objective completed the mission`.

### SU-02a: Dakara M1-M2 briefs (SU-DDK1, DDK2, DDK3a, DDK3b)

- **Source:** unified-uat § Dakara_E1 rows DK-U1 to DK-U17, its prerequisites and logs line; [dakara-e1-rebuild README § the dead-end fix](../dakara-e1-rebuild/README.md#the-dead-end-fix-exactly); [CS-08 worknote](../class-start-v6/worknotes/CS-08.md) (the Free Jaffa creation that passed).
- **Character:** a new Free Jaffa per row group (D-SU5). DK-U10 and DK-U16 need a second, non-Jaffa GM character on the same lane after the Jaffa rows. SU-DDK3b's DK-U16 starts from a fixture at the state DK-U15 leaves (Free Jaffa, "Home to Dakara" completed, 1570 active at "Speak to Bra'tac at his command tent."); DK-U17 starts from a fixture at 1570 "Collect the dog tags" with two sets picked up. Trace both missions' step ids from `db/resources/` and cite them.
- **Gotchas:** mail is read through the mail window (`client_mail_*` tools if present; else NEEDS_HUMAN for the text and the server's `content.send_system_mail` row for the grade); DK-U9 per D-SU4; DK-U11 is a one-time post-deploy check, so SU-DDK3a is the first packet its instance runs after the server starts, and DK-U11 its first row; DK-U17's portrait and speaker label are expected empty.

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

### SU-03a: Human leg briefs (SU-DL1a, DL1b, DL2a, DL2b)

- **Source:** uat-U2.md runs A and B, [crossings.md](../union-sgc-continuation/crossings.md) (C1 to C4), [omega handoff-sgc-beta.md](../union-zone-planning/omega-packets/handoff-sgc-beta.md).
- **Characters:** fixtures at each packet's start: DL1a, a Human on Dakara with 1651 offered; DL1b, a Human in SGC_W1 on 1651 step 4931 (Carter shows "!") and a second fresh Human staged to the same state for U2-07; DL2a, a Human at the 1654 hand-off; DL2b, a Human in SGC_W1 with 1654 completed (chain 3152 spawns Woolsey on `player_loaded`). Trace each step id from `db/resources/` and cite it.
- **Gotchas:** U2-07 needs the second staged Human and a press inside a 3 s window (one `client_batch` with two presses); the stager stages both Humans before DL1b's driver starts (dispatch rule 5).

### SU-03b: Free Jaffa and Omega-arrival leg briefs (SU-DL3, DL4)

- **Source:** uat-U2.md run C, uat-U3.md, the same crossings and hand-off files as SU-03a.
- **Characters:** DL3, a Free Jaffa at the 1654 hand-off; DL4, a Human and a Free Jaffa at 1655 step 4944, the Human holding item 6786.
- **Gotchas:** heading is a record line in U3-03; U2-18 is a FAIL if item 6786 appears.

## Wave 1b: briefs (docs only)

### SU-04: Omega briefs (SU-DOM1a, DOM1b, DOM2 to DOM5)

- **Agent:** documentation-writer. Worktree `su-04`. **Source:** [UO-U1.md](../union-zone-planning/omega-packets/UO-U1.md) preparation table (fixtures R1 to R5), steps R1 to R5, its SigNoz section; unified-uat § Union Omega (row texts and exact log lines).
- **Rewrite rule:** UO-U1 is an owner script ("agents never use the lab"). The briefs keep its rows and checks but are driven by the lab driver under D-SU1. They do not edit UO-U1.md.
- **Character:** SU-DOM1b starts from a fixture in world 80 at the Jackson step (the state UO-H4 leaves; trace the 1655 step id from `db/resources/`), so it does not depend on SU-DOM1a's character.
- **Gotchas:** H9, S8 refill half, S9 per D-SU4; S11 is graded **NOT-RUN** with the reason "not built (D-UO32 (a))" (contract C5 has no N/A grade; UO-U1's "N/A" maps to NOT-RUN), and its brief block only records what the inn spot shows; S13 needs at least 1 500 naquadah (`/gmgivecash` in setup) and six free bag slots; S14 is a refusal check (`reason=not_sellable`); Asgard rows A1 to A4 are not in any brief (D-SU3).

### SU-05: Beta run headers and verify files (SU-DBE1 to DBE7)

- **Agent:** documentation-writer. Worktree `su-05`. **Source:** [Beta uat/README.md](../union-beta-e1-continuation/uat/README.md), [_driver-common.md](../union-beta-e1-continuation/uat/_driver-common.md), [fixtures.md](../union-beta-e1-continuation/uat/fixtures.md), every step file named by the drive packet.
- **Write, per drive packet:** `briefs/SU-DBEn.md` is **a header only**: the contract C2 header lines, § 0 gotchas (the hub-pass lessons not already in `_driver-common.md`), then "Paste `_driver-common.md`, then the step file `<name>.md`, run by run" with the run list and the fixture id for each run, and the C2 § 2 and § 3 sections. Do not copy or edit the step files. `verify/SU-DBEn.md` follows C3 and takes its checks from each step file's verify list.
- **Also:** `rows.md` already notes the HELD Asgard rows; DBE5 states the Free Jaffa character rule (D-SU5) in its header; DBE7 is a two-instance packet (UB-U9: one driver at a time, one lease per instance, per the Beta README).
- **Results:** the verifier writes only this campaign's `results/` and `results/issues/` (contract C3 "do not" list). It never edits [uat-results.md](../union-beta-e1-continuation/uat-results.md); SU-99 copies the Beta grades there (result, evidence, date) from `results/SU-DBE*.md`, because that is the Beta ledger's results table.

## Waves 2 to 6: lab drive packets (ASK-FIRST)

Every row below: **LAB: ASK-FIRST** (contract C1). Host windows (the lab machine). Lease owner `su-<packet>`. Depends on its brief being integrated, D-SU1 and D-SP2.

| Wave | `default` | `p2` | `p3` | Notes |
|---|---|---|---|---|
| 2 | SU-DSG1 → SU-DSG3 | SU-DSG2 | SU-DDK3a → SU-DDK3b → SU-DDK1 → SU-DDK2 | DSG2 and DSG3 use their own staged characters, so they need not wait for DSG1. The Dakara packets share the gate plaza, so they run on one instance, DDK3a first (DK-U11) |
| 3 | SU-DDK4 → SU-DDK6 | SU-DDK5 → SU-DDK7 | SU-DL1a → SU-DDK8 → SU-DL1b | DDK4 and DDK5 both touch Bra'tac's tent: world 62 is instanced per player, so two lanes may run there; if a brief finds it shared, run them one after the other. The `p3` chain holds every SGC_W1 packet of the wave |
| 4 | SU-DDK9 → SU-DDK10 | SU-DL2a → SU-DL2b → SU-DL3 → SU-DL4 | attempt-4 takeovers only | The legs share the SGC Gate Room and the Omega arrival, so they run on one instance. No Omega packet runs in this wave |
| 5 | SU-DOM1a → SU-DOM1b → SU-DOM2 | SU-DOM3 → SU-DOM4 → SU-DOM5 | SU-DBE1 → SU-DBE2 | DOM3 to DOM5 share the Omega hub services, so they run on one instance; the gate, Modi and world 80 rows of DOM1 and DOM2 run beside them unless a brief's § 0 names the same area (rule 3) |
| 6 | SU-DBE3 → SU-DBE5 | SU-DBE4 → SU-DBE6 | attempt-4 takeovers | Keep the Beta README's run order: if a later run order needs an earlier one's state, chain it on the same instance. Then SU-DBE7 on `default` + `p2`, alone on the lab; then any remaining takeovers |

An attempt-4 takeover runs on the failed packet's instance after that instance's chain, or on a free instance the table leaves open, under rule 3.

## Wave 7: specs, roll-up, close-out

### SU-91: TOML specs for passed SGC and Dakara rows

- **Agent:** packet-coder. **Host:** windows. Worktree `su-91`, branch `starter-uat-union/SU-91-specs`. **Commit:** `docs(uat-specs): Union SGC and Dakara rows from starter-uat-union`.
- **Input:** `results/SU-DSG*.md`, `SU-DDK*.md`, `SU-DL1a.md`, `SU-DL1b.md` § Recipe; rows graded PASS or NATIVE_SHORTFALL only.
- **Write:** new `docs/guides/uat-specs/sgc-w1.toml` (section `sgc-w1`) and `dakara-e1.toml` (section `dakara-e1`), `account = "gm"`, `character = "fresh"`, fixtures per contract C4 (`state = "completed"` for 1647). Row shape from [abilities.toml](../../guides/uat-specs/abilities.toml). Row ids prefixed `SU-`. Tags named in a spec must exist in `db/resources/Worlds/Seed/` (the loader checks it).
- **Check (the only cargo):** `Set-Location <worktree>; pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-lab committed_specs_parse_and_validate every_committed_spec_tag_is_seeded`. Both pass. Fix rounds ≤ 5. Foreground, from the worktree. No Rust is edited, so no `cargo fmt` or `cargo clippy`.
- **Line endings:** the TOML specs stay **LF** like the files already in `docs/guides/uat-specs/`. This is the one exception to contract C7 rule 2 (which converts `.md` files only): never run the CRLF conversion on a `.toml`.

### SU-92: TOML specs for passed Omega and Beta rows

As SU-91 (same two-test lane check, same LF rule) for `results/SU-DOM*.md`, `SU-DBE*.md`, `SU-DL2a.md`, `SU-DL2b.md`, `SU-DL3.md`, `SU-DL4.md`, into `omega-site.toml` and `beta-site-e1.toml`. Worktree `su-92`. Commit `docs(uat-specs): Union Omega and Beta rows from starter-uat-union`.

### SU-98: issue roll-up and sign-off sheet

documentation-writer, worktree `su-integration`, commit `docs(starter-uat-union): SU-98 results summary`. A `## Results summary` at the top of README.md (grade counts, per-world sign-off, issues, lessons; the [hub-pass shape](../hub-pass-uat/README.md#results-summary-final-2026-10-13)) and `signoff.md` with every NEEDS_HUMAN row (D-SU7) and its screenshot.

### SU-99: close-out

documentation-writer, worktree `su-integration`, commit `docs(starter-uat-union): SU-99 close-out`. Fill the result columns of the SGC step files, the Omega `worknotes/UO-U1-results.md` (create it, the path UO-U1 names), Beta `uat-results.md` (copy each Beta row's result, evidence and date from `results/SU-DBE*.md`; no verifier writes it) and the Dakara ledger's UAT rows, each citing `results/<packet>.md`. In unified-uat.md: the SG-1 to SG-10 "Lab result" column, the Dakara, Omega and Beta status lines (including F5's stale Omega line). Then the status docs once (`docs/project-status.md`, `docs/gap-analysis*`; walk [doc-update-map.md](../../agents/doc-update-map.md); list the rows in the PR body). RESUME.md says "closed".
