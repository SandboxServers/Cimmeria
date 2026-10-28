# Starter-world UAT: Union, SGC_W1 to Beta Site E1 (SU)

> **Type:** campaign ledger (explanation and status). **Audience:** the orchestrator that runs it, the owner and lomiada who answer its decisions.
> **Planned:** 2026-10-27 against `origin/main` `43a424588`, after Union SGC (#1946), Dakara_E1 (#1945), Beta E1 rewards (#1944) and Union Omega (#1950).
> **Companions:** [work-packets.md](work-packets.md), [RESUME.md](RESUME.md), the shared UAT contract in [starter-uat-praxis/work-packets.md § Shared UAT contract](../starter-uat-praxis/work-packets.md#shared-uat-contract), [unified-uat.md](../../guides/unified-uat.md).

## Sibling campaigns (lomiada's starter-world focus, 2026-10-27)

| Campaign | What it covers |
|---|---|
| [starter-uat-praxis](../starter-uat-praxis/README.md) | Lab UAT of the Praxis chain: Castle_CellBlock (12), Castle (8), Harset (57, 68-70), Tollana (19) |
| **starter-uat-union** (this one) | Lab UAT of the Union chain for the Human and the Free Jaffa: SGC_W1 (58), Dakara_E1 (61/62), Omega (18/80), Beta Site E1 (23) |
| [starter-uat-shops](../starter-uat-shops/README.md) | Vendor behaviour no zone ledger covers: sell, buyback, repair, recharge, the Debug Area vendor spec, and the worlds with no vendor |
| [debug-armory](../debug-armory/README.md) | Debug Area world 1300: wear and fire any weapon, wear any armor, cast every archetype ability, with generated lab specs |

The four campaigns share no files except `docs/guides/unified-uat.md`, which only each close-out packet edits. They can run in any order. The lab takes one campaign's wave at a time.

## Purpose

Contributor lomiada, 2026-10-27: *"all current develop effort should go in the starting worlds up to Tollana / beta site (for each faction): the worlds, gameplay, shops and so on."* For the Union this is the chain the owner chose to plan for all three starts through first Beta E1 play (U-D02, U-D05). Only two of those starts are playable today, the Human and the Free Jaffa, so this campaign runs those two and records every Asgard row BLOCKED (D-SU3). All four zones have merged. Not one of their client rows has run, because their ledgers reserved the lab for the owner. This campaign is that end UAT. It turns the owner-run step files into lab-driver briefs, runs them, grades every row from server evidence, files issues, and turns passes into TOML specs. It writes no game code.

## Worlds in scope and what is known today

"Tested" means a person or the lab saw it in a real client and the server agreed. "Server-only" means unit, replay, executor or live-DB tests. Anything else is untested.

| Order | World (id) | Reached by | Tested in a client | Server-only, never seen in a client | Known broken or missing |
|---|---|---|---|---|---|
| 1 | SGC_W1 (58) | a fresh Human (`SGU_HUMAN_*`) | Class-start v6 CS-08 lab run: Human creation, M1559, Hammond, Teal'c, the elevator, Carter's radio call and the M1562 desk step **PASS**; the action bar stayed empty (F8) ([CS-08](../class-start-v6/worknotes/CS-08.md)) | the restoration from 1562 to the gate (U1-01 to U1-16), the 1651 round trip and 1654/1655 for both races (U2-01 to U2-20), the Omega arrival (U3-01 to U3-06) ([union-sgc-continuation](../union-sgc-continuation/README.md)) | The step files say "agents do not run this" and "build from the SGC branch"; the branch merged in #1946. The 1564 escort may stall (judged, not failed) |
| 2 | Dakara_E1 (61), tent interior (62) | Free Jaffa start (#1273); a Human visits on 1570 and 1651 | CS-08: a new Free Jaffa lands on Dakara with its kit, the Omega address and the Gate Watch mail **PASS** (part of DK-U1) | the rest of DK-U1 (CS-08 passed only the part named on the left), DK-U2 to DK-U37, DK-P1 to DK-P9 ([unified-uat § Dakara_E1](../../guides/unified-uat.md#dakara_e1)) | 1648 to 1654 dormant until 1647 ships (D-DK23): rows DK-U18 on need 1647 set completed first. Placements are LOW confidence estimates. Wave 7 (1647, DK-20, DK-21) not built |
| 3 | Omega_Site (18), Command Center (80) | 1655 from the SGC | none | UO-U1 R1 to R5: UO-H1 to H9, UO-J1/J2, UO-O1 to O3, UO-S1 to S5, UO-R5-S6 to S14 ([UO-U1](../union-zone-planning/omega-packets/UO-U1.md)) | No self-damage command (`.kill` unregistered): H9, S8 refill, S9 may be BLOCKED. Inn and battlemaster not built. Nothing bought sells back (by design, S14). Black Market window needs the client patch. Asgard rows A1 to A4 need a playable Asgard (F42). The unified-uat status line still says R5 is "not yet on main" |
| 4 | Beta_Site_Evo_1 (23) | 1423 (Human), 1500 (Free Jaffa) from Omega | none | UB-U1 to UB-U10, UR1; driver step files already written ([union-beta-e1-continuation/uat](../union-beta-e1-continuation/uat/README.md)) | Asgard rows HELD until Omega UO-16 (HELD-B16). 1503 not built; 1425 and 1427 stay open by design (D-UB5 A). The Free Jaffa fixture character was to be made by hand |

**Asgard** has no playable start (character-creation row 9 is `NON_CANONICAL_BLOCKED_LEGACY`, F42), so every Asgard row is recorded BLOCKED, not run (D-SU3).

## Findings

| # | Finding | Packet |
|---|---|---|
| F1 | No Union world past the SGC opening has had a client playthrough of its restored content. | all drive packets |
| F2 | Three formats exist: SGC owner step files (`uat-U1/U2/U3.md`), Dakara rows only in unified-uat, Omega's UO-U1 owner script, Beta's driver-ready step files. | SU-00 |
| F3 | Beta's step files already follow a driver-plus-verifier flow (`_driver-common.md`, per-row step files, fixtures F-H1 to F-FJ2). They are reused as they are; only a verify file and a header per run are added. | SU-05 |
| F4 | The fixture's mission `state = "completed"` writes the same row the unified-uat SQL for 1647 writes, so M3 to M5 can start from a fixture with no SQL. | D-SU2 |
| F5 | Several status lines are stale: the union planning README says Omega's "PR to main pending" (#1950 merged); unified-uat says the Omega R5 rows are not on main; the SGC step files say to build from `union-sgc/sgc-final`; the Dakara RESUME says the end-of-zone PR is unopened (#1945 merged). | SU-00 (ledgers), SU-99 (unified-uat) |
| F6 | Death and respawn rows (DK-U9, DK-P5, UO-H9, UO-R5-S8/S9, UB-U7) need a way to die on a hub with no hostiles. No self-damage GM command is registered. | D-SU4 |
| F7 | Omega and Beta vendor and trainer rows already exist in UO-U1 R5. They run here, not in the shops campaign. | D-SU6 |
| F8 | Four run groups exceed the 6-row driver cap: DK-U11 to U17 (7 rows), U2 Run A (7), U2 Run B (7) and UO-H1 to H9 (9). Each is split at a point a fixture can reproduce. | SU-DDK3a/b, SU-DL1a/b, SU-DL2a/b, SU-DOM1a/b |

## Decisions

| ID | Decision | Status | Reason |
|---|---|---|---|
| D-SU1 | May agent lab drivers run the Union rows the owner reserved for himself (Dakara owner rule 2026-10-18; UO-U1 rule 1 "agents never use the lab"; SGC step files "agents do not run this"), with the user's OK per lab wave? | **BlockedDecision** (owner) | Same question as Praxis D-SP1; one answer can cover both. If no, the briefs ship as the owner's checklist and the lab waves never open. |
| D-SU2 | M3 to M5 rows start from a fixture with `missions = [{ id = 1647, state = "completed" }]` (plus 1646 completed), not from the unified-uat SQL. | Decided (architect) | F4. If Bra'tac does not offer 1648 after the fixture, DK-U18 and every later M3 to M5 row is BLOCKED and one issue is filed. |
| D-SU3 | Asgard rows are recorded BLOCKED (F42, HELD-B16) and not attempted. A playable Asgard start is outside this campaign. | Decided (architect) | The rows cannot run; attempting them wastes lab time. |
| D-SU4 | A death row runs only if the brief author finds a registered way to die (a GM self-damage or kill command in `crates/cell-console`, or a hostile within reach). Otherwise the row is BLOCKED and one shared issue is filed, cross-referenced with legacy-command-parity (`.kill`). | Decided (architect) | Keeps death rows from being FAIL for a missing tool. |
| D-SU5 | Free Jaffa characters are created by the stager with `lab_create_character` archetype 7, as CS-08 did. Only if that is refused does the brief add a NEEDS_HUMAN setup step for the user during the approved session. | Decided (architect) | Beta's fixtures.md asked for a hand-made character; CS-08 showed the lab can create one. |
| D-SU6 | Vendor and trainer rows that a zone ledger already lists (UO-R5-S6, S12 to S14) run in this campaign. starter-uat-shops covers only what no ledger lists. | Decided (architect) | No row runs twice. |
| D-SU7 | Rows that ask the tester for an opinion (DK-U2, DK-U19 no decline, DK-U24 plain click, DK-U27 fire loop, UB-U1b, UB-U3d) are graded on their server half and listed NEEDS_HUMAN in `signoff.md` for lomiada or the owner. | Decided (architect) | A driver cannot answer "is this acceptable". |
| D-SP2, D-SP3 | Server target and project priority, as in [starter-uat-praxis](../starter-uat-praxis/README.md#decisions). | **BlockedDecision** (owner) | Shared. |

D-SP4 to D-SP7 (issues not fixes, ask first per wave, playthrough order, status docs once) apply here unchanged; the Union close-out SU-99 waits on nothing but SU-98.

## Packet status

Statuses: Ready / BlockedDependency / BlockedDecision / Writing / Review / Integrated / UATPending / Done.

| Wave | Packet | What | Agent | Lab | Status |
|---|---|---|---|---|---|
| 0 | SU-00 | Row inventory `rows.md`; one-line fixes of the stale ledger status lines (F5) | documentation-writer | no | Ready |
| 1a | SU-01 | SGC briefs SU-DSG1 to DSG3 (from uat-U1) | documentation-writer | no | BlockedDependency (SU-00) |
| 1a | SU-02a | Dakara M1-M2 briefs SU-DDK1, DDK2, DDK3a, DDK3b | documentation-writer | no | BlockedDependency (SU-00) |
| 1a | SU-02b | Dakara M3-M5 briefs SU-DDK4 to DDK6 | documentation-writer | no | BlockedDependency (SU-00) |
| 1a | SU-02c | Dakara briefs SU-DDK7, DDK8 (Vocuum pad, SGC end) | documentation-writer | no | BlockedDependency (SU-00) |
| 1a | SU-02d | Dakara placement briefs SU-DDK9, DDK10 (DK-P1 to P9) | documentation-writer | no | BlockedDependency (SU-00) |
| 1a | SU-03a | Human leg briefs SU-DL1a, DL1b, DL2a, DL2b (from uat-U2 runs A and B) | documentation-writer | no | BlockedDependency (SU-00) |
| 1a | SU-03b | Leg briefs SU-DL3, DL4 (from uat-U2 run C, uat-U3) | documentation-writer | no | BlockedDependency (SU-00) |
| 1b | SU-04 | Omega briefs SU-DOM1a, DOM1b, DOM2 to DOM5 (from UO-U1) | documentation-writer | no | BlockedDependency (SU-00) |
| 1b | SU-05 | Beta run headers and verify files SU-DBE1 to DBE7 (reusing the Beta step files) | documentation-writer | no | BlockedDependency (SU-00) |
| 2 | SU-DSG1, DSG2, DSG3, SU-DDK1, DDK2, DDK3a, DDK3b | SGC Human; Dakara Free Jaffa M1-M2 (the Dakara packets in one chain on `p3`) | lab-driver + verifier | **ASK-FIRST** | BlockedDecision (D-SU1, D-SP2) |
| 3 | SU-DDK4 to DDK8, SU-DL1a, DL1b | Dakara M3-M5 and the SGC end; Human 1651 round trip | lab-driver + verifier | **ASK-FIRST** | BlockedDecision (D-SU1, D-SP2) |
| 4 | SU-DDK9, DDK10, SU-DL2a, DL2b, DL3, DL4 | Dakara placement; 1654/1655 both races and the Omega arrival (one chain on `p2`) | lab-driver + verifier | **ASK-FIRST** | BlockedDecision (D-SU1, D-SP2) |
| 5 | SU-DOM1a, DOM1b, DOM2 to DOM5, SU-DBE1, DBE2 | Omega Human, Free Jaffa, re-offer, services, R5; Beta arrival and Human arc | lab-driver + verifier | **ASK-FIRST** | BlockedDecision (D-SU1, D-SP2) |
| 6 | SU-DBE3 to DBE7, retries | Beta rest (cages, Human late arc, Free Jaffa, deaths, layout, parallel clients) and attempt-4 takeovers | lab-driver + verifier | **ASK-FIRST** | BlockedDecision (D-SU1, D-SP2) |
| 7 | SU-91 | TOML specs for passed SGC and Dakara rows | packet-coder | no | BlockedDependency (waves 2-4) |
| 7 | SU-92 | TOML specs for passed Omega and Beta rows | packet-coder | no | BlockedDependency (waves 4-6) |
| 7 | SU-98 | Issue roll-up, sign-off sheet | documentation-writer | no | BlockedDependency (waves 2-6) |
| 7 | SU-99 | Close-out: zone ledgers, unified-uat, status docs once | documentation-writer | no | BlockedDependency (SU-98) |

## Shipping

- **Waves 0, 1a and 1b** ship as **one docs PR** from the integration worktree `su-integration` (branch `starter-uat-union/integration`). Merge order: SU-00 first, then SU-01, SU-02a, SU-02b, SU-02c, SU-02d, SU-03a, SU-03b, SU-04, SU-05 (any order; each writes only its own files).
- **Each lab wave** ends with one docs PR from `su-integration` with that wave's `results/` files, the README rows and the `rows.md` grades. Lab waves write no code.
- **Wave 7** ships as one PR: SU-91, SU-92, then SU-98, then SU-99 last.

## Not in this campaign

- Fixing any product bug found (D-SP4). Building 1647 (Dakara wave 7), 1503, UO-41/42 or an Asgard start.
- Closed-Beta visit 2 and Beta E2 (U-D05). Union Tollana (U-D06).
- Vendor behaviour no ledger lists (sell, buyback, repair, recharge): [starter-uat-shops](../starter-uat-shops/README.md).
