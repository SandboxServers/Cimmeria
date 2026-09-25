> Evidence pass, 2026-09-24. Read-only research against `main` @ 192d4216 and 7 days of colo SigNoz data (2026-09-17 to 2026-09-24). Kept verbatim as the evidence record; the ledger in [../audit.md](../audit.md) supersedes it where they differ.

# SigNoz NPC-AI mining — Castle_CellBlock / Castle, 2026-09-17..24

Source: SigNoz logs, `service.name='cimmeria-server'`, 7-day window ending 2026-09-24.
The analysis scripts and three small outputs (`aggro_by_npc.txt`, `in_place_analysis.txt`, `ticks_analysis.txt`) are in [signoz/](signoz/). The raw JSON pulls (about 12 MB) were not committed; re-pull them with the queries in [../telemetry.md](../telemetry.md). The scripts are
`cond2.py` (JSON to one line per event), `names.py` (npc_id to name[tag]@space, taken from the
spawn logs), `aggro.py`, `aggro2.py`, and `ticks.py`.

## A. Sessions

- **Colo vs local cannot be told apart.** The only resource attributes are
  `deployment.environment=dev`, `service.namespace=cimmeria`, and the SDK attributes. No
  host.name or deploy-env attribute is exported. Two things suggest these are colo runs:
  player_id restarts at 71 several times (the DB is re-seeded, which happens on each colo
  deploy), and entity ids reset (the server restarted).
- **The last play is 2026-09-21 16:52 UTC (11:52 US Central).** After that there are only about
  12 idle log lines a minute (4,320 per 6 h). If the owner played after 09-21, that play is not
  in SigNoz.
- **Activity windows (UTC):** 09-18 10:09 onward, 09-19 12:59 to 09-20 04:20, 09-20 11:52,
  09-20 20:16 to 09-21 03:41, 09-21 11:28, and 09-21 15:55 to 16:52.
- **About 95 world entries.** Almost all are account 6, a fresh character in Castle_CellBlock.
  Account 3 (sdfg, zdfg) and account 4 (test) also went into Castle. The full list is in
  `signoz-raw/ai_main_timeline.txt` (search for "player entered world").
- **NPC-relevant `.bug` bookmarks (19 in total):**
  - 09-19 13:07 awddw, space 65552: "nid guard still standing even tho life is 0".
  - 09-21 00:34:01 wawdadwa, space 65553: "something is wronng with auto attack".
  - 09-21 00:34:13 wawdadwa: **"drone didnt attack"**. PRU 100175 was Fighting,
    has_los_to_caller=False, stationary, dist 13.3. It died 12 s later.
  - 09-21 00:28 zdfg: "why am i not dead" (dead=True, hp 0).
  - 09-21 11:29 w2q: "where is the nid guard?"
  - 09-21 16:07 aweeae: "frost didnt spawnn corpse has no interactibale".
  - Several about the cover indicator: 09-20 11:54 and 09-21 15:59. These are about the
    player's cover, not NPC cover.
  - **No bookmark mentions floating, stuck NPCs, or aggro range.**
  - Full list: `signoz-raw/bookmarks.txt`. The per-entity snapshots are in
    `signoz-raw/bookmark_entities.txt`.

## B. Behaviour reconstruction

### B1. Aggro acquisition (symptom 1)

There are 544 transitions from `NPC aggro: preempt -> Fighting`. I classified each one:

- **Proximity:** an `aggression-driven auto-aggro` log for the same NPC within 2 s before it.
- **Damage or chain:** everything else.

| | proximity | damage/chain |
|---|---|---|
| all transitions | 369 | 175 |
| **first engagement per NPC instance** | **0** | **144** |

Grouped by name and tag, every NPC instance was first engaged by damage or a chain:

- Cellblock Guard [ArmYourself_NIDGuard]: 27
- PRU [ArmYourself_PrisonerRetrievalUnit]: 15
- NID Guard, all Hallway, MessHall, Barracks and Armory tags: 4 to 8 each, 52 in total
- Castle NID Guards, PRUs, Romney, BravoOfficer: 12

**Only two NPCs in the whole tutorial ever get aggression > 0, and both get it from a content
chain:**

- Chain 1008 on `ArmYourself_NIDGuard` (the Cellblock Guard, the first mob outside stasis):
  `set aggression agg_level=1` plus `generate threat 1000`. It fired 50 times.
