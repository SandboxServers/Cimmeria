# Colo playtest 2026-09-18 — Castle_CellBlock + Castle (reconstruction and findings)

First external playtest of the autonomous Castle_CellBlock / Castle (701-708) rebuild, run by Lomiada on the colo
server immediately after Watchtower deployed image `6c01b5089781`. This document aligns the Discord conversation
with server telemetry (SigNoz), states what the logs confirm, contradict, or cannot speak to, and ranks the NPC
navigation defects the session exposed.

Status: **investigation only** — no code or seed was changed. Nothing here has been UAT'd.

Appendices (the full evidence, with file:line pointers and per-seam logging specs):

- [appendix-session-timeline.md](appendix-session-timeline.md) — 92-row server timeline, 30-claim table, 17 telemetry gaps
- [appendix-npc-movement.md](appendix-npc-movement.md) — facing, grounding, wire variants, navmesh, movement telemetry
- [appendix-npc-ai.md](appendix-npc-ai.md) — aggro, leash, follow/escort, cover, second-leg move orders, AI telemetry

## 1. Time base and sources

- Discord timestamps are **CDT (UTC-5)** — the owner's machine reports Central with DST active. The Watchtower post
  at 6:49 PM CDT matches the server process start at **23:49:37 UTC** (16 static spaces created), so the offset is
  verified, not assumed.
- Telemetry: SigNoz, `service.name = cimmeria-server`, window 23:30 → 02:45 UTC (2026-09-18/19). ~110k log rows.
- Player activity: 23:58:43 → 01:37:45 UTC (6:58 PM → 8:37 PM CDT). Account 6, GM `access_level` 2, one client.
  - **Char 71** `esfwswf` — Human Soldier (archetype 1), cell entity 2. 6:59 PM → 8:05 PM CDT.
  - **Char 72** `jaffa` — Jaffa (archetype 8), cell entity 3. 8:05 PM → 8:37 PM CDT (dropped; no logOff, no reason logged).
- **We do not have the tester's client logs.** No `launcher.*` scope and no dev-session traffic exist for this
  session ("you have my logs right?" at 8:24 PM — no). Everything below is server-side only.

## 2. Headline findings

| # | Finding | Confidence | Where |
|---|---|---|---|
| H1 | **Backwards facing is a saturating cast.** `pack_angle` does `(radians / SCALE) as u8`; Rust float→int casts saturate, C++ wraps. NPC yaw is `dx.atan2(dz)` ∈ (-π, π], so every heading with `dx < 0` is sent as byte 0 = due north. Half the compass collapses. | Verified in code; **corroborated by the 7:03 PM screenshot + logs (§8)** | `crates/services/src/mercury/aoi/mod.rs:47-50`, `cell/service/ticks/npc_movement.rs:103,110,156` |
| H2 | **NPCs float because we only ever send the FullPos avatar-update variant (0x10)**, which tells the client to trust our Y verbatim. The OnGround variant (0x18) is byte-identical on the wire and makes the client ray-cast Y itself. Server-side, Y is a linear lerp between path corners; `get_navmesh_height` exists with zero production callers. | Verified in code + docs; 0x18 behaviour needs one in-game UAT | `mercury/aoi/mod.rs:38`, `npc_movement.rs:152`, `space_manager/spatial.rs:71-76`, `docs/drafts/spec/position-updates.md:119-127` |
| H3 | **Castle has no navmesh.** Startup logs `No navmesh for space (optional)` for Castle at **DEBUG**; `data/spaces/castle.nav` does not exist. `find_path` returns `None`, and follow / min-range-backup / investigate all fall back to a **raw straight line in all three axes**. That is "came thru walls and floors" and "levitating". | Confirmed in telemetry + code | `npc_ai/follow.rs:99-111`, `fight.rs:451-452`, `investigate.rs:132`, `space_manager/lifecycle.rs:36-39` |
| H4 | **Why the *second* leg is where it breaks: three leg-boundary artifacts.** (a) Detour's intermediate corners are grid-quantized in Y while the final corner is the true projected surface, and the tick snaps to each — a **measured 0.15–0.18 u Y sawtooth at every leg boundary, on flat floors**. (b) Velocity is zeroed at the last waypoint and the next leg arrives a tick later, so the client halts and jerks. (c) Leg 2+ re-paths toward the player's new position, so the bearing scatters and crosses into H1's broken half of the compass. | (a) measured in telemetry; (b), (c) verified in code | `npc_movement.rs:79,106,110,152` |
| H4b | **An attacking NPC can never turn.** `npc.direction` is written only by the movement tick, which skips NPCs with an empty `nav_path` — and attack-in-place clears `nav_path`. Yaw freezes at the moment the NPC stops; the player strafes; the NPC cannot re-face. `attack_in_place` was 123 of 165 logged decisions. No server-side arc gate exists, so if attacks stop when facing away, the gate is client-side. | Verified in code | `npc_movement.rs:41-47,121,187`, `fight.rs:473` |
| H4c | **Latent: a degenerate repath keeps the stale path.** `find_path` returning `Some(path)` with `len <= 1` hits neither branch of the chase repath — no log, no update. Probably rare (Detour normally returns start + end), and unprovable tonight because the branch is silent. `min_range_backup` is a second latent landmine: a raw single waypoint walking *away* from the target with Y taken from the target (0 firings tonight). | Verified in code; not shown to have fired | `npc_ai/fight.rs:397-416,441-470` |
| H5 | **Proximity aggro is structurally dead and there is no assist aggro.** `aggression` is absent from the spawnlist and template seed → 0 everywhere → every Idle NPC is filtered out of the AI tick. All 50 aggros tonight were the player shooting first. | Confirmed in telemetry + seed | `npc_ai/dispatch.rs:59,105`, `combat/threat/aggro.rs:50-102`, `db/resources/Worlds/Seed/spawnlist.sql` |
| H6 | **Leash never fired, measures the wrong distance, and would desync the client if it did.** The test is spawn→*target* (not spawn→NPC); the handler teleports with a raw field write, never restores `spawn_dir`, and sends no position packet. Fight→Idle also leaves the NPC parked at chase-end position and facing. | Verified in code; zero `leash` rows in any scope | `fight.rs:177-212`, `npc_ai/leash.rs:12-90`, reference impl `ticks/npc_respawn/mod.rs:286-320` |
| H7 | **Escorts die silently.** Marsh's follow target *was* set (chain 1174, both characters) and then produced zero AI events for the whole session. `follow.rs` has four silent early-returns, one of which permanently clears the target; an Idle NPC with `aggression == 0` is never ticked again. Zerutska only works because chain 1302 re-armed follow 10× in 65 s. Coppleman has no `set_follow_target` chain at all. | Confirmed in telemetry; exact silent branch unknown (it is silent) | `npc_ai/follow.rs:42-84` |
| H8 | **Respawn breaks the client's region reporting for the rest of the session.** After `ReanchorPlayer` at 7:37:25 PM the client sent zero region hints for 28 minutes (it kept interacting and killing). That is why the Throne Room "doesn't recognise me" on char 71 but worked first try on the Jaffa, who never died. Likely also "mission log broken". | Strong correlation in telemetry; mechanism inferred from code | `base/world_entry/reanchor_player.rs` vs `cell/service/base_messages/player_init` |
| H9 | **Take-cover objective is an edge-trigger race**, reproduced identically on both characters: the vial sits on the cover desk, so the cover-enter edge fires ~1 s *before* the step activates and never re-fires until the player leaves and re-enters. No crouch check exists. | Confirmed in telemetry | mission 639 / objective 2484, `cell/service/ticks/cover.rs`, chain 1133 |
| H10 | **End state is a server gap.** After the DHD repair the DHD advertises `INT_DHD` (16) but the interact dispatcher has no arm for it (76 dead clicks), and dialog-set 3073 resolves to `dialog_id = None`. | Confirmed in telemetry + code | `cell/interactions/dispatch/interact.rs:208`, `castle_706_708_chains.sql:221-232` |
| H11 | **The AI layer is close to unobservable.** `decision_outcome` is split: `fight.rs` writes it as a log field (no counter); every other handler writes span + counter (no log). The movement "1-in-10 step sample" actually samples `npc_id % 10 == 0` — 10% of *NPCs*, forever; all 40 step rows tonight came from two ids. `setMovementType` sends and UPDATE_AVATAR sends are entirely unlogged. | Verified in code | `npc_ai/mod.rs:90`, `npc_movement.rs:171`, `abilities/messaging.rs:201-249` |

