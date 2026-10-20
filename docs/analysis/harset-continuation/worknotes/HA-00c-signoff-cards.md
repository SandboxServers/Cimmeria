# HA-00c sign-off cards: nine Harset mission graphs and the shared claims

> Type: reference (sign-off packets, drafts). Audience: the mission-systems advisor, the coordinator, Lomiada and the owner. Drafted: 2026-10-18 against campaign tip `3e5f503e861fc39d37526732f2846adfbc2bf053` (branch `content/harset-campaign`). Docs only: no ID is allocated, no seed or code was changed, no cargo, lab or client was used.
> Companions: [HA-00c result](HA-00c.md) (section 8 is the list these cards expand), [ledger](../README.md), [packet contracts](../work-packets.md), [ID proposals](../id-allocation.md), [Lomiada brief](../lomiada-brief.md), [HA-15 audit](HA-15-audit.md), [owner decisions 2026-10-14](../../four-zone-planning/owner-decisions-2026-10-14.md), [parent HA-00c contract](../../four-zone-planning/harset-work-packets.md#ha-00c--seal-authored-and-repeatable-id-allocations).

## 1. What this file is, and what it is not

[HA-00c](HA-00c.md) lists nine graphs (six authored stories, three repeatables) and one shared-claims row, with every signature blank. This file gives each one a card: what exists, what is missing, what the signer would be attesting to, and the open decisions in the wording the [Lomiada brief](../lomiada-brief.md) uses.

- **Every signature cell and every ID cell is blank on purpose.** Nothing here is a signature, a reservation or an allocation. A card with a filled-in ID would be a defect in this file.
- **None of the nine graphs is authored.** The seeds and the stock pak hold no row for them. The mission, step, objective and task rows that cards 1 to 9 can cite are therefore empty by fact, not by omission. What the cards can cite is the surrounding evidence: the packet contract, the chains already preserved in the three faction seeds, the items the gating packets need and the decisions still open.
- **A signature is not a playability claim.** Cellblock's first authored quest has no client proof yet, so no authored quest is known to display, accept or advance in the client. That stays BlockedEvidence and owner-manual ([HA-00c section 9](HA-00c.md#9-what-this-does-not-prove)).
- **Sign-readiness on every card reads NOT READY.** The signer cannot honestly attest to a graph that does not exist. The attestation lines say what the signer would be attesting to once the graph exists; they are a checklist, not a request to sign today.

### Evidence labels

| Label | Meaning |
|---|---|
| FACT | A file, seed row, pak entry or recorded packet result says it. The source is named. |
| INFERENCE | Read from text, adjacency, packet ordering or engine habit. Confidence is high, medium or low. |
| DECISION | Recorded owner or coordinator decision (ledger or owner register). It authorizes work; it is not UAT. |

## 2. Facts shared by all nine cards

These apply to every card, so the cards do not repeat them.

| Topic | Fact | Source |
|---|---|---|
| Authored ID space | Nothing authored is allocated. The static audit found zero collisions across 109 refs and added no claim. | FACT: [HA-00c sections 4 and 5](HA-00c.md#4-result-claim-tables) |
| Stock catalog | `CookedDataMissions.pak` holds 1040 numeric entries (IDs 38 to 1826). Seeds hold 1041 missions; the one seed-only mission is 689. Ten seed-only occupants (689, 68900, 80622, 80623, 80641, 80688, 90622, 90623, 90641, 90688) are real even though the pak does not list them, so a pak-only gap scan is not enough. | FACT: [HA-00c section 3.2](HA-00c.md#32-stock-census-pak-against-the-seeds-at-the-tip) |
| Catalog policy | Root IDs 1 to 65535; 355840-byte cap. This is a policy bound, not a reservation. | FACT: CB-N05a, [HA-00c section 4.2](HA-00c.md#42-merged-claim-table-authored-and-repeatable-mission-family) |
| IDs already claimed in the authored family | Test reservation 61000 to 61003; Cellblock sibling claims 62050, 62060 (missions), 62051, 62054, 62061 (steps), 62052, 62055, 62062 (objectives), 62053, 62056, 62063 (tasks), dialogs 60150, 60151, 60160. These are branch-snapshot claims, not Harset's. | FACT: [HA-00c section 4.2](HA-00c.md#42-merged-claim-table-authored-and-repeatable-mission-family) |
| Not reusable | The 73 reviewed census IDs, the four carry-ins (567, 1360, 1377, 1410) and mission 1664. A guessed gap, a historical mission reuse or a server-only row cannot close the gate. | FACT: [packet contract](../work-packets.md#ha-00c-specialist-allocation-gate) and [parent contract](../../four-zone-planning/harset-work-packets.md#ha-00c--seal-authored-and-repeatable-id-allocations) |
| Provenance | New Cimmeria authoring, marked `CIMMERIA_AUTHORING` in seed and ledger. No player-facing provenance label. Copy must not read as verbatim 2009 material. | DECISION: O14-10, O14-11 |
| Allocation authority | The coordinator may allocate individual IDs after a stock, shared-table, seed and active-branch collision audit. | DECISION: O14-04, HA-D09 |
| Objective primitives | KillCount, CollectItem, VisitRegion, TalkToNpc and UseObject are server-side primitives. The client sees the wire shape, not the type names. A graph is built from existing chain actions; no new executor arm is assumed. | FACT: [HA-00c section 8](HA-00c.md#8-result-mission-graph-signoff-list) |
| Reward path | The existing grant actions for XP, cash and item. Amounts are not recorded anywhere. Ceiling: modest XP and currency, no new currency, no high-tier gear. | DECISION: O14-11, O14-12; amounts open |
| Hard preconditions for HA-17 and HA-18 | HA-00c complete (all nine graph signoffs plus the coordinator's shared signoff), CB-N05a client-proven by Cellblock's first authored quest, restoration UAT, owner gates. | FACT: [packet contract](../work-packets.md#ha-00c-specialist-allocation-gate) |
| Chain IDs already taken in Harset | `harset_space_chains.sql` 6001 to 6007; `harset_goauld_chains.sql` 6101 to 6127; `harset_jaffa_chains.sql` 6301, 6303 to 6308, 6331, 6333 to 6342; `harset_opcore_chains.sql` 6501 to 6505, 6511 to 6516, 6518 to 6523, 6525 to 6528. 6309 and 6310 stay reserved historically. Per-mission ranges are reserved to restoration packets, not to HA-17 or HA-18. | FACT: [ID proposals](../id-allocation.md) |
| Chains for the nine graphs | None exist. No candidate range in [ID proposals](../id-allocation.md) is addressed to HA-17 or HA-18, so every chain ID for these graphs is unassigned. | FACT: ID proposals, read in full |

### Engine traps every graph signer should check

These come from the engine contract in the [packet contracts](../work-packets.md#current-engine-contract-the-old-h-ledger-cannot-supply) and the mission advisor's standing traps.

| Trap | What the signer checks |
|---|---|
| One event fires every matching chain | Sibling chains on one trigger key must have pairwise disjoint persisted mission, step and objective guards. `priority` never excludes. |
| Re-click re-fires dialog chains | Every one-shot effect (grant, reward, accept) carries a persisted gate such as `mission_status ... eq not_active`. `once` resets on relog and world change since #802, so it is not a persisted guard. |
| Action order | Item and bind cleanup go before the mission mutation that emits the next event. Conditions are frozen when the event resolves. |
| Non-interact dialogs | Chains that are not interactions can show only monologue dialogs; choice buttons are keyed by screen. |
| Shared hub actors | Worlds 57 and 68 are shared. No quest may mutate a shared actor's hostility, visibility or despawn state for everyone. |
| `repeats` persistence | The world-entry mission UPSERT does not persist `repeats`. No repeatable can be proved until its own architect-approved arm exists. |

## 3. Cards 1 and 2: HA-17a, two authored Jaffa stories

### Card 1: HA-17a Jaffa story 1

| Field | Content |
|---|---|
| Graph | Authored side story 1 of 2, Jaffa. Branch/commit sealed as `harset/ha-17a-jaffa-stories`, `Harset HA-17a: author Jaffa side stories`. |
| Premise | The contract names two premises for the pair: "watch rotation and supply routing between proved district actors". Which premise is story 1 and which is story 2 is not recorded. INFERENCE (low): watch rotation first, from the listing order. |
| Mission, steps, objectives, tasks | None. No row in the pak or the seeds. Shape from the contract: a visible accept, two meaningful steps using existing district activity, a closing dialog and a reward. FACT for the shape; the rows are open. |
| Grant and recipient | Not named. The contract says only "named actors" are to be owner-approved and the accept is visible. No accepting NPC, no closing NPC, no dialog set is chosen. The actors must be "proved district actors" (world 57); the proving evidence is not recorded. |
| Reward | "Useful supply plus modest XP/currency" (contract). No supply item, XP or currency amount is recorded. |
| Items needed | None named. The HA-15 mission-item census lists 23 items and none belongs to a Jaffa mission (1324, 1325 or 1326 appear nowhere in it). FACT: [HA-15 audit section 3.3](HA-15-audit.md#33-required-items-and-their-sources). The supply reward item, if any, is open. |
| Chains and IDs preserved | `harset_jaffa_chains.sql`: 6301, 6303 to 6308 (mission 1324), 6331, 6333 to 6342 (mission 1326). 6311 to 6330 reserved to 1325 and empty. 6309 and 6310 historical. None is available to this graph. Authored mission, step, objective, task, dialog and map IDs: unassigned. |
| Upstream gate | Restored Jaffa arc client UAT, which includes 1325. 1325 is unauthored, so 1326 cannot start. FACT: [ledger](../README.md#historical-packet-reconciliation-against-the-baseline), rows H21 and H22. |
| Evidence class | Shape and gates: FACT. Premise ordering and any actor choice: INFERENCE or open. Reward values: open. |
| Open decisions (Lomiada brief wording) | "Premise, reward and reset policy: Concrete values for six authored stories and three repeatables." "Sign-off of the nine mission graphs: Sign or reject each graph." Also the 1325 line: "Authoring 1325 and the Mala'c duel: Author chains in 6311 to 6330; duel shape." |
| What the signer attests to | (a) The graph's exact mission, step, objective, task, dialog, dialog-map, item and chain IDs are listed. (b) Each is absent from the stock pak, the seeds and every active-branch snapshot at the recorded SHA, and none reuses a census ID, a carry-in or 1664. (c) The objective primitives and reward actions exist in the engine as used. (d) The accepting and closing actors are real, proved actors and no shared actor is mutated. (e) A persisted guard stops a second reward. (f) The premise, reward and any supply item are recorded with `CIMMERIA_AUTHORING` provenance. (g) Cellblock's first authored quest has a client proof. |
| Sign-readiness | NOT READY: no graph, no IDs, no actor, no reward value, no upstream UAT, no client catalog proof. |
| Mission-advisor signature | (blank) |
| ID allocation | (blank) |

### Card 2: HA-17a Jaffa story 2

| Field | Content |
|---|---|
| Graph | Authored side story 2 of 2, Jaffa. Same branch and commit as card 1. |
| Premise | The other half of "watch rotation and supply routing between proved district actors". See card 1. |
| Mission, steps, objectives, tasks | None; same contract shape as card 1. |
| Grant, recipient, reward, items | As card 1. The two stories must not share an accept NPC state in a way that lets one story's chain fire on the other's dialog (the one-event-many-chains trap above). |
| Chains and IDs preserved | As card 1. |
| Evidence class | As card 1. |
| Open decisions (Lomiada brief wording) | As card 1. |
| What the signer attests to | As card 1, plus: story 2's guards are disjoint from story 1's on every shared trigger key, and the two missions do not share a step ID or a completion chain. |
| Sign-readiness | NOT READY, same reasons as card 1. |
| Mission-advisor signature | (blank) |
| ID allocation | (blank) |

## 4. Cards 3 and 4: HA-17b, two authored OP-CORE stories

### Card 3: HA-17b OP-CORE story 1

| Field | Content |
|---|---|
| Graph | Authored side story 1 of 2, OP-CORE. Sealed as `harset/ha-17b-opcore-stories`, `Harset HA-17b: author OP-CORE side stories`. |
| Premise | The contract names "lab calibration and casualty aid around validated lab/plaza actors and props". Story assignment not recorded. INFERENCE (low): calibration first. |
| Mission, steps, objectives, tasks | None. Contract shape: accept, intervention, debrief. "Calibration uses an existing service/minigame only if its result contract is proved." |
| Grant and recipient | Not named. "Validated lab/plaza actors" are not listed; validation evidence is not recorded. |
| Reward | Modest XP/currency; supplies "consumed once" (contract). Values and the supply item are open. |
| Items needed | None named for the authored graphs. The gating packets (HA-09, HA-10) need items with no source: 4396 Straegis Scanner (mission 1374; event 5 ability 2092), 4690 Nanite Tracking Gun (1363; event 5 ability 2409), 2734 Monitoring Device (1352, 1362). All GAP. Whether story 1 consumes any of them is not decided. FACT: [HA-15 audit section 3.3](HA-15-audit.md#33-required-items-and-their-sources). |
| Chains and IDs preserved | `harset_opcore_chains.sql`: 6501 to 6505, 6511 to 6516, 6518 to 6523, 6525 to 6528. 6501 (Marsh receipt), 6502, 6505 for mission 1360; 6503 and 6504 disabled for 567; 6511 to 6513 and 6528 pin the 1361 hand-off to return door 6007 and must stay enabled. The HA-09 and HA-10 per-mission ranges (6591 to 6690) are reserved to restoration. Authored IDs: unassigned. |
| Upstream gate | HA-09/10 and service UAT; the minigame result contract (H05: the ticket mint still supplies constant competency 1 and abilities 0). FACT. |
| Evidence class | Gates and item gaps: FACT. Premise ordering: INFERENCE. Everything else open. |
| Open decisions (Lomiada brief wording) | "Premise, reward and reset policy", "Sign-off of the nine mission graphs" (as card 1); "Event-5 item behavior: scanner 4396, nanite tag gun 4690, tracking gun: Real ability, or placeholder." Brief caution: "Trace consumption first: pairing event 5 with `remove_item` risks a double consume." |
| What the signer attests to | As card 1 (a to g), plus: any minigame or service used has a proved result contract; any item consumed is consumed exactly once (no event-5 plus `remove_item` double consume); a wrong-faction player cannot advance the graph. |
| Sign-readiness | NOT READY: as card 1, plus HA-09/10 not delivered and the minigame competency gap open. |
| Mission-advisor signature | (blank) |
| ID allocation | (blank) |

### Card 4: HA-17b OP-CORE story 2

| Field | Content |
|---|---|
| Graph | Authored side story 2 of 2, OP-CORE. Same branch and commit as card 3. |
| Premise | The other half of "lab calibration and casualty aid". |
| Mission, steps, objectives, tasks | None; same shape as card 3. |
| Grant, recipient, reward, items, chains | As card 3. |
| Evidence class | As card 3. |
| Open decisions (Lomiada brief wording) | As card 3. |
| What the signer attests to | As card 3, plus: story 2's guards are disjoint from story 1's and from the 1361 hand-off chains 6511 to 6513 and 6528. |
| Sign-readiness | NOT READY, same reasons as card 3. |
| Mission-advisor signature | (blank) |
| ID allocation | (blank) |

## 5. Cards 5 and 6: HA-17c, two authored Goa'uld stories

### Card 5: HA-17c Goa'uld story 1

| Field | Content |
|---|---|
| Graph | Authored side story 1 of 2, Goa'uld. Sealed as `harset/ha-17c-goauld-stories`, `Harset HA-17c: author Goa'uld side stories`. |
| Premise | The contract names "political favor and informant verification around Anat/Petbe/Market without changing shared hub actors". Story assignment not recorded. INFERENCE (low): political favor first. |
| Mission, steps, objectives, tasks | None. Contract shape: two short arcs with a visible choice and consequence and a closing reward. Choice buttons are keyed by dialog screen, and a non-interact chain can show only monologue dialogs. |
| Grant and recipient | Not named. Anat is an existing tagged Command Center actor (ledger row H12 cites 222); Petbe is shared hub spawn 223, which must stay invariant. A private Storage Petbe clone is template 221, owned by HA-13b. Market (world 69) has zero spawns today. FACT: [ledger](../README.md), rows H12 and H45, and the [Lomiada brief](../lomiada-brief.md) section 3. |
| Reward | Modest XP/currency (contract). Values open. |
| Items needed | None named for the authored graphs. The gating packets (HA-13, HA-14) need: 2823 Petbe's Robes and 2825 Petbe's Bloody Robes (1245; GAP; 2823 on placeholder ability 597), 5703, 5704, 5706 Themes (1246; GAP), 4512 Death by Chocolate and 2714 Delicious Confection (1247 and 1353; GAP; the docs disagree on which is the treat), 5736 Lethander's Head (1247; GAP). Sourced already: 2819 Jaffa Disguise (chain 6102), 2820 Scarab Listening Device (6101), 2864 Scarab Listening Device Map (6110), all mission 742. FACT: [HA-15 audit section 3.3](HA-15-audit.md#33-required-items-and-their-sources). |
| Chains and IDs preserved | `harset_goauld_chains.sql`: 6101 to 6120 (mission 742), 6121 to 6127 (mission 1200). HA-13 and HA-14 per-mission ranges (6201 to 6275) and HA-11/12 ranges (6131 to 6200) are reserved to restoration. Authored IDs: unassigned. |
| Upstream gate | HA-13/14 client UAT. Both are BlockedDependency or BlockedEvidence on the items above. |
| Evidence class | Premise text, gating and item states: FACT. Story ordering: INFERENCE. Everything else open. |
| Open decisions (Lomiada brief wording) | "Premise, reward and reset policy" and "Sign-off of the nine mission graphs" (as card 1); "Mission 1353 treat: 4512 or 2714: 4512 'Death by Chocolate' (same as 1247), or generic 2714 'Delicious Confection'" with the follow-on "1353 treat: bought or granted: Vendor stock, or a chain grant at a Nerus-lab step"; "Agnos shards and themes scope (items 5760, 5763, 5766, 5703, 5704, 5706): Loot, grants, or defer Agnos." |
| What the signer attests to | As card 1 (a to g), plus: no shared Petbe, Anat or guard is mutated for other players (two-player Petbe invariant); any robes or theme item is granted before it is equipped or handed in; no item is sourced from a vendor that ignores `max_stack_size` while stack-limited; no invisible auto-completion stands in for a missing Agnos leg. |
| Sign-readiness | NOT READY: as card 1, plus the item-source decisions and HA-13/14 delivery. |
| Mission-advisor signature | (blank) |
| ID allocation | (blank) |

### Card 6: HA-17c Goa'uld story 2

| Field | Content |
|---|---|
| Graph | Authored side story 2 of 2, Goa'uld. Same branch and commit as card 5. |
| Premise | The other half of "political favor and informant verification". |
| Mission, steps, objectives, tasks | None; same shape as card 5. |
| Grant, recipient, reward, items, chains | As card 5. |
| Evidence class | As card 5. |
| Open decisions (Lomiada brief wording) | As card 5. |
| What the signer attests to | As card 5, plus: story 2's guards are disjoint from story 1's and from mission 742 and 1200 chains. |
| Sign-readiness | NOT READY, same reasons as card 5. |
| Mission-advisor signature | (blank) |
| ID allocation | (blank) |

## 6. Cards 7 to 9: HA-18a, b, c, the three repeatables

The contract (parent HA-18) is one packet per faction "if a persistence/cooldown arm is needed". The repeatables are named in the contract as "watch, calibration and intelligence variants". INFERENCE (medium): watch is Jaffa, calibration is OP-CORE, intelligence is Goa'uld, from the order and the faction premises above; the contract does not label them by faction.

### Shared repeatable facts (cards 7, 8, 9)

| Topic | Fact |
|---|---|
| Persistence gap | FACT: the `repeats` field is missing from the world-entry mission UPSERT, so a repeat count silently fails to persist. A repeatable cannot be proved until that arm exists, which is why [HA-18](../work-packets.md#packet-status-reset) sits behind a separate architect-approved cooldown packet. |
| Contract | Unique mission IDs per variant; per-character state; reward once per cycle; visible cooldown and turn-in feedback. "Never reset a historical mission to make it repeatable." Reset period, daily cap, anti-farm rule and maximum reward are listed as "Owner decision required". |
| Policy | DECISION (O14-13): persisted per-character cooldowns, reward limits and anti-farm guards are delegated to the coordinator with economy and authority review. No period, cap or reward value is recorded. |
| Required proof | Two cycles across a clock boundary with a controlled clock, duplicate turn-in, abandon and relog, two characters independent, restart persistence, client journal and feedback. A reverted cooldown arm must fail a test. |
| Items | None named. |
| Chains and IDs | Unassigned. No candidate range addresses HA-18. |

### Card 7: HA-18a Jaffa repeatable

| Field | Content |
|---|---|
| Graph | Repeatable loop, Jaffa (INFERENCE: the watch variant). Sealed as `harset/ha-18-repeatables` (one packet per faction if a cooldown arm is needed). |
| Mission, steps, objectives, tasks | None. |
| Grant, recipient, reward | Not named; reuses "proved hub interactions" (contract). Supplies and modest XP/currency; values open. |
| Upstream gate | HA-17a first, plus the shared facts above. |
| Evidence class | Gating and the persistence gap: FACT. Faction mapping: INFERENCE. Cooldown values: open. |
| Open decisions (Lomiada brief wording) | "Premise, reward and reset policy: Concrete values for six authored stories and three repeatables." "Sign-off of the nine mission graphs." Brief note: "Cellblock's first authored quest must pass a client proof first." |
| What the signer attests to | The attestation list of card 1, plus: the reset period, daily cap, anti-farm rule and maximum reward are recorded as values (not left as a policy name); the cooldown is persisted per character and survives relog and restart; a reward is paid once per cycle; the persistence arm for `repeats` is in place and architect-approved; no historical mission is reset. |
| Sign-readiness | NOT READY: also needs HA-17a, the cooldown values and the `repeats` persistence arm. |
| Mission-advisor signature | (blank) |
| ID allocation | (blank) |

### Card 8: HA-18b OP-CORE repeatable

| Field | Content |
|---|---|
| Graph | Repeatable loop, OP-CORE (INFERENCE: the calibration variant). Same branch as card 7. |
| Mission, steps, objectives, tasks | None. |
| Grant, recipient, reward | Not named. Any minigame or service must have a proved result contract (H05 gap). |
| Upstream gate | HA-17b first, plus the shared facts above. |
| Evidence class, open decisions | As card 7. |
| What the signer attests to | As card 7, plus the minigame or service result contract and the stack-limit rule for any repeated supply grant. |
| Sign-readiness | NOT READY: also needs HA-17b and the H05 competency decision. |
| Mission-advisor signature | (blank) |
| ID allocation | (blank) |

### Card 9: HA-18c Goa'uld repeatable

| Field | Content |
|---|---|
| Graph | Repeatable loop, Goa'uld (INFERENCE: the intelligence variant). Same branch as card 7. |
| Mission, steps, objectives, tasks | None. |
| Grant, recipient, reward | Not named. Shared hub actors stay unchanged. |
| Upstream gate | HA-17c first, plus the shared facts above. |
| Evidence class, open decisions | As card 7. |
| What the signer attests to | As card 7, plus the two-player invariant on Petbe and Anat under repeated play. |
| Sign-readiness | NOT READY: also needs HA-17c. |
| Mission-advisor signature | (blank) |
| ID allocation | (blank) |

## 7. Card S: coordinator shared claims

| Field | Content |
|---|---|
| Row | Shared claims signoff (the "S" row of [HA-00c section 8](HA-00c.md#8-result-mission-graph-signoff-list)). Signer: coordinator, per the parent contract exit ("coordinator signs the shared claims and mission advisor signs all nine exact graphs"). |
| What exists | The merged claim table in [HA-00c section 4.2](HA-00c.md#42-merged-claim-table-authored-and-repeatable-mission-family): 73 reviewed Harset missions, four carry-ins (567, 1360, 1377, 1410), the 1664 interface, the 61000 to 61003 test reservation, the Cellblock 62xxx claims, the Harset chain candidates (6008 to 6032, 6131 to 6710 per-mission ranges, 6151 to 6165, 6531 to 6550). No row was added. |
| Frozen audit | 109 registered refs resolved to SHAs before scanning; 959 files parsed; zero duplicate keys; zero added keys hitting a stock pak ID or a claim; the existing validator passes (495 chain candidates). FACT: [HA-00c sections 5 and 6](HA-00c.md#5-result-frozen-shas-and-branch-collision-scan). |
| Not covered by the audit | Uncommitted work in sibling worktrees, claims written after the SHA freeze, unregistered branches, live database state. FACT: [HA-00c section 5.3](HA-00c.md#53-not-covered). |
| Seed-only occupants | The ten seed-only IDs in section 2 must be checked against the seed as well as the pak before any pick. |
| Items and dialogs | Seed-only dialogs above 6427 (13 rows: 3996, 60100 to 60104, 60110, 100006 to 100011) and seed-only items 9000 to 9014 are real occupants. `dialog_set_maps` has no pak; its tip maximum is 1000000. FACT: [HA-00c section 3.2](HA-00c.md#32-stock-census-pak-against-the-seeds-at-the-tip). |
| Open decisions | "Sign-off of the nine mission graphs" (Lomiada brief, with the shared row). Reconcile uncommitted sibling claims before any pick. |
| What the signer attests to | (a) The shared table matches section 4.2 of the HA-00c result and the four-zone ID table at the recorded SHA. (b) The uncovered sources above were reconciled at allocation time, not at audit time. (c) No ID in any graph overlaps another zone's claim, a census ID, a carry-in, 1664, a seed-only occupant or the 61000 to 61003 and 62xxx claims. (d) The scratch duplicate-key check fails on a planted duplicate and passes after restoration. |
| Sign-readiness | NOT READY: nothing to sign until at least one graph asks for an ID, and the reconciliation in (b) happens at that time. |
| Coordinator signature | (blank) |
| ID allocation | (blank) |

## 8. Context: seeded interfaces that border the nine graphs (not signable here)

These are not among the nine. They are the existing or proposed graphs that the research reports resolved, and the cards above depend on them for upstream gates. Nothing here is signed or allocated. Line references are from the mission and items research reports (read-only, 2026-10-18), re-checked where marked.

| Interface | Steps, objectives, tasks | Grant and recipient as known | Items and chains preserved | Class | Open decision |
|---|---|---|---|---|---|
| Mission 1664 (Castle exit, Harset debrief) | Steps 5224 (index 0), 5225 (1), 5226 (2); all `step_enabled` false in `mission_steps.sql`. Objectives 6063 (blank), 6059 (optional path), 6060 (mandatory door), 6061 (Marsh), 6062 (Moh'katan). Exists in pak and seed. | Grant owned by Castle chain 1398 (`mission_completed 708`, `accept_mission 1664`), RECONSTRUCTION, not a seed row. Recipient: Sgt Kaerlia on Harset (Texts moniker 27315); no template, spawn or dialog binding exists. | No item. Harset candidates (inactive): chains 6008 to 6020, spawn 360, template 260. | Grant, recipient name, step ids: FACT. Dialog 6169 (DSM 7016, speaker 3219) as the debrief: INFERENCE (medium). | "Kaerlia identity and look"; "Does catch-up chain 1399 ship?"; "Door affordance for 1664 step 5225". |
| Missions 618 and 621 | 618 step 2100 ("Speak with the leadership in the Command Center", objective 2438 blank); 621 step 4103, objective 4734. | Recipient unnamed. Tollana owns both graphs (chains 9800 to 9999). Candidates: Moh'katan (template 54, spawn 343) for 618; Baal (template 42, spawn 340) for 621. | Signed tags: `CmdCenter_Mohkatan`, `CmdCenter_Baal`, `CmdCenter_Marsh`, others per the [ledger](../README.md#tollana-recipient-tags). | Dialog 2272 and 4405 openings: FACT. Who is the recipient: INFERENCE (medium for 618, medium to low for 621). | "Recipients for 618 and 621". |
| Missions 1325 and 1326 | 1325 and 1326 exist in pak and seed (`missions.sql` lines 259 and 261). 1325 has no chain graph. | Mala'c is template 200 (level 15, faction 10 hostile); needs a neutral aggression override and a 30 percent health trigger. | Chains 6311 to 6330 empty and reserved to 1325; 1326 offers 6331, 6333 to 6342 gate on `mission_status 1325 eq completed`. | FACT. "Missing 1325" means unauthored, not absent. | "Authoring 1325 and the Mala'c duel". |
| Mission 567 and item 2698 | Steps 2000 (retrieve), 2012 (leave Castle), 4039 (deliver); all `step_enabled` false in `mission_steps.sql`. | Item 2698 "Romney's Files from his Castle Office"; no seed grants it. Recipient Copplemann (spawn 345, template 48). Castle CS-08 owns the route. | Chains 6503 and 6504 disabled; do not flip them because an item row exists. | Item identity and no grant: FACT. Castle-side source: INFERENCE (medium). | "567 item 2698 Castle route". |
| Mission 1360 | Steps 4037 and 4038. | Grant: Cellblock chain 1003 (loot table 1, item 3730). Crossing: Castle chain 1366 (single ID approved). Recipient Marsh (spawn 344, template 10). | Receipt chains 6501, 6502, 6505; DSM 120101. One-shot. | FACT through the recorded packets; no client acceptance. | "Client acceptance of the 1360 chain". |

## 9. How to use these cards

1. The mission advisor does not sign any card now. Each card's sign-readiness line names what has to exist first.
2. When a graph is proposed, fill its card: list every ID, run the collision audit again at that SHA, record the premise and reward values, then sign. The ID allocation cell is filled by the coordinator only after the shared audit, under O14-04.
3. A rejection is a valid outcome. The Lomiada brief asks for "Sign or reject each graph", and a rejected card is recorded in the cell with a reason, not left blank.
4. If the first authored client proof (Cellblock) fails, all nine cards stay NOT READY regardless of anything else on them.
