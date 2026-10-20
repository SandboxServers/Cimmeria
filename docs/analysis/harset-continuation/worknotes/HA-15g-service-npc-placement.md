# HA-15g: Harset vendor and trainer NPC placement plan

> Type: design worknote. Audience: HA-15b (stock and price seal), HA-15c
> (generator), the four-zone coordinator. Updated: 2026-10-18.
> Status: PROVISIONAL (O18-05). Docs only. Nothing here was verified in game.

Inputs: [HA-15f](HA-15f-omega-reference.md) (roster rows 1-14),
[O18-04 and O18-05](owner-decisions-2026-10-18.md),
[HA-01a1 landings](HA-01a1-landing-geometry.md),
[id-allocation](../id-allocation.md), [work-packets](../work-packets.md).

Labels: FACT (read in a seed, file or probe output), INFERENCE (derived from
facts), PROVISIONAL (AI-derived draft, O18-13), DECISION (owner-decided, with
ID), MAINTAINER DECISION (open, not mine to make).

## 1. Summary

1. FACT: Harset already has one service NPC, template 219 "Storage Lo'taur",
   the bank NPC at spawn 314 in the Jaffa Zone. It has no `INT_Banker` bit and
   no vault scope.
2. DECISION (O18-04): the Omega Site is the layout reference. Vendor identity
   (what the NPC looks like and is called) must agree with what it sells.
3. PROVISIONAL: 20 new service NPCs at first seal, 6 districts, all in worlds
   57, 68 and 69. World 70 gets none. Advanced trainers are deferred.
4. PROVISIONAL: the Goa'uld-facing vendor is a Lo'taur (section 5).
5. CORRECTION to HA-15f row 12: do not add a world-70 banker. The Lo'taur
   bank NPC already exists in world 57 (section 4.5).
6. SHORTFALL: the service roster needs about 21 templates; id-allocation
   offers candidate templates 261-269 only (nine). Section 7 asks for capacity.
7. Vendor faction gating: MAINTAINER DECISION, left open (section 6).

## 2. Engine facts that bound the design

| Fact | Label | Consequence |
|---|---|---|
| Vendors have no faction gate in the engine | FACT (agent brief) | Identity is flavour, not enforcement |
| Vendor rows ignore `max_stack_size` | FACT | One unit per row |
| Purchases land in bag 1 | FACT | No bag-choice design needed |
| A quest speaker must carry no vendor or trainer fields | FACT | Talk-then-trade needs two templates |
| faction 10 is damageable and auto-aggros | FACT | Every service NPC uses faction 1 |
| Trainer lists are generator-owned (`tools/ability_trees/generate_seed.py`) | FACT | Hand-edited trainer SQL is overwritten |
| `item_lists` ids are FK-checked | FACT | A vendor cannot reference an unseeded list |
| Trainer list ids seeded in this worktree are 1 and 350 | FACT | HA-15f cited 1-6 and 350-353; I could not reproduce that. HA-15c must re-read |
| Interaction bits: vendor 65536, banker 2, trainer 128, registrar 64, mail 134217728 | FACT | Per template |
| Service templates seen use faction 1, level 1 | FACT | Copy the pattern |

## 3. Client monikers available (FACT, `texts.sql`)

| name_id | Moniker | Use here |
|---|---|---|
| 8009, 8010, 8011 | Basic Equipment Procurement Officer / Quartermaster / Lo'taur | Deferred tier-1 "basic" generalists |
| 8014, 8015 | Weapons / Armor Procurement Officer | Human vendors |
| 8016, 8017 | Jaffa Armor / Weapons Quartermaster | Jaffa vendors |
| 8012, 8013 | Goa'uld Weapons / Armor Lo'taur | Goa'uld vendors |
| 8006, 8007 | Consumables vendors | Consumables, world 57 |
| 28195-28198 | Common crafting vendors | Market |
| 28199 | Op-Core Mission Reward | World 68 |
| 7988-7993 | Global basic trainers | Six basic trainers, world 57 |
| 7994-8004 | TBD advanced trainers | Deferred |
| 27218, 27219 | Omega Asgard trainers | Not used (no Asgard content in Harset) |
| 29463 | Storage Lo'taur | Existing bank NPC |

PROVISIONAL: 7968-7979 are "TBD" basic and rare vendors; do not use a TBD
moniker on a placed NPC unless the client text reads acceptably.

## 4. Placement plan per district

Navmesh method (FACT): the existing `nav_inspect.exe` probe, 0.4 m horizontal
and 0.8 m vertical tolerance, `harset.nav`, `harset_cmdcenter.nav`,
`harset_market.nav`, `harset_storagerm.nav`. "ring n/8" is how many of eight
probes on a 1.5 m circle also hit the same component, a crude clearance check.
`find_path` from each arrival was NOT run. The mesh y of the hub street is
about +0.3 above the floor note, so seed y must come from the mesh, not from
the numbers below (INFERENCE).

