# Handoff Pack v1.2 — World / Starter / Mission-Scope Audit

Read-only audit. No repo file was modified, created, or deleted. No cargo build was run.

**Checkout caveat that gates verdicts 2–5:** this branch (`docs/handoff-pack-phase0-gap-report`) is behind `origin/main`. The three Castle-main chain seeds (`castle_701_chains.sql`, `castle_702_704_chains.sql`, `castle_706_708_chains.sql`) exist only on `origin/main`. Rows marked *(main)* were read via `git show origin/main:`.

## Starter worlds

Authoritative path: `C:\Users\Steve\source\projects\Cimmeria\crates\services\src\base\chardef.rs:9-253` (hardcoded `chardef_lookup`, transcribed from `C:\Users\Steve\source\projects\Cimmeria\db\resources\Archetypes\Seed\char_creation.sql:7-51`), consumed by `C:\Users\Steve\source\projects\Cimmeria\crates\services\src\base\character_create.rs:156-164` and resolved to a numeric id at `:333-347`.

Only two start rows exist, keyed on **alignment**, not archetype:

| Alignment | World | id | x / y / z |
|---|---|---|---|
| SGU | `SGC_W1` | 58 | 201.5 / 1.31 / 49.724 |
| Praxis | `Castle_CellBlock` | 12 | -334.231 / 73.472 / -228.026 |

| Profile | Pack target | Cimmeria today | Verdict |
|---|---|---|---|
| SGU Human (Soldier/Commando/Scientist/Archaeologist) | Earth SGC | `SGC_W1` (58), the scripted Earth tutorial map | **PARTIAL** — an Earth/SGC map, but the tutorial instance, not hub world `SGC` (86) |
| Free Jaffa / Shol'va | Dakara | `SGC_W1` (58) | **CONFLICT** — this is exactly the placeholder the pack forbids |
| Asgard | Pertho | `SGC_W1` (58), chardef 9 only (no female Asgard chardef) | **CONFLICT** — same placeholder |
| Praxis Human / Goa'uld / Loyalist Jaffa | unresolved | `Castle_CellBlock` (12) | **N/A** — pack declines to specify |
| "SGC_W1 placeholder starts not reintroduced for Jaffa/Asgard" | assertion | both currently on SGC_W1 | **CONFLICT** |

Per-chardef detail from `chardef.rs`: SGU side → SGC_W1 = chardefs 2/12 Soldier, 4/14 Commando, 21/23 Scientist, 6/16 Archeologist, 9 Asgard, 8/18 Shol'va. Praxis side → Castle_CellBlock = 1/11 Soldier, 3/13 Commando, 20/22 Scientist, 5/15 Archeologist, 10/19 Goa'uld, 7/17 Jaffa. There is no archetype-, race-, or gender-specific offset anywhere; every SGU character spawns on the identical coordinate, likewise every Praxis one.

**The conflict is not a one-line data fix.** `C:\Users\Steve\source\projects\Cimmeria\crates\services\src\base\world_entry\space_registry.rs:27-37` knows only three world names (`Castle_CellBlock` → 65552, `SGC_W1` → 65553, `CombatSim` → 65554) and **silently falls back to `Castle_CellBlock`** for anything else (`:33`). Repointing a chardef at `Dakara` or `Pertho` today would dump that character into the Praxis cellblock. `C:\Users\Steve\source\projects\Cimmeria\crates\services\src\cell\cell_methods\player\combat\respawn.rs:335-336` re-hardcodes the same cellblock coordinate as the universal death fallback.

Legacy python equivalent, for reference: `C:\Users\Steve\source\projects\Cimmeria\deprecated\python\common\defs\CharacterCreation.py:17` and `C:\Users\Steve\source\projects\Cimmeria\deprecated\python\base\Account.py:163-177`.

`db/resources/Worlds/Seed/spawn_points.sql` is **empty** (a lone `setval(...,5)`) and plays no part in character creation. `respawners.sql` is death-respawn only.

Design docs already record the pack's targets and already contradict the code: `C:\Users\Steve\source\projects\Cimmeria\docs\content\external-data-analysis.md:58-72` and `C:\Users\Steve\source\projects\Cimmeria\docs\content\archetype-content-map.md:108` ("Starting world: Praxis -> Castle_CellBlock, SGU -> SGC_W1").

## World ID crosscheck