## 3. Reconstructed session — Discord aligned with server events

Verdicts: **CONFIRMED** (logs agree) · **CONTRADICTED** · **PARTIAL** · **NO-DATA** (nothing in telemetry can speak to it).

### Pre-session (6:34–6:49 PM CDT)

| CDT | Discord | Server evidence | Verdict |
|---|---|---|---|
| 6:34–6:42 | Plan: deploy untested build; delete the default character and exercise creation; boot + minigame "supposed to be a thing"; follow "allegedly in there". | — | — |
| 6:49 | Watchtower: `7fb07a87b76d → 6c01b5089781` | 23:49:37 UTC process start; cover loader 1,381 sets / 9,353 nodes; navmesh loaded for Castle_CellBlock (1,479 polys), Harset, Agnos; **Castle: none** | CONFIRMED |

### Character 71 — Human Soldier, Castle_CellBlock (6:58–7:21 PM)

| CDT | Discord | Server evidence | Verdict |
|---|---|---|---|
| 6:58 | (plan: delete default char) | Login; char list count=1 (player 66). **No delete message ever reaches the server**; count goes 1→2→3. Two `createCharacter` rejects: surname `"will "` has trailing whitespace. Created as player 71 at 6:59:09. | CONTRADICTED (delete) / CONFIRMED (create) |
| 7:01 | "boot is on but no minigame, can just walk" | Chain 1022 accepts hidden mission 689; chain 1023 launches ability 1597 — documented in the seed as a complete no-op. Minigame is only reachable via `item_use 3438`, which the client never sends. 689 stays active forever on both characters. | CONFIRMED — known-unverified seed assumption, now falsified by UAT |
| 7:03 | "he came from his cover and then... moonwalking in the air" | 7:03:14 region8 → chain 1008 generates threat on `ArmYourself_NIDGuard`; first kill 7:03:54. **No NPC used cover all session** (zero `move_to_cover`/`stay_in_cover`; all five `fire_cover_entered` rows are the *player*). The guard left its spawn pose and chased. The guard's id is not ≡ 0 mod 10, so it has **no step telemetry at all**. CellBlock has a navmesh, so this is H1 + H2 + the unlogged `setMovementType` decoupling, not H3. | PARTIAL — cover half CONTRADICTED; moonwalk half now **CONFIRMED as H1** by screenshot + positions (§8); the guard is visibly airborne on a ramp (H2/H4) |
| 7:07 | "take cover behind the desk isn't registered... hold up it's completed now but I don't know why" | 7:05:45.79 `fire_cover_entered: no chains matched` → 7:05:46.97 step 2144 activates → 7:05:58 drone dead → 7:06:04 uses Ambernol, `no chains matched` ("can't progress") → 7:07:15.79 cover-enter `matched` → chain 1133. | CONFIRMED — H9 |
| 7:07 | "the marsh dialog windows are still wrong" | Dialogs 4001/4000/3999/3998 served; the server logs only `dialog_id`. | NO-DATA — screenshot needed |
| 7:11 | Marsh "still just standing there... doesn't follow but he did came after me" | 7:11:43 chain 1174 `set follow target` on `Preparation_ColMarsh` (npc 100122), `resolved_target Some(2)`. **Then nothing**: that NPC's entire session log is one spawn row + this row. | CONFIRMED — H7. "He did come after me" is unexplained (no movement rows; he is outside the step sample) |
| 7:11 | "also no mess hall dialog" | 680→681 at 7:11:45 has no dialog action. Seed deliberately excludes dialog 5019 (its last screen is out-of-scope Future Self content). | CONFIRMED — by design; owner decision |
| 7:13 | "enemies walk straight to me facing backwards" | `chase` decisions present; yaw is never logged. | NO-DATA per-incident; H1 verified in code (H4 may contribute) |
| 7:13 | "other guard didn't aggro" | MessHall_Guard2 died 7:12:58, Guard1 7:13:33 — sequentially, each aggroed only when shot. Zero auto-aggro events all session. | CONFIRMED — H5 |
| 7:13–7:14 | Flank is optional | Objectives 2725 / 2731 never completed; zero rows containing "flank". The trigger requires a guard to hold a cover slot, and no NPC ever took cover. | CONFIRMED never fired; evaluation itself is unlogged |
| 7:15 | "forced camera came up, dialog went away too fast; only 'find a way out without Marsh' stays" | 7:15:44 matinee; dialog 2516 at 7:15:54.89, dialog 5859 at 7:15:55.49 — 0.6 s later, replacing it. Seed has an intentional ~500 ms gap. | CONFIRMED — seed timing |
| 7:17 | "no remains like blood" | — | NO-DATA (map content) |
| 7:18 | Soldier got stealth armor | 7:17:09 chain 1098: Covert Stealth ×5 + Combat Knife for every non-Jaffa archetype. Explicit `add_item`, not a loot table. | CONFIRMED — only two crate variants authored |
| 7:19 | "lvl 2 now with animation" | Level 2 at 7:19:10 (total_xp 110). | CONFIRMED |
| 7:20 | "killed all nid guards in armory (one) didn't tick" | 7:20:15 death dispatched, `armory_kills = 1`; nothing completes objective 4647 (seed calls the counter cosmetic). | CONFIRMED — one-line seed fix |
| 7:23 | Ring transport animation question | 7:21:11 cross-world teleport → Castle entry 7:21:14, 16 missions reloaded, no errors. | CONFIRMED clean (animation itself NO-DATA) |

### Character 71 — Castle (7:21–8:05 PM)

