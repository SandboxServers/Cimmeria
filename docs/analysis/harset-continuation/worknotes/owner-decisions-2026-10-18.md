# Harset owner decisions — 2026-10-18

> Type: reference (decision register). Audience: Harset coordinator, packet authors, reviewers, Lomiada and the owner. Recorded: 2026-10-18 against campaign tip `59d5d6f0c` (branch `content/harset-campaign`). Docs only: no ID is allocated, no seed or code is changed.
> Companions: [ledger](../README.md), [packet contracts](../work-packets.md), [Lomiada brief](../lomiada-brief.md), [owner decisions 2026-10-14](../../four-zone-planning/owner-decisions-2026-10-14.md), [HA-00c sign-off cards](HA-00c-signoff-cards.md), [HA-15 audit](HA-15-audit.md), [HA-01a BSP door probe](HA-01a-bsp-door-probe.md), [HA-02 offline revalidation](HA-02-offline-revalidation.md).

The owner answered the open rows of the [Lomiada brief](../lomiada-brief.md#3-open-decisions-for-lomiada-or-the-owner) on 2026-10-18. This register records those answers as **O18-01 to O18-13**. No `O18-` ID existed before; the ledger's other series stop at HA-D11, D-H20 and O14-22, so the new IDs collide with nothing.

A decision authorizes work. It does not say the work exists, passed review or passed UAT. Where the owner said "provisional", the work stays provisional: it is labelled as such in the seed comments, the ledger and every worknote until its source evidence and in-game behaviour are checked (O18-13).

## Summary

| ID | Topic | Decision in one line | Main packet effect |
|---|---|---|---|
| O18-01 | Kaerlia, mission 1664 | New provisional Harset NPC | HA-01b identity gate replaced by a design packet (HA-01b2) |
| O18-02 | Catch-up chain 1399 | Omit for now | HA-01b and Castle CS-06 ship without 1399 |
| O18-03 | Mission items | Real, collectible items; no invisible credit | New design packet HA-15d; binds every mission packet |
| O18-04 | Vendors and trainers | Omega Site is the population and service-layout reference | HA-15b/c become research-first (HA-15f, then HA-15g) |
| O18-05 | Trainers, premises, rewards, resets | AI-derived provisional drafts with recorded reasoning | HA-15c, HA-17, HA-18 drafts may be written; still drafts |
| O18-06 | Agnos | Deferred with its related items | HA-06c and the Agnos-item parts of HA-13c deferred |
| O18-07 | Event-5 items | Real functions where source supports; uncertain stays provisional | New audit packet HA-15e |
| O18-08 | Missions 618 and 621 | Current recipient mapping kept as provisional | Tollana may build against it, not seal it |
| O18-09 | Item 2698, mission 567 | Collectible world item; chains stay disabled until the Castle route is verified | 6503/6504 stay disabled; Castle CS-08 owns the route |
| O18-10 | Mission 1325, Mala'c duel | Provisional AI-authored mission and duel design | New design packet HA-03a; 1326 still waits on 1325 |
| O18-11 | Nine graph sign-offs | HA-17a story 1 is the pilot; no IDs before graph and collision checks | New design card packet HA-17a1; the other eight wait on the pilot's client check |
| O18-12 | 1664 step 5225 door | Keep the right-click door; 6006 only if proven equivalent | New feasibility packet HA-01b1 |
| O18-13 | Labelling | Provisional designs and inferred placements stay labelled until checked | Applies to every row above |

## Decisions

The "Owner decision" text in each section is the owner's own wording. "Effect" and "Still undecided" are this register's reading of it; where that reading interprets a phrase, the section says so.

### O18-01 — Kaerlia, mission 1664

**Owner decision:** "Create a new Harset NPC for now. Choose a sensible provisional gender, appearance, clothing and placement from the available assets and context. Keep those choices clearly marked as provisional so they can be revised when better source information is found."

**Effect:** HA-01b's "Kaerlia identity" gate is answered provisionally. The design packet **HA-01b2** proposes her template, look and gate-side spot from existing assets, all marked provisional. The template 260, spawn 360 and chain 6008–6020 candidates in [id-allocation.md](../id-allocation.md) stay candidates. They are allocated only when HA-01b's graph is sealed and its collision check is recorded (O14-04, HA-D09).

**Still undecided:** her final identity (this choice is revisable); whether dialog 6169 / DSM 7016 is her debrief (an inference, medium); Castle CS-06's exact grant edge for 1664 (Castle owns it).

### O18-02 — catch-up chain 1399

**Owner decision:** "Omit it for now. Its regrant behavior after abandonment needs a separate justification and anti-repeat handling before it should ship."

**Effect:** Neither HA-01b nor Castle CS-06 may ship 1399 or any other re-grant of 1664 after abandonment. HA-01b's tests must not assume a re-grant.

**Still undecided:** what a player who abandons 1664 gets instead. Without 1399 they cannot get it again. That gap is recorded, not solved; a fix needs its own justification and anti-repeat design, then an owner decision.

### O18-03 — mission items

**Owner decision:** "Mission items must be real, collectible items in the mission inventory. Do not replace them with invisible completion or automatic objective credit."

**Effect:** Every mission packet that needs an item gives it to the player as a visible, collectible item in the mission inventory. The item may come from a pickup, a kill, an NPC hand-over or a chain grant; existing precedent is chains 6101 (2820), 6102 (2819) and 6110 (2864). Completion then checks or consumes that item. The design packet **HA-15d** classifies the 19 source-less items from the [HA-15 audit](HA-15-audit.md) and records the grant pattern and its hazards (double consume, stack limits, grant before equip). HA-15d does not cover the Agnos items (O18-06), item 2698 (O18-09) or the 4512/2714 treat choice.

**Still undecided:** which item is the 1353 treat, 4512 "Death by Chocolate" or 2714 "Delicious Confection". This decision makes the treat a real item but does not pick which one. HA-15a stays BlockedDecision.

### O18-04 — Harset vendors and trainers

**Owner decision:** "Use the Omega Site map as the population and service-layout reference. Derive a sensible Harset distribution from the available source data, creating NPCs where needed. Keep vendor identity and stock coherent with the setting and factions—for example, Jaffa merchants for Jaffa-facing goods, while considering whether a Jaffa or another appropriate NPC should serve Goa’uld-facing trade. Record the reasoning so it can be reviewed in game."

**Effect:** HA-15b and HA-15c lose their "owner decides" gate and become research-first. The research packet **HA-15f** extracts Omega Site's population, vendor and trainer layout (worlds 18 and 80 in the seed) and proposes a Harset distribution, recording its reasoning. The trainer design packet **HA-15g** follows HA-15f. Implementation still waits on safe placements (HA-16/HA-01a evidence), engine hazards (vendors ignore `max_stack_size` and have no faction gate) and allocated IDs.

**Still undecided:** exact prices and the economy envelope. O14-12 already bounds them (tier-comparable stock, no new currency, no high-tier gear), but no values are recorded. HA-15f proposes values for review; it does not set them. Which NPC serves Goa'uld-facing trade is left to the research, as the owner asked.

### O18-05 — trainer design, story premises, rewards and resets

**Owner decision:** "Have the AI derive provisional MMO-suitable choices from the available source data and existing game systems. Document the evidence and reasoning. These are working drafts for later in-game review and manual adjustment, not final historical claims."

**Effect:** Advisors may draft trainer trees (HA-15g), story premises and rewards (HA-17), and repeatable reset rules (HA-18) without another owner vote, inside O14-10 to O14-13. Each draft records its evidence and reasoning and is labelled a draft (`CIMMERIA_AUTHORING` or `CIMMERIA_RECONSTRUCTION`), never a recovered 2009 fact. The trainer seed is generator-owned: a trainer draft lands through `tools/ability_trees/generate_seed.py` and its workbook, not by hand-editing `trainer_abilities`.

**Still undecided:** the final values. Every draft stays open for in-game review and manual adjustment.

### O18-06 — Agnos

**Owner decision:** "Defer Agnos and its related items until the world and its content are in scope."

**Effect:** HA-06c (1353) is deferred: its Agnos handoff step has no source, and O18-03 forbids completing it invisibly. The Agnos items listed in the brief (5760, 5763, 5766, 5703, 5704, 5706) are out of HA-15d. HA-13c (1246) needs themes 5703, 5704 and 5706, so its theme leg is deferred too.

**Still undecided:** whether the 1246 themes really come from Agnos. Research reads it from Lethander dialog 4311 (inference, medium to low). Pulling them back into scope needs owner direction; this register does not do so.

### O18-07 — event-5 items

**Owner decision:** "Implement real item functions where the source data supports them. Inspect the existing ability and item data first, and use AI-assisted source analysis where useful. Keep uncertain bindings marked as provisional."

**Effect:** The audit packet **HA-15e** checks each event-5 binding (2743→2410, 2818→2068, 4396→2092, 4690→2409) and the ability-597 placeholders on 2819, 2823 and 2864. For each it records whether the source supports a real function, and traces consumption so that event 5 plus a chain `remove_item` cannot consume twice. HA-07, HA-08a, HA-09 and HA-11 consume its result. Bindings it cannot prove stay provisional.

**Still undecided:** each binding's final function, item by item, until HA-15e reports.

### O18-08 — missions 618 and 621

**Owner decision:** "Keep the current recipient mapping as provisional. Do not treat it as sealed until verified in the client."

**Effect:** Tollana owns both graphs (chains 9800–9999). This register reads "the current recipient mapping" as the one the brief presented: 618 to Moh'katan (`CmdCenter_Mohkatan`, spawn 343 / template 54, dialog 2272 opens with him) and 621 to Baal (`CmdCenter_Baal`, spawn 340 / template 42, dialog 4405 opens with him). Tollana may build its recipient chains against that mapping, labelled provisional. No Harset packet changes. The coordinator relays this to Tollana; the shared four-zone contract is not edited here.

**Still undecided:** the sealed recipients. They need a client check; until then, README HA-D06's "recipient BlockedEvidence" reads "provisional, client verification pending". If Tollana's ledger holds a different "current mapping", the coordinator reconciles before Tollana seals.

### O18-09 — item 2698 and mission 567

**Owner decision:** "Make item 2698 a collectible world item, rather than silently granting it as mission credit. The exact Castle pickup location and acquisition route still need source verification; keep the dependent Harset chains disabled until that route is established."

**Effect:** Chains 6503 and 6504 stay disabled. Castle CS-08 owns the pickup; Harset authors nothing for it. Once Castle proves the route, the Harset receiving pair is enabled under the existing closure contract (owned-item consumption, duplicate refusal).

**Still undecided:** the Castle pickup location and route (Castle, source verification).

### O18-10 — mission 1325 and the Mala'c duel

**Owner decision:** "Prepare a provisional AI-authored mission and Mala’c duel design, including health-trigger, failure/death handling and effect cleanup. Keep it open for later in-game review and manual editing. Mission 1326 remains dependent on 1325."

**Effect:** The design packet **HA-03a** writes the provisional 1325 graph and duel design: the 30% health trigger, surrender, death and failure handling, relog and abandon reset, and cleanup of every effect it applies. It also names the engine seams the design needs. **HA-03b** seals the missing `remove_effect` executor arm as a Haiku contract, and **HA-03c** implements it. The seed graph (HA-03d) still waits on Market travel, a duel pin and ID allocation. 1326's offers (6331, 6333, 6341) stay gated on 1325 completion.

**Still undecided:** the final duel tuning (open for in-game review); the Market duel position (HA-01a/HA-02 evidence).

### O18-11 — the nine graph sign-offs

**Owner decision:** "Start with HA-17a’s first Jaffa story as the pilot graph. The HA-00c report says the first authored quest needs client catalog proof before further authored work. After that pilot passes the owner-run client check, continue with the remaining five stories and three repeatables. Do not allocate IDs until the required graph and collision checks are complete."

**Effect:** HA-17a is split into the pilot story (HA-17a1 design card, HA-17a2 implementation) and the second Jaffa story (HA-17a3). HA-17a1, the pilot's graph design and filled sign-off card 1, may start now. It allocates no ID. The pilot's IDs are allocated only after its graph sign-off and a fresh collision check. HA-17a3, HA-17b, HA-17c and HA-18a/b/c stay blocked until the pilot passes the owner-run client check, and then still need their own graph and collision checks.

**Still undecided:** two older gates the answer does not lift explicitly, so they stay in force:

1. HA-17a's parent gate "restored Jaffa arc client UAT" (which includes 1325).
2. HA-00c's "Cellblock's first authored quest client proof".

Whether the Harset pilot itself may serve as the first authored client proof is an owner question; see the [open list](#what-stays-open).

### O18-12 — 1664 step 5225 door

**Owner decision:** "Preserve the source-described right-click door interaction. The existing 6006 enter-region trigger does not match that affordance, so implement or adapt a right-click interaction and verify its visible client behavior. Reuse 6006 only if it can be shown to provide the required interaction without changing the mission step’s behavior."

**Effect:** The feasibility packet **HA-01b1** finds a server-authoritative right-click affordance at the Command Center door, for example a server-spawned interactable entity whose interaction credits 6059/6060. It shows the visible feedback on the first press and gives a yes or no on 6006 reuse. If the only working option needs a client change (new UI, new opcode or a map patch), HA-01b1 stops and records a maintainer decision; it is not built.

**Still undecided:** the mechanism (HA-01b1 result); a maintainer decision if a client change turns out to be needed; the client check of the visible behaviour.

### O18-13 — labelling

**Owner decision:** "These are owner-directed working decisions. Provisional designs and inferred placements must remain labeled as such until their source evidence and in-game behavior are checked."

**Effect:** Every packet built on O18-01 to O18-12 labels its provisional parts in seed comments, worknotes and the ledger. A reviewer treats a missing label as a finding.

## What stays open

These are not decided by the 2026-10-18 answers and stay open.

| Open item | Kind | Blocks |
|---|---|---|
| 1353 treat: 4512 or 2714, and bought or granted | Owner decision | HA-15a, HA-06c (also deferred), HA-14 |
| Price and economy values inside O14-12 | Owner review of HA-15f's proposal | HA-15b |
| Does the Harset pilot wait for the restored Jaffa arc UAT and for Cellblock's first authored client proof, or may it be the first proof itself? | Owner decision | HA-17a2 |
| Abandon path for 1664 without 1399 | Owner decision after a design | HA-01b |
| Whether the 1246 themes are Agnos items | Evidence, then owner | HA-13c |
| Right-click door if it needs a client change | Maintainer decision (only if HA-01b1 finds no server-only path) | HA-01b |
| Market exit arch, 1006 or 1551 | One client glance (owner-manual) | HA-01a |
| World-57 exterior door positions | Hand reading or exterior probe | HA-01a |
| E03 move or drop, Market landing box shrink, Storage region raise for E13–E15 | Coordinator call under O14-12 after HA-01a1 evidence | HA-01a, HA-08, HA-11 |
| Castle 2698 pickup route | Castle CS-08 source verification | 567 |
| 618 and 621 sealed recipients | Client check | Tollana T-05/T-07/T-09 |
