# G-MED evidence: mission 1520 dispenser, minigame and radio

> **Type:** Reference (evidence packet). **Audience:** Castle orchestrator, mission/minigame/authority reviewers, CS-02 authors.
> **Last updated:** 2026-10-17. **Inspected at:** `FEATURE_SHA` `c481008904ba39c80168424f67d96b9b271b9ac9`, QA client install `SGW\Stargate Worlds-QA\Working\SGWGame`.
> **Companions:** [cooked interaction maps](cooked-interaction-maps.md), [seed excerpts](seed-excerpts.md), [dialog excerpts](dialog-excerpts.md), [reconciliation](reconciliation.md), [worknote](../worknotes/CS-00Ra.md).

This packet answers the five CS-00Ra questions. Each row gives the value, one provenance label, the proof and a confidence. It adds three facts to the 2026-10-14 record:

1. **No map-bound use actor exists.** All 146 Castle map packages, decompressed, carry no dispenser, `MsOb` or Ambernol name, and no Kismet `SeqEvent_Used` class (E1, E2).
2. **Set 1571 ships a mission-world-object map.** Cooked map 5828 carries `INT_MissionWorldObject` with no dialog. That is the shape of a clickable quest object, not an NPC (E4).
3. **The "radio" report is not a radio-type dialog.** Cooked dialog 4985 has `UIScreenType="2"` and `KismetEventSetID="0"`, so it can only open as an immediate Dialog window. It cannot wait behind the radio icon (E5).

G-MED remains open for the dispenser's placement (mesh and coordinate), the original game type and the radio trigger. Bounded reconstruction candidates with a collision audit are in [Q5](#q5-bounded-reconstruction-candidates-o14-03).

## Evidence register

| # | Source | What it shows |
|---|---|---|
| E1 | `tools/upk_parser.py::PackageReader` over every package in `CookedPC\Maps\Castle` (146 parsed, 0 errors). It decompresses LZO bodies and scans the name tables and decompressed bodies. | No name or body string matches `Dispenser`, `MsOb` or `Ambernol`. Infirmary-named assets are only `EM-Infirmary00-Generator` prefab parts and `EM-Infirmary00_0[1-4]*` materials in tiles `00040009`, `0004000a` and `00070006`. Three `8896` byte hits are inside numeric float runs, not strings. |
| E2 | Same scan, Kismet class names across all Castle tiles | The Kismet events present are `SeqEvent_Touch`, `RemoteEvent`, `LevelLoaded`, `SequenceActivated`, `Console`, `Designer`, `Stargate` and `TimeofDay`. There is **no `SeqEvent_Used`** and no SGW mission or minigame sequence class. The only SGW actions are `SGWSeqAct_ApplyDSP` and `SGWSeqAct_DSPEffect`, which are audio. |
| E3 | `db/resources/Texts/Seed/texts.sql:57107` | Moniker 8896 `DN_MsOb_Castle_mini_InfirmaryMedDispenser` = "Med Dispenser". In the shipped moniker scheme, `DN_` is a display name, `MsOb` a mission object and `mini` a minigame object. |
| E4 | `SourceCache.en-us\CookedInteractionSet.pak` `_5828` ([receipt](cooked-interaction-maps.md)); `dialog_set_maps.sql` row 5828 ([excerpt](dialog-excerpts.md)); bit meaning from `docs/content/interaction-flags.md:120` and `entities/defs/enumerations.xml` | Set 1571, dialog NULL/0, flags `1073741824` = bit 30 `INT_MissionWorldObject` ("quest item glow"). Map 5827 (dialog 4983, Ogilvie speaker 2490) carries bit 24 `INT_AStoryMissionActive`. Maps 5824–5826 carry 0. No set-1571 map carries a minigame bit (256–4096). |
| E5 | `SourceCache.en-us\CookedDataDialogs.pak` `_4982`–`_4985`; `dialogs.sql:8329–8335`; `docs/content/dialog-ui-client-contract.md:104–116` | All four have `UIScreenType="2"` (`DUIST_DefaultDialog`), `KismetEventSetID="0"` and `DialogFlags="0"`. The seed agrees. A non-immediate type-2 dialog vanishes. Only type 4 (`DUIST_DefaultRadio`) queues behind the radio icon. |
| E6 | `db/resources/Entities/Seed/entity_templates.sql`, template 173 header (`Castle_CommsTerminal`) and template 147 | Castle clickable props are server-spawned `being` templates (`GLB_Components.WorldObject_Small`) whose `name_id` is the prop's `DN_Ob_*` moniker. Template 173 uses `name_id` 7720 and template 147 uses 7722. |
| E7 | `git grep -w` for moniker IDs 28133–28553 (the other `DN_ob_*_mini_*` objects) and 8896 across `db/resources`, `entities` and `crates` | No template, spawn or chain uses any `*_mini_*` moniker. The stock seed carries no mission-object placement for any world, so the missing dispenser row is a corpus-wide loss, not a Castle-only one. |
| E8 | `docs/gameplay/minigame-system.md:71–76`; branch `work-packets.md:187–195` | Hack, Bypass and four other games run on the auto-win `PlaceholderGame`. Livewire validates moves on the server. Livewire with cursor 256 is the signed O14-03 game choice for this beat. |
| E9 | `mission_steps.sql`/`mission_objectives.sql`/`mission_tasks.sql` ([seed excerpts](seed-excerpts.md):28–40, 118–136, 214–238) | Step 4452 is "Secure a supply…", step 4453 is "Hack the Med Dispenser…" and step 4454 is "Use your radio to report to Capt. Copplemann." Objectives 5171 and 5172 (hidden) belong to step 4452, 5173 to 4453, and 5174 to 4454. Tasks 6372–6376 all have type 1. Steps and objectives are disabled, and tasks are enabled. |
| E10 | Name-table scan of `CookedPC\Maps\Castle` for medical props | `EM-MedicalBox00` appears in tiles `00070002`, `00070004`, `00080003` and `00090003`. `EM-LockerMed00` and its `_Pf0` prefab appear in tiles `00060005`, `00070002`, `00080002` and `000a0002`. These are package-present asset names only; no instance or coordinate join was decoded. |

