# Tollana archaeology, wave 1

> **Title**: Tollana archaeology, wave 1
> **Type**: Reference
> **Audience**: Tollana campaign engineers, mission advisor, coordinator
> **Last updated**: 2026-10-17
> **Companions**: [Tollana ledger](../README.md), [T-01 source/map contract](T-01-source-map-contract.md)

Offline answers to five open questions for the 568 pilot and the 618/621
Harset handoffs. **Nothing here was lab-tested.** No client process, debugger,
server or lab was used. Evidence is the seed tree (`db/resources/`), repo docs,
the handoff ZIP extracts and read-only parses of cooked client packages with
`tools/upk_parser.py`. No client asset was copied into the repo; only package
and object names are quoted.

## Confidence labels

| Label | Meaning |
|---|---|
| ORIGINAL_DATA | Read directly from a seed row, a cooked package table or a handoff CSV row. An absence claim means a full scan of the named files found nothing. |
| INFERRED | Follows from ORIGINAL_DATA by naming, speaker order, topic or VO match. Not proven by a binding row. |
| UNKNOWN | No evidence found either way. |

Paths below are relative to `db/resources/` unless they start with `docs/`.
`file:N` is a line number in the zone-tollana worktree. Bare file names are
the `Seed/` files: missions, steps, objectives and tasks under `Missions/`,
dialogs, set maps, screens and speakers under `Dialogs/`, `texts.sql` under
`Texts/`, `entity_templates.sql` under `Entities/`, `items.sql` and
`items_event_sets.sql` under `Items/`, `entity_interactions.sql` under
`Worlds/`.

## Conventions used in the answers

- Dialog set maps flag the role of a dialog inside a set
  (`docs/content/interaction-flags.md:102-120`): 8388608 Offer,
  16777216 Active, 33554432 TurnIn, 1073741824 MissionWorldObject (MWO).
- Dialog sets bind to NPCs at runtime through content-chain `add_dialog_set`
  actions. `entity_templates.interaction_set_id` is vestigial
  (`docs/content/association-map.md:139-157`).
- `entity_templates.event_set_id` 570 is event set 'Players default event set'
  (`Events/Seed/event_sets.sql:1323`). It is not dialog set 570; the two ID
  spaces are separate.
- Journal text monikers use a 0-based step index: `DS<n>` is step index n.
- VO cue names read `VO_D_<NPC>_<MsM|MsS##>_<Mission>`. `MsS##` is the same
  0-based DS index, shown by mission 611 below. The handoff grades all such
  name links `CANDIDATE_NOT_PROVEN_NAME_ASSOCIATION`.

### Correction recorded during this pass

An earlier draft tied `VO_D_Baal_MsS05_HarsetTurnIn_*` to mission 621 and read
`MsS##` as 1-based. Both were wrong. The cue belongs to mission 611
'Harset Turn In' (`Missions/Seed/missions.sql:1545`). Its DS5 step is 4027
"Report Nerus' findings to Baal" (`Texts/Seed/texts.sql:11504`,
`DS5_..._GLD_HarsetTurnIn`). MsS05 = DS5 = step index 5, so the index is
0-based. It is not 621 evidence.

## Q1. Who receives missions 618 and 621?

### 618, objective 2438, step 2100

**Answer: Moh'katan (`CmdCenter_Mohkatan`, world 68, spawn 343, template 54).
INFERRED, medium-high.**

| Evidence | Source | Label |
|---|---|---|
| 618 'Delivering the Book of the Gates', level 14 | `Missions/Seed/missions.sql:1559` | ORIGINAL_DATA |
| Steps: 2099 (idx 0) and 2100 'Speak with the leadership in the Command Center' (idx 1); monikers DS0/DS1 `_Ms_A01_Tollana_DRK_BookDelivery` | `mission_steps.sql`, `texts.sql` | ORIGINAL_DATA |
| Objective 2438 text is blank | `mission_objectives.sql:6229`, `texts.sql` | ORIGINAL_DATA |
| Dialog set 567 'Book Delivery': 2269 Offer (speaker 0), 2270 flag 0 (Captain Copplemann 1100: "Take the book to the Council Chambers on Harset"), 2272 TurnIn | `Dialogs/Seed/dialog_set_maps.sql:1879-1883`; `dialog_screens.sql:10382` | ORIGINAL_DATA |
| TurnIn dialog 2272 uses event set 540 'Book Delivery Mohkatan/Anat/Marsh/Baal dialog VO' | `dialogs.sql:4043`; `Events/Seed/event_sets.sql:1305` | ORIGINAL_DATA |
| 2272 speaker order: 945, 944, 942, 945, 942, 941, 942, 0, 945, 944, 945. Moh'katan (945) opens and closes | `dialog_screens.sql:10386-10406`; `speakers.sql:63` | ORIGINAL_DATA |
| `d_tollana_BookDelivery.fsb` (builds 55124/57050): `VO_D_Mohkatan_MsS02_BookDelive` x7, `VO_D_Copplemann_MsM_BookDelive` x1 | handoff `Tollana_08` VO rows | ORIGINAL_DATA (rows); INFERRED (link) |

