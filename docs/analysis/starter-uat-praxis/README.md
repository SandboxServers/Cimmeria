# Starter-world UAT: Praxis, Castle_CellBlock to Tollana (SP)

> **Type:** campaign ledger (explanation and status). **Audience:** the orchestrator that runs it, the owner and lomiada who answer its decisions.
> **Planned:** 2026-10-27 against `origin/main` `43a424588`, after the Praxis four-zone squash (#1948), Union Omega (#1950) and the Cellblock boot lock (#1952).
> **Companions:** [work-packets.md](work-packets.md) (dispatch rules, the shared UAT contract, one section per packet), [RESUME.md](RESUME.md), [unified-uat.md](../../guides/unified-uat.md), [hub-pass UAT](../hub-pass-uat/README.md) (the pipeline this copies).

## Sibling campaigns (lomiada's starter-world focus, 2026-10-27)

| Campaign | What it covers |
|---|---|
| **starter-uat-praxis** (this one) | Lab UAT of the Praxis chain: Castle_CellBlock (12), Castle (8), Harset (57, 68-70), Tollana (19), and the hand-offs between them |
| [starter-uat-union](../starter-uat-union/README.md) | Lab UAT of the SGU chain: SGC_W1 (58), Dakara_E1 (61/62), Omega (18/80), Beta Site E1 (23) |
| [starter-uat-shops](../starter-uat-shops/README.md) | Vendors, trainers, sell, buyback, repair and recharge in both chains, and the worlds that have none |
| [debug-armory](../debug-armory/README.md) | Debug Area world 1300: wear and fire any weapon, wear any armor, cast every archetype ability, with generated lab specs |

The four campaigns share no files except `docs/guides/unified-uat.md`, which only each close-out packet edits. They can run in any order. The lab takes one campaign's wave at a time.

## Purpose

Contributor lomiada, 2026-10-27: *"all current develop effort should go in the starting worlds up to Tollana / beta site (for each faction): the worlds, gameplay, shops and so on. When possible the dev map should have a function for us to wear and test every weapon and armor as well as abilities."*

For Praxis, "up to Tollana" means the four-zone chain. Every one of those zones merged in #1948 under the owner's rule "no lab until the end" (D-CA11, D-CC13, D-T009), so each zone ledger still owes a pile of `UATPending` client rows. This campaign is that end UAT. It drives the real client through the lab, grades every row against server evidence, files a GitHub issue for each failure, and turns every row that passes into a repeatable TOML spec. It writes no game code. Fixing what it finds is follow-up work that the issues seed.

## Worlds in scope and what is known today

"Tested" means a person or the lab saw it in a real client and the server agreed. "Server-only" means unit, replay or live-DB tests. Anything else is untested.

| Order | World (id) | Reached by | Tested in a client | Server-only, never seen in a client | Known broken or missing |
|---|---|---|---|---|---|
| 1 | Castle_CellBlock (12) | fresh Praxis Commando start (`char_creation` 13, `PRA_OPCORE_COMMANDO`) | 2026-09-18 colo playtest. 2026-10-21 colo run: T01-T22 and T25 confirmed **server-side only**, visuals pending ([unified-uat](../../guides/unified-uat.md)) | CB-U01 to CB-U12 ([cellblock-continuation](../cellblock-continuation/README.md), [owner-lab-checklist](../cellblock-continuation/owner-lab-checklist.md)). Mission 62060 confirmed server-side, but no portrait and dialog 60151 threw a client syntax error | Pistol #1667; LMG effect 644 inert; the exit has no ring animation (needs the [ring-transport](../ring-transport-cellblock-castle/README.md) client patch); K5, K7, K8, K9, K11; CB-N05b waits on Harset HA-1360 |
| 2 | Castle (8) | cross-world ring, chain 1109 | 2026-09-18 colo playtest only. Quartermaster store-open seen server-side 2026-10-21 | everything in [end-uat.md](../castle-continuation/uat/end-uat.md) (A1-A5, D1-D7, Mission 1521 S1-S8, hub quartermaster S1-S5, HB1-HB2, RQ-R2 and more); population CP1-CP20 is seed-only; FZ-U1 (mission 567) owed | K20 (friendly NPCs give no kill credit); interior and exterior navmesh not connected; a fresh client logging straight into Castle hangs (#756); CS-02/1520, CS-03 held BlockedEvidence |
| 3 | Harset (57; interiors 68, 69, 70) | Castle DHD; chain 1357 grants address 3 | **never played in a client** ("rebuilt, unplayed", [project-status](../../project-status.md)) | H20, H31, H40, H41, HA-1360a/b (Frost's Letter hand-in), the 1664 Kaerlia chain (built by FZ-18 to FZ-27); its UAT row FZ-U2 owed, split here into FZ-U2a to FZ-U2h ([harset-continuation](../harset-continuation/README.md)) | Vendors HA-15b and trainers HA-15c not built; Market has no navmesh; door pairs unplaced; 39 older-revision rows held; H21-H47 held |
| 4 | Tollana (19) | Harset hand-off | **never played in a client** | the mission 568 pilot (chains 9001-9018) and phasing PH-01a/b: nextest and live-DB only. Owner rows P1-P10, U1-U7, T18 ([tollana-continuation](../tollana-continuation/README.md)); no Tollana section in unified-uat | the seed has only a DHD and two rings plus the pilot; all 51 census missions held; the item-backed 568 waits on HA-1360a; the RESUME still says "local only, no PR" although #1948 merged |

The Goa'uld branch through Castle mission 1520 is held BlockedEvidence (CS-02) and is out of scope.

## Findings

| # | Finding | Packet |
|---|---|---|
| F1 | No Praxis world past Cellblock has had a client playthrough of its current content. Every continuation row is untested in a client. | SP-D* (all drive packets) |
| F2 | The rows exist but are scattered across four ledgers in four formats (checklist, end-uat table, ledger rows, owner P/U/T lists). Nobody can see the whole owed list. | SP-00 |
| F3 | The hand-offs (Cellblock→Castle ring, Castle→Harset gate with Frost's Letter, Harset→Tollana) are each tested only from one side, if at all. | SP-DL1 to SP-DL3 |
| F4 | The lab pipeline that worked (hub-pass: Haiku driver ×3, Sonnet takeover, independent verifier against ClickHouse and the DB) needs written briefs; none exist for these zones. | SP-01 to SP-05 |
| F8 | FZ-U2 (the Harset 1664 row in unified-uat) is one row with about 20 clauses, two report orders, abandons at every step and a second race. A single packet cannot hold it under the 5-row, 25-call cap. | SP-00 split, SP-DHA3 to DHA5 |
| F5 | Rows that pass by hand are lost again at the next regression. The TOML spec runner (`lab uat`) can replay them. | SP-91, SP-92 |
| F6 | After every colo deploy, seeded characters start as Praxis Commandos in Castle_CellBlock (K24). Later worlds need a fixture (world, position, missions, items) or a GM teleport; lab fixtures have no level, cash or ability fields, so setup uses `.givexp`, `/gmgivecash` and `/gmgiveability`. | contract § C4 |
| F7 | The lab-server MCP refused connections when this plan was written (2026-10-27). Verifiers need either it or ClickHouse over ssh (`reference_signoz_clickhouse_via_ssh`). | contract § C3 |

## Decisions

| ID | Decision | Status | Reason |
|---|---|---|---|
| D-SP1 | May agent lab drivers run the client rows that D-CA11, D-CC13 and D-T009 reserved for the owner ("no lab until the end", owner runs every row), with the user's OK per lab session? | **BlockedDecision** (owner) | Those decisions were the owner's. The user framed this plan as lab UAT, but only the owner can lift his own reservation. Default if yes: drivers run every row; rows that need a human eye are graded NEEDS_HUMAN and listed in the sign-off sheet. If no: SP-01 to SP-05 still ship and the briefs become the owner's checklist. |
| D-SP2 | Which server the lab plays against: the colo after a release that contains #1948/#1950/#1952, or a local server built from `origin/main` at a recorded SHA. | **BlockedDecision** (owner) | A colo deploy needs a release, and the colo DB is rebuilt from seeds on every deploy (good for fresh characters). A local server needs no release but its telemetry lands only locally. Architect default: local server at a recorded SHA, so the campaign does not wait on a release. |
| D-SP3 | Does lomiada's direction ("all current develop effort should go in the starting worlds") become the project's priority, pausing unrelated campaigns? | **BlockedDecision** (owner) | lomiada is a contributor, not the owner. This campaign does not depend on the answer; its scheduling does. |
| D-SP4 | Product bugs found here are filed as issues, not fixed in this campaign. | Decided (architect) | Keeps every packet docs-only and inside the 40k budget; a fix campaign is sliced from the issues afterwards. |
| D-SP5 | Each lab wave asks the user first, every time. The four-zone standing approval O14-08 does not cover this campaign. | Decided (architect, from the lab-uat skill and `feedback_ask_before_lab`) | O14-08 was granted to the four zone sessions, which have closed. |
| D-SP6 | Rows are run in playthrough order (Cellblock, Castle, Harset, Tollana), so a blocker early in the chain is found before later rows depend on it. | Decided (architect) | |
| D-SP7 | Status docs (`docs/project-status.md`, `docs/gap-analysis*`) change once, in SP-99, and only after the four-zone FZ-99 close-out has landed its own lines. | Decided (architect) | FZ-99 still owes the Praxis rows; two packets must not write them. |

## Packet status

Statuses: Ready / BlockedDependency / BlockedDecision / Writing / Review / Integrated / UATPending / Done.

| Wave | Packet | What | Agent | Lab | Status |
|---|---|---|---|---|---|
| 0 | SP-00 | Row inventory `rows.md`; fix the stale Tollana RESUME line | documentation-writer | no | Ready |
| 1 | SP-01 | Cellblock driver and verify briefs (SP-DCB1 to DCB3) | documentation-writer | no | BlockedDependency (SP-00) |
| 1 | SP-02a | Castle briefs part 1 (SP-DCA1 to DCA4) | documentation-writer | no | BlockedDependency (SP-00) |
| 1 | SP-02b | Castle briefs part 2 (SP-DCA5 to DCA7) | documentation-writer | no | BlockedDependency (SP-00) |
| 1 | SP-03a | Harset briefs part 1 (SP-DHA1, DHA2) | documentation-writer | no | BlockedDependency (SP-00) |
| 1 | SP-03b | Harset briefs part 2: the FZ-U2 split (SP-DHA3 to DHA5) | documentation-writer | no | BlockedDependency (SP-00) |
| 1 | SP-04 | Tollana briefs (SP-DTO1 to DTO4) | documentation-writer | no | BlockedDependency (SP-00) |
| 1 | SP-05 | Hand-off leg briefs (SP-DL1 to DL3) | documentation-writer | no | BlockedDependency (SP-00) |
| 2 | SP-DCB1, DCB2, DCB3, SP-DL1, SP-DCA1, SP-DCA2 | Cellblock, ring to Castle, Castle part 1 | lab-driver + verifier | **ASK-FIRST** | BlockedDecision (D-SP1, D-SP2) |
| 3 | SP-DCA3 to DCA7, SP-DL2 | Castle part 2, Castle→Harset with Frost's Letter | lab-driver + verifier | **ASK-FIRST** | BlockedDecision (D-SP1, D-SP2) |
| 4 | SP-DHA1 to DHA5, SP-DL3 | Harset (DHA3, DHA4, DHA5 one after another on `p3`), Harset→Tollana | lab-driver + verifier | **ASK-FIRST** | BlockedDecision (D-SP1, D-SP2) |
| 5 | SP-DTO1 to DTO4, retries | Tollana, one packet at a time on `default`; the two-player packet last, alone; any attempt-4 takeovers | lab-driver + verifier | **ASK-FIRST** | BlockedDecision (D-SP1, D-SP2) |
| 6 | SP-91 | TOML specs for passed Cellblock and Castle rows | packet-coder | no | BlockedDependency (waves 2-3) |
| 6 | SP-92 | TOML specs for passed Harset, Tollana and leg rows | packet-coder | no | BlockedDependency (waves 4-5) |
| 6 | SP-98 | Issue roll-up and sign-off sheet | documentation-writer | no | BlockedDependency (waves 2-5) |
| 6 | SP-99 | Close-out: zone ledgers, unified-uat, status docs once | documentation-writer | no | BlockedDependency (SP-98, FZ-99) |

## Shipping

- **Wave 0 and wave 1** ship as **one docs PR** from the integration worktree `sp-integration` (branch `starter-uat-praxis/integration`). Merge order into it: SP-00, then SP-01, SP-02a, SP-02b, SP-03a, SP-03b, SP-04, SP-05 (any order; each writes only its own brief files and its own `rows.md` lane column cells are already fixed by SP-00).
- **Each lab wave** ends with one docs PR from `sp-integration` carrying the wave's results: `results/<packet>.md` files, the README status rows, and the `rows.md` grade column. Lab waves write no code.
- **Wave 6** ships as one PR: SP-91 and SP-92 (spec files), then SP-98, then SP-99 last.

## Not in this campaign

- Fixing any product bug found (D-SP4). Building Harset vendors and trainers (that is [harset-continuation](../harset-continuation/README.md) HA-15b/c); shop UAT lives in [starter-uat-shops](../starter-uat-shops/README.md).
- The Goa'uld branch (CS-02/1520), held Tollana census missions, Harset's held H21-H47, the ring-transport client patch.
- Union worlds: see [starter-uat-union](../starter-uat-union/README.md).
