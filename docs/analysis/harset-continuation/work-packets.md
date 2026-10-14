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
| HA-00c | Authored/repeatable exact IDs | BlockedEvidence | CB-N05a is merged; inspect stock catalog and Cellblock's first authored client proof, then perform current global ID audit under O14-04/10/13 |
| HA-1360a1 | Atomic base transaction and writer fence | Ready | Exact [a1 manifest and guards](worknotes/HA-1360a-seal.md#ha-1360a1--transaction-and-stale-writer-fence) sealed after database, mission, authority and testing review; implementation/test proof open |
| HA-1360a2a | Session token world-entry delivery and lifetime | Ready | [Exact manifest and deterministic replacement/reuse tests](worknotes/HA-1360a-seal.md#a2a-dispatch-seal); implementation and checks open |
| HA-1360a2b | Correlated request/result routing and A1 invocation | BlockedDependency | A1 and a2a integrated; exact spawned DB handoff, wire DTO and routed live-DB tests still to seal |
| HA-1360a3 | Strict action, pending/recovery and continuation | BlockedDependency | A2 transport interface; exact cell teardown and result manifest still to seal |
| HA-1360b | Marsh receiving seed and cross-zone proof | BlockedDependency | HA-1360a integrated/reviewed; CS-08a actual crossing and retained letter; receiving graph signoff |
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
| HA-15a | Item 4512 source | BlockedEvidence | O14-12 allows a conservative item policy; an owned acquisition source remains unproved |
| HA-15b | Faction vendors | BlockedEvidence | O14-12 permits tier-comparable stock/prices; audit existing tiers, faction/level bands and template/dialog isolation |
| HA-15c | Trainers | BlockedEvidence | O14-12 permits conservative trees; audit existing ability tiers and teaching policy |
| HA-16a | World-57 districts | BlockedEvidence | O14-12 permits conservative density; measure safe roles/placements and separate ambient templates from quest speakers |
| HA-16b | Command Center ambient | BlockedEvidence | O14-12 permits conservative density; measure interior placements and shared-space load |
| HA-17a | Two Jaffa stories | BlockedDependency | O14-10/11 delegate lore-safe premise/rewards and no player-facing label; restoration UAT, HA-00c IDs and Cellblock's first authored catalog client proof remain |
| HA-17b | Two OP-CORE stories | BlockedDependency | Same O14-10/11 policy; HA-00c, authored catalog client proof, restoration UAT and HA-09/10/services remain |
| HA-17c | Two Goa'uld stories | BlockedDependency | Same O14-10/11 policy; HA-00c, authored catalog client proof, restoration UAT and HA-13/14 remain |
| HA-18a | Jaffa repeatable | BlockedDependency | O14-13 delegates persisted cooldown/reward/anti-farm design; HA-00c, authored catalog proof and HA-17a remain |
| HA-18b | OP-CORE repeatable | BlockedDependency | Same O14-13 authority; HA-00c, authored catalog proof and HA-17b remain |
| HA-18c | Goa'uld repeatable | BlockedDependency | Same O14-13 authority; HA-00c, authored catalog proof and HA-17c remain |
| HA-19 | Integrated UAT/ledger closeout | BlockedDependency | Restoration graphs and authorized client UAT; FZ-99 owns shared status docs |

## HA-00c specialist allocation gate

The [HA-00c contract](../four-zone-planning/harset-work-packets.md#ha-00c--seal-authored-and-repeatable-id-allocations) owns six authored stories and three repeatables. It records exact mission/step/objective/task/dialog-map/dialog/item/chain IDs, client catalog source/build/hash, global seed and active-branch collisions, approved reuse, provenance and owner story/reward/reset decisions. No catalog IDs are sealed by HA-00b. A guessed stock gap, historical mission reuse or server-only row cannot close this gate.

CB-N05a is merged, but HA-00c remains BlockedEvidence until the stock catalog is inspected, Cellblock's first authored quest passes client journal/dialog/advance/relog proof, and the current active-branch collision audit is documented. O14-04/10/13 delegate individual allocation, lore-safe authored content and repeatable policy with domain review; no further owner vote is required inside those bounds. Completion needs the mission advisor's nine graph signoffs and coordinator's shared claims signoff. HA-17a/b/c and HA-18a/b/c remain BlockedDependency on HA-00c, that first authored client proof and restoration UAT. This docs/research gate uses the parent contract's static checks and scratch duplicate failure proof; implementation retains its ordered lane checks.

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

CS-08 owns upstream changes for 1360/567. Owner decisions on 2026-10-14 select CS-08a's Castle-origin `stargate_crossed` Castle→Harset reconstruction, chain 1366 (single ID approved), for 1360 step 4037→4038, with letter 3730 retained. Marsh must require the letter before completion; HA-1360a and HA-1360b explicitly expand the work into an engine prerequisite and Harset receipt. Harset keeps receiving chains 6501/6502/6505 and disabled 6503/6504. Neither packet adds a Castle grant or a second crossing advance.

For 1360, replay the existing Cellblock item-3730 accept, Castle 4037→4038 and world-hop into Marsh's world-68 receipt. Include wrong 4037, second click, missing/foreign item, the 1361 bind-disjointness state and relog. An executor test must observe the real item removal and mission persistence, rather than injecting 4038 only.

For 567, first prove the item-2698 acquisition and faction-compatible Castle acceptance; then 2000→2012→4039, owned-item consumption and duplicate refusal at Copplemann. Only after that evidence can the receiving pair be enabled together and guarded by a cross-world replay/live-DB test. Item-grant/action side effects route through inventory ownership/persistence review.

## HA-1360a — authoritative letter turn-in prerequisite

**Status: BlockedEvidence; design and seal required before implementation.** Audience: engine implementer and database, authority, mission and testing reviewers. The owner requires an owned letter for Marsh's completion; the exact primitive, signatures, pending state, storage and recovery design remain proposals. This is a bounded mission-1360 seam, not a redesign of every inventory or mission writer. Split into smaller children before dispatch if the sealed manifest exceeds the shared 40k working-token / 90k context envelope or contains multiple unresolved engine seams.

### Current source evidence

| Evidence you can inspect | What it establishes / limit |
|---|---|
| `db/resources/Content/Seed/harset_opcore_chains.sql`, chain 6501 | World 68, active 1360/4038, disjoint from active 1361/4694. Actions display 4576, remove 3730 ×1, clear DSM 5356 slot 10, complete 1360. Its comment explicitly permits missing-letter completion and contains an outdated no-container-filter claim. |
| `crates/cell-content/src/cell/content/executor/inventory.rs::remove`, `executor/mod.rs::execute_actions`, `executor/mission.rs::complete` | Removal sends a cell→base request without a mutation verdict; the action list continues. Completion changes cell state, sends a separate MissionUpdate and fires follow-up chains. Emitted removal intent cannot prove consumption. |
| `crates/content-engine/src/conditions/mod.rs::HasItem`, `src/loader/condition.rs`, `src/loader/mod.rs`; cell-content `event_dispatch/interaction.rs` | HasItem reads a free-form count; the loader has no `has_item` arm and skips unknown conditions. Interaction does not supply authoritative inventory counts. A seed-only possession condition cannot close this defect. |
| `crates/base-methods/src/base/world_entry/methods/inventory/core/{remove_by_type,access}.rs` | Existing removal selects an owned, accessible row under `FOR UPDATE`, separately from mission persistence. It is not the proposed strict mission transaction. By-type content removal uses `VaultAccess::NO_SESSION`: containers 1–15, excluding buyback 16, bank 17, organization vault and escrow. |
| `crates/base-session/src/base/inventory_locks.rs`; base-methods `missions/mod.rs::handle_mission_update`; base-session `outbox/mod.rs` | Shared advisory locks precede inventory/player rows. MissionUpdate uses an unconditional UPSERT. Outbox delivery is keyed by entity ID; durable delivery alone cannot prove character/session-safe result recovery or stale-update exclusion. |

These observations establish a receiving defect by source inspection. No new engine implementation, atomicity proof, historical client behavior or live-client evidence is claimed.

### Seal gates and required signoffs

| Gate | Contract you must close | Required signoff |
|---|---|---|
| Authority | Bind actor to trusted current character/session; trusted content names mission 1360, expected active step 4038, item type 3730 and positive quantity 1. Revalidate eligible turn-in state, world/recipient and 1361 overlap at the authoritative boundary; do not trust client counts or instance IDs. Reject unreadable/locked mission state, forged or stale actor/state, foreign/missing/inaccessible/zero-count items. | server-authority-enforcer, mission-systems-advisor |
| Atomicity and locks | Exact owned accessible instance consumption and persisted mission completion must commit together or both roll back. Reuse `MissionInstance::complete` semantics for objectives, completed steps and repeats. Require exact quantity and mutation row count; no clamped consume. Publish one lock order compatible with inventory, player, mission, abandon and competing removal writers, including all error paths. | database-persistence, server-authority-enforcer, mission-systems-advisor |
| Stale writers / CAS | Enumerate MissionUpdate, abandon, restore/relog and competing completion writers. Either prove ordering/serialization excludes every stale overwrite, or seal persisted compare-and-swap (CAS) predicates/revisions and all required writers. A queued active snapshot must not undo a committed completion. Choose from evidence; an outbox is not a CAS. | database-persistence, mission-systems-advisor, server-authority-enforcer |
| Pending result and recovery | Seal nonblocking request/result correlation, conflicting-mutation policy, timeout/refusal retry, disconnect/relog/world-transfer behavior, commit-before-delivery crash and redelivery. Fence attempt, character and session; reused entity IDs must not receive old results. Define reconciliation after commit, acknowledgement/deduplication and which follow-ups survive recovery. Do not leave a cell loop awaiting its own result. | database-persistence, mission-systems-advisor, server-authority-enforcer |
| Continuation | Success follows commit: display Marsh 4576 and clear DSM 5356 slot/template 10 before the mission-completed event opens 6511's 1361 offer. Refusal gives feedback on the first click, retains the binding and mission, and performs no success side effects. Duplicate results cannot consume or fire the offer again. Preserve ordinary completion behavior elsewhere. | mission-systems-advisor, server-authority-enforcer |
| Regression proof and size | Seal actual test names, fixtures, filters, failure injection and revert mutations, plus exact bounded file/function allowlist, internal message/schema shapes, lane checks and measured packet budget. Confirm recovery/once-only assertions are observable rather than assumptions. | testing-validation-engineer, rust-gameserver-dev; coordinator approves manifest/budget |

A compound action provisionally named `complete_mission_with_item` is a candidate. A base transaction with a correlated committed result and transactional outbox is also a candidate. Neither name nor architecture is settled here. A snapshot `HasItem` check followed by independent remove/complete cannot meet the atomicity gate. Broad loader hardening, generic turn-ins, client protocol extensions, inventory propID changes and mission 567 remain outside this packet.

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
**HA-1360a2** owns internal messages, base routing and session fence;
**HA-1360a3** owns the strict content action, cell pending/result state,
retry and relog proof. HA-1360b remains the seed/receipt child. A1 is Ready at
the contract level after the focused 2026-10-14 signoffs; a2/a3 retain their
named interface/manifest gates. No child has implementation or test proof yet.
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

**Future branch / worktree / commit:** `harset/ha-1360a-letter-turn-in` / `harset-ha-1360a` / `Harset HA-1360a: require committed letter turn-in`. Use the current recorded campaign integration SHA; no early standalone merge is authorized by this contract. Publish all signoffs and unresolved choices in this ledger before changing BlockedEvidence to Ready.

## HA-1360b — Marsh receiving seed and cross-zone proof

**Status: BlockedDependency.** Requires HA-1360a integrated with its atomicity, authority, stale-writer and recovery guards, independent review, and a sealed action/result contract; CS-08a must supply the actual Castle-origin crossing and retained 3730. Mission and testing advisors sign the final receipt graph and end-to-end guard; database-persistence and server-authority-enforcer sign reuse of the engine contract for this receipt. This packet implements only mission 1360's Harset receipt.

**Files and IDs:** `db/resources/Content/Seed/harset_opcore_chains.sql`, only 6501/6502/6505; `crates/cell-content/src/cell/content/chain_replay_tests/mission_1360_harset.rs`; one focused cross-service test/module declaration under `crates/services/src/` named at seal; `docs/content/mission-chains.md`, this ledger/RESUME and a HA-1360b worknote; UAT checklist and `docs/guides/unified-uat.md` when implementation publishes runnable steps. Preserve existing ID ownership: 6501 Marsh receipt, 6502 load/relog binding, 6505 post-1361 restore; 6503/6504 remain disabled for 567. No new content IDs are required by the proposed compound action. If a separate continuation chain becomes necessary, return to HA-00b to reserve and globally collision-check an individual free 6506–6510 ID before dispatch; the numeric gap is not an allocation.

Replace 6501's independent removal/completion and precommit success side effects with the sealed HA-1360a operation/continuation. Retain world 68, active 1360/4038 and 1361/4694 disjointness guards, Marsh speaker routing, 4576, DSM 5356 template slot 10 and the 6511 offer order. Correct the obsolete container-filter and intentional missing-letter comments. Do not author `has_item`, add a duplicate Castle grant/advance, change 1361 rewards or enable 567. Bind restoration after refusal/relog must remain usable and cannot replay success into a new session.

**Tests and revert proof:** seeded exact-action replay plus production executor/base live-DB consumption/persistence. Begin at actual Cellblock acceptance/grant chains 1003/1121, execute CS-08a's actual Castle→Harset crossing, then the Harset world-68 receipt; injecting 4038 proves only the local graph. Include wrong 4037/world/recipient, missing/foreign/storage-only letter, active 1361/4694 overlap, duplicate click, relog before/after commit, recovered delivery and two characters on the shared Marsh. Assert exact owned decrement, persisted objectives/completion/repeats, binding/4576/6511 order, refusal feedback and no success frames/events through a real receiver. Existing executor message-count tests are insufficient. Separately remove the receiving action, HA-1360a executor/result arm and CS-08a advance in committed disposable reverts; the relevant guard must fail. Do not call a skipped live-DB pass acceptance.

**Checks / telemetry:** common fmt, lane clippy/nextest for cell-content and services (plus any sealed changed crate), worktree-only seed reload, focused live-DB filters `mission_1360_harset` and the sealed cross-service filter, then merged-wave full checks and fresh review. Retain chain/step/refusal telemetry and HA-1360a committed-result correlation; negative guards inspect refusal reasons and absence of success events. Before a seed rollback, preserve the engine primitive for any in-flight durable result; restoring the old free-completion graph violates the accepted owner decision. Seal compatible disable/restore and persisted-state reconciliation with DB/mission reviewers.

**UAT pending:** O14-08 supplies standing owner approval for a coordinated lab lease/client/server run. Use an ordinary character to acquire 3730 in Cellblock, cross Castle→Harset, relog, deliver to Marsh, verify count/mission persistence and 1361 offer, repeat click/relog, then exercise missing-letter feedback and storage/overlap refusals. Attest against DB, structured server logs and packet tap; record fixture deviations. Planning source inspection and Rust tests do not establish client acceptance. HA-19 records the campaign outcome; FZ-99 owns shared status pages.

**Future branch / worktree / commit:** `harset/ha-1360b-marsh-receipt` / `harset-ha-1360b` / `Harset HA-1360b: guard Marsh letter receipt`. No worker receives this packet until dependency and final file/test manifests are signed.

## Tollana closure contract

Use the exact [signed actor tags](README.md#tollana-recipient-tags). Tollana owns source and recipients for 603/611/618/621/1626/1632/1643 in its own seed files, using chains 9800–9999. Conditional whole missions 1364/1366 remain solely T-05a/b. Harset does not claim these IDs or implement duplicate recipients.

618 and 621 require a recovered dialog→speaker relation or approved reconstruction before a specific actor is bound. 1626:4838 has two mandatory objectives (5545 Moh'katan, 5908 Marsh); persist each independently. Do not assume 1626:5169 or 1366:4058 is Harset because the name Copplemann appears. Tollana requires its own template/tag for that speaker. Source world, mission/faction/state, item ownership, repeat click, relog and simultaneous differing missions on the same Harset actor must all be guarded. Mutation of the recipient action/state guard must fail the end-to-end replay/live-DB test.

## Validation, telemetry and documentation owed by implementation

Runtime packets choose test types from TESTING.md before coding. Chain-only changes need exact-action replay plus adjacent mission/step/world/faction negatives and a live-DB reachability/item relationship guard; executor changes need side-effect tests. Encounters additionally test disconnect/abandon/relog cleanup and two-player isolation. Placement guards load the actual current mesh with a nonvacuity control. Authored missions need client catalog visibility proof. Repeatables require persistence and once-per-cycle reward guards. Record a reverted-chain/row/arm failure; do not count skip-as-pass or static string checks as those gates.

Follow the parent contracts for ordered fmt, focused lane clippy/nextest, worktree-only reload and live-DB lane filters. Add all changed crates. No compiling command runs outside the lane. HA-00b is docs-only and uses census uniqueness, seed PK/tag collision checks, source/guard inspection, relative-link validation, CRLF validation, markdownlint and `git diff --check`.

Each later brief names telemetry for chain resolution/execution, mission/step/objective progress and refusal reason. Existing executor `Content: accept_mission refused by offer guard`, `once_spent` and mission objective signals can be reused only where they cover the actual edge. The specialist seal must identify the concrete event/message/fields for wrong world/state/faction/already-completed/missing-item refusal; a prose promise is not acceptance. Replay test telemetry where practical and attest runtime logs during authorized UAT.

Implementation updates `docs/content/mission-chains.md` and its worknote. Engine semantic changes additionally update the matching existing mission bible chapter when present; per-mission graphs remain content documentation. HA-19 owns this ledger/UAT checklist, FZ-99 shared gap/status/zone-audit pages. HA-00b integration adds the continuation index link and dated sourced main-session memory pointer. Historical ledger decisions are not rewritten.