`C:\Users\Steve\source\projects\Cimmeria\db\resources\Worlds\Seed\worlds.sql` holds **91 INSERT rows**, matching the pack's "all 91 records" claim for `structured/WORLDS.json`. Every ID asserted in `MASTER_SOURCE(4).md` section 7 matches — all 38 checked:

| Pack claim | Our seed | Verdict |
|---|---|---|
| Harset 57 / CmdCenter 68 / Market 69 / Storage 70 | identical (`Harset` has_script true, client_map `Harset`) | MATCH |
| SGC_W1 58, SGC 86, SGC_W2 87 | identical | MATCH |
| Dakara E1 61 / StoryRm 62 / E2 63 / E3 64 / Superweapon 65 | identical | MATCH |
| Pertho 83 / Genetics Lab 84 / StoryRm 85 | identical | MATCH |
| Naitac 16, **empty ClientMap** | `worlds.sql:17`, `client_map = ''` | MATCH, including the emptiness |
| MissionTestNaitac 41 | `MissionTestNaitac` | MATCH |
| Pen-Lai 76 | `Pen-Lai`, `client_map = 'Pen-Lai'`, flags 1 | MATCH |
| Agnos 10 / Library 20, Lucia 15, Tollana 19 / Curia 88 | identical | MATCH |
| Omega Site 18 / CmdCenter 80 / Ruins 81 / Storage 82 | identical | MATCH |
| Beta Site Evo 1 = 23, Beta Site E2 = 60 | identical | MATCH |
| Asgard High Council 59 | `Asgard_Hgh_Council` | MATCH |
| Ihpet Crater Dark 72 / Light 73, Ihpet E1 74 / E2 75 | identical | MATCH |
| Menfa Dark 77 / Light 78 | identical | MATCH |
| Castle IDs (not asserted in pack section 7) | `Castle` = 8 (flags 0, has_script true), `Castle_CellBlock` = 12 (flags 1 = `WF_Instance`) | ours only |

`WF_Instance` enum lives at `C:\Users\Steve\source\projects\Cimmeria\db\resources\Worlds\Types\EWorldFlags.sql`. Two seed oddities unrelated to the pack: world 2 `SandBox` carries `client_map = 'Harset_CmdCenter'`, and there is no row for world id 71.

## Castle CellBlock scope verdicts

| QA_TESTS item | Verdict | Evidence |
|---|---|---|
| 1. Player progression remains inside CellBlock through Mission 688 | **MATCH** | `db\resources\Content\Seed\castle_cellblock_chains.sql` contains exactly one `cross_world_teleport` action, chain 1109 at `:2128`. Nothing else leaves world 12. |
| 2. Drones → Ambernol → Ring Transport minigame sequence preserved | **PARTIAL — the pack's sequence is wrong** | See breakdown below. |
| 3. Armory is the CellBlock end | **MATCH** | Mission 688 "Secure the Armory", chains 1105-1111 at `:1982-2162` plus blurb chain 1154. The full set of `accept_mission` targets in the file is {689, 638, 641, 682, 684, 686, 687, 688, 1360} — nothing is accepted after 688, on either HEAD or `origin/main`. |
| 4. Transition after Armory goes to Castle main | **MATCH** | Chain 1109: trigger `interact_tag 'Cellblock_ArmoryRingSwitch'`, condition `step_status(688, 80688) = 'active'`, actions `complete_mission 688` → `set_interaction_type ~INT_MissionWorldObject` → `cross_world_teleport target_key 'Castle'`, params `{x: 466.365, y: 70.397, z: 991.466}`. Mirrored at `db\resources\Worlds\Seed\ring_transport_regions.sql:84` (region 34, world_id 8, `Castle_ArmoryRingDropZone`) and `respawners.sql:59-61`. Arrival is caught *(main)* by chain 1201, `castle_701_chains.sql:139`. |
| 5. Mission 701+ content not spawned/started inside CellBlock | **MATCH** *(main)* | All Castle-main chains gate on `player_loaded 'Castle'` or `enter_region 'Castle.ThroneRoom' / '.InterrogationBlock' / '.CommsRoom'`. I grepped all three Castle-main files for "cellblock": every hit is a code comment citing a CellBlock precedent. Zero trigger keys. Copplemann and Zuritska have no CellBlock spawnlist rows. |

### Item 2 detail — where the pack's sequence diverges

The four beats are all real content, but they are not four consecutive missions and the stated order is wrong. Actual implemented CellBlock order, derived from the `accept_mission` / `complete_mission` action graph:

```
689  Prison Boot Lock (internal, hidden, Cimmeria-authored)  chains 1022-1025
622  Arm Yourself!                                           chains 1001-1008
       (+ 1360 Frost's Letter accepted here)
638  Speak to Prisoner 329        Livewire #1, cell-door hack
639  Find Ambernol               *** THE DRONE LIVES INSIDE THIS MISSION ***
640  Hack the Rings               Livewire #2, ring-transport hack
641  Preparation                  Livewire #3, terminal
680  Escape the Cellblock
681  Mess Hall Controller
682-686  Hallway01-05 Controllers (hidden; 686 adds the Straegis attack scene)
687  Aftermath                    archetype-split crate rewards
688  Secure the Armory            terminal + optional NID guard, then ring switch
       --> cross_world_teleport to 'Castle' (world 8)
```

- **"Drones" is not a mission.** The drone is entity tag `ArmYourself_PrisonerRetrievalUnit`, aggro'd by chain 1032 when the player takes the vial. Step 2144 of mission 639 requires **both** objective 2482 (kill the drone) and objective 2484 (take cover, cover_set 1381) before advancing to step 2343 — chains 1033 / 1131 / 1132 / 1133 at `castle_cellblock_chains.sql:742-837` (packet C05, PR #653). So the drone comes *after* the Ambernol vial, not before the mission.
- **Ambernol** is mission 639 itself, steps 2117 → 2145 → 2144 → 2343. Item 19 is consumed by chain 1034, which completes 639 and accepts 640.
- **Ring Transport minigame** is mission 640 "Hack the Rings" (Livewire on `HackTheRings_Switch`, chains 1041/1042; completed by chain 1044 on `teleport_in regionId=2`). It is the **second** of three Livewire games in the zone, not the third.
- **Armory** (688) is five missions later, not the successor of 640.

The only other drone content is five unwired ambient Prisoner Retrieval Units on world 8 — `Castle_PRU1`-`PRU5`, spawns 90/98/106/111/114 in `db\resources\Worlds\Seed\spawnlist.sql`, template 145. No chain in any of the four Castle seed files references them.

### Mission inventory by world

CellBlock instance, world 12, `castle_cellblock_chains.sql`: 689, 622, 1360, 638, 639, 640, 641, 680, 681, 682, 683, 684, 685, 686, 687, 688. Mission 642 "Escape the Cells" is hidden orphan content with no chain and no script. *(main)* adds chains 1141/1142 (C06 flanking) and 1171-1175 (GC1 Col. Marsh escort).

Castle main, world 8, *(main only)*: 701 Reinforce Copplemann (chains 1201-1205, 1231-1243), 702 Rescue Dr. Zuritska (1261-1265), 703 Payback (1271-1273), 704 Hack Communications (1291-1302), 706 Power Behind the Throne (1321-1323), 708 Secure the Stargate (1341-1365). **Missions 705 and 707 do not exist** in `missions.sql`. Chain graph: 701 → {702, 703} in parallel → 702 completes and accepts 704 → 704 accepts 706 → 706 accepts 708 → 708 completes on `stargate_crossed 'Harset'`.

Copplemann (seed spells it `Coppleman`, one n): spawn 87, world_id 8, template 48, tag `Castle_Coppleman`, (352.69, 70.27, 952.32). Zuritska: *(main)* spawns 238 `Castle_Zuritska_Cell` and 239 `Castle_Zuritska_Comms`, template 168, both world 8. On this branch Zuritska is narrative data only — `grep -c Zuritska spawnlist.sql` = 0.

### Scope-metadata defect

Chain 1008 (`castle_cellblock_chains.sql:467`) is declared `scope_type='space', scope_id=8` — world 8 is Castle, not CellBlock (12). Its trigger key is `'Castle_CellBlock.Region8'`, so it fires in the correct world; the `scope_id` almost certainly echoes "Region**8**". Harmless today because `DbChainRow` in `C:\Users\Steve\source\projects\Cimmeria\crates\content-engine\src\loader\mod.rs:41-49` carries `scope_type` / `scope_id` as editor metadata and never filters on them at runtime. **The effective world gate is always the trigger `event_key`, not `scope_id`.** Worth fixing before anyone builds tooling that trusts `scope_id`.

Related: region keys are spelled `Castle_Cellblock.*` (lowercase b) everywhere except `Castle_CellBlock.Region8` (capital B), deliberately, because that is the spelling in `db/resources/Events/Seed/point_sets.sql` set_id 2039 and matching is exact string compare (`crates\content-engine\src\triggers\matching.rs`). Documented at `castle_cellblock_chains.sql:435-442`.

