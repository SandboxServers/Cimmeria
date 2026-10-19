# Omega Site developer concept art

> **Type:** Reference
> **Audience:** the Union Omega coordinator, the movement, items and mission advisors, and the owner (the open questions)
> **Checked:** 2026-10-18 against `origin/main` @ `eb29b0c2c` (seeds, texts, ring regions) and the four images below
> **Companions:** [Omega campaign](omega.md), [Omega work packets](omega-work-packets.md), [evidence](evidence.md), [execution contract](execution-contract.md)

## Short answer

The owner supplied developer concept art for Omega Site (world 18) on 2026-10-18: one overview and three enlargements of it. Together they show **one map**, not separate zones. The Council Chamber is at the north. Below it is the **Common Area** hub, with the **SGC** (Human) wing to the south-west and the **Free Jaffa** wing to the south-east. A river runs north to south, east of the hub, between the hub and the Free Jaffa wing. Colour-coded labels mark the developers' proposed NPC and service spots.

What the art settles:

- which faction's services belong in which sub-area;
- that Omega has four networked teleporters plus a Command Center beam;
- that the designers wanted class trainers, crafting trainers and a full set of town services in the hub.

What it does not settle: coordinates, NPC names, stock and mechanics. Several of its spots conflict with the current plan, and those are owner questions (§ Conflicts), not overrides.

## Source and provenance

| Item | Value |
|---|---|
| What | Developer concept art of Omega Site with developer-proposed NPC spots, supplied by the owner as confirmed restoration data |
| Class | **RESTORED_SOURCE** for the *intent*: each label's text, its colour (kind) and its placement relative to the other labels and sub-areas. **Not** a coordinate source. |
| Build or date | Unknown. The art is undated and names no build, so it may come from an earlier or later layout than the cooked `Omega_Site` map. Record it beside the map, never instead of it. |
| Positions | Approximate, read by eye from the images. A position below is the label's place in the picture (compass direction, plus pixel x,y on the 1141 × 856 overview), not a world coordinate. |
| Spelling | Labels are quoted as drawn. "Combate Instance" is the art's spelling (read as "Combat Instance"). |

Each table marks what is **read** (visible in the image) and what is **inferred** (joined to repo data by this document). Every join to a seed row, moniker, world or coordinate is inferred unless the row says otherwise.

## Images

| File | Content |
|---|---|
| [omega-site-overview.webp](omega-concept-art/omega-site-overview.webp) | The whole map, on a green ground, with the river. It is the primary image. |
| [omega-site-common-area.webp](omega-concept-art/omega-site-common-area.webp) | Enlargement of the hub and its pods, titled "Common Area" |
| [omega-site-sgc-area.webp](omega-concept-art/omega-site-sgc-area.webp) | Enlargement of the south-west wing, titled "SGC", as a street grid |
| [omega-site-free-jaffa-area.webp](omega-concept-art/omega-site-free-jaffa-area.webp) | Enlargement of the south-east wing, titled "Free Jaffa", as a ring plaza |

## Colour key

| Colour | Kind |
|---|---|
| Red | Location or service (a building, travel point or town service) |
| Green | Vendor |
| Orange | Crafting trainer |
| Purple | Class trainer |

The owner gave the key with the art. A label's kind below comes from its colour (read).

## Layout (read)

From north to south on the overview:

1. **Council Chamber**: a large disc-shaped structure across the top centre, with three "Combate Instance" labels.
2. **Four outer pods**:
   - two small standalone pods north of the hub;
   - two large pods on walkway arms off the hub's north-west and north-east.
3. **Common Area**: an oval hub in the centre, with the **Science Center** pod at its south tip. Two small pods flank the hub to the west and east.
4. **SGC** wing: the grey landmass to the south-west, around a small pod.
5. **Free Jaffa** wing: the grey landmass to the south-east, around a small pod.

The river enters from the north-east in two branches. They join east of the hub, pass between the hub and the east pods, then run between the Science Center and the Free Jaffa wing and leave at the south edge. A white, unpainted band along the south edge has no label; its meaning is unknown.

## Council Chamber (overview only)