| CDT | Discord | Server evidence | Verdict |
|---|---|---|---|
| 7:23–7:24 | "copple man isn't moving"; "she had the finish dialog... now I can progress" | 701: Coppleman dialog 7:22:47 → Livewire → step 2421 advanced by a **timer** (not arrival) → completed 7:24:02. **No `set_follow_target` exists for Coppleman** — zero rows. | CONFIRMED — missing chain, not a follow bug |
| 7:25 | Too many slap-packs | Item 2893 on 26/26 lootable kills (`probability = 1`). | CONFIRMED — deliberate |
| 7:26 | Screenshots: "that didn't work", "quest path not yet right" | 7:24:38 slap-pack used; 7:26:16 InterrogationBlock entered. | NO-DATA — screenshots needed |
| 7:26–7:29 | "zuritska is following me... levitating but following... ah he came down... came thru walls and floors" | 7:27:29 chain 1263 arms follow on `Castle_Zuritska_Cell` (npc 100112); chain 1302 re-arms 10× by 7:28:37. **54 `follow_routed` : 54 `waypoint reached`** — every leg was a single straight-line waypoint (no navmesh). Logged leg distances 49.7 / 69.8 / 94.8 against a 5-unit follow band. | CONFIRMED — H3 (through geometry); levitation is the follower **copying the player's airborne Y** (§8), rendered literally because of H2 |
| 7:31 | — | `.speed 300` (GM). Follower default speed is 6.0 u/s vs player 8.125 u/s *before* the speed-up, so the escort can never close the gap. | — |
| 7:33 | "another zerutska was standing at the communication terminal room; I entered the region and the follower vanished" | Two seeded rows live from boot: 100112 `Castle_Zuritska_Cell` and 100113 `Castle_Zuritska_Comms`. 7:33:03.269 chain 1291 (`Castle.CommsRoom` enter) fires `set follow target` with `resolved_target None` = documented *clear*. The escort did not despawn; it stopped 50–95 units behind, out of sight. | CONFIRMED — content design gap |
| 7:36 | "guess who's back"; boot on Zerutska | Player walked back into AoI range of the stalled escort. | CONFIRMED (inferred from position history) |
| 7:37 | "enemies also respawned"; "I died" | 19 deaths → 19 respawns, all Castle, `respawn_secs 120`, measured 120.2–120.9 s, position snapped + state cleared. Player killed 7:37:11 by NPC 100034 (ability 592). | CONFIRMED — respawn path is healthy |
| 7:37–7:39 | "I respawned, heart still beating... take damage, heartbeat is gone"; "every respawn point available, even armory" | `onBeginAidWait respawner_count = 4` (all Castle rows; filter is `world_name` only, no discovery state). `callForAid respawner_id = 3`. Reanchor replays appearance + tint + inventory only. | CONFIRMED (list) / NO-DATA (audio) — see H8 |
| 7:40 | `gotoxyz 320.64 66.79 1042.59` | 7:40:17 `.gotoxyz 320.64 66.97 1042.95` accepted (digits transposed, ~0.4 u off, harmless). | CONFIRMED |
| 7:41–7:47 | Romney behind a sealed wall; hole in floor; mirrored wing | Spawn 240 at (320.64, 66.79, 1042.59), template 169. **But the Jaffa killed Romney 24 s after entering the block at 8:20:14 with no teleport** — so either the room is reachable, he was shot through geometry, or he moved. | **CONFIRMED sealed** — screenshots + seed + the Jaffa's flight path (§8.5); he ghosted through the wall |
| 7:47 | "mission log broken" | No server-side anomaly; mission state persisted correctly throughout. Occurs 10 min after the reanchor. | NO-DATA on the client view — suspect H8 |
| 7:48–7:49 | "romney dead but no mission complete"; "I don't know how I got the data crystal"; "romney mission now gone" | 7:48:04 Romney dead → chain 1272 grants item 2135 *Romney's NID Badge* and completes 703 — silently (reward_xp 0, no dialog). The Data Crystal (5029) came from the comms terminal at 7:34:13. Item 2135 has an `IconMissing` icon and a copy-pasted Ambernol description. | PARTIAL — server state correct; presentation is what failed |
| 7:55 | "I am at throne room now, it doesn't recognise me" | 706 on step 2411 (client-hinted region). **Zero region hints from 7:36:29 PM to logoff.** | CONFIRMED — H8 |
| 7:57–8:04 | "no .mission commands in?"; `.searchmission` | 20 dot-commands, 12 accepted. `.missionadvance`, `.advance_step`, `.mission advance`, `.speed400`, bare `.searchmission` rejected **with no server log**. Accepted commands do not log their output. | PARTIAL |

### Character 72 — Jaffa replay (8:05–8:37 PM)

| CDT | Discord | Server evidence | Verdict |
|---|---|---|---|
| 8:04 | "jaffa had the boot in main menu after creation" | `Sending character visuals` component_count = 14 (10 body + 4 items); component names not logged on that line. | PARTIAL — consistent with force-equipped item 3438 |
| 8:04 | "display more information... nothing happens" | 28 `fire_dialog_choice: no chains matched` rows carry no button id. | NO-DATA |
| 8:10 | Cover objective "just completes after some time... should when crouching" | Identical race: no-match 8:08:34.79, step active 8:08:35.68, matched 8:09:30.79. | CONFIRMED — H9 |
| 8:10 | "marsh last dialog should play before ring transport is unlocked" | Chain 1061 fires dialog 3998 + complete 641 + accept 680 in one action list; ring switch usable immediately. | CONFIRMED (ordering) |
| 8:10 | "marsh really doesn't want to follow" | 8:11:36 chain 1174 `resolved_target Some(3)` on npc 100150; zero AI events afterwards. Unlike the first run's Marsh (100122), id 100150 *is* inside the `npc_id % 10` step sample and still logged zero steps — he genuinely never moved. | CONFIRMED — H7 |
| 8:10 | "jaffa got the staff but armor isn't right" | Chain 1099: Armored Prison Jacket + Serpent Staff only. | CONFIRMED |
| 8:17 | Radio function missing; Moh'katan dialog plays under Gerschon's name | Dialog 5862 fires from `dialog_choice 5861`, pinned to the interact target (Gerschon) because the executor needs a context entity. | CONFIRMED — by design; no remote-speaker presentation exists |
| 8:17 | "this time I got the throne room" | 8:21:55 `Castle.ThroneRoom` matched → chain 1321. This character never died. | CONFIRMED — corroborates H8 |
| 8:17 | "missions don't give xp" | 47 XP grants all session, all kill XP, flat 10 each. `reward_xp = 0` on all 22 missions touched. | CONFIRMED |
| 8:24 | "killed an officer that cleared the mission, I got the crystal" | 8:25:43 `Castle_BravoOfficer1` dead → chain 1346 explicit grant 2790, no dialog. | CONFIRMED — by design |
| 8:24 | "he isn't really dead, also nidguard with quest icon... clicked the guard again, dead now" | Nothing in the AI layer explains it. Candidates: the 120 s respawn standing a corpse back up (respawns at 8:26:48 / 8:27:13 / 8:27:28), or a DoT-pulse kill that bypasses the death path (`effects/pulsing/tick.rs::fire_pulse` has no alive→dead detection). | NO-DATA — needs upright-vs-prone from the tester |
| 8:24 | "fix dhd should have the crystal minigame" | All 12 minigames were Livewire (11 wins, 1 abort at the DHD). | CONFIRMED |
| 8:24 | "can't dial — end state" | 8:28:16 step 4462; then 76 clicks on the DHD → `target has no static interaction type`; `add_dialog_set 3073` → `dialog_id None`; 8:28:30 "stargate region entered with no gate dialled". | CONFIRMED — H10 |
| 8:34 | "1521 Symbiote isn't in" | No chain references 1521 (deferred CA15 / D-CA14). | CONFIRMED |
| 8:37 | — | `DisconnectEntity` for player 72 with no logOff and no reason. | — |

## 4. NPC navigation — what is actually wrong

The owner's first-hand description, which the session corroborates: the first straight-line move usually looks fine;
on the *next* move the NPC leaves the ground or walks directly backwards while facing that backwards direction;
grounding errors occur on stairs/ramps **and** flat floors; NPCs that end up facing away stop attacking but keep
threat; leashed NPCs show the same defects.

These are five independent defects that overlap visually.

### 4.1 Facing (H1) — one-line fix

`pack_angle` saturates negative yaws to 0. Any NPC travelling with `dx < 0` renders facing north; one heading
south-west toward the player renders walking backwards. Two further paths to "faces the wrong way":

- `npc_movement.rs:106` sets `yaw = 0.0` on a zero-length hop (coincident waypoints).
- **Nothing in the AI ever sets facing (H4b).** `npc.direction` is only written as a side effect of translation
  (`npc_movement.rs:121,187`), and the tick skips NPCs with an empty `nav_path` — which is exactly what
  attack-in-place produces (`fight.rs:473`). There is no face-target step and no turn-in-place, so an attacking NPC's
  yaw is frozen while the player strafes around it. There is *no* server-side facing gate on NPC attacks, so the
  owner's "faces away, stops attacking, keeps threat" is either a client-side arc gate fed by our frozen yaw, or the
  silent `needs_repath == false` return at `fight.rs:436` — both unlogged today. A yaw-only update costs nothing
  new on the wire: direction bytes are already in every `0x10`.
