# STARGATE WORLDS RECONSTRUCTION — MASTER SOURCE / HANDOFF

**Snapshot:** 17 September 2026  
**Purpose:** portable continuity source for reconstructing the missing Stargate Worlds server/gameplay layer and repairing/polishing the surviving QA client.  
**Instruction:** In every new chat, read this file before doing SGW work.

## 1. Project goal

This project is not just about launching the surviving client. The target is a coherent playable reconstruction of Stargate Worlds, including the missing server-side gameplay/content layer and unfinished QA-client content.

The work includes: determining where every NPC, enemy, vendor, black market, mission object, prop, loot object, trigger, Stargate, audio/Kismet hook and quest area belongs; linking NPC ↔ interaction ↔ dialog ↔ mission ↔ step/objective/task ↔ item ↔ world; rebuilding persistent spawns, regions, teleport/ring/Stargate travel, entity interactions, scripts and mission state; completing unfinished classes/abilities/skill trees; and repairing broken/unpolished maps, assets, materials and appearance state.

## 2. Required evidence labels

Every substantive result must be labeled as one of:

- **CONFIRMED / SOURCE-BACKED** — directly supported by raw client/cache/database/internal technical source.
- **USER-CONFIRMED** — established by project-owner testing/known project evidence, but not yet independently found in the raw source set.
- **RECONSTRUCTION / INFERENCE** — best-fit implementation derived from clues; usable for the rebuild but not claimed as original FireSky data.
- **PARTIAL / UNRESOLVED / MISSING** — evidence is incomplete, contradictory or absent; state what source would resolve it.

Never invent missing coordinates, spawn counts, stats, vendor inventories, Kismet semantics, mission conditions or skill values and call them original/canonical.

## 3. Source hierarchy

When sources disagree, investigate the conflict rather than silently choosing one.

1. **Raw QA client/runtime evidence:** UMAP/UPK/U packages, SGW.exe/config, cooked SourceCache XML, Common schemas/entity definitions, UI and audio.
2. **Original/direct server/database exports and internal technical docs** where provenance is established.
3. **Cross-source exports/references:** dialogue dumps, mission/item DB exports, minimaps, technical worksheets.
4. **USER-CONFIRMED observations/testing.**
5. **Reconstruction/community/project planning material.**

A QA client can itself contain wrong/unfinished placements. “Present in the map” is evidence of the QA build, not automatic proof of intended final design.

## 4. Audited client snapshot

The authoritative raw-client set contains **21 ZIP archives**, **7,671 outer files**, **4.04 GiB compressed** and **6.68 GiB uncompressed**. The authoritative archives in `indexes/SOURCE_MANIFEST.json` passed ZIP CRC validation during this audit.

`SGW.exe` contains QA/working-build paths such as `c:\build\qa\sgw\working\...`; the PE timestamp observed in the audit is **20 June 2009**. Treat the client as a QA/development snapshot, not a clean retail build.

Authoritative raw groups:

- `SGW_Packages_001`–`006`
- `SGW_Maps_001`, `002`, **`SGW_Maps_003(1)`**, `004(2)`, `005(2)`, `006(2)`
- `SGW_Audio_001`–`003`
- `SGW_Content_00`
- `SGWGame_mix`
- `UI`
- `Engine`
- `SGW_Binaries_001`
- `Common`

**Maps_003 rule:** use `SGW_Maps_003(1).zip`. The first upload was observed truncated during the earlier audit. The replacement is CRC-clean and restored the full `Dakara_E1_MapData.upk`, `Dakara_E1_StoryRm` data and `Harset.umap`/`Harset_MapData.upk` plus streaming cells. `SGW_Maps_003_recovered.zip` is salvage/history only.

`StarGate_Worlds.zip` is supplemental mixed-provenance research/reference material, not another raw client chunk.

## 5. Cooked structured data available

Inside `SGWGame_mix.zip`, `SourceCache.en-us/*.pak` files are ZIP containers with XML records. Counts excluding metadata:

- Missions: **1,040**
- Dialogs: **5,405**
- Items: **6,059**
- Abilities: **1,886**
- Effects: **3,216**
- Kismet sequence events: **1,973**
- Kismet event sets: **675**
- Interaction set maps: **4,663**
- WorldInfo: **91**
- Stargates: **28**
- Disciplines: **78**
- Paradigms: **5**
- Sciences: **4**
- Text strings: **29,126**

The handoff contains parsed JSONL for the main gameplay datasets, plus the original `SGWGame_mix.zip` and `Common.zip`.

## 6. Physical production maps in the uploaded raw map set

Main UMAPs detected (streaming cells omitted):

`Agnos`, `Agnos_Library`, `Agnos_ship`, `Beta_Site_Evo_1`, `Castle`, `Castle_CellBlock`, `Dakara_E1`, `Dakara_E1_StoryRm`, `Harset`, `Harset_CmdCenter`, `Harset_Market`, `Harset_StorageRm`, `Ihpet_Crater_Dark`, `Ihpet_Crater_Light`, `Login_Map`, `Lucia`, `Menfa_Dark`, `Menfa_Light`, `Omega_Site`, `Omega_Site_CmdCenter`, `SGC`, `SGC_W1`, `Sewer_Falls`, `Tollana`, `Tollana_Curia`.

All of these except `Agnos_ship` have a detected matching `_MapData.upk` in the raw map set. Harset is complete enough at the client-map layer to inspect its main map, MapData, streaming cells and the CmdCenter/Market/Storage interiors.

Cooked WorldInfo references additional worlds/maps not physically present as production UMAPs in the uploaded raw map archives. That does not automatically mean the upload is incomplete; some were unfinished, test-only or server-defined.

## 7. Key WorldInfo facts

Raw cooked WorldInfo establishes, among others:

- Agnos = 10; Agnos Library = 20
- Lucia = 15
- Naitac = 16 with empty `ClientMap`; MissionTestNaitac = 41
- Omega Site = 18; CmdCenter = 80; Ruins = 81; Storage = 82
- Tollana = 19; Curia = 88
- Beta Site Evo 1 = 23; Beta Site E2 = 60
- Harset = 57; CmdCenter = 68; Market = 69; Storage = 70
- SGC_W1 = 58; SGC = 86; SGC_W2 = 87
- Asgard High Council = 59
- Dakara E1 = 61; StoryRm = 62; E2 = 63; E3 = 64; Superweapon = 65
- Ihpet Crater Dark = 72; Light = 73; Ihpet E1 = 74; E2 = 75
- Pen-Lai = 76
- Menfa Dark = 77; Light = 78
- Pertho = 83; Genetics Lab = 84; StoryRm = 85

Use `structured/WORLDS.json` for all 91 records and `structured/STARGATES.json` for all 28 cooked Stargates, including address/transform/prefabSequence fields where present.

## 8. Client data is not enough for full server reconstruction

The mission/editor reconstruction documentation explicitly says important mission details are missing from client cache files. Full reconstruction can require `missions`, `mission_steps`, `mission_objectives`, `mission_tasks`, spawn/template tables, entity-interaction/dialog-set-map data, world/mission/effect scripts and server-side region data.

**Client hinted generic regions are server-side.** The editor documentation states that such regions are stored in server resources such as `resources.point_sets` and `resources.point_set_points`; UnrealEd is an editing interface and the authoritative region data is not stored in the client map file. Thus trigger boxes, teleport zones and similar quest areas cannot always be recovered from UMAP geometry alone.

Persistent NPC/enemy placement likewise depends on server spawn/template data. Map geometry is one evidence layer, not the full answer.

## 9. Canonical reconstruction example: Mission #742 “Giving the Walls Ears”

`Implementing a sample mission.docx` documents a worked rebuild of Harset mission #742. Source-backed facts in that document include:

- quest giver: **Anat**
- Petbe provides **Item #2819 — Jaffa Disguise**
- **Item #2820 — Scarab Listening Device**
- **Item #2864 — Scarab Listening Device Map**
- devices are planted in the **Jaffa area of Harset**
- Anat and Nerus are in the **Harset Command Center** context
- Anat template used by the tutorial: **#43**
- starter dialog: **#2636**
- interaction set map example: **#3127 — Giving the Walls Ears (more info)**
- Harset Command Center transition example: region tag `Harset.CommandCenterRegion` → `Harset_CmdCenter`, destination approximately `0, 0.355, -25`

The tutorial also explicitly improvises when original details are missing. Preserve that distinction; its improvised crate/device placement is reconstruction, not proof of original FireSky placement.

## 10. Established faction/start/progression project facts

Treat the following as **USER-CONFIRMED** until independently corroborated in raw server data:

- Free Jaffa canonical start: **Dakara**, not the current SGC_W1 server placeholder/template.
- Asgard canonical start: **Pertho**, not the current SGC_W1 server placeholder/template.
- OP-Core Human progression: **Tollana → Harset → Pen-Lai → Dakara**.
- Loyalist Jaffa progression: **Tollana → Pen-Lai → Naitac → Dakara**.
- Goa'uld progression: **Tollana → Harset → Earth/SGC → Naitac → Dakara**.
- Pen-Lai is World 76; a minimap/reference survives although its production UMAP is not in the uploaded raw map set.
- Naitac is World 16 with empty cooked ClientMap and appears unfinished; MissionTestNaitac is World 41.

Do not use `SGW_ Progression.xlsx` as sole canonical proof; it is a reconstruction/planning aid.

## 11. Character creation / appearance reconstruction

`CookedCharCreation.pak` contains BodySet, GenderId, ArchetypeId, AlignmentId, visual groups, component choices and item references. It does not by itself establish canonical world-start coordinates.

Open USER-REPORTED bug: **primary/main colour selected during character creation is not correctly transferred to the created character.** Diagnose in this order:

1. UI character-creation widget/event output and parameter names (`UI.zip`).
2. `CookedCharCreation` visual groups/choices plus DefaultAppearances/AnimMap configuration.
3. Compiled gameplay/runtime references in Binaries/SGWGame for appearance serialization/deserialization.
4. Server character-creation payload/database fields versus client material/body-set expectations.
5. Determine whether the value is never saved, saved under the wrong field, or lost/not reapplied during world load.
6. Test faction/gender/body-set combinations independently.

Do not patch only the material until the persistence path is understood.

## 12. Map/NPC polish workflow

Current USER-REPORTED examples:

- **Agnos:** several assets appear misplaced/not properly polished.
- **Lethander:** skin colour/appearance is wrong.

For every such issue trace:

`UMAP actor/reference → MapData → referenced UPK asset/material → archetype/template/appearance data → server spawn transform/state → minimap/concept/reference → in-game test`.

Classify the fault before editing: map actor transform; server-spawn transform; missing streaming cell; missing/stale package dependency; material instance/parameter; body-set/appearance component; QA placeholder; collision/navmesh/Kismet; or reconstruction-server bug. Preserve original values before modification.

## 13. NPC/dialog/mission/item/world joining workflow

For each NPC/quest chain build one evidence record:

`World → map/interior → zone/region → NPC/template → spawn evidence → interactions → interaction set map → DialogID → speakers/text → MissionID → steps → objectives → tasks → items/rewards/requirements → Kismet/script events → audio → source status → reconstruction decision`.

Rules:

- SpeakerID is a clue, not sufficient proof of spawn location.
- A mission spreadsheet row is not enough to place an NPC; corroborate world/context/dialog/server spawn evidence.
- Dialogue mention of an item does not prove inventory behavior; inspect item/mission/script evidence.
- Exact coordinates require raw map/server/Stargate/region/spawn evidence. Otherwise propose a region and mark RECONSTRUCTION.
- Vendors/black markets require interaction/inventory evidence; do not invent inventories as canon.
- Enemy zones require spawn/spawn-region/mission/combat evidence; environment art alone is insufficient.

