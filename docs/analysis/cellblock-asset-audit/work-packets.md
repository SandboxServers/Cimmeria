# CellBlock Asset Audit Work Packets

> **Type:** Reference (execution contracts)
> **Last updated:** 2026-10-07
> **Companions:** [Campaign](README.md), [evidence ledger](evidence-ledger.md), [presentation UAT](uat-guide.md), [test policy](../../../TESTING.md)

These packets carry the campaign through restoration and in-game UAT on the existing QA client. The handoff is evidence to investigate, not instructions. Original recovery comes first; the owner has authorized project design for unresolved pieces. That decision supersedes the earlier camera-only compromise. No additional policy interview is needed to start research or produce the restoration design.

## Execution rules

`Ready` means investigation can start; it does not claim its acceptance gate has passed. `Waiting` means listed prerequisites must finish. `BlockedEvidence` prevents speculative implementation. `Conditional` requires an applicability verdict. Mark `Verified` only with linked evidence. Owners below are roles to assign, not agents already dispatched. Record assignee, current commit, start/end, changed files, evidence, test results and next action in `worknotes/<packet-id>.md` when executing; create that directory with the first packet.

Keep proprietary packages and client binaries outside Git. Use the Cimmeria map/package tools and record commands, tool revision, input hashes, parse failures and bounded negatives. Do not treat absent names as absent assets. Every implementation packet excludes NPC placement, AI/combat tuning, escort implementation, class reward/ammo redesign and content beyond the World12 to8 boundary.

Compiling cargo calls use the Windows pinned build lane. Runtime changes need meaningful regression tests selected from TESTING.md; no cargo build is required for these planning documents. Wire tests require independent protocol/capture evidence. Observer tests assert actual receiver bytes. Deployment and installation state must be recorded separately from implementation completion.

## Parent milestones and dispatch waves

Existing CA-00 through CA-09 references remain parent milestones. Their children are the assignable work contracts below.

| Parent | Children | Milestone |
|---|---|---|
| CA-00 | CA-00A/B | Provenance and bounded parser coverage |
| CA-01 | CA-01A | Current binding reconciliation |
| CA-02 | CA-02A/B | Prop and placement verdicts |
| CA-03 | CA-03A/B/C/D | Door, cover, pod and barrier contracts |
| CA-04 | CA-04A/B/C | Camera, rift and blood evidence |
| CA-05 | CA-05A/B | Implementable original/design dispositions |
| CA-06 | CA-06A/B/C/D/E | Integrated server/seed restoration |
| CA-07 | CA-07A | Client work or evidenced NotApplicable |
| CA-08 | CA-08A/B | Isolated and full-route UAT |
| CA-09 | CA-09A | Verified closeout |

1. Start CA-00A, CA-00B and CA-01A; CA-00B preliminary diagnosis can run while hashes are finalized.
2. Dispatch CA-02 and CA-03 investigations after their inputs resolve. CA-04A starts alongside them; CA-04B follows camera/timeline evidence. CA-04C can run independently.
3. Join all verdicts in CA-05A, then resolve remaining design dispositions in CA-05B. A resolved asset need not wait for unrelated replacement-art work if its state contract is complete.
4. Assign CA-06 implementation by module ownership. CA-06E is the sole shared-seed integrator. CA-06B/C/D touch overlapping executor/state modules and must serialize edits or agree exact file ownership before starting. CA-07A proceeds only for demonstrated client necessity.
5. Run CA-08A, repair failures, then CA-08B and CA-09A. A research-only report is not campaign completion.

Research packets own their named verdict documents and may run concurrently. Parser changes belong only to CA-00B. All contributors preserve other contributors' edits. Do not concurrently edit shared seed files or regenerate campaign JSON outputs from different corpus revisions.

## Packet status ledger

