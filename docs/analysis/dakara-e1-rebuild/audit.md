# Dakara_E1 Rebuild: Audit Of What Exists

> Type: reference. Audience: Claude Code coordinator, packet workers and reviewers.
> Written 2026-10-06 against `main` @ `ddd549797`. Companions: [ledger and decisions](README.md), [work packets](work-packets.md), [Class Start v6 ledger](../class-start-v6/README.md), [Harset placement method](../harset-rebuild/placements/METHOD.md).

This is a static audit of worlds 61 `Dakara_E1` and 62 `Dakara_E1_StoryRm`: the seed under `db/resources/`, the server's space and navmesh data, the owner's client copy, and the external handoff archive's `WORLDS` directory. Nothing was built, no test ran, and no client or lab tool was launched.

## Sources And Labels

| Label | Meaning |
|---|---|
| CLIENT | Read in this session from the client's own files (cooked maps under `Working/SGWGame/CookedPC/Maps`, cooked data PAKs under `Working/SGWGame/SourceCache.en-us`) with the repo's readers `tools/extract_actors.py` and `tools/kismet_extractor.py`, or by opening the PAK entry. The strongest evidence available here. |
| SEED | A row in `db/resources/**`. The seed is Project Giza's reconstruction from the client's cooked data ([rules-and-gotchas](../../agents/rules-and-gotchas.md)); it is never CME server data. Where a row was also checked against the client PAK the row says so. |
| HANDOFF | A claim in the external archive, cited by its path inside the archive. Each one used below carries a verdict: CONFIRMED, CONTRADICTED or UNVERIFIABLE here. |
| PROJECT | A decision of this project, not recovered data (`PROJECT_FINAL_S2C`, `PROJECT_FINAL_RECONSTRUCTION` in the Class Start v6 vocabulary). |

Seed facts were queried from a recent load of `db/database.sql` and are cited by seed file. Coordinates are BigWorld metres, Y up; UE3 to BigWorld is `(ue.y, ue.z, ue.x) / 100`.

The archive is the "SGW Final Complete Reconstruction" handoff (`REPORTS/00_FINAL_SUMMARY.txt`: script version `SGW_FINAL_COMPLETE_RECONSTRUCTION_2026-10-01_V1`, completed 2026-10-02). The extract holds 615 files, 24 recovered-world folders and 87 missing-world folders, which matches the developer-handoff ZIP the main session reviewed on 2026-10-02 ([project memory](../../../.claude/agent-memory/main-session/project_final_re_bundles.md)); the ZIP was not re-hashed here. Its `00_README.md` names `SGW_FINAL_RE.sqlite`, `SGW_ALL_FINDINGS_ARCHIVE.sqlite` and `SOURCE_INDEX/`; none of them is in the extract.

## Headline

1. **The story is there, the world is not.** Twelve missions belong to Dakara_E1 (1570 from its third step, and 1645-1655). Every step, objective, dialog line, mission item and name string for them is in the seed and in the client. No NPC position, hostile, encounter, region, respawn point or script for them exists anywhere: not in the seed, not in the client map, not in the archive.
2. **The archive adds almost nothing the seed and the client do not already hold.** Its Dakara_E1 folder is a per-build file manifest, a mission list that is missing four of the twelve missions, and actor rows read from the same cooked maps the repo's tools read. Its spawn, encounter, player-start and dialog-to-script files are empty.
3. **The recovered arc is not a level-1 start.** Its missions are level 3 to 5, and the first Dakara beat (mission 1570, step 4906) greets a Jaffa player with "Welcome back, my friend" after the SGC attack. Starting Free Jaffa on Dakara at level 1 is a project decision (`PROJECT_FINAL_S2C`), so the first minutes on Dakara have no recovered content at all.

## Layer By Layer

### Client map and packages