| Label | Kind | Approximate position (overview px) |
|---|---|---|
| Council Chamber | location | top centre, on the disc structure (580, 138) |
| Combate Instance | location | north end of the structure (581, 86) |
| Combate Instance | location | west wing of the structure (449, 145) |
| Combate Instance | location | east wing of the structure (703, 144) |

Inferred, not read: the structure's silhouette resembles an Asgard ship hull, and the shipped Command Center (world 80) is "aboard the Asgard Capitol Ship". The art may therefore draw the Capitol Ship here; nothing in the image says so. The three "instance" labels suggest instanced combat entrances. The only other Omega world rows are 81 Ruins and 82 StorageRm, which are out of scope (§ Conflicts, D-UO18).

## Common Area

The hub core, north to south. Every label here is identical on the overview and the enlargement.

| Label | Kind | Approximate position (overview px) |
|---|---|---|
| Command Center | location | hub's north end, top of the axis (585, 390) |
| Stargate | location | just south of the Command Center, on the axis (585, 437) |
| General | vendor | west of the Stargate (505, 432) |
| Consumable | vendor | east of the Stargate (672, 429) |
| Battlemaster | location | on the axis, below the Stargate (588, 477) |
| Inn | location | below the Battlemaster, west of the axis (546, 507) |
| Infirmary | location | below the Battlemaster, east of the axis (618, 507) |
| Teleporter | location | hub's west edge (460, 514) |
| Teleporter | location | hub's east edge (705, 512) |
| Weapon | vendor | west half, mid-hub (510, 531) |
| Armor | vendor | east half, mid-hub (646, 532) |
| Guild Center | location | on the axis, below Inn and Infirmary (584, 547) |
| Consumable | vendor | west half, below Weapon (497, 555) |
| Consumable | vendor | east half, below Armor (668, 557) |
| Auction House | location | south-west quadrant (540, 594) |
| Bank | location | south-east quadrant (624, 589) |
| General | vendor | south-west, below the Auction House (524, 628) |
| General | vendor | south-east, below the Bank (645, 621) |
| Science Center | location | the pod at the hub's south tip (582, 717) |
| Biology | crafting trainer | inside the Science Center pod, west (548, 676) |
| Physics | crafting trainer | inside the Science Center pod, east (618, 676) |

The pods around the hub:

| Pod | Labels | Kind | Approximate position (overview px) |
|---|---|---|---|
| North-west standalone pod | Adv. Electronics | crafting trainer | (437, 286) |
| North-east standalone pod | Adv. Engineering | crafting trainer | (731, 289) |
| North-west large pod (walkway arm) | Adv. Weapon, Adv. Armor | vendors | (381, 385), (347, 418) |
| North-east large pod (walkway arm) | Engineering, Electronics | crafting trainers | (806, 368), (832, 410) |
| West small pod | Asgard | class trainer | (338, 520) |
| East small pod, across the river | Armor, Weapon | vendors | (825, 513), (842, 541) |

## SGC (Human wing)

The enlargement is a street grid and carries more labels than the overview (§ Cross-check). This table follows the enlargement; the overview column says whether the overview shows the label.

| Label | Kind | Approximate position (SGC enlargement) | On overview |
|---|---|---|---|
| Teleporter | location | north edge, where the two diagonal roads meet, beside a small pentagonal building | yes (387, 580) |
| Consumable | vendor | north-west block, north face | yes (289, 572) |
| General | vendor | north-west block, centre | no |
| Scientist | class trainer | north-west block, south-east corner | yes (279, 605) |
| Adv. Physics | crafting trainer | north-west block, west face | yes (207, 605) |
| Adv. Biology | crafting trainer | north-west block, west face, below Adv. Physics | yes (231, 624) |
| Armor | vendor | north row of buildings, centre-north | no |
| Weapon | vendor | north row, below Armor | no |
| General | vendor | central block, north face | yes (325, 655) |
| Archaeologist | class trainer | central block, west face | yes (305, 678) |
| Consumable | vendor | central block, east face | no |
| Soldier | class trainer | south-east diagonal block, east face, north end | yes (432, 667) |
| Commando | class trainer | south-east diagonal block, east face | yes (432, 693) |
| General | vendor | south-east diagonal block, west face | no |
| Weapon | vendor | south-east diagonal block, west face | yes (355, 634) |
| Armor | vendor | south-east diagonal block, west face | yes (368, 663) |
| Consumable | vendor | south-east diagonal block, south end | no |

