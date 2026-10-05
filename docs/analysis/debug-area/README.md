# Debug Area and Starter Kit

> Type: how-to and ledger. Audience: the coordinator, packet workers and the
> playtesters. Opened 2026-10-04 against `main` @ `bd020cb95`. Prefix `DA-` for the
> Debug Area, `SA-` for the starter kit. Same dispatch rules as the
> [ability mechanics ledger](../ability-mechanics/work-packets.md#dispatch-rules).

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
| D-SA1 | Every character starts with a basic pistol and ammo in the active bandolier slot. | Makes Pistol Shot usable at spawn, as asked. The Castle CellBlock tutorial still hands out its pistol; SA-01 records any deviation. |
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
| Z3 dummies range | line x 240..264 at z -872, y 6.6 | Non-retaliating dummies, friendly heal target | DA-02 |
| Z4 faction yard | (416.0, -5.7, -786.0), radius 25 | Friendly row x≈400, neutral row x≈416, hostile pen x≈434 | DA-03 |
| Z5 AI behaviour slope | A (20.0, 13.9, -760.0) ↔ B (22.0, 20.6, -650.0); wanderer (74.0, -0.4, -746.0) | Patrol, wander, leash, assist pair | DA-03 |
| Z6 NPC-vs-NPC arena | pit (250.0, -32.4, -725.0), flat radius ≥ 25 | Squads at x 238 / 262 | DA-04 |
| Z7 enemy gallery | terrace y 23.1, rows z -592 / -612, x 55..170 and 305..465 | Every hostile template, passive | DA-03 |
| Z8 cover course | south compound west wing: entry (204, 7.0, -926), riflemen (166, 6.7, -930), (142, 6.7, -958) | `use_cover` riflemen among 161 cover nodes | DA-04 |
| Z9 death and respawn test + respawner B | (438.0, 10.4, -916.0) | Respawner 131, a lethal hostile | DA-04 |

## Id blocks

| Range | Owner |
|---|---|
| World 1300, respawners 130-131 | DA-01 |
| Templates 1300-1329, spawns 13000-13199 | DA-02 |
| Templates 1330-1369, spawns 13200-13599, point sets 13200-13209, points 13200-13299 | DA-03 |
| Templates 1370-1399, spawns 13600-13799 | DA-04 |
| Spawns 13810-13817, ring regions 35-42, event sets / point sets / points / chains 13810-13817, sequences 10189-10204 (13800 is DA-07's) | DA-08 |

Check each range is free in the seed before using it; raise a clash with the
coordinator instead of moving into another packet's block.

## Packets

| Packet | Scope | Depends on | Status |
|---|---|---|---|
| SA-01 | Seeded playtest characters rebuilt as Praxis Commandos identical to normal creation; starter pistol and ammo for every char_def (D-SA1); live-DB guards that every char_def and seeded character holds 592, 597, 1646, 1218 and the pistol. | none | Integrated (#1218) |
| SA-02 | Client patch `009-starter-hotbar` (D-SA2), Lua logic UAT against stock and v26 `ActionProfiles.lua`. Publishing the manifest is a coordinator step. | none | Integrated (#1213); manifest not yet published |
| DA-00 | This plan. | none | Integrated (#1210) |
| DA-01 | World plumbing: world row, `spaces.xml`/`cell_spaces.xml`, a table of Cimmeria-added worlds feeding `world_id_for_name`, `client_map_for_world` and `WORLD_INFO_OVERRIDES` (also closing the missing shipped names: 50, 61, 62, 69, 70, 72, 73, 78), fail-closed `resolve_space_id_fallback`, nav/occ fallback to the client map (D-DA5), advisory list, respawners 130/131, generated cover nodes for world 1300, `.gotolocation DebugArea`. | none | Integrated (#1223) |
| DA-02 | Z2 services plaza (vendor, trainer, dialog NPC, terminal, loot crate, registrars, pet trainer, bankers, mail clerk, Black Market auctioneer, crafting stations) and an ability granter that gives the clicking player every ability of their archetype; Z3 non-retaliating dummies (D-DA7). | DA-01 | Integrated (#1230): 21 plaza NPCs (spawns 13001-13022), an ability granter and reset NPC (templates 1300/1301, content action `gm_ability_bulk`, chains 13000-13008), a munitions vendor (1302, buy list 1300), and five training dummies (templates 1310-1314, spawns 13100-13104) flagged by the new `entity_templates.training_dummy` column, which reuses `.dummy`'s mark. Reference: [debug-area.md](../../content/debug-area.md). |
| DA-03 | Z4 faction yard, Z5 patrol/wander/leash/assist, Z7 enemy gallery with every hostile template (D-DA9). Seeded: templates 1330-1333 and 1340-1343, spawns 13200-13245 and 13300-13398, patrol point set 13200 (points 13200-13201). Gallery: 99 of 101 hostile templates placed (140/141, children, excluded), 24 with an ability set, 75 on the 592 fallback, 62 with no display name. Yard rows moved to z -806..-794 where nav and terrain agree. See [debug-area.md](../../content/debug-area.md). | DA-01 | Integrated (#1222; seed-wide guard fix #1235) |
| DA-04 | Z6 arena (D-DA8), Z8 cover course, Z9 death and respawn test. Templates 1370-1377, spawns 13600-13641 in `*_debug_area_combat.sql`; reference [debug-area.md](../../content/debug-area.md). | DA-01 | Integrated (#1224) |
| DA-07 | The Debug Area stargate: gate 29 (outbound-only dial hub) on the Ihpet_Crater_Light gate prop, its gate volume, DHD and cooked entry; a GM at its DHD can dial every gate on a loadable world; nobody can dial in (D-DA4 as amended); template 1 DHDs made clickable. Ids: spawn, point set and point 13800, stargate 29. | DA-01 | Integrated (#1232) |
| DA-05 | `docs/content/debug-area.md`, `docs/guides/uat-specs/debug-area.toml`, unified UAT section mapping every system to a station. | DA-02..04 | Review (#DA05PR): 46 spec rows, 42 ready in the lab and 4 blocked (non-GM account, store buttons, seeded account, DA-08); [unified UAT, Debug Area](../../guides/unified-uat.md#debug-area) DA-U1 to DA-U46; station index in [debug-area.md](../../content/debug-area.md#which-station-tests-what); the [DA-06 checklist](#da-06-live-client-checks) below |
| DA-06 | Live-client check in the lab: the map loads as world 1300, spawn heights, doorway collision, map Kismet, every station answers. DA-08 rings: all eight rigs render at floor height and the base is not black or flat (unlit clone outdoors); `onSequence` 10189-10204 animates the right rig and the ring sound plays (`prp_gen/rings/transport`, waveform in `genprp/prp_gen.fsb`, which 006 does not copy); two clients: a witness by the source pad sees Teleport Out, and a destination witness sees Teleport In (read `client.sequence.dropped` for 10190/10192/.../10204 on the second client; a drop is the documented cosmetic limitation). Fixes as `DA-F<n>`. Checklist: [DA-06 live-client checks](#da-06-live-client-checks). | DA-01..04 deployed | BlockedDependency (needs a deploy and the lab) |
| DA-08 | Ring transports: client patch `010-debug-area-rings` clones eight working ring rigs (region 3's Castle rig with its Kismet) into `Ihpet_Crater_Light-fff80002`, and the world-1300 seed wires a fully connected ring network across them (Compound, Faction yard, AI slope, Arena rim, Arena pit, Gallery west, Gallery east, Death yard). Ring `onSequence` now reaches witnesses too. Station table in [debug-area.md § Ring transports](../../content/debug-area.md#ring-transports). Published in the signed content manifest after 007 (content-current, 2026-10-05). | DA-01 | Integrated (#1234) |
| REL | Close-out: status docs, unified UAT, content patch publish, `/release`. | all above | BlockedDependency |

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

Each risk below is a step of the ordered [DA-06 checklist](#da-06-live-client-checks).

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

- The server runs a build with DA-01 to DA-04, SA-01 and SA-02 (check `service.version` in SigNoz). Checks 22-26 need DA-07 (#1232, merged), and checks 27-34 need #1234.
- The lab client has patch 009 (`cimmeria-patchset apply data/client-patches/009-starter-hotbar.zip --install <client>`). For checks 27-34 it also needs 007, then 010.
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
| 16 | The arena squads on the pit's water plane | The pit floor is a water collision plane (K27). The squads must not be drawn swimming, sunk or floating | `lab_screenshot` from the west rim (176, -6, -702) and from the pit floor | #1224, #1234 |
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
| 27 | After #1234: patch 010 loads in world 1300 and world 73; eight rigs on the floor | The patched chunk `Ihpet_Crater_Light-fff80002` streams in both worlds; all eight rigs render at floor height. The slope and death-yard pads sit on 0.5-1 m slopes | `lab_screenshot` at each of the eight pads; `client_events_read` with no `lua.error` or crash; `lab_crash_report` empty | #1234 |
| 28 | After #1234: the rig base is lit outdoors | The cloned base has no lightmap (`LMT_None`) in a baked-light outdoor map. It must not render black or flat | `lab_screenshot` of a pad in daylight | #1234 review |
| 29 | After #1234: sequences 10189-10204 animate the right rig | Right-click a console, pick a destination, step on the pad: that pad's rings rise, flash and drop, then the destination's. Rigs 1-3 of 007 are also unconfirmed in-client | Tap `onSequence` with the station's ids; `lab_screenshot` mid-sequence; `client_events_read` `sequence.dropped` | #1234 |
| 30 | After #1234: the ring sound | The rig plays the FMOD event `prp_gen/rings/transport`; check that the `prp_gen` bank loads on this map | Listen during check 29; the client log for an FMOD bank error | #1234 |
| 31 | After #1234: the destination picker | The picker opens on world 1300's world map with the seven other stations at the right places (no labels: every ring's display name is empty, as on shipped rings) | `lab_screenshot` of the picker; `client_window_read` | #1234 |
| 32 | After #1234: what witnesses see, at both ends | A second player beside the source pad sees the rings animate. A second player at the destination sees Teleport In: the client may drop it when the traveller's pawn is not ready yet | The second client: `client_events_read` `sequence.dropped` with `path = no_source_pawn` for ids 10190, 10192, ... is the failure; `lab_screenshot` on both clients | #1234 review |
| 33 | After #1234: the pit pad on the water plane | The arena-pit pad sits 0.54 m above the water plane; the chunk's splash Kismet may fire on arrival | `lab_screenshot` on arrival at the pit pad | #1234 |
| 34 | After #1234: world 73's platforms are inert scenery | World 73 shows the eight platforms but nothing on them works. `GLB-Global` now loads on demand there | `lab_screenshot` in world 73; no console to click; `client_events_read` with no errors | #1234 |