| Fact | Evidence |
|---|---|
| `Maps/Dakara_E1` holds 401 `.umap` tiles plus `Dakara_E1_MapData.upk` (164 MB). `Maps/Dakara_E1_StoryRm` holds 2 tiles plus its `MapData.upk`. | CLIENT |
| All 401 + 2 tiles are byte-identical (SHA-256) to the archive's `QA4046` rows, and to no earlier build's rows. | CLIENT against HANDOFF `WORLDS/RECOVERED/Dakara_E1/01_MAP_PACKAGES.csv` and `.../Dakara_E1_StoryRm/01_MAP_PACKAGES.csv` |
| No `Dakara`, `Dakara_E2`, `Dakara_E3` or `Dakara_Superweapon` map directory exists in the client. | CLIENT (`CookedPC/Maps` has 22 directories; two are Dakara) |
| No `PlayerStart` actor on either map. | CLIENT; agrees with [map arrival points](../../../.claude/agent-memory/main-session/reference_map_arrival_points.md) and HANDOFF `12_PLAYER_STARTS.csv` (empty) |
| World 61 actor census: 23,011 StaticMeshActor, 1,132 PrefabInstance, 271 Trigger, 51 TriggerVolume, 25 InterpActor, and at least 8,022 `SGWSpecCoverNode`. Bounds about x -558..663, z -534..728. | CLIENT (`extract_actors.py` JSON mode; the cover-node count is from its summary mode, which reported parse errors on 99 of the 401 tiles, so it is a floor) |
| The StoryRm map is **one** furnished tent interior (x 50..85, z 10..45, floor y 0): braziers, seven skeletal meshes, one blocking volume. | CLIENT |

### Server space, navmesh and cell config

| Fact | Evidence |
|---|---|
| World 61 is a shared startup space: `entities/cell_spaces.xml:18`, `entities/spaces.xml:18` (`Instanced="false"`, bounds ±2000). World 62 is instanced: `entities/spaces.xml:19`, `worlds.flags = 1`, and the client's `CookedWorldInfo.pak` entry `_62` has `Flags="1"`. | SEED, CLIENT |
| `data/spaces/dakara_e1.nav` (7.3 MB, tiled, 699 components) and `dakara_e1_storyrm.nav` (6 components) exist, built by NA26 on 2026-09-25 from the cooked maps. Both worlds run `navmesh_mode = 'advisory'`; nobody has walked either under containment. Occluder files exist for both. | `db/resources/Worlds/Seed/worlds.sql:141-151`, `data/spaces/README.md` |
| Worlds 24 `Dakara`, 30 `MissionTestDakara`, 63, 64 and 65 have `worlds` rows and no space, map or navmesh. | SEED; CLIENT for the missing maps |
| `cover_sets` has no row for world 61 (rows exist only for worlds 8, 12 and 1300). | SEED |

### Stargate, DHD, addresses and rings