| Packet | Initial status | Dependencies | Owner role |
|---|---|---|---|
| CA-00A | Ready | None | game-archaeology-specialist |
| CA-00B | Ready | CA-00A | game-archaeology-specialist; testing-validation-engineer |
| CA-01A | Ready | None; merge corpus joins after CA-00A | mission-systems-advisor; items-systems-advisor |
| CA-02A | Ready | CA-00A, CA-01A | items-systems-advisor; game-archaeology-specialist |
| CA-02B | Ready | CA-00A, CA-01A | items-systems-advisor; mission-systems-advisor |
| CA-03A | Ready | CA-00B, CA-01A | mission-systems-advisor; game-archaeology-specialist |
| CA-03B | Ready | CA-00B, CA-01A | mission-systems-advisor; game-archaeology-specialist |
| CA-03C | Ready | CA-00B, CA-01A | game-archaeology-specialist; server-authority-enforcer |
| CA-03D | Ready | CA-00B, CA-01A | mission-systems-advisor |
| CA-04A | Ready | CA-00B, CA-01A | game-archaeology-specialist; bigworld-engine-advisor |
| CA-04B | Ready | CA-00B, CA-04A | game-archaeology-specialist |
| CA-04C | Ready | CA-00B, CA-01A | game-archaeology-specialist |
| CA-05A | Waiting | CA-02A/B, CA-03A/B/C/D, CA-04A/B/C | bigworld-engine-advisor; aoi-witness-broadcast; server-authority-enforcer |
| CA-05B | Waiting | Relevant CA-02/03/04 verdicts, CA-05A | game-archaeology-specialist; documentation-writer |
| CA-06A | BlockedEvidence | CA-05A; CA-05B where applicable | rust-gameserver-dev; items-systems-advisor |
| CA-06B | BlockedEvidence | CA-05A; CA-05B where applicable | rust-gameserver-dev; mission-systems-advisor; server-authority-enforcer |
| CA-06C | BlockedEvidence | CA-05A, CA-04A; CA-05B where applicable | rust-gameserver-dev; bigworld-engine-advisor; game-archaeology-specialist |
| CA-06D | BlockedEvidence | CA-05A; contracts from CA-06A/B/C | rust-gameserver-dev; aoi-witness-broadcast; server-authority-enforcer |
| CA-06E | Waiting | CA-06A/B/C/D; CA-07A if required | Coordinator; database-persistence; testing-validation-engineer |
| CA-07A | Conditional | CA-05A, CA-05B; demonstrated client-side necessity | game-archaeology-specialist; bigworld-engine-advisor |
| CA-08A | Waiting | CA-06E, CA-07A disposition | testing-validation-engineer; QA owner |
| CA-08B | Waiting | CA-08A fixes verified | testing-validation-engineer; movement-teleport-advisor; aoi-witness-broadcast |
| CA-09A | Waiting | CA-08B; all required failures repaired | documentation-writer; Coordinator |

## CA-00A — Freeze corpus and provenance

**Status:** Ready. **Depends on:** None.  
**Owner/advisors:** game-archaeology-specialist.  
**Entry points / inputs:** `research_corpus.py; evidence-ledger.md`; [campaign evidence](evidence-ledger.md).

**Work and scope:** Hash QA packages and supplied archives; reconcile CSV origin, 65 maps versus 64 streamed plus persistent, and duplicate copies. Record tool revision, extraction commands and coordinate convention.

**Deliverables / ownership:** corpus-manifest.md with hashes, source IDs, counts and exclusions. Keep evidence outputs in this campaign directory. Source paths listed without a directory resolve through CA-01A's current binding matrix; research outputs named here are planned artifacts, not files already available.

**Verification:** Re-run manifest and compare counts; no binary assets committed. **UAT:** CA-U20 in [the presentation guide](uat-guide.md).

**Acceptance:** All inputs have provenance or an explicit missing-input record. Record findings and limitations in the packet worknote before advancing the ledger.

**If blocked:** Name the missing prerequisite, affected actor/export/state and next investigative action. Research packets return bounded evidence gaps to CA-05; implementation packets wait for their contract and must not guess IDs. UAT failures return to the owning implementation packet and require the affected scenario to be rerun.

## CA-00B — Close parser coverage gaps

**Status:** Ready. **Depends on:** CA-00A.  
**Owner/advisors:** game-archaeology-specialist; testing-validation-engineer.  
**Entry points / inputs:** `tools/upk_parser.py; tools/extract_actors.py; tools/kismet_extractor.py; crates/upk/; crates/upk-objects/`; [campaign evidence](evidence-ledger.md).

**Work and scope:** Investigate four failed actor tiles and persistent Kismet export 135 bounds failure. Cross-check Python and Rust output; distinguish malformed data from unsupported parsing.

**Deliverables / ownership:** parser-coverage.md and corrected extraction evidence; parser fixes only when demonstrated. Keep evidence outputs in this campaign directory. Source paths listed without a directory resolve through CA-01A's current binding matrix; research outputs named here are planned artifacts, not files already available.

**Verification:** Synthetic parser unit/fixture tests for any fix, including malformed bounds; repeat corpus extraction. **UAT:** CA-U18 in [the presentation guide](uat-guide.md).

**Acceptance:** Every failure is recovered or bounded with affected objects and confidence limits. Record findings and limitations in the packet worknote before advancing the ledger.

**If blocked:** Name the missing prerequisite, affected actor/export/state and next investigative action. Research packets return bounded evidence gaps to CA-05; implementation packets wait for their contract and must not guess IDs. UAT failures return to the owning implementation packet and require the affected scenario to be rerun.

## CA-01A — Reconcile current bindings and sibling decisions

**Status:** Ready. **Depends on:** None; merge corpus joins after CA-00A.  
**Owner/advisors:** mission-systems-advisor; items-systems-advisor.  
**Entry points / inputs:** `db/resources/Content/Seed/castle_cellblock_chains.sql; db/resources/Worlds/Seed/spawnlist.sql; db/resources/Worlds/Seed/ring_transport_regions.sql`; [campaign evidence](evidence-ledger.md).