- Players have a separate facing bug through the same function (packed bytes stored as f32 inbound), already tracked
  as P49 in `docs/analysis/legacy-command-parity/work-packets.md`. NPCs store radians; players store bytes. Fix both
  in one pass without regressing either.

Fix: `(radians.rem_euclid(TAU) / SCALE) as u8`. The regression guard **must use a negative angle** — a `[0, π]`
fixture passes with the bug present.

### 4.2 Grounding (H2) — one byte on the wire, pending UAT

Per the position-updates spec, FullPos / OnChunk / OnGround carry identical bytes; only the client handler differs,
and OnGround (`FUN_00ddb830`) discards wire Y and terrain-ray-casts. UE3 collision is disabled on `ABigWorldEntity`
and neither avatar filter does a ground query, so the client cannot correct a bad Y under FullPos. Switching NPC
broadcasts from `0x10` to `0x18` hands grounding to the client in every world, including meshless ones.

Unknowns: whether `0x18` grounds or sinks in practice (never exercised by us), and the meaning of the hardcoded
`physics = 0x01` byte (`mercury/aoi/update.rs:43`) — the PHYS_* table is undocumented. Keep players on FullPos.

Server-side truth is a separate, lower-priority fix: wire `get_navmesh_height` into the movement tick and snap
spawn Y at spawn time. Stationary NPCs are never touched by the tick, so an authored (model-origin) Y is broadcast
forever until the first path lerps it down — "levitating, then he came down".

Log proof of the lerp: NPC 100170 climbs `cur_y` 24.670 → 24.800 toward `wp_y`, then holds a bit-exact 24.800
across ~10 units of XZ travel (01:13:57.892 → 01:13:58.993 UTC).

### 4.3 Through geometry (H3) — generate `castle.nav`, and stop failing silent

`find_path`, `has_line_of_sight` and `is_position_valid` all **fail open** with no mesh, and the load miss is DEBUG
"optional". Fixes in cost order: promote the miss to WARN; log every straight-line fallback; clamp fallback Y to the
NPC's current Y so a blind follower cannot climb; generate `castle.nav` (`crates/navmesh-extractor/` exists).
The tester being a GM compounds this: 26 `navmesh_gm_bypass` events mean the chase/follow *target* was often
off-mesh even where a mesh exists.

### 4.4 Second leg (H4) — leg-boundary artifacts, and one move-order choke point

The two investigations disagree on emphasis here, and the disagreement is worth keeping visible.

**Measured (movement appendix A0).** `waypoint_reached` telemetry shows intermediate waypoints on a quantized grid
and final waypoints on the true surface — NPC 100138: intermediates `34.6, 34.6, 34.6, 34.6`, final `34.779`;
NPC 100137: `34.6` → `34.751`. The tick snaps to each waypoint and lerps between them, so the NPC rides ~0.15–0.18 u
below the floor for a leg, snaps up at the end, and drops again when the next leg's quantized corners arrive. The
code comment at `npc_movement.rs:149-151` ("waypoints from findStraightPath are on the navmesh surface") is wrong:
poly mesh ≠ detail mesh. Stack on that the zero-velocity halt between legs and the bearing scatter feeding H1, and
"first leg fine, second leg goes up / turns backwards" needs no separate path-replacement bug. The movement
investigation explicitly checked and ruled out: stale start position, stale segment index, yaw from the previous
segment, mid-path replacement, and a different code path for leg 1. Start-poly lookup failure after Y drift is a
*latent* risk (±0.5 vertical search box vs 0.18 measured drift) but logged zero times.

**Latent (AI appendix F).** `nav_path` is written by seven sites with three inconsistent policies
(clear-then-push, leave-stale, never-touch). The chase repath silently keeps a stale path when the new path is
degenerate; the follow fallback includes Y and ends leg 1 off-mesh so leg 2's `find_path` starts from an invalid
position (this one *is* live in Castle). Proposed fix regardless of which mechanism dominates: a single
`issue_move_order(npc_id, dest, move_reason)` helper that always clears, always logs, and stamps a per-entity
`leg_seq` — which also gives the telemetry its correlator (§6).

**Unresolved, and the most useful thing the next session can tell us:** when an NPC "walks backwards", is its
*translation* wrong (moving away from where it should go) or only its *facing* (moving correctly, body reversed)?
H1 explains only the latter. `yaw_byte` + `leg_start` telemetry settles it in one query.

### 4.5 Animation vs translation (moonwalk)

The client selects mob animation from the `setMovementType` byte (client FSM `FUN_00deb660`), not from velocity.
`npc_movement_tick` never broadcasts it, `broadcast_movement_type` dedups identical kinds, and `kind = None` clears
the cache without sending. An NPC can translate while the client still plays a stationary pose. Already documented
as a gap in `docs/reverse-engineering/findings/npc-movement-pathfinding.md:150-158`. Mechanism confirmed; that it
is what happened at 7:03 PM is inference, because these sends are unlogged.

### 4.6 Leash and disengage (H6)

- Predicate measures spawn→target; should be spawn→NPC. `LEASH_DISTANCE = 50.0` is global.
- Handler teleports via raw `npc.position =` (bypassing `update_entity_position`, so the AoI grid desyncs), never
  zeroes velocity, never restores `spawn_dir`, sends no `EntityMoved`. The client's avatar filter extrapolates along
  the stale chase velocity and lerps across the snap gap — which is what reads as "walking home off the geometry,
  facing the wrong way". The respawn tick does all of this correctly and is the reference.
- The leash handler's log line has no `target:` and no fields; give it both.
- `Leashing` is missing from `generate_threat`'s preemption list: damage during the ≤2 s window is discarded.
- No threat decay, no out-of-AoI disengage. A mob you run from stays `Fighting` forever where the chase ended.
- Follow never resumes after combat (`Follow` is preemptable; every exit lands in `Idle`).
- The legacy Python `SGWMob.py` has no leash state at all — `AiState::Leashing` is a Rust addition, so the
  walk-home design is ours to choose (owner preference: server-authoritative, client-non-breaking).

### 4.7 Recommended order

| # | Change | Cost | Notes |
|---|---|---|---|
| 1 | `pack_angle` wrap + negative-angle guard | 1 line + test | No client risk |
| 2 | Missing-navmesh → WARN; log straight-line fallbacks; fix the step sampler | small | Makes the next session self-diagnosing |
| 3 | Unify `decision_outcome` (log + span + counter from one helper); log the silent `follow.rs` / `fight.rs:436` branches | small | H11 |
| 4 | NPC broadcasts → OnGround `0x18` | 1 byte + UAT | Needs an in-game look before merge |
| 5 | Yaw-only re-face for attacking / path-less NPCs with a combat target | small | H4b — same `0x10`, no new wire traffic |
| 5b | `issue_move_order` choke point (fixes the stale path, stamps `leg_seq`); keep velocity across leg boundaries; stop snapping Y to quantized corners | medium | H4 / H4c — item 4 masks the Y sawtooth client-side |
| 6 | Leash: spawn→NPC predicate, reuse respawn snap block (or walk home), restore `spawn_dir`, add `Leashing` to preemption list | medium | H6 |
| 7 | Follow robustness: survive transient target miss, resume after combat, per-template `move_speed` | medium | H7 |
| 8 | Generate `castle.nav` | depends on extractor | Real fix for H3 |
| 9 | Couple `setMovementType` to path start/stop; add a face-target step | medium | Needs an enum-per-state decision |
| 10 | `aggression` column + seed values; optional social-aggro radius | schema + seed | H5 — behaviour change, owner sign-off |

## 5. Other defects surfaced

- **Reanchor does not resend regions / missions / abilities / stats** (H8). Highest-impact non-NPC bug of the night.
- **Take-cover race** (H9): re-evaluate cover chains for players already inside a set when a step activates, and/or
  gate on crouch (owner decision).
