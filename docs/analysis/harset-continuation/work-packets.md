# Harset continuation packet contract

> Type: reference. Audience: coordinator, implementation workers and reviewers. Updated: 2026-10-14.
> Baseline and status: [ledger](README.md). Parent scopes: [four-zone Harset contracts](../four-zone-planning/harset-work-packets.md).

## Dispatch boundary

HA-00b owns this reset and its documentation only. The original HA contracts remain the source for step/dialog references and allowlists; this document records their current dependency reset and exact unresolved interfaces. A candidate ID or coordinate must not become a sealed implementation contract by implication.

Every worker starts at a recorded `content/harset-campaign` integration SHA in its registered worktree, with isolated test DB. Serialize writers to each faction seed. The coordinator merges commits locally in dependency order, performs the measured evidence/implementation pilot, then full local checks and fresh reviewer/fixer rounds per [execution contract](../four-zone-planning/execution-contract.md). Packet branches are not pushed; the complete campaign gets one remote PR. Record actual tokens/context, elapsed work/check time, review effort and pending UAT at each wave. Target about 40k working tokens within the 90k context cap; split at seal time if the contract does not fit.

No coder may choose a missing grant source, reward, historical build, recipient, route, item ID or coordinate. Return that seam as BlockedEvidence or BlockedDecision. IDs in [id-allocation.md](id-allocation.md) must be integrated into the shared table and rechecked before use. Each new beat carries RESTORED_SOURCE, CIMMERIA_RECONSTRUCTION or CIMMERIA_AUTHORING with exact source relation, known build and advisor approval. Giza seed/Python behavior is a reconstruction/reference; seed existence alone does not prove retail provenance.

## Current engine contract the old H ledger cannot supply

- Conditions are evaluated when the event resolves. All matching chains may execute; priority does not make them exclusive. Give sibling chains on one trigger pairwise disjoint persisted mission/step/objective guards. A nested mission event may resolve fresh context during a sequential action list, so put required item/bind cleanup before the mission mutation that emits it.
- `once` is now enforced per `(cell entity, chain)` by `executor/once_gate.rs`; it resets on relog/world change. Persistent one-shots still need a persisted condition. The old “once is dead” rule is superseded.
- Binding targets a template slot, while interaction/death tags match exact actor tags. Restore bindings on `player_loaded`; repaint on the appropriate mission transition/abandon edge and clear old binds before adding new ones. First bound dialog precedence and `last_interaction_target` routing must be checked against current `cell-interactions` code, not inferred from an SQL action list.
- `advance_step` and objective completion have different semantics. Seal the specific mutation at each beat against current mission code and guards; never auto-complete a whole mission to bypass a missing leg. `remove_effect` has a loader/enum variant but no executable dispatch arm on this baseline.
- All four Harset worlds have meshes and are advisory in the seed. Worlds 57/68 are shared; 69/70 remain private per player. Shared Petbe/Anat/guards must not receive hostile/despawn/visibility mutations from one character's quest. Private spawn tags are mission-specific and every wave member has a distinct tag.
- `start_minigame` difficulty is supported; ticket mint still supplies constant competency 1 and abilities 0. H05 needs a specialist state-transfer contract before level-specific hacks promise accurate scaling.

## Packet status reset

Each blocked row names the contract that must be closed before the original implementation allowlist becomes dispatchable. Parent families remain umbrellas; retain the mandatory children below. HA-04/HA-09/HA-11/HA-12 may need further one-mission splits after exact graphs are measured.