**Work and scope:** Build phase/actor/event/item/chain matrix against current checkout and sibling campaigns. Preserve spawn79, region33, chain1109, five-way M687 rewards and loaded-weapon policy.

**Deliverables / ownership:** binding-matrix.md with source lines, state owner and stale handoff claims. Keep evidence outputs in this campaign directory. Source paths listed without a directory resolve through CA-01A's current binding matrix; research outputs named here are planned artifacts, not files already available.

**Verification:** Read-only SQL/source reconciliation; check each claimed action against executor support. **UAT:** CA-U01, CA-U05, CA-U06, CA-U13, CA-U15 in [the presentation guide](uat-guide.md).

**Acceptance:** Every route object has a current binding, explicit gap or out-of-scope disposition. Record findings and limitations in the packet worknote before advancing the ledger.

**If blocked:** Name the missing prerequisite, affected actor/export/state and next investigative action. Research packets return bounded evidence gaps to CA-05; implementation packets wait for their contract and must not guess IDs. UAT failures return to the owning implementation packet and require the affected scenario to be rerun.

## CA-02A — Resolve Ambernol mesh and placement

**Status:** Ready. **Depends on:** CA-00A, CA-01A.  
**Owner/advisors:** items-systems-advisor; game-archaeology-specialist.  
**Entry points / inputs:** `db/resources/Entities/Seed/entity_templates.sql; db/resources/Items/Seed/items.sql; package-search.json`; [campaign evidence](evidence-ledger.md).

**Work and scope:** Inspect template18, item19, ShelfBox13 export1710, syringe mesh export5 and PFX syringe lead. Follow materials and transforms; inspect existing QA client counter side by side.

**Deliverables / ownership:** ambernol-verdict.md with visual captures, reference paths and exact intended pickup binding. Keep evidence outputs in this campaign directory. Source paths listed without a directory resolve through CA-01A's current binding matrix; research outputs named here are planned artifacts, not files already available.

**Verification:** Parsed references plus QA visual inspection; confirm pickup identity independent of icon. **UAT:** CA-U03 in [the presentation guide](uat-guide.md).

**Acceptance:** Original representation and transform proved, or bounded gap ready for CA-05B. Record findings and limitations in the packet worknote before advancing the ledger.

**If blocked:** Name the missing prerequisite, affected actor/export/state and next investigative action. Research packets return bounded evidence gaps to CA-05; implementation packets wait for their contract and must not guess IDs. UAT failures return to the owning implementation packet and require the affected scenario to be rerun.

## CA-02B — Audit remaining props and controls

**Status:** Ready. **Depends on:** CA-00A, CA-01A.  
**Owner/advisors:** items-systems-advisor; mission-systems-advisor.  
**Entry points / inputs:** `map-name-census.json; handoff-crosscheck.json; db/resources/Worlds/Seed/spawnlist.sql`; [campaign evidence](evidence-ledger.md).

**Work and scope:** Resolve corpse/start pickup, medical counter, internal rings, locker, Mess Hall table and crate actor/mesh/transform/state. Preserve SMG item21 and existing loot/reward contracts.

**Deliverables / ownership:** prop-verdicts.md with per-object disposition and evidence. Keep evidence outputs in this campaign directory. Source paths listed without a directory resolve through CA-01A's current binding matrix; research outputs named here are planned artifacts, not files already available.

**Verification:** Map/export reference joins and QA visual survey; no item or reward redesign. **UAT:** CA-U01, CA-U05, CA-U06, CA-U09, CA-U13 in [the presentation guide](uat-guide.md).

**Acceptance:** Every listed prop has a verified binding or bounded gap. Record findings and limitations in the packet worknote before advancing the ledger.

**If blocked:** Name the missing prerequisite, affected actor/export/state and next investigative action. Research packets return bounded evidence gaps to CA-05; implementation packets wait for their contract and must not guess IDs. UAT failures return to the owning implementation packet and require the affected scenario to be rerun.

## CA-03A — Trace cell door and cover marker

**Status:** Ready. **Depends on:** CA-00B, CA-01A.  
**Owner/advisors:** mission-systems-advisor; game-archaeology-specialist.  
**Entry points / inputs:** `kismet-selected.json; sequence-export-evidence.json`; [campaign evidence](evidence-ledger.md).

**Work and scope:** Trace Prisoner329CellDoor SetBool/PlaySound/Interp and TakeCoverIndicator ToggleHidden. Determine designer index, event mapping, phase and collision behavior from parsed references.

**Deliverables / ownership:** cell-door-cover-contract.md. Keep evidence outputs in this campaign directory. Source paths listed without a directory resolve through CA-01A's current binding matrix; research outputs named here are planned artifacts, not files already available.

**Verification:** Graph reference checks and isolated QA activation; record cancel/failure/success separately. **UAT:** CA-U02, CA-U04, CA-U16 in [the presentation guide](uat-guide.md).

**Acceptance:** Exact actor/event/state contract supported by evidence. Record findings and limitations in the packet worknote before advancing the ledger.