- **DHD `INT_DHD` has no interact arm; dialog-set 3073 inert** (H10).
- **Respawner list is unfiltered** — every Castle respawner offered; no discovery state exists.
- **Speed validator divides by inter-packet wall-clock** (35–60 ms windows; `Inf`/`NaN` reach the metric). 756
  warnings. Window it over game ticks *before* recalibrating; the 8.125 baseline also looks low (ratios cluster
  1.6–2.0). The `.speed` GM command accounts for the 24.375 / 32.5 cohorts.
- **DoT-pulse kills bypass the death path** — no `BSF_DEAD`, no loot, no kill credit. Candidate for "isn't really dead".
- **Console**: rejected dot-commands leave no server log; accepted ones don't log results.
- **Content polish**: dialog 5859 replaces 2516 after 0.6 s; item 2135 has a missing icon and wrong description;
  objective 4647 never completes; mission XP is 0 everywhere and kill XP is a flat 10; two Zerutskas need a
  suppress/despawn action; Coppleman has no follow chain; Romney placement; `cover_set_id` is always 1381 (equal to
  the set count — possible last-set fallback in the lookup).
- **Character delete** never reached the server — unknown whether the tester pressed it.

## 6. What we could not verify, and the seams that would fix that

Full specs (target, level, fields, function) are in the appendices. The owner's requirement — *where the NPC is,
where it is going, where it is facing, what it is following* — reduces to three joined events:

| Event | Emitted | Key fields |
|---|---|---|
| `npc_ai` `event="decision"` — **every NPC, every AI tick, no silent returns** | `record_decision_outcome` (`npc_ai/mod.rs:90`), called by *all* handlers including `fight.rs` | `npc_id, npc_name, ai_state, decision_outcome, npc_x/y/z, npc_yaw, target_id, target_x/y/z, dist_to_target, dist_to_spawn, has_los, in_range, nav_path_len, leg_seq, move_reason, follow_target_id` |
| `npc_ai` `event="move_order"` — once per leg | new `issue_move_order` helper | `leg_seq, move_reason, from_*, to_*, path_len, routed, straight_line_fallback, prior_path_cleared, start_on_navmesh, dest_on_navmesh, move_speed` |
| `movement.npc` `step` / `waypoint_reached` | `ticks/npc_movement.rs` | add `leg_seq, move_reason, npc_name, yaw_rad, yaw_byte, y_source, ground_y, y_offset_from_ground`; sample per-leg (first N steps of every leg), not by `npc_id` |

The two appendices name the correlator differently (`leg_seq` in the AI spec, `leg_id` plus `leg_start` / `leg_end`
events in the movement spec). They describe the same thing; pick one name when implementing.

New `decision_outcome` values needed: `hold_no_repath`, `repath_degenerate`, `follow_no_path`, `follow_target_lost`,
`follow_dropped_no_target`, `auto_aggro_none`, `idle_not_ticked`, `leash_snap`, `fight_idle_reset`, `cover_skipped`.

Other seams, by the claim they would have settled:

| Unverifiable tonight | Seam |
|---|---|
| Which way was the NPC facing? | `yaw_byte` on movement events; sampled UPDATE_AVATAR send log (`entity_id, witness_id, pos, yaw_byte, physics_byte`) — these sends never reach `wire.out` |
| Was it playing the wrong animation? | `movement.movement_type` on every `broadcast_movement_type` outcome: `sent / deduped / cleared / rejected_player` |
| Why did this NPC behave that way? | Spawn-time behaviour dump: `template_id, aggression, use_cover, is_stationary, move_speed, respawn_secs, follow_min/max` |
| Does this world have a navmesh? | One INFO line per space at startup: `world_name, navmesh_loaded, poly_count`; WARN on miss |
| Why did a chain not match? | `condition_failed` debug per candidate chain: `chain_id, condition_type, expected, actual` |
| Did the client lose its regions? | Server-side containment WARN `client_region_hint_missing`; reanchor log listing `resent` / `not_resent` |
| Heartbeat after respawn | `respawn: player revived` with `state_flags_before/after`, `dead_flag_cleared`, health sent |
| Which respawners were offered? | `respawner_ids`, `filter` on the existing `onBeginAidWait` row |
| Player death | First-class `player death` INFO (today only discoverable via `Target killed!` with `is_npc = false`) |
| Dialog presentation / "more information" button | `speaker, screen_count, replaced_dialog_id, ms_since_previous`; WARN `dialog_displaced` under 3 s; `button_id` on dialog choice |
| Flank evaluation | Cover slot reserve/release + flank check result |
| Console results | Reject reason; result summary per accepted command |
| DHD dead clicks | WARN `unhandled_interaction_flag` when a target advertises flags the dispatcher has no arm for |
| Client intent in `wire.in` | Resolve `method_name` for entity methods (all rows are `unknown` today) |
| Disconnect reason | `disconnect_reason` on `Destroying Player entity` |
| Anything client-side | Operational: get testers onto `sgw-launcher` with telemetry on; add a liveness panel |

## 7. Open questions

Owner:

1. Was `castle.nav` ever generated? Is its absence deliberate (extractor failure, geometry size)?
2. When an NPC "walks backwards": is it actually *travelling* the wrong way (away from where it should go), or
   travelling correctly with its body reversed? H1 explains only the second.
3. Does the moonwalk reproduce in Harset or Agnos (both meshed)? That separates the animation decoupling from the
   missing mesh with no instrumentation.
4. Chain 1302 re-arming follow 10× in 65 s — intentional keep-alive or trigger misfire? It is currently the only
   reason Zerutska follows, so "fixing" it would regress the one working escort.
