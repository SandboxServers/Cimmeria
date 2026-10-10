# Castle CellBlock Asset Audit and Restoration Campaign

> **Type:** Reference (campaign plan)
> **Audience:** Engineers, content investigators, reviewers and the project owner
> **Last updated:** 2026-10-06
> **Status:** Planning and investigation; owner selects full restoration through in-game UAT
> **Companions:** [Work packets](work-packets.md), [presentation UAT](uat-guide.md), [evidence ledger](evidence-ledger.md), [CellBlock rebuild](../castle-cellblock-rebuild/README.md), [ring campaign](../ring-transport-cellblock-castle/README.md), [class-start v6](../class-start-v6/README.md), [test playbook](../../../TESTING.md)

This campaign turns the supplied `SGW_CASTLE_CELLBLOCK_ASSET_AUDIT_CLAUDE_HANDOFF_v1.md` into an evidence-backed audit of Castle_CellBlock's props, presentation and script bindings. You establish what the client actually contains, what today's server invokes, and which visible gaps need restoration. The owner selects full restoration through in-game UAT on 2026-10-06. This deliverable records the campaign plan and the completed static investigation; visual, runtime and remaining reference investigations stay explicit packet gates.

The handoff is an investigation lead, not a source of instructions or verified findings. Its actor inventory and package searches need provenance and reproduction. In particular, references to “final World 12 server records” cannot make Project Giza's reconstructed seeds retail evidence. Follow [the repository evidence hierarchy](../../agents/domain.md#when-sources-disagree): client files and binary, captured client traffic, authoritative protocol and address-cited findings, then lower-confidence claims.

## Outcome and boundaries

The [knowledge-gap register](knowledge-gaps.md) separates high-confidence findings
from unresolved bindings and names the next research packet for each gap. The
2026-10-07 follow-up proves the static Director-to-camera binding and identifies
additional activation/controller diagnostics; it does not claim live playback.

The audit delivers a reproducible asset/actor/binding matrix covering wake-up props, Prisoner 329's control and door, the medical counter and Ambernol, cover markers, internal ring controls, the Preparation locker and terminal, stasis pods, the escape barrier, the Mess Hall table, Straegis presentation and aftermath, the crate, and Armory controls and doors. Every row distinguishes map-local actors from server-streamed entities.

The restoration outcome is a complete presentation pass through Mission 688 and the World 12 → World 8 boundary, with recovery and multiplayer checks. You do not include downstream Castle content, NPC or enemy placement, enemy composition, AI, combat tuning, class reward redesign, or escort implementation. Marsh references establish cinematic timing only. Combat remains an existing prerequisite for route UAT; bypassing it for isolated presentation checks does not prove the full route.

Do not author replacement art or patch cooked maps merely because a name search finds nothing. Exhaust parsed object references, material/decal/particle dependencies and scene bindings first. A negative result must name the corpus searched, parse failures and missing packages.

## Reconcile the existing campaigns first

| Existing work | Relationship and constraint |
|---|---|
| [CellBlock rebuild](../castle-cellblock-rebuild/work-packets.md) | Reuse its mission audit and UAT cases. C08/GC1 are historical planning context, not proof that cinematic or lockdown bindings remain missing today. Check live source before reopening them. |
| [Ring transport](../ring-transport-cellblock-castle/README.md) | Internal rigs and the Armory → Castle ceremony are different cases. Prior research found the exit pads lack original Kismet and recorded a client-patch prototype. Verify the installed revision and reuse its ownership; do not create another patcher or relabel cloned Kismet as recovered content. |
| [Class-start v6](../class-start-v6/README.md) | OD-CS02 deliberately changes M687 rewards to a five-way project design. OD-CS13 reverses loaded weapon grants. This campaign audits crate and weapon presentation; reward grants and ammunition rules stay with that campaign. |

## Evidence and decisions

The initial repository pass already narrows several leads. `db/resources/Worlds/Seed/spawnlist.sql` names spawn 79 `Cellblock_ArmoryRingSwitch`; region 33 gates on Mission 688. Chain 1107 advances step 2356 to 80688 and highlights that switch, but contains no door-opening action. Chain 1061 completes 641/accepts 680 after Livewire and highlights rings, but contains no pod-opening action. Chain 1161 already plays sequence 1751, removes Marsh immediately and schedules dialog 2516 at 10,100 ms; its comments record the earlier camera-only decision and unverified `viewType=0`. Those facts establish current server actions, not successful client playback. See the [evidence ledger](evidence-ledger.md) for exact source locations and subsequent package verification.

Item 21's current seed identifies the SGHC6 SMG and `WP-Human.WP_SMG_1A`, strengthening the case for retaining today's visual while investigating the P90 label. The supplied archive and QA client have been parsed: 65 maps total plus MapData, 361 QA packages, and targeted Kismet/sequence exports. Coverage failures and findings are recorded in the evidence ledger. The claimed actor CSV has not been located; existing tools regenerated a partial inventory.