**If blocked:** Name the missing prerequisite, affected actor/export/state and next investigative action. Research packets return bounded evidence gaps to CA-05; implementation packets wait for their contract and must not guess IDs. UAT failures return to the owning implementation packet and require the affected scenario to be rerun.

## CA-03B — Separate stasis pods from block doors

**Status:** Ready. **Depends on:** CA-00B, CA-01A.  
**Owner/advisors:** mission-systems-advisor; game-archaeology-specialist.  
**Entry points / inputs:** `kismet-selected.json; db/resources/Content/Seed/castle_cellblock_chains.sql`; [campaign evidence](evidence-ledger.md).

**Work and scope:** Trace stasis assets individually. Existing StasisBlockDoubleDoors open/close graph is not proof of pod animation; chain1061 currently has no pod opening. Resolve phase timing and collision for each.

**Deliverables / ownership:** stasis-contract.md listing pods and doors separately. Keep evidence outputs in this campaign directory. Source paths listed without a directory resolve through CA-01A's current binding matrix; research outputs named here are planned artifacts, not files already available.

**Verification:** Graph/asset inspection and QA animation observation. **UAT:** CA-U07, CA-U16 in [the presentation guide](uat-guide.md).

**Acceptance:** Pod and block-door dispositions independently proved or explicitly unresolved. Record findings and limitations in the packet worknote before advancing the ledger.

**If blocked:** Name the missing prerequisite, affected actor/export/state and next investigative action. Research packets return bounded evidence gaps to CA-05; implementation packets wait for their contract and must not guess IDs. UAT failures return to the owning implementation packet and require the affected scenario to be rerun.

## CA-03C — Resolve escape energy barrier

**Status:** Ready. **Depends on:** CA-00B, CA-01A.  
**Owner/advisors:** game-archaeology-specialist; server-authority-enforcer.  
**Entry points / inputs:** `sequence-export-evidence.json; map-name-census.json`; [campaign evidence](evidence-ledger.md).

**Work and scope:** Trace barrier visual, collision, activation source, mission phase and streaming defaults. Establish whether client-local or server state owns each component.

**Deliverables / ownership:** barrier-contract.md. Keep evidence outputs in this campaign directory. Source paths listed without a directory resolve through CA-01A's current binding matrix; research outputs named here are planned artifacts, not files already available.

**Verification:** Inspect dependency graph; QA visibility and traversal before/after transition. **UAT:** CA-U08, CA-U17, CA-U19 in [the presentation guide](uat-guide.md).

**Acceptance:** Visibility and collision transition contract complete. Record findings and limitations in the packet worknote before advancing the ledger.

**If blocked:** Name the missing prerequisite, affected actor/export/state and next investigative action. Research packets return bounded evidence gaps to CA-05; implementation packets wait for their contract and must not guess IDs. UAT failures return to the owning implementation packet and require the affected scenario to be rerun.

## CA-03D — Resolve Armory door linkage

**Status:** Ready. **Depends on:** CA-00B, CA-01A.  
**Owner/advisors:** mission-systems-advisor.  
**Entry points / inputs:** `db/resources/Content/Seed/castle_cellblock_chains.sql; sequence-export-evidence.json`; [campaign evidence](evidence-ledger.md).

**Work and scope:** Trace exact door actors/events after terminal use. Chain1107 advances2356 to80688 and swaps highlights but has no door action. Preserve that mission progression.

**Deliverables / ownership:** armory-door-contract.md. Keep evidence outputs in this campaign directory. Source paths listed without a directory resolve through CA-01A's current binding matrix; research outputs named here are planned artifacts, not files already available.

**Verification:** Graph/source reconciliation and QA door/collision observation. **UAT:** CA-U14, CA-U16 in [the presentation guide](uat-guide.md).

**Acceptance:** Door linkage resolved independently of objective advancement. Record findings and limitations in the packet worknote before advancing the ledger.

**If blocked:** Name the missing prerequisite, affected actor/export/state and next investigative action. Research packets return bounded evidence gaps to CA-05; implementation packets wait for their contract and must not guess IDs. UAT failures return to the owning implementation packet and require the affected scenario to be rerun.

## CA-04A — Complete cinematic routing investigation

**Status:** Ready. **Depends on:** CA-00B, CA-01A.  
**Owner/advisors:** game-archaeology-specialist; bigworld-engine-advisor.  
**Entry points / inputs:** `ghidra-findings.md; crates/cell-content/src/cell/content/executor/dispatch.rs; db/resources/Events/Seed/sequences.sql`; [campaign evidence](evidence-ledger.md).

**Work and scope:** Continue SGW.exe trace from onSequence through activation and Director target selection/restoration. Current views0 and3 share activation slot; do not infer changing to3 fixes camera. Compare isolated1751 invocation with real chain1161 and capture payload/source lifetime.