- Chain 1032 on `ArmYourself_PrisonerRetrievalUnit`: the same two actions. It fired 19 times.
- No other tag ever gets `Content: set aggression`.

Bookmark snapshots agree:

- NID Guard: aggression=0 in 22 of 22 samples.
- Cellblock Guard and PRU: aggression=1.

All 369 proximity auto-aggros come from those two tags, and only after the chain had fired.

**Symptom 1 is confirmed.** Every NID Guard after the first encounter has aggression 0, so it
never takes the `npc_ai_idle_auto_aggro` path and engages only when shot.

### B2. The leash ping-pong loop (symptom 4, plus wasted wire traffic)

Where to look: `signoz-raw/aggro_by_session.txt`, the 16:13 dawdwd and 16:31 wdwaddd sessions.

The loop repeats every 6 s (with the 2 s AI tick):

1. Idle.
2. Auto-aggro on any AoI witness. There is no aggro radius; it fired at 60 to 201 m.
3. Fighting.
4. The target is more than `LEASH_DISTANCE` = 50 **from the NPC's spawn**, so the NPC leashes.
5. The NPC snaps to spawn with full HP.
6. Idle again.

Leash counts:

- NPC 100630: 120 leashes in one session (16:40 to 16:52, `dist_to_spawn=85.0` repeated for
  minutes while the player stood still).
- NPCs 100299, 100271, 100574, 100908, 100467, 100154, 100266: 96, 59, 16, 14, 16, 13, and 11
  leashes respectively.
- About 340 of the 353 leashes across the whole week come from this loop.

During the loop the NPC never moves: every tick shows `dist_to_spawn=0`, `nav_path_len=0`, and
x/y/z = spawn. Each cycle still sends `onSequence(Leash)`, `onStatUpdate` and
`onStateFieldUpdate` to the witness.

**Snap-home event.** NPC 100574 at 09-21 16:14:29 to 16:14:33:

- The NPC is at (-283.4, 65.6, -124.4), 30.7 m from spawn.
- The player moves more than 50 m from spawn, so the NPC leashes.
- The next tick shows the NPC at spawn (-289.5, 68.5, -154.3). That is a 30 m server-side
  teleport.
- wire.out for that window has only `onSequence 0x05`, `onStatUpdate` and `onStateFieldUpdate`
  for 100574. There is no position record, because position updates are not logged in wire.out.
- **So we cannot tell whether the client ever saw the snap.** If it did not, the client shows
  the NPC frozen where it leashed. That matches "moves only so far and gets stuck".

**Stale path after a leash.** NPC 100908 on 09-20 around 03:43:31 to 03:43:35:

- After the leash snap, movement steps resume from about spawn toward the old chase waypoint
  (-286.35, 65.6, -120.4) while the NPC is Idle.
- `npc_ai_leash` sets position = spawn but does not clear `nav_path`. The preempt path does
  clear it.

**How far an NPC chases is bounded by the player's distance from spawn, not the NPC's.** When
the player steps past 50 m from spawn, the NPC gives up wherever it is. When the player comes
back inside 50 m, the loop re-engages from spawn.

### B3. Path failures and height (symptom 3)

- **`npc_ai.path_fail`: 2 in 7 days.**
  - 09-20 03:45:33, Cellblock Guard 100908 to player 5: `reason=no_path`,
    `dy=+28.5` (npc_y 68.5, dest_y 97.0), dist 30. The code then **falls back to a straight
    line through geometry**, which is the float-up-toward-a-player-above mechanism.
  - 09-21 00:37:23, NID Guard 100192 [Hallway05_Guard1]: `degenerate_path`, target 0.7 m away,
    3 ticks with no movement.
- **NPC Y is always interpolated between path waypoints** (`y_source=lerp` in 815 of 815
  samples). It is never snapped to the floor.
- **The sampled movement data shows no steep climb.** No step leg has |dy| / horizontal
  distance above 0.45. The straight-line fallback leg after the no_path event was not in the
  sample.
- **`ground_y` cannot be trusted on multi-level Cellblock.**
  - In 397 of 815 NPC steps, y_offset_from_ground is more than 3. The typical case is
    ground_y=0.2 with the NPC at y about 69, 35, 40 or 25.
  - The path waypoints for those same steps sit at y=68.6, which also comes from the navmesh.
  - So the height query picks a lower navmesh level; it is not measuring a real float.
  - The bookmark `y_above_ground` field is wrong for the same reason. For example, NID Guards
    show y_above_ground 24 to 39.