### 4.1 World 57, gate plaza (Tau'ri arrival, trainers)

Existing cast: Hansen 300 and Jacobs 301 at the gate (FACT, placements.md).
Guards stand at x about +-18.6, z 11-22 (FACT, PL-B).

| Role | Template look | Moniker | Candidate (x, z), component 11 | Ring |
|---|---|---|---|---|
| Basic trainer 1 | SGC uniform, clone of 212/213 | 7988 | (-14, 30) | 8/8 |
| Basic trainer 2 | SGC uniform | 7989 | (14, 30) | 8/8 |
| Basic trainer 3 | SGC uniform | 7990 | (-8, 18) | 8/8 |
| Basic trainer 4 | SGC uniform | 7991 | (8, 18) | 8/8 |
| Basic trainer 5 | SGC uniform | 7992 | (0, 20) | 8/8 |
| Basic trainer 6 | SGC uniform | 7993 | (-10, 26) | 6/8 |

FACT: all six pass on component 11. Reserve (10, 26), ring 5/8, as a spare.
INFERENCE: which archetype each trainer serves is generator output
(archetypes seeded: Soldier, Commando, Scientist, Archeologist, Asgard,
Goauld, Sholva, Jaffa, Any). HA-15c decides the mapping; this note only
reserves six slots. A Jaffa or Goa'uld player trains at the same plaza
because the plaza is where every arrival lands (INFERENCE).

Look: the SGC uniform is the Tau'ri precedent, but a trainer in a Jaffa or
Goa'uld arrival must not read as a Tau'ri officer teaching a Jaffa skill.
PROVISIONAL: dress trainers by archetype (Tau'ri SGC for Soldier, Commando,
Scientist, Archeologist; Jaffa Praxis for Jaffa and Sholva; Goa'uld Praxis for
Goauld). Archetype to look pairing is HA-15c's output; flag it as a visual
check.

### 4.2 World 57, merchant street (consumables)

| Role | Look | Moniker | Candidate | Ring |
|---|---|---|---|---|
| Consumables | Jaffa merchant, clone of 201 | 8006 | (18, -24) | 8/8 |
| Consumables, second | Jaffa merchant | 8007 | (-22, -24) | 8/8 |

FACT: Lo'rak 302 stands at (22, -30) as template 201 and is a quest speaker,
so the vendor must be a new template (talk-then-trade rule). INFERENCE: keep
the vendor at least 4 m from Lo'rak. (18, -24) is 7.2 m away. Spare
candidates on component 11: (28, -20), (30, -14), (24, -36), (-22, -30).
Rejected: (-28, -20) fails horizontal tolerance; (34, -26) is on component 90.

### 4.3 World 57, OP-Core zone (Human weapons and armor)

FACT: set 2101 spans x 126-256, z -64-164 and contains the landmark
EM-Quartermaster01 at (188.51, 156.71).

| Role | Look | Moniker | Candidate | Ring |
|---|---|---|---|---|
| Human weapons | SGC uniform, clone of 214 | 8014 | (188.51, 156.71) | 7/8 |
| Human armor | SGC uniform | 8015 | (180, 150) | 7/8 |

FACT: both component 11. The landmark name suggests a quartermaster counter
already exists in the map (INFERENCE; no model check). Spare: (188.5, 145),
ring 8/8. Second cluster near EM-Bunker00 (202.08, -9.04), component 11:
(210, -9) and (185, -9). PROVISIONAL: leave the bunker cluster for advanced
vendors. Rejected: (195, 150) is on component 226; (195, -2) on 107.

### 4.4 World 57, Jaffa zone (Jaffa weapons and armor)

FACT: set 2100 spans x -208 to -128, z -70 to 140.

| Role | Look | Moniker | Candidate | Ring |
|---|---|---|---|---|
| Jaffa weapons | Jaffa Praxis, clone of 160 | 8017 | (-160, 84) | 8/8 |
| Jaffa armor | Jaffa Praxis, clone of 159 | 8016 | (-170, 95) | 8/8 |

FACT: both component 11. Spare: (-175, 70), ring 8/8. Keep both well away
from the bank courtyard (-184.5, 162.3).

### 4.5 World 57, Goa'uld terrace (Goa'uld vendors) and the bank

| Role | Look | Moniker | Candidate (palace terrace y -28.25) | Ring |
|---|---|---|---|---|
| Goa'uld weapons | Lo'taur (section 5) | 8012 | (-165, 226) | 5/8 |
| Goa'uld armor | Lo'taur | 8013 | (-172, 230) | 6/8 |

