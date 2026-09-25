# NPC AI decision-layer forensics — Lomiada playtest 2026-09-18/19

Session window: 23:55 UTC → 01:35 UTC (6:55 PM → 8:35 PM CDT).
Server boot with new image: 23:49:37 UTC (cover service load timestamp).
Players: entity_id **2** (human soldier, ~23:55–01:03 UTC) and entity_id **3**
(Jaffa, ~01:04–01:35 UTC). Castle = space 65537 (non-instanced).
Castle_CellBlock instances = space 65552 (char 2) and 65553 (char 3).

All times below are UTC unless marked CDT. CDT = UTC − 5h.

---

## 0. The headline: the AI layer is nearly unobservable, and two whole
## behaviors never executed at all

Two structural facts dominate every claim below.

**(a) `decision_outcome` is split into two mutually invisible halves.**

`dispatch.rs:70-81` declares a `npc_ai.decision` span with an empty
`decision_outcome` field. Two *different* mechanisms fill it:

- `crates/services/src/cell/service/npc_ai/fight.rs` writes
  `decision_outcome = "..."` as an **inline log-event field** on
  `tracing::info!/debug!` (lines 187, 273, 286, 298, 368, 408, 426, 457, 484,
  497). These are queryable as logs but **never increment the
  `npc_ai_decisions_total` counter**.
- Every other handler (`follow.rs`, `patrol.rs`, `wander.rs`,
  `investigate.rs`, `lifecycle.rs`) calls
  `record_decision_outcome()` (`npc_ai/mod.rs:90-96`), which records to the
  **span** and increments the **counter** — and emits **no log line at all**.

Consequence: the team lead's "zero `follow_band`, zero patrol/wander outcomes"
is a *measurement artifact*, not evidence of absence. `follow_band` fired on
every in-band follow tick; it is simply not a log. Conversely `leashed`,
`no_path`, `move_to_cover` are logs with no counter.

Verified: `signoz_aggregate_logs groupBy=decision_outcome` over the whole
session returns only `attack_in_place` 123, `chase` 25, `stationary_holds` 17.
Those are exactly the three fight.rs log-field outcomes that fired. Nothing
from `record_decision_outcome` appears, because those sites emit no log.

**(b) Idle-proximity aggro never ran once — not once in 100 minutes.**

`npc_ai_idle_auto_aggro` (`fight.rs:23-77`) emits
`tracing::info!("NPC AI: aggression-driven auto-aggro on opposing-faction
player")` whenever it picks a target. That string has **zero occurrences** in
the session. It is gated by `dispatch.rs:59`:

```rust
|| (*state == AiState::Idle && (*aggression > 0 || *has_patrol || *has_wander))
```

and by `dispatch.rs:105` (`if aggression > 0`).

`db/resources/Worlds/Seed/spawnlist.sql` has **no `aggression` column** — the
INSERT column list is `(spawn_id, x, y, z, heading, world_id, template_id,
tag, set_name)`. `entity_templates.sql` has no `aggression` either. So
`aggression == 0` for every seeded NPC in the game, and every Idle NPC is
filtered out of the AI-tick snapshot before any handler runs.

**Every one of the 50 aggros this session came from the player shooting
first** (`threat::aggro` "NPC aggro: preempt -> Fighting", 50 events, all via
`generate_threat`). No mob has ever noticed a player walking past it.

---

## A. Per-claim verdicts

### Claim 1 — 7:03 PM CDT (00:03 UTC): Cellblock guard "came from his cover then moonwalking in the air"

**CONTRADICTED (the cover half). NO-DATA (the moonwalk half — sibling agent).**

NPCs never used cover this session. Zero `move_to_cover`, `stay_in_cover`,
`cover_released_flanked` events. The cover system *is* loaded —
`cimmeria_services::cell::cover::loader` at 23:49:37 logged
`Loaded cover sets count=1381` and `Loaded cover nodes count=9353`
(`skipped_height=0, skipped_quality=0, skipped_tail=0`) — so data is present
and healthy.

The 5 `fire_cover_entered` events in the session are all **player** entities
(`entity_id` 2 and 3, `player_id` 71/72) entering cover *regions* — the
content-trigger path, not the NPC AI path:
- 00:05:45 `no chains matched` (entity 2)
- 00:07:15 `matched`, actions=2, HEIGHT_Low/QUALITY_Good (entity 2, player 71)
- 00:07:42 `no chains matched` (entity 2)
- 01:08:34 `no chains matched` (entity 3)
- 01:09:30 `matched`, actions=2 (entity 3, player 72)

NPC cover requires `use_cover && !is_stationary` (`fight.rs:256`) **and** the
NPC to already be `Fighting` with `!in_range`. Since `use_cover` comes from
the template and no NPC ever produced a cover decision log, `use_cover` is
almost certainly false across the seed (worth a direct DB check — see §D).

What the tester actually saw: a guard the player had shot, transitioning
Idle→Fighting via `generate_threat`, then `chase` (25 events) walking toward
them. The "came out of cover" reading is the guard leaving its *spawn pose*,
not a cover slot. The moonwalk/air part is movement kinematics — sibling agent.

*Note:* `cover_set_id: 1381` appears on every `fire_cover_entered` event, and
1381 is also the total set count. That is suspicious enough to check whether
the lookup is returning a last/fallback set rather than the true nearest set.

### Claim 2 — Marsh (7:11 PM / 00:11 UTC, and 8:10 PM / 01:11 UTC) and Coppleman (7:23 PM) do not follow

**CONFIRMED-BY-LOGS — and this is the most serious finding.**

`set_follow_target` **did** fire correctly for Marsh, twice:

| UTC | CDT | chain | npc_id | entity_tag | resolved_target | use_player |
|---|---|---|---|---|---|---|
| 00:11:43.293 | 7:11:43 PM | 1174 | 100122 | `Preparation_ColMarsh` | `Some(2)` | true |
| 01:11:36.992 | 8:11:36 PM | 1174 | 100150 | `Preparation_ColMarsh` | `Some(3)` | true |

Both resolved a real player. Per `executor/world/mod.rs:183-194` that sets
`follow_target_id = Some(player)`, `ai_state = AiState::Follow`,
`nav_path.clear()`.

**Then nothing.** The complete log inventory for npc_id 100122 and 100150
across the entire session is **four rows total**: two `Spawned instance NPC
from DB` and the two `set follow target` rows above. Zero `follow_routed`,
zero `NPC reached waypoint`, zero movement of any kind, zero AI decisions.