### B4. Stationary PRU does not fire (the "drone didnt attack" bookmark)

- 19 runs where an NPC stayed in Fighting and did not move. 17 of them are the PRU at
  (-220.3, 66.7, -121.4): `stationary_holds`, has_los=False, in_range=True, 12 to 16 m from the
  player, 3 to 22 ticks per run.
- 113 stationary_holds ticks in total, across 14 PRU instances.
- LoS fails at about 12 m every time. The unit is presumably hovering at y 66.7.

### B5. NPC cover (symptom 2)

- The cover system loads 1,381 sets and 9,353 nodes at every startup.
- There are **zero** NPC cover decisions in the whole week: no `stay_in_cover`,
  `move_to_cover` or `cover_released_flanked`.
- That is across 1,200+ Fighting ticks, with use_cover=True on every NPC.
- `maintain_cover_for_npc` returns NoCover whenever the target is in range. The guards
  attack_in_place 574 times, and the silent NoCover branches (pick_best finds nothing, or
  reservation fails) are never logged.
- **Symptom 2 is confirmed only by absence.**

## C. Evidence per symptom

| # | Symptom | Telemetry evidence | Verdict |
|---|---|---|---|
| 1 | Later mobs don't aggro until attacked | 0 of 144 first engagements were proximity; only chains 1008 and 1032 ever set aggression; NID Guard aggression=0 in bookmarks | **Direct evidence** |
| 2 | No cover seeking or holding | 0 cover-decision logs despite use_cover=True and 1,200+ fight ticks | **Indirect (absence only)** |
| 3 | Floating / walking into the air | 1 `no_path` event to a player 28.5 m above, followed by a straight-line fallback; NPC Y is always lerped; ground_y broken | **Weak.** Mechanism visible, rise not observed |
| 4 | Gets stuck partway, won't re-engage | Leash measured from target to spawn at 50 m; 30 m server snap home with no position record in the log; ping-pong loop keeps the NPC at spawn; stale nav_path after a leash | **Strong mechanism evidence.** What the client saw is unverified |

## D. Missing telemetry (what would have shown each symptom)

1. **Idle NPCs with aggression 0 are never logged.**
   - `npc_ai_tick` filters out Idle NPCs that have no aggression, patrol or wander. So there is
     no log when a player walks past a NID Guard.
   - Needed: a sampled "idle NPC saw an opposing player at distance d, aggression=0, no seed"
     event.
   - Needed: an aggro radius as a field. Today the effective radius is the AoI radius.
2. **Every aggro should be logged with its cause.**
   - Preempt logs `attacker` but not the source (auto-seed, damage or content chain), nor the
     threat amount, nor the distance.
   - I had to infer the cause from timing.
3. **wire.out does not log NPC position or volatile updates.**
   - We cannot see what position the client was sent after a leash snap, or during a
     straight-line fallback.
   - Needed: a sampled `aoi.position_emit` for NPCs, including a forced-snap flag.
4. **The ground height query is wrong on multi-level maps.**
   - `get_navmesh_height` returns 0.2 (a lower level) on Cellblock's upper floors. That makes
     y_offset_from_ground and bookmark y_above_ground useless.
   - Needed: a height query that is aware of the current Y, plus an `npc_off_mesh` or
     `npc_y_above_floor > 1.5` warning on each step.
5. **Movement sampling misses the dangerous legs.**
   - The sampler covers the first few steps and then about one per second.
   - Needed: always log every step leg whose endpoint came from the straight-line fallback,
     and log slope > 0.6 or |dy| > 2 per leg.
6. **path_fail is almost never emitted.**
   - Only `no_path` and `degenerate_path` exist. There is no event for "target unreachable
     vertically", for "partial path (end not near target)", or for an NPC that makes no
     progress over N ticks while chasing.
   - Needed: a `npc_ai.stuck` event with position delta, next_wp, nav_path_len and has_los.
7. **Leash events lack context.**
   - They carry target-to-spawn distance but not NPC-to-spawn distance, NPC position before
     the snap, or whether a follower was exempt.
   - The fields named `dist_to_spawn` in the leash log and in the tick log mean different
     things (target vs NPC), which is confusing.