5. Should chain 1291 clear follow at the comms room? If yes it must also suppress `Castle_Zuritska_Comms` or
   teleport the escort ("walk to the console and stay" is Lomiada's ask).
6. Decisions: leash as walk-home vs snap; `aggression` defaults + social aggro; crouch requirement for take-cover;
   dialog 5019; per-archetype crate loot; mission XP; respawner discovery; boot-lock movement gate; 1521 / CA15.

Tester:

1. Did you press Delete on the default character? Did the client show an error for the surname with a trailing space?
2. ~~How did you reach Romney?~~ Answered in §8.5: client-side `ghost` (noclip) through the sealed wall. Please confirm.
3. The "not really dead" officer at 8:24 PM: standing upright, or prone but still clickable?
4. After respawning at 7:37 PM: was the mission log empty, stale, or missing entries?
5. When Marsh "did come after me" at ~7:11 PM, had he or a nearby guard taken damage first?
6. Were you in GM fly / off-mesh during the 7:03 and 7:13 PM sightings?
7. Were you launching through `sgw-launcher`? No client telemetry reached us.

Screenshots that would materially help (Discord CDT):

| Time | Why |
|---|---|
| 7:03 PM | Moonwalking guard: is the *body* facing screen-north while translating? Separates H1 from the animation decoupling |
| 7:11 PM | Marsh "did came after me" — the only evidence he moved at all |
| 7:26–7:29 PM | Zerutska levitating — height relative to the floor; any debug HUD readout gives both Y values |
| 7:33 PM | Both Zerutskas in the comms room |
| 7:07 PM | The "wrong" Marsh dialog window |
| 7:26 PM | "that didn't work" / "quest path not yet right" — no server-side clue what these were |
| 7:42–7:43 PM | Romney's sealed room / hole in floor — placement fix |
| 7:55 PM | Mission log at the Throne Room (post-respawn client state) |
| 8:24–8:34 PM | "Not really dead" officer; the end-state DHD |

## 8. Screenshot evidence (supplied by the owner after the first draft)

Four Discord screenshots were checked against telemetry for the same seconds. Images are not committed; filenames
are the owner's local names.

### 8.1 `moonwalk.png` — 7:03 PM CDT, Cellblock Guard

What it shows: a hostile Cellblock Guard in a **full run animation with his back to the player**, feet clearly above
his drop shadow, on or beside a ramp. The player is still in the orange prison jumpsuit, which dates it before the
crate at 7:17 PM.

Telemetry for 00:03:16–00:03:18 UTC:

| Entity | Position (x, y, z) | Notes |
|---|---|---|
| Guard `npc_id 100131` | (-287.8, 68.6, -160.6) → (-287.5, 68.6, -162.1) | `waypoint_reached`, 2 then 1 waypoints remaining; Y is the quantized 68.6 on both |
| Player (entity 2) | (-314.2, 73.47, -190.3) → (-307.8, 71.09, -180.4) | velocity (+3.26, 0, +5.04); Y falling 2.4 u in 2 s — **the player is walking down a ramp toward the guard** |

Guard → player is (Δx, Δz) ≈ (-24, -26). A chase heading toward the player is `atan2(-24, -26) ≈ -2.4 rad` —
negative, so `pack_angle` saturates it to byte 0 = facing +Z. The player is on the guard's -Z side, so **+Z is
directly away from the player**: the guard runs at the player showing his back. That is the screenshot.

Two conclusions:

- **This incident is H1, not the animation decoupling.** The guard is playing a run cycle, so the client *did* have
  a moving animation state; what is wrong is the body yaw. §4.5 remains a real gap but is not what "moonwalk" meant
  here. This also answers the open question in §4.4 for this case: translation correct, facing reversed.
- The first logged corner leg had `dx = +0.3` (yaw positive, transmitted correctly). The facing broke on the
  *following* legs, when the bearing swung to `dx < 0` — the "first leg fine, next leg backwards" pattern,
  produced by bearing rather than by leg number, exactly as the movement appendix argues.

The float: the guard's path Y is pinned at the quantized 68.6 while the scene is a ramp (the player's own Y moves
2.4 u across it). Lerping between quantized corners across a ramp and sending the result as FullPos (H2) puts him
in the air. The shadow-to-feet gap in the image is on the order of half a metre — far more than the 0.15–0.18 u
flat-floor sawtooth, as expected on a slope.

### 8.2 `levitate.png` — ~7:27–7:29 PM CDT, Dr. Zuritska following

What it shows: Zuritska upright, in an idle pose, well above the floor (his drop shadow appears on the floor tiles
left of the player, far below his feet), quest marker overhead, body in profile rather than facing the player.

Telemetry for 00:29:43–00:29:49 UTC:

| UTC | Follower waypoint Y (`npc_id 100112`) | Player Y (entity 2) | Player `vy` |
|---|---|---|---|
| 00:29:43.19 | 69.921 | — | — |
| 00:29:45.15 | — | 71.735 | -0.5 |
| 00:29:45.19 | **70.899** | — | — |
| 00:29:45.82 | — | 70.109 | 0 (grounded) |
| 00:29:46.46 | — | 70.357 | **+7.37** |
| 00:29:47.10 | — | 71.032 | **-5.28** |
| 00:29:47.19 | 70.222 | — | — |
| 00:29:48.99 | 70.127 | — | — |

The floor here is Y ≈ 70.1 (the player's grounded sample). **The tester was jumping**, and the follower's
destination Y tracks the player's *airborne* Y: it was sent to 70.899 — 0.8 u above the floor — because the player
was mid-jump when `follow.rs:88-98` sampled `target_pos.y`. Earlier in the same corridor (z ≈ 1036) the follower's
Y swings 67.10 → 68.51 → 67.24 → 68.25 on consecutive legs, the same signature.

This **replaces** the "authored spawn Y" explanation for "levitating, then he came down" (movement appendix A6):
`other z dude.png` shows Zuritska standing correctly on the floor of his cell, so his authored Y is fine. The
mechanism is: no navmesh in Castle (H3) → straight-line fallback → destination Y copied from a jumping player →
rendered literally (H2) → he "comes down" on the next leg when the player happens to be grounded at sample time.
With a navmesh `find_path` would project the destination onto the mesh and hide this; the robust fix is to never
take a follower's Y from the target — clamp to the NPC's own Y when unrouted, and to the mesh when routed.

Facing: he is in profile to the player with an idle pose, consistent with H4b (yaw frozen at the end of the last
leg) — not provable from one still.

### 8.3 `one z dude.png` and `other z dude.png` — the two Zuritskas

- `one z dude.png`: the Comms twin (`Castle_Zuritska_Comms`, 100113) at the Communications Terminal, grounded,
  **with a mission marker overhead**.
- `other z dude.png`: Zuritska in a cell, grounded, no marker, prison boot visible.
- `levitate.png`: the escort (`Castle_Zuritska_Cell`, 100112) **also carries a mission marker**.

So both rows advertise the same mission interaction at the same time — beyond the visual duplicate, a player could
plausibly turn in at either. The suppress/despawn action proposed in §5 should cover the marker as well as the model.
Both show the prison boot, matching the tester's "so he has the boot too".

### 8.4 What the screenshots changed

| Item | Before | After |
|---|---|---|
| H1 as the cause of "walks at me backwards" | Verified in code, unproven per-incident | Positions + image agree for the 7:03 PM guard |
| Open question: translation wrong, or only facing? | Open | Facing only, for this incident |
| Animation decoupling as the 7:03 PM cause | Inferred | Ruled out for this incident (run cycle is playing) |
| "Levitating then came down" | Authored spawn Y | Follower copies the jumping player's Y; authored Y is fine |
| New seam | — | Log `target_y`, `target_vy` / `target_on_ground` and `dest_y` on every follow `move_order`; a follower whose `dest_y` differs from its own Y with no routed path is the air-climb signature |

### 8.5 The Romney screenshots — `romney.png`, `wall-shouild-be-door.png`, `wall-no-texture.png`, `floor-with-hole.png`

What they show:

- `romney.png`: NID Interrogator Romney inside a cell behind an open doorway numbered **01**, in a cell identical to
  Zuritska's — standing **with his back to the doorway**.
- `wall-shouild-be-door.png`: the corridor outside that cell ends in a solid panelled wall between two pillars where
  the connection to the rest of the block should be. The chat box reads `.gotoxyz 320.64 66.97 1042.95` →
  `gotoxyz: moved entity 2 to (...)` → **`Ghost enabled.`**
- `wall-no-texture.png`: the other end of that corridor is closed by a flat **untextured** surface.
- `floor-with-hole.png`: a gap in the floor beside a grate, open to the void.

An untextured blocking surface plus a hole in the floor is unfinished developer geometry that was walled off, not a
door that failed to open.

Why Romney is in there (seed, `db/resources/Worlds/Seed/spawnlist.sql:353-416`): CA05 recovered 36
`CA-Cell_Doorway01_Pf0` prefab instances across two map tiles and anchored both NPCs on them —

| Row | Tile | Position | Heading |
|---|---|---|---|
| 238 `Castle_Zuritska_Cell` | `Castle-000a0002` | (268.00, 66.79, 1042.59) | 0 |
| 240 `Castle_Romney` | `Castle-000a0003` — "second half of the corridor (interpreted as Interrogation Room 02)" | (320.64, 66.79, 1042.59) | 0 |

Same Y, same Z, X offset by exactly 52.64 — the same cell in the neighbouring tile. The raw map read cannot see
that tile `000a0003` is the sealed, unfinished mirror of `000a0002`; the comment rates the placement HIGH
confidence. **Fix: move spawn 240 to a doorway prefab in tile `Castle-000a0002`** (the accessible wing — the escort's
logged path runs x 240 → 277 along z ≈ 1036), matching Lomiada's "one door after the Dr." Treat every other
reconstructed Castle row anchored in `000a0003`, or in any tile not yet walked in-client, as suspect until a
`.location` walk confirms it; the seed comments already ask for that walk for the Comms room.

How the Jaffa reached him in 24 seconds with no `.gotoxyz` — `movement.player`, entity 3:

| UTC | Position (x, y, z) | Velocity (vx, vy, vz) |
|---|---|---|
| 01:19:56.8 | (297.9, 67.18, 1038.0) | (0, 0, 3.9) — at the sealed wall |
| 01:20:08.1 | (299.3, 66.84, 1035.8) | (**17.5, -4.2**, 0.9) |
| 01:20:08.8 | (308.3, 67.29, 1036.3) | (**17.6, -3.6**, 1.0) |
| 01:20:09.9 | (314.2, 66.99, 1039.1) | (0, 0, 0) |
| 01:20:11.5 | (318.8, 66.99, 1037.1) | (0, 0, 0) — outside Romney's cell; Romney dies 01:20:14 |

A straight 20-unit run in +X through where the wall stands, with a sustained vertical velocity of about -4 while Y
stays level (every on-foot sample in the session has `vy = 0` on flat floor) — that is the UE3 `ghost` fly/noclip
cheat, the same one the first character's chat box shows being enabled. The room **is** sealed; the second run
simply went through the wall. The server saw nothing unusual: Castle has no navmesh, so `is_position_valid` fails
open for everyone there (H3), GM or not.

Two side findings:

- **Every reconstructed Castle row has `heading = 0`**, i.e. facing +Z. The cells sit on the +Z side of the corridor,
  so both Romney and Zuritska spawn facing the back wall — which is exactly `romney.png`. This is the owner's
  "wrong facing while idle at spawn", and it is authoring, not `pack_angle`: seed headings are stored in [0, 2π),
  which the saturating cast handles correctly (it only breaks *negative* yaws from `atan2`). Cell occupants want
  `heading ≈ 3.14159`. The image also independently confirms the axis convention used in §8.1 (yaw 0 = +Z).
- Client-side `ghost` leaves no server trace. With the speed and navmesh validators both warn-only / fail-open, a
  flying player is only inferable from `vy ≠ 0` on level ground. A cheap seam: log `vy` sign-vs-ΔY disagreement, or
  at minimum tag `movement.player` samples with `access_level` so GM noclip can be excluded when reading NPC
  chase/follow data (an NPC pathing to a ghosting target is not a fair test of the pathing).

Correction to §8.2: the tester's airborne Y at 7:29 PM reads as jumping (velocity +7.4 then -5.3 with Y rising then
falling — ballistic), and ghost mode is only evidenced from 7:40 PM onward. Either way the defect is the same: the
follower adopts the target's airborne Y.