FACT: both component 11. Spare (-166, 233), ring 6/8. Rejected: Petbe's
position (-160.5, 232.6) is on component 278. INFERENCE: Petbe's stands on
a detached component, so no vendor goes within 5 m of it.

Bank: FACT, spawn 314 at (-186, -41.28, 160), template 219 "Storage Lo'taur".
At the default 2 m / 3 m tolerance it is on component 11; at the tight 0.4 m /
0.8 m tolerance it reads dy -8.85 on component 236 (conflicting probe
results, section 8). PROVISIONAL: a nearby re-seat candidate is (-180, 165),
ring 8/8, component 11. A later seed packet adds `INT_Banker` and a vault
scope to 219; this note does not change it. Drop HA-15f row 12 (world-70
banker): one bank NPC suffices (INFERENCE).

### 4.6 World 68, command centre (mission-reward vendors, registrar)

FACT: arrival (0, 0.355, -20); existing cast in set 2079, x -13 to 10.6,
z -37.6 to -29.5. Component 1.

| Role | Look | Moniker | Candidate | Ring |
|---|---|---|---|---|
| Mission reward 1 (Tau'ri) | SGC uniform | 28199 | (-10, -14) | 8/8 |
| Mission reward 2 (Jaffa) | Jaffa Praxis | 28199 | (6, -14) | 8/8 |
| Mission reward 3 (Goa'uld) | Lo'taur | 28199 | (10, -12) | 6/8 |
| Registrar | SGC uniform | none | (-8, -22) | 8/8 |

FACT: all four on component 1. Spare (0, -12), (-5, -8). Rejected: (12, -20)
is off-mesh; (30, -18) is on component 3. INFERENCE: (-8, -22) is 8 m from
the arrival and 7.5 m from the existing cast box; fine for a registrar, but
keep the Mission-reward trio south of z -12 so they do not crowd the arrival.
The three reward vendors share one moniker, so give each a distinct
`name_id` only if the client text supports it (open).

### 4.7 World 69, Market (crafting)

FACT: Market landing 1551 at (20, 4.81, 8.5); door-1006 landing at (80, 3.61,
61); respawner 22 at (48, 3.61, 78). Main component 5.

| Role | Look | Moniker | Candidate | Ring |
|---|---|---|---|---|
| Common crafting | Jaffa merchant | 28195 | (75, 75) | 7/8 |
| Crafting 1 | Jaffa merchant | 28196 | (55, 60) | 8/8 |
| Crafting 2, 3 | needs more probes | | | |

FACT: only two non-landmark candidates passed (component 5). Failed: (60,50),
(50,70) and (45,45) read dy -14, a lower floor; (70,55) and (65,65) are on
other components; (85,70) reads dy -39. The door landing (80, 61) and the
respawner (48, 78) are valid mesh but must not hold vendors (respawner kill
zone, door arrival). PROVISIONAL: HA-15f counted 12 crafting at full and 3 at
first seal; only two sites exist. Seal 2 now. The remaining slots wait on a
Market probe pass (section 8).

### 4.8 World 70, Storage

FACT: arrival (61.4, 5.12, 5.2), component 4 (all probes pass). PROVISIONAL:
no service NPC at first seal; the storage room is the destination for a bank
interaction, not a hub. Reserve (55, 8), (66, 8), (60, 12) (ring 6-8/8) if
the banker decision reverses.

## 5. The Goa'uld-facing vendor: Lo'taur (DECISION basis O18-04)

PROVISIONAL decision: the Goa'uld vendor is a Lo'taur.

Reasoning:

1. FACT: the client has four Lo'taur monikers (8011, 8012, 8013, 29463).
   The Goa'uld Weapons and Armor Lo'taur pair (8012, 8013) exists exactly for
   this role.
2. INFERENCE: a Jaffa serves a Goa'uld, but a Jaffa vendor selling Goa'uld
   tech blurs the three-faction identity. A Lo'taur (a Goa'uld's servant)
   reads as "the Goa'uld's own supply", which is what Omega Site conveys.
3. INFERENCE: using a Lo'taur keeps Jaffa vendors as Quartermasters (8016,
   8017) and Tau'ri vendors as Procurement Officers (8014, 8015), so the
   three trades never share a name or look.
4. FACT: the Harset bank NPC (219) is already a Lo'taur, so the look is not
   new to the zone.

Look: FACT: template 353 "Lo'Taur Servant" has `BS_GoauldMale` and the AR_G_Underlings
component set but is a class-pet template with flag 1024, and has never been
rendered. PROVISIONAL: clone 353 with a mob class and no pet flag. Fallback if
the clone renders badly: a Goa'uld Praxis look (template 360 on the Lethander
head) keeping the Lo'taur name. Template 219 (human-female, pistol) is the
visual precedent for the bank only and is NOT the vendor look. The clone is
unverified in game.