8. **The cover decision path is invisible when nothing happens.**
   - Needed: log NoCover reasons (`in_range`, `no_candidate`, `reserve_failed`,
     `use_cover=false`, `stationary`) plus candidate count, sampled.
9. **LoS failures have no detail.**
   - The PRU's has_los=False at 12 m comes with no ray start/end heights and no navmesh vs
     geometry source.
10. **Deployment identity is missing.**
    - Resource attributes need host.name or `cimmeria.deploy_env`, and a git SHA, so colo and
      local sessions can be told apart and matched to builds.

## E. Follow-up: "frozen but attacking" and "running in place" (colo, GM account)

The owner confirmed that all of these sessions ran on the colo deploy. Analysis: `signoz-raw/in_place_analysis.txt`, produced by
`inplace.py` from every `npc_ai.tick` and `movement.npc` event in the 7-day window.

**Mechanism: velocity goes stale when a path is cut off mid-leg. The code side is confirmed; the
client side is inferred.**

- Every 100 ms the AoI tick sends each witness an `EntityMoved` (UPDATE_AVATAR). It carries the
  NPC's stored `position`, `direction`, and **`velocity`**, whether or not the NPC moved
  (`space_manager/aoi.rs:228-240`).
- `velocity` is written only by `update_entity_position`, which the movement tick calls. It is
  zeroed only when a path finishes at its last waypoint, on death (`combat/state.rs:121`), on
  respawn, and in `npc_ai/lifecycle/mod.rs:144`.
- Several call sites clear `nav_path` mid-leg and **leave velocity at the chase value**, about
  6 u/s (move_speed 0.6 × 10):
  - `attack_in_place`: `fight.rs:551`
  - aggro preempt: `aggro.rs:127`
  - cover release: `fight.rs:332`
  - min-range backup: `fight.rs:521`
  - patrol, wander, follow, investigate, and the content executor
- Leash snaps position to spawn and clears neither `nav_path` nor velocity.
- **Result:** the server keeps sending "this NPC is at X, moving at 6 u/s" while X never
  changes. Our reading is that the client animates a run in place, or rubber-bands. That matches
  "running in place" and "frozen but attacking".

**Telemetry evidence**

- 96 moments where a Fighting NPC went from having a path to having none between two AI ticks:
  - **67 were INTERRUPTED mid-leg.** Every one was `attack_in_place` clearing the path; no
    `path_complete=true` event came before it.
  - 29 ended normally, with the path completed.
- After an interruption, the NPC stands still at that spot and keeps firing:
  - 100768 [ArmYourself_NIDGuard]: 52 s and 23 attacks, from 09-20 00:40:47.
  - 100350: 50 s, from 09-21 11:28:49.
  - 100210: 48 s, from 09-21 00:30:37.
  - 100467: 44 s, from 09-19 20:48:39.
  - 100691 and 100154: 32 s each.
  - 100434: 28 s, from 09-21 15:58:03.
  - 100574: 20 s, stopped mid-ramp at (-296.3, 67.4, -142.3), from 09-21 16:14:05.
  - Hallway, MessHall and Barracks NID Guards too: 100193 for 18 s, 100186 for 12 s, several
    others for 6-10 s.
- Throughout `attack_in_place` (574 ticks), `stationary_holds` (113) and `no_ability` (16), the
  broadcast movement type stays **CombatAdvance**. The client is told "advancing" the whole time
  the NPC is still.
- "Frozen but attacking" runs (4 or more attack/hold ticks with the NPC's x/z unchanged): 319
  Cellblock Guard ticks, 150 PRU ticks, and about 120 ticks across other NID Guards.
- Some NPCs park **inside the player** while attacking: Hallway05_Guard2 at 0.35–0.7 m from its
  target for 10 ticks (09-19 22:37 and 09-21 00:37); Barracks_Guard2 at 0.69 m.
- 2 Idle ticks still had a path: 100908 walked its stale chase path after a leash.

**Blind spot**

- A sampled `wire.out.avatar_update` log exists in the code: 1 in 100 sends, with vx/vy/vz,
  added 2026-09-19 00:18 -0500. **It has zero events in SigNoz.** Either the colo build predates
  it or the debug filter drops it.
- So I could not directly observe the nonzero velocity being sent to the client while the NPC
  stood still. That proof needs the target enabled on the colo, or a per-NPC "velocity nonzero
  but position unchanged for N ticks" warning.
