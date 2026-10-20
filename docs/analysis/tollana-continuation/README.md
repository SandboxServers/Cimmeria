# Tollana continuation ledger

> **Title**: Tollana continuation ledger
> **Type**: Reference
> **Audience**: Campaign engineers and coordinators
> **Last updated**: 2026-10-17
> **Companions**: [Wave resume](RESUME.md), [W1 packets](w1-packets.md), [W1 frozen literals](w1-frozen.md), [manifest](manifest.md), [T-00 worknote](worknotes/T-00.md), [T-01 contract](evidence/T-01-source-map-contract.md)

T-00 evidence snapshot, 2026-10-14. Base: `d3efb7d41aad0b6008397f112d224ce7430d7acf`.
This ledger continues the [Tollana plan](../four-zone-planning/tollana.md) under the
[execution contract](../four-zone-planning/execution-contract.md).
No runtime restoration or client UAT is claimed.

## Evidence gate

The [source matrix](tollana-source-matrix.csv) contains exactly the 51 IDs in the
[planning census](../four-zone-planning/tollana-mission-census.csv). It records 51
verified `hold` dispositions because compatible mission chronology is unresolved.
`owner_directed_restoration_intent` separately preserves 23 local `restore` and
seven `cross-zone` candidates, with 21 conditional/lineage-held intents. These
are source-backed planning intentions; none authorizes activation or dispatch.
All 51 rows have `dispatch_status=BlockedEvidence`. The holds include all 16
formerly conditional side records, level-1 743/744/860/863 and disabled 1723.
The 51 records retain their original faction/story scope; none is declared dead.

Per-ID `district` and `district_evidence` quote named locations from cited
journal steps. They describe text-supported areas; exact current districts and
placements remain `UNRESOLVED` for T-01. A row without a named area explicitly
records that gap. `dependency` and `dependency_evidence` record cited journal
actions and planning gates separately from `UNRESOLVED` prerequisite mission
edges. Neither a journal sequence nor shared cast proves a compatible arc.

The mission graph agrees between the supplied ZIP and this seed: 240 steps,
259 objectives and 262 task triples, with no missing or extra triples. Of 51
mission rows, 50 have `is_enabled=true`; 1723 is false. **Every one of the 240
steps and 259 objectives has its enabled flag false.** These independent flags
must be reconciled against compatible client evidence before implementation;
T-00 does not authorize bulk enablement. The ZIP has no per-mission Build column.
Graph agreement does not establish build chronology or gameplay reachability.

The matrix embeds exact SQL paths and starting lines for mission, step,
objective, task and reward rows. `dialog_presence` checks direct
`dialogs.accepts_mission_id` joins and their screens/set maps: zero matches for
these 51 IDs. This is not proof that all relevant dialog prose is absent.
The ZIP dossier independently reports zero map-linked dialog/EventSet rows.
The exact JSON `mission_id` scan across content seed files found no references
for these IDs; no `tollana_*_chains.sql` exists at the base. This bounded scan is
not a semantic proof that indirect trigger paths cannot exist.

## Source and build boundaries

The source is `SGW_FINAL_DEVELOPER_HANDOFF.zip`, member
`WORLDS/RECOVERED/Tollana/02_MISSIONS.csv`, keyed by MissionID. Each row carries
`PROVEN_MISSION_MONIKER_WORLD_EXACT`, an exact moniker, resolved world 19 and
`ALIAS_EXACT` resolution. Those are the archive author's relation grades; T-00
verified the rows and seed joins, not the unavailable underlying SQLite query.
`SGW_FINAL_RE.sqlite` and `SGW_ALL_FINDINGS_ARCHIVE.sqlite` were not supplied.
Their absence limits proof and does not block documenting the evidence.

