# Debug Area and Starter Kit

> Type: how-to and ledger. Audience: the coordinator, packet workers and the
> playtesters. Opened 2026-10-04 against `main` @ `bd020cb95`. Prefix `DA-` for the
> Debug Area, `SA-` for the starter kit. Same dispatch rules as the
> [ability mechanics ledger](../ability-mechanics/work-packets.md#dispatch-rules).
>
> **Campaign status (2026-10-05): released (v2026-10-05.1 + v2026-10-05.2),
> live-verified, follow-ups listed.** Every packet and every DA-F fix is merged
> and released, and the status docs are closed out (see [Release](#release)).
> The DA-06 lab run on v2026-10-05.1 found nine faults (DA-F1 to DA-F9); all
> nine shipped in v2026-10-05.2, and the re-check on v2026-10-05.2 confirmed
> each fix the lab can see ([DA-06 results](#da-06-results)). Client patches
> 007, 009, 011 and 012 are in the signed content manifest; 010 was pulled
> ([incident](#patch-010-incident)). What is left is in
> [Open follow-ups](#open-follow-ups), plus the tester UAT.

## Why

Maintainer feedback from playtesting on the colo (2026-10-04), which takes
priority over the rest of the ability-mechanics work:

1. "Starting classes should all spawn with at least pistol shot, focus regen
   and health regen abilities."
2. "We'll soon need some kind of test map which has all enemies, a grant all
   abilities NPC for my class etc. Basically like the debug rooms from Oblivion
   and Skyrim." The debug area "must be able to UAT all feature and system
   functionality we have implemented as best we can".
3. Perform a release once both are ready. A map may be patched or created, but
   no CME bytes in the repo: only diffs.

The owner added: the debug area should be "a bit more spread out and has cover
and rooms and stuff we can use to test all functionality like respawn and
friendly, neutral, enemy npcs, hostile npc battles", and the seeded playtest
characters should be Praxis Commandos identical to characters made through
normal character creation ("today they are half formed").

The existing [stasis-room debug hub](../../content/debug-hub.md) stays. It is
full, every player sees it, and it has no enemies, cover or room to fight in.

## What the starter-kit gap actually is

- `char_creation_abilities.sql` already grants every char_def 1-23 the abilities
  592 Pistol Shot, 594 Strike, 597 Heal Focus, 1646 Health Heal and 1218
  Recuperation (audit row B-01).
- The colo's seeded playtest characters (`db/sgw/Players/Seed/sgw_player.sql`:
  cady, jorsh, cake, lomiada1 and others) carry only `{592, 594, 597}`, spawn in
  SGC_W1 and lack most of what character creation writes. Confirmed on the colo
  2026-10-04: player 71, created in-game, has all five; players 62-70 have three.
- No character starts with a weapon. 592 has `required_ammo = 1`, so Pistol Shot
  is refused with NoAmmo at spawn.
- The action bar is client Lua state (`GActionProfiles`). The server cannot place
  abilities on it, so a fresh character's bar is empty and the granted abilities
  only appear in the Abilities window.

## Decisions taken for this campaign

The owner can reverse any of these. Each is recorded so a reviewer can find it.

| Id | Decision | Why |
|---|---|---|
| D-DA1 | The Debug Area is a new server world **1300 `DebugArea`** on the shipped client map **Ihpet_Crater_Light**. | Only candidate with rooms, dense map-authored cover (6,324 `SGWSpecCoverNode`s), a flat 60 m sunken pit for an arena, a 420 m terrace for a gallery and a crater wall as its boundary. Survey: [npc-ai-spawn-advisor memory](../../../.claude/agent-memory/npc-ai-spawn-advisor/debug-area-map-survey.md). The original test maps (Combat_Terrain_Test, Mob_TestMap, InteriorCombatTest, MissionTest*) were never recovered in any archived build. |
| D-DA2 | New id, served through cooked-data category 12 (`CookedWorldInfo`), not a remap of a shipped test world id (54, 53, 21, 1). | The category-12 route is proven in-client by the historical CellBlocks (1201-1207). Remapping a shipped `_N` entry breaks the "every shipped world served untouched" rule. |
| D-DA3 | Shared and always loaded (`Instanced="false"`, listed in `cell_spaces.xml`). | Two testers can meet, NPC battles and respawn timers keep running. |
| D-DA4 | GM-only travel *in*: `.gotolocation DebugArea` (and the native `/gmgotolocation`). **Amended 2026-10-04 (DA-07):** world 1300 does get a stargate row, gate 29 on the map's own gate prop, but it is outbound only (`stargates.debug_dial_hub`): nobody can dial it or learn its address, and a GM at its DHD can dial every gate on a world the server loads. `.gotolocation DebugArea` with no coordinates now lands on the gate's arrival pin, which is the Z1 point (respawner 130). DA-02 (#1230) enforces GM-only entry on the server: a non-GM is sent to their faction start at login and on every cross-world transfer, so the hub's non-GM branch is reachable only by a GM demoted while inside. | Keeps ordinary players out without a client patch; the seeded playtest accounts are GMs. The amendment is the owner's request: "I want to be able to outbound dial any gate in the game" from the Debug Area gate. A teleporter NPC in the stasis hub is left to the owner. Details: [gate-travel.md](../../gameplay/gate-travel.md#debug-area-dial-out). |
| D-DA5 | The world reads the client map's navmesh and occluder files (`ihpet_crater_light.nav/.occ`) when it has none of its own. | Avoids a 5 MB copy per added world. |
| D-DA6 | Tags `DebugArea_*`, never `DebugHub_*`. | The hub's tests count the `DebugHub_` prefix. |
| D-DA7 | Training dummies never retaliate. | A NEUTRAL faction-10 NPC still fires back with 592 when shot, which spoils damage and heal measurements. Reuse the `.dummy` "never attacks" mechanism as a seedable flag. |
| D-DA8 | NPC-vs-NPC arena runs two fights: a faction 3 vs 10 squad fight a player can join, and a spectator-only pair from the player-safe hostile pairs (2-19, 2-27, 2-29, 23-29, 27-29, 27-36). | 3 vs 10 is the only seeded mutual-hostile pair; the spectator pair shows NPC-only kills paying nothing (#1009). |
| D-DA9 | Gallery hostiles are seeded passive (`aggression_override`), so a tester walks the line and picks a fight. | 101 hostile templates in one line would otherwise chain-pull. They stay damageable. |
| D-SA1 | Every character starts with a basic pistol and ammo in the active bandolier slot. **Superseded for normal progression by OD-CS01 (2026-10-05, [Class Start v6](../class-start-v6/README.md#owner-decisions)):** only debug-kit profiles and the `NON_CANONICAL_BLOCKED_LEGACY` holding states keep it. | Makes Pistol Shot usable at spawn, as asked. The Castle CellBlock tutorial still hands out its pistol; SA-01 records any deviation. |
| D-SA2 | A fresh character's action bar is filled once with its known abilities by client patch `009-starter-hotbar`, a delta on a UI file the testers' WQHD UI pack (v26) does not replace. | The server cannot write the bar. The pack replaces `ActionProfiles.lua`, so a delta on that file would not apply for them. |

## Zone layout (world 1300)

Heights are BigWorld metres. On open terrain the navmesh and the occluder terrain
disagree by up to 3.3 m, so terrain points use occluder heights; interiors, the
terrace and the pit agree within about 1 m. Every hostile pair of zones is at
least 86 m apart with a wall between, against an 18 m default aggro radius and
10 m assist radius.

![Proposed zones on the Ihpet_Crater_Light crater floor](layout-ihpet-crater-light.png)

| Zone | Point (x, y, z) | Contents | Packet |
|---|---|---|---|
| Z1 arrival + respawner A | (251.0, 8.0, -962.0) | Arrival, respawner 130 | DA-01 |
| Z2 services plaza | (252.0, 7.0, -923.0), clear radius 12 | Copies of every hub service, ability granter | DA-02 |
| Z3 dummies range | line x 234..258 at z -872, y 6.6 | Non-retaliating dummies, friendly heal target (west end since DA-F7) | DA-02 |
| Z4 faction yard | (416.0, -5.7, -786.0), radius 25 | Friendly row x≈400, neutral row x≈416, hostile pen x≈434 | DA-03 |
| Z5 AI behaviour slope | A (20.0, 13.9, -760.0) ↔ B (22.0, 20.6, -650.0); wanderer (74.0, -0.4, -746.0) | Patrol, wander, leash, assist pair | DA-03 |
| Z6 NPC-vs-NPC arena | east shelf (354.0, -11.12, -730.0), terrain terrace x 318..395, z -680..-749 (DA-F2; was the pit, a water plane) | Fight 1 lines at x 354 (NID) / 378 (Praxis), fight 2 at x 343 / 364 | DA-04, DA-F2 |
| Z7 enemy gallery | terrace y 23.1, rows z -592 / -612, x 55..170 and 305..465 | Every hostile template, passive | DA-03 |
| Z8 cover course | south compound west wing: entry (204, 7.0, -926), riflemen (166, 6.7, -930), (142, 6.7, -958) | `use_cover` riflemen among 161 cover nodes | DA-04 |
| Z9 death and respawn test + respawner B | (438.0, 10.4, -916.0) | Respawner 131, a lethal hostile | DA-04 |
| Z10 Visual NPC Lineup | south compound east wing, x 302-382, z -888..-961, y 6.58; doorway (300, 6.8, -897); ring pad (287.0, 6.80, -914.0) planned | 161 passive, friendly display actors: one per character look (155) and per template-less body set (6), in 15 rows | DA-10 |

## Id blocks

| Range | Owner |
|---|---|
| World 1300, respawners 130-131 | DA-01 |
| Templates 1300-1329, spawns 13000-13199 | DA-02 |
| Templates 1330-1369, spawns 13200-13599, point sets 13200-13209, points 13200-13299 | DA-03 |
| Templates 1370-1399, spawns 13600-13799 | DA-04 |
| Spawns 13810-13817, ring regions 35-42, event sets / point sets / points / chains 13810-13817, sequences 10189-10204 (13800 is DA-07's) | DA-08 |
| Templates 1400-1409, spawns 13850-13869, ambient chatter group 1 | DA-09 |
| Templates 1410-1599, spawns 13870-14099 | DA-10 |

Check each range is free in the seed before using it; raise a clash with the
coordinator instead of moving into another packet's block.

## Packets

| Packet | Scope | Depends on | Status |
|---|---|---|---|
| SA-01 | Seeded playtest characters rebuilt as Praxis Commandos identical to normal creation; starter pistol and ammo for every char_def (D-SA1); live-DB guards that every char_def and seeded character holds 592, 597, 1646, 1218 and the pistol. | none | Integrated (#1218) |
| SA-02 | Client patch `009-starter-hotbar` (D-SA2), Lua logic UAT against stock and v26 `ActionProfiles.lua`. Published in the signed content manifest (content-current, 2026-10-05). | none | Integrated (#1213) |
| DA-00 | This plan. | none | Integrated (#1210) |
| DA-01 | World plumbing: world row, `spaces.xml`/`cell_spaces.xml`, a table of Cimmeria-added worlds feeding `world_id_for_name`, `client_map_for_world` and `WORLD_INFO_OVERRIDES` (also closing the missing shipped names: 50, 61, 62, 69, 70, 72, 73, 78), fail-closed `resolve_space_id_fallback`, nav/occ fallback to the client map (D-DA5), advisory list, respawners 130/131, generated cover nodes for world 1300, `.gotolocation DebugArea`. | none | Integrated (#1223) |
| DA-02 | Z2 services plaza (vendor, trainer, dialog NPC, terminal, loot crate, registrars, pet trainer, bankers, mail clerk, Black Market auctioneer, crafting stations) and an ability granter that gives the clicking player every ability of their archetype; Z3 non-retaliating dummies (D-DA7). | DA-01 | Integrated (#1230): 21 plaza NPCs (spawns 13001-13022), an ability granter and reset NPC (templates 1300/1301, content action `gm_ability_bulk`, chains 13000-13008), a munitions vendor (1302, buy list 1300), and five training dummies (templates 1310-1314, spawns 13100-13104) flagged by the new `entity_templates.training_dummy` column, which reuses `.dummy`'s mark. Reference: [debug-area.md](../../content/debug-area.md). |
| DA-03 | Z4 faction yard, Z5 patrol/wander/leash/assist, Z7 enemy gallery with every hostile template (D-DA9). Seeded: templates 1330-1333 and 1340-1343, spawns 13200-13245 and 13300-13398, patrol point set 13200 (points 13200-13201). Gallery: 99 of 101 hostile templates placed (140/141, children, excluded), 24 with an ability set, 75 on the 592 fallback, 62 with no display name. Yard rows moved to z -806..-794 where nav and terrain agree. See [debug-area.md](../../content/debug-area.md). | DA-01 | Integrated (#1222; seed-wide guard fix #1235) |
| DA-04 | Z6 arena (D-DA8), Z8 cover course, Z9 death and respawn test. Templates 1370-1377, spawns 13600-13641 in `*_debug_area_combat.sql`; reference [debug-area.md](../../content/debug-area.md). | DA-01 | Integrated (#1224) |
| DA-07 | The Debug Area stargate: gate 29 (outbound-only dial hub) on the Ihpet_Crater_Light gate prop, its gate volume, DHD and cooked entry; a GM at its DHD can dial every gate on a loadable world; nobody can dial in (D-DA4 as amended); template 1 DHDs made clickable. Ids: spawn, point set and point 13800, stargate 29. | DA-01 | Integrated (#1232) |
| DA-05 | `docs/content/debug-area.md`, `docs/guides/uat-specs/debug-area.toml`, unified UAT section mapping every system to a station. | DA-02..04 | Integrated (#1240): 46 spec rows, 43 ready in the lab and 3 blocked (non-GM account, store buttons, seeded account); [unified UAT, Debug Area](../../guides/unified-uat.md#debug-area) DA-U1 to DA-U46; station index in [debug-area.md](../../content/debug-area.md#which-station-tests-what); the [DA-06 checklist](#da-06-live-client-checks) below |
| DA-06 | Live-client check in the lab: the map loads as world 1300, spawn heights, doorway collision, map Kismet, every station answers. DA-08 rings: all eight rigs render at floor height and the base is not black or flat (unlit clone outdoors); `onSequence` 10189-10204 animates the right rig and the ring sound plays (`prp_gen/rings/transport`, waveform in `genprp/prp_gen.fsb`, which 006 does not copy); two clients: a witness by the source pad sees Teleport Out, and a destination witness sees Teleport In (read `client.sequence.dropped` for 10190/10192/.../10204 on the second client; a drop is the documented cosmetic limitation). Fixes as `DA-F<n>`. Checklist: [DA-06 live-client checks](#da-06-live-client-checks). | DA-01..04 deployed | Done 2026-10-05: run on v2026-10-05.1, re-check on v2026-10-05.2. 18 of 34 checks pass; the rest are partial, blocked or not run for lab reasons, and none fails. [Results](#da-06-results), [open follow-ups](#open-follow-ups) |
| DA-08 | Ring transports: client patch `010-debug-area-rings` clones eight working ring rigs (region 3's Castle rig with its Kismet) into `Ihpet_Crater_Light-fff80002`, and the world-1300 seed wires a fully connected ring network across them (Compound, Faction yard, AI slope, Pit overlook, Arena shelf, Gallery west, Gallery east, Death yard). Ring `onSequence` now reaches witnesses too. Station table in [debug-area.md § Ring transports](../../content/debug-area.md#ring-transports). 010 was published after 007 (content-current, 2026-10-05) and pulled the same night: it hung the client (DA-F1). | DA-01 | Integrated (#1234); patch superseded by 011 (DA-F1) |
| DA-09 | The System Lords' summit: six System Lords (Ra, Ba'al, Anat, Athena, Morrigan, Nerus) and Ra's Jaffa in a circle on the compound courtyard, each dressed from the client's own costume packages, squabbling in say chat on a schedule. Adds ambient chatter: the `ambient_chatter_groups` / `_lines` seed tables and the `cimmeria-cell-chatter` plugin ([ambient-chatter.md](../../content/ambient-chatter.md)). Station: [debug-area.md § System Lords' summit](../../content/debug-area.md#system-lords-summit). | DA-01 | Lab-checked on a local server 2026-10-05 ([DA-09 lab check](#da-09-system-lords-summit)); released with the next server release |
| DA-10 | Z10, the Visual NPC Lineup, asked for 2026-10-05 ("one of every NPC as a friendly unit, all models and all combos"; owner-confirmed requirements the same day). 161 passive, friendly display actors (templates 1410-1570, spawns 13870-14030): one per distinct character look in `entity_templates` (body set, components as a sorted set, colours, skin tint, static mesh with NULL = '': 155 looks from 225 templates, NPC Child 1 and 2 included) and one per character body set no template uses (6). **Coverage delta 162 → 161** (owner-approved): compared column for column there are 156 looks, but Nerus (53, static mesh NULL) and Sandbox Greeting NPC (166, '') differ only in NULL vs '', which the client draws alike; the actor is named after 53 and its table row lists 53, 166, 305 and 1405 ([note](../../content/debug-area.md#coverage-delta-162--161)). The 161 entries cover every visual appearance, not every template; the doc maps each actor to every template sharing its look. Display copies only: faction 1, no event set (570, the plain default sequence set, goes back if the client shows actors that do not animate), abilities, loot, dialog, interactions, patrol or content hooks (live-DB guard); the Idle aggro scan and every hit gate refuse them (cell guard). Nameplates read `<name> #<source id> <body set>` through a new opt-in `entity_templates.display_name` column sent as `onBeingNameUpdate(WSTRING)` after the `name_id` text on every AoI introduction, late joiners included (wire and AoI tests). Tags `DebugArea_VisualLineup_<source id>`. Placed in 15 rows in the south compound's east wing; ring pad (287.0, 6.80, -914.0) and console (290.0, 6.58, -915.3) picked for rz10's ring packet. Props go to a planned Z11 Props / Interactables Gallery. `BS_RaJaff` (mesh `Ra_500` has no export) and `HM_BodySet` (no reference mesh) are placed but expected not to render; `live_db_eye_heights` lists both as unmeasurable. Station: [debug-area.md, Visual NPC Lineup](../../content/debug-area.md#visual-npc-lineup); UAT DA-U48. | DA-01 | Seeded and tested on the server's data. Lab 2026-10-05 (local branch server, arrival at Z1): with the lineup 415 reliable packets, 43 FPS, 11.8 s load, and a permanent `mercury.tx_hole` stall on Petbe #221's 1504-byte cascade (74 NPCs never delivered); without it 93 packets, 60 FPS, 4.0 s. Blocked on the Mercury oversize fix; nameplates, (b) rerun and the template-less look still to do |
| DA-F1 | Ring patch crash. DA-06's lab run found 010 froze the client on every load of the Ihpet Crater map (`ACCESS_VIOLATION` at `SGW.exe+0xbc6a0`). Root cause: the cloned `LightingChannels` struct names a property `Dynamic`, an editor-only name entry in Ihpet's table, which the client reads as `None`; the property list ends early and the next tag is read 3 bytes off. The cloner now adds a client-loadable entry for a name the target holds with narrower load bits. Patch `011-debug-area-rings-fix` replaces 010 (clean installs from stock + 007, installs that applied 010 from 010's own output, 402-byte delta) and moves region 39 off the pit's water plane to the east shelf (331, -11.12, -693). Details in [client-patches README](../../../data/client-patches/README.md#011-debug-area-rings-fix). | DA-08, DA-F2 | Released (#1247, v2026-10-05.2); 011 confirmed in the lab: world 1300 and world 73 load, rings rise |
| DA-F2 | DA-06 check 16 and the colo's first 30 minutes on v2026-10-05.1. (1) **Arena off the water.** The pit floor is a water collision plane that players sink through (to y -52) while the squads stand on it. Both fights moved to the east shelf, flat terrain at y -11.12. Fight 1 stands in the shelf's one strip where a 24 u 3-against-3 line has clear sight (NID x 354, Praxis x 378, z -738/-741/-744; the guards stay 40 u from the faction-yard ring pad). Review round 1 (#1244) closed the rows to 3 u apart after a navmesh pull map found the third NID guard pulling the fight-2 room's south-west corner past the long wall; `arena_pull_map.rs` now guards it. Fight 2 is at x 343 / 364, z -696..-700, 39 u or more from the guards. The Praxis `aggro_radius` drops from 30 to 28, so the Idle scan's 2x reach stays off the yard's damageable faction-10 rows. Region 39's pad is left to daf1 (coordinates sent). (2) **No-witness WARNs need a player present.** `wire_npc_no_witnesses` and `abilities.sequence outcome=no_witnesses` are written only when `SpaceManager::player_present`: a player has the NPC (the shooter) within their AoI radius (the stale-witness-list fault the WARN exists for; the target's range does not count, see the follow-up below), or the NPC, the target or a threat-list entry is a player, pet, deployable or lab dummy; NPC-vs-NPC with no player writes nothing (owner decision 2026-10-05). (3) **Leash loop false positive.** Every arena `event=loop` WARN was `trigger=target_dead` at `npc_to_spawn` 0: a won fight's reset, not a loop. `target_dead` and `target_gone` leashes no longer count toward it; a table test pins every trigger, and a pass that drops a live target lost beyond AoI and then a corpse keeps the `target_out_of_aoi` label. Tests: messaging, sequence and detector unit guards, `arena.rs` terrain, reachability and spectator guards, live-DB arena cycle. | DA-06 | Released (#1244, v2026-10-05.2); the re-check saw both fights run on the shelf with no WARN rows (check 16). Follow-up: the colo still wrote `no_witnesses` every ~78 s for a Soldier shooting a NID Guard while a lone player idled 145 m from the guards, past 150 m from the Soldier. The in-range test counted the target's AoI; it now tests the shooter only |
| DA-F3 | The Debug Area DHD listed the 11 hub-granted gates as "Unknown" on the first open (DA-06 check 22). The client resolves a new `updateStargateAddress` id from its cooked cache asynchronously (`GateTravel` handler `FUN_00e2eff0`) and never redraws an open DHD. The grant now also runs at world entry (`InitPlayerState`, after the book is stamped and after `setupStargateInfo` reached the client), so the DHD-open pass finds nothing new; the world-entry grants ride one `EntityMethodCallBatch` (PR #410 precedent). Tests: `debug_area_hub_grant.rs` (cell, through the real `InitPlayerState` dispatch, including the DB book stamp ordering), `the_world_entry_pass_grants_on_the_hub_world_and_leaves_the_dhd_nothing_to_send` (cell-interactions). | DA-07 | Released (#1252, v2026-10-05.2); re-check: all 13 gates named on the first open |
| DA-F4 | `.gotolocation <world> <x y z>` moved the GM's server-side selection (legacy `target or player`); the client sends nothing on Escape, so the server keeps a target the tester cleared. `.gotolocation` now always moves the caller (the `.summon` intent rule); `.gotoxyz` / `.goto` still move a selection on purpose. Test: `gotolocation_with_an_npc_selected_moves_the_caller_not_the_npc` (cell-console). | DA-01 | Released (#1252, v2026-10-05.2); re-check: with Harriman selected, `.gotolocation` moved the caller and Harriman stayed put |
| DA-F5 | An out-of-range `interact` gave no feedback. A too-far click on an NPC the player can see now prints "You are too far away from <name>. Move closer to interact." (Banker and registrar keep their own lines) and logs `event = "interaction.out_of_range"`. Review fixes: only for a target in the player's witness set or AoI radius (a far id stays a silent drop, so the line is no NPC-name oracle), and one line per player per 1.5 s. Tests: `out_of_range_interact_on_a_plain_npc_tells_the_player_to_move_closer`, `out_of_range_interact_on_an_npc_out_of_view_sends_nothing`, `repeated_out_of_range_clicks_send_one_line_per_interval` (cell-methods). | DA-02 | Released (#1252, v2026-10-05.2); re-check: the line shows, at most one per interval. Open: it names the server template, not the shown name ([follow-up (a)](#open-follow-ups)); the out-of-view case cannot be sent from a stock client ((d)) |
| DA-F6 | Health Heal 1646 and Recuperation 1218 showed `IconMissing`, and error 42 printed `CONDITION_FEEDBACK_OutsideWeaponRange`. Cooked attribute patches (`crates/resources/src/base/attribute_patches/`, version bump a stable 32-bit FNV-1a) serve the client's Medkit icon (its health-restore item art; the imagesets hold no health-heal ability icon) and "Your target is out of range" (worded like the shipped `_39`); the seed carries the same values. Tests: `attribute_patches/tests.rs` (resources). | SA-01 | Released (#1252, v2026-10-05.2); re-check: Medkit icons on a-4/a-5, "Your target is out of range" at 9.5 m |
| DA-F7 | The friendly dummy (spawn 13104) stood behind a wall at the line's east end, out of Health Heal's 5 m reach. Moved to (234, 6.82, -872), the open west bay beside L1. Test: `the_friendly_dummy_is_reachable_and_visible_from_the_dummy_line` (cell-world); DA-02's placement guard and DA-U20 moved with it. | DA-02 | Released (#1252, v2026-10-05.2); re-check: healed from 4.5 m, no wall in the way |
| DA-F8 | Native `/gm*` commands read "Invalid command." for a GM (DA-06 report observation O1; the report's own DA-F8, the dropped same-port re-login, is DA-F9). Not a server bug: the launcher install has no `Common/xml/slash_commands/InternalSlashCommands.xml`, the QA client file that defines all 167 `/gm*` commands, so the client's command map never holds them (104 entries instead of 266). Lab check (da06): with the QA file copied in, the map held 266, `/gmdhd`'s role mask read `0x2FF` against the client mask 2, and `/gmdhd 29` / `/gmdhd 3` reached the server. The character's `access_level` (2, written at creation) and the class flip were already right. Fix: client patch `012-gm-slash-commands` ships a project-written file (162 `/gm*` commands, `Access="p"`, no CME text; generated by `tools/client-patches/gm_slash_commands.py`), see [client patches](../../../data/client-patches/README.md#012-gm-slash-commands). Lab check with it installed: map 266, `/help` "266 commands found.", `/gmdhd 3` dials, `/gmdhd 29` refused, chat and UI unaffected; a non-GM account was not available. Documented in [commands.md](../../commands.md#game-master-commands) and [gm-cell-method-gating.md](../../architecture/gm-cell-method-gating.md#the-clients-own-command-gate). | DA-06 | Released (diagnosis #1252, patch 012 #1254, v2026-10-05.2 manifest); re-check: `/gmspawnbycmd`, `/gmdespawn` and `/gmdhd 3` work on a GM. Open: the level-0 refusal needs a non-GM account ([follow-up (e)](#open-follow-ups)); two commands' parameter types are unverified ((h)) |
| DA-F9 | A client killed and relaunched within about 60 s could not log in (the DA-06 report's own "DA-F8"). The client binds a fixed UDP port, so the relaunched client's plaintext `baseAppLogin` reached its dead session's established channel, failed to decrypt and was dropped as a retry, while the old tick loop kept sending under the old key. Now a plaintext `baseAppLogin` on an established channel that does not decrypt and carries an unconsumed, unexpired ticket goes to `handle_login`: a session of the same account is evicted (`relaunch_takeover`), a ticket for another account is refused and burned, and `last_recv` refreshes only on fresh traffic. Details: [login-handshake.md](../../protocol/login-handshake.md#a-client-relaunched-on-the-same-addressport). Tests: `relaunch_tests`, `session_teardown_tests` and `liveness_tests` (base). | none | Released (#1246, v2026-10-05.2); the re-check's relaunch logged straight in, but NAT gave it a new source port, so the same addr:port path is covered by the unit tests only ([follow-up (c)](#open-follow-ups)) |
| REL | Close-out: status docs, unified UAT, content patch publish, `/release`. | all above | Done: v2026-10-05.1 carries every packet, v2026-10-05.2 carries DA-F1 to DA-F9; the manifest lists 007, 009, 011 and 012 (010 retired); status docs closed out in #1240. See [Release](#release) |

## Release

- **Server releases:**
  - `v2026-10-05.1` (cut 2026-10-05 02:34Z; `/release` was posted on #1234) carries every packet: SA-01 (#1218), DA-01 (#1223), DA-02 (#1230), DA-03 (#1222, guard fix #1235), DA-04 (#1224), DA-07 (#1232), DA-08 (#1234) and DA-05 (#1240).
  - `v2026-10-05.2` (cut 06:03Z, server `f8f2fec6e`) carries the follow-up wave: DA-F2 (#1244), DA-F9 (#1246), DA-F3 to DA-F8 (#1252), DA-F8's patch 012 (#1254) and DA-F1's patch 011 (#1247).
- **Client content:** the signed content manifest (`content-current`) lists the earlier entries through `007-castle-armory-ring`, then `009-starter-hotbar` (#1213), `011-debug-area-rings-fix` (#1247) and `012-gm-slash-commands` (#1254). `010-debug-area-rings` is retired: see [Patch 010 incident](#patch-010-incident). 011 and 012 both name `"after": "009-starter-hotbar"` instead of chaining one after the other. The launcher's `blocked_by_failure` checks only the one id named, so a ring patch that fails to apply can't hold back the GM commands. The launcher's Update installs them.
- **Status docs:** [gap-analysis.md](../../gap-analysis.md#since-2026-09-25) and [project-status.md](../../project-status.md) gain eleven NT rows (504 features; 138 NT), all new features: three in §5 Character Creation, two in §6 World Entry, one in §16 NPC AI, two in §20 Stargate Travel, two in §37 Ring Transport and one in Admin / GM Tools. No existing row changed status.
- **Still open:** the [open follow-ups](#open-follow-ups) and the tester UAT ([unified UAT § Debug Area](../../guides/unified-uat.md#debug-area)). A new failure becomes a `DA-F<n>` row in the packet table.

### Patch 010 incident

- **What happened.** `010-debug-area-rings` went into the signed manifest at about 02:19Z on 2026-10-05. From then on, every client that loaded Ihpet_Crater_Light hung: in world 1300, and in the public world 73 (Ihpet Crater, Light, gate 20). The client took an `ACCESS_VIOLATION` at `SGW.exe+0xbc6a0`, its main thread stopped ticking, and the watchdog killed it about 30 s later. No minidump was written. DA-06 hit it at 02:41Z, on the first load.
- **Root cause.** The cloned rig's `LightingChannels` struct names a bool property `Dynamic`. The cloner reused Ihpet's name entry for that string. In Castle's name table, where the rig came from, the entry is client-loadable; in Ihpet's it is editor-only (no `RF_LoadForClient`). The client reads an editor-only name as `None`, so the property list ends early and the next tag is read 3 bytes off.
- **Response.** DA-06's A/B test (the stock chunk loads, 010's hangs) pinned it on 010, and 010 was pulled from the manifest at about 02:50Z. daf1 bisected the rig in the lab and found the name. #1247 fixed the cloner: a name is reused only when the target's entry carries every load bit the source's had, and a name audit runs on every cloned object. `011-debug-area-rings-fix` replaced 010 in v2026-10-05.2. It rebuilds the same chunk either from stock plus 007, or from 010's output on installs that applied 010.
- **Damage.** One affected install was repaired by hand. Any other install that applied 010 is repaired by 011 on the launcher's next Update.
- **Why it got out.** 010 was signed before any client had loaded its chunk: the DA-08 checks were queued for DA-06, which ran after the publish. The cause and the guards are in [client patches, 011](../../../data/client-patches/README.md#011-debug-area-rings-fix).

## What each implemented system is tested with

DA-05 wrote the final table, with the step id for each station, in [debug-area.md, Which station tests what](../../content/debug-area.md#which-station-tests-what). The planning version stays below for the record.

| System (unified UAT section) | Station |
|---|---|
| Ability mechanics, ability trees | Z2 ability granter and trainer, Z3 dummies, `.effects` / `.cooldowns` |
| NPC AI (aggro, assist, leash, patrol, wander, cover) | Z4, Z5, Z8 |
| NPC-vs-NPC (#1009) | Z6 |
| Combat, death, respawn | Z7, Z9, respawners 130/131 |
| Vendors, special ammo, consumables, deployables | Z2 vendor |
| Bank and vault, mail, organizations, Black Market, pets, crafting | Z2 copies of the hub services |
| Loot | Z2 crate, any Z7 kill |
| Minigames | Z2 terminal |
| Dialog UI | Z2 dialog NPC |
| GM console parity | anywhere in world 1300 |

Systems tied to a particular map (Castle CellBlock tutorial, Castle, Harset, ring
transport, historical CellBlocks, gate travel) stay tested in their own zones.

## Risks needing the live client

Each risk below is a step of the ordered [DA-06 checklist](#da-06-live-client-checks); the outcomes are in [DA-06 results](#da-06-results).

- The client loading Ihpet_Crater_Light under world 1300 / area name `DebugArea`.
- NPC heights on open terrain (navmesh vs terrain gap).
- Invisible walls or blockers at compound doorways.
- The map's own Kismet or stargate prefab firing on load. Gate 29 now carries the map gate's event set, so the gate opens and the DHD answers (DA-07); the dial-out itself needs a live check (DA-06).
- DA-07 checks for DA-06: as a GM, the Debug Area DHD lists the 13 offered
  gates after the category-13 resync, and a dial opens the gate and the
  `DebugArea.Stargate` volume crosses it. Men'fa (SGU), gate 22: find the
  real gate pad on Menfa_Light (the row is ~192 m below the playable
  surface) and pin `stargates.arrival_*`, then drop it from
  `HUB_EXCLUDED_GATES`. SGC W1 (gate 27): the landing beside its off-mesh
  gate row. A non-GM on an ordinary world (Harset, say) opens a DHD after
  the stargate-table resync, and their list and a dial still work: the
  category-13 bump resyncs every player's gate table, not only GMs'.
- `.gotolocation DebugArea` from Ihpet_Crater_Light (world 73), and back. Both
  send `mapPath = Ihpet_Crater_Light`, and the client skips the UE3 load when
  `mapPath` equals the map it holds (SGW.exe `FUN_00df27f0`). Expected: no
  loading screen, world 73's level state (Kismet, map actors) carries over,
  and the entry still completes, because the client still sends
  `onClientReady` and the base already finishes cross-world entries from it
  (`handle_on_client_ready` synthesises `mapLoaded`). A stall would be fixed
  server-side.
- The minimap location text reads the world-1300 name
  (`getWorldInfo(1300).Name`, from `setupWorldParameters.worldId`), and the
  world map copes with an id that has no shipped map data.

## DA-06 live-client checks

Every "needs the live client" item from the DA and SA packets and their reviews (#1213, #1218, #1222, #1223, #1224, #1230, #1232, #1234), in the order one lab session meets them. Each check names what to look at and the evidence that settles it. A failure becomes a `DA-F<n>` row in the packet table. The tester-facing steps that overlap are in [unified UAT, Debug Area](../../guides/unified-uat.md#debug-area) (DA-U ids), and the lab spec is [debug-area.toml](../../guides/uat-specs/debug-area.toml).

**Before the session.**

- The server runs a build with DA-01 to DA-04, SA-01 and SA-02 (check `service.version` in SigNoz). Checks 22-26 cover DA-07 (#1232) and checks 27-34 DA-08 (#1234), both merged.
- The lab client has patches 007, 009, 011 and 012, all from the signed content manifest (content-current). Never install 010: it hangs the client ([incident](#patch-010-incident)).
- Hold the lab lease, start a packet tap on the lab session (`server_packet_tap_start`), and type `.bug da06 <n>` at each check so SigNoz has a bookmark.
- Use a GM account. Keep a second, non-GM account ready for check 25, and a second client for check 32.

| # | Check | What to look at | Proved by | From |
|---:|---|---|---|---|
| 1 | The client loads Ihpet_Crater_Light as world 1300 after the category-12 resync | From another world, `.gotolocation DebugArea`: a loading screen, then the crater compound; no crash or hang | `client_player_state` `world_id = 1300`. SigNoz `event = 'cooked_data.version_reply' AND category_id = 12` (`full_resync` on the first login after the deploy). Tap: `setupWorldParameters` to the client | #1223 |
| 2 | Minimap location text and the world map for world 1300 | The minimap names the Debug Area (`getWorldInfo(1300).Name`), not CombatSim. Open the world map (M): it copes with a world id that has no shipped map data | `client_lua_eval` `return getWorldInfo(1300).Name`; `lab_screenshot` of the minimap and the world map; no `lua.error` rows in `client_events_read` | #1223 review |
| 3 | Arrival at respawner 130 | You stand on the courtyard floor at (251, 8, -962), not in the air or under the ground. Respawn rotation is always 0, so the facing is whatever that gives | `client_player_state` position; `lab_screenshot` | #1223 |
| 4 | The map's own Kismet and stargate prefab on load | Nothing fires on its own when the map loads: no gate kawoosh, no stray sound or camera. Gate 29 carries the map gate's event set 10005 since DA-07, so the gate should open only when dialled | `lab_screenshot` facing the gate (28 m behind the arrival); `client_events_read` `sequence.dropped`, `lua.error` and `cegui.log` | plan, #1223 |
| 5 | Same-map travel between world 73 and world 1300 | `.gotolocation Ihpet_Crater_Light`, then `.gotolocation DebugArea`, then back. Expected: no loading screen (the client skips the UE3 load when `mapPath` is unchanged), the entry still completes, and world 73's level state carries over. A stall is a server fix, not a client patch | `client_player_state` `world_id` flips 73 and 1300 and the player can move. Tap: `onClientReady` to the server each time. SigNoz: the base's "synthesising mapLoaded for cross-world transition" line. The minimap text changes (check 2) | #1223 review |
| 6 | Plaza NPC heights and clearance | All 21 plaza NPCs and the 5 dummies stand on the floor, and the 10.5 u ring is clear of the courtyard's walls and props | `client_entity_find` (`max_distance_m` 20 from (252, 7, -923)) positions; `lab_screenshot` from the plaza centre, turning with `client_camera` | #1230 |
| 7 | Spawn heights on open terrain | Z4 yard rows; the Z5 patroller between z -690 and -650, where the navmesh floats up to 2.6 m above the terrain; the Z9 squad and guards, grounded on a navmesh up to 2 m off the terrain; the largest gallery bodies (Titan, BattleWalker, Carnosaur), which may overlap at 8 m | `client_entity_find` `position.server.y` against what the screenshot shows; `server_entity_query` `around_point` for the server's y; `lab_screenshot` at each station | #1222, #1224 |
| 8 | Doorway collision | Walk through the compound's doorways: the cover-course entry (204, 7, -926), the west room's doorway on the x 151 wall (z -961 to -963), and the courtyard's own gaps. No invisible wall | `client_move_to` through each doorway: a `leg_N` failure with unstick attempts is a blocker. `lab_screenshot` where it stops | plan |
| 9 | The ability granter's click reaches the server | Right-click Train Testing Abilities: the trainer cursor shows, and the click is not swallowed by a trainer window (the template has no trainer list) | Tap: `interact` to the server, then `onKnownAbilitiesUpdate` to the client. SigNoz `event = 'ability_granter' AND change = 'grant_all'`. Chat: "Ability granter: granting ..." (DA-U3) | #1230 |
| 10 | Dialog 5738 renders for Airman Lance | Two screens, Next pages through them, one Generic button; the button brings "Dialog round trip complete"; X sends nothing | `client_ui_state` `/dialog`; `lab_screenshot` of each screen. SigNoz `scope_name = 'dialog.display' AND dialog_id = 5738`. Tap: `dialogButtonChoice` to the server (DA-U9) | #1230 |
| 11 | The friendly dummy can be targeted for a heal | Left-click the Injured SGC Guard: it becomes the target. Health Heal raises its bar, not yours | `client_target` (`Unit.Target` becomes it, no fallback); `client_player_state` `/target/health/current` before and after. SigNoz `event = 'beneficial_cast'` `resolution` (DA-U20) | #1230 |
| 12 | Passive faction-10 NPCs and the gallery's look | How the client colours a passive (NEUTRAL override) faction-10 NPC in the gallery and the yard's pinned row; 62 blank nameplates (K26); 75 pistol animations on creature and staff bodies (K26) | `client_entity_find` `hostility`; tap `onAggressionOverrideUpdate`; `lab_screenshot` along the terrace | #1222 |
| 13 | Do riflemen crouch visibly at cover? | Shoot rifleman 1 from the hall (DA-U35). There is no server-to-client pose message, so record whether the client crouches or leans the NPC at its marker | SigNoz `scope_name = 'cover.stance' AND event = 'granted' AND template_id = 1375` gives the moment; `lab_screenshot` at that moment | #1224 |
| 14 | NPC-on-NPC hit visuals | In the arena (DA-U32): shots between the squads draw fire animations and hit effects, and the target's health bar drops. This is the open CP19 question (K20) | `client_combat_log` records where neither side is the player; `lab_screenshot` during a round. SigNoz `scope_name = 'npc_ai.aggro' AND tag LIKE 'DebugArea_Arena_%'` for the timing | #1224, CP19 |
| 15 | Lucia names and bodies | The spectator pair shows "Green Sniper" and "Yellow Faction" nameplates, and the female Yellow body draws | `client_entity_find` names `Green Sniper`, `Yellow Faction`; `lab_screenshot` | #1224 |
| 16 | The arena squads stand on the floor | DA-06 failed this on the pit's water plane (DA-F2). Since DA-F2 the squads are on the east shelf (y -11.12): they stand on the terrain, fight 1's lines see each other past the ruin walls, and a player walking on from the east gap reaches them at the same height | `lab_screenshot` from the terrain north of the shelf (354, 9, -660) and from the shelf beside the Praxis line (390, -11.1, -741); `server_entity_query` heights of the player and the squads agree | #1224, DA-F2 |
| 17 | Patch 009 seeds a new character's bar | A new character's first login: buttons 11-15 (Alt+1 to Alt+5) hold 592, 594, 597, 1646, 1218 with icons; one chat line "Your starting abilities are on your action bar."; Alt+4 fires Health Heal on the first press | `client_hotbar {include_empty: true}`; chat; `client_use_ability {ability_id: 1646, press: key}` reads `effect_applied` (DA-U42) | #1213 |
| 18 | Patch 009 across relogs | Relog the same character without restarting the client: the bar is unchanged (native action persistence) and the line does not repeat. Then create and play a second new character in the same process: it is seeded. An existing character's bar is never touched | `client_hotbar` after each login; chat; `lab_logout` and `lab_play_character` without `lab_client_restart` | #1213 |
| 19 | The login marker and the known-abilities call | `getSystemTime()` differs between two logins in one process (the login marker relies on it when the Lua state is rebuilt). `getAbilityList()` returns the known ids, and `getAbilityList(2)` raises | `client_lua_eval` `return getSystemTime()` at each login; `return getAbilityList()`; `return pcall(getAbilityList, 2)` returns false | #1213 |
| 20 | The starter pistol at spawn, drawn on the first press | A new character spawns with the SI 3 9mm Pistol holstered in the active bandolier slot, 15 rounds. The first Pistol Shot draws it (about 1 s), then fires; later presses fire at once; no out-of-ammo error | `client_player_state` `/ammo` (`weapon_item_id` 55, `current` 15). SigNoz `event = 'weapon_draw_queued'`, then `event = 'weapon_ability_redirect'` with `weapon_ability_id = 579` (DA-U39, DA-U40, SK1) | #1218 |
| 21 | Mission 622 with two pistols | In the Castle Cellblock: search the Guard (a second item 55 lands in the backpack), drag it into a bandolier slot: 622 completes and the stasis door opens. Moving the starter pistol out of the bandolier and back also satisfies the step | `client_inventory` diff; SigNoz `body CONTAINS 'mission_id=622'`; the door opening in `lab_screenshot` (Cellblock T03/T04) | #1218 |
| 22 | The Debug Area DHD lists 13 gates | After the category-13 (stargate table) resync, right-click the DHD as a GM: the window lists the 13 gates on loaded worlds (Men'fa (SGU) is left out on purpose). Also check whether an already-open window refreshes on `updateStargateAddress` (the server sends them before opening it) | `client_window_read` on the DHD window; tap `updateStargateAddress` (method 66) before `onDisplayDHD`; SigNoz `reason = 'gm_dial_hub_grant'` and `event = 'cooked_data.version_reply' AND category_id = 13` | #1232 |
| 23 | Event set 10005 opens the map gate under world 1300 | Dial Harset: the map's own gate opens about 4 s later; walking through the `DebugArea.Stargate` volume loads Harset | `lab_screenshot` of the open gate; `client_move_to` into the event horizon; `client_player_state` `world_id` changes; SigNoz gate-travel rows for the dial | #1232 |
| 24 | The Men'fa (SGU) and SGC W1 arrivals | SGC W1's gate row is 3.4 m off the navmesh: record where you land. Men'fa (SGU) is not offered at the hub, because its gate row is about 192 m below the playable level; pin its arrival from an in-client look (`/gmgotolocation Menfa_Light ...`) so it can come back. `.gotolocation DebugArea` recovers | `client_player_state` position after each dial; `lab_screenshot` | #1232 review |
| 25 | A non-GM DHD on another world after the resync | The category-13 bump resyncs every player's gate table. A non-GM on an ordinary world opens a DHD: their own list shows and dialling still works | The non-GM account: `client_window_read` of the DHD list; one dial completes. SigNoz `reason = 'dial_hub_not_gm'` never appears outside world 1300 | #1232 review |
| 26 | The template-1 DHDs on the nine other worlds now open | Harset, Tollana, Lucia, Omega Site, Beta Site E1, Dakara E1, both Ihpet Craters and Men'fa (Praxis): a right-click opens the dialling window | `client_world_click` on each DHD with `expect: window` | #1232 |
| 27 | patch 011 loads in world 1300 and world 73; eight rigs on the floor | The patched chunk `Ihpet_Crater_Light-fff80002` streams in both worlds; all eight rigs render at floor height. The slope and death-yard pads sit on 0.5-1 m slopes | `lab_screenshot` at each of the eight pads; `client_events_read` with no `lua.error` or crash; `lab_crash_report` empty | #1234 |
| 28 | the rig base is lit outdoors | The cloned base has no lightmap (`LMT_None`) in a baked-light outdoor map. It must not render black or flat | `lab_screenshot` of a pad in daylight | #1234 review |
| 29 | sequences 10189-10204 animate the right rig | Right-click a console, pick a destination, step on the pad: that pad's rings rise, flash and drop, then the destination's. Rigs 1-3 of 007 are also unconfirmed in-client | Tap `onSequence` with the station's ids; `lab_screenshot` mid-sequence; `client_events_read` `sequence.dropped` | #1234 |
| 30 | the ring sound | The rig plays the FMOD event `prp_gen/rings/transport`; check that the `prp_gen` bank loads on this map | Listen during check 29; the client log for an FMOD bank error | #1234 |
| 31 | the destination picker | The picker opens on world 1300's world map with the seven other stations at the right places (no labels: every ring's display name is empty, as on shipped rings) | `lab_screenshot` of the picker; `client_window_read` | #1234 |
| 32 | what witnesses see, at both ends | A second player beside the source pad sees the rings animate. A second player at the destination sees Teleport In: the client may drop it when the traveller's pawn is not ready yet | The second client: `client_events_read` `sequence.dropped` with `path = no_source_pawn` for ids 10190, 10192, ... is the failure; `lab_screenshot` on both clients | #1234 review |
| 33 | the arena shelf pad | Region 39 moved off the pit's water plane to the east shelf (331, -11.12, -693) in DA-F1: it must stand on rendered ground, with its rig clear of the shelf's ruin walls | `lab_screenshot` on arrival at the shelf pad | #1234, DA-F1 |
| 34 | world 73's platforms are inert scenery | World 73 shows the eight platforms but nothing on them works. `GLB-Global` now loads on demand there | `lab_screenshot` in world 73; no console to click; `client_events_read` with no errors | #1234 |

## DA-06 results

Two lab sessions on 2026-10-05, both against the colo with the GM lab account (worker da06):

- **Run 1, v2026-10-05.1** (server `86fe5dcab`, 02:37-04:01Z), with a fresh SGU Soldier. Its faults are DA-F1 to DA-F9 in the packet table. Patch 010 hung the client, so the ring checks ran on daf1's candidate fix chunk (the name-table fix 011 later shipped) once the bisect had found the cause.
- **Re-check, v2026-10-05.2** (server `f8f2fec6e`, 06:07-06:27Z), with a fresh SGU Soldier on the colo DB rebuilt by the deploy. 011 and 012 were installed with `cimmeria-patchset apply`; both zip hashes matched the live manifest. It re-ran every check a fix touched.

**Totals: 18 PASS, 12 PARTIAL, 2 BLOCKED, 2 NOT RUN, no open FAIL.** Most PARTIALs are lab limits, not faults: the harness can't turn the camera ([#1243](https://github.com/SandboxServers/Cimmeria/issues/1243)), so a check that needs a look in a particular direction stopped at what the telemetry proves.

| # | Result | Evidence | Release |
|---:|---|---|---|
| 1 | PASS | Run 1 hung on every load with 010 installed (DA-F1). With the stock chunk, `getCurrentWorldID()` = 1300, and `cooked_data.version_reply` went out for categories 12 and 13 on each of 6 logins. The re-check loaded world 1300 on 011 with no `client.os.exception` | v2026-10-05.1 (FAIL with 010), v2026-10-05.2 |
| 2 | PASS | `getWorldInfo(1300).Name` = "DebugArea"; the minimap reads DebugArea, not CombatSim; the world map draws the crater with the station icons | v2026-10-05.1 |
| 3 | PASS | After `onClientReady` the player settles at y 7.11 on the courtyard floor at (251, -962); the reply ends `[stargate arrival]` | v2026-10-05.1 |
| 4 | PARTIAL | No kawoosh and no `lua.error`; all 176 `sequence.dropped` rows are arena casts. One 2 s `SeqAct_Interp_0` of unknown content plays as `fff90002` streams in. The gate-facing screenshot needs the camera | v2026-10-05.1 |
| 5 | PASS | 1300 → 73 → 1300 in about 2 s each with no loading screen; `onClientReady` reaches the server and the base synthesises `mapLoaded` (4 rows); the minimap text flips | v2026-10-05.1 |
| 6 | PASS | The 20 named plaza NPCs and the crate stand at y 6.99-7.10 on the 10.5 u ring, the 5 dummies at 6.59-7.08, all on the floor in the screenshot | v2026-10-05.1 |
| 7 | PARTIAL | Yard: all 11 client y values equal the server's, on the snow. Gallery row z -612 at y 23.19-23.46. The Z5 patroller, the Z9 squad and the largest gallery bodies were not viewed | v2026-10-05.1 |
| 8 | PARTIAL | The west room's x 151 doorway is open (strafed through to x 147.4). The cover-course entry and the courtyard gaps were not walked | v2026-10-05.1 |
| 9 | PASS | `interact` → "Ability granter: granting 72 abilities ..." → `onKnownAbilitiesUpdate` (79); SigNoz `ability_granter` `grant_all`, chain 13000; no trainer window | v2026-10-05.1 |
| 10 | PASS | Dialog 5738 on entity 100207: two screens, one Generic button; `dialogButtonChoice(5738, 198)` brings "Dialog round trip complete"; the portrait renders. X was not tried | v2026-10-05.1 |
| 11 | PASS | Run 1: targeting and the heal work (500000 → 600000, `beneficial_cast resolution='ally'`), but only after moving the dummy out from behind a wall (DA-F7). The re-check healed it at its new spot from 4.5 m on the open south side. No heal number shows: the server sends no `onEffectResults` for the heal | v2026-10-05.2 |
| 12 | PARTIAL | Passive gallery NPCs read hostility 3 (the NEUTRAL colour); blank nameplates confirmed (K26). The animations need a human look | v2026-10-05.1 |
| 13 | PARTIAL | Rifleman 2 acquired the character (`npc_ai.aggro acquired`), but no `cover.stance` row came and the pose was not seen | v2026-10-05.1 |
| 14 | PARTIAL | The server sends NPC-vs-NPC `onSequence`, `onEffectResults` and `onStatUpdate` to an observer, and the client plays the fire sequences (`client.ability.shown kind=sequence_played`). `SCTMod.onUnitCombat` caught 0 events, so NPC-only hits show no floating combat text (K20). The health-bar drop was not seen | v2026-10-05.1 |
| 15 | PARTIAL | "Green Sniper" ×2 and "Yellow Faction" ×2 nameplates; the female Yellow body needs a human look | v2026-10-05.1 |
| 16 | PASS | Run 1 FAIL: the player sank to y -52 under squads standing at y -33 (DA-F2). Re-check on the east shelf: both fights started within 14 s of arrival; at (372, -741) a NID Guard acquired the player at 18.2 m with clear sight, and the Praxis line finished the NIDs; the player stood at shelf height (`is_on_ground true`). Over 4 min: 15 deaths, 11 respawns, no arena WARN or ERROR rows, every leash a `target_dead` reset | v2026-10-05.2 |
| 17 | PASS | Buttons 11-15 = 592, 594, 597, 1646, 1218, and the one chat line, on the first login. Run 1 showed gear placeholders on a-4 and a-5 (DA-F6); the re-check's fresh character shows the Medkit icons | v2026-10-05.2 |
| 18 | PASS | The bar is unchanged over 2 relogs and 3 client restarts, and the line does not repeat; a second new character in the same process was seeded once | v2026-10-05.1 |
| 19 | PASS | `getAbilityList()` returns the five ids; `pcall(getAbilityList, 2)` is false; `getSystemTime()` per login reads 30.79, 4.98, 10.99 (relative to the Lua state, so it differs between logins) | v2026-10-05.1 |
| 20 | PASS | `/ammo`: SI 3 9mm Pistol, 15/15, slot 1. The first Alt+1 draws it (`onSequence 1872`, `weapon_draw_queued`) and fires about 1.0 s later; the second press fires at once; 2× `weapon_ability_redirect` 579; no ammo error | v2026-10-05.1 |
| 21 | NOT RUN | The Cellblock flow needs looting and drag-drop with camera control | none |
| 22 | PASS | Run 1: 13 gates granted, but 11 read "Unknown" on the first open (DA-F3). Re-check: the first open lists all 13 by name; the world-entry grant gave 12 (1 already known), so the DHD open had nothing left to send | v2026-10-05.2 |
| 23 | PARTIAL | Dialling Harset plays `onSequence 10061` and writes "dial accepted, gate opens in 4s" and "opening gate"; a `.gotolocation` into the gate centre while it was open loaded Harset. Walking into the volume and the open gate on screen were not seen. The re-check's `/gmdhd 3` dials the same way | v2026-10-05.1, v2026-10-05.2 |
| 24 | NOT RUN | Not tried. With no pin, world 73's arrival lands on its gate row (251.25, 10.606, -989.781) | none |
| 25 | BLOCKED | No non-GM lab account ([follow-up (e)](#open-follow-ups)) | none |
| 26 | PARTIAL | Harset's DHD opens on right-click; the other eight worlds were not tried | v2026-10-05.1 |
| 27 | PARTIAL | 010 hung the client (DA-F1). daf1's fix chunk, and the 011 zip in #1247's lab run, load world 1300 and world 73 with no exception; the re-check ran on 011's chunk (`52b4f3ad...`). Rigs seen at floor height: Compound, Death yard, Arena shelf. The other five pads were not viewed | v2026-10-05.1 + fix chunk, v2026-10-05.2 |
| 28 | PASS | The Compound and Death yard bases render on the floor and lit, not black or flat | v2026-10-05.1 + fix chunk |
| 29 | PASS | Compound → Death yard plays 10189 then 10204, and the rings visibly rise (fix chunk). Re-check: Compound → Arena shelf plays 10189, then 10198 4.1 s later; back plays 10197 then 10190. 007's rigs 1-3 were not checked | v2026-10-05.1 + fix chunk, v2026-10-05.2 |
| 30 | PARTIAL | `audio.event` `transport` starts and stops with result 0; nobody listened | v2026-10-05.1 + fix chunk |
| 31 | PASS | The picker opens on world 1300's map with 7 icons where the other stations are | v2026-10-05.1 |
| 32 | BLOCKED | No second lab client | none |
| 33 | PARTIAL | Blocked on run 1 (010, and the pad was still on the water). Re-check: arrived at (331, -10.58, -693) on the ground; the frames show the ruin blocks behind the rear pillars with a gap and nothing intersecting. The ring discs were never caught on screen ([follow-up (b)](#open-follow-ups)) | v2026-10-05.2 |
| 34 | PASS | World 73 shows the platforms with no console entity | v2026-10-05.1 + fix chunk |

**Tester steps run in the lab.** PASS: DA-U1, U3, U4, U9, U20 and U39 to U43. PARTIAL: DA-U19 (the hit drains the absorb pool, health stays full and the dummy never fires back; the level-10 damage number was not checked), U21 (out of combat 11 s after the last shot; the refill and XP were not checked), U45 (the list, a dial and `/gmdhd 3` work; walking into the gate was not seen) and U46 (travel works; the sound, a second player and the discs are open). Blocked by the spec itself: DA-U2, U7 and U44. The other rows were not run, because the harness can't turn the camera.

## Open follow-ups

None of these blocks the campaign. Each names where the work lands. Fix **(o)** first: it is the only one that shows players something they should never see.

| Id | Follow-up | Where |
|---|---|---|
| (a) | The too-far feedback line names the server template ("You are too far away from Debug Hub - Gate Mail Clerk"), not the name the client shows (Sgt. Harriman). It should use the shown moniker. | `reject_interact_out_of_range` in `crates/cell-interactions/src/cell/interactions/dispatch/range_feedback.rs` (DA-F5) |
| (b) | One human look at the ring discs rising on the Arena shelf rig, against the ruin walls behind it. The lab frames show the pillars clear of the walls, but never caught the discs. | Check 33, DA-U46 |
| (c) | #1246's same addr:port relaunch path was not exercised live: NAT gave the relaunched client a new source port. The unit tests cover it (`relaunch_tests`). | DA-F9; a lab client that keeps its port, or a LAN client |
| (d) | The out-of-view too-far case can't be tested from a stock client: it needs an `interact` for an entity id the client never received. `out_of_range_interact_on_an_npc_out_of_view_sends_nothing` covers it. | DA-F5 |
| (e) | The non-GM checks need a non-GM lab account: DA-U2, check 25, GM-only entry to world 1300, and the client refusing `/gm*` at access level 0 (012's `Access="p"` mask). | Lab accounts; DA-F8 |
| (f) | Lab harness issues: camera turning, `lab_uat_run`'s 422s and 60 s timeout, `client_hotbar`'s `sub_id`, chat after a relog. | [#1243](https://github.com/SandboxServers/Cimmeria/issues/1243) |
| (g) | DHDs render as solid black, untextured silhouettes: the Debug Area DHD (spawn 13800, template 1) and world 73's. Clicking works. | Unified UAT K28 |
| (h) | `/gmgiveminigamecontact` and `/gmremoveminigamecontact`: the parameter types 012 declares are unverified (#1254). | `data/client-patches/012-gm-slash-commands/commands.toml` |
| (i) | The cooked-data categories in `metadata_bump.rs` still derive their version from `DefaultHasher`, whose algorithm std leaves unspecified between Rust releases. A toolchain bump could then change the versions and make every client resync those categories once. DA-F6's attribute patches already use a stable 32-bit FNV-1a. | `crates/resources/src/base/resources/metadata_bump.rs` |
| (j) | The pre-existing CME prose file `docs/analysis/sgw-handoff-pack-v1.2/pack/references/source_extracts/SlashCommands_FullTable(1).txt` waits on an owner decision (keep, or remove under the no-CME-text rule). 012 is generated without it. | Owner |
| (k) | SGC_W1's Groom Jaffa 1-3 spawn off the navmesh. This is pre-existing seed placement, not a Debug Area change. | `db/resources/` SGC_W1 spawn seed |
| (l) | Men'fa (SGU), gate 22: its arrival is about 190 m under the map, so it stays out of the dial hub (`HUB_EXCLUDED_GATES`) until an in-client look pins a real pad (check 24). | `crates/cell-interactions/src/cell/gate_travel/dial_hub.rs`, `stargates.arrival_*` |
| (m) | The dial hub's GM grant (`gm_dial_hub_grant`) logs at WARN twice per GM session, at world entry and at the first DHD open, and WARNs flow to Discord. It follows the gmDHD `gm_address_grant` audit precedent. Decide whether GM audit lines should be INFO, or go to a dedicated audit channel. | `dial_hub.rs`; `crates/cell-console/src/cell/console/gm/travel.rs` |
| (n) | `/gmdespawn` still sends target id 0 from the client (`00000000 7d 00000000`). It works only through the server's fallback to the GM's selected target (#1254). | 012's `/gmdespawn` declaration; the server fallback |
| (o) | **Most important.** `.`-console commands typed in chat are echoed to the Say channel (`[<name>] says .gotolocation ...`), so nearby players see GM command text. The command still runs. A `.` line should be consumed by the console and never broadcast. | The chat path that hands `.` lines to the console (`crates/cell-console/src/cell/console/chat/mod.rs`, `dispatch.rs`); DA-06 observation O6 |
| (p) | A heal shows no number: the server sends no `onEffectResults` for a heal (Health Heal on the Injured SGC Guard raised its health with nothing on screen). | `crates/cell-combat/src/cell/abilities/use_ability/beneficial.rs`; check 11 |
| (q) | The client drops method 116 on the GM player class (`client.dispatch.method_dropped`, `type_id 3`, `method_index 116`) about once per `.gotolocation`. The teleport still works, through the forced position. Probably `onPlayerTeleport` on `SGWGmPlayer`. | GM class method table against the client's; DA-06 observation O2 |
| (r) | "New Mission: Report to someone in charge." is announced again on every world entry, centre-screen and in chat. | Mission state sent at world entry; DA-06 observation O5 |
| (s) | A ring hop makes the client send `setTargetID(<own id>)` and then `setTargetID(0)`. Harmless so far, but the server briefly holds the player as their own target. | Ring transport flow; re-check observation O-r3 |

**Lab client state.** The lab client has 007, 009, 011 and 012 applied by hand with `cimmeria-patchset apply`, but its `launcher-installed.json` still lists only 001-006 and the black-market overlay. A launcher run would try to apply those four patches again. Reconcile the file, or let the launcher reinstall from stock, before the lab's next launcher Update.

## DA-09 System Lords' summit

Asked for 2026-10-05: a circle of System Lords somewhere fitting, dressed
"nicely and uniquely", with petty needs and squabbles barked at each other
on a schedule, "something that would really get Teal'c's goat". Tested in
the lab before release.

**Lab check** (2026-10-05, a local server built from the branch on its own
`sgw_da_lords` database, the lab client logged in through a temporary
`Local` row in the lab install's `LoginInternal.lua`, removed afterwards):

| Check | Result |
|---|---|
| The chatter plugin installs and the catalog loads | PASS: `cell plugins installed plugins=["pets","duel","org","chatter"]`, `Loaded ambient chatter groups=1 lines=75`, `chatter.ready live_groups=1` |
| Lines reach the client as NPC say chat | PASS: the chat window shows `[Ra] says Why is my throne the same height...`, `[Ba'al] says ...`, `[Ra's Jaffa] says Indeed.`; `client_chat_log` reads speaker `Ra` / `Ba'al`, channel 0 |
| A scene starts only with a listener, then runs on its delays and gap | PASS: exchange 1 started 1 s after the GM arrived; lines 4.6-7.1 s apart; next exchange 30 s after the last line; exchanges 1 to 11 in order over 12 minutes |
| Lines go only to players within the hear radius | PASS: from 19.5 m Ba'al's lines went to nobody (`listener_count=0`) while Ra's and the Jaffa's, within 18 m, arrived |
| Every lord renders, dressed | PASS after one fix: Ra drew as the client's placeholder cube with `NPC_Ra_Head_00` and `NPC_RaG_FingerNail_00` added to his kit; a side-by-side of six variants showed template 41's kit renders, so 1400 now uses it. Ba'al (trader robes), Athena, Morrigan, Anat, Nerus and Ra's Jaffa rendered as designed |
| The spot looks right | FAIL on the first spot, fixed: the palace terrace (218, 30.9, -532) draws its terrain white with magenta streaks and the terrace east of it is grey void. Moved to the compound courtyard (282, 6.9, -944), which draws properly |
| The landing spot hears every lord | Not seen in the client: the lab avatar snapped back after a same-space `.gotolocation` twice (the server logged the teleport; the client stayed put). Pinned instead by `the_landing_spot_hears_every_lord`; the 15 m radius first chosen left Ra's Jaffa out of earshot of it, so the radius is 18 m |

Not checked: a second player hearing the same scene (no second lab client),
and the lords' facing in the client (the lab harness cannot turn the
camera, #1243).