**Deliverables / ownership:** camera-routing-contract.md with addresses, payload evidence and runtime observations. Keep evidence outputs in this campaign directory. Source paths listed without a directory resolve through CA-01A's current binding matrix; research outputs named here are planned artifacts, not files already available.

**Verification:** Independent protocol/capture verification; camera acquire/restore and wrong/missing source diagnostics. **UAT:** CA-U10 in [the presentation guide](uat-guide.md).

**Acceptance:** Cause demonstrated and smallest compatible remedy identified, or bounded unresolved diagnosis. Record findings and limitations in the packet worknote before advancing the ledger.

**If blocked:** Name the missing prerequisite, affected actor/export/state and next investigative action. Research packets return bounded evidence gaps to CA-05; implementation packets wait for their contract and must not guess IDs. UAT failures return to the owning implementation packet and require the affected scenario to be rerun.

## CA-04B — Recover original rift presentation

**Status:** Ready. **Depends on:** CA-00B, CA-04A.  
**Owner/advisors:** game-archaeology-specialist.  
**Entry points / inputs:** `package-search.json; sequence-export-evidence.json; db/resources/Content/Seed/castle_cellblock_chains.sql`; [campaign evidence](evidence-ledger.md).

**Work and scope:** Trace GroundVoid/Hold exports35142/35143 and scene dependencies. Determine timeline and attachment relative to Marsh removal. Existing chain1161 plays1751 and destroys Marsh immediately; do not duplicate sequence call.

**Deliverables / ownership:** rift-verdict.md with asset paths, timing and original-evidence confidence. Keep evidence outputs in this campaign directory. Source paths listed without a directory resolve through CA-01A's current binding matrix; research outputs named here are planned artifacts, not files already available.

**Verification:** Dependency and sequence inspection; isolated QA effect and real route observation. **UAT:** CA-U11 in [the presentation guide](uat-guide.md).

**Acceptance:** Original effect binding proved or search exhaustion documented for design. Record findings and limitations in the packet worknote before advancing the ledger.

**If blocked:** Name the missing prerequisite, affected actor/export/state and next investigative action. Research packets return bounded evidence gaps to CA-05; implementation packets wait for their contract and must not guess IDs. UAT failures return to the owning implementation packet and require the affected scenario to be rerun.

## CA-04C — Recover original blood aftermath

**Status:** Ready. **Depends on:** CA-00B, CA-01A.  
**Owner/advisors:** game-archaeology-specialist.  
**Entry points / inputs:** `package-search.json; sequence-export-evidence.json`; [campaign evidence](evidence-ledger.md).

**Work and scope:** Follow material/decal/particle dependencies including puddles632..634 and waterMat38. Name search negatives across361 QA packages are bounded evidence, not proof blood is absent. Inspect rendered candidates and stream persistence.

**Deliverables / ownership:** blood-verdict.md with bounded corpus/search and visual evidence. Keep evidence outputs in this campaign directory. Source paths listed without a directory resolve through CA-01A's current binding matrix; research outputs named here are planned artifacts, not files already available.

**Verification:** Parsed dependency search and QA material/decal observation. **UAT:** CA-U12 in [the presentation guide](uat-guide.md).

**Acceptance:** Original binding proved or unresolved appearance/timing constraints documented. Record findings and limitations in the packet worknote before advancing the ledger.

**If blocked:** Name the missing prerequisite, affected actor/export/state and next investigative action. Research packets return bounded evidence gaps to CA-05; implementation packets wait for their contract and must not guess IDs. UAT failures return to the owning implementation packet and require the affected scenario to be rerun.

## CA-05A — Specify state, timing and recovery

**Status:** Waiting. **Depends on:** CA-02A/B, CA-03A/B/C/D, CA-04A/B/C.  
**Owner/advisors:** bigworld-engine-advisor; aoi-witness-broadcast; server-authority-enforcer.  
**Entry points / inputs:** `binding-matrix.md; contracts from research packets`; [campaign evidence](evidence-ledger.md).

**Work and scope:** For every restoration define per-player/shared state, durable mission-derived reconstruction, enter/leave visibility, collision, idempotency and cinematic interruption. Choose existing supported protocol actions before new APIs.

**Deliverables / ownership:** restoration-spec.md with actor/event matrix and change list. Keep evidence outputs in this campaign directory. Source paths listed without a directory resolve through CA-01A's current binding matrix; research outputs named here are planned artifacts, not files already available.

**Verification:** Review range/phase/repeat attacks and two-client phase divergence; select regression test type before coding. **UAT:** CA-U16, CA-U17, CA-U18, CA-U19 in [the presentation guide](uat-guide.md).

**Acceptance:** Implementation contracts and receiver/recovery behavior explicit. Record findings and limitations in the packet worknote before advancing the ledger.

**If blocked:** Name the missing prerequisite, affected actor/export/state and next investigative action. Research packets return bounded evidence gaps to CA-05; implementation packets wait for their contract and must not guess IDs. UAT failures return to the owning implementation packet and require the affected scenario to be rerun.