## 6. Identity versus stock coherence, and faction gating

| Vendor | Identity | Stock theme (for HA-15b) |
|---|---|---|
| Human weapons / armor | SGC officer | Tau'ri weapons / armor, tier 1-2 |
| Jaffa weapons / armor | Jaffa quartermaster | Jaffa weapons / armor, tier 1-2 |
| Goa'uld weapons / armor | Lo'taur | Goa'uld weapons / armor, tier 1-2 |
| Consumables | Jaffa merchant | Faction-neutral consumables |
| Crafting, common crafting | Merchant | Crafting inputs only |
| Mission reward (3) | One per faction | Reward-class stock, no purchase of tier above 2 |

DECISION basis O14-12: tier-comparable stock, no new currency, no high-tier
gear.

INFERENCE: with no engine gate, a Goa'uld player can buy at the OP-Core
officer. Coherence therefore means (a) the three factions' rows and prices are
mirror images (same tier, same item count, same price ladder) and (b) the
name matches the theme. Any faction gate is a MAINTAINER DECISION; the
options are no gate (this plan), a dialog-side check on the speaker, or a
future engine gate. I recommend no gate, with parity, until the maintainer
decides.

## 7. Roster at first seal (PROVISIONAL)

| Group | Count | Templates needed |
|---|---:|---|
| Basic trainers | 6 | 6 (`ID needed`) |
| Faction weapons / armor vendors | 6 | 6 |
| Consumables | 2 | 1 (shared) |
| Crafting + common crafting | 2 | 2 |
| Mission reward | 3 | 3 |
| Registrar | 1 | 1 |
| Existing bank 219 | 0 new | 0 (seed edit later) |
| Total new | 20 | about 19 |

Crafting at full scale (12 + 4), the three basic-equipment generalists
(8009-8011), and the eleven advanced trainers are deferred.

Capacity: id-allocation holds templates 261-269 as unassigned capacity (nine).
About 19 are needed. All service template, list and spawn IDs below are
`ID needed`; I cite only the candidate ranges that exist. Request: the
coordinator widens the template range or assigns one template per distinct
(look, list) pair. Service spawns have no candidate range at all (360-379 are
spoken for); point sets (2134, 2135, 2146-2159) are unneeded because service NPCs
are single-point spawns.

## 8. Open items and conflicts

1. Bank probe conflict: tight and default tolerance disagree on whether spawn
   314 is on component 11. A re-seat candidate is above. Unresolved.
2. Market has two usable sites for 3-4 crafting NPCs; more probes needed.
3. No `find_path` run from any arrival to any candidate.
4. No clearance check against doors, respawners and props beyond the
   respawner (48, 78) and door landing (80, 61).
5. Lo'taur clone rendering unverified.
6. HA-15f cited trainer lists 1-6 and 350-353; the seeded ids are 1 and 350.
7. Trainer archetype to look mapping not decided.
8. Mission-reward moniker 28199 is shared; separate names need client text.

## 9. Hand-offs

HA-15b (stock and price seal):

- Build one stock per (faction, role, tier). Parity between factions is the
  coherence rule.
- One unit per row (no `max_stack_size`); tier 1 at 1.0x, tier 2 at 2.5x,
  ladder only. Do not anchor on debug list prices.
- No new currency, no tier above 2.
- Item list ids must exist before any vendor references them (FK-checked).

HA-15c (generator):

- Trainer lists are generator output; reserve six basic slots and decide the
  archetype map. Defer the eleven advanced slots.
- Re-read the seeded list ids (1 and 350) and reconcile with HA-15f.
- Emit a vendor template only with a valid buy-list id.

## Tests the implementation owes

1. Seed test: every service template has faction 1 and the right interaction
   bit; none is faction 10.
2. Seed test: no template that is a quest speaker also carries vendor or
   trainer fields.
3. Seed test: every vendor `item_list` id resolves; each faction pair of
   vendors has equal row counts and equal price ladders.
4. Seed test: no vendor row sets a stack above 1.
5. Placement test: every service spawn lies on the world's main mesh
   component within tolerance, and at least 4 m from any quest speaker,
   respawner and door landing.
6. Revert proof: removing a vendor's list id must fail the FK check.
7. Rendering check for the Lo'taur clone: a lab item for the owner, not an
   automated test.

## Telemetry the implementation owes

1. A counter per vendor purchase keyed by vendor template and item tier, so a
   faction parity skew is visible.
2. A log row when a vendor or trainer interaction opens, with NPC template,
   player faction and world id.
3. A warning when a service spawn fails mesh placement at startup, with the
   world id and candidate coordinates.
4. A counter for a refused purchase (bag full, funds), so the bag-1 landing
   is observable.