The independent `01_MAP_PACKAGES.csv` has 6,433 observations across builds
43485, 45032, 46694, 47977, 49486, 55124, 57050, 58674, 60130, 62429, 63682 and
QA4046. **None of those build labels may be assigned to a mission row.**
`04_KISMET_EVENTSET_ACTORS.csv` contains QA4046 ring sequences for EventSet
932 (`Tollana-fffa0002`), 933 (`Tollana-0003fffe`) and 934
(`Tollana-0003fff8`). These prove reported transport actor relations, not mission
NPC placement or a complete current map export. Current repository navigation
and collision artifacts `data/spaces/tollana.nav` and `tollana.occ` exist.
T-00 did not inspect or hash-match a cooked map; the subsequent
[T-01 checkpoint](evidence/T-01-source-map-contract.md) matched local 58674
and configured QA4046 root map packages to the archive. Quest coordinates and
navmesh reachability remain unverified.

`db/resources/Worlds/Seed/spawnlist.sql`, filtered by world 19, contains only
spawn 39/template 1 (`Tollana_DHD`) and spawns 155/156/template 3 (rings).
There is no mission cast in that static spawnlist slice. Historical dossier
spawn/encounter roots are zero. Neither observation rules out unexamined
indirect spawn mechanisms. The old `docs/content/zone-audit.md` Tollana section
calls these three NPCs and says no navmesh/mission records exist; those counts
are superseded by this snapshot. FZ-99 owns correction of that shared document.
`docs/content/external-data-analysis.md` remains an earlier narrative source,
not a replacement for build-qualified objective or dialog evidence.

## Cross-zone contract

Tollana owns both ends of 603, 611, 618, 621, 1626, 1632 and 1643. Chain ranges
remain 9000–9799 locally and 9800–9999 for Harset recipients, per the
[shared allocation](../four-zone-planning/id-allocation.md); T-00 allocates none.
The seed confirms the following actor identities, consistent with the HA-00b
early finding supplied by the coordinator. Identity verification is not final
HA-00b integration or permission to bind a blank objective to an actor.

| Mission | Text-supported Harset recipients | Evidence and remaining gate |
|---|---|---|
| 603 | Marsh, Nerus, Hansen | Steps 2013–2017/3023/4015; seal ration-card grant/consume and both-world conditions. |
| 611 | Nerus, Hansen, Baal | Steps 2059–2063/4027; seal relic and ration-card state. |
| 618 | Unresolved leadership | Step 2100 names Command Center leadership; objective 2438 is blank. BlockedEvidence. |
| 621 | Unresolved | Step 4103 names Harset; objective 4734 is blank. BlockedEvidence; T-08e also precedes phasing. |
| 1626 | Marsh and Moh'katan | First step 4838; later step 5169 names Copplemann without a world. Tollana inference needs confirmation. |
| 1632 | Moh'katan | Step 5170; seal control-tech grant/consume path. |
| 1643 | Moh'katan | Steps 4870/4872; preserve separate local research recipients in 4869. |

| World | Spawn/template | Tag |
|---|---|---|
| 57 | 300/212 | `Harset_Hansen` |
| 68 | 344/10 | `CmdCenter_Marsh` |
| 68 | 347/53 | `CmdCenter_Nerus` |
| 68 | 340/42 | `CmdCenter_Baal` |
| 68 | 343/54 | `CmdCenter_Mohkatan` |
| 68 | 345/48 | `CmdCenter_Copplemann` |

Tollana needs its own Copplemann template and tag. Step 1963 of mission 568
places him on Tollana; Harset template 48 has existing Harset chain bindings
6503/6504 and cannot serve as an unreviewed shared mission actor. T-01 reserves
the new identity. Mission 619 remains local: steps 2102/2827/3909/4072 name
Travell, Lethander and Svarog's officers, without a Harset destination.
Mission 601 investigates Lyala and reports to Harta; held DRK 879 is the Lyala
return. Their shared cast does not justify merging their progression.

### Conditional missions outside the 51-row census

1364 and 1366 have Harset monikers and stay outside this CSV so its census is
exact. Tollana owns their complete conditional T-05a/T-05b chains.

- **1364:** enabled level-1 seed mission; disabled steps 4054 (Royal Guard),
  4695 (Anat's tank), 4696 (return to Tollana/Lethander), 4697 (ten Jaffa
  staves), 4049 (deliver to Lethander). Hold on build chronology, tank grant,
  item IDs, kill credit and receiving conditions.