## CA-05B — Design unresolved presentation

**Status:** Waiting. **Depends on:** Relevant CA-02/03/04 verdicts, CA-05A.  
**Owner/advisors:** game-archaeology-specialist; documentation-writer.  
**Entry points / inputs:** `restoration-spec.md; unresolved verdicts`; [campaign evidence](evidence-ledger.md).

**Work and scope:** Owner authorizes restoring originals and designing unresolved pieces. Use bounded original search first; label replacements as project design with visual/timing/state criteria and reference captures. Supersede earlier camera-only compromise where this campaign requires recovered or designed rift/blood.

**Deliverables / ownership:** design-dispositions.md and reviewable mockup/asset proposal for each unresolved item. Keep evidence outputs in this campaign directory. Source paths listed without a directory resolve through CA-01A's current binding matrix; research outputs named here are planned artifacts, not files already available.

**Verification:** Compare QA visual proposal with documented requirements and route constraints. **UAT:** CA-U03, CA-U07, CA-U08, CA-U11, CA-U12 in [the presentation guide](uat-guide.md).

**Acceptance:** Each gap has implementable original/design disposition; no invented retail claim. Record findings and limitations in the packet worknote before advancing the ledger.

**If blocked:** Name the missing prerequisite, affected actor/export/state and next investigative action. Research packets return bounded evidence gaps to CA-05; implementation packets wait for their contract and must not guess IDs. UAT failures return to the owning implementation packet and require the affected scenario to be rerun.

## CA-06A — Implement prop and pickup corrections

**Status:** BlockedEvidence. **Depends on:** CA-05A; CA-05B where applicable.  
**Owner/advisors:** rust-gameserver-dev; items-systems-advisor.  
**Entry points / inputs:** `db/resources/Entities/Seed/entity_templates.sql; db/resources/Worlds/Seed/spawnlist.sql; db/resources/Items/Seed/items.sql`; [campaign evidence](evidence-ledger.md).

**Work and scope:** Apply only proved mesh/placement/presentation corrections. Coordinate shared seed edits with CA-06E. Preserve reward, item identity and loot semantics.

**Deliverables / ownership:** Minimal seed/runtime diff and worknotes/CA-06A.md. Keep evidence outputs in this campaign directory. Source paths listed without a directory resolve through CA-01A's current binding matrix; research outputs named here are planned artifacts, not files already available.

**Verification:** Chain replay for changed pickup behavior; live-DB only for changed SQL semantics; QA visuals. **UAT:** CA-U01, CA-U03, CA-U05, CA-U06, CA-U09, CA-U13 in [the presentation guide](uat-guide.md).

**Acceptance:** Correct visuals and pickup behavior with regression checks passing. Record findings and limitations in the packet worknote before advancing the ledger.

**If blocked:** Name the missing prerequisite, affected actor/export/state and next investigative action. Research packets return bounded evidence gaps to CA-05; implementation packets wait for their contract and must not guess IDs. UAT failures return to the owning implementation packet and require the affected scenario to be rerun.

## CA-06B — Implement doors, cover, pods and barrier

**Status:** BlockedEvidence. **Depends on:** CA-05A; CA-05B where applicable.  
**Owner/advisors:** rust-gameserver-dev; mission-systems-advisor; server-authority-enforcer.  
**Entry points / inputs:** `crates/cell-content/src/cell/content/executor/; crates/content-engine/src/; castle_cellblock_chains.sql via CA-06E`; [campaign evidence](evidence-ledger.md).

**Work and scope:** Implement contract-specific actions and mission conditions, keeping pod and door targets separate. Use coordinator integration for chain SQL; no speculative designer/event IDs.

**Deliverables / ownership:** Runtime action diff and worknotes/CA-06B.md. Keep evidence outputs in this campaign directory. Source paths listed without a directory resolve through CA-01A's current binding matrix; research outputs named here are planned artifacts, not files already available.

**Verification:** Chain replay success/failure/cancel/repeat; wire-format for changed payloads; collision QA. **UAT:** CA-U02, CA-U04, CA-U07, CA-U08, CA-U14, CA-U16 in [the presentation guide](uat-guide.md).

**Acceptance:** Exact targets respond once in intended phases and reject invalid triggers. Record findings and limitations in the packet worknote before advancing the ledger.

**If blocked:** Name the missing prerequisite, affected actor/export/state and next investigative action. Research packets return bounded evidence gaps to CA-05; implementation packets wait for their contract and must not guess IDs. UAT failures return to the owning implementation packet and require the affected scenario to be rerun.

## CA-06C — Implement cinematic and aftermath bindings

**Status:** BlockedEvidence. **Depends on:** CA-05A, CA-04A; CA-05B where applicable.  
**Owner/advisors:** rust-gameserver-dev; bigworld-engine-advisor; game-archaeology-specialist.  
**Entry points / inputs:** `crates/cell-content/src/cell/content/executor/dispatch.rs; event/sequence seeds; chain SQL via CA-06E`; [campaign evidence](evidence-ledger.md).