| Fact | Evidence |
|---|---|
| Gate 25 `Dakara E1`: address 8-36-31-17-4-10, origin glyph 13, world 61, (96.174, -15.164, 253.206), yaw 3.072, event set 10013, no `arrival_*` pin. The row equals the client's `CookedDataStargates.pak` entry `_25` field for field. | `db/resources/Worlds/Seed/stargates.sql:85`; CLIENT |
| The gate prefab's static mesh sits at UE (25263.5, 9633.7, -1253.0), which is (96.3, -12.5, 252.6): the row is the prefab, within a metre. | CLIENT; HANDOFF `13_TRAVEL_TRANSPORT.csv` CONFIRMED |
| Gate volume: `point_sets` 1005 `Dakara_E1.Stargate`, cylinder radius 2.5, flags 3, one point at (96.147, -15.164, 253.706). The gate row itself is inside this volume. | `db/resources/Events/Seed/point_sets.sql:93`, `point_set_points.sql` |
| DHD: `spawnlist` 38 `Dakara_E1_DHD`, template 1, (98.087, -16.769, 237.335). Template 1 carries `INT_DHD` since DA-07, so it opens the dialling window. | `db/resources/Worlds/Seed/spawnlist.sql:379`; [gate-travel.md](../../gameplay/gate-travel.md#dhd-interaction) |
| A new character's address book is empty; the only grant paths are a committed gate arrival and the content action `grant_stargate_address`. | [gate-travel.md](../../gameplay/gate-travel.md#address-unlock-on-arrival) |
| The SGC worlds have no DHD prop (worlds 58 and 86 have no template-1 spawn), and gate 27 `SGC W1` is about 17 m under the floor. A player who gates to the SGC today cannot dial out. Omega Site (gate 5, world 18) has a DHD (`spawnlist` 41), five ring regions and no respawner. | SEED; [map arrival points](../../../.claude/agent-memory/main-session/reference_map_arrival_points.md) |
| `ring_transport_regions` has no row for worlds 61 or 62. | SEED |
| The map has two `GLB-RingTransporter00_Pf0` prefabs, at (-139.3, -16.4, 49.0) and (358.0, -16.2, 102.1). Their Kismet is "Designer 0: Outgoing Ring Transport!". Each stands under one of the two Ha'tak prefabs, so these are the mission 1652 devices, not a travel network. | CLIENT; HANDOFF `13_TRAVEL_TRANSPORT.csv` CONFIRMED (30 of 30 QA4046 rows match) |

### Respawners and start point

| Fact | Evidence |
|---|---|
| On `main` world 61 has no respawner; world 62 has respawner 25 at (71, 0.05, 30) in the tent interior. | `db/resources/Worlds/Seed/respawners.sql:196-200` |
| PR #1273 (CS-02, not merged) adds respawner 610 `Dakara Gate Plaza` and the `SGU_FREE_JAFFA` start, both at (100, -17.4, 230) on navmesh component 279, 7.6 m from the DHD, with a live-DB navmesh guard. | `origin/cs6/cs02-start-profiles`: `respawners.sql`, `char_creation.sql`, `crates/cell-world/src/cell/dakara_start_tests.rs` |
| The client's strings name one Dakara_E1 respawner: `DN_Respawner_DakaraE1_MedTent` "Med Tent". No coordinate is recorded. Rak'nor's line in dialog 6110 says Bra'tac "has moved his command tent just to the east of the Stargate, next to the healing tent", which is the only recovered statement of where the command tent and the Med Tent are. | `db/resources/Texts/Seed/texts.sql:36022`; `db/resources/Dialogs/Seed/dialog_screens.sql` (dialog 6110, screen 102199) |

### NPC templates and spawns

| Fact | Evidence |
|---|---|
| World 61 has one spawn (the DHD). World 62 has none. No hostile, no friendly, no vendor. | SEED |
| Templates exist for Bra'tac (59 `Bra'tak`, speaker 2323, level 50, faction 1) and Moh'katan (54 `Moh'Katan`, speaker 945; shared with Harset). None exists for Loth'ta, Rak'nor, the Jaffa Captain, Ba'al's hologram, Free Jaffa defenders or any Dakara hostile. Template 35 `SGC Ba'al Jaffa` (level 1, faction 10) is the nearest hostile. | `db/resources/Entities/Seed/entity_templates.sql:35` (35), `:71` (54), `:123` (59) |
| Name strings exist for the cast: `DN_npc_mg_Bratac_DakaraE1`, `DN_npc_mg_Mohkatan_DakaraE1`, `DN_npc_int_Lothta_DakaraE1`, `DN_npc_int_Raknor_DakaraE1` "Rak'nor", `DN_npc_int_JaffaCaptain_DakaraE1`, `DN_npc_int_Baal_DakaraE1_Hologram` "Baal (Hologram)", `DN_npc_esc_Jaffa_DakaraE1_Escort` "Free Jaffa Warrior" (ids 26717-26723; four have empty text). Speakers 771/2323 Bra'tac, 945/2499 Moh'katan, 781/2335 Loth'ta. Speakers 2956 (dialogs 6110/6111, the gate greeter) and 2959 (6106/6107, the gate commander) have empty names; reading them as Rak'nor and the Jaffa Captain is an inference from objective 5936 and step 4911. | `db/resources/Texts/Seed/texts.sql:35976-35988`; `db/resources/Dialogs/Seed/speakers.sql` |
| Object strings name the interactables: tent flaps `DN_ob_DakaraE1_sc_TentFlap_ToCommand` "Command Tent Entrance", `_ToMohkatan` "Moh'katan's Tent Entrance", `_FromCommand` "Return to Dakara", `_FromMohkatan`; `_int_DropLoc` "Drop Location"; `_int_SG18Corpse01..03` "Maj. Louis", "Lt. Nguyn", "Lt. Waters"; `_int_CommandTerminal`; `DN_ob_DakaraE1StoryRm_int_MohkatanTerminal`; `_int_RingControl`; `_int_TurretPowerSupply` "Power Supply"; `_int_Vocuum` "Vocuum" (ids 26724-26737). None has a template or a position. | `texts.sql:35990-36020` |
| HANDOFF `15_SPAWN_ENCOUNTER_ROOTS.csv` and `16_SPAWN_ENCOUNTER_BINDINGS.csv` are empty, and `HANDOFFS/SERVER_ENTITY_PROTOCOL_HANDOFF.md` says spawn logic "belongs to the server/entity layer, not static UMAP NPC placement". | HANDOFF, CONFIRMED by the client map having no spawn actors |

### Missions

All twelve are `mission_label = 'General'`, story missions, `reward_xp = 0`, `reward_naq = 0`, no reward group, no script name, and no content chain (`db/resources/Content/Seed/` has no file or row for world 61, mission 1570 or missions 1645-1655). The client's `CookedDataMissions.pak` has entries `_1570` and `_1645` to `_1655`; the opening of 1645 and 1649 was compared with the seed (level, label, step ids and step text) and matches. Every task in the cooked data is `TaskType="1"` with no parameters, so the client carries no objective semantics (kill counts, targets, regions).

The mission order comes from the client's mission name strings `DN_ms_A00_DakaraE1_sgc_01SG18` to `..._12Shutdown` (`texts.sql:34567-34593`). "Giver" below is read from the speakers of the mission's offer dialog; nothing records an offer rule or a prerequisite.

| Seq | Id | Name | Lvl | Giver and cast | Steps (ids in order) | Shape | Dialog set |
|---|---|---|---|---|---|---|---|
| 01 | 1570 | SG-18 | 3 | Hammond at the SGC; on Dakara Bra'tac and Moh'katan, then Loth'ta | 4641, 4642 (SGC: Harriman dials, enter the gate); 4906 command tent, 4904 Loth'ta, 4902 search three sites, 4901 collect dog tags, 4905 radio Hammond, 4903 report to Bra'tac | Talk, search, collect. No hostile is required. The Dakara dialogs come in pairs (5810/5811, 5812/5813, 5815/5816); 5811 and 5813 address a Jaffa, 5810 and 5812 one of the Tau'ri. | 1656 (27 rows) |
| 02 | 1645 | Withdrawal Orders | 3 | Moh'katan and Bra'tac (5807); Loth'ta (5808) | 5168, 4907 | Take item 6779, deliver it. One optional follow-the-path objective with four tasks. | 1832 (4) |
| 03 | 1646 | Moh'katan's Scouts | 3 | Moh'katan (5819, 5820) | 4908 (five drop locations), 4909 | Interact with five props (5821). | 1833 (8) |
| 04 | 1647 | Enemy at the Gates | 3 | Moh'katan (5822, 5827); Jaffa Commander (5823) | 4911, 4912, 4910, 4916, 4915, 4913, 4917, 4914 | Deliver supplies, defend the Western Gate, radio, defend the Eastern Gate. **Combat.** | 1834 (9) |
| 05 | 1648 | Loth'ta's Withdrawal | 3 | Bra'tac (5828, 5830); Loth'ta (5829) | 4919, 4918 | Talk, carry item 6781. | 1835 (4) |
| 06 | 1649 | Moh'katan | 4 | Bra'tac (5832, 5838); Teal'c by radio (5833) | 4923, 4921, 4922 (hack terminal, search drops, decode), 4920 | Talk, radio, minigame, search, item use. | 1836 (10) |
| 07 | 1650 | Confront Moh'katan | 4 | Bra'tac (5839, 5841, 5842); Ba'al hologram scene (5840) | 4927, 4924, 4926, 4925 | Tent scene, search, item 6787. | 1837 (6) |
| 08 | 1651 | Renewed Attack | 4 | Bra'tac (5843); Hammond and Carter (5844, 5845) | 4932, 4930, 4928, 4931, 4929, 5171 | Gate to the SGC and back with generators (6784). | 1838 (4) |
| 09 | 1652 | Fireball | 4 | Bra'tac (5846) | 4933, 4934 | Destroy two Ha'taks with overloaded generators (6785). | 1839 (4) |
| 10 | 1653 | Superweapon | 5 | Bra'tac (5848, 5849) | 4936, 4938, 4937, 4935 | Reach the courtyard, destroy two turret power supplies, optional vocuum, radio. **Combat.** | 1840 (3) |
| 11 | 1654 | Aftermath | 5 | Bra'tac (5850/5851); Hammond (5852/5853) | 4939 | Gate to the SGC, speak to Hammond. Leaves the zone. | 1841 (5) |
| 12 | 1655 | Shutdown | 5 | Woolsey (5854 Human, 5855 Jaffa); Hammond (5856/5875); Harriman | 4943, 4944, 4941, 4940, 4942 | SGC, then Omega Site. Not on Dakara. | 1842 (7) |

Step and objective rows: `db/resources/Missions/Seed/mission_steps.sql`, `mission_objectives.sql`, `mission_tasks.sql`. `step_enabled` is false on every row, as it is on every working Cellblock mission; the loader never reads it ([Harset audit](../harset-rebuild/audit.md)).

Two things in the dialogs decide how the arc can start:

- Dialog 5811, the Jaffa branch of step 4906, opens "Welcome back, my friend. Did Teal'c hear our warning?" and the player answers that Ba'al's forces invaded the Tau'ri base. Dialog 6110, the greeter's Jaffa version, has the player say "Ba'al's forces invaded as I was delivering my warning". Dialog 5855, the Jaffa branch of 1655, calls the player the "liaison to the Jaffa on Dakara". In the client's data the Free Jaffa character reaches Dakara from the SGC, after the SGC_W1 missions.
- Dialog 5807 (1645's offer) has the player say "Master Bra'tac, I need to speak with Loth'ta already", and 5819 (1646's offer) opens "The death of SG-18 is very serious". Both assume 1570 has been played.

### Dialogs and dialog sets

| Fact | Evidence |
|---|---|
| Dialog sets 1832-1842 (one per mission 1645-1655) hold 64 `dialog_set_maps` rows; set 1656 (1570) holds 27. Every dialog they name has its screens seeded. Flags follow the story-mission ladder: bit 23 available (a `DUIST_DefaultBlurb` with Accept and More Info), bit 24 active, bit 25 turn-in, bit 30 world object. | `db/resources/Dialogs/Seed/dialog_set_maps.sql`, `dialog_screens.sql`; [interaction-flags.md](../../content/interaction-flags.md) |
| Four rows in sets 1832-1842 have a NULL dialog (6874, 6760, 6770, 6892), and two in set 1656 (6197, 7156); since CA02 these are flag-only binds. | SEED |
| Radio beats use dialogs with a named NPC speaker and no NPC present: 5815/5816 (Hammond), 5824 (Moh'katan), 5833 (Teal'c). `display_dialog` from a non-interact chain can only show a dialog whose every screen has speaker 0. | SEED; [dialog authoring rules](../../../.claude/agent-memory/mission-systems-advisor/dialog-chain-authoring-rules.md) |
| No dialog in sets 1832-1842 carries an event set, and HANDOFF `03_DIALOG_EVENTSET_SCRIPT.csv` is empty. | SEED; HANDOFF CONFIRMED |

### Regions and triggers

| Fact | Evidence |
|---|---|
| The only seeded region on world 61 is the gate volume (point set 1005 and its legacy `generic_regions` twin). | SEED |
| The client names six discovery areas: Command Tent, Jaffa Command, Moh'katan's Tent, Naquadah Repository, Stargate Plaza (empty text), Dakara Superweapon Courtyard (`string_DakaraE1_Discovery_DisplayName_*`, ids 27039-27043 and 27057). No bounds exist for any of them. | `texts.sql:36658-36672`, `:36692` |
| The map's 51 TriggerVolumes are prefab parts of tents (`JF-Tent00`, `JF-MilitaryTent00`, `JF-Tent02`, `HB-Tent_*`) and one merchant tent (`GA-MerchantTent00`, at (306.7, -15.7, 129.7)). Their Kismet is sound and cloth response only. The other 271 Triggers belong to bushes. None is mission logic. | CLIENT; HANDOFF `14_TRIGGER_ACCESS.csv` CONFIRMED (322 of 322 QA4046 rows match) |
| Tent volumes fall in six groups (candidate landmarks, not placements; x, y, z ranges): the nearest, 200-280 m from the gate, at (270..320, -18, 80..165) with the merchant tent among them; (440..465, -10.5, 85..95); (544..594, -7.1, 252..274); a camp at (525..607, -10.5, 371..477); a group at (388..485, -11..-4, 497..602); and two camps at negative x, (-457..-379, -10.7, 185..258) and (-302..-226, -9.6, 480..559). Which of these are the mission texts' "Eastern tents", "Western Gate" and "Eastern Gate" is not recorded. | CLIENT |

### Loot

No loot table references Dakara, no Dakara template exists to carry one, and the seed has 14 loot tables in total. The nine mission items exist: 6778 Dog Tags, 6779 Withdrawal Orders, 6780 Encrypted Message, 6781 Message to Bra'tac, 6782 Copy of Moh'katan's Orders, 6783 Decoded Message, 6784 Naquadah Generator, 6785 Overloaded Naquadah Generator, 6787 Downloaded File, all in the mission bag (`db/resources/Items/Seed/items.sql`). 6786 Letter to Daniel Jackson belongs to 1655.

### Kismet sequences and event sets

| Fact | Evidence |
|---|---|
| Event set 10013 `Dakara E1 Stargate`: events 6100-6113 on `Dakara_E1.Main_Sequence.Prefabs.GLB-Stargate_Prefab_Seq`. | `db/resources/Events/Seed/event_sets.sql:1355`, `sequences.sql` |
| Event sets 1194 and 1195: events 6000 and 6001 on `...GA-Hatak00_Pf0_Seq` and `...GA-Hatak00_Pf0_Seq_0`. Nothing references either set. The map sequences are "Designer 0: Unhide Hatak" and "Designer 1: Hatak Explosion"; the Ha'tak meshes hang at (367.2, 359.6, 136.1) and (-111.7, 353.9, 78.1). | `event_sets.sql:1087` (1194), `:73` (1195); CLIENT |
| The map's gameplay Kismet is the gate, the two Ha'taks and the two ring transporters. Everything else (600 sequences) is weather, music, foliage and tent ambience. The StoryRm has weather and music only. | CLIENT (`kismet_extractor.py --survey`) |

## Handoff Claims Used, And Their Verdicts

| # | Claim (path inside the archive) | Verdict | Basis |
|---|---|---|---|
| 1 | Dakara_E1 has 401 map packages per build across five builds (`WORLDS/RECOVERED/Dakara_E1/01_MAP_PACKAGES.csv`) | CONFIRMED for QA4046 | 401 of 401 client tiles match by SHA-256. Earlier builds cannot be checked: the archive has hashes, not files. |
| 2 | "Missions: 8" for Dakara_E1 (`00_WORLD_CONTENT_DOSSIER.md`, `02_MISSIONS.csv`) | **CONTRADICTED** | The seed and the client hold twelve. The file omits 1649 and 1653, and files 1570 only under `SGC_W1` and 1654 under `Castle_CellBlock` (as `DN_ms_A00_Cellblock_opc_13Aftermath`). |
| 3 | Mission rows are `PROVEN_MISSION_MONIKER_WORLD_EXACT` | **CONTRADICTED** as a description of the method | The cooked mission entries carry no name string id. The four omissions and mis-filings are exactly the four `DN_ms_A00_DakaraE1_*` strings whose text is empty in the client (`01SG18`, `06Mohkatan`, `10Superweapon`, `11Aftermath`), which is what a join on display text would produce. The eight rows it does list are right. |
| 4 | Event set 1194 binds both Ha'tak sequence roots (`04_KISMET_EVENTSET_ACTORS.csv`) | PARTLY CONFIRMED | Both roots and their actors exist in the client map. The seed binds the second root (`..._Seq_0`) to event set 1195, not 1194. |
| 5 | 150 travel and transport rows (`13_TRAVEL_TRANSPORT.csv`) | CONFIRMED for QA4046 | 30 of 30 rows match client actors by class and position. |
| 6 | 1,081 trigger and access rows (`14_TRIGGER_ACCESS.csv`) | CONFIRMED for QA4046 | 322 of 322 rows match. 947 of the 1,081 rows are foliage triggers. |
| 7 | One trigger is a `VENDOR` system context | UNVERIFIABLE as a system | The volume exists and its prefab is a merchant tent. Nothing in the client or seed makes it a vendor. |
| 8 | No player starts, no spawn or encounter roots, no dialog-to-script links, no cinematic paths (nine empty files) | CONFIRMED | The client map has none of those actor classes, and the seed has no such rows. |
| 9 | Dakara_E1_StoryRm exists from build 62429 only | UNVERIFIABLE here | Only the QA4046 client is on hand. |
| 10 | `Dakara` (24), `Dakara_E2` (63), `Dakara_E3` (64), `Dakara_Superweapon` (65) are referenced worlds with no recovered map (`WORLDS/MISSING_UNRECOVERED/*/00_WORLD_EVIDENCE.md`) | CONFIRMED | Seed `worlds` rows exist; the client has no such map. Their asset lists (3,377 to 3,441 rows each) are name matches on "dakara", graded `CANDIDATE_NOT_PROVEN` by the archive itself. |

Claims from the earlier [handoff pack v1.2](../sgw-handoff-pack-v1.2/README.md) Dakara workbooks that this plan relies on or rejects:

| Claim | Verdict | Basis |
|---|---|---|
| "Starter begins: 1645 / World61" (`SGW_Dakara_Free_Jaffa_Starter_Dev_Master_v1`, sheet 24) | **CONTRADICTED** by the client's own data | Sequence 01 of the Dakara arc is 1570 (`DN_ms_A00_DakaraE1_sgc_01SG18`), 1645's dialog assumes it, and the Jaffa dialogs assume an SGC start. |
| "Planets & Locations calls Dakara_E1 the starter instance for Jaffa" (sheet 21) | UNVERIFIABLE here | That workbook is not in the repo or the archive. The pack marks the Dakara start `USER-CONFIRMED`, a project target. |
| "SGU Human SG-18 Dakara visit: do not grant to Free Jaffa" (sheet 23) | **CONTRADICTED** | Mission 1570's Dakara dialogs include Jaffa-addressed versions (5811, 5813). |
| Dialog sets 1832-1842 map to 1645-1655 (sheet 05) | CONFIRMED | 64 seed rows, same ids and flags. |

## What Could Not Be Checked

- The Rust extractors (`archetype_census`, `nav_inspect`, `obj_slab`, `extract_kismet`) were not built, so no prefab landmark has a name-plus-floor check and no candidate point has a navmesh component. Packet DK-02 does this.
- No earlier client build is on hand, so every per-build claim in the archive other than QA4046 is unverified.
- Whether a gate arrival on world 61 (which lands on the gate row, inside volume 1005) behaves well in a client. No Dakara arrival has been observed.
- Whether the client shows an NPC-speaker dialog with no speaker entity (the radio beats).
- The cover data: the client ships `Cache/covernodes_*.pak`; whether they hold Dakara_E1 sets was not opened.
- Nothing was run, built or tested.