Contrast Zerutska (100112), the only NPC that followed: 54 `follow_routed` +
54 `NPC reached waypoint`.

So Marsh entered `Follow` and then either was never ticked, or exited Follow
silently on the first tick. `npc_ai_follow` (`follow.rs:21-125`) has **four
early-return branches that emit no log whatsoever**:

1. `follow.rs:42-48` — `follow_target_id == None` → `ai_state = Idle`, silent.
2. `follow.rs:50-58` — target entity not found in `space_mgr` → clears
   `follow_target_id`, `ai_state = Idle`, **silent**.
3. `follow.rs:69-78` — in band (`dist < min_d` or `dist <= max_d`) →
   `record_decision_outcome("follow_band")` — counter only, no log.
4. `follow.rs:80-84` — `nav_path` non-empty → `follow_band`, no log.

Branch 2 is the lethal one: it *permanently clears* the follow target and
drops to Idle, and once Idle with `aggression == 0` / no patrol / no wander,
`dispatch.rs:59` excludes the NPC from the tick snapshot **forever**. A single
transient failure to resolve the player entity strands the escort for the rest
of the session, and leaves no trace.

**Why Zerutska worked and Marsh didn't — the re-arming difference.** Chain
1302 re-fired `set_follow_target` on Zerutska **10 times in 65 seconds**
(00:27:32.589, 00:27:40.604, 00:27:41.110, 00:27:41.759, 00:27:41.901,
00:27:42.049, 00:27:42.149, 00:27:42.279, 00:27:42.404, 00:28:37.029 — all
`resolved_target Some(2)`), on top of chain 1263 at 00:27:29.829. Marsh's
chain 1174 fires **exactly once**. If something knocks the NPC out of Follow,
only the repeatedly-re-armed Zerutska recovers. Marsh has no second chance.

This also matches "he did came after me" later (7:11 PM) and
"marsh really doesnt want to follow" (8:10 PM) — intermittent, consistent with
a race on first tick rather than a hard config problem.