- **1366:** enabled level-13 seed mission; disabled steps 4071 (Machra),
  4058 (deliver symbiotes to Copplemann). Hold on build chronology, Machra's
  world, symbiote grant/consume and Tollana recipient identity. Mission label
  Tollana is supporting context, not proof of Machra's location.

Citations: `db/resources/Missions/Seed/missions.sql`, keyed by mission IDs
1364/1366; `mission_steps.sql`, keyed by the step IDs above; ownership in
[T-05a/T-05b](../four-zone-planning/tollana-work-packets.md).

## W1 findings (2026-10-17)

ARCH-W1 sealed the 568 item-less pilot and the phasing core from offline evidence only (lab use denied by the user). Evidence: [offline placement](evidence/T-01-offline-placement.md), [568 chain spec](evidence/T-03a-568-chain-spec.md), [W1 archaeology](evidence/T-archaeology-w1.md), [phasing design](evidence/phasing.md). Every row is CIMMERIA_RECONSTRUCTION.

| Finding | Packet |
|---|---|
| Copplemann's 568 dialogs (2153 offer, 2154 turn-in) and the food-unit dialogs (4211, 4213, 4214) are zero-button; a close sends `-1`, so accept and turn-in fire on any close. | T-03a1 |
| Template 48 (Command Center Copplemann) is bound by Harset chains 6503/6504; Tollana needs its own template 420. The three units share prop template 421. | T-02a |
| Offline navmesh placement near gate 4 on `tollana.nav` (component 89) gives a primary set and a fallback set; the T-02a nav test picks between them. | T-02a |
| `complete_objective` auto-completes the mission when every non-optional active objective is done, so 2311 must never be completed directly; the Core double-advances instead. | T-03a2, T-03a3 |
| Dialog binds are per player and rebuilt empty on each world entry, so restore chains on `player_loaded Tollana` are required. | T-03a1 |
| Mission state is on `CellEntity.missions` and is testable through the real executor (`mission_681_686_backstop.rs` rig), not only at resolve level. This corrects the earlier D-TW11 premise. The `open_dialog_id` double-click gate is the one thing the rig cannot cover (T18, UAT). | T-03a2..4 |
| Phasing fits as an AoI membership filter in `compute_player_aoi`; leaving AoI already sends `LeftAoI`. The state lives in `CellEntity.extensions`, so `entity_struct.rs` is not touched. | PH-01a, PH-01b |
| Collision rescan across seven refs (origin/main and six content branches): no hit for chains 9001-9018, templates 420-421, spawns 19001-19010 or the four tags. | all |

## Decisions and packet status

| Decision | Status | Reason |
|---|---|---|
| D-T001 preserve core intent | Evidence recorded | 23 local and seven cross-zone intents preserved separately; all verified dispositions hold. |
| D-T002 hold unresolved chronology | BlockedEvidence | All 51 rows lack compatible chronology; 21 were already held, including placeholder 860 and disabled 1723. |
| D-T003 build provenance | BlockedEvidence | Mission CSV omits Build; map builds cannot supply it. |
| D-T004 staged Curia | Owner decision inherited | World-19 staging is CIMMERIA_RECONSTRUCTION, not a recovered interior; T-01/T-08e required. |
| D-T005 recipient separation | Evidence recorded | Tollana owns both ends; blank 618/621 recipient objectives stay unresolved. Proposal only: 618 → Moh'katan, 621 → Ba'al. |
| D-T006 bounded active candidate | Specialist and mission review | 568 first; 569/570 held (T-03b/c). |
| D-T007 reconstruction authority | Owner decision inherited | O14-03 permits reviewed bounded inferred links/placements with provenance. |
| D-T008 individual IDs | Extended 2026-10-17 | W1 claims chains 9001-9018, templates 420-421, spawns 19001-19004 (19005-19010 reserved) after the seven-ref no-collision scan; no dialog or item id allocated. |
| D-T009 lab and close-out | Superseded for W1 | The user denied all agent lab use on 2026-10-17; every in-client check is UATPending for the user's manual UAT. |
| D-TW1 item-less pilot | Proposed | No repair-fragment item until Harset HA-1360a ships a possession and consume primitive. |
| D-TW2 accept on any close of 2153 | Proposed | The dialog has zero buttons; a Decline button is a BlockedDecision. |
| D-TW3 Core double-advance; Nest credits 2312 | Proposed | Avoids the 2311 auto-complete trap. |
| D-TW4 enabled flags untouched | Proposed | Loaders ignore `step_enabled`; U7 checks the journal. |
| D-TW5 no faction gate | Proposed | No evidence of one for 568. |
| D-TW6 template 420 keeps event_set 570 | Proposed | Matches template 48's row. |
| D-TW7 ViewScreen02 placeholder mesh | Proposed | No food-unit mesh identified offline; swap in UAT P10. |
| D-TW8 primary placement, respawn 30, setval floor 19010 | Proposed | F2-B fallback is used wholesale if the nav test fails. |
| D-TW9 PhaseState in `extensions` | Proposed | Keeps the 850-line `entity_struct.rs` untouched. |
| D-TW10 PH-01 decision-independent core only | Proposed | `phase_visible` treats players as unphased (D-PH1 default, one line to reverse). |
| D-TW11 resolve-level shapes plus an executor rig | Proposed | Corrected premise; T18 to UAT. |
| O1 568 reward | BlockedDecision | Owner; ships with no reward. |
| Decline button on 2153 | BlockedDecision | Owner design call; a button strands the accept. |
| D-PH1..D-PH7 | BlockedDecision | Phasing: players phased (rec. no), show-only (rec. yes), persistence table, per character, spawn column, wire change (rec. no), 621 vault id 2766. [phasing.md](evidence/phasing.md). |