**Work and scope:** Apply proven camera remedy and rift/blood timing. Retain existing1751 invocation unless evidence requires change; preserve mission/dialog progression and explicit camera restoration.

**Deliverables / ownership:** Cinematic diff and worknotes/CA-06C.md. Keep evidence outputs in this campaign directory. Source paths listed without a directory resolve through CA-01A's current binding matrix; research outputs named here are planned artifacts, not files already available.

**Verification:** Wire-format against independent evidence; chain replay timing; QA camera interruption and aftermath. **UAT:** CA-U10, CA-U11, CA-U12 in [the presentation guide](uat-guide.md).

**Acceptance:** Camera restores and effects follow contracted timeline without duplicate playback. Record findings and limitations in the packet worknote before advancing the ledger.

**If blocked:** Name the missing prerequisite, affected actor/export/state and next investigative action. Research packets return bounded evidence gaps to CA-05; implementation packets wait for their contract and must not guess IDs. UAT failures return to the owning implementation packet and require the affected scenario to be rerun.

## CA-06D — Implement replay and observer recovery

**Status:** BlockedEvidence. **Depends on:** CA-05A; contracts from CA-06A/B/C.  
**Owner/advisors:** rust-gameserver-dev; aoi-witness-broadcast; server-authority-enforcer.  
**Entry points / inputs:** `crates/cell-content/src/cell/; crates/cell-content/src/cell/content/chain_replay_tests/`; [campaign evidence](evidence-ledger.md).

**Work and scope:** Reconstruct presentation after relog, late AoI and map streaming according to state ownership. Avoid leaking one player phase to another; maintain collision consistency. Coordinate changes to executor modules with CA-06B/C.

**Deliverables / ownership:** Recovery diff and worknotes/CA-06D.md. Keep evidence outputs in this campaign directory. Source paths listed without a directory resolve through CA-01A's current binding matrix; research outputs named here are planned artifacts, not files already available.

**Verification:** Chain replay relog; fan-out byte assertions on real receiver; concurrency where shared state changes. **UAT:** CA-U17, CA-U18, CA-U19 in [the presentation guide](uat-guide.md).

**Acceptance:** All state boundaries recover deterministically for two different mission phases. Record findings and limitations in the packet worknote before advancing the ledger.

**If blocked:** Name the missing prerequisite, affected actor/export/state and next investigative action. Research packets return bounded evidence gaps to CA-05; implementation packets wait for their contract and must not guess IDs. UAT failures return to the owning implementation packet and require the affected scenario to be rerun.

## CA-06E — Integrate shared seeds and regression suite

**Status:** Waiting. **Depends on:** CA-06A/B/C/D; CA-07A if required.  
**Owner/advisors:** Coordinator; database-persistence; testing-validation-engineer.  
**Entry points / inputs:** `db/resources/Content/Seed/castle_cellblock_chains.sql; other changed seed files; TESTING.md`; [campaign evidence](evidence-ledger.md).

**Work and scope:** One owner serializes shared SQL changes and joins runtime actions with seeds. Validate existing1109 exit and1107 objective progression, class reward/ammo contracts and chain1061 minigame behavior.

**Deliverables / ownership:** Integrated diff, test results and migration/reseed instructions in worknotes/CA-06E.md. Keep evidence outputs in this campaign directory. Source paths listed without a directory resolve through CA-01A's current binding matrix; research outputs named here are planned artifacts, not files already available.

**Verification:** Required targeted tests through Windows pinned build lane; regression guards fail on reversion; live-DB when needed. **UAT:** CA-U14, CA-U15, CA-U16, CA-U20 in [the presentation guide](uat-guide.md).

**Acceptance:** Integrated changes pass required checks and preserve sibling contracts. Record findings and limitations in the packet worknote before advancing the ledger.

**If blocked:** Name the missing prerequisite, affected actor/export/state and next investigative action. Research packets return bounded evidence gaps to CA-05; implementation packets wait for their contract and must not guess IDs. UAT failures return to the owning implementation packet and require the affected scenario to be rerun.

## CA-07A — Conditional QA client asset work

**Status:** Conditional. **Depends on:** CA-05A, CA-05B; demonstrated client-side necessity.  
**Owner/advisors:** game-archaeology-specialist; bigworld-engine-advisor.  
**Entry points / inputs:** `Existing QA CookedPC; crates/upk/; applicable existing tools under tools/`; [campaign evidence](evidence-ledger.md).

**Work and scope:** Implement only required recovered/designed assets or cooked binding changes. Document deterministic patch, original hash, rollback and compatibility; keep backups outside CookedPC. If server-only resolution suffices, record evidence-backed NotApplicable.

**Deliverables / ownership:** Patch/art artifact manifest, install/rollback guide and worknotes/CA-07A.md. Keep evidence outputs in this campaign directory. Source paths listed without a directory resolve through CA-01A's current binding matrix; research outputs named here are planned artifacts, not files already available.