## Q1. Which actor is the Med Dispenser, and how does the client target it?

| Field | Value | Provenance | Proof | Confidence |
|---|---|---|---|---|
| Display identity | Moniker 8896 "Med Dispenser" | SHIPPED_DATA | E3 | High |
| Map-bound use actor | **None exists** in any Castle package: no dispenser name and no `SeqEvent_Used` | CLIENT_MAP (negative) | E1, E2 | High for these 146 packages |
| Targeting model | A server-spawned entity carrying `name_id` 8896 and an interaction mask, not a map-bound interactable | RECONSTRUCTION (inferred) | E2 rules out a Kismet use event. E3's `MsOb` naming, E4's mission-world-object map in set 1571 and E6's Castle prop pattern all point to a spawned `being`. | Medium |
| Client cue | `INT_MissionWorldObject` (bit 30) glow, with set 1571 attached to the entity | CLIENT_MAP (cooked interaction map) | E4, map 5828 | Medium. The flag is shipped, but its join to the dispenser entity is inferred from the set and topic. |
| Map package, actor name, location | **UNRESOLVED (G-MED placement)** | none | E1 found no named actor. The E10 medical meshes are candidates only. | n/a |

**Missing evidence:** the original spawn row (template, mesh and coordinate) for moniker 8896. Any of these would close the row:

- a server spawn-set source;
- a decoded `StaticMeshActor` → `EM-MedicalBox00`/`EM-LockerMed00` instance join inside the Infirmary bounds, as a placement anchor only;
- a captured 2009 `createEntity` for this object.

## Q2. Which minigame does the dispenser start, and with which parameters?

| Field | Value | Provenance | Proof | Confidence |
|---|---|---|---|---|
| Original game type | **UNRESOLVED (G-MED game)** | none | E9 has only the step verb "Hack". E4 shows no minigame bit in any set-1571 map. E5 shows no Kismet event set on the dialogs. | n/a |
| Verb in shipped text | "Hack" (step 4453) | SHIPPED_DATA | E9 | High |
| Is "Hack" server-safe today? | No. It runs `PlaceholderGame`, where a client `victory` wins. | CIMMERIA_AUTHORING (current code fact) | E8 | High |
| Signed choice | Livewire, mask `INT_MinigameLivewire` 256, difficulty from the `start_minigame` param (1–5, default 1) | RECONSTRUCTION (O14-03, signed 2026-10-14) | E8; `minigame-system.md:159–160` | Signed game choice only. It is not a claim about the original game. |
| Difficulty and parameters | **UNRESOLVED (G-MED game)**. No shipped difficulty, TC level or archetype bitfield exists for this object. | none | E1, E4, E9 | n/a |