| Packet | Status | Next evidence or dependency |
|---|---|---|
| T-00 | Integrated | Evidence packet only: f58bc29f1 + fixer b105967cc. |
| T-01 | Integrated (offline) | Offline placement in [T-01-offline-placement.md](evidence/T-01-offline-placement.md); in-client checks are UAT P1-P9. |
| T-02a | Integrated; UATPending (W1a pilot) | [w1-packets.md](w1-packets.md#t-02a--copplemann-and-food-unit-cast-pilot) |
| T-03a1 | Integrated; UATPending (W1b) | Done in W1b-INT; in-client checks in the user UAT. |
| PH-01a | Integrated; UATPending (W1b) | Done in W1b-INT; in-client checks in the user UAT. |
| T-03a2 | Integrated; UATPending (W1c) | Done in W1c-INT; in-client checks in the user UAT. |
| PH-01b | Integrated; UATPending (W1c) | Done in W1c-INT; in-client checks in the user UAT. |
| T-03a3, T-03a4 | Integrated; UATPending (W1d) | Done in W1d-INT; in-client checks in the user UAT. |
| 568 UAT (U1-U7, T18, P1-P10) | UATPending | The user, after W1d-INT. |
| Item-backed 568 | BlockedDependency | Harset HA-1360a. |
| T-03b, T-03c | BlockedEvidence | 569/570 direction, combat and item contracts. |
| T-04–T-11 | BlockedEvidence | Mission chronology. |
| T-05a/T-05b | BlockedEvidence | 1364/1366 lineage and grant/recipient contracts. |
| T-08e, T-09 | BlockedDependency | Staging engine design; vault placement. |
| T-11s | BlockedEvidence | No side child admitted. |
| T-12a–c/T-13 | BlockedDependency | CB-N05a and authored catalog proof. |
| PH-02, PH-05 | BlockedDependency | PH-01b; PH-01 to PH-04. |
| PH-03, PH-04 | BlockedDecision | D-PH3/D-PH4; D-PH5. |
| T-14 | BlockedDependency | Campaign implementation, review and the user's UAT. |

## Handoff

[Signed manifest](manifest.md) freezes the T-00 evidence snapshot; [T-00 worknote](worknotes/T-00.md) and [T-01 worknote](worknotes/T-01.md) record earlier checks. W1 is sealed in [w1-packets.md](w1-packets.md) with every literal in [w1-frozen.md](w1-frozen.md); the [ID register](tollana-id-register.csv) and [placement register](tollana-placement-contract.csv) carry the W1 claims. [RESUME.md](RESUME.md) holds the dispatch order. Nothing has run in a client: native travel, the mission journal, dialogs and reachability remain untested until the user's UAT.