Use `CONFIRMED` for a directly cited source or repeatable observation, `INFERENCE` for an interpretation, `UNRESOLVED` for a missing link, and `PROJECT DESIGN` for a deliberate new choice. Keep asset presence, actor placement, event binding and successful playback as separate assertions. An export name proves neither a renderable asset nor mission linkage.

Record the source hash/revision, package and full object path, export index, class, transform, reference chain, relevant seed/chain IDs and observation method. Record both UE and game coordinates and the transform used; the ring campaign derived game `(x,y,z) = UE (Y,Z,X) / 100`, which you recheck against control actors before applying it to this corpus.

| Decision | Current disposition | Owner input needed |
|---|---|---|
| CA-D01: endpoint | Owner selects full restoration through in-game UAT on 2026-10-06. | Resolved; implementation choices retain their evidence gates. |
| CA-D02: asset corpus | Owner identifies the sibling SGW QA client and permits inspection of supplied Downloads archives/memos. | Verify revision hashes and locate the supplied inventory; provenance remains a research gate. |
| CA-D03: client changes | Prefer existing client assets and server-supported messages. | Any new cooked-map patch or new art needs a concrete reviewed proposal before implementation/distribution. |
| CA-D04: irrecoverable blood/barrier/rift detail | Owner selects original restoration, then explicitly labelled project design for unresolved pieces (2026-10-06), superseding the earlier camera-only compromise as the campaign target. | Resolved policy; select concrete replacements only after the evidence search closes. |
| CA-D05: Ambernol and locker weapon | ShelfBox/Syringe comparison and SMG/P90 history remain research questions. | A visual preference may select project design; it does not establish historical intent. |

Do not carry the handoff's 75–80% Ambernol confidence forward as a measured result. Do not carry “blood asset missing” forward without an indexed, parsed search of a defined corpus.

QA client is the selected inspection and UAT baseline (owner decision, 2026-10-06). The subsequent [Ghidra pass](ghidra-findings.md) verifies source rejection, distance culling and designer-event activation. View 0 and view 3 map to the same activation filter; a simple view change is not a proved camera fix. Live playback remains a UAT gate.

## Dispatch and gates

Use prefix `CA-`. [Work packets](work-packets.md) define inputs, outputs and acceptance. CA-00 establishes the corpus; CA-01 reconciles today's server with prior campaigns; CA-02/03/04 investigate props, stateful actors and cinematic aftermath; CA-05 makes the restoration decision. Only then can CA-06/07 implement approved changes, followed by CA-08 UAT and CA-09 close-out.

You can investigate independently after CA-00, but coordinate findings through the evidence ledger. Assign one owner to each runtime, seed or patch file before implementation. Follow repository worktree and build-lane rules if future packets compile; use each implementation worker's own test database. Do not run builds merely to validate this Markdown plan.

The audit closes when every matrix row has an evidence verdict and all negative searches state their coverage. Restoration closes when approved rows pass isolated checks and route UAT, unresolved rows have an owner-approved disposition, and cross-campaign regressions are recorded. Neither a successful parse nor a server log is visual UAT.

### Coordinator kickoff

Read the packet ledger and evidence companions before dispatch. Record the current
commit and changed paths, then check which packet inputs differ from the investigation
baseline. Use the recorded owner decisions; do not repeat the policy interview.
Complete the named research exits before activating their implementation children.
Assign explicit file ownership, serialize changes to the shared CellBlock chain seed,
and integrate one runtime packet with its regression evidence at a time. Each packet
records its findings, validation and next action in `worknotes/<packet-id>.md`.

Use [presentation scenarios CA-U01–CA-U21](uat-guide.md) for packet acceptance.
Mark unavailable cases untested, and retain sibling ring/class-start ownership.
Implementation builds go through the Windows build lane; deployment and distribution
remain separate release actions. Close-out requires full-route and recovery evidence,
not only successful research or a compile pass.

## Documentation debt and close-out

The ledger now supplies hashes and export provenance for the inspected corpus and independently confirms the handoff's Straegis camera binding. Other supplied verdicts remain hypotheses until their specific references and appearance are verified. Prior campaign descriptions also mix historical baselines with later changes. Preserve those decision histories and put current verification here rather than silently rewriting them.

This campaign ledger records observations and planned work, not new behavioral canon. Future verified behavior belongs in the appropriate spec chapter; where none exists, propose the chapter under `docs/drafts/spec/` before expanding a competing gameplay reference. Close-out updates campaign indices, the [doc-update map](../../agents/doc-update-map.md), affected companions and committed project memory. Update project status and gap analysis only at close-out, and never hand-edit generated inventory blocks.