## Progression routes

**Nothing is implemented.** No schema column, no code, no runtime gate.

- `worlds.flags` (`db/resources/Worlds/Tables/worlds.sql:8`) is instance-vs-persistent only. It is never read as a starter or unlock gate anywhere in `crates/`.
- `db/resources/Worlds/Seed/stargates.sql` is pure address→world data (`address1..6`, `address_origin`, `world_id`, transform) with no unlock, level, or faction gate.
- `C:\Users\Steve\source\projects\Cimmeria\crates\services\src\base\world_entry\gate_travel\mod.rs` applies no destination gating.
- Everything matching "progression" in `crates/` is either XP/level (`base\world_entry\methods\progression\mod.rs`, LEVEL_XP at `:18`) or mission-step progression (`cell\missions\progression.rs`). Neither gates worlds.

Routes exist only as documentation, at `docs/content/external-data-analysis.md:58-72` (sourced from `SGW_ Progression.xlsx`, which the pack itself says is not canonical proof) and referenced again in `docs/content/zone-audit.md` around lines 865-871. Comparison against the pack:

| Faction | Pack route (USER-CONFIRMED) | Our doc | Verdict |
|---|---|---|---|
| Loyalist Jaffa | Tollana → Pen-Lai → Naitac → Dakara | Tollana → Pen-Lai → Men'fa → Naitac → Ihpet → Dakara | **COMPATIBLE** — pack is a clean subset |
| Goa'uld | Tollana → Harset → Earth/SGC → Naitac → Dakara | Tollana → Men'fa → Naitac → Harset → Lucia → Hebridan → Harset → Agnos → … | **CONFLICT** — Harset placement differs, pack adds an Earth/SGC leg we lack |
| Praxis Human / OP-Core | Tollana → Harset → Pen-Lai → Dakara | no counterpart row | **PARTIAL** — our *implemented* Praxis flow is Castle_CellBlock → Castle → Harset (mission 708 completes on `stargate_crossed 'Harset'`). Harset-next agrees; the Tollana leg does not exist. |

Nearest existing gating primitive worth reusing if routing is built: `ring_transport_regions.required_mission_id` (30 rows, every one NULL today).

## Stargates and partial worlds

`db/resources/Worlds/Seed/stargates.sql` holds **28 rows**, matching the pack's "28 cooked Stargates" exactly. All 28 stargate ids, names and `world_id` values match `references/source_extracts/stargates(1).txt` one-for-one, and the `event_set_id` column matches on every row that carries one.

Three `address6` values differ. In each case the pack extract contains a **duplicate address tuple** that our seed disambiguated by bumping the sixth glyph to the next free value:

| Stargate | Pack address6 | Ours | Collided in pack with |
|---|---|---|---|
| 24 Egypt | 29 | **32** | 23 SGC (28,26,5,36,11,29,1) |
| 20 Ihpet Crater (SGU) | 28 | **31** | 8 Ihpet Crater (Praxis) (3,8,16,24,23,28,2) |
| 22 Men'fa (SGU) | 33 | **34** | 7 Men'fa (Praxis) (4,13,5,6,7,33,10) |

In all three the row our seed *kept* unchanged is the lower-id / Praxis-side one. No doc under `docs/` records this divergence — `grep -rn "address6\|address collision\|duplicate address" docs/ --include=*.md` returns nothing. This needs a SOURCE_POLICY label decision: either the collisions are genuine 2009-snapshot data to be preserved, or our disambiguation is a deliberate RECONSTRUCTION and must be labelled as one.

**Naitac (16)** — our seed independently reproduces the empty `client_map`, so we agree with the pack that it is unfinished. It has no stargate row. **Pen-Lai (76)** — has `client_map = 'Pen-Lai'` and stargate id 14 at (-104.5, -11.554, 128.714), but that stargate's `prefab_sequence` is **empty in both our seed and the pack extract**, which is consistent with the absent production UMAP. Neither world appears in the 24-zone playability matrix at `docs/content/README.md`, so neither is claimed content-ready. Note the pack's section 6 UMAP list also omits Pertho and Dakara-beyond-E1, which matters for the starter-world retarget in Phase 5.

## Server-side data we already hold

The pack's section 8 claim is that the client cannot supply region data, spawn/template data, and the four mission tables. We hold the **mission tables in full**; the **region data covers three worlds only**.