The TurnIn is a council scene: Moh'katan, Anat, Baal and Marsh all speak.
Moh'katan is the first and last speaker, and the event set lists him first.
Step 2100's "leadership" fits a group scene with one owning actor.
`MsS02` does not match a 618 step by index (618 has only DS0/DS1). It is still
Moh'katan VO in the BookDelivery bank. That is a naming oddity, not a
contradiction. The Copplemann `MsM` cue fits dialog 2270, where Copplemann on
Tollana sends the player to Harset.

Open: no chain binds set 567 to any actor. Baal (942) is the alternate if the
council scene is owned by the speaker who takes the book.

### 621, objective 4734, step 4103

**Answer: Baal (`CmdCenter_Baal`, world 68, spawn 340, template 42).
INFERRED, medium.**

| Evidence | Source | Label |
|---|---|---|
| 621 'Storming the Curia', level 15 | `missions.sql:1565` | ORIGINAL_DATA |
| Step 4103 (idx 4) 'Bring the Gate Address and Activation Code back to Harset' | `mission_steps.sql`; `texts.sql:15130` (`DS4_..._DRK_StormingtheCuria`) | ORIGINAL_DATA |
| Objective 4734 text is blank | `mission_objectives.sql:6255` | ORIGINAL_DATA |
| Dialog set 570 'Storming the Curia': 2282 Offer, 2283 More Info, 5344 MWO (no dialog), 6154 TurnIn = dialog 4405 | `dialog_set_maps.sql:1895,1897,5269,6581` | ORIGINAL_DATA |
| Dialog 4405 speaker order: 942, 0, 0, 944, 945, 941, 0, 944, 0, 944, 945, 944. Baal opens by taking the crystal | `dialog_screens.sql:18109-18133`; `dialogs.sql:7305` (no event set) | ORIGINAL_DATA |
| Item 2770 'Activation Code', "related to getting information from the Book of the Gates" | `Items/Seed/items.sql:11788` | ORIGINAL_DATA |

Caveats:

- The TurnIn prose names a crystal or key. Step 4103 names a gate address and
  an activation code. That item drift is unresolved.
- No `StormingtheCuria` or Curia VO was found in the handoff VO rows searched.
- Dialog set 570 has no chain or template binding.
- Anat (944) speaks most often in 4405. Baal is chosen because he opens the
  scene and takes the delivered object.

## Q2. Who is speaker 959, and is the 568 offer off Tollana?

**Answer: speaker 959 is Colonel Marsh. INFERRED, strong. The 568 offer is
made off Tollana, on Harset. INFERRED, medium. The offer's exact world and
actor binding are UNKNOWN.**

| Evidence | Source | Label |
|---|---|---|
| Speaker 959 has an empty name and appears only in dialog 2151 | `speakers.sql:369`; `dialog_screens.sql` scan | ORIGINAL_DATA |
| Dialog 2151 uses event set 538 'Tollan Tech Marsh dialog VO' | `dialogs.sql:3801`; `event_sets.sql:543` | ORIGINAL_DATA |
| 959 speaks screens at index 0, 2 and 4, i.e. lines 1, 3 and 5 | `dialog_screens.sql:9674-9682` | ORIGINAL_DATA |
| `d_tollana_TollanaTech.fsb` holds `Marsh_MsM_TollanTech_1`, `_3`, `_5`; the line numbers match 959's lines | handoff `Tollana_08` VO rows | ORIGINAL_DATA (rows); INFERRED (link) |
| 2151 ends "...Report to Captain Copplemann for more details when you reach the world." | `dialog_screens.sql:9682` | ORIGINAL_DATA |
| 2151 sits in dialog set 545 (map 2657, flag 0) beside Offer 2150 "Find Tollan weapon technology to use against the Straegis threat." | `dialog_set_maps.sql:1679-1681`; `dialog_screens.sql:9672` | ORIGINAL_DATA |
| Mission 568 step 1963 'Report to Captain Copplemann who is running the operation on Tollana.' | `mission_steps.sql:5205` | ORIGINAL_DATA |

"When you reach the world" means the briefing happens before Tollana.
Marsh's only seeded placement is `CmdCenter_Marsh` (world 68, spawn 344,
template 10 'Col Marsh (pet)', speaker 941, `event_set_id` NULL;
`entity_templates.sql:27`). Speaker 959 is a separate row from 941; this looks
like a duplicate speaker ID for one VO bank, not a second character.

