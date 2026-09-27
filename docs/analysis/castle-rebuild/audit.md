# Castle Rebuild Spec: Audit Against Cimmeria

> Type: reference. Audience: Claude Code coordinator and packet workers.
> Updated: 2026-09-17. Companions: [launch prompt and decisions](README.md), [packet ledger](work-packets.md), [Cellblock audit](../castle-cellblock-rebuild/audit.md).
> Baseline: `main` at `d91c5c8c`; Cellblock branch `content/castle-cellblock-rebuild` at `1d9f88dc`. Documentation only; no build, test or client run accompanies this audit.

## What The Spec Is And Where It Is Right

The input is `SGW_Castle_Rebuild_Spec_v21.xlsx` (Revision 2, 2026-09-13, 25 sheets). Unlike the Cellblock spec, it already knows `Castle.py` exists and treats it as recovered script; it correctly separates original data, recovered script, current Cimmeria behaviour and reconstruction, and it correctly places mission 688 in the Cellblock. Its evidence hierarchy (sheet `Evidence_Rules`) is adopted here as D-CA01. Its mission, dialog, dialog-set, spawn, region and item inventories were re-verified row by row against the seed and hold.

| Spec claim | Repository reality | Confidence |
|---|---|---|
| Missions 701-708 rows, steps, objectives and tasks | Match `db/resources/Missions/Seed/` exactly: missions.sql:1647-1657 (all `script_name NULL`, `reward_xp 0`); steps 2399/2400/2401/2421 (701), 2402/2419 (702), 2403/2404 (703), 2405/2406/2407 (704), 2411/2412 (706), 2415/2416/2417/2418/4462/4469 (708) at mission_steps.sql:5955-5991. Objectives per step and tasks 5701→4653, 6341→2784, 3448→2795, 6393→5185, 6394→5186, 6403→5198, 6404→5200 at mission_objectives.sql:6585-6641 and mission_tasks.sql. | HIGH |
| Optional/hidden objectives on 2415 and 2416 | Present in the seed: 2794 (opt, surrender), 2795 (opt, panel), 2796 (hidden) on 2415; 2797, 2798 (opt, Bravo), 2799 (opt, Muelbach) on 2416. One advisor pass reported these missing; the seed grep proves otherwise. | HIGH |
| Client PAK parity | `data/cache/CookedDataMissions.pak` carries every Castle step and objective id with matching optional/hidden flags. No `MissionOverride` needed. | HIGH |
| Dialog sets 649, 654, 656, 1571, 1572 exist; 5861/5862 have no set map | `dialog_set_maps.sql`: set 649 rows 3059 (2572, `?`), 3060 (2573), 3061 (2574, `!`), 3062 (NULL, `!`), 3063 (2576, `?` turn-in), 4961 (2575); set 654 rows 3071 (NULL, quest glow), 5711 (2584); set 656 rows 3073 (NULL, `INT_Dhd`), 5840 (5003), 5841 (5004), 5846 (NULL, glow), 5850/5851 (5008/5009, `!`), 5852-5855 (5010/5011), 5863 (NULL); sets 1571/1572 rows 5824-5832. Dialogs 5861/5862 exist (dialogs.sql:9925-9927) with no map row; the map ids 5861/5862 belong to set 1472 "Burn File". | HIGH |
| Static World 8 spawns (33 rows) and story actors absent | `spawnlist` world 8 as listed on the `Spawns_Entities` sheet; `entity_templates.sql` has no Zuritska/Romney/Muelbach/Ogilvie/Warden/Officer rows. Speakers: 1109 (blank, Gerschon), 1110 Copplemann (Castle), 1113 (blank) and 1114 Zuritska, 1093 (blank, surrender guard), 2490 (blank, Ogilvie), 261 Col. Marsh, 2499 Moh'katan. | HIGH |
| Named regions ThroneRoom / CastleFrontCourtyard / Infirmary exist only in `Castle.py` | They are seeded point sets: 2049 `Castle.ThroneRoom` (box x 331-396, y 41-48, z 617-683; the Access Panel at (330.5, 41.2, 653.1) sits on its west edge), 2050 `Castle.CastleFrontCourtyard` (box), 2051 `Castle.Infirmary` (one point at (371.0, 70.1, 904.6), on the 701 hallway route), 1002 `Castle.Stargate` (cylinder), 2081 `Castle.ArmoryRingDropZone` (Cimmeria-added, ring region 34). `point_sets.sql:43-47,77,132`, `point_set_points.sql:121-137`. | HIGH |
| Mission items are explicit grants, no loot tables | Items 5029, 2790, 2698, 2135, 2136 exist in `items.sql` with `container_sets {2}` (mission container). 2135/2136 carry a copy-pasted Ambernol description. Prison Boots exist twice (3438, 5865; see Cellblock C00). No `mission_reward_groups` for 701-708. | HIGH |
| Respawners at (0,0,0) | `respawners.sql:7-13`, four World 8 rows at the origin; world 12's row 5 has real coordinates, so this is a data gap, not convention. | HIGH |
| Event set 10011 / events 6100-6113 exist and are not emitted | `event_sets.sql:1351`, `event_sets_sequences.sql:3839-3865` (sequences 10145-10158). `grep 6100\|MakeGate crates/` is empty. | HIGH |
| World 8 config | `worlds.sql` row 8 matches the `World_Config` sheet. Nothing to do. | HIGH |
| 144 streaming sublevels | 146 files in the local map directory: `Castle.umap`, `Castle_MapData.upk`, 144 sublevels. | HIGH |

