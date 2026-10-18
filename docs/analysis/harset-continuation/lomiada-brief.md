# Harset decision brief for Lomiada

> Type: reference (decision register). Audience: Lomiada and the owner. Updated: 2026-10-18.
> Companions: [ledger](README.md), [wave resume](RESUME.md), [packet gates](work-packets.md), [placements](placements.md), [ID proposals](id-allocation.md), [owner decisions 2026-10-14](../four-zone-planning/owner-decisions-2026-10-14.md), [HA-00c result](worknotes/HA-00c.md), [HA-15 audit](worknotes/HA-15-audit.md), [HA-1360b seal](worknotes/HA-1360b-seal.md), [old Harset rebuild decisions](../harset-rebuild/README.md).

This page lists what the Harset campaign has already decided, what it is waiting on you or the owner to decide, and what only someone standing in the world can settle. Nothing here claims that any Harset content works in game. No Harset client UAT is recorded.

How to read the labels. **Fact** means a file or seed row says it. **Inference** means the research reasoned from text, adjacency or stock BigWorld habits. Confidence is high, medium or low. The research behind the open items comes from three read-only reports (missions, items, world) written 2026-10-18; they sit in the campaign's ignored `.claude/reports/harset-continuation/` folder, so they are not linked. One caveat applies to all world claims: the world research did not re-open the spawnlist seed, so re-check spawn and chain facts before a packet relies on them.

## 1. Where the campaign stands

- The HA-1360 letter chain (a1, a2a, a2b, a3a, a3b, a3c, b1, b2) and the a2c post-handoff fence are integrated and reviewed on the local `content/harset-campaign` branch, with lane results recorded in the [resume](RESUME.md).
- The HA-00c static audit (stock catalog, 109 refs, zero collisions) and the HA-15 vendor, trainer and mission-item census are integrated on that branch. Neither allocated an ID or added content.
- Five complete Harset mission graphs exist in the seed (742, 1200, 1324, 1326, 1361). 1326 cannot start until 1325 is authored.
- No Harset client UAT, lab result or crossing proof is recorded. All UAT is owner-manual; the lab is not in use.
- There is no remote PR and nothing is pushed. The campaign gets one PR when it closes.

## 2. Settled decisions

These are recorded in the ledger or the owner register. A decision authorizes work; it does not mean the work passed UAT.