## 14. Class and skill-tree completion workflow

Join:

`class/archetype/faction → ability → effect(s) → target method → cooldown/warmup/range → discipline/science/paradigm → prerequisite/training evidence → animation/Kismet/audio → UI tree evidence → level/progression references`.

For every proposed missing node/value keep separate fields for **surviving original evidence** and **reconstruction decision**. Balance changes made for playability must never later be relabeled as original SGW stats.

## 15. Audio

The audited GameplayEngine configuration referenced **274 FMOD FEV projects/files** and all referenced FEVs were found across the uploaded audio archives. Audio filenames/events are useful corroboration for faction/weapon/world/content naming and hooks, but not standalone proof of mission placement.

## 16. Supplemental provenance warning

`StarGate_Worlds.zip` is mixed provenance. Important examples:

- `Implementing a sample mission.docx`, `ASEditor Reference Manual.docx`: valuable technical/reconstruction docs; distinguish original-system descriptions from tutorial improvisation.
- `Mission data_Dialogue_Ids.xlsx`: project-era modified working file, not pristine developer authority.
- `SGW_ Progression.xlsx`: reconstruction/planning aid.
- `SGW_ Milky Way Missions_Dev_Ver.xlsx`: useful cross-reference; verify critical rows against raw/client/database evidence.
- minimaps exist for additional worlds including Asgard High Council, Beta Site E2, Dakara E2/E3, Egypt, Ihpet E1, Pen-Lai, Pertho, SGC_W2, Vitrus and Yotunheim.

## 17. Future-chat procedure

For ordinary mission/dialog/item/world/server reconstruction, upload this handoff ZIP first. It bundles the structured core sources and searchable indexes.

For exact binary/map/asset work, additionally upload the raw archive identified by `indexes/FULL_CLIENT_FILE_INDEX.jsonl`. Examples:

- Harset/Dakara E1 binary map work → `SGW_Maps_003(1).zip` plus referenced Packages archive(s).
- Agnos placement repair → Agnos map archive plus referenced Packages archive(s).
- Character creation/UI → `UI.zip`; for compiled logic add `SGW_Binaries_001.zip` and possibly `Engine.zip`.
- Audio hooks → relevant `SGW_Audio_00x.zip`.

Use SHA-256 values in `indexes/SOURCE_MANIFEST.json` to verify a re-upload matches this snapshot.

## 18. Handoff contents

- `MASTER_SOURCE.md` — this document.
- `NEW_CHAT_START.txt` — paste-ready continuation prompt.
- `PROJECT_ISSUES.json` — current open bugs/data gaps.
- `indexes/SOURCE_MANIFEST.json` — archive hashes/integrity/counts.
- `indexes/FULL_CLIENT_FILE_INDEX.jsonl` — every outer archive entry with archive/path/size/CRC.
- `indexes/SOURCECACHE_COUNTS.json` — nested cooked counts.
- `indexes/SOURCECACHE_ROOT_INDEX.jsonl` — fast searchable cooked record index.
- `structured/*.jsonl` — parsed mission/dialog/item/ability/effect/Kismet/interaction/etc. data.
- `structured/WORLDS.json`, `STARGATES.json`, `PHYSICAL_MAPS.json`.
- `core_archives/SGWGame_mix.zip`, `core_archives/Common.zip` — original compact structured client sources.
- `key_sources/` — compact supplemental docs/exports already extracted from the research pack.

## 19. Continuity rule

The target is a playable reconstructed SGW, not merely a catalogue. Make implementation decisions when evidence requires reconstruction, but always preserve the boundary between recovered original data and new reconstruction. Missing evidence must remain visible, and every reconstruction should be reversible.