## Where The Spec Is Wrong Or Superseded

| Spec claim | Repository reality | Impact |
|---|---|---|
| Stargate event semantics "unresolved; do not assign guessed meanings" (sheets `Kismet_Cameras`, `Legacy_Unresolved`, `Server_Scripts` row 20) | The names are original data in four layers: `entities/defs/enumerations.xml:820-834`, `db/resources/Events/Types/ESequenceEventType.sql:56-69`, `deprecated/python/Atrea/enums.py:581-594`, and the 2009 server's emitter `deprecated/python/cell/SGWPlayer.py:2078-2129`: `beginDialing` starts a 4 s timer; `gateDialTimerExpired` sends `onSequence(Stargate_MakeGate)` (6100); `stargatePassed` sends `onSequence(Stargate_CrossGate)` (6113) then `moveTo`; `cancelDialing` never emits `Stargate_DestroyGate` (6103) and nothing emits the chevron events 6106-6112. `docs/gameplay/cinematic-system.md:254-267` already records this. | Gate presentation is a bounded engine packet (CA10), not an RE project. Emit 6100 and 6113 only. |
| Castle maps are external archives (`Castle(3).7z`) and `castle.nav` is absent from the game install | The cooked map tree is on this machine at `..\SGW\Stargate Worlds-QA\Working\SGWGame\CookedPC\Maps\Castle\` (the path issue #46 cites). No `.nav` exists for it anywhere; the five shipped `.nav` files are original 2012-2014 assets (`b1a6515a`). The server would load `data/spaces/castle.nav` (`space_manager/lifecycle.rs:27-28`). | `castle.nav` is producible (CA14) once extractor phases 1.3/1.4 land. Respawner and prefab positions are recoverable from the same files (CA05). |
| `set_visible` exists (sheet `Cimmeria_Current`, `Mission_Logic` row 4) | The arm sends `CellToBaseMsg::EntityMethodCall` keyed on the target id (`executor/world/mod.rs:315-325`); base routes that to the entity's own session (`cell_dispatch/aoi_dispatch.rs:365`, comment at `cell_dispatch/aoi.rs:461`: the map holds player entries only). For an NPC the packet is dropped. Even if fanned to witnesses, every AoI create packet appends `onVisible(1)` (`mercury/aoi/create.rs:234`), so hiding is undone on re-entry. | The Python's "hide static Copplemann" cannot port. CA11 fixes the primitive; D-CA02 chooses not to need it first. |
| `move_waypoint` exists but is unused; `set_follow_target` exists | `move_waypoint` is an instant grid snap plus validator reseed (`executor/world/mod.rs:337-344`, issue #616), not a walk; there is no NPC-arrival event (`ticks/npc_movement.rs:120-135` only logs `waypoint_reached`; `triggers/mod.rs` has no movement variant). `set_follow_target` cannot target a player (players have no `tag`) and the follower runs at 6.0 u/s against 8.125 (Cellblock GC1b-0 findings; `npc_ai/follow.rs`, `cell_entity/construction.rs:86`). | The 701 walk and the 704 escort are engine work (CA13, GC1b-0), not seed work. |
| `spawn_entity` is a blocker for every story actor | `Action::SpawnEntity`/`DespawnEntity` are declared (`content-engine/src/actions.rs:63,69`) with no loader verb and no arm. But no existing Cimmeria mission NPC is chain-spawned: `spawner::spawn_instance_npcs_from_records` inserts every `spawnlist` row at space creation and chains bind per-player state to those entities. Static rows for the story actors need no engine work (D-CA06). | Dynamic spawning is deferred (CA12); actors are seed rows (CA05). |
| `HasItem` "populator missing", implying a populator is the fix | The cell has no inventory at all: `CellEntity` carries only bandolier and loot (`entity_struct.rs:582-691`), and `InventoryItemGranted`/`Removed` from base are debug-logged and discarded (`base_messages/inventory_events.rs:52-81`). A bandolier-only populator would never see a mission item. | Step-state gates (D-CA08), the chain 1003 → 1005 pattern. |
| Dialog set 3062 binding is a plain port (`Mission_Logic` rows 1, 6) | `load_dialog_set_maps` drops every row with `dialog_id IS NULL` (`spawner/dialogs.rs:45`, pinned by `live_db_loaders.rs:269`). Rows 3062, 3071, 3073, 5828, 5829, 5846 and 5863 never reach the runtime cache; binding them is a warn and a no-op (`executor/dialog.rs:175`). `add_dialog_set` takes the `dialog_set_map_id` in `target_id` and the template in `params.slot` (chain 1001 → row 5229 → set 803). | CA02 or the sibling-row fallback (D-CA03). |
| `dialog_set.open` node ports to the `dialog_set_open` trigger | The trigger loads (`loader/trigger.rs:60`) and matches (`triggers/matching.rs:169`) but no `fire_dialog_set_open` exists in services. It has never fired. | Re-author on `interact_tag` + step conditions (D-CA04). |
| Mission 703 acceptance "must not be automatic unless proven" | `Castle.py` accepts only 702 at 2576 and never references 703, 704, 706 or 708 anywhere; `accepts_mission_id` is NULL on 2572-2577 and 5861/5862; 2576's only button is "Take Missions" (`dialog_screen_buttons.sql:4485-4489`, the one plural in the seed). | D-CA05 accepts 703 with 702, labelled reconstruction. |
| Dialog 5861 is a drop-in Jaffa equivalent of 2573 | 2573 carried an Accept button on every screen; 5861 had Accept on screens 96782-96786 only and none on 96787-96789. | D-CA13 uses it. The UI gap it filed is **RESOLVED** by packet DU-02b (2026-09-21): both dialogs now carry exactly one Accept, on their final screen. |
| Copplemann "pinned-hallway wave" negative evidence | Confirmed: the `interact_tag Castle_Coppleman` node displays 2574 on `2399 active` with no kill condition (`Castle.py:264-286`). | Nothing to author. T05 holds. |

## Live Defects A Castle Player Hits Today

| ID | Defect | Evidence | Confidence |
|---|---|---|---|
| B1 | Dying anywhere in Castle respawns at (0,0,0). `resolve_respawn_target` matches the World 8 rows by id or world name and returns their zeros; the safe fallbacks are unreachable because the rows exist. | `cell_methods/player/combat/respawn.rs:301-347`, `respawners.sql:7-13` | HIGH |
| B2 | `set_visible` on any NPC tag is a silent no-op (routing) and would be undone by AoI create (`onVisible(1)`). | see table above | HIGH |
| B3 | Interaction-only dialog-set rows are dropped at load; seven Castle rows are unbindable. | `spawner/dialogs.rs:45` | HIGH |
| B4 | A minigame session whose client never connects is never expired (`session.rs:27,86` set `created_at`; nothing reads it), so re-interacting hits the duplicate reject (`session.rs:90-93`, warn at `cell_dispatch/minigame.rs:82-85`) until relog. Castle adds three Livewire touchpoints to the Cellblock's three. `MinigameInstance::aborted()` has no call site; SWF close sends nothing (`minigame/server.rs:288-292`). | minigame advisor pass | HIGH |
| B5 | `start_minigame` difficulty is a literal 1 (`executor/mod.rs:213`); the original asserted 1-5. No chain-replay test pins any Livewire victory pair (1016/1017, 1041/1042, 1060/1061). | `castle_cellblock_chains.sql:317-326`, `base_messages/tests/minigame.rs` | HIGH |
| B6 | Region-1002 gate travel works but emits neither 6100 nor 6113. | `gate-travel.md:34-36` | HIGH |
| B7 | `move_waypoint` emits no position broadcast; witnesses learn the new position from the per-tick ghost relay only. **Resolved:** the snap is now broadcast per-witness as an immediate `EntityMoved` (bypassing the 100 ms AoI tick), so a chain-repositioned NPC moves on the next frame. | issue #616 | MEDIUM (unobserved in-client) |

## Engine Facts The Packets Rely On

- **Region entry** is client-hinted and keyed by `point_sets.name`; `fire_enter_region` (`event_dispatch/region.rs:26`) is called from the player world handler. Castle regions exist for the throne room, courtyard, infirmary and gate; the Interrogation Block, Communications room, Checkpoint Bravo and the bunker need new point sets (CA05).
- **Death by tag** reads the live entity's tag (`abilities/use_ability/kill_credit.rs:88-91`) and fires `fire_entity_death` (`event_dispatch/lifecycle.rs:66-73`); the killer must have a `player_id`. Works for static and dynamic NPCs alike.
- **AoI introduction** of a newly inserted entity is covered by the ordinary tick (`space_manager/aoi.rs:19-33,60-103`); the `#582` seams report emit success.
- **Mission progression**: `advance_step` is unconditional and force-completes the current step's objectives (`missions/progression.rs:20-129`); `complete_objective` on the last non-optional objective calls `mission.complete()` (`progression.rs:176-213`). Step 2417 therefore needs one archetype-gated chain that completes the matching objective (5185 or 5186) and advances to 2418 in the same action list; completing both objectives ends 708 at 2417.
- **Minigame victory** fires `fire_chain_by_id` with `ResolvedActions::default()` (`event_dispatch/mod.rs:53-82`): no conditions are evaluated on the victory chain. Defeat (code 2) carries no chains; retry works because the session is removed on exit (`server.rs:367`). Only Livewire is real (`minigame/games/mod.rs:11-23`); the rest are auto-win placeholders. `remove_dialog_set` takes `dialog_set_id` (the map id) plus `slot` (template), so `removeDialog(48, 3062)` maps 1:1.
- **Deferred actions** (`delay_ms`, Cellblock C08a, `executor/deferred.rs`, `space_manager/deferred_content_actions.rs`) exist on the Cellblock branch, not on `main`.
- **Navmesh absence** fails open: `find_path` returns `None` (`space_manager/spatial.rs:49`), follow pushes the destination as a single waypoint (`npc_ai/follow.rs:110`), `is_position_valid` passes (`spatial.rs:65`).
- **The gate emitter template** is `ring_transport/wire_helpers.rs:33-71` (`build_on_sequence_args`, `send_play_sequence`, `ON_SEQUENCE`, `KISMET_VIEW_EventInvoker`), already used for `Region_Teleport_Out/In`.
- **Open PRs**: #618 adds `Action::MoveEntity` (player path via `transport::teleport`, NPC path via `move_waypoint`, cross-world guard) and `grant_xp`; #619 (stacked) adds `LaunchAbility`/`ApplyEffect` through a non-public `effect_apply.rs`. Both extend chain-replay tests to execute actions, and TESTING.md type 6 with them.

