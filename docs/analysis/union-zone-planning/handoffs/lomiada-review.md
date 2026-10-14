# Lomiada review handoff: Union starts through Beta E1

> **Purpose:** Give this file and the planning PR to Lomiada's GPT for an independent source and campaign review.
> **State:** Planning documents only. No Union implementation, new map package, live lab run or mission grant is part of this PR.
> **Date:** 2026-10-14

## What the PR contains

The PR plans four **independent zone restoration main sessions**:

1. [SGC_W1](../sgc.md): complete the Human attack and both Human/Free Jaffa SGC return legs.
2. [Dakara_E1](../dakara.md): continue the already-started Dakara rebuild, add the approved Free Jaffa level-1 bridge, and complete its shared Human/Jaffa crisis and SGC handoffs.
3. [Omega Site](../omega.md): make the first three-race SGU hub convergence playable; use a new post-Pertho Asgard prologue and remote Council hearing in existing map geometry.
4. [Beta Site E1](../beta.md): make the first shared field visit playable for all three races with separate Human, Free Jaffa and Asgard quest families.

The [Free Jaffa](../free-jaffa-route.md) and [Asgard](../asgard-route.md) briefs are cross-zone journey and acceptance contracts. The [post-Pertho design](../asgard-post-pertho.md) records the author's chosen creative departure from the unavailable original Pertho and High Council maps. Each zone has a future dispatch contract: [SGC](../sgc-work-packets.md), [Dakara](../dakara-work-packets.md), [Omega](../omega-work-packets.md) and [Beta](../beta-work-packets.md). [Evidence and chronology](../evidence.md) and the [execution contract](../execution-contract.md) state source precedence and handoff ownership. The [index](../README.md) is the decision ledger.

The owner chose **Human first for implementation**, but **all three SGU race paths are planned now** through arrival and first play at Beta E1. The new Free Jaffa opening scouts divided Jaffa loyalties; the new Asgard recovery protects rescued consciousness databanks and makes their custody central to a remote Council hearing. The four Union implementation campaigns wait until the separate Cellblock, Castle, Harset and Praxis Tollana continuations and their FZ-99 close-out are complete. Planning this PR does not dispatch them.

## The chronology the review should protect

| Route | Planned early path | Critical difference |
|---|---|---|
| SGU Human | SGC_W1 → Dakara_E1 → SGC_W1 → Omega → Beta E1 | Human attack and Hammond-to-Daniel letter; `The SGU` mission 1423. |
| Free Jaffa / Shol'va | Dakara_E1 level-1 authored bridge → Dakara crisis → SGC support → Dakara climax → SGC aftermath → Omega → Beta E1 | Current project start is Dakara; recovered quests are level 3–5 and imply earlier SGC events. Bra'tac letter and `The SGU` mission 1500 are Jaffa-specific. |
| Asgard | New post-Pertho private Omega prologue → remote Council ruling → public Omega → Beta E1 | Pertho worlds 83–85 and physical Council world 59 have no named production map in the audited handoff. Do not auto-complete original Pertho/physical-Council quests. Beta mission 481 begins recovered field content. |

**Omega is the first shared hub; Beta E1 world 23 is the first common field map.** Beta's Closed-Beta Visit 2 and E2 world 60 occur later and are research holds, not a substitute map for E1. Tollana is Praxis progression and is excluded from the four Union campaigns. Men'fa and Ihpet later use distinct Light/Dark faction maps, so they are not earlier shared-world meetings.

## Source set and limits

- The [Lomiada handoff pack](../../sgw-handoff-pack-v1.2/README.md) contains rendered Human, Free Jaffa, Pertho, Council, Omega and Beta masters. Its [source policy](../../sgw-handoff-pack-v1.2/pack/docs/SOURCE_POLICY.md) and [world audit](../../sgw-handoff-pack-v1.2/audits/audit-worlds.md) should control claims about original maps and mission joins.
- The [Dakara_E1 rebuild ledger](../../dakara-e1-rebuild/README.md), [Class Start v6](../../class-start-v6/README.md), current `db/resources/` seed and `entities/spaces.xml` control **present Cimmeria behavior**. The older content/zone audit is a dated snapshot.
- The final developer ZIP's mission-to-world moniker associations are useful leads but do not prove original server grant edges, actor positions, rewards or playable scripts. Its named SQLite databases were not supplied. The planning PR must not convert a matched title or speaker into an exact runtime relationship.
- Existing map geometry is a design constraint for the Asgard alternate prologue. [Debug Area](../../debug-area/README.md) proves the same-map alias technique on one live world, not the proposed private Omega alias. Omega client placement and phase isolation remain future proof gates.

## Questions for Lomiada's GPT

1. Are any early route edges, mission/step/objective IDs or faction variants wrong, especially Free Jaffa 1570/1655/1500 and Asgard 1522/481? Quote the primary row or client text that contradicts the plan.
2. Does any supplied build actually contain a usable Pertho, Pertho Genetics Lab or High Council production map that our audited handoff missed? Identify the package/build and whether it has playable geometry, not only WorldInfo or a minimap.
3. Does the approved post-Pertho Asgard prologue preserve enough of the Pertho/Council stakes without representing new Omega scenes as 2009 content? Flag any source line that would make the proposed remote Council ruling or Beta communiqué incoherent.
4. Is the Free Jaffa level-1 Dakara bridge consistent with the current project start and the recovered level-3–5 arc? Identify exact steps that would falsely mark unplayed SGC events complete.
5. Are the zone ownership and receiving states sufficient for separate main sessions to implement without duplicating chains or accepting the wrong race's mission? Point to a missing travel, item, dialog or phasing contract.
6. Are any planned first-Beta E1 missions actually from the later Closed-Beta revision or E2? Name exact IDs and build evidence for a correction.
7. The Beta master marks some steps of 381, 1502 and 481 `OFFWORLD / HANDOFF` in its all-missions table but `BETA LOCAL` in race sections. Which locations do raw objective text, actor data and direct world bindings support? Mark unresolved rows explicitly.
8. Mission 421 step 1460 names an “SGC Administrative Complex Infirmary.” Is that a Beta-local named facility or an offworld visit? Give the target world/map evidence and any receiver/return step before recommending that 421 leave its hold.

Please classify each finding as **source contradiction**, **current-runtime conflict**, **untested technical assumption**, or **creative recommendation**, and give a file/row/ID citation. A claim from the progression spreadsheet alone should not override the cooked mission/dialog evidence. A source gap is valuable to report, but it is not proof the plan's chosen authored story is historical.

## Expected review outcome

A useful review either confirms the route and ownership boundaries or returns a short list of concrete corrections with evidence. It need not implement content. The implementation sessions will create exact ID allocations, coordinates, test fixtures and client UAT after the first four-zone close-out and the Human baseline gate.