**Verification:** QA load/stream/rollback visual verification; no proprietary binaries committed. **UAT:** Affected CA-U03..CA-U14, CA-U18 in [the presentation guide](uat-guide.md).

**Acceptance:** Client revision and installation reproducible; rollback verified or packet NotApplicable. Record findings and limitations in the packet worknote before advancing the ledger.

**If blocked:** Name the missing prerequisite, affected actor/export/state and next investigative action. Research packets return bounded evidence gaps to CA-05; implementation packets wait for their contract and must not guess IDs. UAT failures return to the owning implementation packet and require the affected scenario to be rerun.

## CA-08A — Run isolated presentation and negative UAT

**Status:** Waiting. **Depends on:** CA-06E, CA-07A disposition.  
**Owner/advisors:** testing-validation-engineer; QA owner.  
**Entry points / inputs:** `uat-guide.md; existing QA client`; [campaign evidence](evidence-ledger.md).

**Work and scope:** Run CA-U01..16 and targeted interruption cases on recorded build/client/seed revisions. Distinguish diagnostic GM setup from genuine route completion. Capture actual visual/collision/camera results.

**Deliverables / ownership:** worknotes/CA-08A.md with PASS/FAIL/BLOCKED/NOT RUN and evidence. Keep evidence outputs in this campaign directory. Source paths listed without a directory resolve through CA-01A's current binding matrix; research outputs named here are planned artifacts, not files already available.

**Verification:** Manual QA plus retained automated regression results; frontend REPL and visual only if frontend changed. **UAT:** CA-U01..CA-U16, CA-U21 conditional in [the presentation guide](uat-guide.md).

**Acceptance:** Every scenario has evidence; failures become named repair packets. Record findings and limitations in the packet worknote before advancing the ledger.

**If blocked:** Name the missing prerequisite, affected actor/export/state and next investigative action. Research packets return bounded evidence gaps to CA-05; implementation packets wait for their contract and must not guess IDs. UAT failures return to the owning implementation packet and require the affected scenario to be rerun.

## CA-08B — Run full route, relog, streaming and multiplayer UAT

**Status:** Waiting. **Depends on:** CA-08A fixes verified.  
**Owner/advisors:** testing-validation-engineer; movement-teleport-advisor; aoi-witness-broadcast.  
**Entry points / inputs:** `uat-guide.md; sibling CellBlock and ring UAT guides`; [campaign evidence](evidence-ledger.md).

**Work and scope:** Complete each supported CellBlock route through mission688 and World12 to8 without GM forcing. Exercise relog around all state boundaries, unload/reload, late AoI and two-client divergent phases.

**Deliverables / ownership:** worknotes/CA-08B.md with branch coverage, captures and residual gaps. Keep evidence outputs in this campaign directory. Source paths listed without a directory resolve through CA-01A's current binding matrix; research outputs named here are planned artifacts, not files already available.

**Verification:** Full route manual UAT and recovery matrix; verify ring arrival contract. **UAT:** CA-U15, CA-U17, CA-U18, CA-U19, CA-U20 in [the presentation guide](uat-guide.md).

**Acceptance:** All supported branches and recovery cases pass or campaign remains open with specific blockers. Record findings and limitations in the packet worknote before advancing the ledger.

**If blocked:** Name the missing prerequisite, affected actor/export/state and next investigative action. Research packets return bounded evidence gaps to CA-05; implementation packets wait for their contract and must not guess IDs. UAT failures return to the owning implementation packet and require the affected scenario to be rerun.

## CA-09A — Close campaign and publish evidence

**Status:** Waiting. **Depends on:** CA-08B; all required failures repaired.  
**Owner/advisors:** documentation-writer; Coordinator.  
**Entry points / inputs:** `README.md; evidence-ledger.md; docs/readme.md; docs/project-status.md; docs/gap-analysis.md`; [campaign evidence](evidence-ledger.md).

**Work and scope:** Reconcile verified original bindings versus project design, remaining debt, packet results and release disposition. Update status/gap documents once at closeout and runtime documentation with behavior PRs. Do not hand-edit generated blocks.

**Deliverables / ownership:** closeout.md, indexed evidence and reproducible release/rollback notes. Keep evidence outputs in this campaign directory. Source paths listed without a directory resolve through CA-01A's current binding matrix; research outputs named here are planned artifacts, not files already available.

**Verification:** Link and ledger checks; confirm every acceptance gate and UAT result has evidence. **UAT:** CA-U01..CA-U20; CA-U21 disposition in [the presentation guide](uat-guide.md).

**Acceptance:** Restoration through in-game UAT achieved; deployment state accurately recorded. Record findings and limitations in the packet worknote before advancing the ledger.

**If blocked:** Name the missing prerequisite, affected actor/export/state and next investigative action. Research packets return bounded evidence gaps to CA-05; implementation packets wait for their contract and must not guess IDs. UAT failures return to the owning implementation packet and require the affected scenario to be rerun.