## Mission 701 Port Table (`Castle.py` → chains)

| `Castle.py` node | Chain shape | Status |
|---|---|---|
| `player.loaded`, 701 not active → `addDialog(149, 3062)` | `player_loaded` key `Castle` / `mission_status 701 eq not_active` / `add_dialog_set 3062 {"slot":149}` | Blocked by B3 (D-CA03) |
| `client_hinted_region` ThroneRoom/Courtyard/Infirmary → `onSystemCommunication(11, 5189/5190/5188)` | `enter_region` + `system_message` | Authorable; `system_message` arm is log-only (issue #268). Out of scope. |
| `interact.tag Castle_SgtGerschon`, 2399 not active → display 2573 | `interact_tag` / `step_status 701 2399 eq not_active` / `display_dialog 2573` (+ archetype split for 5861) | Portable |
| `dialog.choice 2573` → accept 701, `removeDialog(149, 3062)`, `addDialog(48, 3062)` | `dialog_choice 2573` / `accept_mission 701`, `remove_dialog_set`, `add_dialog_set` | Portable modulo B3 |
| `interact.tag Castle_Coppleman`: 2399 active → display 2574; 2400 active → Livewire | two chains; `start_minigame Livewire {"on_victory_chains":[N]}` gated on the launcher | Portable (chain 1016/1017 precedent) |
| `dialog.choice 2574` → advance 2400 | `advance_step 701 '2400'` | Portable |
| Livewire won → `removeDialog(48, 3062)`, display 2575, advance 2401 | victory chain (no conditions) | Portable |
| `dialog.choice 2575` → create clone tpl 48 at (354.419, 70.272, 952.801), hide static, walk to (358.809, 70.156, 889.724), on arrival `addDialog(48, 3062)` | none today | Blocked (B2, no spawn arm, no arrival event); D-CA02 |
| `dialog_set.open 3062`: 2401 active → advance 2421; 2421 active → display 2576 | never fires | Re-author on `interact_tag` + step (D-CA04) |
| `dialog.choice 2576`, 2421 active → `removeDialog(48, 3062)`, complete 701, accept 702 | `dialog_choice 2576` / `step_status 701 2421 eq active` / actions | Portable (+703 per D-CA05) |

Relog restore chains (precedent 1006/1007): 701 not active → bind on 149; 2399, 2400 and 2421 active → bind on 48; 2401 active is unrecoverable under option C (the clone is gone) and trivial under option A (the deferred advance re-arms from the restore chain).

## Missions 702-708: What Exists To Reconstruct From

| Mission | Steps and objectives | Dialogs and sets | Actors and props | Reconstruction notes |
|---|---|---|---|---|
| 702 Rescue Dr. Zuritska | 2402 (2778; 2779 opt hidden) → 2419 (4653, task 5701) | 2577 (rescue) | Zuritska (speaker 1114), Interrogation Room prefabs 01/02, a cell control | Free by interact on the cell actor; 2577 on the choice completes 702 |
| 703 Payback | 2403 (2780) → 2404 (2781) | none specific; legacy 522-524 excluded | Romney (item 2135 identifies him) | `entity_death` by tag completes; 2135 optional explicit grant |
| 704 Hack Communications | 2405 (2782; 2783 opt hidden) → 2406 (2784, task 6341) → 2407 (5151) | 4866 (comms room), 2580 (terminal result), 2581 (briefing) | Zuritska at workstation (speaker 1113), a comms terminal | Escort, Livewire, `add_item 5029`, delivery completes 704 and accepts 706 |
| 706 Power Behind the Throne | 2411 (2790) → 2412 (2791 use, 2792 locate) | 2584 via set 654 | `Castle_AccessPanel` (spawn 92, template 147) at (330.49, 41.18, 653.11); region 2049 | Region enter advances 2411; panel interact completes 2412 objectives, plays 2584, completes 706, accepts 708 |
| 708 Secure the Stargate | 2415 (2794 opt surrender, 2795 opt panel, 2796 hidden) → 2416 (2797; 2798 opt Bravo, 2799 opt Muelbach) → 2417 (5184; 5185 Marsh; 5186 Moh'katan) → 2418 (5197) → 4462 (5198, task 6403) → 4469 (5200, task 6404) | 5003 (surrender), 5004 (panel diagnostic), 5008/5009 (Alpha by archetype), 5010/5011 (final lines), 2586 (dial); set 656 | surrender guard (speaker 1093), Bravo officers, Muelbach (item 2136), `Castle_ColMarsh` (spawn 118, tpl 10), `Castle_Mohkatan` (spawn 120, tpl 54), `Castle_DHD` (spawn 2, tpl 162) at (806.27, 55.10, 517.24), gate at (761.68, 63.47, 551.72), region 1002 | Either diagnosis route advances 2415; Bravo or Muelbach death grants 2790 and advances 2416; archetype-gated report advances 2417; DHD Livewire advances 2418; dial advances 4462 and starts the 4 s timer to 6100; region 1002 fires 6113, completes 708, travels |

The Marsh at Checkpoint Alpha is the present-day Marsh (spawn 118, "Col Marsh (pet)"), not the future Marsh who dies in the Cellblock; dialog 5008 ("You saw my future self die") is the canonical bridge.

## Evidence The Map Files Can Still Give

A plain-string scan of the 146 `.umap` files found: `SymbioteChamber` in `Castle-00090003`; `Infirmary` in `00040009`, `0004000a`, `00070006`; `Humvee` in `00060006`; `SecurityLock` in `00080002` and `00090004`; a `ThronePilla` name in `00050007`; `Bunker00`/`BunkerTunL` in `00040009`/`0004000a`; `Stargate_Prefab` only in `Castle.umap`. No `Respawn`, `Checkpoint`, `Interrogation` or `Bravo` strings appear, so either those actors are named differently or the relevant packages are compressed. CA05 uses `crates/upk-objects` to enumerate actors properly rather than trusting the scan.

## Explicit Non-Goals

| Item | Why |
|---|---|
| Recreating mission 688 or the Armory in World 8 | Cellblock-local; chain 1109 already hands off. |
| Chevron-lock events 6106-6112 and `Stargate_DestroyGate` 6103 | The 2009 server never emitted them. |
| Legacy Romney laundry-crate flow (dialogs 522-524) and the security-grid/MALP revision (963-968) | Older story revision; keep disabled. |
| Humvee disable (dialog 2585) | No mission link recovered. |
| Castle character VO, new cinematics | No assets; new content if ever. |
| Mission XP | Same as Cellblock GC3; `grant_xp` arm arrives with PR #618, the formula does not. |
| Owner-scoped (per-player) entities | Design gate GCA1; no engine support and no packet needs it under D-CA06. |
