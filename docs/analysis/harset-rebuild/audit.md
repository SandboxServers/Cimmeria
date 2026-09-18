# Harset Rebuild Spec: Audit Against Cimmeria

> Type: reference. Audience: Claude Code coordinator and packet workers.
> Updated: 2026-09-17. Companions: [launch prompt and decisions](README.md), [packet ledger](work-packets.md).
> Baseline: `main` at `d91c5c8c`, branch `content/castle-cellblock-rebuild` at `1d9f88dc`, PRs #618/#619, inspected 2026-09-17. Documentation only; no build, test or client run accompanies this audit.

## What The Spec Is And What It Gets Right And Wrong

The input is `SGW_Harset_Complete_Rebuild_Spec_v2.xlsx` (31 sheets: a 24-sheet base plus seven `_v2` sheets on progression, offworld handoffs, class skills, vendor tiers, item crosswalk, asset restore and build order). It reconstructs the Praxis hub for all three factions on the premise that the original server scripts are missing. Unlike the Castle Cellblock spec, that premise is essentially correct here.

| Spec premise | Repository reality | Confidence |
|---|---|---|
| Original Harset server scripts are missing and 36 missions must be reconstructed from DB step rows | True for 35 of 36. The Atrea tree is complete (18 mission + 10 space scripts) and Harset holds exactly three: `deprecated/python/cell/spaces/Harset.py` (70 lines: five ring switches and the Command Center door), `Harset_CmdCenter.py` (30 lines: the return door), and `deprecated/python/cell/missions/Harset/GivingTheWallsEars.py` (261 lines, mission 742) with its `.script` source. A grep of all of `deprecated/` for the other 35 mission ids and 15 NPC names finds nothing. | HIGH |
| `spaces/Harset_Market.script` and `Harset_StorageRm.script` may have existed | They did not. `worlds.sql` has `has_script = false` for 69 and 70, and no file exists. Market and Storage transitions are new authoring. | HIGH |
| Command Center return handler and target are unresolved | Resolved by the Python: `Harset_CmdCenter.HarsetTransition` moves the player to Harset at (0, -67.600, -231). The entry target is (0, 0.355, **-20**), not the spec's -25. Both arrivals sit 7-9 units outside the opposing trigger box, so there is no ping-pong by construction. | HIGH |
| "All DB mission steps are disabled; server logic must enable them" | Literally true (139/139 steps, 176/176 objectives `false`) and irrelevant. The loader in `crates/services/src/cell/spawner/missions.rs:54-90` has no `WHERE` on any enabled column, and every working, chain-replay-tested Cellblock mission (622, 638, 639, 641, 682, 687, 688) has 0 enabled steps too. Seed-wide the flag is 1,497 true / 1,982 false. **No step-enabling work exists.** | HIGH |
| `mission_tasks.task_type` distinguishes talk/kill/use/region objectives | All 4,358 task rows in the seed are `task_type = 1`. The Rust server never reads `mission_tasks`. The `KillCount/CollectItem/...` enum in `crates/game/src/missions/objectives.rs` is dead code. All progress is hand-authored chains. | HIGH |
| Harset_CmdCenter (68), Market (69), StorageRm (70) are instanced because `worlds.flags = 1` | `worlds.flags` is dead data; instancing comes from `entities/spaces.xml`. 68 is **`Instanced="false"`**, a shared startup space (also listed in `cell_spaces.xml`). 69 and 70 are instanced, **one instance per player** (`space_manager/lifecycle.rs:67-99`). There is no party system anywhere in `crates/`. | HIGH |
| Static spawns: 22-23 rows on 57, Anat on 68, none on 69/70 | Exact. 22 rows on world 57, 1 on 68, 0 on 69 and 70 (`spawnlist.sql`). Every coordinate in the spec's Spawns_Entities sheet matches. | HIGH |
| Ring network regions 4-8, all-to-all, event sets 935-939, point sets 2052-2056, tag typo `HarsetinRingRightRegion` | Exact. The typo is cosmetic: `ring_transport_regions.tag` is loaded but never wire-encoded or string-matched; routing is by `point_set_id`. | HIGH |
| Stargate 3, address 21-17-24-31-10-30, origin 6, position (-0.076, -67.274, 38.011), yaw 3.141, event set 25 | Exact (`stargates.sql:39`). | HIGH |
| Speaker ids: Moh'katan 945, Mala'c 957, Marsh 941, Anat 944, arrival NPC 3219 | These are `speakers.speaker_id`, not template ids. 941 is the Cellblock Marsh (template 10). 3219 has an empty name. | HIGH |
| Petbe template 163, Anat 43, Ba'al 42, Nerus 53, Moh'katan 54, Lethander 46, Goldam 56, Marsh 10 | All exist. Only 43 and 163 are spawned in Harset. 54 and 48 (`CaptCoppleman`) are spawned only in world 8 (Castle). 42, 46, 53, 56 have no spawn anywhere. | HIGH |
| Hansen, Jacobs, Lo'rak, Mala'c, Opheltes, Blackstock, Grogan, Dawson, Bra'hin, Royal Guard, NID Operative, Jaffa Volunteer, PvP Registrar, Storage Lo'taur and ~60 vendor/trainer declarations exist as content | They exist as `texts.sql` display-name monikers and, for some, as dialog speakers. **None has an `entity_templates` row.** | HIGH |
| Vendor inventories and prices are missing | Correct, and the gap is system-wide: `item_lists.sql` has exactly two test rows and template 25 (a debug NPC) is the only vendor or trainer in the whole seed. Vendor and trainer Rust is complete. | HIGH |
| Harset has no random loot | Correct: `loot_table_id` is non-NULL on 2 of 153 templates, neither Harset. | HIGH |
| Duplicate item designs are unresolvable | Four of five groups resolve from `items_event_sets` bindings and the 742 script (see [Items](#items)). | HIGH |
| Only Stargate and ring Kismet sequences are registered for Harset | Correct: sequences 113-126 and 2045-2054 only. | HIGH |
| Petbe/Lethander per-player state needs a "Player-Only Property Update" | Cimmeria has no per-observer entity state. Dialog-set bindings are per-player; everything else on an NPC (name, faction, aggression, visibility) is one value broadcast to every witness. | HIGH |

Archetype ids (`EArchetype`): 0 Any, 1 Soldier, 2 Commando, 3 Scientist, 4 Archeologist, 5 Asgard, 6 Goauld, 7 Sholva, 8 Jaffa. Harset faction gating in chains is by archetype: `eq 8` Jaffa, `eq 6` Goa'uld, `neq 8 and neq 6` Human (Praxis Goa'uld reach Harset by a non-Castle path per the spec's Start_Profiles).

## Evidence Precedence Used Here

1. Original Python and Atrea scripts (`deprecated/`), ground truth for the three things the 2009 server did in Harset.
2. Cimmeria's seed (`db/resources/`) and content engine, ground truth for what runs and what can be authored today.
3. The spec, for step order, NPC roles, dialog ids and acceptance tests. Rows marked INFERRED, PLACEMENT-INFERRED or UNRESOLVED are proposals.

## Live Defects Found During The Audit

These are bugs in the current tree or seed that the campaign hits first, not spec gaps.

| ID | Defect | Evidence | Confidence |
|---|---|---|---|
| H-B1 | **Gate arrival at Harset is off-navmesh and unrecoverable.** `handle_dial_gate` (`cell/gate_travel.rs:49,104`) places the player on the stargate row's own coordinate, the prefab origin. `harset.nav` has no vertex within 5 units of it (the `GLB-Stargate_Prefab_Seq` footprint carve-out) and the row's y is 1.5 units above the floor. `is_point_valid` searches ±3.0 horizontally (`navigation/mod.rs:43,78-82`) and cannot reach the mesh. Recovery has no candidate: `get_nearest_point` returns the input unchanged on miss (`mod.rs:466-468`), world 57 has no respawner (`respawners.sql` covers worlds 8, 12, 23 only), and the AABB clamp is a no-op. Result: `CorrectionSuppressed` from the first packet, a silent freeze for witnesses while the player's own client moves normally. Castle has no navmesh so this never fired there; Harset is the first navmesh-backed gate destination. | Direct decode of `data/spaces/harset.nav`; `client_move.rs:387-467`. | HIGH |
| H-B2 | **Walking into the stargate does nothing.** `REGION_FLAG_Stargate` (bit 2) is never tested; only `REGION_FLAG_CLIENT_HINTED = 1` exists (`space_manager/mod.rs:41`). `stargateRegionTriggered` exists only as a string in `generic_regions.sql:41`. `onDisplayDHD` (method 120) is never emitted and there is no DHD interaction handler (`cell/interactions/mod.rs:9-13`). Only the client-initiated `onDialGate` message works. | grep; `cell_methods/player/world/mod.rs:100-107` | HIGH |
| H-B3 | **Ring FSM has no timeouts in four states and no disconnect cleanup.** `SendWait`, `RemoteLoadWait`, `RecvWait`, `RecvWarmup` never expire; `destroy_entity` never touches `ring_transporters`. A player who drops mid-transport leaves `num_remote_players` unsatisfiable and `handle_select_destination` refuses a non-`Idle` destination (`ring_transport/runtime.rs:305-321`), so one stall removes that ring from all four peers in an all-to-all mesh. Three code comments assert a timeout that does not exist (`dispatch.rs:96,103-104,156-157`). | `ring_transport/transporter/mod.rs:45-51` | HIGH |
| H-B4 | **The `dialog_set_open` trigger never fires.** Authorable (`loader/trigger.rs:60`) and matched, but no `fire_*` site constructs it. Mission 742's bug-planting loop and the 2009 `Event_DialogSetMap` node compile to it. Also: `dialog_set_maps` rows with NULL `dialog_id` (the topic/indicator carriers, including 742's `1000000`) are dropped at load (`spawner/dialogs.rs:28`). | agent memory `dialog-set-engine-gaps`; `spawner/tests/live_db_loaders.rs:269` | HIGH |
| H-B5 | **`set_visible` is a wire no-op for NPCs.** `executor/world/mod.rs:305-326` sends `CellToBaseMsg::EntityMethodCall` keyed on the target, which routes to the target's owning client; NPCs have none (`base/world_entry/cell_dispatch/aoi.rs:466-467`). The existing test only asserts enqueue. | code read | HIGH |
| H-B6 | **Content `destroy_entity` skips witness cleanup.** `executor/world/mod.rs:204-216` calls bare `SpaceManager::destroy_entity`; GM `.despawn` uses `despawn_npc` (`space_manager/entities.rs:191-274`) which fans `LeftAoI` immediately and scrubs witness sets. Content-driven despawns reproduce the #582 invisible-corpse shape. | code read | HIGH |
| H-B7 | **Every Harset NPC is one-shot.** `respawn_secs` is NULL on all 153 templates and all 167 spawn rows (the seed INSERT column lists omit it), so `mark_npc_dead` never stamps `respawn_at` (`combat/state.rs:110-112`). Kill the eight Praxis guards and the plaza is empty until restart. Same for `patrol_path_id` (0 rows), `wander_radius` (0), `is_stationary` (1 row, Cellblock). | seed read | HIGH |
| H-B8 | **Harset mobs fire a pistol.** `ability_set_id` is set on 3 of 153 templates; every other NPC falls back to `NPC_DEFAULT_ABILITY = 592` (Pistol Shot, `combat/threat/aggro.rs:19`). Jaffa guards with staff models shoot pistols. Petbe has NULL faction, level and alignment. | seed read | HIGH |
| H-B9 | **`harset.nav` is fragmented.** 39,652 vertices, 19,345 polygons, **1,939 connected components**; the largest island is 3,088 polygons (16%); 671 singletons; 52.7% of edges have no neighbour. Of 12 checked spawn and arrival coordinates only 3 are on-mesh, and one of those (the DHD) sits on a one-polygon island. `harset_storagerm.nav` is similar (104 components). No mesh exists for worlds 68 or 69. NPC chase has no straight-line fallback (`npc_ai/fight.rs:405`, the #407 `no_path` outcome): an NPC that aggros off-mesh or across a component boundary freezes. Player movement in world 57 hard-rejects off-mesh (`client_move.rs:295`); in 68/69 it fails open with a 20 km bounds box. The regeneration pipeline exists (`crates/navmesh-extractor` plus the C++ NavBuilder, issue #46) but its terrain and BSP decoders are unfinished follow-ups; Castle CA14 is scheduled to finish them, and the Harset rebuild is GH1 (U15). | direct decode; flood fill; `navmesh-extractor/README.md` | HIGH on the numbers, LOW on cause |
| H-B10 | **`space_castle_cellblock_chains.sql` fires in Harset.** Its 28 `space:8` chains have NULL `event_key`, and `scope_id` is never read by the runtime matcher (only by `admin-api/routes/editor.rs:159-167`), so they run on `player_loaded` in every space. Cellblock C01 deletes the file on the integration branch; not yet on `main`. | code and seed read | HIGH |
| H-B11 | `Harset_Market` has a degenerate AABB (`spaces.xml:15`, all four bounds 0). Inert today because `WorldDef.min_x..max_y` are parsed and never read (movement bounds come from the navmesh or `SpaceBounds::FALLBACK`), but it rejects every position the day someone wires those fields. | `space_manager/xml.rs:87-97` | HIGH |
| H-B12 | `known_stargates` is stored and sent but never written and never checked on dial (`cell/gate_travel.rs:49` hits the global cache). A crafted `onDialGate` reaches any gate. The 2009 server enforced it (`deprecated/python/cell/SGWPlayer.py:2060-2064`). Server-authority finding CAT-O-01 already covers it. | code read | HIGH |
| H-B13 | `crates/admin-api/src/routes/content.rs:310` selects `step_name` and `sort_order` from `mission_steps`; neither column exists (`step_display_log_text`, `index`). Fails at runtime. Not Harset-specific, found in passing. | schema read | HIGH |
| H-B14 | Moh'Katan (template 54) has body set `BS_JaffaFemale` with male `AR_JM_*` components. Probable asset bug in the original data. | `entity_templates.sql:63` | MEDIUM |
| H-B15 | Mission 742's per-bug `Act_RemoveItems(2820)` node is orphaned in the compiled Python (`GivingTheWallsEars.py:128-135`, the Player port was never wired); the player keeps all three devices. `docs/content/mission-chains.md:1017` says they are consumed; that line is wrong. Template 164 is a merchant basket prop, not an NPC. | script read | HIGH |

## Engine Gaps, Ranked By Missions Blocked

Capability inventory as of `main` at `d91c5c8c`: 22 authorable triggers, 6 authorable conditions, 28 authorable-and-executed actions; PR #618 (`f23e73fb`, merged after the evidence passes) adds `grant_xp` and an executor arm for `move_entity`, making it 30. Full catalog in [content-engine.md](../../content/content-engine.md#3-the-vocabulary); dispatch-side gaps not listed there are in H-B4 above.

| # | Gap | Harset missions blocked | Where it lands |
|---|---|---|---|
| 1 | **No NPCs and no `spawn_entity`.** ~22 story NPCs have no template; 8 templates have no spawn; worlds 69/70 have zero spawns. `Action::SpawnEntity`/`DespawnEntity` are enum ghosts (no loader arm, no executor arm, `actions.rs:63,69`). Content chains have never created an entity; the Cellblock pattern is pre-spawn everything and bind dialogs by template. | all 33 in-zone; even 742 cannot complete (SecondBug, ThirdBug, Nerus missing) | H03 (action), H11-H14 (templates and spawns), M0 (coordinates) |
| 2 | **No named regions.** Point sets for Harset: the gate, five ring pads, two transition boxes. No Jaffa Zone, OpCORE Zone, Market, Storage, Shield Towers, Bar, Bank, Petbe's quarters, holding pens. `generic_regions` has 0 Harset rows. | 1343 (3 patrols), 1241 (4 scans), 1243 (6 anchors), 1362 (4 anchors), 1374 (4 scans), 1240 (3 towers), 1322, 1352 | H15 |
| 3 | **No reward values and no cash grant.** `grant_xp` is authorable and executed since `f23e73fb` (PR #618); no cash action exists; all 36 have `reward_xp = 0`, `reward_naq = 0`, so nothing tells a chain what to grant. | all 36 rewards | GH3 formula gate (Cellblock GC3) |
| 4 | **No per-player entity state** (H-B5, shared `aggression: i32`, broadcast name/faction). | 1245, 1246, 1352, 741, 1348, 1241, 1580 | D-H03: instancing via H03 |
| 5 | **No item-use-at-anchor or use-item-on-target.** `item_use` carries only `item_id`; `Condition::InRegion` and `HasItem` are unauthorable. | 742, 1352, 1362, 1243, 1410 (plant), 1363, 1365, 1374, 1377, 1240, 1241 (use-on) | authoring workaround: `interact_tag` per prop or NPC gated on `objective_status`; fidelity note per packet |
| 6 | **`dialog_set_open` never dispatched; NULL-dialog dsm rows dropped** (H-B4). | 742 and any Atrea `Event_DialogSetMap` port | D-H05 workaround |
| 7 | **`launch_ability`, `apply_effect`, `remove_effect` unarmed.** | 1325 (movement lock), 1365 (Tollan extraction), 741 (infect Dawson), 1353 (implant) | U1 (PR #619) |
| 8 | **No health-threshold trigger; `AiState::Submit` unreachable from damage.** | 1325 | H04 |
| 9 | **No timers; `delay_ms` discarded on `main`.** | staged scenes in 1348, 1241 | U2 (Cellblock C08a) |
| 10 | **`move_waypoint` snaps, discards `speed`; `set_follow_target` cannot target a player.** | 1372 (Follow Lethander) | Castle CA13 (U14); GH5 is a pointer |
| 11 | **Minigame `difficulty` and `tech_competency` hardcoded to 1; only Livewire is real.** | 1377, 1244 (placeholder by 2009 design) | H05 |
| 12 | **Stargate flag handler and DHD interaction absent** (H-B2). | all arrivals in fiction | H01 |
| 13 | **Gate arrival unvalidated** (H-B1). | all arrivals | H01, M0 |
| 14 | **Ring FSM timeouts** (H-B3). | all ring travel | H02 |
| 15 | **No bank service.** "Storage Lo'taur (Bank)" has a moniker and a `BANK` container (id 17) but no deposit/withdraw handlers. | none (service NPC) | out of campaign |
| 16 | **Duel system is a stub; Converse is a placeholder auto-win.** | 1325 (rerouted, D-H11), four social checks (rerouted, D-H12) | none |

## Row-By-Row Audit Of The Spec's Mission_Logic Sheet

Status vocabulary: **RESTORE** (a script exists and Cimmeria does not run it), **NEW** (no script; needs authoring against DB steps and dialogs), **OUT** (excluded by decision). Nothing is DONE. `Steps` is the DB step count; every step is `step_enabled = false`, which is not a signal (see premise table). `Needs` lists the non-chain prerequisites: NPC templates or spawns (T), regions (R), engine gaps by number from the table above (E), items (I). Packet ids refer to [work-packets.md](work-packets.md).

### Loyalist Jaffa

| Mission | Steps | Needs | Notes | Status | Packet |
|---|---|---|---|---|---|
| 1324 Present Yourself (L6) | 2: talk Ba'al 3953, return Moh'katan 3954 | T: Ba'al 42 and Moh'katan 54 spawned in 68; dialogs 4357/4358/4363 bound | Pure talk chain. Council dialog 4363 (17 screens, speakers 942/945/941/944) exists at `dialog_screens.sql:10191-10224`. | NEW | H20 |
| 1325 Rin'la (L8) | 4: challenge Mala'c 3957, position 3958, commence 3959, return 4036 | T: new Mala'c template, stationary, in 69; E7 (`apply_effect` Stun), E8 (health trigger); dialogs 4368-4372 | Ritual, not death. Mala'c speaker 957 exists. | NEW | H21 |
| 1326 Lan'toc (L10) | 2: present Lan'toc 3960, return 4603 | T: former-Ra Jaffa templates (new) in 57 Jaffa Zone; dialogs 4373-4376 | Accept/reject is two `dialog_choice` chains ending in `complete_objective`; per-player, shared NPC untouched. Free pattern. | NEW | H22 |
| 1343 Enemies Within (L24) | 5: patrol Jaffa Zone 3974, Storage 3975, Market 3976, kill Ra's Jaffa 5342, report 3977 | R: Jaffa Zone; T: Ra's Jaffa hostile templates in 69/70 instances; E1, E4; dialog 4445 area | First combat mission; needs GH1 navmesh outcome for chase. | NEW | H23 |
| 1347 Divided Loyalties (L36) | 3: question Lo'rak or Hansen 3990 (5970 optional), question Lethander 3991, report 3992 | T: Hansen, Lo'rak (new), Lethander 46 spawned; dialogs 4424/4426 | Talk chain with an optional branch: `complete_objective` on 5970/5971, `advance_step` after. | NEW | H24 |
| 1348 Shut Down Lethander (L38) | 4: find Lethander 3993, hold off Marketplace assault 3994, talk 3995, report 3996 | T: Free Jaffa attacker templates in 69 instance; E1, E4, E9 (waves); dialogs 4430/4432 | Wave shape unrecovered; author a fixed count with `delay_ms` spacing and record it as design. | NEW | H25 |
| 1351 Counter Intelligence (L42) | 3: approach Opheltes 4005, convince 4006, report 4715 | T: Opheltes (new) in 68; dialog 4442 | Convince is a dialog choice (D-H12). | NEW | H26 |
| 1352 Murder Spree (L41) | 5: plant device Market 4007, Bar 4008, Storage 4009, take care of Crogan 5174 (4 objectives, 3 optional incl. "hide body" 5998), report 4034 | R: Market, Bar, Storage anchors; T: Crogan (new) in instance; I: 2734 Monitoring Device; E1, E4, E5 | "Hide Crogan's body" has no primitive; treat as `destroy_entity` on the corpse inside the instance and record the fidelity gap. | NEW | H27 |
| 1353 Transplant (L46) | 7: gather shards on Agnos 4010, symbiote from Anat's tank 4011, take to Nerus 4012, buy confection 4013, deliver 4712, rendezvous volunteer 4713, report 4714 | T: Nerus 53 spawned, Jaffa Volunteer (new), Anat's tank prop (unresolved actor); I: 5760/5763/5766 shards, 2818 symbiote, 4512 or 2714 confection; E7 (implant effect), GH2 (confection purchase) | Agnos leg is out of zone: step 4010 is satisfied by a scripted grant at a Harset NPC until an Agnos campaign exists (record as deviation). "Buy" a confection needs a vendor or a grant. | NEW (partial) | H28 |

### OP-CORE Human

| Mission | Steps | Needs | Notes | Status | Packet |
|---|---|---|---|---|---|
| 1360 Frost's Letter (L1) | 2: 4037 (Cellblock, U7), **4038 give letter to Marsh** | T: Marsh 10 spawned in 68; I: 3730 (granted by Cellblock chain 1003) | Harset owns 4038 only. `remove_item 3730` and `complete_mission`. | NEW | H30 |
| 567 Romney's Files (L1) | 3: retrieve 2000, get off Castle 2012, **deliver to Copplemann 4039** | I: 2698 has no grant path anywhere; the Castle ledger's CA06 explicitly excludes it ("no acquisition evidence"); T: Coppleman 48 spawned in 68 | Harset can own 4039 only if a Castle packet ever grants the item (U17). Included conditionally per D-H02. | NEW (conditional) | H30 |
| 1361 Meet The Praxis (L1) | 6: talk Moh'katan 4040, convince Hansen 4041, deliver samples 4042, talk Ba'al 4043, talk Anat 4693, return Marsh 4694 | T: Moh'katan 54, Hansen (new), Ba'al 42, Anat 43 (exists), Marsh 10; dialogs 4363 council | Convince Hansen is a dialog choice (D-H12); "weapon samples" is an item grant on the choice (item id unrecovered; the spec's Items sheet has none; record as design). | NEW | H31 |
| 1362 Security (L1) | 2: install 4 monitoring devices 4044 (objectives 4658-4661: Operations Center, Research Facility, Guardhouse, Science tent), return Marsh 4045 | R: four OpCORE anchors (props or regions); I: 2734; dialog 4468 | Four `interact_tag` anchors each `complete_objective`; `advance_step` when all four; relog restore of the anchor bit. | NEW | H32 |
| 1363 Prudence (L1) | 2: tag Nerus, Petbe, Lo'rak, Athena, Lethander 4047 (5 objectives), confirm with Blackstock 4048 | T: all five targets spawned (Athena 44 has no spawn; Petbe 163 exists), Blackstock (new); I: 4690 Nanite Tracking Gun (bound to ability 2409 "Use Tagging Gun" in legacy data) | Use-on-NPC becomes `interact_tag` on each target gated on `objective_status` while 4690 is held (no `HasItem` condition; gate on step instead). Must not aggro (spec M-10). | NEW | H32 |
| 1365 Tollan Tech (L1) | 4: report Marsh 4050, test troopers in Storage 4051 (3 objectives, 2 optional), kill Dawson and extract symbiote 4052, report 4053 | T: Dawson (new) and troopers in 70 instance; I: 2743 Tollan Control Technology, 2720 Dawson's Symbiote; E1, E4, E7 | Legacy effect 3472 "Use on Dawson: detects Goa'uld, prompts hostility switch" is the model. | NEW | H33 |
| 1371 Profiling (L18) | 3: arrest Ra's former Jaffa 4073 (objective 4698 has **zero tasks**, defect H-D1), kill Bra'hin 4074, report Marsh 4075 | T: Bra'hin (new), suspects in 57 Jaffa Zone; R: Jaffa Zone; E1 | "Arrest" is a dialog-choice on tagged suspects. | NEW | H37 |
| 1372 Tail (L20) | 3: follow Lethander 4077, confront 4078, report Blackstock 4699 | T: Lethander 46, Blackstock (new); E10 (NPC walks a route) | Blocked on GH5. | NEW | H37 |
| 1374 Security Holes (L24) | 5: scanner at Shield Controls 4087, Bank 4088, Market 4089 (+talk Haughty Goa'uld 4719), Storage 4090 (+soothe Angry Jaffa 4721), report Blackstock 4091 | R: Shield Controls, Bank, Market, Storage anchors; T: Haughty Goa'uld, Angry Jaffa (new); I: 4396 Straegis Scanner (ability 2092 "Use Straegis Scanner") | Soothe is a dialog choice (D-H12). | NEW | H36 |
| 1375 Moles (L26) | 5: ask Praxis members 4093, find Lethander in Storage 4094, talk 4095, eliminate NID in Market 4096, report Copplemann 4092 | T: Lethander spawned in 70 instance for this step, NID Operative templates (new) in 69 instance; E1, E4 | Lethander appears in Storage for this mission only: a mission-scoped spawn (D-H03). | NEW | H34 |
| 1377 Replitech (L31) | 3: hack Devlin's device 4099, use device on 3 Replitech crates 4100 (3 tasks), report Copplemann 4101 | T: Devlin's device and 3 crate props (new templates) in 70; E11 (Livewire difficulty) | Livewire pair per Cellblock 1060/1061; crates are `interact_tag` x3 with a counter. | NEW | H35 |
| 1401 Letters Home (L38) | Beta Site E2 | | Out of zone. | OUT | - |
| 1407 Data Recovery (L38) | Beta Site E2 | | Out of zone. | OUT | - |
| 1409 Black Hole (L44) | Yotunheim | | Out of zone. | OUT | - |
| 1410 Trust (L48) | 2: talk Moh'katan 4220, deactivate monitoring devices 4221 (**one** objective 4875 "remove the first") | R: reuse 1362's anchors | Truncated in the original data (defect H-D2); implement the one objective as authored. | NEW | H37 |
| 1580 Moles, Part 2 (L1, chain-gated) | 4: approach Grogan in Storage 4700, search containers 4701, eliminate Grogan and NID operative 4702 (2 objectives), report Copplemann 4703 | T: Grogan, NID Operative, container props (new) in 70 instance; E1, E4 | Containers use the Cellblock 1032 interact-grant-destroy pattern (D-H13). | NEW | H34 |

### Goa'uld

| Mission | Steps | Needs | Notes | Status | Packet |
|---|---|---|---|---|---|
| 1200 Meet Your Queen (L1) | 2: convince Royal Guard 3584, speak to Anat 3585 (+optional "Ask Ba'al" 5399, the only hidden objective in the 36) | T: Royal Guard (new) in 68; Anat 43 exists | Convince is a dialog choice. | NEW | H40 |
| 742 Giving the Walls Ears (L1) | 5: disguise from Petbe 2502, put on 2503, hide 3 devices 2504 (objectives 2913-2915), report Anat 2505, map to Nerus 2506 | T: SecondBug/ThirdBug baskets (template 164, new spawns), Nerus 53 spawned; I: 2819, 2820, 2864; E6 (D-H05) | The one RESTORE. Anat already carries the only `entity_interactions` row in the game (template 43, dsm 3127, `missions_not_accepted {742}`) and dialog 2636 the only `accepts_mission_id`. Dialog-set maps 3129/3130/3131 exist. Keep the Python's un-consumed devices (H-B15) or consume them: record the choice. | RESTORE | H41 |
| 741 Plant Spy (L6) | 7: symbiote from tank 2491, speak Ba'al 2492, put Dawson at ease 2493 (+use symbiote 2902), interrogate witnesses 2494, speak Dawson 2495, collect footage 2496, report Anat 3583 | T: Dawson, witnesses (new) in 70 instance; tank prop; I: 2818 (ability 2068 "Infect Dawson"), footage item unrecovered; E7 | Footage collection is a Cellblock-1032 search. | NEW | H42 |
| 1243 Surveillance (L10) | 3: plant 3 Scarabs in OpCORE Zone 3610 (objectives 4182-4184), 3 in Jaffa Zone 3611 (4185-4187), report Anat 3612 | R: six anchors (Operations Center yard, Overflow yard, Blackstock's office, Petbe's quarters exterior, Fountain, Bookseller's stalls); I: 2820; dialog 4080 | Six `interact_tag` anchors; two steps of three. | NEW | H42 |
| 1240 Infiltrators (L1) | 5: examine 3 Shield Towers 3606, return Ba'al 3607, use Ba'al's device 3608, check signal source 4688, report 4689 | R or T: three tower props; T: Ashrak assassin (new) at the signal source; I: tracker item unrecovered; dialog 4294; E1 | | NEW | H43 |
| 1241 Invasion Plans (L1) | 6: scan CmdCenter Lab 3609, Market 3613, Jaffa Zone 3614, Storage 3615, kill infiltrators and dismantle beacon 3616 (2 objectives), report Ba'al 4833 | R: four scan regions; T: infiltrator templates and beacon prop in 69 instance; I: 5146 beacon core; E1, E4, E9; dialog 2487 | | NEW | H43 |
| 1244 Find the Mole (L1) | 4: speak Hansen 3617, info from Lo'rak 3618, search Petbe's quarters 3619 (objective text literally "Placeholder minigame."), paperwork to Ba'al 3620 | T: Hansen, Lo'rak (new), quarters prop; dialogs 4299/4300/4303 | Search is the 1032 pattern (D-H13); the 2009 designers never finished this step. | NEW | H44 |
| 1245 Extreme Prejudice (L42) | 3: coerce Lo'rak 3622, ambush and kill Petbe in Storage 3623, report Ba'al 3624 | T: Petbe clone (template 163) spawned hostile in 70 instance; I: 2823/2825 robes; E1, E4; dialog 4305 | Shared-hub Petbe (spawn 223) untouched (D-H03). | NEW | H45 |
| 1246 Petbe's Murderer (L44) | 5: Anat's demands to Ba'al 3625, what Lethander knows 3626, recover 3 Straegis Themes 3627 (3 tasks), deliver 3706, report 3869 | T: Lethander 46; I: 5703/5704/5706 themes (source unrecovered: scripted grant); dialog 4309 | Works even while another player's Petbe is alive, because nothing in it touches the shared entity. | NEW | H46 |
| 1247 Patsy (L46) | 5: see Nerus 3628, purchase treat 3629, deliver 3630, go to Marketplace 3707 (2 tasks), deliver Lethander's head to Anat 3934 | T: Nerus 53, Lethander clone in 69 instance; I: 4512 or 2714 treat, 5736 head; GH2 (purchase) or grant; E1, E4 | | NEW | H47 |
| 1322 Vendetta (L34) | 3: arrest Suspicious Jaffa in Jaffa Zone 3939, kill Bra'hin 3940, report 3944 | R: Jaffa Zone; T: Suspicious Jaffa, Bra'hin (new); E1 | Shares Bra'hin with 1371; per-player kill via `entity_dead_tag` on a mission-scoped spawn. | NEW | H47 |

### Older Revisions (spec sheet Legacy_Unresolved)

57 ids requested; 54 exist (579, 756, 763 are absent). All `General` label (751 is `Lucia`), no script, level 1 (590 is L9). 20 have zero steps. Exact-name collisions with the current 36: none. Step-text overlap: only generic "Report to Marsh/Copplemann" lines (max Jaccard 0.25). These are independent stubs, not earlier revisions of the current chain. **OUT**, no packet.

## Spec Sheets With Server Work Outside Missions

| Sheet | Repository state | Work |
|---|---|---|
| World_Config | Rows exact. Instancing per `spaces.xml`, not `flags` (68 shared, 69/70 per-player). Market AABB degenerate (H-B11). | Decision D-H04; note in H10 |
| Regions_Transport | All 8 Harset point sets and 5 ring rows exact. Ring FSM data-driven and complete; **zero** `trigger_transporter` chains for Harset (the Python bound the switches in the space script). Stargate 3 exact; H-B1, H-B2. | H01, H02, H10 |
| Command_Center | Entry and return coordinates recovered from Python. Anat spawned; Ba'al, Moh'katan, Marsh, Copplemann, Nerus, Opheltes have templates or need them but no spawn. No navmesh. Sarcophagus, symbiote tank, lab consoles: asset strings only, no actor. | H11, M0; tank prop is an authoring choice in H28/H42 |
| Spawns_Entities | Exact (23 rows). Debug spawns 1 and 42 excluded. Nothing respawns (H-B7). | H12, H13 |
| NPCs_Actors | 6 of 30 have templates; 2 of 30 are spawned in Harset. Coordinates for 24 unrecovered. | H11-H14, M0 |
| Props_Interactables | Only the DHD, ring switches, one bug basket and the two transition boxes exist. Shield towers, sarcophagus, tank, quarters, containers, beacon, holding pens: no template, no spawn, no region. | H14, H15 |
| Vendors_Shops, Vendor_Tier_v2, Class_Skills_v2 | Vendor and trainer Rust complete (7,267 lines, live-DB tested); seed has two test lists and one debug vendor. All ~60 Harset declarations are `texts.sql` monikers (several with empty `text`), no templates. `fix/vendor-index-gm-gate` unmerged (U8). Class_Skills_v2 is a reconstruction proposal keyed on real ability ids; `archetype_ability_trees` has only Soldier and Commando populated. | GH2 |
| Items_Loot, Harset_Items_v2 | All 42 ids exist, all `container_sets = {2}` (mission bag). Legacy `items_event_sets` bindings are reference data, not live wiring; the live path is an `item_use` chain (zero exist for Harset). Duplicates resolved per D-H14. 2819 Jaffa Disguise has no appearance ability anywhere: a "wear disguise" beat is an `item_use` chain that advances the step and nothing visual, unless a disguise effect is authored (record as design). | per mission packet; canonical ids in [work-packets.md](work-packets.md#worker-input-and-ownership) |
| Dialogs | 95 Harset screens across sets 525, 560, 656, 661, 947 plus set 689 (742); speakers for Ba'al 942, Nerus 943, Anat 944, Moh'katan 945, Marsh 941, Copplemann 968, Lethander 978/496, Mala'c 957. **One** `entity_interactions` row and **one** `accepts_mission_id` exist in the whole game (both 742). `dialog_set_maps.missions_completed/not_accepted` are `{}` on all 4,671 rows. Every mission accept and every NPC topic is authored as chains. | every mission packet has a dialog-evidence step |
| Kismet_Cameras | Sequences 113-126 (gate) and 2045-2054 (rings) registered; nothing mission-specific. | none; `play_sequence` only where an id exists |
| Multiplayer_State | See D-H03, D-H04. No party system. Dialog-set bindings are the one per-player mechanism and are in-memory (restore on `player_loaded`). | H03, every instance-bound packet |
| Anim_VFX_Audio | Client-side; `d_Harset` has 152 SoundBank entries. Nothing for the server. | none |
| Level_Files, Source_Index, Asset_Restore_v2 | Provenance. The cooked maps are the only source for prop actor ids if GH1 or H15 need them. | GH1 evidence |
| UI_Behavior | Dialog, DHD, ring dialog, vendor, minigame surfaces all exist server-side except the DHD interaction (H-B2). | H01 |
| Offworld_Handoffs_v2 | Agnos, Beta Site E2, Yotunheim zones do not exist. | OUT; 1353 step 4010 stubbed |
| Build_Order_v2 | The spec's order (travel, population, faction entry, CmdCenter, missions, Market/Storage, vendors, items, offworld, combat, assets, validation) matches this ledger's lanes; the spec's step 5 "enable disabled steps" is deleted. | ledger |

## Items

Canonical ids after D-H14 and the legacy binding evidence (`items_event_sets.sql`, which Cimmeria does not read but which records 2009 intent):

| Group | Canonical | Why | Legacy binding |
|---|---|---|---|
| Scarab Listening Device | **2820** | `GivingTheWallsEars.script` node 52 grants 2820 x3; 6818 appears nowhere | none |
| Straegis Scanner | **4396** | only id bound to ability 2092 "Use Straegis Scanner" (effect 2816); 2828/4651/4709 bind the generic placeholder 597 "Heal Focus" | scan pulse |
| Tollan Control Technology | **2743** | bound to ability 2410, effect 3472 "Use on Dawson: detects Goa'uld, prompts hostility switch"; 2716 binds 597 | Dawson only |
| Goa'uld Symbiote | **2818** | bound to ability 2068, effect 2745 "Infect Dawson"; 2718/4652 unbound | Dawson only |
| Petbe's Bloody Robes | 2825 (arbitrary) | rows byte-identical, no reference anywhere | none |
| Nanite Tracking Gun | 4690 | ability 2409 "Use Tagging Gun: places a micro tracker on target" | tag NPC |
| Jaffa Disguise | 2819 | binds only placeholder 597; no appearance ability exists | none |

Frost's Letter 3730 is granted by Cellblock chain 1003 (`castle_cellblock_chains.sql:107`); Romney's Files 2698 is granted nowhere. Mission-bag items survive a gate hop (no inventory touch in `base/world_entry/gate_travel/`).

## Acceptance Test Mapping (spec Validation_Tests sheet)

| Spec test | Automated guard planned | Gap today |
|---|---|---|
| H-01 / J-01 / G-01 arrival | H01 arrival `is_point_valid` test; live-DB persistence of 1360 across the hop (Cellblock C04 test) | H-B1, H-B2 |
| W-01 / W-02 Command Center | H10 replay: `enter_region` with world condition resolves one `cross_world_teleport` each way; pinned coordinates | coordinates unpinned |
| S-01 / S-02 gate identity, DHD | seed assertion; DHD UI needs H01's interaction handler | H-B2 |
| R-01 / R-02 rings | H10 replay: five `interact_tag` chains each resolve `trigger_transporter` with the right region; H02 timeout tests | no chains; H-B3 |
| M-01 to M-18 | one chain-replay file per mission (`mission_<id>.rs`), positive and adjacent-negative per chain, relog restore | everything |
| N-01 static population | H12/H13 live-DB: exactly the seeded rows spawn, debug 1/42 excluded, respawn stamps | H-B7 |
| V-01 vendors fail closed | GH2 | GH2 |
| L-01 no random loot | H13 asserts `loot_table_id` NULL on every Harset template | free |
| A-01 / A-02 audio | client-side | none |
| P-01 relog | every packet's `player_loaded` restore replay | standing rule |
| P-02 two players at different Petbe stages | H45/H46 executor test: mission spawn lives in the instance; shared spawn 223 unchanged | H03 |

## Audit Validation Record

| Check | Outcome |
|---|---|
| All 31 sheets exported and read | Done (openpyxl, `data_only`) |
| Deprecated tree swept for 36 mission ids and 15 NPC names | Zero hits outside the three Harset files |
| Seed read: missions (139 steps, 176 objectives, 194 tasks), spawnlist (23 Harset rows), entity_templates (153 rows, field census), point sets (8), ring regions (5), stargates (1), respawners (0 Harset), dialogs (95 screens, 22 dsm rows), items (42), item_lists (2), cover sets (no world column) | Done, cited |
| Content engine inventory against `main` and three branches | Done; PR #618 merged as `f23e73fb` during the session and is reflected above; #619 carries the remaining effect arms |
| `harset.nav` and `harset_storagerm.nav` decoded (header, flood fill, 12 coordinates) | Done; direct-index adjacency 100% reciprocal |
| Build, test, live DB, client | Not run |