| Packet | Missions / scope | State | Gate before seal |
|---|---|---|---|
| HA-00a | 73 census | Done at CSV evidence level | Reviewed dispositions in README; missing build lineage remains held |
| HA-00b | Reset/evidence/restoration ID proposals | Integrated | Wave-0 documentation evidence only; three findings fixed by `22982c17a`, fresh rereview clean. Proposals stay inactive until allocation signoff |
| HA-00c | Authored/repeatable exact IDs | BlockedDependency | CB-N05a insertion contract/catalog evidence; owner story/reward/reset choices and global ID audit |
| HA-01a | Travel/placement | BlockedEvidence | Four Market/Storage thresholds + arrivals, clearance, wrong-world and relog; preserve 6007/1361 linkage |
| HA-01b | 1664, proposed | BlockedEvidence | Castle CS-06 grant edge, Kaerlia identity, dialog/faction graph and placement |
| HA-02 | Navigation/encounter anchors | BlockedEvidence | Current NA40 mesh coverage vs raised telemetry positions and current interior connectivity; non-GM containment proof |
| HA-03 | 1325 | BlockedEvidence | Private Mala'c ritual graph/health/death fallbacks, movement-lock cleanup and remove_effect implementation contract |
| HA-04 | 1343/1347 | BlockedEvidence | Patrol volumes, encounter placement, Lethander clue/dialog; split if both graphs exceed budget |
| HA-05 | 1348 | BlockedEvidence | Private Market waves/count/timing/cleanup, HA-01a/02 |
| HA-06a | 1351 | BlockedEvidence | Exact Opheltes/social branch dialog and outcomes |
| HA-06b | 1352 | BlockedEvidence | Bar, monitoring/body item and hide-body semantics |
| HA-06c | 1353 | BlockedDependency | HA-15a item 4512 plus explicitly reviewed Agnos leg; no invisible auto-completion |
| HA-07 | 1362/1363 | BlockedEvidence | Four anchors, five dart tags/ability binding, consumption and faction guards |
| HA-08a | 1365 | BlockedEvidence | Storage private cast and Tollan technology grant/consume |
| HA-08b | 1375 | BlockedEvidence | Separate Market/Storage mole encounter and location graph |
| HA-08c | 1580 | BlockedEvidence | Private Grogan/NID/container corpse/item mapping |
| HA-09 | 1377/1374 | BlockedEvidence | Replitech device/crates, Haughty/Angry/Bar and scan mapping; H05 competency gap |
| HA-10a | 1371 | BlockedEvidence | Arrest/Bra'hin actor and trigger mapping |
| HA-10b | 1372 | BlockedEvidence | Current move_waypoint/arrival semantics and measured Lethander route; old CA13 status is insufficient |
| HA-10c | 1410 | BlockedEvidence | Four-device placement vs truncated single removal objective; owner/advisor decision |
| HA-11 | 741/1243 | BlockedEvidence | Dawson/tank/witness/footage binding plus six scarab anchors, HA-01a/02 |
| HA-12 | 1240/1241 | BlockedEvidence | Tower/ashrak/beacon private encounter graph and waves |
| HA-13a | 1244 | BlockedEvidence | Exact paperwork search/grant source, adjacent-step guard |
| HA-13b | 1245 | BlockedEvidence | Private Storage Petbe clone template 221; immutable shared spawn 223 and bloody-robes source decision |
| HA-13c | 1246 | BlockedEvidence | Contact and evidence/dialog chain; isolate from 1245 state |
| HA-14 | 1247/1322 | BlockedDependency | HA-15a treat, HA-01a/02 private encounter placements |
| HA-15a | Item 4512 source | BlockedDecision | Source/stock policy, owned acquisition rather than GM grant |
| HA-15b | Faction vendors | BlockedDecision | Exact stock, prices, faction/level bands, template/dialog isolation |
| HA-15c | Trainers | BlockedDecision | Exact approved ability trees and teaching policy |
| HA-16a | World-57 districts | BlockedDecision | Density/roles/placements; separate ambient templates from quest speakers |
| HA-16b | Command Center ambient | BlockedDecision | Interior density/roles and shared-space load contract |
| HA-17a | Two Jaffa stories | BlockedDependency | Restoration UAT + HA-00c sealed IDs AND CB-N05a merged/client proof + premise/reward/presentation approval |
| HA-17b | Two OP-CORE stories | BlockedDependency | HA-00c AND CB-N05a, restoration UAT and catalog/provenance gates plus restored HA-09/10 and services |
| HA-17c | Two Goa'uld stories | BlockedDependency | HA-00c AND CB-N05a, restoration UAT and catalog/provenance gates plus restored HA-13/14 |
| HA-18a | Jaffa repeatable | BlockedDependency | HA-00c AND CB-N05a; owner reset/cooldown/persistence/anti-farm decision after HA-17a |
| HA-18b | OP-CORE repeatable | BlockedDependency | HA-00c AND CB-N05a; same owner reset gates after HA-17b |
| HA-18c | Goa'uld repeatable | BlockedDependency | HA-00c AND CB-N05a; same owner reset gates after HA-17c |
| HA-19 | Integrated UAT/ledger closeout | BlockedDependency | Restoration graphs and authorized client UAT; FZ-99 owns shared status docs |

## HA-00c specialist allocation gate