Where the overview shows one Weapon, Armor, General or Consumable, this document cannot say which of the enlargement's copies it stands for. The "yes" rows above are a best match by position (inferred).

## Free Jaffa wing

A circular plaza with the Jaffa trainer at its centre, ringed by square buildings, with a forked path leading west. The overview and the enlargement match label for label.

| Label | Kind | Approximate position (Free Jaffa enlargement) | Overview px |
|---|---|---|---|
| Jaffa | class trainer | centre of the ring plaza | (819, 664) |
| Consumable | vendor | ring, north-north-west building | (782, 634) |
| General | vendor | ring, north-north-east building | (858, 637) |
| Consumable | vendor | ring, east-south-east building | (882, 670) |
| Armor | vendor | ring, south-south-west building | (797, 695) |
| Teleporter | location | west of the ring, a pentagonal building on the path | (717, 661) |
| General | vendor | west, beside the path | (703, 687) |
| Weapon | vendor | far south-west, at the end of the path's southern fork | (690, 724) |

## Cross-check: enlargements against the overview

| Area | Overview | Enlargement | Reading |
|---|---|---|---|
| Common Area hub core | 21 labels (the table above) | the same 21, plus the title "Common Area" | identical |
| North and north-east pods, west pod | as tabled | identical | identical |
| East pod | Armor **and** Weapon | Armor only | difference |
| South-east small pod | no label of its own (it sits inside the Free Jaffa cluster) | **Weapon** | difference |
| South-west small pod | no label of its own (it sits inside the SGC cluster) | **General** above, **Consumable** below | difference |
| SGC wing | 1 each of Consumable, General, Weapon, Armor; all trainers, crafting trainers and the teleporter | 3 Consumable, 3 General, 2 Weapon, 2 Armor; the same trainers, crafting trainers and teleporter | the enlargement is more complete; use its counts |
| Free Jaffa wing | 8 labels | the same 8 | identical |
| Council Chamber | 4 labels | not enlarged | overview only |

The three pod differences have two readings, and the art does not decide between them:

- the Common Area crop moved nearby wing labels onto the pods (the east pod's Weapon to the south-east pod, and an SGC General and Consumable to the south-west pod);
- the small pods carry vendors of their own.

This document follows the overview for the pods and records the difference. It blocks nothing (ledger F35).

## Totals by kind (overview, with the SGC enlargement's counts)

| Kind | Common Area (with pods) | SGC | Free Jaffa | Total |
|---|---|---|---|---|
| General vendor | 3 | 3 | 2 | 8 |
| Consumable vendor | 3 | 3 | 2 | 8 |
| Weapon vendor | 2 (hub, east pod) | 2 | 1 | 5 |
| Armor vendor | 2 (hub, east pod) | 2 | 1 | 5 |
| Adv. Weapon / Adv. Armor vendor | 1 / 1 (north-west pod) | 0 | 0 | 2 |
| Class trainer | 1 (Asgard) | 4 (Scientist, Archaeologist, Soldier, Commando) | 1 (Jaffa) | 6 |
| Crafting trainer | 6 (Engineering, Electronics, Biology, Physics, Adv. Electronics, Adv. Engineering) | 2 (Adv. Physics, Adv. Biology) | 0 | 8 |
| Teleporter | 2 | 1 | 1 | 4 |
| Other locations | Command Center, Stargate, Battlemaster, Inn, Infirmary, Guild Center, Auction House, Bank, Science Center | none | none | 9 |

The art draws **no crafting vendor** anywhere, and no Goa'uld trainer.

## Joins to shipped data (inferred)

| Art label | Shipped counterpart | Status |
|---|---|---|
| Stargate | Gate 5 at (34.705, -50.634, -149.092), the only gate on world 18 (`stargates.sql`) | inferred |
| Command Center | Ring region **14** `OmegaSiteToCmdCenterRegion` at (34.575, -35.770, 55.698), whose only destination is world-80 region 17 (`ring_transport_regions.sql`); destination moniker 27423 `gr_OmegaSite_AsgardBeamNetwork_CommandCenter` | inferred: the art's label marks the beam, not the room (D-UO20) |
| Teleporter × 4 | Ring regions **10** Center (34.41, -42.65, -75.46), **11** Bottom (-18.56, -29.49, -207.52), **12** Left (-210.15, -36.73, -92.33) and **16** Right (191.98, -48.93, -122.78). They form one network: each lists the other three as destinations (`ring_transport_regions.sql`, spawns 132-136 `OmegaSiteBeam*`). | the count of four is read from the seed; which art teleporter is which region is inferred |
| Beam destination names | 26927 "Jaffa Plaza", 26973 "Human Plaza", 26981 "Bank", 26982 "Asgard Platform", 26983 "Omega Site", 27423 "Command Center": six names for the six Omega regions (10, 11, 12, 16, 14, 17) | inferred. Plausible: Human Plaza is the SGC wing's teleporter (region 12, west) and Jaffa Plaza the Free Jaffa wing's (region 16, east). The two hub teleporters' names stay open. |
| Bank | Banker moniker 29462 "Storage Officer" (UO-10) and beam destination 26981 "Bank" | inferred |
| Guild Center | Registrar moniker 29068 "Organization Registrar" (UO-10, two NPCs, F24) | inferred |
| Science Center | The Omega master's "Science Center" (1426) and "Science Facility" (1425) | inferred, by name |
| SGC Weapon / Armor | Human Weapons 27240-27243, Human Armor 27244-27247 | inferred |
| Free Jaffa Weapon / Armor | Jaffa Weapons 27224-27227, Jaffa Armor 27228-27231 | inferred |
| Hub and east-pod Weapon / Armor; Adv. Weapon / Adv. Armor | No direct moniker. Candidates: the higher tiers of the Human or Jaffa vendors, or the Asgard programs and armor 27252-27259 (held, UO-16). | open (D-UO16) |
| Consumable × 8 | Consumables 27264, one moniker | inferred |
| General × 8 | No Omega moniker. The only general-goods moniker is Harset's 8006 `DN_npc_ven_TBD_Harset_ConsumablesGeneral`. | none (D-UO16) |
| (no art spot) | Crafting vendors: Human 27248-27251, Common 27236-27239, Jaffa 27232-27235, Asgard 27260-27263 | the art has no crafting vendor (D-UO16) |
| Soldier, Scientist, Jaffa, Commando, Archaeologist | Global basic-skill trainers 7988, 7989, 7990, 7992, 7993 (`DN_npc_trn_Global_*BasicSkills`); trainer clone pattern: template 301 (`INT_TRAINER` 128, trainer list 1) | inferred (D-UO17) |
| Asgard | Omega trainers 27218 "Asgard Basic Skills", 27219 "Asgard Advanced Skills" | inferred; held with the Asgard route (UO-16) |
| Engineering, Electronics, Biology, Physics and the four Adv. | No crafting-trainer moniker exists anywhere in `texts.sql`; every `DN_npc_trn_*` row is a skill trainer | none (D-UO17) |
| Inn, Infirmary, Battlemaster, Auction House | No Omega moniker | none (D-UO19) |
| Council Chamber, Combate Instance | No world row of that name; worlds 81 Ruins and 82 StorageRm are the other Omega rows | open (D-UO18) |

## Orientation against the world axes (inferred)

The art has no compass or scale. Read with art up = +z and art right = +x:

| Anchor | World (x, z) | Art | Agrees? |
|---|---|---|---|
| Gate 5 | (34.7, -149.1) | Stargate, hub centre-north | anchor |
| Region 14 (Command Center beam) | (34.6, 55.7): same x as the gate, 205 m +z | Command Center directly above the Stargate | yes |
| Region 12 Left; UO-01's Human vendor props (x ≈ -170, z ≈ -166 to -171) | west of the gate | SGC wing, south-west | west: yes; south: roughly (Human props sit 17-22 m -z of the gate) |
| Region 16 Right; UO-01's Jaffa vendor props (x ≈ 216 to 389, z ≈ -44 to -109) | east of the gate | Free Jaffa wing, south-east | east: yes; south: **no** (the props sit 40-105 m +z of the gate) |
| Region 11 Bottom | (-18.6, -207.5), south of the gate | the art's south (Science Center, the wings) | roughly |
| Region 10 Center | (34.4, -75.5), between the gate and region 14 | the art's hub teleporters sit west and east, below the Stargate | **no** |

East and west agree with the art; north and south only partly. The art is schematic, so it can never move a coordinate. UO-01 navmesh-probes every placement as before. It also records, per key, which art sub-area the point falls in and any disagreement (D-UO15).

## Conflicts with the current plan (owner questions)

The plan stays as written until the owner answers. Each row is a BlockedDecision in the [ledger](omega-work-packets.md#held-and-blocked).

| ID | The art shows | The plan says | Question |
|---|---|---|---|
| D-UO16 | General, Consumable, Weapon and Armor vendors in every area (8 / 8 / 5 / 5), plus Adv. Weapon and Adv. Armor, and no crafting vendor | Eight moniker-led templates: Human and Jaffa Weapons, Armor and Crafting, Common Crafting and Consumables, one spawn each (D-UO8, UO-06, UO-11, UO-12) | Which roster does Omega get? See the ledger row for options A-C. |
| D-UO17 | Six class trainers and eight crafting trainers | No trainers | May Omega have class trainers on the shipped global monikers, and crafting trainers, which have no moniker anywhere? |
| D-UO18 | A Council Chamber with three combat instances, north of the hub | U-D08: a remote Council hearing in Command Center geometry, the physical-Council quests held; worlds 81/82 out of scope; no instance mechanic planned | Is the Council Chamber in scope, and is it the venue for the Asgard hearing? |
| D-UO19 | An Inn, Infirmary, Battlemaster and Auction House in the hub | "Rejected additions": no inn, auctioneer, battlemaster or medic without an Omega moniker | Does the art's developer intent lift the "no moniker" rejection for these four? |
| U-D08 / U-D11 (extended) | An Asgard class trainer in the hub's west pod | The Asgard route and its services are held (UO-16) | When the Asgard route opens, does its trainer stand in the west pod? |

Decided by the architect, not the owner, and recorded so it is not silent:

- **D-UO15:** the art is layout intent, never coordinates.
- **D-UO20:** the art's hub Command Center is read as the pad-14 beam, and the cast stays in world 80, because the shipped mission, dialog and ring rows put it there.

The art *reconciles* the plan in three places:

- the Bank and the Guild Center match the planned banker and registrars;
- the four teleporters match the seeded beam network, which no packet changes;
- the faction wings match the planned Human-west, Jaffa-east vendor props.

## What the art does not settle

- Coordinates, headings, floor heights or navmesh reachability.
- NPC names, bodies, dialog, stock, prices or trainer ability lists.
- Which hub teleporter is which ring region, and the names of the two hub teleporters.
- Whether the Common Area crop's pod labels are relocated wing labels (§ Cross-check).
- What the white band along the south edge is.
- Whether the Council Chamber is the Capitol Ship (world 80), new geometry, or an entrance to worlds 81/82.
- The build or date of the art, and so whether it predates or follows the cooked map and the client's monikers.

## Where the art is wired

| Packet | What the art changes |
|---|---|
| [UO-01](omega-packets/UO-01.md) | New input: the worknote gains an art-area column and a § Art conflicts section, plus the orientation table above |
| [UO-05](omega-packets/UO-05.md) | Note only: the cast stays in world 80 (D-UO20) |
| [UO-06](omega-packets/UO-06.md) | The worknote maps the art's vendor kinds and states D-UO16 with options A-C |
| [UO-10](omega-packets/UO-10.md) | Banker at the art's Bank and registrars at its Guild Center (Common Area); the report quotes UO-01's art area |
| [UO-11](omega-packets/UO-11.md) | Tau'ri vendors belong in the SGC wing; D-UO16 joins D-UO8 as a start gate |
| [UO-12](omega-packets/UO-12.md) | Jaffa vendors belong in the Free Jaffa wing; Jaffa Crafting has no art spot (D-UO16) |
| [UO-14](omega-packets/UO-14.md) | Airmen in the SGC and Common areas, Free Jaffa warriors in the Free Jaffa and Common areas |