## 9. Replacing the Discord chat with telemetry

§6 lists seams per defect. This section asks a different question: **what did the chat give the investigation that
the logs could not, and what would have to be emitted so the next session can be reconstructed with no chat at
all?** The chat supplied six kinds of information.

### 9.1 What the chat contributed

| # | What the chat gave us | Example tonight | Can telemetry replace it? |
|---|---|---|---|
| 1 | **A pointer in time** — "look here, something is wrong" | "moonwalking in the air" at 7:03 PM turned 110k rows into a 3-second window | Yes — tester bookmark (§9.2) |
| 2 | **Non-events** — things that should have happened and did not | "other guard didn't aggro", "marsh doesn't follow", "cover isn't registered", "throne room doesn't recognise me", "can't dial", "no minigame" | Mostly — expectation seams + stall detectors (§9.3) |
| 3 | **What the client rendered** | backwards body, floating feet, dialog replaced too fast, heartbeat audio, "quest path not right" | Partly — log what we *told* the client (§9.4); the rest needs client telemetry (§9.6) |
| 4 | **Tester-side state that contaminates the data** | `Ghost enabled`, `.speed 300`, jumping | Yes — tag it (§9.5) |
| 5 | **Identity and build** | which account is Lomiada, which image was deployed, when | Yes — trivially (§9.5) |
| 6 | **Intent and judgement** | "should complete when crouching", "crystal should be a drop", "too many slap-packs", the work-order at 9:06 PM | No — this is feedback, not telemetry. The bookmark's free text is the right home for it |

Seven of the headline findings (H3, H4, H5, H7, H8, H9, H10) were *visible* in the logs alone; none of them was *found* until the chat
said where to look. Category 1 is therefore worth more than all the others combined, and it is the cheapest.

### 9.2 A tester bookmark: `.bug <free text>` (highest value, smallest change)

A dot-console command (GM `access_level`, same dispatcher as `.location`) that emits **one INFO event, target
`playtest.bookmark`**, carrying a server-side snapshot of everything a screenshot plus a chat line would have told us:

```
player_id, account_id, entity_id, character_name, archetype, level,
world_name, space_id, x, y, z, yaw, region_tags (all regions currently inside),
target_entity_id, target_npc_name, target_tag, target_x/y/z, target_yaw, target_ai_state, target_health,
active_missions = [(mission_id, step_id, [objective_id:status])],
nearby_npcs = [(npc_id, npc_name, tag, ai_state, x, y, z, yaw_rad, yaw_byte, nav_path_len,
                leg_seq, move_reason, target_id, follow_target_id, dist)]   -- within ~40 u, capped at 16
last_dialog_id, ms_since_last_dialog, last_interact_tag, last_interact_outcome,
gm_flags (ghost/fly if known, speed_multiplier), client_addr_hash,
note = "<free text>"
```

Every screenshot tonight would have been answerable from that one row: the guard's `yaw_byte = 0` while his bearing
to the player was negative; Zuritska's Y against the player's Y; both Zuritskas in `nearby_npcs`; Romney's position
against a region list that shows no connected region.

Two extensions that cost little:

- **`.bug` also force-enables verbose capture** for the named target (or all `nearby_npcs`) for the next 60 s —
  un-sampled `movement.npc` steps and per-tick `npc_ai` decisions for just those ids. This sidesteps the sampling
  problem entirely: full fidelity exactly where a human said it matters, nothing elsewhere.
- **Relay it to Discord** through the existing `cimmeria-discord` notifier (new `EventKind`, see the CLAUDE.md row
  for adding one) with a SigNoz deep link for ±60 s around the timestamp. The tester keeps the habit of talking in
  Discord — they just type it in-game — and every remark arrives already aligned to server time. Timestamps in the
  chat tonight were minute-granular; the defects were second-granular.

### 9.3 Non-events: expectation seams and stall detectors

Logs are naturally silent about what did not happen, and six of the tester's complaints were non-events. Two
complementary mechanisms:

**(a) Log the "no" at each decision point** — already specified in §6 and the appendices: `condition_failed` per
candidate chain, `auto_aggro_none`, `idle_not_ticked`, `follow_target_lost`, `hold_no_repath`,
`unhandled_interaction_flag`, `client_region_hint_missing`, `effect_noop`.

**(b) Detect the player being stuck, without knowing why.** These are behavioural signatures that were visible in
tonight's data in hindsight and would have flagged every soft-lock with no human involved. Target
`playtest.friction`, WARN, one event per episode (not per repeat):