| Table | Rows (this branch) | Rows (origin/main) |
|---|---|---|
| `resources.missions` | 1,041 | 1,041 |
| `resources.mission_steps` | 3,480 | 3,480 |
| `resources.mission_objectives` | 4,037 | 4,037 |
| `resources.mission_tasks` | 4,358 | 4,358 |
| `resources.point_sets` | 62 | 66 |
| `resources.point_set_points` | 119 | 135 |
| `resources.entity_templates` | 153 | 159 |
| `resources.spawnlist` | 167 | 176 |
| `resources.worlds` | 91 | 91 |
| `resources.stargates` | 28 | 28 |
| `resources.ring_transport_regions` | 30 | 30 |
| `resources.respawners` | 8 | 8 |
| `resources.mission_rewards` | 8 | 8 |
| `resources.spawn_points` | **0** | **0** |
| `resources.paths` | **0** | **0** |

Read that as: the pack's mission-table gap **does not apply to Cimmeria at all** — we hold the complete original export of all four tables, which is the single largest piece of "missing server data" the pack flags. The template/spawn gap does not apply at the schema level either; 153 templates and 167 spawn rows exist, they are just concentrated in three worlds. The **region gap is real and is the long pole**: 66 point sets across 91 worlds covers Castle, Castle_CellBlock and SGC_W1 and essentially nothing else, and `spawn_points` plus `paths` are empty files.

Reward data is the other real hole: 8 `mission_rewards` rows against 1,041 missions, consistent with `docs/content/README.md`'s "zero missions have XP or currency rewards".

## Blockers

1. **The space registry, not char-creation data, blocks Phase 5.** Adding `Dakara` or `Pertho` to `char_creation.sql` without extending `space_registry.rs:27-37` silently routes those characters into `Castle_CellBlock` (the `:33` fallback). Make that fallback fail loudly before touching any start row. Route through `rust-gameserver-dev` + `database-persistence`. The same hardcode exists in `respawn.rs:335-336`.
2. **"Free Jaffa" vs "Shol'va" needs an explicit ruling.** Our seed has two distinct archetypes: `ARCHETYPE_Shol'va` (7, SGU alignment) and `ARCHETYPE_Jaffa` (8, Praxis alignment). The pack's single "Free Jaffa / Shol'va" profile maps to 7. Nobody should move archetype 8 off the Castle cellblock while acting on the pack.
3. **This branch cannot validate Castle verdicts 2, 4 and 5.** Rebase onto `origin/main` before running the Castle CellBlock regression suite; the three Castle-main seed files are not in this checkout.
4. **Phase 6 region authoring is the critical path.** Every world beyond the three scripted ones needs `point_sets` / `point_set_points` authored from evidence, and `spawn_points` / `paths` are empty. Pertho and Dakara have no point sets, no spawn entries and (per the pack's own section 6) Pertho has no production UMAP in the uploaded set — so retargeting Asgard to Pertho is blocked on content authoring, not just on the registry fix.
5. **Harset mission 742, the pack's own canonical worked example, has zero chains.** It exists in `missions.sql` ("Giving the Walls Ears") with a python reference under `deprecated/python/cell/missions/Harset/`, but there is no Harset chain seed file on either branch — `db/resources/Content/Seed/` holds only `castle_cellblock_chains.sql`, `consumables_chains.sql`, `effects_chains.sql`, `sgc_w1_chains.sql` locally, plus the three Castle files on main. The Harset campaign (`docs/analysis/harset-rebuild/`) owns this.
6. **Two engine defects already force chain hand-splits.** Issue #657 (`active_objective_ids` persisted as `[step_id]` on relog) and issue #656 (`advance_step` emits no objective-update tick for implicitly completed objectives) have each forced manual chain splits, in missions 639 and 688. Any new multi-objective step in Phases 6-7 will hit them again.
7. **No QA item asserting mission XP or currency can pass.** Eight `mission_rewards` rows total.
8. **Doc drift to fix alongside this audit.** `docs/project-status.md:140` says 29 stargates; we have 28. `docs/content/mission-chains.md:677` still calls mission 687 the end of the CellBlock chain and `:793` calls mission 702 a dead end — packet CA16 in `docs/analysis/castle-rebuild/work-packets.md` owns that sync. `docs/content/README.md:46-48` still lists Castle as PARTIAL with two scripted missions. The stargate `address6` divergence needs a source label recorded somewhere in `docs/`.