Set 545 also carries a Jaffa variant of the same offer:

- 5682 Offer, Moh'katan: "Keep an eye on Op-CORE by working with them on
  Tollana" (`dialog_set_maps.sql:1685`; `dialog_screens.sql:25000`).
- 5684 flag 0, Moh'katan briefing: "...Copplemann is already there... Go.
  Copplemann waits." (`dialog_set_maps.sql:1683`;
  `dialog_screens.sql:25014-25034`).

So 568 has two faction-variant offers (Marsh, Moh'katan). Both read as made
off Tollana. No faction filter on the set maps separates them; every 545 map
row has empty faction and mission arrays.

## Q3. Which template or event set owns dialog set 545?

**Answer: none. Dialog set 545 is unbound in every seed. ORIGINAL_DATA
(absence, full scan).**

Scanned and empty for 545:

- Every content-chain `add_dialog_set` payload. The IDs present are 2794,
  2817, 3062, 3063, 3073, 3127, 3129-3131, 4751, 4817, 5149, 5151, 5159, 5161,
  5229, 5230, 5253, 5254, 5356, 5866, 6360, 6395-6399, 120001 and 120002.
- `entity_templates.interaction_set_id` and `static_interaction_sets`.
- `entity_interactions.sql`: one row only (35, template 43, interaction set
  map 3127).

Set 545 contents, all topic 'Finding Tollan Technology'
(`dialog_set_maps.sql`):

| Map | Line | Dialog | Flag | Speaker / content |
|---|---|---|---|---|
| 2656 | 1679 | 2150 | Offer | Blurb, Op-CORE side |
| 2657 | 1681 | 2151 | 0 | Marsh (959) briefing, event set 538 |
| 2659 | 1683 | 5684 | 0 | Moh'katan (945) briefing |
| 2660 | 1685 | 5682 | Offer | Moh'katan blurb, Jaffa side |
| 4958 | 4675 | 4213 | MWO | Rodent nest; discarded (`dialog_screens.sql`) |
| 4959 | 4677 | 2153 | Active | Captain Copplemann (1100), event set 548: "Welcome to what's left of Tollana... Repair Facility..." (`dialog_screens.sql:9686-9694`, `dialogs.sql:3805`) |
| 4960 | 4679 | 4214 | MWO | Fungus; put back |
| 6392 | 6937 | 4211 | MWO | Food duplication unit; player pulls "the core" (`dialog_screens.sql:17194`) |
| 6393 | 6939 | 2154 | TurnIn | Captain Copplemann (1100), event set 550: "the core matter duplicator" (`dialog_screens.sql:9696-9700`, `dialogs.sql:3807`) |

The event sets that exist (538, 548, 550) are per-dialog VO sets, not owners
of the dialog set. Template 48 (`CaptCoppleman`, Harset,
`entity_templates.sql:283`) has `event_set_id` 570, the players' default event
set. That has nothing to do with dialog set 545.

**Design intent (INFERRED):** set 545 is the whole 568 flow in one set.
It needs chain bindings to three kinds of actor:

1. The Harset offer actors: Marsh (maps 2656/2657) and Moh'katan (2659/2660).
2. A Tollana Copplemann, speaker 1100 (4959 Active, 6393 TurnIn). The
   ledger already reserves this as a new template; Harset template 48 must not
   be reused.
3. Three Tollana device props (4958, 4960, 6392 MWO).

Nothing in set 545 points at mission 569 or 570.

## Q4. Which static mesh fits the Food Duplication Unit (moniker 7558)?

**Answer: `HT-Props.HT-FoodUnit00`. The export exists (ORIGINAL_DATA). The
identity match is INFERRED. Fallback: `Em-Props.EM-ViewScreen02`.**

Context: no `entity_templates` row uses any `DN_Ob_Ms_Tollan` name ID (7558,
7595, 8008, 8062, 8482, 8415, 25932; 22 such monikers in total). The device
props have no seeded template at all.

| Candidate | Evidence | Label |
|---|---|---|
| `HT-Props.HT-FoodUnit00` | Top-level StaticMesh export in `CookedPC\Packages\HT-Props.upk` (291 StaticMesh exports). HT-Props is imported by 195 Tollana tiles. | ORIGINAL_DATA |
| (same) | The object itself is imported by no Tollana, Harset, Harset_Market, Harset_CmdCenter, Harset_StorageRm, Castle or Tollana_Curia tile. It is unplaced on every shipped map checked. | ORIGINAL_DATA |
| `Tol-Props00.*` | 17 StaticMesh exports (e.g. `Tol-HolographicKiosk00`, `Tol-IonCannon00`); no food or duplicator mesh. | ORIGINAL_DATA |
| Handoff `Tollana_11` RUNTIME_ASSET_USAGE | Its only StaticMesh is `GLB-RingTransporter00`; `Tollana_01`/`_05` have no food or duplicator hits. | ORIGINAL_DATA |
| Others seen | `Em-Props.EM-ComputerTower00`, `Em-Props.EM-DetectionDevice00`, `TC-Props.TC-Vending_machine00`, `TC-Props.TC-Console00`, `HT-Props.HT-Kiosk01`. Weaker name fits. | INFERRED |

**Known-good fallback: `Em-Props.EM-ViewScreen02` with
`GLB_Components.WorldObject_Small`.** This exact pair is seeded on template 173
(`Castle_CommsTerminal`, name ID 7720, `entity_templates.sql:491`), and the
mesh also serves templates 19, 37 and 310-313. The mesh is imported by five
Tollana tiles (`fff60002`, `fff70003`, `fff8ffff`, `fff9ffff`, `fffaffff`), and
Em-Props by 79. It is the safest choice for a first lab pass.

Risk on the primary: because `HT-FoodUnit00` is never placed, it is not proven
that the client streams HT-Props for a server-spawned entity before the mesh
is needed. HT-Props being imported by 195 Tollana tiles makes a load likely,
but that is unverified. Lab-verify the primary; ship the fallback if it fails.
Package case varies in the seeds (`Ga-Props` / `GA-Props`); UE3 names are
case-insensitive, so either spelling resolves.

## Q5. Do items 2343 and 2742 link to mission 568? Is either "the core"?

**Answer: no seed links either item to 568. ORIGINAL_DATA (absence). Neither
item is the "core". Which item the 2009 game granted is UNKNOWN.**

| Evidence | Source | Label |
|---|---|---|
| 2343 'Fragmented Tollan Device', "Fragments of Tollan technology.", flags 36864, container set {2} | `Items/Seed/items.sql:11433` | ORIGINAL_DATA |
| Neighbours 2273 'Scanning Device' and 2342 'Tollan Tech Scanning Device' share flags 36864 | `items.sql` | ORIGINAL_DATA |
| 2742 'Tech Fragment', "...integral to some really cool system...", flags 32768 | `items.sql:11731` | ORIGINAL_DATA |
| Neighbours 2743 Tollan Control Technology, 2744 Ion Core | `items.sql` | ORIGINAL_DATA |
| No `items_event_sets` row has `item_id` 2343 or 2742. Matches on `item_event_id` 2343/2742 are ID collisions | `items_event_sets` seed | ORIGINAL_DATA |
| No loot, reward, task or chain reference to either item. All other numeric hits are ID collisions (abilities, effects, dialogs) | full `db/resources/` scan | ORIGINAL_DATA |
| `mission_tasks` has no item column: (task_id, objective_id, award_xp, difficulty, is_enabled, task_type) | `mission_tasks.sql` | ORIGINAL_DATA |
| 568 step 1965 'Collect fragments of technology from the devices...' holds objectives 2311 and 2312 (2312 optional); both DO texts blank | `mission_steps.sql:5205-5211`; `mission_objectives.sql:5747,5749`; `texts.sql:16679,16681` | ORIGINAL_DATA |
| Tasks 2938 -> objective 2311; 2939 and 5389 -> objective 2312 | `mission_tasks.sql:6241-6245` | ORIGINAL_DATA |
| Dialogs 4211 and 2154 call the found object "the core" and "the core matter duplicator" | `dialog_screens.sql:17194,9696-9700` | ORIGINAL_DATA |
| No item is named "core matter duplicator". Other "core" items: 8921 'Tollan Computer Core', 2744 'Ion Core' | `items.sql` | ORIGINAL_DATA |

**Best fit (INFERRED):** 2343 'Fragmented Tollan Device' matches step 1965's
"fragments of technology" and sits in the scanning-device cluster used by
steps 1964-1965. It is the best candidate for objective 2311's collect step.
2742 belongs to the 2743/2744 control-tech cluster, which reads like later
Tollana content (1632 control-tech path), not 568.

The "core" is either an item that was never seeded or one of these items under
another name. The MWO dialog 4211 grants it at the food duplication unit, and
the TurnIn 2154 consumes it. Objective 2312 (optional, two tasks) is the
likely slot for it. Allocating a new item is a reviewed reconstruction under
D-T007/D-T008, not recovered data.

## Open gates

- No chain binds dialog sets 545, 567 or 570. Every actor answer above is
  INFERRED until a reviewed chain is authored.
- The 621 TurnIn prose (crystal/key) and step 4103 (address and code)
  disagree on the delivered item.
- `HT-Props.HT-FoodUnit00` load on a spawned entity is lab-untested.
- The 568 "core" item ID is unallocated; 2343 vs a new item needs a decision.
- No build label applies to any of this (ledger D-T003).
