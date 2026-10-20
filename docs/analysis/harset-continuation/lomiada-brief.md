# Harset decision brief for Lomiada

> Type: reference (decision register). Audience: Lomiada and the owner. Updated: 2026-10-18 (refreshed after the owner's answers).
> Companions: [ledger](README.md), [wave resume](RESUME.md), [packet gates](work-packets.md), [owner decisions 2026-10-18](worknotes/owner-decisions-2026-10-18.md), [owner decisions 2026-10-14](../four-zone-planning/owner-decisions-2026-10-14.md), [placements](placements.md), [ID proposals](id-allocation.md), [HA-00c result](worknotes/HA-00c.md), [HA-15 audit](worknotes/HA-15-audit.md), [HA-02 offline revalidation](worknotes/HA-02-offline-revalidation.md), [BSP door probe](worknotes/HA-01a-bsp-door-probe.md), [sign-off cards](worknotes/HA-00c-signoff-cards.md), [old Harset rebuild decisions](../harset-rebuild/README.md).

This page lists what the Harset campaign has already decided, what it is still waiting on you or the owner to decide, and what only someone standing in the world can settle. Nothing here claims that any Harset content works in game. No Harset client UAT is recorded.

How to read the labels. **Fact** means a file or seed row says it. **Inference** means the research reasoned from text, adjacency, geometry or stock BigWorld habits; confidence is high, medium or low. **Provisional** means an owner-directed working choice that stays open for in-game review (O18-13). The earlier research comes from three read-only reports (missions, items, world) written 2026-10-18; they sit in the campaign's ignored `.claude/reports/harset-continuation/` folder, so they are not linked.

## 1. Where the campaign stands

- The HA-1360 letter chain (a1, a2a, a2b, a3a, a3b, a3c, b1, b2) and the a2c post-handoff fence are integrated and reviewed on the local `content/harset-campaign` branch, with lane results recorded in the [resume](RESUME.md).
- The HA-00c static audit (109 refs, zero collisions), the HA-15 census, the HA-02 offline nav revalidation and the BSP door probe are integrated on that branch. None allocated an ID or added content.
- The owner answered thirteen questions on 2026-10-18 (O18-01 to O18-13). They unblock eight docs-only design and research packets ([waves R1 and R2](work-packets.md#research-waves-r1-and-r2-after-the-2026-10-18-decisions)); none has run yet.
- Five complete Harset mission graphs exist in the seed (742, 1200, 1324, 1326, 1361). 1326 cannot start until 1325 is authored.
- No Harset client UAT, lab result or crossing proof is recorded. All UAT is owner-manual; the lab is not in use. Nothing is pushed; the campaign gets one PR when it closes.

## 2. Settled decisions

These are recorded in the ledger or an owner register. A decision authorizes work; it does not mean the work passed UAT.

### Owner decisions of 2026-10-18

Exact owner wording and effects: [O18 register](worknotes/owner-decisions-2026-10-18.md).

| ID | Decision | What it unblocks |
|---|---|---|
| O18-01 | Kaerlia (1664) is a new Harset NPC. Gender, look, clothing and placement are chosen from existing assets and stay **provisional**. | HA-01b2 design card |
| O18-02 | Catch-up chain 1399 is omitted for now. | 1664 ships without it; the abandon path is still open |
| O18-03 | Mission items are real, collectible items in the mission inventory. No invisible completion or automatic credit. | HA-15d grant design; binds every mission packet |
| O18-04 | Omega Site is the population and service-layout reference for Harset vendors and trainers. Vendor identity follows faction (Jaffa merchants for Jaffa-facing goods); who serves Goa'uld-facing trade is reasoned and recorded. | HA-15f research, then HA-15b/c |
| O18-05 | Trainer trees, story premises, rewards and resets are AI-derived **provisional** drafts with recorded evidence, not historical claims. | HA-15g, HA-17, HA-18 drafts |
| O18-06 | Agnos and its items are deferred until that world is in scope. | HA-06c and the HA-13c theme leg are deferred |
| O18-07 | Event-5 items get real functions where source data supports them; uncertain bindings stay **provisional**. | HA-15e audit |
| O18-08 | 618 → Moh'katan and 621 → Baal stay **provisional** until verified in the client. | Tollana may build against it, not seal it |
| O18-09 | Item 2698 is a collectible world item. Its Castle pickup route still needs source verification; Harset chains 6503/6504 stay disabled until then. | Nothing in Harset yet; Castle CS-08 owns the route |
| O18-10 | 1325 and the Mala'c duel get a **provisional** AI-authored design (health trigger, failure and death, effect cleanup). 1326 still waits on 1325. | HA-03a design, then HA-03b/c `remove_effect` arm |
| O18-11 | HA-17a's first Jaffa story is the pilot graph. The other five stories and three repeatables wait until it passes the owner-run client check. No IDs before the graph and collision checks. | HA-17a1 design card |
| O18-12 | Step 5225 keeps its right-click door. Enter-region 6006 is reused only if shown to give the same interaction. | HA-01b1 feasibility |
| O18-13 | Provisional designs and inferred placements stay labelled until their evidence and in-game behaviour are checked. | Every row above |

### Earlier decisions

| ID | Decision | Source |
|---|---|---|
| O14-03 | A coordinator may approve a bounded `CIMMERIA_RECONSTRUCTION` for missing 2009 wiring after domain signoff. | [owner register](../four-zone-planning/owner-decisions-2026-10-14.md) |
| O14-04 | The coordinator may allocate individual IDs after a stock, shared-table, seed and active-branch collision audit. | [owner register](../four-zone-planning/owner-decisions-2026-10-14.md), [HA-D09](README.md#decisions) |
| O14-07, O14-09 | A campaign may close with historical rows on an explicit evidence hold; with the lab unavailable, a reviewed PR may merge with named rows `UATPending`. A broken observed step still blocks. | [owner register](../four-zone-planning/owner-decisions-2026-10-14.md) |
| O14-10, O14-11 | Lore-safe authored premises, copy and modest rewards are delegated; no player-facing provenance label. The first authored quest needs a live-client catalog proof. | [owner register](../four-zone-planning/owner-decisions-2026-10-14.md) |
| O14-12 | Conservative density, safe placements, tier-comparable vendor and trainer stock and reward ceilings are delegated. No new currency, no high-tier gear. | [owner register](../four-zone-planning/owner-decisions-2026-10-14.md) |
| O14-13 | Persisted per-character repeatable cooldowns, reward limits and anti-farm guards are delegated, with economy and authority review. | [owner register](../four-zone-planning/owner-decisions-2026-10-14.md) |
| O14-16 to O14-19 | The Castle → Harset crossing (chain 1366) advances 1360 from 4037 to 4038 and keeps letter 3730; Marsh's hand-in requires the letter as one transaction; 1360 is one-shot. | [owner register](../four-zone-planning/owner-decisions-2026-10-14.md), [HA-D03, HA-D11](README.md#decisions) |
| HA-D01, HA-D02 | Active set is 29 core rows plus four carry-ins (at most 33). Worlds 57 and 68 are shared; Market 69 and Storage 70 are per player. | [ledger](README.md#decisions) |
| HA-D04 to HA-D07 | 567 stays disabled; 1664 is provisional (now with O18-01/02/12); Harset recipient tags are signed off and Tollana owns its chains; enabled door chains 6007, 6511 to 6513 and 6528 are preserved. | [ledger](README.md#decisions) |
| HA-D08, HA-D10 | The rebuilt meshes are accepted; bounded content policy records provenance, economy, cooldown and anti-farm reasoning per packet. | [ledger](README.md#decisions) |
| D-H14 | Duplicate item designs resolve to Scarab 2820, Straegis Scanner 4396, Tollan Control Technology 2743 and Goa'uld Symbiote 2818; Bloody Robes 2825 is an arbitrary pick. | [Harset rebuild](../harset-rebuild/README.md) |

## 3. Open decisions for Lomiada or the owner

Only these remain after 2026-10-18 ([What stays open](worknotes/owner-decisions-2026-10-18.md#what-stays-open)).

| Decision | What the evidence says | Blocks |
|---|---|---|
| 1353 treat: 4512 "Death by Chocolate" or 2714 "Delicious Confection", and bought or granted | Fact: 4512 binds to 1247 by moniker; nothing in the repo sources either item. O18-03 makes the treat a real item either way. Inference (low): designers reused one treat; a visible chain grant beats a vendor for a mission item. | HA-15a, HA-14 (HA-06c is also deferred) |
| Price and economy values | O14-12 bounds them; HA-15f will propose values from Omega Site for review. It does not set them. | HA-15b |
| Does the Harset pilot wait for the restored Jaffa arc UAT and Cellblock's first authored client proof, or may it be that first proof? | O18-11 names the pilot but does not lift the older O14-10/11 gates explicitly, so they stay in force until answered. | HA-17a2 |
| 1664 abandon path without 1399 | Fact: 1399 (`player_loaded 'Castle'`) was the proposed regrant after an abandon; the owner wants a separate justification and anti-repeat handling first. Inference (medium): with it omitted, a player who abandons 1664 has no way back unless the design adds one. | HA-01b |
| Are the 1246 themes (5703, 5704, 5706) Agnos items? | Inference (medium to low): Lethander dialog 4311 hands them out. Deferred with Agnos unless you pull them back in. | HA-13c |
| Right-click door needs a client change? | Only if HA-01b1 finds no server-only mechanism. Then it is a **maintainer decision** (new UI, opcode or map patch). | HA-01b |

## 4. Needs someone in the world or the client

All UAT is owner-manual. The lab is not in use and no lab lease is requested here.

| What to observe | Why | Blocks |
|---|---|---|
| One glance: which Market arch is the exit, 1006 or 1551 | Fact: the BSP probe found both arches. Inference: 1006 at (97.28, 3.52, 60.32) is the likely exit (position high, role medium); 1551 at (19.92, 4.80, 1.28) has no known role. | HA-01a |
| The two world-57 exterior door positions (Market and Storage) | Fact: interior data cannot recover them. A hand reading with `.location`, or an exterior probe. | HA-01a |
| Spawn 308 and the five raised telemetry points | Fact: not re-measured in the offline pass. Inference (medium): an undecoded raised platform. | HA-02 |
| Ordinary-player containment walk in worlds 57, 68, 69, 70 | Fact: nobody has walked these meshes as an ordinary player. All four worlds are `advisory` (pathing and line of sight only). | HA-02 |
| Anchor placements for HA-07, HA-11, HA-12 and HA-14 | Fact: no anchors exist yet. | HA-07, HA-11, HA-12, HA-14 |
| Kaerlia, the 5225 door and dialog 6169 in play | Once HA-01b is built: the provisional NPC, the right-click feedback and whether 6169 reads as the debrief. | HA-01b |
| The 618 and 621 recipients in the client | O18-08: provisional until seen. | Tollana T-05, T-07 |
| Client acceptance of the 1360 chain | Fact: unit, cross-service and revert-probe evidence exists; no crossing or client acceptance is recorded. | HA-1360 closeout |
| The pilot story (HA-17a2) | O18-11: the owner-run client check gates the other eight graphs. | HA-17a3, HA-17b/c, HA-18 |

## 5. Offline findings since the first brief

The offline work the first brief proposed is done. None of it changes content or allocates an ID.

| Finding | Kind | Source |
|---|---|---|
| The BSP probe found interior door arches: Market arch 1006 (97.28, 3.52, 60.32, yaw -180), Market arch 1551 (19.92, 4.80, 1.28, yaw -90) and Storage arch 28 (61.36, 5.12, -2.60, yaw -90). The "the extractor does not read the BSP" premise is stale. | Fact (positions high) | [BSP door probe](worknotes/HA-01a-bsp-door-probe.md) |
| Landing estimates: Market ≈(89.5, 3.6, 60.3) and ≈(19.9, 4.8, 9.0); Storage ≈(61.4, 5.2, 5.2). Storage arch 28 is the exit (medium). | Inference (low to medium) | [BSP door probe](worknotes/HA-01a-bsp-door-probe.md) |
| The small Storage doorways are internal; the three Storage volumes are reverb triggers; the Market trigger volume is water. None is a door. | Fact | [BSP door probe](worknotes/HA-01a-bsp-door-probe.md) |
| Placement E03 (94.5, 3.65, 44.9) fails, with a 4.85 m gap to the mesh. E04, E06, E07 and E13 pass with a nudge; E08 is a marginal pass (clearance 1.20 m); E12 sits 0.3 m south of the box. | Fact | [HA-02 revalidation](worknotes/HA-02-offline-revalidation.md) |
| The Market landing box should shrink to about x[16,24], z[9,22]. | Inference (medium) | [HA-02 revalidation](worknotes/HA-02-offline-revalidation.md) |
| The Storage pen floor and upper wing are connected on the rebuilt mesh. Storage region 2122 (ceiling 4.00 plus a 1.5 m pad) excludes E13 to E15 at y ≈ 5.2, so it needs a raise or those points move. | Fact | [HA-02 revalidation](worknotes/HA-02-offline-revalidation.md) |
| Door 6006 landing passes; region 2079's southern corners are 2.1 to 2.9 m off the mesh. Door 6007 passes. | Fact | [HA-02 revalidation](worknotes/HA-02-offline-revalidation.md) |
| The HA-02 note labels world 69's mesh as `harset_cmdcenter.nav`; `worlds.sql` says 69 is `harset_market.nav` and 68 is `harset_cmdcenter.nav`. HA-01a1 settles which mesh was measured. | Fact (a labelling discrepancy) | [HA-02 revalidation](worknotes/HA-02-offline-revalidation.md) |
| All nine graph sign-off cards are blank and NOT READY: no graph is authored, no signature or ID exists. | Fact | [sign-off cards](worknotes/HA-00c-signoff-cards.md) |

The coordinator calls E03 (move or drop), the box shrink and the region raise under O14-12 after HA-01a1 reports.

Next offline work, all docs-only: HA-01b1 door feasibility, HA-01b2 Kaerlia design, HA-03a duel design, HA-15d item grants, HA-15e event-5 audit, HA-17a1 pilot design card, HA-01a1 landing geometry and HA-15f Omega Site reference. Briefs: [waves R1 and R2](work-packets.md#research-waves-r1-and-r2-after-the-2026-10-18-decisions).

Documentation debt to flag: older agent-memory notes that quote 1,939 components, "no interior mesh" or "BSP not read" are history; use `data/spaces/README.md`, the NA40 notes and the two worknotes above.
