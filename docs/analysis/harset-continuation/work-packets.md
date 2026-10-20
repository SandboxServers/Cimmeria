# Harset continuation packet contract

> Type: reference. Audience: coordinator, implementation workers and reviewers. Updated: 2026-10-18.
> Baseline and status: [ledger](README.md). Parent scopes: [four-zone Harset contracts](../four-zone-planning/harset-work-packets.md). Owner decisions of 2026-10-18: [O18 register](worknotes/owner-decisions-2026-10-18.md); the packets they unblock are [research waves R1 and R2](#research-waves-r1-and-r2-after-the-2026-10-18-decisions).

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
| HA-00c | Authored/repeatable exact IDs | BlockedEvidence | CB-N05a is merged; inspect stock catalog and Cellblock's first authored client proof, then perform current global ID audit under O14-04/10/13; static half done 2026-10-18 at tip `3e5f503e8`, recorded `2785df721`, zero collisions across 109 refs, see [HA-00c result](worknotes/HA-00c.md) (no IDs allocated); first authored client proof stays BlockedEvidence |
| HA-1360a1 | Atomic base transaction and writer fence | Integrated | Reviewed transaction and stale-writer guards; 670 nextest and seven live-DB tests passed |
| HA-1360a2a | Session token world-entry delivery and lifetime | Integrated | Reviewed entry-epoch and replacement guards; default and `lab-chaos` lane suites passed |
| HA-1360a2b | Correlated request/result routing and A1 invocation | Integrated | [Packet and three reviewed repairs](worknotes/HA-1360a-seal.md#ha-1360a2--internal-transport-and-session-fence); default, `lab-chaos` and focused live-DB lane checks passed |
| HA-1360a3a | Strict mandatory action and loader | Integrated | `dcf7423bf`; 299 nextest passed, independent review found no code defect; action/telemetry references updated |
| HA-1360a3b | Pending request and same-attempt retry | Integrated | [Reviewed packet and repair](worknotes/HA-1360a-seal.md#ha-1360a3--strict-action-pending-state-and-continuation); 1,608 affected-crate tests passed |
| HA-1360a3c | Correlated result and continuation | Integrated (campaign tip `e7dd74336`) | [Exact result manifest](worknotes/HA-1360a-seal.md#ha-1360a3--strict-action-pending-state-and-continuation); dispatch from integrated a3b |
| HA-1360b | Marsh receiving seed and cross-zone proof | Integrated: b1 merge `e1c03b107`, b2 merge `9455f2af2` ([HA-1360b seal](worknotes/HA-1360b-seal.md)); owner-manual client UAT pending | HA-1360a integrated; CS-08a1 crossing carried as `d2a6913cd`; the actual client crossing and receiving graph acceptance stay owner-manual |
| HA-1360a2c | World-entry continuation fence (preexisting race) | Integrated by merge `41d099ef5` (branch `harset/ha-1360a2c-entry-fence`: A `6aded1bb6`, B `4a9cd55e2`); lane fmt, clippy and nextest passed | [HA-1360a2c seal](worknotes/HA-1360a2c-seal.md): step A stage guards, step B send-chokepoint entry guard; base was the campaign tip at dispatch |
| HA-01a | Travel/placement | BlockedEvidence | Interior arches and landing estimates found offline ([BSP door probe](worknotes/HA-01a-bsp-door-probe.md)). Remaining: HA-01a1 landing geometry, Market exit arch 1006 or 1551 (one client glance, owner-manual), the two world-57 exterior door positions (hand reading or exterior probe); then clearance, wrong-world and relog; preserve 6007/1361 linkage |
| HA-01a1 | Interior landing geometry check | Done 2026-10-18 (worknote `caafb2e9f`, reviewed `7f9803833`); PROVISIONAL, untested in game | [Sealed below](#ha-01a1--interior-landing-geometry-check); offline lane build of the extractor, no client run |
| HA-01b | 1664 debrief | BlockedDependency | O18-01 (provisional Kaerlia), O18-02 (no 1399) and O18-12 (right-click door) decided. Waits on HA-01b1 and HA-01b2, Castle CS-06's integrated grant edge (chain 1398), a maintainer decision only if HA-01b1 finds no server-only door, then graph seal and ID allocation |
| HA-01b1 | 5225 right-click door feasibility | Done 2026-10-18 (`caafb2e9f`): server-only feasible on paper; client cursor unverified | [Sealed below](#ha-01b1--step-5225-right-click-door-feasibility) |
| HA-01b2 | Provisional Kaerlia NPC design | Done 2026-10-18 (`caafb2e9f`), PROVISIONAL | [Sealed below](#ha-01b2--provisional-kaerlia-npc-design-card) |
| HA-02 | Navigation/encounter anchors | BlockedEvidence | Offline pass done ([revalidation](worknotes/HA-02-offline-revalidation.md)): pen floor and upper wing connected, E03 fails, E13–E15 sit above Storage region 2122. Remaining: spawn 308 and the five raised points re-measured, ordinary-player containment walk (owner-manual), coordinator calls on E03 and the region raise after HA-01a1 |
| HA-03a | 1325 and Mala'c duel design (O18-10) | Done 2026-10-18 (`caafb2e9f`), PROVISIONAL | [Sealed below](#ha-03a--mission-1325-and-malac-duel-provisional-design) |
| HA-03b | Seal the `remove_effect` executor arm | Done 2026-10-18 (`4c9af5f34`, [contract](worknotes/HA-03b-combat-contract.md)); player_died trigger stays a MAINTAINER DECISION. Original gate: | HA-03a names the effects and targets to remove; then combat-systems-advisor writes the Haiku contract for HA-03c |
| HA-03c | Implement the `remove_effect` arm | Integrated (merge `98b9b8741`; impl `8824244d1`, tests `b72378e94`, review fix `80ecedb5c`; lane checks passed, owner-manual UAT n/a). Original gate: | HA-03b sealed contract (packet-coder, lane checks, revert proof) |
| HA-03d | 1325 seed graph (parent HA-03 allowlist) | BlockedEvidence | HA-01a Market travel, HA-02 duel position, HA-03a design reviewed, HA-03c integrated, chain IDs allocated in 6311–6330 after a collision check. 1326 stays gated on 1325 completion |
| HA-04 | 1343/1347 | BlockedEvidence | Patrol volumes, encounter placement, Lethander clue/dialog; split if both graphs exceed budget |
| HA-05 | 1348 | BlockedEvidence | Private Market waves/count/timing/cleanup, HA-01a/02 |
| HA-06a | 1351 | BlockedEvidence | Exact Opheltes/social branch dialog and outcomes |
| HA-06b | 1352 | BlockedEvidence | Bar, monitoring/body item and hide-body semantics |
| HA-06c | 1353 | BlockedDependency (deferred) | O18-06 defers Agnos and its items until that world is in scope; O18-03 forbids invisible completion of the Agnos leg. Also HA-15a treat choice |
| HA-07 | 1362/1363 | BlockedEvidence | Four anchors, five dart tags/ability binding, consumption and faction guards; consumes HA-15d and HA-15e results for 2734 and 4690 |
| HA-08a | 1365 | BlockedEvidence | Storage private cast and Tollan technology grant/consume; consumes HA-15e result for 2743 |
| HA-08b | 1375 | BlockedEvidence | Separate Market/Storage mole encounter and location graph |
| HA-08c | 1580 | BlockedEvidence | Private Grogan/NID/container corpse/item mapping |
| HA-09 | 1377/1374 | BlockedEvidence | Replitech device/crates, Haughty/Angry/Bar and scan mapping; H05 competency gap; consumes HA-15e result for 4396 |
| HA-10a | 1371 | BlockedEvidence | Arrest/Bra'hin actor and trigger mapping |
| HA-10b | 1372 | BlockedEvidence | Current move_waypoint/arrival semantics and measured Lethander route; old CA13 status is insufficient |
| HA-10c | 1410 | BlockedEvidence | Four-device placement vs truncated single removal objective; owner/advisor decision |
| HA-11 | 741/1243 | BlockedEvidence | Dawson/tank/witness/footage binding plus six scarab anchors, HA-01a/02; consumes HA-15d and HA-15e results for 2818 and 2820 |
| HA-12 | 1240/1241 | BlockedEvidence | Tower/ashrak/beacon private encounter graph and waves |
| HA-13a | 1244 | BlockedEvidence | Exact paperwork search/grant source, adjacent-step guard |
| HA-13b | 1245 | BlockedEvidence | Private Storage Petbe clone template 221; immutable shared spawn 223 and bloody-robes source decision |
| HA-13c | 1246 | BlockedEvidence; theme leg deferred | Contact and evidence/dialog chain; isolate from 1245 state. Themes 5703/5704/5706 were presented as Agnos items, so O18-06 defers that leg; reclassifying them needs owner direction |
| HA-14 | 1247/1322 | BlockedDependency | HA-15a treat, HA-01a/02 private encounter placements |
| HA-15a | Treat item source | BlockedDecision | Still owner: 4512 or 2714, and bought or granted (not answered on 2026-10-18). O18-03 makes it a real collectible item either way. Audit: [HA-15 audit](worknotes/HA-15-audit.md#13-result-audit-re-run-2026-10-18) |
| HA-15b | Faction vendors | BlockedDependency | O18-04: Omega Site is the reference. Waits on HA-15f, safe placements (HA-16/HA-01a evidence), then an items-advisor seal of stock lists and a price proposal inside O14-12; vendor engine has no faction gate and ignores `max_stack_size` |
| HA-15c | Trainers | BlockedDependency | O18-04/05: AI-derived provisional trainers. Waits on HA-15g; trainer rows land through `tools/ability_trees/generate_seed.py` and its workbook, never by hand |
| HA-15d | Mission-item grant design (O18-03) | Done 2026-10-18 (`caafb2e9f`), PROVISIONAL | [Sealed below](#ha-15d--mission-item-grant-design) |
| HA-15e | Event-5 item function audit (O18-07) | Done 2026-10-18 (`caafb2e9f`) | [Sealed below](#ha-15e--event-5-item-function-audit) |
| HA-15f | Omega Site population and service reference (O18-04) | Done 2026-10-18 (`caafb2e9f`) | [Sealed below](#ha-15f--omega-site-population-and-service-reference) |
| HA-15g | Provisional trainer design (O18-05) | Placement plan done 2026-10-18 (`4c9af5f34`, [HA-15g note](worknotes/HA-15g-service-npc-placement.md)); trainer trees still owed. Original gate: | HA-15f integrated; combat-systems-advisor drafts trees from the generator workbook and existing ability data |
| PU-05 | HA-01b graph seal (1664) | PROVISIONAL offline draft, `740c619fb` (PU-05 verdict ship); awaits merge into the combined branch | [Worknote](worknotes/HA-01b-graph-seal.md) (branch `four-zone-review/PU-05-ha01b-seal`). Owner: O18-02 abandon path; D-PU1 whether 1664 ships in this PR (recommend no) |
| PU-06 | H05 minigame competency and abilities contract | PROVISIONAL offline contract, `818f31ec1`; fix applied in a later commit; awaits merge | [Worknote](worknotes/H05-minigame-competency-contract.md). HA-05 and HA-09 stay BlockedEvidence |
| PU-07 | HA-15g trainer trees (draft input for HA-15c) | PROVISIONAL offline draft, `d035385e9`, fix `bfd4ab43d`; awaits merge | [Worknote](worknotes/HA-15g-trainer-trees.md). Roster reconciliation is a coordinator call under O14-04 |
| PU-08 | HA-15b stock lists and price proposal (draft) | PROVISIONAL offline draft, `545f95812`, nits fixed `623fbe722`; awaits merge | [Worknote](worknotes/HA-15b-stock-draft.md). Owner: D-PU2 prices inside O14-12; HA-15a treat. Maintainer: vendor faction gating |
| HA-16a | World-57 districts | BlockedEvidence | O14-12 permits conservative density; measure safe roles/placements and separate ambient templates from quest speakers |
| HA-16b | Command Center ambient | BlockedEvidence | O14-12 permits conservative density; measure interior placements and shared-space load |
| HA-17a | Two Jaffa stories (umbrella) | Split | O18-11: story 1 is the pilot (HA-17a1, HA-17a2); story 2 is HA-17a3 |
| HA-17a1 | Pilot graph design card, Jaffa story 1 | Done 2026-10-18 (`caafb2e9f`), PROVISIONAL; awaits owner sign-off | [Sealed below](#ha-17a1--pilot-graph-design-card-jaffa-story-1); docs only, no ID |
| HA-17a2 | Pilot implementation | BlockedDependency + BlockedDecision | HA-17a1 graph sign-off, then IDs allocated after a fresh collision check (O18-11, O14-04). Owner: does the pilot still wait for the restored Jaffa arc UAT and Cellblock's first authored client proof, or may it be that proof? Then the owner-run client check |
| HA-17a3 | Jaffa story 2 | BlockedDependency | Pilot passes the owner-run client check (O18-11), then its own graph and collision checks; prior HA-17a gates |
| HA-17b | Two OP-CORE stories | BlockedDependency | Pilot client check (O18-11); then HA-00c, restoration UAT and HA-09/10/services. Drafts may follow O18-05 |
| HA-17c | Two Goa'uld stories | BlockedDependency | Pilot client check (O18-11); then HA-00c, restoration UAT and HA-13/14 |
| HA-18a | Jaffa repeatable | BlockedDependency | Pilot client check (O18-11); O14-13 and O18-05 delegate the provisional reset design; the `repeats` UPSERT gap needs its own arm; HA-17a3 |
| HA-18b | OP-CORE repeatable | BlockedDependency | Same as HA-18a; HA-17b |
| HA-18c | Goa'uld repeatable | BlockedDependency | Same as HA-18a; HA-17c |
| HA-19 | Integrated UAT/ledger closeout | BlockedDependency | Restoration graphs and authorized client UAT; FZ-99 owns shared status docs |

## HA-00c specialist allocation gate

The [HA-00c contract](../four-zone-planning/harset-work-packets.md#ha-00c--seal-authored-and-repeatable-id-allocations) owns six authored stories and three repeatables. It records exact mission/step/objective/task/dialog-map/dialog/item/chain IDs, client catalog source/build/hash, global seed and active-branch collisions, approved reuse, provenance and owner story/reward/reset decisions. No catalog IDs are sealed by HA-00b. A guessed stock gap, historical mission reuse or server-only row cannot close this gate.
 The static half (stock catalog, collision audit, signoff list; no IDs allocated) is sealed in [worknotes/HA-00c.md](worknotes/HA-00c.md); Cellblock's first authored client proof stays BlockedEvidence.
CB-N05a is merged, but HA-00c remains BlockedEvidence until the stock catalog is inspected, Cellblock's first authored quest passes client journal/dialog/advance/relog proof, and the current active-branch collision audit is documented. O14-04/10/13 delegate individual allocation, lore-safe authored content and repeatable policy with domain review; no further owner vote is required inside those bounds. Completion needs the mission advisor's nine graph signoffs and coordinator's shared claims signoff. HA-17a/b/c and HA-18a/b/c remain BlockedDependency on HA-00c, that first authored client proof and restoration UAT. This docs/research gate uses the parent contract's static checks and scratch duplicate failure proof; implementation retains its ordered lane checks.

## Castle 1664 provisional interface

The owner assigned the exit beat to Castle and the debrief to Harset. This allocation is accepted. The proposed exact grant and geography have only partial evidence and are **not sealed**. The current seed names these steps/objectives:

| Step | Evidence | Proposed receiving beat / unresolved seam |
|---|---|---|
| 5224 (index 0) | “Speak to Sgt. Kaerlia by the Stargate”; objective 6063 blank | Harset gate-side Kaerlia talk. Texts moniker 27315 `DN_npc_int_SgtKaerlia_Harset` confirms Harset identity. No template/spawn exists. Dialog 6169/DSM7016/speaker3219 directs arrival to the Command Center, but the speaker-to-Kaerlia join is unproved. Reserve candidates only. **O18-01:** create a new provisional Harset NPC; gender, appearance, clothing and placement are chosen from existing assets and labelled PROVISIONAL (design: HA-01b2). |
| 5225 (index 1) | “Make your way to the Command Center”; 6059 optional path, 6060 mandatory right-click door | Existing 6006/point set 2078 is an enter-region door. **O18-12:** keep the right-click door; reuse 6006 only if it is shown to give the same interaction without changing the step. HA-01b1 finds a server-authoritative mechanism or stops at a maintainer decision if only a client change works. |
| 5226 (index 2) | “Report for debriefing”; 6061 Marsh, 6062 Moh'katan, both nonoptional | World 68 actors 344/10 and 343/54 exist. Complete only according to the sealed faction/objective graph; do not drop one mandatory objective without evidence. |

Castle CS-06 must name an exact trigger/chain/action that accepts 1664 once and persists active step 5224 before travel. Neither owner intent nor mission 708 completion is enough to pick that trigger. Harset HA-01b receives active step 5224, handles 5224→5225→5226 and relog/abandon, and must not add a duplicate arrival accept. Chain candidates 6008–6020, spawn 360, template 260 are held pending both signoffs. No public prose may call this proposal “sealed”. **O18-02:** catch-up chain 1399 is omitted; the 1664 graph neither restores nor replaces its regrant after abandon, and the abandon path without it is still open.

Required future guard: begin at the actual Castle exit edge; duplicate edge does not reward/reaccept; gate/Command Center world transfer preserves the mission; Kaerlia only advances 5224; 5225 advances via the sealed door affordance; 6061/6062 preserve per-character objective progress across relog and cannot complete another faction/step. Remove the actual Castle accept action and the cross-world test must fail. No lab lease is authorized by this evidence packet.

## Castle carry-in closure contract

CS-08 owns upstream changes for 1360/567. Owner decisions on 2026-10-14 select CS-08a's Castle-origin `stargate_crossed` Castle→Harset reconstruction, chain 1366 (single ID approved), for 1360 step 4037→4038, with letter 3730 retained. Marsh must require the letter before completion; HA-1360a and HA-1360b explicitly expand the work into an engine prerequisite and Harset receipt. Harset keeps receiving chains 6501/6502/6505; 6503/6504 were disabled at this decision and ship enabled since FZ-12 (D-FZ1). Neither packet adds a Castle grant or a second crossing advance.

For 1360, replay the existing Cellblock item-3730 accept, Castle 4037→4038 and world-hop into Marsh's world-68 receipt. Include wrong 4037, second click, missing/foreign item, the 1361 bind-disjointness state and relog. An executor test must observe the real item removal and mission persistence, rather than injecting 4038 only.

For 567, first prove the item-2698 acquisition and faction-compatible Castle acceptance; then 2000→2012→4039, owned-item consumption and duplicate refusal at Copplemann. Only after that evidence can the receiving pair be enabled together and guarded by a cross-world replay/live-DB test. Item-grant/action side effects route through inventory ownership/persistence review. **O18-09:** 2698 is a collectible world item, not a silent mission grant. Castle CS-08 owns the pickup location and route, which still need source verification; 6503/6504 were held disabled until that route was established; FZ-12 (D-FZ1) enabled them with the Castle leg. Harset authors nothing for the pickup.

## HA-1360a — authoritative letter turn-in prerequisite

**Status: Integrated through b2 (`9455f2af2`) and a2c (`41d099ef5`); owner-manual UAT pending.** Audience: engine implementer and database, authority, mission and testing reviewers. The owner requires an owned letter for Marsh's completion. A1 transaction/receipt, A2a session-token/entry-epoch, A2b plugin route/client sync, A3a strict action and A3b pending/retry are integrated with reviewed tests. A3c (merge `e7dd74336`), b1 (`e1c03b107`) and b2 (`9455f2af2`) are integrated; owner-manual UAT is pending. A2c is sealed in [its worknote](worknotes/HA-1360a2c-seal.md) and separately tracks a preexisting general post-handoff race. [The engine worknote](worknotes/HA-1360a-seal.md) supersedes the provisional design details below and records the current exact manifests. This is a bounded mission-1360 seam, not a redesign of every inventory or mission writer. Split any child that exceeds the shared 40k working-token / 90k context envelope.

### Current source evidence

| Evidence you can inspect | What it establishes / limit |
|---|---|
| `db/resources/Content/Seed/harset_opcore_chains.sql`, chain 6501 | World 68, active 1360/4038, disjoint from active 1361/4694. Actions display 4576, remove 3730 ×1, clear DSM 5356 slot 10, complete 1360. Its comment explicitly permits missing-letter completion and contains an outdated no-container-filter claim. |
| `crates/cell-content/src/cell/content/executor/inventory.rs::remove`, `executor/mod.rs::execute_actions`, `executor/mission.rs::complete` | Removal sends a cell→base request without a mutation verdict; the action list continues. Completion changes cell state, sends a separate MissionUpdate and fires follow-up chains. Emitted removal intent cannot prove consumption. |
| `crates/content-engine/src/conditions/mod.rs::HasItem`, `src/loader/condition.rs`, `src/loader/mod.rs`; cell-content `event_dispatch/interaction.rs` | HasItem reads a free-form count; the loader has no `has_item` arm and skips unknown conditions. Interaction does not supply authoritative inventory counts. A seed-only possession condition cannot close this defect. |
| `crates/base-methods/src/base/world_entry/methods/inventory/core/{remove_by_type,access}.rs` | Existing removal selects an owned, accessible row under `FOR UPDATE`, separately from mission persistence. It is not the proposed strict mission transaction. By-type content removal uses `VaultAccess::NO_SESSION`: containers 1–15, excluding buyback 16, bank 17, organization vault and escrow. |
| `crates/base-session/src/base/inventory_locks.rs`; base-methods `missions/mod.rs::handle_mission_update`; base-session `outbox/mod.rs` | Shared advisory locks precede inventory/player rows. MissionUpdate uses an unconditional UPSERT. Outbox delivery is keyed by entity ID; durable delivery alone cannot prove character/session-safe result recovery or stale-update exclusion. |

These observations established the receiving defect before the engine packets. A1's implementation and live-DB evidence are in the current worknote; historical client behavior and live-client receipt evidence remain unproved.

### Seal gates and required signoffs

| Gate | Contract you must close | Required signoff |
|---|---|---|
| Authority | Bind actor to trusted current character/session; trusted content names mission 1360, expected active step 4038, item type 3730 and positive quantity 1. Revalidate eligible turn-in state, world/recipient and 1361 overlap at the authoritative boundary; do not trust client counts or instance IDs. Reject unreadable/locked mission state, forged or stale actor/state, foreign/missing/inaccessible/zero-count items. | server-authority-enforcer, mission-systems-advisor |
| Atomicity and locks | Exact owned accessible instance consumption and persisted mission completion must commit together or both roll back. Reuse `MissionInstance::complete` semantics for objectives, completed steps and repeats. Require exact quantity and mutation row count; no clamped consume. Publish one lock order compatible with inventory, player, mission, abandon and competing removal writers, including all error paths. | database-persistence, server-authority-enforcer, mission-systems-advisor |
| Stale writers / CAS | Enumerate MissionUpdate, abandon, restore/relog and competing completion writers. Either prove ordering/serialization excludes every stale overwrite, or seal persisted compare-and-swap (CAS) predicates/revisions and all required writers. A queued active snapshot must not undo a committed completion. Choose from evidence; an outbox is not a CAS. | database-persistence, mission-systems-advisor, server-authority-enforcer |
| Pending result and recovery | Seal nonblocking request/result correlation, conflicting-mutation policy, timeout/refusal retry, disconnect/relog/world-transfer behavior, commit-before-delivery crash and redelivery. Fence attempt, character and session; reused entity IDs must not receive old results. Define reconciliation after commit, acknowledgement/deduplication and which follow-ups survive recovery. Do not leave a cell loop awaiting its own result. | database-persistence, mission-systems-advisor, server-authority-enforcer |
| Continuation | Success follows commit: display Marsh 4576 and clear DSM 5356 slot/template 10 before the mission-completed event opens 6511's 1361 offer. Refusal gives feedback on the first click, retains the binding and mission, and performs no success side effects. Duplicate results cannot consume or fire the offer again. Preserve ordinary completion behavior elsewhere. | mission-systems-advisor, server-authority-enforcer |
| Regression proof and size | Seal actual test names, fixtures, filters, failure injection and revert mutations, plus exact bounded file/function allowlist, internal message/schema shapes, lane checks and measured packet budget. Confirm recovery/once-only assertions are observable rather than assumptions. | testing-validation-engineer, rust-gameserver-dev; coordinator approves manifest/budget |

The reviewed implementation uses a strict `complete_mission_with_item` action, A1 atomic transaction/receipt and an A2 plugin-routed correlated result; no entity-ID-only outbox carries the turn-in result. A snapshot `HasItem` check followed by independent remove/complete cannot meet the atomicity gate. Broad loader hardening, generic turn-ins, client protocol extensions, inventory propID changes and mission 567 remain outside this packet.

**2026-10-14 design review (conditional, not a Ready seal).** Database, authority,
mission and testing reviewers agree that the current 6501 removal intent cannot
prove possession or consumption. A mission-1360-specific transaction can consume
one owned, accessible 3730 and persist the exact `MissionInstance` completion
postimage together, with an inventory-first lock order and exact mutation counts.
The generic mission writer must reject independent 1360 completion and stale
updates; abandon must not delete a committed completion. A correlated result
needs a bounded same-session retry and relog reconciliation; the existing
entity-ID-only outbox is not sufficient. Without durable result acknowledgement,
Marsh's transient 4576 dialog cannot be promised exactly once after a crash.

The reviewed size calls for three engine children: **HA-1360a1** owns the base
transaction, mission writer guard and live-DB failure/concurrency proof;
**HA-1360a2** owns internal messages, base routing and session fence, split
into a2a token/entry epoch and a2b route/sync; a2c separately tracks a
preexisting general post-handoff entry race;
**HA-1360a3** owns the strict content action, cell pending/result state,
retry and relog proof, split into a3a parser, a3b request/retry and a3c
result/continuation packets. HA-1360b remains the seed/receipt child. A1 and
a2a, a2b, a3a and a3b are integrated with reviewed tests; a3c remains.
The engine children remain undeployed campaign integration until 6501 is
replaced: the old chain would otherwise remove a letter and have its
independent completion rejected.

O14-19 resolves the repeat question: mission 1360 is one-shot. HA-1360b changes
the seed to `num_repeats=0`; the committed postimage has repeats 1. The
mission-specific writer fence can reject every later nonterminal overwrite or
abandon, including a queued stale snapshot. The reviewed a1/a2 contract, exact
file scope and tests are in [the engine worknote](worknotes/HA-1360a-seal.md).

**Candidate edit sites, to narrow at seal:** content-engine `actions.rs`, `loader/action.rs` and focused loader tests; cell-content `executor/{dispatch,mod,mission}.rs`, a focused turn-in handler/tests and `cell/missions/` pending guards; wire `cell/messages/{cell_to_base,base_to_cell,cell_to_base_kind}.rs`; base-world-entry `cell_dispatch/{mod,progression_dispatch}.rs`; base-methods `methods/missions/` focused handler/tests and existing mission writer only where stale-writer protection requires it; base-session `outbox/` and shared lock helper only if the chosen design requires them; cell `service/base_messages/` response handler/tests; entity `missions.rs` only for shared completion semantics. Routing/name exhaustiveness edits and any schema file must be explicitly named before Ready. No gameplay seed is edited by HA-1360a.

**Required tests:** unit/loader guards for strict parameters, malformed-action fail-closed conversion, correlation, pending/refusal and duplicate/stale results; live-DB handler tests for counts 1/2, wrong type/owner, zero/missing and storage-only items, exact completed arrays/repeats, and mission-write/result-enqueue failure after item mutation rolling both changes back. Barrier-controlled live-DB concurrency exercises same-type replacement/storage movement, two clicks, competing consume/abandon and queued stale MissionUpdate with captured Results and timeouts. A legal outcome consumes exactly one and completes once, or refuses with active mission and no turn-in consumption. Test commit→crash, redelivery and relog using real receivers and persisted state. Existing inventory/mission serializers get byte-exact and transport smoke checks for committed frames once and no refusal success frames. Negative-log guards require observable refusal reasons. Commit before isolated revert probes; reverting ownership/access/count/transaction/CAS/result guards must fail their specific tests. Testing reviewer signs which guards fail each mutation.

**Telemetry to seal:** requested, rejected (reason), committed, applied, duplicate and stale turn-in-result events. Include correlation, character/session identity, transaction outcome, exact quantity/before/after and expected/current mission state; pair player, chain, mission, step, item and world IDs with names under instrumentation rules. Cover wrong-world/state/overlap refusals, not only matched chains. Existing `Content: RemoveItem` and pre-persistence completion logs do not prove a committed turn-in. Event spellings/fields remain proposed until the manifest names them.

**Checks and docs after seal:** fmt; lane clippy and focused nextest for every changed crate in dependency order; worktree-only live-DB lane filters for transaction/concurrency/recovery tests; merged-wave full checks and independent reviewer/fixer. No cargo or database operation belongs to this planning gate. The implementation owes the game-systems/content-engine and persistence rows of the [doc-update map](../../agents/doc-update-map.md), existing mission engine bible/reference where applicable, and a worknote. Propose a missing engine chapter before writing parallel canon. Rollback scope includes schema/outbox compatibility and committed-result reconciliation; do not blindly delete persisted completions or restore consumed items. The DB and mission reviewers must sign a safe rollback/recovery procedure before Ready.

**Branches and worktrees:** The current per-child branch, worktree, commit subject, manifest and checks are in [the engine worknote](worknotes/HA-1360a-seal.md). The integrated a1/a2a commits remain undeployed until a2b/a3/b complete and replace old chain 6501; a2c is a separate world-entry follow-up. No early standalone merge is authorized.

## HA-1360b — Marsh receiving seed and cross-zone proof

**Status: Integrated (b1 `e1c03b107`, b2 `9455f2af2`); owner-manual UAT pending.** The original gate, now met except the client crossing, was: requires HA-1360a integrated with its atomicity, authority, stale-writer and recovery guards, independent review, and a sealed action/result contract; CS-08a must supply the actual Castle-origin crossing and retained 3730. Mission and testing advisors sign the final receipt graph and end-to-end guard; database-persistence and server-authority-enforcer sign reuse of the engine contract for this receipt. This packet implements only mission 1360's Harset receipt.

**Files and IDs:** `db/resources/Content/Seed/harset_opcore_chains.sql`, only 6501/6502/6505 plus the provisional DSM 120101 row and its sequence advance; `db/resources/Missions/Seed/missions.sql`, only mission 1360 `num_repeats`; `crates/cell-content/src/cell/content/chain_replay_tests/mission_1360_harset.rs` and `mission_1361/disjointness.rs`, with comment-only `mission_1361/handoff.rs` and `crates/content-engine/tests/it/interact_tag_linter.rs`; extend Castle's existing `crates/services/src/mission_round_trip_tests.rs` once CS-08a1 is present; `docs/content/mission-chains.md`, this ledger/RESUME and a HA-1360b worknote; UAT checklist and `docs/guides/unified-uat.md` when implementation publishes runnable steps. Name any additional test-support file before editing. Preserve existing chain ID ownership: 6501 Marsh receipt, 6502 load/relog binding, 6505 post-1361 restore; 6503/6504 ship enabled for 567 (FZ-12). The compound action needs no new chain ID. If a separate continuation chain becomes necessary, return to HA-00b to reserve and globally collision-check an individual free 6506–6510 ID before dispatch; the numeric gap is not an allocation.

Replace 6501's independent removal/completion and precommit success side effects with the sealed HA-1360a operation/continuation. Retain world 68, active 1360/4038 and 1361/4694 disjointness guards, Marsh speaker routing, 4576 and the 6511 offer order. Reserve DSM 120101 as an interaction-only Marsh binding for slot 10 after a fresh collision check; leave stock DSM 5356 untouched. Correct the obsolete container-filter and intentional missing-letter comments. Do not author `has_item`, add a duplicate Castle grant/advance, change 1361 rewards or enable 567. Bind restoration after refusal/relog must remain usable and cannot replay success into a new session.

**Tests and revert proof:** seeded exact-action replay plus production executor/base live-DB consumption/persistence. Begin at actual Cellblock acceptance/grant chains 1003/1121, execute CS-08a's actual Castle→Harset crossing, then the Harset world-68 receipt; injecting 4038 proves only the local graph. Include wrong 4037/world/recipient, missing/foreign/storage-only letter, active 1361/4694 overlap, duplicate click, relog before/after commit, recovered delivery and two characters on the shared Marsh. Assert exact owned decrement, persisted objectives/completion/repeats, binding/4576/6511 order, refusal feedback and no success frames/events through a real receiver. Existing executor message-count tests are insufficient. Separately remove the receiving action, HA-1360a executor/result arm and CS-08a advance in committed disposable reverts; the relevant guard must fail. Do not call a skipped live-DB pass acceptance.

**Checks / telemetry:** common fmt, lane clippy/nextest for cell-content and services (plus any sealed changed crate), worktree-only seed reload, focused live-DB filters `mission_1360_harset` and the sealed cross-service filter, then merged-wave full checks and fresh review. Retain chain/step/refusal telemetry and HA-1360a committed-result correlation; negative guards inspect refusal reasons and absence of success events. Before a seed rollback, preserve the engine primitive for any in-flight durable result; restoring the old free-completion graph violates the accepted owner decision. Seal compatible disable/restore and persisted-state reconciliation with DB/mission reviewers.

**UAT pending:** O14-08 records owner intent for a coordinated lab run; ask the user and wait for a fresh yes before taking the lease, starting or restarting the client, or changing the lab server, as the current repo instructions require. Then use an ordinary character to acquire 3730 in Cellblock, cross Castle→Harset, relog, deliver to Marsh, verify count/mission persistence and 1361 offer, repeat click/relog, and exercise missing-letter feedback and storage/overlap refusals. Attest against DB, structured server logs and packet tap; record fixture deviations. Planning source inspection and Rust tests do not establish client acceptance. HA-19 records the campaign outcome; FZ-99 owns shared status pages.

**Future branch / worktree / commit:** `harset/ha-1360b-marsh-receipt` / `harset-ha-1360b` / `Harset HA-1360b: guard Marsh letter receipt`. No worker receives this packet until dependency and final file/test manifests are signed.

## Tollana closure contract

Use the exact [signed actor tags](README.md#tollana-recipient-tags). Tollana owns source and recipients for 603/611/618/621/1626/1632/1643 in its own seed files, using chains 9800–9999. Conditional whole missions 1364/1366 remain solely T-05a/b. Harset does not claim these IDs or implement duplicate recipients.

618 and 621 require a recovered dialog→speaker relation or approved reconstruction before a specific actor is bound. **O18-08:** the current mapping stays provisional until verified in the client: 618 → Moh'katan (`CmdCenter_Mohkatan`, spawn 343 / template 54) and 621 → Baal (`CmdCenter_Baal`, spawn 340 / template 42). Tollana may build against it, labelled PROVISIONAL; the coordinator relays this and reconciles if Tollana's ledger holds a different mapping. 1626:4838 has two mandatory objectives (5545 Moh'katan, 5908 Marsh); persist each independently. Do not assume 1626:5169 or 1366:4058 is Harset because the name Copplemann appears. Tollana requires its own template/tag for that speaker. Source world, mission/faction/state, item ownership, repeat click, relog and simultaneous differing missions on the same Harset actor must all be guarded. Mutation of the recipient action/state guard must fail the end-to-end replay/live-DB test.

## Research waves R1 and R2 after the 2026-10-18 decisions

The [O18 register](worknotes/owner-decisions-2026-10-18.md) turns several owner gates into research-first work. The packets below are the only ones that are supportable now without client, lab or in-world evidence. Each is a docs-only design or research packet for one domain advisor, in one worktree, writing one new worknote. None writes code or seeds and none allocates an ID. No Haiku code packet is Ready yet: HA-03c is the first one, and it waits on HA-03b.

**Waves.** R1: HA-01b1, HA-01b2, HA-03a, HA-15d, HA-15e, HA-17a1 (six independent packets, one packet-review-fixer pass). R2: HA-01a1 and HA-15f now; HA-03b and HA-15g join R2 once HA-03a and HA-15f are integrated.

**Worktree recipe (coordinator, before dispatch).** From the main checkout: `pwsh tools/build-lane/mk-worktree.ps1 <branch> <worktree>`; then `git -C <wt> rebase --onto <campaign tip> origin/main <branch>`; check that HEAD and `git merge-base` equal the recorded campaign tip. After integration, retire with `rm-worktree.ps1 --force <worktree>`; never `git worktree prune`.

### Rules every R-wave brief carries

- **R1 Scope.** Edit only the one new worknote the packet names. No seed, code, schema, ledger (README, work-packets, RESUME, id-allocation, sign-off cards) or status-doc edit; the coordinator owns the ledger and FZ-99 owns status docs.
- **R2 Shell.** PowerShell only, absolute paths, `Set-Location <wt>;` at the start of every call. No bash. No direct `cargo`: a compile goes through `pwsh tools/build-lane/lane.ps1 cargo ...` in the foreground. No stash, prune, push or PR.
- **R3 No live systems.** No lab, client launch, server, Ghidra, x64dbg or DB reload. Read-only reads of the cooked client tree are allowed only where the packet says so. Scratch scripts and outputs go outside the repo (never `target/desktop`).
- **R4 Labels.** Every claim is FACT (with file and line or row), INFERENCE (high/medium/low, with the reasoning), DECISION (with its O14/O18 ID) or PROVISIONAL (an AI-derived draft). Every proposed beat names RESTORED_SOURCE, CIMMERIA_RECONSTRUCTION or CIMMERIA_AUTHORING.
- **R5 No IDs.** Cite only candidates already listed in [id-allocation.md](id-allocation.md); otherwise write `ID needed: <kind>`. A free numeric gap is not an allocation.
- **R6 Repo rules.** Seeds in `db/resources/` are the source of truth; no `db/scripts` migration. Prefer server-authoritative designs. Anything that needs a new opcode, client UI or a map patch is marked **MAINTAINER DECISION** and the design stops at that point. Every button press or interaction gets visible feedback on the first press. Shared worlds 57/68 actors are never mutated per player. Check the [engine contract](#current-engine-contract-the-old-h-ledger-cannot-supply) traps: every matching chain fires, `once` resets on relog, item/bind cleanup goes before the mission mutation, `remove_effect` has no executor arm, quest speakers carry no vendor/trainer fields, vendors ignore `max_stack_size` and have no faction gate, `repeats` is not persisted by the world-entry UPSERT, event 5 plus `remove_item` consumes twice.
- **R7 Doc form.** CRLF line endings; under 500 lines (hard cap 700); `npx --yes markdownlint-cli2 --no-globs <file>` clean; relative links resolve; no IPs, credentials or account names. Each worknote ends with "Tests the implementation owes" (named tests and the revert that must make each fail) and "Telemetry the implementation owes".
- **R8 Commit and report.** One commit with an explicit path: `git -C <wt> commit -F <msgfile> -- <worknote>`, subject `Harset <PACKET>: <summary>`. Report to `<wt>\.claude\reports\harset-continuation\<packet>.md`; return one line `<packet> <done|blocked|failed> <sha|-> <report-path>`.
- **R9 Budget.** About 30k working tokens. If the packet will not fit, stop, commit what is verified, and return `blocked` with the remaining questions.

Shared inputs (read-only): this file, the [O18 register](worknotes/owner-decisions-2026-10-18.md), [README](README.md), [placements.md](placements.md), [id-allocation.md](id-allocation.md), and the earlier research notes under `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\zone-harset\.claude\reports\harset-continuation\` (`research-missions-advisor.md`, `research-items-advisor.md`, `research-world-advisor.md`; those reports are not in git).

### HA-01b1 — step 5225 right-click door feasibility

**Agent:** mission-systems-advisor. **Branch / worktree:** `harset/ha-01b1-door-affordance` / `harset-ha-01b1`. **Output:** `docs/analysis/harset-continuation/worknotes/HA-01b1-door-affordance.md`. **Decision:** O18-12 ("Preserve the source-described right-click door interaction ... Reuse 6006 only if it can be shown to provide the required interaction without changing the mission step’s behavior").

Answer, with code and seed evidence:

1. What objectives 6059 (optional path) and 6060 (mandatory right-click door) contain in `db/resources/Missions/Seed/`, and what task type the client expects for 6060.
2. Which server-only mechanisms can give a right-click affordance at the Command Center door on world 57: a server-spawned interactable entity with an `interact_tag`, an existing map actor, or chain 6006 reused. For each, cite the `cell-interactions` and content-engine code path that turns the click into objective credit.
3. Whether 6006 (enter-region, point set 2078) can provide the same interaction without changing the step. Give a yes or no with evidence.
4. The visible first-press feedback each option shows, and whether any option needs a client change. If every working option needs one, write **MAINTAINER DECISION** and stop.
5. A graph sketch for 5225 (guards: active 1664, step 5225, world 57; duplicate click; relog) with `ID needed:` placeholders.

Stop if: the door position is needed (world-57 door positions are still BlockedEvidence; leave a placeholder).

### HA-01b2 — provisional Kaerlia NPC design card

**Agent:** npc-ai-spawn-advisor. **Branch / worktree:** `harset/ha-01b2-kaerlia-design` / `harset-ha-01b2`. **Output:** `worknotes/HA-01b2-kaerlia-design.md`. **Decision:** O18-01 ("Create a new Harset NPC for now. Choose a sensible provisional gender, appearance, clothing and placement ... clearly marked as provisional").

Produce, every choice labelled PROVISIONAL with its reasoning:

1. Gender, body, head and clothing taken from existing appearance and template rows in `db/resources/` (name the rows reused). Rank (Sgt.) and faction context from texts moniker 27315 and dialog 6169.
2. A quest-speaker template shape with no vendor or trainer fields; candidate template 260 and spawn 360 cited from id-allocation.md, not allocated.
3. A gate-side placement near the Stargate arrival on world 57, from seeded points and [placements.md](placements.md). Optional: one foreground `pwsh tools/build-lane/lane.ps1 cargo run -p cimmeria-navmesh-extractor --bin nav_inspect -- <harset nav>` check that the point is on the mesh.
4. The dialog 6169 / DSM 7016 / speaker 3219 join: what is FACT and what is INFERENCE.
5. Interaction guards: Kaerlia only advances 5224, and a player without active 1664 gets visible feedback rather than silence.

Stop if: a needed appearance has no existing row (write `asset needed`; do not invent one).

### HA-03a — mission 1325 and Mala'c duel provisional design

**Agent:** mission-systems-advisor. **Branch / worktree:** `harset/ha-03a-rinla-design` / `harset-ha-03a`. **Output:** `worknotes/HA-03a-rinla-duel-design.md`. **Decision:** O18-10 ("Prepare a provisional AI-authored mission and Mala’c duel design, including health-trigger, failure/death handling and effect cleanup ... Mission 1326 remains dependent on 1325").

Design, labelled CIMMERIA_AUTHORING or CIMMERIA_RECONSTRUCTION per beat:

1. The step graph over the seeded steps 3957, 3958, 3959 and 4036, from `db/resources/Missions/Seed/missions.sql` and the existing 1325 rows.
2. The duel: `Rinla_Malac` in the private Market instance (world 69), aggression override at start, the 30% health trigger and surrender, player death or failure resetting to 3958, an `entity_dead_tag` fallback if Mala'c dies, abandon and relog cleanup. Name the content-engine condition and action for each edge and cite its code.
3. An effect inventory: every effect the duel applies (movement lock, aggression, invulnerability), who receives it, and when it must be removed. This list is the input to HA-03b.
4. Chain slots inside the reserved 6311–6330 band by role only (`ID needed: chain`); 1326's offers 6331, 6333 and 6341 stay gated on 1325 completion.
5. The duel position stays a placeholder (HA-01a/HA-02 evidence).

Stop if: a beat needs a client change (MAINTAINER DECISION).

### HA-15d — mission-item grant design

**Agent:** items-systems-advisor. **Branch / worktree:** `harset/ha-15d-item-grants` / `harset-ha-15d`. **Output:** `worknotes/HA-15d-mission-item-grants.md`. **Decision:** O18-03 ("Mission items must be real, collectible items in the mission inventory. Do not replace them with invisible completion or automatic objective credit").

For each of the ten in-scope GAP items from the [HA-15 audit](worknotes/HA-15-audit.md): 2720, 2734, 2743, 2818, 2823, 2825, 4396, 4690, 5146, 5736. Out of scope: the Agnos items 5703/5704/5706/5760/5763/5766 (O18-06), 2698 (O18-09, Castle) and 4512/2714 (HA-15a).

1. Item row facts: container, stack size, event set, flags.
2. A visible source type (pickup, kill, NPC hand-over, chain grant) with the mission text that suggests it, labelled FACT or INFERENCE.
3. The grant primitive and its precedent (chains 6101 → 2820, 6102 → 2819, 6110 → 2864), the consume point, duplicate and relog guards, and the first-press feedback.
4. Hazards: double consume with event 5 (cross-reference HA-15e), stack limits, grant before equip, shared-world actors.
5. Which mission packet (HA-04 to HA-14) owns each grant.

### HA-15e — event-5 item function audit

**Agent:** items-systems-advisor. **Branch / worktree:** `harset/ha-15e-event5-audit` / `harset-ha-15e`. **Output:** `worknotes/HA-15e-event5-audit.md`. **Decision:** O18-07 ("Implement real item functions where the source data supports them ... Keep uncertain bindings marked as provisional").

1. For each binding 2743 → 2410, 2818 → 2068, 4396 → 2092 and 4690 → 2409: read the ability row, its effects and targeting from `db/resources/`, and record whether the source supports a real function.
2. The ability-597 placeholders on 2819, 2823 and 2864: what 597 is, and whether a better-supported ability exists.
3. Trace the `consumable_use` path in code (UseInventoryItem to ItemUsed) and show where event 5 consumes. State the exact case where a chain `remove_item` would consume twice, and the guard that prevents it.
4. Verdict per item: REAL, PROVISIONAL or PLACEHOLDER, and the packet that consumes it (HA-07, HA-08a, HA-09, HA-11).

### HA-17a1 — pilot graph design card, Jaffa story 1

**Agent:** mission-systems-advisor. **Branch / worktree:** `harset/ha-17a1-pilot-design` / `harset-ha-17a1`. **Output:** `worknotes/HA-17a1-pilot-design.md`. **Decision:** O18-11 ("Start with HA-17a’s first Jaffa story as the pilot graph ... Do not allocate IDs until the required graph and collision checks are complete") and O18-05 (AI-derived provisional premises and rewards).

1. One premise (CIMMERIA_AUTHORING, PROVISIONAL) inside O14-10/11: lore-safe, no player-facing "authored" label.
2. Actors from existing Jaffa-side spawns only; the graph is accept, two steps, close and reward, with guards.
3. The client catalog delivery path through CB-N05a: what the client needs to show the journal entry and dialog.
4. ID needs by kind (mission, step, objective, task, dialog map, dialog, chain), with no numbers.
5. A filled copy of [card 1](worknotes/HA-00c-signoff-cards.md#card-1-ha-17a-jaffa-story-1) inside the new worknote, minus signatures and IDs. Do not edit the sign-off cards file.
6. The collision-check procedure to run before allocation, and an owner-run client check script (journal, dialog, advance, relog, reward).

### HA-01a1 — interior landing geometry check

**Agent:** movement-teleport-advisor (lane build allowed). **Branch / worktree:** `harset/ha-01a1-landing-geometry` / `harset-ha-01a1`. **Output:** `worknotes/HA-01a1-landing-geometry.md`. **Inputs:** the [BSP door probe](worknotes/HA-01a-bsp-door-probe.md) and [HA-02 revalidation](worknotes/HA-02-offline-revalidation.md). Read-only cooked root: `C:\Users\Steve\source\projects\SGW\Stargate Worlds-QA\Working\SGWGame\CookedPC`.

1. One foreground build: `Set-Location <wt>; pwsh tools/build-lane/lane.ps1 cargo build -p cimmeria-navmesh-extractor --release --bin extract_map --bin obj_slab`.
2. `extract_map extract` Harset_Market and Harset_StorageRm into a scratch directory outside the repo.
3. `obj_slab --column` and `--at` at the three landing estimates (Market ≈(89.5, 3.6, 60.3) and ≈(19.9, 4.8, 9.0); Storage ≈(61.4, 5.2, 5.2)): true floor y and 1.5 m clearance. Optionally run `nav_inspect` on the existing meshes at the same points.
4. Settle the world label: `worlds.sql` says world 69 = `harset_market.nav` and 68 = `harset_cmdcenter.nav`; the HA-02 heading says otherwise.
5. Recommend, as INFERENCE for the coordinator's O14-12 call: E03 move or drop, the Market box shrink to about x[16,24] z[9,22], and whether to raise Storage region 2122 to include E13–E15.

Stop if: the build fails twice (report the lane log path).

### HA-15f — Omega Site population and service reference

**Agent:** npc-ai-spawn-advisor. **Branch / worktree:** `harset/ha-15f-omega-reference` / `harset-ha-15f`. **Output:** `worknotes/HA-15f-omega-reference.md`. **Decision:** O18-04 ("Use the Omega Site map as the population and service-layout reference ... Record the reasoning so it can be reviewed in game").

1. Census of worlds 18 and 80 in `db/resources/`: spawns, templates, vendor and trainer fields, item lists and prices.
2. The service-layout pattern: which services, how many, where relative to arrival.
3. A provisional Harset distribution by role, district (worlds 57/68/69/70) and faction, with reasoning; a Jaffa merchant for Jaffa-facing goods, and a reasoned choice of who serves Goa'uld-facing trade.
4. A price and stock envelope proposal inside O14-12 (tier-comparable, no new currency, no high-tier gear). It proposes values; it does not set them.
5. Engine hazards: no vendor faction gate, `max_stack_size` ignored, quest speakers kept separate.

### Gated R-wave follow-ons (not dispatchable)

- **HA-03b** (combat-systems-advisor; `harset/ha-03b-remove-effect-seal` / `harset-ha-03b`): seals a Haiku contract for the `remove_effect` arm. Facts for the sealer: `Action::RemoveEffect { effect_id: i32 }` at `crates/content-engine/src/actions.rs:62`, loaded at `loader/action.rs:365`; no arm in `crates/cell-content/src/cell/content/executor/dispatch.rs` (ApplyEffect arm near line 588, fallthrough logs "Content: unhandled action"); `effect_apply.rs::apply_effect` is the mirror; removal helpers exist in cell-combat `stat_buffs/mod.rs`, cell-world `stat_buff/ledger.rs` and entity `cell_entity/stat_buff.rs`. Needs HA-03a's effect inventory.
- **HA-03c** (packet-coder): implements the sealed arm with a dispatch test that fails when the arm is reverted. Needs HA-03b.
- **HA-15g** (combat-systems-advisor): provisional trainer trees through `tools/ability_trees/generate_seed.py` and its workbook. Needs HA-15f.

## Validation, telemetry and documentation owed by implementation

Runtime packets choose test types from TESTING.md before coding. Chain-only changes need exact-action replay plus adjacent mission/step/world/faction negatives and a live-DB reachability/item relationship guard; executor changes need side-effect tests. Encounters additionally test disconnect/abandon/relog cleanup and two-player isolation. Placement guards load the actual current mesh with a nonvacuity control. Authored missions need client catalog visibility proof. Repeatables require persistence and once-per-cycle reward guards. Record a reverted-chain/row/arm failure; do not count skip-as-pass or static string checks as those gates.

Follow the parent contracts for ordered fmt, focused lane clippy/nextest, worktree-only reload and live-DB lane filters. Add all changed crates. No compiling command runs outside the lane. HA-00b is docs-only and uses census uniqueness, seed PK/tag collision checks, source/guard inspection, relative-link validation, CRLF validation, markdownlint and `git diff --check`.

Each later brief names telemetry for chain resolution/execution, mission/step/objective progress and refusal reason. Existing executor `Content: accept_mission refused by offer guard`, `once_spent` and mission objective signals can be reused only where they cover the actual edge. The specialist seal must identify the concrete event/message/fields for wrong world/state/faction/already-completed/missing-item refusal; a prose promise is not acceptance. Replay test telemetry where practical and attest runtime logs during authorized UAT.

Implementation updates `docs/content/mission-chains.md` and its worknote. Engine semantic changes additionally update the matching existing mission bible chapter when present; per-mission graphs remain content documentation. HA-19 owns this ledger/UAT checklist, FZ-99 shared gap/status/zone-audit pages. HA-00b integration adds the continuation index link and dated sourced main-session memory pointer. Historical ledger decisions are not rewritten.