The [HA-00c contract](../four-zone-planning/harset-work-packets.md#ha-00c--seal-authored-and-repeatable-id-allocations) owns six authored stories and three repeatables. It records exact mission/step/objective/task/dialog-map/dialog/item/chain IDs, client catalog source/build/hash, global seed and active-branch collisions, approved reuse, provenance and owner story/reward/reset decisions. No catalog IDs are sealed by HA-00b. A guessed stock gap, historical mission reuse or server-only row cannot close this gate.

HA-00c is BlockedDependency until CB-N05a's insertion contract and inspected stock catalog establish a safe allocation path. Completion needs the mission advisor's nine graph signoffs and coordinator's shared claims signoff. HA-17a/b/c and HA-18a/b/c remain BlockedDependency on **HA-00c AND CB-N05a** merged, reviewed and client-proven, plus restoration UAT and owner decisions. This docs/research gate uses the parent contract's static checks and scratch duplicate failure proof; implementation retains its ordered lane checks.

## Castle 1664 provisional interface

The owner assigned the exit beat to Castle and the debrief to Harset. This allocation is accepted. The proposed exact grant and geography have only partial evidence and are **not sealed**. The current seed names these steps/objectives:

| Step | Evidence | Proposed receiving beat / unresolved seam |
|---|---|---|
| 5224 (index 0) | “Speak to Sgt. Kaerlia by the Stargate”; objective 6063 blank | Harset gate-side Kaerlia talk. Texts moniker 27315 `DN_npc_int_SgtKaerlia_Harset` confirms Harset identity. No template/spawn exists. Dialog 6169/DSM7016/speaker3219 directs arrival to the Command Center, but the speaker-to-Kaerlia join is unproved. Reserve candidates only. |
| 5225 (index 1) | “Make your way to the Command Center”; 6059 optional path, 6060 mandatory right-click door | Existing 6006/point set 2078 is an enter-region door. Reusing it changes the authored affordance; requires explicit reconstruction review and observable client feedback. |
| 5226 (index 2) | “Report for debriefing”; 6061 Marsh, 6062 Moh'katan, both nonoptional | World 68 actors 344/10 and 343/54 exist. Complete only according to the sealed faction/objective graph; do not drop one mandatory objective without evidence. |

Castle CS-06 must name an exact trigger/chain/action that accepts 1664 once and persists active step 5224 before travel. Neither owner intent nor mission 708 completion is enough to pick that trigger. Harset HA-01b receives active step 5224, handles 5224→5225→5226 and relog/abandon, and must not add a duplicate arrival accept. Chain candidates 6008–6020, spawn 360, template 260 are held pending both signoffs. No public prose may call this proposal “sealed”.

Required future guard: begin at the actual Castle exit edge; duplicate edge does not reward/reaccept; gate/Command Center world transfer preserves the mission; Kaerlia only advances 5224; 5225 advances via the sealed door affordance; 6061/6062 preserve per-character objective progress across relog and cannot complete another faction/step. Remove the actual Castle accept action and the cross-world test must fail. No lab lease is authorized by this evidence packet.

## Castle carry-in closure contract

CS-08 owns upstream changes for 1360/567. Harset keeps receiving chains 6501/6502/6505 and disabled 6503/6504. A future narrow Harset correction packet is justified only by a proved receiving defect; it must not expand into the Castle grant.

For 1360, replay the existing Cellblock item-3730 accept, Castle 4037→4038 and world-hop into Marsh's world-68 receipt. Include wrong 4037, second click, missing/foreign item, the 1361 bind-disjointness state and relog. An executor test must observe the real item removal and mission persistence, rather than injecting 4038 only.

For 567, first prove the item-2698 acquisition and faction-compatible Castle acceptance; then 2000→2012→4039, owned-item consumption and duplicate refusal at Copplemann. Only after that evidence can the receiving pair be enabled together and guarded by a cross-world replay/live-DB test. Item-grant/action side effects route through inventory ownership/persistence review.

## Tollana closure contract

Use the exact [signed actor tags](README.md#tollana-recipient-tags). Tollana owns source and recipients for 603/611/618/621/1626/1632/1643 in its own seed files, using chains 9800–9999. Conditional whole missions 1364/1366 remain solely T-05a/b. Harset does not claim these IDs or implement duplicate recipients.

618 and 621 require a recovered dialog→speaker relation or approved reconstruction before a specific actor is bound. 1626:4838 has two mandatory objectives (5545 Moh'katan, 5908 Marsh); persist each independently. Do not assume 1626:5169 or 1366:4058 is Harset because the name Copplemann appears. Tollana requires its own template/tag for that speaker. Source world, mission/faction/state, item ownership, repeat click, relog and simultaneous differing missions on the same Harset actor must all be guarded. Mutation of the recipient action/state guard must fail the end-to-end replay/live-DB test.

## Validation, telemetry and documentation owed by implementation

Runtime packets choose test types from TESTING.md before coding. Chain-only changes need exact-action replay plus adjacent mission/step/world/faction negatives and a live-DB reachability/item relationship guard; executor changes need side-effect tests. Encounters additionally test disconnect/abandon/relog cleanup and two-player isolation. Placement guards load the actual current mesh with a nonvacuity control. Authored missions need client catalog visibility proof. Repeatables require persistence and once-per-cycle reward guards. Record a reverted-chain/row/arm failure; do not count skip-as-pass or static string checks as those gates.

Follow the parent contracts for ordered fmt, focused lane clippy/nextest, worktree-only reload and live-DB lane filters. Add all changed crates. No compiling command runs outside the lane. HA-00b is docs-only and uses census uniqueness, seed PK/tag collision checks, source/guard inspection, relative-link validation, CRLF validation, markdownlint and `git diff --check`.

Each later brief names telemetry for chain resolution/execution, mission/step/objective progress and refusal reason. Existing executor `Content: accept_mission refused by offer guard`, `once_spent` and mission objective signals can be reused only where they cover the actual edge. The specialist seal must identify the concrete event/message/fields for wrong world/state/faction/already-completed/missing-item refusal; a prose promise is not acceptance. Replay test telemetry where practical and attest runtime logs during authorized UAT.

Implementation updates `docs/content/mission-chains.md` and its worknote. Engine semantic changes additionally update the matching existing mission bible chapter when present; per-mission graphs remain content documentation. HA-19 owns this ledger/UAT checklist, FZ-99 shared gap/status/zone-audit pages. HA-00b integration adds the continuation index link and dated sourced main-session memory pointer. Historical ledger decisions are not rewritten.