Coppleman: `CaptCoppleman` spawned once in Castle (space 65537). **No
`set_follow_target` ever fired for it** — zero rows. So Coppleman is not a
follow bug at all; the content chain that should arm him never ran (mission
agent's territory).

Secondary, from agent memory and confirmed by code: even when Follow works,
`leash.rs:57` ends at `AiState::Idle`, not back to `Follow`, and
`threat/aggro.rs:69-76` lists `Follow` as preemptable. **A follower that ever
takes damage stops following permanently.** Marsh spawns in the Preparation
room among hostile NID guards.

### Claim 3 — 7:13 PM: "enemies walk straight to me facing backwards" / "other guard didnt aggro"

**"Didn't aggro": CONFIRMED-BY-LOGS, root cause identified.** See §0(b).
`aggression` does not exist as a column in the spawnlist seed, so
proximity/social aggro is structurally impossible. There is **no assist/social
aggro code path at all** — `generate_threat` (`threat/aggro.rs:50-102`) only
ever touches the single NPC that was hit. A guard standing next to the one you
shot has no mechanism to join. This is a missing feature, not a regression.

"Walk straight to me": consistent with `chase` (25 events) — see Claim 5 for
why the paths are straight lines. "Facing backwards" is the sibling agent's
facing/yaw domain.

### Claim 4 — 7:20 PM: killed all NID guards in armory, mission didn't tick

**Partial — death hooks fired for the deaths that were logged.**
19 `NPC death: respawn scheduled` + 19 `NPC respawned` — every death that
reached `mark_npc_dead` also armed a respawn, so the death path itself is
sound for direct-damage kills.

The known hole (agent memory, verified still present): **DoT / effect-pulse
kills never fire the death path at all.** `cell/effects/pulsing/tick.rs::fire_pulse`
has no alive→dead detection; `mark_npc_dead` is only reachable from
`cell/abilities/damage_apply/mod.rs:222` and `cell/abilities/death.rs:299`. A
mob whose last HP comes from a bleed sits at 0 HP with `ai_state` unchanged —
no `BSF_DEAD`, no loot, no `respawn_at`, no `fire_entity_death`, so no
kill-count credit. **That is a direct candidate for both claim 4 and claim 7.**
Mission agent owns the objective side; flagging the engine cause here.

### Claim 5 — 7:26–7:36 PM: Zerutska follows, levitates, walks through walls; a second Zerutska; follower vanishes then returns

**CONFIRMED-BY-LOGS, all four sub-claims, with exact causes.**

**Two Zerutskas = two seeded spawnlist rows.** Not a follower copy, not a
duplicate spawn. Both exist from server boot in Castle (space 65537):

| npc_id | name | tag |
|---|---|---|
| 100112 | `Castle_Zuritska` | `Castle_Zuritska_Cell` |
| 100113 | `Castle_Zuritska` | `Castle_Zuritska_Comms` |

(Note the seed spells it *Zuritska*.) The Cell one is the escort; the Comms
one is the static "already arrived" actor at the communication terminal.
Nothing hides or despawns either, so once the escort reaches the comms room
the player sees both. This is a **content/seed design gap**, not an AI bug:
the chain needs to despawn or suppress `Castle_Zuritska_Comms` until the
escort completes (or despawn the escort on arrival).

**"The follower vanished when I entered the region" — exact match.**
7:33 PM CDT = **00:33:03.269 UTC**: chain **1291** fired `set follow target`
on npc_id 100112 with `resolved_target: "None"`, `target_tag: "None"`,
`use_player` absent. That is the documented *clear* shape
(`executor/world/mod.rs:190-193`): `follow_target_id = None`,
`ai_state = Idle`, `nav_path.clear()`.

The NPC did **not** despawn. It stopped dead wherever it was — and per the
follow logs it was routinely 50–95 units behind the player, so it stopped out
of sight. Confirmed: in the 00:33–00:40 window npc_id 100112 produced exactly
1 movement/AI event and then nothing. The player read "stopped far behind me +
I'm now looking at the Comms twin" as "vanished". "Guess whos back" at 7:36 PM
is the player walking back into AoI range of the stalled escort.

The clear fired again for the Jaffa run: chain 1291 at **01:21:01.519** on
100112 (`resolved_target None`, entity 3), after chain 1263 armed it at
01:20:24.071.

**"Levitating" / "came thru walls and floors" — root cause proven by log
arithmetic.** npc_id 100112 produced **exactly 54 `follow_routed` and exactly
54 `NPC reached waypoint`**. A 1:1 ratio is only possible if every single
follow leg queued exactly one waypoint.

`follow.rs:99-111`:

```rust
let path = space_mgr.find_path(npc_id, &npc_pos, &dest).unwrap_or_default();
if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
    npc.nav_path.clear();
    if path.len() > 1 {
        for wp in path.into_iter().skip(1) { npc.nav_path.push_back(wp); }
    } else {
        npc.nav_path.push_back(dest);   // <-- straight line through geometry
    }
}
```

`find_path` returned `None` (or a ≤1-point path) on **all 54 legs**, so every
leg took the `else` branch: one raw straight-line waypoint, interpolated
directly through walls and floors, with the destination's Y taken from the
player's Y (hence "levitating", then "he came down" when the player descended).
`unwrap_or_default()` swallows the `None` — **there is no `no_path` log in the
follow handler at all**, unlike `fight.rs:423-433` which has one. This failure
is 100% silent.

Sample legs (npc 100112 → target 3): `dist=94.852`, `dist=69.855`,
`dist=49.756` against `max_d=5`. The escort is chronically 10–19× outside its
own follow band. Per agent memory the cause is the AI tick cadence (2 s, 20th
AoI tick, `message_loop.rs:126`) combined with re-pathing only when `nav_path`
empties — plus default `move_speed 0.6` (6.0 u/s) vs a player's 8.125 u/s,
which never converges. Only template 10 "Col Marsh (pet)" has a non-NULL
`move_speed` (0.9) in the seed.

### Claim 6 — 7:37 PM: enemies respawned

**CONFIRMED-BY-LOGS. Cadence is correct and the respawn is clean.**

19 deaths → 19 respawns, all in Castle, all `respawn_secs: 120`. Death→respawn
deltas are exact:

| npc_id | death scheduled | respawned | delta |
|---|---|---|---|
| 100022 | 01:27:26.205 | 01:29:26.793 | 120.6 s |
| 100116 | 01:26:30.924 | 01:28:31.793 | 120.9 s |
| 100015 | 01:25:28.399 | 01:27:28.792 | 120.4 s |
| 100014 | 01:25:13.621 | 01:27:13.793 | 120.2 s |
| 100030 | 01:24:48.269 | 01:26:48.792 | 120.5 s |

Respawn snaps to `spawn_pos`, resets `state_field` to 0 (BSF_DEAD cleared) and
`interaction_flags` to 0 (loot cleared), restores HP, notifies witnesses
(`ticks/npc_respawn/mod.rs`). Threat list, loot and cooldowns are reset
(`npc_respawn/mod.rs:200-233`); the `tag` survives so `entity_dead_tag`
triggers re-arm.

Only Castle rows have `respawn_secs` set (CA05/#667 set the 8 Castle hostiles
to 120 s). Castle_CellBlock instance mobs still have NULL `respawn_secs` and
are one-shot — the corpse stays forever.

### Claim 7 — 8:24 PM (01:24 UTC): killed an officer, "he isnt really dead", clicked again → dead

**NO-DATA from the AI layer; two ranked engine candidates.**

Nothing in the AI decision layer explains an alive-looking corpse. Two causes,
in confidence order:

1. **The 120 s respawn colliding with the player's mental model.** At exactly
   this window mobs the player had killed 2 minutes earlier were coming back
   alive in place (respawns at 01:26:48, 01:27:13, 01:27:28 — deaths at
   01:24:48, 01:25:13, 01:25:28). "He isn't really dead" is a plausible read of
   a mob that died and stood back up. Not a bug; a tuning/communication issue.
2. **DoT-pulse kill with no death path** (see Claim 4). The mob sits at 0 HP
   with `ai_state` unchanged and `BSF_DEAD` never set, so it renders alive.
   "Clicked the guard again dead now" fits an interaction-path refresh
   recomputing the interaction flags from the real (0) HP.

The death-fanout / `BSF_DEAD` broadcast half is the sibling agent's and the
combat agent's territory.

### Claim 8 — "leashed npcs exhibited odd behavior"

**CONTRADICTED. Leash never fired — it is close to unreachable, and when it
does fire it does the wrong thing.**

Zero `leashed` events in the session. Three separate reasons:

**(i) The leash test measures the wrong distance.** `fight.rs:178-180`:

```rust
if let Some(spawn) = spawn_pos {
    let dist_to_spawn = spawn.distance_to(&target_pos);   // spawn → TARGET
    if dist_to_spawn > combat::LEASH_DISTANCE {
```

It measures spawn→**player**, not spawn→**NPC**. So an NPC that has chased the
player 300 units will *not* leash if the player circles back near the mob's
spawn; and an NPC that has never moved *will* leash the instant the player
walks 50 units away from its spawn point. The doc comment on line 177 says "if
target is too far from NPC's spawn point" — the code matches the comment, but
the comment encodes the wrong invariant. Every other engine measures
NPC→spawn. This is the fix that matters.

**(ii) `LEASH_DISTANCE = 50.0` is a global constant**
(`combat/threat/aggro.rs:11`) with no per-template override.

**(iii) There is no leash in the canonical reference at all.**
`deprecated/python/cell/SGWMob.py` only ever assigns
`AI_STATE_Spawning` (line 22), `AI_STATE_Fighting` (161), `AI_STATE_Idle`
(291, 305) and `AI_STATE_Dead` (315). There is no leash state, no
walk-home, no heal-on-reset. `AiState::Leashing` is a Rust invention.

**What the tester actually saw instead of leashing.** The only ways an NPC
leaves `Fighting` today are: target dead (`fight.rs:156-164`, 12 events),
target gone (`fight.rs:168-174`), or no threat left (`fight.rs:128-143`,
7 events). None of those move the NPC. There is **no threat decay, no
out-of-AoI disengage, no distance-based reset**. So a mob you shot and ran
away from stays in `Fighting` indefinitely, standing wherever the chase ended,
re-pathing toward you every 2 s forever. That is "odd leash behavior".

**What leash does when it does fire** (`leash.rs:12-90`): broadcasts
`MobMovementType::Leash`; **teleports** the NPC to `spawn_position` instantly
(no walk-back — `leash.rs:48`, and skipped entirely when
`follow_target_id.is_some()`); restores HP to max; `ai_state = Idle`;
`threat_list.clear()`; `clear_all_cooldowns()`; sends entity methods 20 (stats)
and 19 (state field); then `broadcast_movement_type(None)`.

**Fighting → Idle does NOT restore position or heading.** The
no-threat-targets path (`fight.rs:128-143`) sets `ai_state = Idle`, clears
threat, releases cover and returns — the NPC is left standing at its
chase-end position with its chase-end facing, permanently (it is then
filtered out of the tick by `dispatch.rs:59` because `aggression == 0`).
Only the respawn tick ever restores spawn position and `spawn_dir`.

**Cross-check on the GM navmesh bypass:** `movement.navmesh_gm_bypass` fired
26×. When a GM stands off-mesh, `find_path` to that position fails, and both
`follow.rs:99` (silently) and `fight.rs:417` (`no_path`) degrade. For follow
the degradation is the straight-line fallback — which is exactly what the
54/54 ratio shows. The tester being a GM plausibly *worsened* the Zerutska
wall-clipping.

---

## B. Root-cause hypotheses, ranked by confidence

**1. (Certain) Proximity aggro is structurally dead.** `aggression` is absent
from the spawnlist and entity_templates seed → 0 everywhere →
`dispatch.rs:59` excludes every Idle NPC from the tick, and `dispatch.rs:105`
would gate it again. No NPC can notice a player. There is also no
assist/social aggro code anywhere. Fix: add `aggression` to the spawnlist
schema + seed; add a `social_aggro_radius` pass in `npc_ai_idle_auto_aggro`.

**2. (Certain) The follow handler's pathfinding failure is total and silent.**
`follow.rs:99-111` — `find_path(...).unwrap_or_default()` converts `None` into
a straight-line waypoint with no log, no counter, no warn. Proven by the exact
54 `follow_routed` : 54 `waypoint_reached` ratio for npc 100112. Fix: emit the
same `no_path` negative log `fight.rs:423` already has, and decide whether the
fallback should exist at all (agent memory: a cross-navmesh-component follow
in Castle_CellBlock fails this way 100% of the time).

**3. (High) The escort dies silently on the first tick, and only re-armed
chains survive.** `follow.rs:50-58` clears `follow_target_id` and drops to
Idle with zero logging when the target entity lookup misses; once Idle with
`aggression == 0` the NPC is never ticked again. Marsh (armed once) produced
zero AI events; Zerutska (re-armed 11×) worked. Fix: `warn!` on that branch
per the negative-logging convention, and make Follow survive a transient
lookup miss (retry N ticks before clearing).

**4. (High) Follow never resumes after combat.** `threat/aggro.rs:69-76` makes
`Follow` preemptable into `Fighting`; `leash.rs:57` and `fight.rs:130` both
end at `Idle`, never back at `Follow`. `follow_target_id` survives but is
inert. An escort that takes one point of damage stops escorting forever. Marsh
spawns among hostile NID guards.

**5. (High) The leash predicate measures spawn→target instead of spawn→NPC**
(`fight.rs:179`), and no threat-decay / AoI-disengage path exists, so mobs
park permanently wherever a fight ended. This is the "odd leashed behavior".

**6. (High) Fighting→Idle restores neither position nor heading**
(`fight.rs:128-143`). Only `npc_respawn` restores spawn pose.

**7. (Medium-high) Two Zerutskas is seed/content design, not a runtime bug.**
`Castle_Zuritska_Cell` (100112) and `Castle_Zuritska_Comms` (100113) are two
spawnlist rows both live from boot. Needs a despawn/suppress action in the
escort chain.

**8. (Medium) NPC cover is loaded but never exercised.** 9,353 nodes / 1,381
sets load cleanly; `use_cover` appears false across the seed, and the gate
also requires `!is_stationary` and `!in_range` (`fight.rs:256`). Needs a DB
check on `entity_templates.use_cover`.

**9. (Medium) DoT-pulse kills bypass the death path entirely**
(`cell/effects/pulsing/tick.rs::fire_pulse` has no alive→dead detection) —
candidate for both the missed kill-count (claim 4) and the alive-corpse
(claim 7).

**10. (Low-medium) `cover_set_id` is always 1381**, which equals the total
set count — possible last-set/fallback lookup rather than nearest-set.

---

## C. Missing telemetry seams

### C1. Unify `decision_outcome` — the single highest-value fix

`fight.rs` should call `record_decision_outcome(...)` at each of its 10
outcome sites *in addition to* its log, and `record_decision_outcome`
(`npc_ai/mod.rs:90`) should itself emit a `tracing::debug!(target: "npc_ai",
event = "decision", decision_outcome = outcome, npc_id, ...)`. Today no single
query can enumerate what the AI decided. Per
`docs/architecture/observability.md` the `npc_ai.decision_outcome` enum is
supposed to be the stable query surface; half of it is unqueryable as logs and
the other half is unqueryable as metrics.

### C2. Negative logs that do not exist (per docs/architecture/negative-logging-convention.md)

| Target | Level | Where | Fields | Why |
|---|---|---|---|---|
| `npc_ai` `decision_outcome="follow_no_path"` | WARN | `follow.rs:99` — the `find_path` → `unwrap_or_default()` seam | `npc_id, target_id, npc_x/y/z, dest_x/y/z, dist, navmesh_loaded, straight_line_fallback=true` | The entire Zerutska wall-clipping failure is invisible. `fight.rs:423` already has the twin. |
| `npc_ai` `decision_outcome="follow_target_lost"` | WARN | `follow.rs:50-58` | `npc_id, prior_target_id, reason="entity_not_found"` | Silently and permanently strands every escort. Prime Marsh suspect. |
| `npc_ai` `decision_outcome="follow_dropped_no_target"` | DEBUG | `follow.rs:42-48` | `npc_id` | Distinguishes "cleared by chain" from "lost target". |
| `npc_ai` `decision_outcome="auto_aggro_none"` | DEBUG | `fight.rs:57` else-branch | `npc_id, witnesses_scanned, rejected_faction, rejected_dead, nearest_dist` | The **why-not-aggro** seam. Zero coverage today; would have answered claim 3 instantly. |
| `npc_ai` `decision_outcome="idle_not_ticked"` | DEBUG, sampled | `dispatch.rs:45-60` filter | `npc_id, aggression, has_patrol, has_wander` | Nothing records that an Idle NPC was excluded from the tick. This is why "the guard did nothing" produces no data at all. |
| `npc_ai` `decision_outcome="fight_idle_reset"` | INFO | `fight.rs:128-143` | `npc_id, npc_x/y/z, spawn_x/y/z, dist_from_spawn, returned_home=false` | Makes "mob parked where the fight ended" measurable. Currently DEBUG with only `npc_id`. |
| `cover` | INFO once at Fighting entry | `fight.rs:256` gate | `npc_id, use_cover, is_stationary, in_range` | Explains why 9,353 loaded cover nodes are never used. |

### C3. Fields missing from events that *did* fire

- **`follow_routed`** (`follow.rs:116-124`) carries only `npc_id, target_id,
  dist, max_d`. It needs `npc_x/y/z`, `target_x/y/z`, `dest_x/y/z`,
  `path_len`, `used_fallback`, `move_speed`. Without `path_len` I had to infer
  the straight-line fallback from a 54:54 log-count coincidence.
- **`attack_in_place` / `chase` / `stationary_holds`** carry no NPC position,
  no target position, no yaw/facing, no `spawn_dist`. A fight cannot be
  reconstructed spatially.
- **`NPC aggro: preempt -> Fighting`** (`threat/aggro.rs:84-89`) carries no
  `threat_amount`, no `aggro_reason` (damage / content `generate_threat` /
  auto-aggro), and no positions. Add `aggro_reason` as an explicit enum
  parameter to `generate_threat`.
- **`Spawned instance NPC from DB`** (`spawner/npcs.rs`) has no `template_id`,
  no `aggression`, no `use_cover`, no `is_stationary`, no `move_speed`, no
  `respawn_secs`. Every "why does this NPC behave like that" question needs a
  DB round-trip. Adding the resolved behavior flags to the spawn log would
  make the seed self-documenting in telemetry.
- **`Content: set follow target`** should log the resulting
  `follow_min_distance` / `follow_max_distance` / `move_speed` so a
  never-converging escort is visible at arm time.

### C4. Sampling is too sparse to reconstruct a fight

`npc_ai` is DEBUG and produced 219 events for ~50 aggros over 100 minutes —
roughly 4 events per fight, from a handler that runs every 2 s. The in-band and
no-op branches emit nothing at all, so a fight's *duration* is unknowable. The
`npc_ai.decision` span exists (`dispatch.rs:70`) and is the right place to fix
this: it should always record `decision_outcome`, plus `npc_x/y/z`,
`dist_to_target` and `dist_to_spawn`, so every tick of every NPC is one
queryable span even when the handler no-ops. Per
`docs/architecture/instrumentation-discipline.md` that is a span-attribute
change, not new log volume.

---

## D. Questions only the owner / playtester can answer

1. **DB check (owner, fastest path):** what are `aggression`, `use_cover`,
   `is_stationary`, `move_speed`, `follow_min_distance`, `follow_max_distance`
   on `entity_templates` for template 10 (Col Marsh pet) and the Zerutska
   template? My read is `aggression = 0` and `use_cover = false` across the
   board, but the column may not exist at all — the spawnlist seed has neither.
2. **Was Castle's navmesh (`castle.nav`) actually loaded at boot?** 54/54
   straight-line fallbacks in Castle says either "not loaded" or "Zerutska
   spawns off-mesh". There is no startup log naming which `.nav` files loaded —
   worth adding regardless.
3. **Chain 1291** (the Zerutska follow clear at 00:33:03 and 01:21:01) — is
   clearing the follow at the comms region *intended*? If yes, the chain also
   needs to despawn/suppress `Castle_Zuritska_Comms` (100113) or teleport the
   escort, or the player will always see two.
4. **Chain 1302 fired `set_follow_target` 10× in 65 s** on the same NPC with
   the same target. Is that an intentional keep-alive or a trigger misfire? It
   is currently the only reason Zerutska follows at all, so "fixing" it would
   regress the one working escort.
5. **Chain 1174 (Marsh)** — is there meant to be a re-arm or a periodic
   refresh, as Zerutska has?
6. **Coppleman** has no `set_follow_target` at all. Is a chain missing, or was
   he never meant to follow? (Mission agent.)
7. **Playtester:** when Marsh "did came after me" at ~7:11 PM, had you damaged
   him or had any NID guard damaged him beforehand? This distinguishes
   hypothesis 3 (silent target-lost) from hypothesis 4 (combat preemption).
8. **Playtester:** the "not really dead" officer at 8:24 PM — was he standing
   upright, or lying down but still clickable? Upright ⇒ DoT-kill bypass
   (hypothesis 9); lying down ⇒ interaction-flag refresh.

### Screenshots that would materially help

- **7:03 PM** — the "moonwalking" guard, to see whether it is near a cover
  node position or just mid-chase.
- **7:33 PM** — the comms terminal room showing *both* Zerutskas, to confirm
  the Comms twin (100113) is the one the player saw.
- **8:24 PM** — the "not really dead" officer, upright vs. prone.

---
---

# ADDENDUM — owner interview follow-ups (leash trace, second-leg order issuance, facing gate, logging spec)

## E. Leash: full end-to-end trace

**Rigorous negative first.** A free-text search for `leash` across **every
scope, no filter**, 23:26–01:35 UTC returns **zero rows** (76 rows scanned).
Both leash log sites are INFO. So leash genuinely did not fire tonight — it is
not hidden under another scope. The owner's memory of leash working is from
prior builds or from the client-side animation, not from this session.

### E1. What starts it — exactly one site

`crates/services/src/cell/service/npc_ai/fight.rs:177-212`. Preconditions, all
required:

- `ai_state == Fighting` (only `npc_ai_fight` contains the check)
- a live top-threat target exists (checked at `fight.rs:124-175` first)
- `spawn_position.is_some()`
- `spawn.distance_to(&target_pos) > LEASH_DISTANCE (50.0)` — **spawn→TARGET,
  not spawn→NPC** (`fight.rs:179`)

Nothing else in the codebase ever writes `AiState::Leashing` outside tests.

### E2. The transition (fight.rs:181-211)

1. `npc.ai_state = AiState::Leashing`
2. `npc.threat_list.clear()` — **at the transition, not at completion**
3. `tracing::info!` target `npc_ai`, `event="decision"`,
   `decision_outcome="leashed"`, fields `npc_id, target_id, dist_to_spawn`
4. `space_mgr.cover.release_for_entity(...)`
5. `broadcast_movement_type(Some(MobMovementType::Leash))` — wire
   `setMovementType`, byte **5** (`cell_methods/being.rs:92`)
6. `return`

### E3. The "return home" — there is no path home

`npc_ai_leash` (`leash.rs:12-90`) runs on the **next** AI tick (up to 2 s
later) and does all of this in one shot:

| Step | Code | Behavior |
|---|---|---|
| Re-broadcast Leash | `leash.rs:24-30` | dedup'd, normally a no-op |
| **Teleport home** | `leash.rs:48-50` | `npc.position = spawn_pos` — a **raw field write** |
| Heal | `leash.rs:53-55` | `health.set_current(health.max)` |
| Reset state | `leash.rs:57` | `ai_state = Idle` |
| Clear threat **again** | `leash.rs:58` | `threat_list.clear()` |
| Clear cooldowns | `leash.rs:59` | `abilities.clear_all_cooldowns()` |
| Send stats | `leash.rs:79` | entity method **20** |
| Send state field | `leash.rs:83` | entity method **19** |
| Drop movement cache | `leash.rs:89` | `broadcast_movement_type(None)` — emits no wire byte |

**Answers to the owner's specific questions:**

- **How does it path home?** It doesn't. Instant teleport, single tick. There
  is no walk-back, no `nav_path`, no navmesh use. The `fight.rs:199-203`
  comment admits it: *"the leash handler itself snaps the position instantly,
  so even though there's no actual leash-walk yet…"*.
- **Does it restore spawn heading?** **No.** `leash.rs` never touches
  `npc.direction`, and there is no `spawn_dir` read anywhere in the file. The
  NPC lands at its spawn point still facing whatever direction the chase left
  it. **This is the owner's "faces the wrong way" — confirmed as a real
  divergence.**
- **Does it heal?** Yes, to full, instantly, and it clears all cooldowns.
- **Is it immune / re-aggroable en route?** There is no "en route". But the
  2 s window between the transition and the handler is a **damage black
  hole**: `generate_threat`'s preemption list (`threat/aggro.rs:69-76`) is
  `Idle | Patrol | Wander | Investigating | Follow` — **`Leashing` is absent**,
  so damage during that window accumulates into `threat_list` without changing
  state, and then `leash.rs:58` clears the list. Damage dealt in that window is
  silently discarded and does not re-aggro the mob.

### E4. The bug that would surface the instant leash fires

Compare with the respawn path, which is the **correct** reference
implementation (`ticks/npc_respawn/mod.rs:286-320`):

```rust
space_mgr.update_entity_position(entity_id, [pos.x, pos.y, pos.z], [0,0,0], [0.0;3]);
if let Some(npc) = space_mgr.get_entity_mut(entity_id) { npc.direction = spawn_dir; }
// ... then explicit EntityMoved fan-out to every witness
```

Respawn does three things leash does **none** of:

1. goes through `update_entity_position` (keeps the AoI grid + entity
   bookkeeping in sync) — leash writes `npc.position` directly;
2. restores `spawn_dir` at full precision;
3. explicitly fans `EntityMoved` to witnesses **before** the state packets.

Leash sends only methods 20 and 19 — **no position packet at all**. A client
watching a leashing NPC gets a `setMovementType(Leash)` byte, then a stat
update and a state-field update, and **never learns the NPC moved**. It will
keep rendering the mob at the chase-end position until the next AoI sweep
happens to resync it. *That* is "stuck to geometry", and it is why the owner's
recollection of a clean leash return does not match this build.

> **Correction (2026-09-24, NPC AI audit S4):** "no position packet at all" is wrong. `compute_player_aoi` pushes an `EntityMoved` for every entity still in a witness's AoI on every AoI tick, straight from `other.position` (`space_manager/aoi.rs`), so the snap reaches clients within about 100 ms. It arrives as an unreliable avatar update carrying the stale chase velocity, not a forced position. The real defects are that the leash never clears `nav_path` (the movement tick walks the NPC from spawn back out along the leftover chase path), never zeroes velocity, and bypasses the spatial-grid update in `write_position`. See [audit S4](../../npc-ai-restoration/audit.md#3-symptom-4-stuck-partway-frozen-attacking-or-running-in-place) and [ai-aggro-audit.md](../../npc-ai-restoration/evidence/ai-aggro-audit.md). The text above is kept as the playtest record.

**Fix shape:** leash should reuse the respawn snap block verbatim
(`update_entity_position` + `npc.direction = spawn_dir` + `EntityMoved`
fan-out), or — better, matching the owner's memory — become a multi-tick
walk-home that installs a nav path to `spawn_position` and only heals/clears on
arrival. `spawn_dir` is already stored on the entity and already read by
`npc_respawn`, so the heading fix is a two-line change today.

## F. Who issues a second move order, and with what inputs

**The answer to "the first leg looks fine, the second leg floats or goes
backwards" is that `nav_path` is written by seven sites with three different
and mutually inconsistent policies.**

| # | Site | Clears old path? | On `find_path` failure | Sets facing? | Start pos passed |
|---|---|---|---|---|---|
| 1 | `fight.rs:397-416` chase repath | **No** — only assigns when `path.len() > 1` | **Leaves the stale path in place**; logs `no_path` only when `find_path` returns `None` | No | `npc_pos` read at tick top (`fight.rs:102`) |
| 2 | `fight.rs:305-311` cover released (flanked) | `nav_path.clear()`, installs nothing | n/a | No | n/a |
| 3 | `fight.rs:268-292` cover Stay/Move | No direct write — redirects `nav_target_pos`, then falls through to #1 | via #1 | No | via #1 |
| 4 | `fight.rs:448-468` min_range_backup | `clear()` then `push_back(backup)` | no pathfinding at all — **raw straight line** | No | `npc_pos` |
| 5 | `fight.rs:472-474` attack in place | `nav_path.clear()` | n/a | No | n/a |
| 6 | `follow.rs:99-111` follow repath | `clear()` then either routed path or **`push_back(dest)` straight-line fallback** | **silent** — `unwrap_or_default()` | No | `npc_pos` read at `follow.rs:28` |
| 7 | `leash.rs:48` | never touches `nav_path` — teleports | n/a | **No** | n/a |

Three defects fall straight out of this table.

**F1 — `fight.rs` keeps a stale path when the repath produces ≤1 waypoint.**

```rust
if let Some(path) = space_mgr.find_path(npc_id, &npc_pos, &nav_target_pos) {
    if path.len() > 1 {
        /* install */
    }
    // <-- no else. path.len() == 1 falls through silently.
} else {
    /* no_path log */
}
```

When `find_path` succeeds but returns a single waypoint (start and end resolve
to the same navmesh poly — very common once the NPC is close, or once it is
standing off-mesh), **neither branch runs**: no log, no path update, and the
NPC keeps walking its *previous* path toward where the player used to be. That
is literally "walks directly backwards, facing backwards" — the movement tick
is faithfully steering toward a stale waypoint behind the NPC, and
`npc_movement_tick:156` sets `yaw = dx.atan2(dz)` from that stale heading, so
the model faces backwards too. **This is my top candidate for the owner's
second-leg report.**

Session arithmetic is consistent: 50 aggros, 25 `chase`, **0** `no_path`. About
half the aggros produced neither log.

**F2 — the follow fallback launches the NPC into the air, and the second leg
then starts off-mesh.**

`follow.rs:88-98` builds `dest` by scaling the raw NPC→target vector,
**including Y**:

```rust
let dy = target_pos.y - npc_pos.y;
...
let dest = Vector3::new(npc_pos.x + dx*scale, npc_pos.y + dy*scale, npc_pos.z + dz*scale);
```

When `find_path` fails, that unrouted `dest` is pushed as the sole waypoint and
`npc_movement_tick:152` interpolates `new_y = cur_pos.y + dy * t` straight
toward the player's altitude — through the air. **Leg 1 therefore ends with the
NPC airborne and off-navmesh.** Leg 2 calls `find_path(npc_id, &npc_pos, …)`
from that off-mesh start, which fails again, so it compounds: float, float,
float. Exactly "the first move looks fine, then it floats up".

The GM navmesh bypass makes this worse: `movement.navmesh_gm_bypass` fired 26×,
so the tester's own position was frequently off-mesh, guaranteeing the
`find_path` target was unreachable too.

**F3 — nothing on the AI side ever sets facing.** `npc.direction` is written in
exactly two places, both inside `npc_movement_tick`
(`ticks/npc_movement.rs:121` and `:187`), always as a side effect of
translation. No AI handler sets it. So facing is whatever the last *movement
step* implied, and an NPC that stops moving freezes its yaw forever. Note also
`npc_movement_tick:110` — on the final waypoint the code comments *"stopping,
keep current facing"* but actually assigns `dx.atan2(dz)`; if the final leg is
degenerate (`dist == 0`, e.g. a `dest` equal to the current position) that
evaluates to `0.0.atan2(0.0) == 0.0`, snapping the NPC to world-north
regardless of where its target is.

**Single-choke-point fix:** introduce
`issue_move_order(npc_id, dest, move_reason, space_mgr) -> LegOutcome` in
`npc_ai/mod.rs` that *always* clears `nav_path`, installs either the routed
path or an explicit failure, bumps a per-entity `nav_leg_seq`, stamps
`nav_move_reason`, and emits one log. Route all seven sites through it. That
removes F1 and the follow/fight policy divergence in one change, and gives
section C its correlator for free.

## G. Facing arc, LoS gate, and "stops attacking but keeps aggro"

**There is no facing/arc gate anywhere in the NPC attack decision, and no
turn-in-place branch anywhere in the AI.** The attack gate is exactly two
predicates (`fight.rs:239-241`):

```rust
let in_range = dist_to_target <= max_range;
let has_los  = space_mgr.has_line_of_sight(npc_id, target_id);
```

`has_line_of_sight` (`space_manager/spatial.rs:15-37`) is a **navmesh raycast
between the two entity positions** — a pure geometry occlusion test. It is
orientation-independent: it does not know or care which way the NPC faces. It
also **fails open**: no space info, no navmesh, or missing entity all return
`true` (lines 18, 22, 26, 30, 34).

So the owner's "faces away → stops attacking → keeps threat" is **not** a
facing gate; facing is a *symptom* sharing a cause with the stop. The real
sequence is:

1. The NPC's `nav_path` is stale or degenerate (F1/F2), so the movement tick
   steers it away from the player and sets its yaw to that wrong heading.
2. Now displaced, `!in_range || !has_los` becomes true (`fight.rs:354`).
3. It enters the repath block — and `needs_repath` (`fight.rs:380-395`) is
   computed as `last_wp.distance_to(&nav_target_pos) > 5.0`. If the stale
   path's last waypoint is still within 5 units of the target, **`needs_repath`
   is false**, so the whole `if needs_repath { … }` body is skipped and
   `fight.rs:436` returns.

**That return path emits no log of any kind.** It is the non-stationary twin of
`stationary_holds`, and it has zero instrumentation. Threat is untouched, so
the mob stays `Fighting` and aggroed forever while doing nothing.

**Which `decision_outcome`?** *None* — and that is the bug. It is **not**
`stationary_holds`: that branch (`fight.rs:355-378`) is gated on
`is_stationary`, which is a template flag for turrets/fixed defenders. Tonight's
17 `stationary_holds` were 2 NPCs only — 100152 (11) and 100124 (6) — genuinely
stationary mobs, plus one attack. Ordinary guards can never reach it.

Corroborating negative: **zero** `"NPC AI: attack tick produced no ability
fire"` warns (`fight.rs:521`) all session. The full WARN inventory is
`movement.speed_warning` 756, fallback-ability-tree 47,
`movement.navmesh_gm_bypass` 26, one WSTRING decode. So `handle_use_ability`
never rejected a launch — every NPC that *reached* the attack branch fired.
The NPCs that appeared frozen never reached it.

**Recommended behavior change (needs owner sign-off):** add an explicit
`face_target` step — when `in_range && !has_los` or when the NPC is idle in
`Fighting` with an empty `nav_path`, set `npc.direction` toward the target and
broadcast it, rather than leaving yaw as movement residue. That is a genuine
behavior addition, not a bug fix, so it belongs in a chapter amendment.

## C (REVISED). Concrete logging spec for diagnosable playtests

Design rules, per `docs/architecture/observability.md`,
`instrumentation-discipline.md`, and `negative-logging-convention.md`:

- **One event per NPC per AI tick, unconditionally.** Every branch terminates in
  exactly one emission. No silent returns.
- **Correlator:** two new `CellEntity` fields — `nav_leg_seq: u64` (monotonic
  per entity, bumped on every nav_path install) and
  `nav_move_reason: &'static str`. Both are stamped by `issue_move_order` (§F)
  and echoed by the movement tick, so an AI decision and its resulting movement
  legs join on `(npc_id, leg_seq)`. This is the shared correlator the sibling
  agent's per-leg event should carry.
- **Level discipline:** DEBUG for steady-state outcomes; INFO for state
  transitions; WARN only for expectation-unmet seams.

### C1. `npc_ai.decision` — promote the span to the primary record

File: `npc_ai/dispatch.rs:70-81`. Add to the existing span, recorded by
`record_decision_outcome` so every handler fills them:

```
npc_id, ai_state, space_id, world_name, template_id, npc_name,
decision_outcome,                      // ALWAYS set — no empty slot
npc_x, npc_y, npc_z, npc_yaw,
target_id, target_x, target_y, target_z,
dist_to_target, dist_to_spawn, spawn_x, spawn_y, spawn_z,
has_los, in_range, min_range, max_range,
threat_top, threat_count,
nav_path_len, leg_seq, move_reason,
aggression, is_stationary, use_cover, move_speed,
follow_target_id, follow_min_distance, follow_max_distance
```

Make `record_decision_outcome` (`npc_ai/mod.rs:90`) emit a
`tracing::debug!(target: "npc_ai", event = "decision", …)` in addition to the
span record + counter, and make **`fight.rs` call it** at all 10 of its
outcome sites instead of inlining the field. This single change makes the
`decision_outcome` vocab queryable as logs *and* metrics for the first time.

### C2. New `decision_outcome` vocab (add to the observability.md enum table)

| Outcome | Level | Site | Why it must exist |
|---|---|---|---|
| `hold_no_repath` | **DEBUG** | `fight.rs:436` — the `needs_repath == false` return | §G's silent frozen-mob state. Highest-value new event. |
| `repath_degenerate` | **WARN** | `fight.rs:398` — `find_path` returned `Some(path)` with `len <= 1` | §F1 stale-path bug. Fields: `path_len`, `kept_stale_path=true`, `stale_wp_x/y/z`. Currently 0 logs for ~25 aggros. |
| `follow_no_path` | **WARN** | `follow.rs:99` — the `unwrap_or_default()` seam | §F2. Fields: `dest_x/y/z`, `straight_line_fallback=true`, `navmesh_loaded`. The 54/54 wall-clipping. |
| `follow_target_lost` | **WARN** | `follow.rs:50-58` | Permanently strands escorts (Marsh). `reason="entity_not_found"`. |
| `follow_dropped_no_target` | DEBUG | `follow.rs:42-48` | Distinguishes chain-clear from target-loss. |
| `auto_aggro_none` | DEBUG | `fight.rs:57` else-branch | The **why-not-aggro** seam. Fields: `witnesses_scanned`, `rejected_same_faction`, `rejected_dead`, `nearest_dist`. |
| `idle_not_ticked` | DEBUG, 1-in-50 | `dispatch.rs:45-60` filter | Records that an Idle NPC was excluded. Fields: `aggression, has_patrol, has_wander`. |
| `leash_snap` | **INFO** | `leash.rs:48` | Fields: `from_x/y/z`, `to_x/y/z`, `yaw_before`, `yaw_after`, `spawn_dir_restored` (false today), `witnesses_notified` (0 today). Makes §E4 visible. |
| `fight_idle_reset` | **INFO** | `fight.rs:128-143` | Fields: `npc_x/y/z`, `spawn_x/y/z`, `dist_from_spawn`, `returned_home=false`. Currently DEBUG with only `npc_id`. |
| `cover_skipped` | DEBUG, once per Fighting entry | `fight.rs:256` gate | Fields: `use_cover, is_stationary, in_range`. Explains 9,353 unused cover nodes. |

### C3. `npc_ai.move_order` — new event at the single choke point

Target `npc_ai`, level **INFO** (low volume: once per leg, not per tick), in
the new `issue_move_order` helper:

```
npc_id, leg_seq, move_reason,          // chase | cover_advance | min_range_backup
                                       // | leash_return | follow_close | patrol_leg
                                       // | wander_leg | investigate_leg
from_x/y/z, to_x/y/z, requested_x/y/z,
path_len, routed (bool), straight_line_fallback (bool),
prior_path_len, prior_path_cleared (bool),
start_on_navmesh (bool), dest_on_navmesh (bool),
dist, move_speed, est_legs
```

`start_on_navmesh` / `dest_on_navmesh` use the existing
`SpaceManager::is_position_valid` (`spatial.rs:54`) and are what turn §F2 from
a three-hour log-arithmetic exercise into a single query.

### C4. Movement-tick echo (coordinate with the sibling agent)

`ticks/npc_movement.rs` already emits `waypoint_reached` (always) and `step`
(1-in-10 by `npc_id`, `NPC_STEP_LOG_SAMPLE = 10`). Two changes:

- Add `leg_seq`, `move_reason`, `yaw`, `npc_x/y/z` to **both** events so they
  join to `npc_ai.move_order` and `npc_ai.decision`.
- The `npc_id.is_multiple_of(10)` sampling is **not** a 10% sample — it is a
  fixed 10% *of NPCs*, chosen by id. All 40 step rows tonight came from ids
  100170 and 100140. Zerutska (100112) and the first run's Marsh (100122) could
  never emit a step event no matter what they did; the Jaffa run's Marsh
  (100150) was inside the sample and still logged zero steps. Replace with a per-leg counter so every
  NPC gets its first N steps of each leg.

### C5. Spawn-time behavior dump

`spawner/npcs.rs` already logs `name`, `tag`, `world`, `spawn_id`, `npc_id`,
`space_id`. Add the resolved behavior flags — `template_id`, `aggression`,
`use_cover`, `is_stationary`, `move_speed`, `respawn_secs`,
`follow_min/max_distance`, `patrol_len`, `wander_radius`, `ability_ids.len()`.
Every "why did this NPC behave that way" question currently needs a DB
round-trip; this makes the seed self-documenting in telemetry and would have
answered the `aggression == 0` question in one query instead of a schema read.

### C6. Startup navmesh inventory

Target `cell.startup`, INFO, one line per space: `world_name`, `space_id`,
`navmesh_loaded`, `nav_file`, `poly_count`, `component_count`. `find_path` and
`has_line_of_sight` both **fail open / fail silent** when no navmesh is present
(`spatial.rs:26`, `:49`), so "no navmesh" is currently indistinguishable from
"clear line of sight everywhere" in the logs. The cover loader already sets the
precedent with `Loaded cover nodes count=9353 skipped_height=0 …`.