## Q3. What starts the 1520 radio beat (objective 5174, dialog 4985, map 5826)?

| Field | Value | Provenance | Proof | Confidence |
|---|---|---|---|---|
| Dialog window type | Type 2 Dialog window, opened immediately | SHIPPED_DATA (cooked) | E5 | High |
| "Radio" mechanism | **Not** the client radio queue: 4985 is type 2, not `DUIST_DefaultRadio` (4). A non-immediate send would vanish. | SHIPPED_DATA (negative) | E5 | High |
| Kismet or event-set trigger | None: `KismetEventSetID="0"` and seed `event_set_id` NULL | SHIPPED_DATA (negative) | E5; [reconciliation](reconciliation.md):37 | High |
| Map 5826 indicator | Flags 0, so no NPC quest indicator. That fits a speaker (1110, Copplemann's radio voice) with no world body. | CLIENT_MAP | E4 | High |
| Firing transition and actor | **UNRESOLVED (G-MED radio)**. No shipped row names the step, objective or actor that displays 4985. | none | E5, E9, [reconciliation](reconciliation.md):37 | n/a |

**Consequence for CS-02:** step 4454's text asks the player to "use your radio", but the shipped dialog cannot wait as a radio lure. The server therefore has to display 4985 immediately, at a server-chosen moment. The graph candidate's "show 4985 after the 4984 close" is compatible with this. Showing it as a lure is not, because it would vanish. Opening a lure also never reaches the server (`dialog-ui-client-contract.md:116`).

## Q4. Which transitions for steps 4452–4454 and objectives 5171–5174 are proven?

| Transition | Status | Provenance | Proof |
|---|---|---|---|
| Offer 1520 on a validated `Castle.Infirmary` enter (world 8, 704 done, archetypes 1–4/6) | Signed reconstruction (chain 1367) | RECONSTRUCTION | branch `work-packets.md` § CS-02/03 entry-only seal |
| Credit 5171 while step 4452 is active (region replay) | Signed reconstruction (chain 1368) | RECONSTRUCTION | same |
| Ogilvie talk shows 4983 → 4452→4453, credit hidden 5172 | Inferred. Supported by map 5827 `INT_AStoryMissionActive` on 4983 (speaker 2490) and 5172's text "Speak to Ogilvie…". | CLIENT_MAP + SHIPPED_DATA (inference) | E4, E9 |
| Dispenser interaction while 4453 is active starts the game | Inferred. Supported by step 4453's text and map 5828 `INT_MissionWorldObject`. | SHIPPED_DATA (inference) | E4, E9 |
| Victory → 4453→4454, credit 5173, show 4984 | Inferred. 4984 says the dispenser "now provides an unlimited supply", and 5173's text is blank. | SHIPPED_DATA (inference) | [dialog excerpts](dialog-excerpts.md):134; E9 |
| 4985 display → close → credit 5174, complete 1520 | Inferred. 4985 is type 2 and immediate (E5). Its trigger is unrecovered. | SHIPPED_DATA (inference) | E5, E9 |
| Dialog 4982 (on the 704 completion) | Unsealed cue, separate from these steps | UNRESOLVED (G-MED radio) | branch `work-packets.md:170–171` |

**Proven** means a shipped row or client record binds the transition. None of the five in-mission transitions is proven at that level. Each inferred row rests on shipped text and flags, with no shipped binding row.

## Q5. Bounded reconstruction candidates (O14-03)

These are candidates for a coordinator signature, not decisions. All mutations stay mission-state only (no item, XP or cash), as the 2026-10-14 graph candidate requires.

| Candidate | ID | Shape |
|---|---|---|
| Dispenser entity template | **412** | A clone of template 147's shape (`being`, `GLB_Components.WorldObject_Small`, flags 0, `interaction_type` 0), with `name_id` **8896** and `template_name` `Castle_MedDispenser`. The mesh is a choice among the E10 package-present medical meshes, and the client must check that it renders. |
| Dispenser spawn | **13801** | World 8 at an Infirmary coordinate inside seeded region 2051 and on navmesh component 250. The coordinate stays **UNRESOLVED (G-MED placement)** until the E10 join or a lab placement check. Tag `Castle_MedDispenser`. |
| Interaction chain (4453 active → `set_interaction_type` mask 256 + `start_minigame` Livewire) | **1371** | Guarded on world 8 and active step 4453, with `delay_ms=0`, through the CS-02a condition-aware callback seam. |
| Victory callback (advance 4454, credit 5173, show 4984 immediate) | **1372** | Fires only through the CS-02b2 identity gate. It clears mask 256 and bit 30. |
| Radio close (4985 `-1` close → credit 5174, complete 1520) | **1373** | 4985 is shown immediately after the 4984 close and never as a lure (E5). |

**Collision audit.** The command of record is plain `git grep`, run at each ref. `-w` matches whole IDs. The base ref is shown. The same checks were run on the other three refs (Cellblock `3b5ddf475`, Harset `7965f653d`, Tollana `a1e99c0e5`). They give no chain-row hit for 1371-1377; 412 has no `entity_templates` row and 13801 has no `entity_templates` or `spawnlist` row:

```text
git grep -n -w 412 c481008904ba39c80168424f67d96b9b271b9ac9 -- db/resources/Entities db/resources/Worlds db/resources/Content
git grep -n -w 13801 c481008904ba39c80168424f67d96b9b271b9ac9 -- db
git grep -n -w 1371 c481008904ba39c80168424f67d96b9b271b9ac9 -- db/resources/Entities db/resources/Worlds db/resources/Content
git grep -n -w 1372 c481008904ba39c80168424f67d96b9b271b9ac9 -- db/resources/Entities db/resources/Worlds db/resources/Content
git grep -n -w 1373 c481008904ba39c80168424f67d96b9b271b9ac9 -- db/resources/Entities db/resources/Worlds db/resources/Content
git grep -n -E "\(\s*137[1-7]\s*," c481008904ba39c80168424f67d96b9b271b9ac9 -- db/resources/Content
```

Base results (checked 2026-10-17):

- **412:** 15 hits. None is in `entity_templates`, so template 412 is free. `blueprints_components.sql` (8) and `blueprints.sql` (1) use `blueprint_id`. `spawnlist_debug_area_npcs.sql` (5) is one comment at `:29` and four rows with 412 as the x coordinate. `spawnlist.sql:1171` (1) is `spawn_id` 412 with `template_id` 312. These are other namespaces.
- **13801:** 0 hits in `db/resources/Entities`, `Worlds` and `Content`. Under `db` the only hit is the moniker row `texts.sql:8537`. The archived `deprecated/db-monolithic-sql/db-deprecated/resources.sql:98400` carries the same moniker row. Spawn ID 13801 is free.
- **1371:** 12 hits in 7 files. Template rows, with `template_id` 1371: `entity_templates_debug_area_combat.sql:28` and `entity_templates_debug_area_lineup.sql:218`. Spawn row with template 1371: `spawnlist_debug_area_combat.sql:42`. Comments: `entity_templates.sql:676, 692, 693, 704`, `entity_templates_debug_area_combat.sql:25`, `entity_templates_debug_area_lineup.sql:217`, `spawnlist.sql:962`, `spawnlist_debug_area_lineup.sql:203` and `harset_opcore_chains.sql:20` (a Harset mission-to-chain range).
- **1372:** 15 hits in 7 files. Ability rows: `castle_cellblock_chains.sql` (5: comments at `:1041`, `:1043`, `:1054` and `:1070`, and the `launch_ability` row at `:1091`). Template rows: `entity_templates_debug_area_combat.sql:29`. Spawn rows with template 1372: `spawnlist_debug_area_combat.sql:44–46` (3). Comments: `entity_templates_debug_area_combat.sql:26`, `entity_templates_debug_area_lineup.sql:51`, `entity_templates.sql:814` and `harset_opcore_chains.sql:20`. ID lists: `resource_versions.sql:445` and `:449` (2).
- **1373:** 6 hits in 4 files. Template row, with `template_id` 1373: `entity_templates_debug_area_combat.sql:41`. Spawn rows with template 1373: `spawnlist_debug_area_combat.sql:47–48` (2). Comment: `entity_templates_debug_area_lineup.sql:79`. ID lists: `resource_versions.sql:445` and `:449` (2).

No chain or trigger row uses 1371–1377 (the last command, scoped to `db/resources/Content` where every `content_chains` insert lives, returns nothing at base; the same pattern under `db` matches only unrelated dialog, event, sequence and mission rows). 412 is not a template ID, and 13801 is not a spawn ID. The numbers 1371–1373 are template IDs in debug-area files and ability or mission mentions. Those are other namespaces and do not collide with chain IDs. Chains 1371–1373 sit in the Castle block, which `id-allocation.md:67` lists as "unclaimed candidates requiring their own check". The coordinator records the claim. This packet does not edit the shared table.

**Cross-reference (CS-00Rb).** The sibling packet CS-00Rb (`castle/cs-00rb-g-sym`, `evidence/g-sym.md`, mission 1521) first drafted chains 1371–1374. Its collision audit covered only the four branches and did not see this packet. The settled allocation gives Ra chains **1371–1373** and Rb **1374–1377**, as recorded in the Collision audit of Rb's `evidence/g-sym.md`. Chains 1374–1377 are free at base. This packet does not edit Rb's files.

## Negative claims, one per provenance class

| Class | Negative claim | Check |
|---|---|---|
| SHIPPED_DATA | IDs 8896/8897 in `items.sql:2509–2511` are "Dross Kit" items, not the dispenser. Item and moniker namespaces are separate. | `git grep -w 8896 -- db/resources` |
| CLIENT_MAP | No `SeqEvent_Used` and no dispenser name in any of the 146 Castle packages | E1, E2 |
| CLIENT_MAP (cooked dialogs) | 4985 has no Kismet event set and is not the radio type | E5 |
| RECOVERED_SCRIPT | Legacy `Castle.py` binds `Castle.Infirmary` only to room text 5188, with no 1520 actor | [reconciliation](reconciliation.md):11; branch `work-packets.md:84–86` |
| RECONSTRUCTION | No current Castle chain or template targets 1520 steps 4453/4454 | E7; `castle_70*_chains.sql` carries no 4453/4454 |
| ORAL_HISTORY | Gives tone only. No 1520 mechanic is taken from it. | [reconciliation](reconciliation.md):16 |
| CIMMERIA_AUTHORING | `PlaceholderGame` "Hack" is not evidence of the original game | E8 |

## Open gates after this packet

- **G-MED placement:** dispenser mesh and coordinate.
- **G-MED game:** the original game type and difficulty.
- **G-MED radio:** the step or actor that displays 4985.
- **G-REVIEW and client UAT:** unchanged.

## R2 seal

CS-02S (2026-10-17, base `da2b82c70`) froze the mission 1520 rows. Full SQL, answers and tests: [CS-02S worknote](../worknotes/CS-02S.md). Verdict **SEALED**.

- Template 412 `Castle_MedDispenser` (clone of 147, name_id 8896, `interaction_type 0`) and spawn 13801 at (356.0, 70.18, 892.0), inside region `Castle.Infirmary`, navmesh component 250 (same as Ogilvie), 30.8 u from the nearest hostile. Collision audit empty on four refs.
- Per-player glow through `add_dialog_set` on slots 412 and 179 with existing maps 5828 and 5827; `set_interaction_type` is global and is not used.
- Chains 1367, 1368, 1371-1373 and 1378-1383, one new seed file `castle_1520_chains.sql` included after `db/database.sql:397`.
- Q5 candidates resolved: game `Livewire` difficulty 1, 4984 on victory, 4985 on the 4984 close, relog recovery auto-completes at 4454. Original game and radio actor stay UNRESOLVED; the choices are RECONSTRUCTION.
- Binding: 1371 must not ship before CS-02L's guarded victory seam. Client-only claims are UATPending (D-CA11).