| ID | Decision | Source |
|---|---|---|
| O14-03 | A coordinator may approve a bounded `CIMMERIA_RECONSTRUCTION` for missing 2009 wiring after domain signoff. | [owner register](../four-zone-planning/owner-decisions-2026-10-14.md) |
| O14-04 | The coordinator may allocate individual IDs after a stock, shared-table, seed and active-branch collision audit. | [owner register](../four-zone-planning/owner-decisions-2026-10-14.md), [HA-D09](README.md#decisions) |
| O14-07 | A campaign may close with historical rows on an explicit evidence hold. | [owner register](../four-zone-planning/owner-decisions-2026-10-14.md) |
| O14-09 | If the lab stays unavailable, a fully reviewed PR may merge with named rows marked `UATPending`. A broken observed step still blocks. | [owner register](../four-zone-planning/owner-decisions-2026-10-14.md) |
| O14-10, O14-11 | Lore-safe authored premises, copy and modest rewards are delegated (about 5 to 8 stories per large hub). No player-facing provenance label. The first authored quest needs a live-client catalog proof before further authored work. | [owner register](../four-zone-planning/owner-decisions-2026-10-14.md) |
| O14-12 | Conservative ambient density, safe placements, tier-comparable vendor and trainer stock and reward ceilings are delegated. No new currency, no high-tier gear. | [owner register](../four-zone-planning/owner-decisions-2026-10-14.md) |
| O14-13 | Persisted per-character repeatable cooldowns, reward limits and anti-farm guards are delegated, with economy and authority review. | [owner register](../four-zone-planning/owner-decisions-2026-10-14.md) |
| O14-16, O14-18 | The Castle to Harset stargate crossing advances 1360 from step 4037 to 4038 and keeps letter 3730. Chain 1366 is approved as the single Castle bridge ID. | [owner register](../four-zone-planning/owner-decisions-2026-10-14.md), [HA-D03](README.md#decisions) |
| O14-17, O14-19 | Marsh's hand-in must require the letter, as one safe transaction. Mission 1360 is one-shot. | [owner register](../four-zone-planning/owner-decisions-2026-10-14.md), [HA-D11](README.md#decisions) |
| HA-D01 | Active set is 29 ZIP-linked core rows plus four carry-ins (at most 33); 39 older revisions are held for build lineage. | [ledger](README.md#decisions) |
| HA-D02 | Worlds 57 and 68 are shared; Market 69 and Storage 70 are per player. `entities/spaces.xml` governs instancing. | [ledger](README.md#decisions) |
| HA-D03 | Castle owns the crossing (chain 1366); Harset owns the receiving legs. 567's acquisition leg stays with Castle CS-08. | [ledger](README.md#decisions) |
| HA-D04 | 567 stays disabled (chains 6503, 6504) until an acquisition path for item 2698 is proved. | [ledger](README.md#decisions) |
| HA-D05 | Castle accepts 1664 and Harset owns the debrief (steps 5224 to 5226). Provisional: the exact grant edge, Kaerlia template and dialog mapping are unproved. | [ledger](README.md#decisions), [1664 interface](work-packets.md#castle-1664-provisional-interface) |
| HA-D06 | Existing Harset recipient tags are signed off (spelling, world, template). Tollana owns both ends of its chains (9800 to 9999). | [ledger](README.md#tollana-recipient-tags) |
| HA-D07 | Preserve the enabled door chains 6007 and 6511 to 6513, 6528. Market and Storage doors need four doorway observations. | [ledger](README.md#decisions) |
| HA-D08 | The rebuilt meshes for all four worlds are accepted; the old "no interior mesh" and 1,939-component claims are stale. | [ledger](README.md#decisions) |
| HA-D10 | Bounded content policy: record provenance, economy reasoning, cooldown and anti-farm guards per packet. | [ledger](README.md#decisions) |
| D-H09, GH2 | Vendors and trainers are a separate design gate. Vendor and trainer Rust exists; the seed holds no Harset stock or prices. D-H09 was adopted as a working default (D-H19, 2026-09-17) and the stock policy was left to the owner. | [Harset rebuild](../harset-rebuild/README.md) |
| D-H14 | Duplicate item designs resolve to Scarab 2820, Straegis Scanner 4396, Tollan Control Technology 2743 and Goa'uld Symbiote 2818; Bloody Robes 2825 is an arbitrary pick. | [Harset rebuild](../harset-rebuild/README.md) |

Research findings the ledger does **not** record as decisions (evidence found, not yet a recorded decision):

| Finding | Kind | Confidence |
|---|---|---|
| Chain 1398 (Castle, trigger `mission_completed 708`, action `accept_mission 1664`) is the frozen 1664 grant in Castle's CS-06 seal. | Fact | High |
| The 1664 turn-in NPC is Sgt Kaerlia on Harset (Texts moniker 27315). | Fact | High |
| 1325 and 1326 are in the stock pak and seed; chains 6311 to 6330 for 1325 are empty. "Missing 1325" means unauthored. | Fact | High |
| 1326 offers are correctly gated on `mission_status 1325 eq completed`. | Fact | High |
| Item 2698 is "Romney's Files from his Castle Office"; no seed row grants it. | Fact | High |
| 4512 binds to mission 1247 (Patsy) by moniker convention; nothing in the repo sources 4512 or 2714. | Fact | High |
| No Harset NPC has vendor or trainer wiring; 19 of 23 mission-required items have no source ([HA-15 audit](worknotes/HA-15-audit.md)). | Fact | High |
| Vendor purchases ignore `max_stack_size` and the vendor engine has no faction gate. | Fact | High |
| Agnos is absent from the seed (no world, spawn, entity, loot or content rows). | Fact | Medium |

## 3. Open decisions for Lomiada or the owner

| Decision | Options | What the evidence says | Blocks |
|---|---|---|---|
| Kaerlia identity and look (1664 debrief) | Pick a template, costume and gate-side spot; reuse an existing template or author a new one | Fact: no Kaerlia template, spawn or dialog binding exists; speaker 3219 is a blank row. Inference (medium): dialog 6169 "Debriefing" (DSM 7016) is the debrief, from its text and adjacency only. Placement near the gate is feasible (medium), but identity and look are a design call. | HA-01b |
| Does catch-up chain 1399 (`player_loaded 'Castle'`) ship? | Ship (re-grants after an abandon) or omit | Fact: 1399 is a coordinator-pending reconstruction. Neither 1398 nor 1399 is a committed seed row. Castle's verdict on 1664 is BlockedDependency on HA-01b. | HA-01b, Castle CS-06 |
| Mission 1353 treat: 4512 or 2714 | 4512 "Death by Chocolate" (same as 1247), or generic 2714 "Delicious Confection" | Fact: 4512 is bound to 1247 by moniker. 1353 is undetermined (medium to low); the docs disagree, and the "4512" claim traces to an external workbook that is not in the repo. Inference: designers likely reused one treat. | HA-15a, HA-06c, HA-14 |
| 1353 treat: bought or granted | Vendor stock, or a chain grant at a Nerus-lab step | The rebuild docs name "GH2 vendor or Nerus-lab grant" as a design gate. Prefer a visible chain grant (grant before any equip) over a vendor for mission items. Source is inference, low. | HA-15a, HA-06c, HA-14 |
| Vendor stock and prices | Faction vendor lists per tier; which NPCs sell | Fact: worlds 69 and 70 have zero spawns, no Harset spawn carries vendor fields, the six service templates are debug content. O14-12 permits tier-comparable stock, but no values are recorded. Do not vendor stack-limited mission items until the engine honors `max_stack_size`. | HA-15b |
| Trainer NPCs and trainer tree | Which NPCs train, what they teach, costs | Fact: only debug trainer lists exist; `trainer_abilities` is generated by `tools/ability_trees/generate_seed.py` from a workbook, and the trainer is archetype and level gated only. | HA-15c |
| Agnos shards and themes scope (items 5760, 5763, 5766, 5703, 5704, 5706) | Loot, grants, or defer Agnos | Fact: Agnos has no content in the seed, so shards and themes have no home. Inference (medium to low): Lethander dialog 4311 hands out themes. No invisible auto-completion is allowed. | HA-06c, HA-13c |
| Event-5 item behavior: scanner 4396, nanite tag gun 4690, tracking gun | Real ability, or placeholder | Fact: event 5 binds 2743 (ability 2410), 2818 (2068), 4396 (2092), 4690 (2409); ability 597 on 2823, 2819 and 2864 is a placeholder. Legacy abilities 2092 and 2409 exist (medium to low). Trace consumption first: pairing event 5 with `remove_item` risks a double consume. | HA-09, HA-07, HA-08a |
| Recipients for 618 and 621 | Moh'katan for 618, Baal for 621, or another command-center NPC | Tollana owns both graphs (chains 9800 to 9999); Harset must not author a duplicate. Fact: turn-in dialog 2272 (618) opens with Moh'katan, then Marsh, Baal, Anat. Dialog 4405 (621) opens with Baal. Inference: medium for 618 (an ensemble scene), medium to low for 621 (the text is about the Activator Crystal, step 4103 asks for a gate address). Anat has no tag, so she cannot be a click target. | Tollana T-05, T-07 |
| 567 item 2698 Castle route | A Castle pickup (Romney's office), or a Castle mission reward | Fact: no loot, vendor or reward row names 2698. Inference (medium): the source is Castle-side. Castle CS-08 owns the route; 6503 and 6504 stay disabled until it is proved. | CS-08, 567 |
| Authoring 1325 and the Mala'c duel | Author chains in 6311 to 6330; duel shape | Fact: Mala'c is template 200 (level 15, faction 10 hostile). It needs a neutral aggression override and a 30% health trigger; `remove_effect` has no executor arm. Needs the HA-00c graph signoff. The old plan treated the duel as a scripted PvE ritual (D-H11, medium). | HA-03, then 1326 |
| Premise, reward and reset policy | Concrete values for six authored stories and three repeatables | O14-10, O14-11, O14-13 delegate the policy, but no premise, reward amount or reset rule is recorded. Cellblock's first authored quest must pass a client proof first. | HA-17a/b/c, HA-18a/b/c |
| Sign-off of the nine mission graphs | Sign or reject each graph | Fact: the list is in the [HA-00c result](worknotes/HA-00c.md); none of the nine graphs is authored, and every signature cell is blank. No IDs are allocated. | HA-00c, HA-17, HA-18 |
| Door affordance for 1664 step 5225 | Reuse the enter-region door 6006, or author a right-click door | Fact: 6006 is an enter-region door; reusing it changes the authored affordance and needs reconstruction review. | HA-01b |

## 4. Needs someone in the world or the client

All UAT is owner-manual. The lab is not in use and no lab lease is requested here.

| What to observe | Why | Blocks |
|---|---|---|
| Four doorway readings (Market and Storage, each way) plus both landings, with `.location` | Fact: no authored door triggers exist for the two interiors. Both are mostly BSP, so the opening is a brush face the extractor does not read. Two-player isolation check for instanced worlds. | HA-01a (HA-D07), HA-05, HA-08 |
| Raised-platform reading, and spawn 308 | Fact: five clustered telemetry points (8 to 10 m above decoded geometry) and spawn 308 (about 3.4 m gap) are unexplained. Inference (medium): an undecoded raised platform. | HA-02 |
| Ordinary-player containment walk in worlds 57, 68, 69, 70 | Fact: nobody has walked these meshes as an ordinary player. Interiors have no telemetry. All four worlds are `advisory`, which means pathing and line of sight only. | HA-02 |
| Anchor placements for HA-07, HA-11, HA-12 and HA-14 | Fact: no anchors exist (four for HA-07, six scarab anchors for HA-11, the private encounter for HA-12, the Bar and holding pens for HA-14). | HA-07, HA-11, HA-12, HA-14 |
| Confirm dialog 6169 is the Kaerlia debrief | Inference only; confirm in play. | HA-01b |
| Confirm the 618 and 621 recipients | Check which NPC's dialog actually shows the turn-in scene before a binding is relied on. | Tollana T-05, T-07 |
| Client acceptance of the 1360 chain | Fact: unit, cross-service and revert-probe evidence exists; no crossing or client acceptance is recorded. | HA-1360 closeout |

## 5. What agents can still do offline without a decision

- **Nav revalidation.** Re-run the existing nav inspection pipeline against the shipped meshes for the placement candidates (E01 to E15), the 6006 and 6007 pair and the proposed landing boxes. Compare the last waypoint, since Detour can return partial paths. This is the "Revalidation required" section of [placements](placements.md). Confidence: high that it is feasible.
- **Storage pen-floor route.** Re-measure whether the pen floor connects to the upper wing on the rebuilt Storage mesh (medium for the pen floor, low for the upper wing until re-measured).
- **Optional BSP door decode.** Prototype a decode of the Market and Storage interiors for door brushes. Medium effort and untested; it would only narrow the doorway readings, not replace them.

None of these changes content or allocates an ID. Documentation debt to flag: the older notes in agent memory that quote 1,939 components or "no interior mesh" are history; use `data/spaces/README.md` and the NA40 notes instead.