| Signal | Rule of thumb | Tonight |
|---|---|---|
| `repeat_interact_no_effect` | ≥5 interacts on the same target in 60 s, all ending in a no-op outcome | 76 clicks on the DHD |
| `repeat_item_use_no_chain` | same item used ≥2× with `no chains matched` while a mission step references it | Ambernol at 7:06 PM |
| `step_stalled` | a mission step active > N min while the player keeps acting inside the step's area | take-cover (77 s), Throne Room (14 min), boot lock (forever) |
| `objective_never_completed` | mission completes with an optional objective untouched, logged at completion | flank 2725 / 2731, armory 4647 |
| `region_dwell_no_hint` | server-side containment says the player is inside a trigger volume, no client hint arrived | Throne Room after respawn |
| `escort_separated` | an NPC with `follow_target_id` is > 3× `follow_max_distance` from its target for > 10 s, or has not moved since being armed | Marsh (never moved), Zuritska (50–95 u behind) |
| `console_reject_streak` | ≥3 rejected dot-commands in 2 min — the tester is hunting for a command that does not exist | `.missionadvance`, `.advance_step`, `.mission advance` |
| `dialog_displaced` | a dialog replaced < 3 s after display | 2516 → 5859 in 0.6 s |
| `death_then_silence` | after a respawn, a previously chatty client message type goes to zero | region hints after 7:37 PM |

### 9.4 What we told the client (the server-side half of "what did it look like")

We cannot see the client's screen, but every visual complaint tonight traced to something the server *sent* and did
not record. Log the outbound intent, not just the internal state:

- `yaw_byte`, position variant (`0x10` / `0x18`), `physics_byte` on (sampled, or bookmark-boosted) UPDATE_AVATAR sends
- every `setMovementType` outcome (`sent` / `deduped` / `cleared`)
- dialog sends with `speaker`, `screen_count`, `replaced_dialog_id`, `ms_since_previous`
- mission-log mutations actually sent (accept / step / complete / remove), with `reward_xp` and `has_rewards`
- on respawn: exactly which state blocks were resent and which were not; state flags before/after
- the respawner id list offered, not just its count

### 9.5 Identity, build and contamination tags

- **`deployment.environment` is wrong on the colo today.** Every row in this session carries
  `deployment.environment = "dev"`, although `docker/compose.yml:72` sets
  `OTEL_RESOURCE_ATTRIBUTES=deployment.environment=colo`. `crates/server/src/otel.rs:176-183` always calls
  `.with_attribute("deployment.environment", deploy_env)` with a `"dev"` default, and an explicit builder attribute
  wins over the SDK's env detector — the comment at `otel.rs:197-202` asserts the opposite precedence.
  (`service.namespace = cimmeria` from the same env var *did* arrive, which is how we know the variable was read.)
  Colo and laptop data are currently indistinguishable by resource attribute. Fix: set `CIMMERIA_DEPLOY_ENV=colo`
  in the compose file, or only add the explicit attribute when `CIMMERIA_DEPLOY_ENV` is set.
- **Build identity.** Emit `service.version` (git SHA + image digest) as a resource attribute and in one startup INFO
  line. Tonight the only record that the build changed at 6:49 PM was a Watchtower post in Discord.
- **A `play_session_id`** minted at `playCharacter` and attached to every event for that player (span attribute or
  explicit field), with a `session.start` / `session.end` pair carrying `character_name`, `archetype`,
  `access_level`, `disconnect_reason`. Today correlation hops between `player_id`, `entity_id`, `account_id`, and
  several fields exist as both string and number types in SigNoz (`player_id`, `item_id`, `dialog_id`, `seq`), which
  silently splits aggregations.
- **Contamination tags** on `movement.player` samples and on any NPC decision whose target is that player:
  `access_level`, `speed_multiplier`, and `airborne` / `vy`-vs-ΔY disagreement (the only server-visible trace of
  client `ghost`). An NPC chasing a noclipping, 3×-speed GM is not evidence about pathing.
- **Names on every NPC row.** `movement.npc` rows carry only `npc_id`; resolving "which one was the guard" needed a
  cross-scope join. Add `npc_name` and `tag` everywhere an `npc_id` appears.

### 9.6 A session journal, and the client

- **`session.journal`** — one low-volume INFO stream per player with a fixed `event` vocabulary: `world_enter`,
  `mission_accept`, `step_advance`, `objective_complete`, `mission_complete`, `dialog_shown`, `minigame_result`,
  `item_grant`, `xp_grant`, `level_up`, `death`, `respawn`, `teleport`, `gm_command`, `bookmark`, `friction`. The
  92-row timeline in the appendix was assembled by hand from ~40 scopes with different field names; it should be
  `scope_name = 'session.journal' AND play_session_id = …`.
- **Client telemetry was absent, and nothing said so.** No `launcher.*` rows exist for this session. Add a
  `session.start` field `client_telemetry = true/false` (did a dev-session token accompany this login?) and a
  friction-style WARN when a GM-level tester plays without it. Facing, grounding, audio and UI layout are only ever
  *provable* from the client side; everything in §9.4 is the server's best proxy.

### 9.7 Order of work

1. `.bug` bookmark with snapshot (§9.2) — replaces the chat's most valuable function outright.
2. Fix `deployment.environment`; add `service.version` and `play_session_id` (§9.5).
3. `session.journal` (§9.6) — makes reconstruction one query.
4. `playtest.friction` detectors (§9.3b), starting with `repeat_interact_no_effect`, `step_stalled`,
   `escort_separated`.
5. The per-defect seams in §6, with bookmark-boosted verbosity replacing id-based sampling.
6. Outbound-intent logging (§9.4), then Discord relay of bookmarks.

### 9.8 What has shipped (PR `feat/playtest-bug-bookmark`)

| Item | Status |
|---|---|
| `.bug <note>` bookmark — header row + one row per nearby entity, joined on `bookmark_id` (§9.2) | **Shipped.** Targets `playtest.bookmark` / `playtest.bookmark.entity`; 60 u radius, nearest 32, selected target always included. The note also reaches the GM Discord channel through the existing console audit relay |
| `yaw_byte` — the facing actually transmitted — on bookmark rows and sampled `movement.npc` steps (§9.4) | **Shipped** |
| Step sampler observes every NPC (global counter, not `npc_id % 10`) (§6) | **Shipped** |
| Missing navmesh → WARN `movement.navmesh` (§4.3) | **Shipped** |
| `follow_no_path`, `follow_target_lost`, `follow_dropped_no_target` (§6) | **Shipped**, with regression guards |
| `repath_degenerate`, `hold_no_repath` (§6) | **Shipped** |
| Colo rows tagged `dev` (§9.5) | **Fixed** — `CIMMERIA_DEPLOY_ENV: "colo"` in `docker/compose.yml` |
| 60 s verbose capture after `.bug`; Discord deep link | Not yet |
| `playtest.friction` detectors (§9.3b) | **Shipped: 4 of 9** — `repeat_interact_no_effect`, `repeat_item_use_no_chain`, `console_reject_streak` (plus a DEBUG row per rejected command with its `reason`), `escort_separated`. Once per episode, re-firing once per window while the condition persists. Not yet: `step_stalled`, `objective_never_completed`, `region_dwell_no_hint`, `dialog_displaced`, `death_then_silence` |
| `session.journal`, `play_session_id`, `service.version` (§9.5–9.6) | Not yet |
| Unified `decision_outcome` (log + span + counter from one helper); `issue_move_order` + `leg_seq` | Not yet |
| Outbound-intent logging: `setMovementType` outcomes, sampled UPDATE_AVATAR sends, dialog / mission sends, respawn resend list (§9.4) | Not yet |
| Console reject logging, `unhandled_interaction_flag`, `condition_failed`, region-hint containment | Not yet |

No behaviour was changed in this PR — it is logging only. The defects in §2 are all still present.

Reading a bookmark in SigNoz: `scope_name = 'playtest.bookmark'` lists the notes; take a `bookmark_id` and query
`scope_name = 'playtest.bookmark.entity' AND bookmark_id = <id>`. For "the NPC is facing the wrong way", sort by
`wire_facing_vs_caller_deg` — a chasing NPC near 180 with `yaw_byte = 0` is H1. For "it is floating", read
`y_above_ground` (meshed worlds) or `dy_vs_caller` (meshless worlds such as Castle, where `ground_y` is absent and
`navmesh_loaded = false` on the header says why).
