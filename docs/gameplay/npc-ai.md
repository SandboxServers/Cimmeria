---
title: "NPC AI System"
type: reference
audience: engineers
last_updated: 2026-09-25
---

# NPC AI System

> **Last updated**: 2026-09-25
> **Status**: All 12 Atrea AI states are now wired in the Rust runtime. Behavior states (Patrol, Wander, Investigating, Follow) are driven by `npc_ai_tick`; terminal states (Despawning, Submit, Error) are reachable via the `SetNpcAiState` content action. Implementation status detail is in the [summary table](#implementation-status-summary) at the bottom; the historical "Python design" sections below are kept for reference but no longer reflect the runtime.

## Overview

NPC mob behavior is driven by a state machine implemented in the Rust cell service (`crates/services/src/cell/service/npc_ai/`). The runtime mirrors the Python `SGWMob` design — every 2 seconds the `npc_ai_tick` snapshot-and-dispatch loop routes each NPC into a per-state handler. Threat events preempt behavior states into `Fighting` with per-state scratch preserved.

The Detour-backed navmesh (via `space_mgr.find_path`) handles pathfinding for all movement states. Movement is interpolated at 100 ms cadence by `npc_movement_tick`.

**Key files (Rust runtime):** `crates/entity/src/cell_entity/mod.rs` (the 12-state `AiState` enum + per-state scratch fields), `crates/services/src/cell/service/npc_ai/` (state-machine dispatch in `dispatch.rs` plus one module per behavior state — `fight.rs`, `fight_target.rs`, `patrol.rs`, `wander.rs`, `follow.rs`, `investigate.rs`, `leash/` (policy, walk home, reset), `lifecycle.rs`, `ability_select.rs`), `crates/services/src/cell/combat/threat/` (`generate_threat` preemption and the leash evade, the default `LEASH_DISTANCE` in `aggro.rs`, the player-side drain in `player_combat.rs`), `crates/services/src/cell/service/ticks/npc_respawn/` (Dead → Idle promotion), `crates/services/src/cell/cover/` (cover selection and reservation). The Python design files referenced in legacy sections (`deprecated/python/cell/SGWMob.py`, `deprecated/python/Atrea/enums.py`) are kept for evidence-of-intent only.

---

## AI State Machine

### State Definitions

Defined in `deprecated/python/Atrea/enums.py` lines 228-239.

| State | Value | Implemented | Notes |
|-------|-------|-------------|-------|
| `AI_STATE_Spawning` | 0 | INERT | Variant preserved for source-enum completeness but the Rust runtime starts every NPC at Idle and never enters this state. Future spawn-VFX hooks (Goa'uld ribbon-device reveal, etc.) can plug in here. |
| `AI_STATE_Idle` | 1 | DONE | Waits for threat or AI tick promotion into Patrol / Wander / auto-aggro. |
| `AI_STATE_Investigating` | 2 | DONE | `npc_ai_investigate` — pathfind to `poi`, dwell 5 s on arrival, return to Idle. Reached via `SetNpcPoi` content action. |
| `AI_STATE_Fighting` | 3 | DONE | Target selection, ability selection, fire. Per-ability range gating + retry-on-launch-failure (see [#329](https://github.com/SandboxServers/Cimmeria/issues/329)). |
| `AI_STATE_Leashing` | 4 | DONE | The NPC walks home on the navmesh, evading, then heals and resets. Entered when the NPC itself goes past its leash radius or loses its last target. See [Leash and reset](#leash-and-reset-na12). |
| `AI_STATE_Dead` | 5 | DONE | Set on death via `combat::mark_npc_dead`. `npc_respawn_tick` (1 Hz) promotes back to Idle when `respawn_at` elapses. |
| `AI_STATE_Despawning` | 6 | DONE | `npc_ai_despawn` removes the entity from the space. Reached via `SetNpcAiState`. |
| `AI_STATE_Follow` | 7 | DONE | `npc_ai_follow` maintains distance band `[follow_min_distance, follow_max_distance]` to a target. Reached via `SetFollowTarget`. |
| `AI_STATE_Patrol` | 8 | DONE | `npc_ai_patrol` walks a loop from `entity_templates.patrol_path_id` → `point_set_points`, dwells on arrival at each waypoint. Threat preemption preserves the index. |
| `AI_STATE_Wander` | 9 | DONE | `npc_ai_wander` samples random points within `wander_radius` of spawn, dwells for `[wander_min_dwell_secs, wander_max_dwell_secs]` between hops. |
| `AI_STATE_Submit` | 10 | DONE | `npc_ai_submit` clears combat state and holds. Reached via `SetNpcAiState`. |
| `AI_STATE_Error` | 11 | DONE | `npc_ai_error` is quiescent — diagnostic fallback. Reached via `SetNpcAiState` or the `enterErrorAIState` slash command. |

### State Transitions

`generate_threat` preempts any non-Dead non-Fighting state to Fighting (with per-state scratch preserved so the post-fight return can re-evaluate). Idle promotion priority is **proximity aggro > patrol > wander**: a hostile NPC scans first and falls through when nobody qualifies (NA13).

```
Idle      -->  Fighting    (generate_threat fires + NPC was Idle / Patrol / Wander / Investigating / Follow)
Idle      -->  Patrol      (npc_ai_tick observes patrol_path non-empty)
Idle      -->  Wander      (npc_ai_tick observes wander_radius > 0 and no patrol_path)
Idle      -->  Investigating (SetNpcPoi content action)
Idle      -->  Follow      (SetFollowTarget content action with a valid target)
Fighting  -->  Leashing    (the NPC is past its leash radius from spawn: leash_out)
Fighting  -->  Leashing    (last target dead / gone / out of AoI for 5 s: target_lost;
                            threat list already empty: threat_empty)
Leashing  -->  Idle        (walked home: leash_arrived; no route or 20 s timeout: leash_snap_fallback)
Any alive -->  Dead        (HP -> 0; combat::mark_npc_dead)
Dead      -->  Idle        (npc_respawn_tick promotes; respawn_at elapsed)
Any alive -->  Despawning / Submit / Error  (SetNpcAiState content action)
```

### Tick Loop

`doAiAction()` is the main AI tick, called on a recurring timer:

```python
def doAiAction(self):
    if self.isDead():
        return
    state = self.AIState
    if state == AI_STATE_Spawning:
        self.doAiSpawnAction()
    elif state == AI_STATE_Idle:
        self.doAiIdleAction()
    elif state == AI_STATE_Fighting:
        self.doAiFightingAction()
    # All other states fall through without action
```

---

## Threat System

### Data Structure

Threat is stored as a plain dict on the mob instance:

```python
self.threat = {}  # entityId (int) -> accumulated threat (float)
```

### Damage-to-Threat Conversion

```python
# In onStatChange() or equivalent damage handler:
threat = -healthChange * 2 - focusChange
self.threatGenerated(attackerEntityId, threat)
```

Health damage is weighted 2x relative to focus damage. The formula is negated because `healthChange` is negative when damage is dealt.

### Threat Accumulation

```python
def threatGenerated(self, entityId, threat):
    if entityId not in self.threat:
        self.threat[entityId] = 0.0
    self.threat[entityId] += threat
    if self.AIState == AI_STATE_Idle and not self.isDead():
        self.AIState = AI_STATE_Fighting
```

### Target Selection

`getTopThreateningEntity()` does a linear scan of the threat dict:

1. Iterates all entries in `self.threat`.
2. Prunes entries where the entity is dead (`entity.isDead()` or entity no longer exists in AoI).
3. Returns the entity ID with the highest accumulated threat value.
4. Returns `None` if the list is empty after pruning, which triggers a transition back to Idle.

**Known issues:**
- Entity ID recycling: a dead entity's ID may be reused by a new entity, which would falsely inherit threat.
- Distance, line-of-sight, and cover are not factored into target selection.
- No threat decay over time.

### Unimplemented Threat Methods

Declared on `SGWMob` but contain no logic: `addDirectToThreatList`, `addBuffToThreatList`, `addHealToThreatList`, `addToThreatList`, `removeFromThreatList`, `onGroupMateEnteredCombat`, `onGroupMateThreatTransfer`.

---

## Aggression System

### Aggression Levels

`EMobAggressionLevel` (`entities/defs/enumerations.xml`). Low is hostile.

| Level | Value | Meaning in the Rust server |
|-------|-------|---------|
| `HOSTILE` | 1 | Attacks on sight: the only level the Idle proximity scan acts on |
| `SUSPICIOUS` | 2 | Not aggressive on sight |
| `NEUTRAL` | 3 | Fights back only when attacked |
| `FRIENDLY` | 4 | Not aggressive on sight |
| `DEFAULT` | 5 | Not aggressive on sight (python compared the stored level against NEUTRAL) |

### Effective aggression (NA13, D-NA01)

An NPC's aggression toward players is its **override** when one is set, otherwise the **faction reaction** of the player's faction toward the NPC's faction. This is python's `SGWPlayer.getAggressionLevel`.

- **Override.** `CellEntity::aggro.override_level` (python `aggressionOverride`). It is seeded from `spawnlist.aggression_override` (1-5, CHECK-constrained) and changed at runtime by the `set_aggression` content action, the `spawn_entity` action's `aggression` parameter, the GM `.aggression` command, and the surrender path (NEUTRAL). Content levels keep their pre-NA13 numbers: `1` was "aggressive" and is HOSTILE; `0` was "passive" and maps to NEUTRAL.
- **Faction reaction.** `FACTION_REACTION_TABLE` (44 x 44) in `enumerations.xml`, identical to `deprecated/python/Atrea/enums.py`, is ported as a constant table in `crates/services/src/cell/combat/faction_reaction.rs`. A unit test re-parses the XML so the two cannot drift. It is a code constant rather than a seed table because it is engine data the client ships, not per-zone content, and the scan reads it every tick. Players react as faction **3** (`Praxis`), the faction every client is told on world entry (`mercury::aoi::PLAYER_FACTION`). A player's server-side `CellEntity::faction` stays 0, which the `faction == 10` damage and right-click gates rely on.

In the seeds, faction 10 (`Straegis`: the NID guards and PRUs) reads HOSTILE, factions 1 and 3 read FRIENDLY, and NULL or 0 reads NEUTRAL.

### Proximity aggro scan

`npc_ai_tick` admits an Idle NPC when it is hostile to players, has a patrol path, or has a wander radius. A hostile Idle NPC runs `npc_ai_idle_auto_aggro` every AI tick (about 2 s) over its AoI witnesses. A witness becomes a candidate only when every gate in `npc_ai/aggro_gates.rs` passes, in this order:

| Gate | Reject reason |
|---|---|
| Is a player | `not_player` |
| Alive | `dead` |
| Not on the NPC's server-side faction | `same_faction` |
| The NPC's effective aggression toward it is HOSTILE | `not_hostile` |
| Not a GM with `.aggro off` set | `gm_ignored` |
| Height difference `abs(dy) <= 4` u (`AGGRO_VERTICAL_BAND`) | `out_of_vertical_band` |
| Horizontal distance within `entity_templates.aggro_radius` (NULL: 18 u, `DEFAULT_AGGRO_RADIUS`) | `out_of_radius` |
| Navmesh line of sight is `Clear` | `no_los` |

The closest candidate gets a 1.0 threat seed (`cause=proximity`). During the NA12 post-reset window every witness is rejected with `post_reset_suppressed`. Rejects are logged as `npc_ai.aggro_scan event=candidate_rejected` with the `reason` above, sampled per NPC and player (NA02).

**Line of sight fails closed for aggro (D-NA08).** An `Unknown` answer, meaning an endpoint the navmesh does not cover, rejects the candidate. The attack check in the fight keeps failing open, so an NPC already fighting does not stop shooting over a mesh hole. A space with no navmesh has nothing to check and passes; there the vertical band is the only storey guard. The navmesh ray cannot see floors or ceilings (audit S15), so the band is also what stops a guard under a ramp from seeing a player on it.

**A guard in cover looks from its peek point (NA23, D-NA12).** An NPC standing at the cover slot it holds, which a guard authored in cover does from spawn, casts the ray from the slot's peek point past its prop, and sees the candidate when that ray or its own is clear. Its own ray alone hits the prop, a navmesh hole, within half a metre: at UAT-1 `Hallway01_Guard` rejected a player 8.1 u in front of its counter as `no_los`. The assist check uses the same rule. See [architecture/cover-system.md decision 9](../architecture/cover-system.md#9-the-peek-point-na23-d-na12).

If the scan finds nobody, a hostile NPC that also has a patrol path or wander radius falls through to it, so faction-derived hostility does not freeze a patroller. A patrolling or wandering NPC does not scan until it is Idle again.

### Same-room assist (NA14, D-NA04)

**A marked deviation from legacy.** The 2009 server had no assist: a mob only took threat from its own attackers, so shooting one of two guards standing side by side left the other watching. The owner approved same-room assist because that reads as broken AI.

When an NPC enters Fighting from damage (`cause=damage`) or proximity aggro (`cause=proximity`), `combat::generate_threat` calls `npc_ai::recruit_assisters` (`npc_ai/assist.rs`). Every NPC in the victim's space on the victim's server-side faction and within twice its own assist radius is considered, and joins only when every gate passes, in this order:

| Gate | Reject reason |
|---|---|
| Alive | `dead` |
| Idle, patrolling or wandering. Fighting, Leashing, Investigating and Follow NPCs are never pulled | `not_idle` |
| Itself HOSTILE to players (override, else faction reaction) | `not_hostile` |
| Not inside its NA12 post-reset window | `post_reset_suppressed` |
| The target is not a GM with `.aggro off` set | `gm_ignored` |
| Height difference to the victim `abs(dy) <= 4` u | `out_of_vertical_band` |
| Horizontal distance to the victim within its own `entity_templates.assist_radius` (NULL: 10 u, `DEFAULT_ASSIST_RADIUS`) | `out_of_radius` |
| Navmesh line of sight from it to the victim is `Clear` (`Unknown` fails closed where a mesh exists, as for aggro) | `no_los` |

A joining NPC takes a 1.0 threat seed on the victim's target and enters Fighting with `cause=assist` (transition `reason=assist`). Only a live player target recruits.

- **No chaining.** An assister enters Fighting with `cause=assist`, which does not recruit, so a fight never ripples from room to room: in a line of three guards 7 u apart, shooting the first pulls the second but not the third.
- **Content threat does not recruit.** A chain's `generate_threat` keeps a scripted fight exactly as scripted.
- **Chain-armed spawns are safe.** Spawns 10 and 20 are seeded NEUTRAL (below), so they fail `not_hostile` and wait for their chain.
- In Castle Cellblock the MessHall guards (spawns 28 and 29, 7.2 u apart, same room) assist each other; the Hallway guards are 18.6 u or more apart and do not.

Telemetry: the join is `npc_ai.aggro event=acquired cause=assist` (counter `npc_ai_aggro_total{cause="assist"}`), plus a DEBUG `npc_ai.aggro_scan event=assist_joined` that names the `victim_id`. A considered neighbour that was passed over logs `npc_ai.aggro_scan event=assist_rejected` with the `reason` above, sampled per assister and victim.

### Chain-armed spawns (D-NA01a)

A spawn whose fight a content chain must start carries `aggression_override = 3` (NEUTRAL), and the chain runs `set_aggression 1` plus `generate_threat`:

| Spawn | Tag | Chain |
|---|---|---|
| 20 | `ArmYourself_NIDGuard` | 1008 (enter `Castle_CellBlock.Region8`) |
| 10 | `ArmYourself_PrisonerRetrievalUnit` | 1032 (Ambernol vial interaction) |

No other Castle (world 8) or Harset seed calls `set_aggression`, so no other spawn needs the override. Every other faction-10 spawn now aggroes on sight within its radius: the Cellblock and Castle NID guards and PRUs, `Castle_Romney`, `Castle_Muelbach`, the Castle Bravo officers, and the SGC_W1 Ba'al Jaffa.

### GM switch: `.aggro on|off` (D-NA02)

Mobs aggro onto GMs like any player. `.aggro off` in the GM `.`-console makes the proximity scan skip the caller, `.aggro on` restores it, and `.aggro` alone reports it. It is server-side because the client's ghost or noclip never reaches the server (audit A8). It covers proximity aggro and assist: damage and content threat still engage a GM, but the mob a GM shoots does not pull its neighbours in (NA14). It is keyed by character, survives zone changes and relogs, is lost on a server restart, and is ignored if the character loses GM access.

### Wire: broadcast to witnesses (NA33, D-NA16)

Python's `setAggression` sent `onEntityProperty(GENERICPROPERTY_MobAggression = 6, level)` to the owner and witnesses, but no client handler for property type 6 was ever found — that call was dead on arrival in 2009. `createOnClient` separately sent `onAggressionOverrideUpdate(level)`, once, only when an override was already set at spawn/reconnect time, to the ClientMethod the client handler at `0x00d31bd0` actually reads (stores the INT8 `aAggressionLevel` at `GameMob + 0x16c`; registered through `MemberCallback<GameMob, Event_NetIn_onAggressionOverrideUpdate>`, paired with an `onAggressionOverrideCleared` handler `0x00d31cd0` never called from legacy python at all).

NA33 confirmed the SGWMob flat index (27 for Update, 28 for Cleared — `Lootable` contributes no client methods, so SGWMob's own two begin right after the shared SGWSpawnableEntity/SGWBeing 0-26 prefix) and wired the server to use the ClientMethod, not the dead property, on every path:

- **Runtime change** (content `set_aggression` action, GM `.aggression` command, the surrender/`npc_ai_submit` disarm): broadcasts `onAggressionOverrideUpdate(level)` to every witness, or `onAggressionOverrideCleared` when the override is cleared. This is a deliberate divergence from legacy's literal wire call — see [findings/npc-aggression-broadcast.md](../reverse-engineering/findings/npc-aggression-broadcast.md) for why finishing `createOnClient`'s intent (not `setAggression`'s dead one) is correct and needs no client patch.
- **AoI entry**: replays `onAggressionOverrideUpdate` to a newly-arrived witness when an override is active, mirroring `createOnClient`'s conditional send exactly (a faction-derived, no-override mob still sends nothing).

It remains a display value only: the client derives friend or foe from the faction it is sent, and the aggression level's exact on-screen effect (nameplate color, reticle color, or an interaction verb — `UIAggressionLevel` is registered as a Lua-scriptable enum type alongside `UIArchetype`/`TargetType`/etc.) was not directly observed, since no client Lua source is present in this tree.

### Timed Overrides

```python
def overrideAggression(self, level, entityBase, seconds):
    # Sets aggressionOverride, schedules revert after `seconds`
```

This let scripted events change a mob's stance for a while (for example, a friendly NPC turned hostile during a mission encounter) and revert it afterwards. Not ported.

---

## Ability Selection (Combat AI)

### Classification

`classifyHostileAbility(target, ability)` evaluates a single ability and returns one of:

| Result | Value | Condition |
|--------|-------|-----------|
| `ABILITY_Usable` | 1 | Passes all checks |
| `ABILITY_CoolingDown` | 2 | On cooldown |
| `ABILITY_Filtered` | 3 | Heal, buff, or non-single-target mode |
| `ABILITY_NeedsAmmo` | 4 | No ammo remaining |

Classification logic in order:
1. **Filter heals and buffs** — Abilities that restore health or apply positive effects to self are excluded.
2. **Require single-target mode** — Only `TCM_Single` targeting mode is accepted. AoE and cone abilities return `ABILITY_Filtered`.
3. **Check cooldown** — If the ability's cooldown timer is active, returns `ABILITY_CoolingDown`.
4. **Check ammo** — If the ability requires ammo and the mob has none, returns `ABILITY_NeedsAmmo`.

### Selection Loop

`selectHostileAbility(target)` iterates the mob's ability set:

```python
def selectHostileAbility(self, target):
    needs_ammo = []
    for ability in self.getAbilities():
        result = self.classifyHostileAbility(target, ability)
        if result == ABILITY_Usable:
            return ability       # First usable ability wins
        elif result == ABILITY_NeedsAmmo:
            needs_ammo.append(ability)
    if needs_ammo:
        self.triggerReload()    # All blocked by ammo: reload
    return None
```

There is no priority weighting — the first usable ability in iteration order is selected. No distance checks, no cooldown preference, no situational logic (e.g., prefer ranged when target is far).

### Rust: reach-filtered selection

The Rust selector lives in `crates/services/src/cell/service/npc_ai/ability_select.rs`
and diverges from the loop above in two ways.

**Deterministic order.** `choose_npc_ability` sorts the known ability ids
ascending and takes the first one off cooldown. The original returned
`usable[0]` out of a CPython 2 dict, so its order was a hash artefact; the
original developers left `# TODO: Which ability should we use? Use some
weighting/rand here.` directly above that line, so lowest-id is a Cimmeria
convention rather than a divergence from a contract. The practical
consequence for a content author is that **the lowest-id member of an
ability set is the NPC's primary attack** and the rest are cooldown
fallbacks.

**A distance gate the original never had.** `classifyHostileAbility` carries
the literal comment `# TODO: Check distance, LOS` above its
`return ABILITY_Usable`. `choose_npc_ability_within_reach` is that TODO:

- An `is_ranged = false` ability is only selectable when the target is
  inside `NPC_MELEE_RANGE` (3 m). Without the gate a melee auto-attack
  resolves to the 30 m ranged default, because `abilities.max_range` is the
  `0` "use the server default" sentinel on every auto-attack in the seed and
  `is_ranged` is otherwise read only by `calculate_qr` to pick the
  accuracy/defence branch. The visible defect is an NPC playing a staff or
  ribbon *swing* at a target thirty metres away.
- When nothing is in reach it falls back to the unfiltered pick rather than
  returning `None`. `None` is the caller's "all cooling, hold fire" signal,
  so returning it would freeze a melee-only NPC at distance. Handing back
  the out-of-reach ability lets `ability_ranges` report its real 3 m
  `max_range`, and the fight tick's existing out-of-range arm then does the
  right thing: a mobile NPC chases and swings once it arrives, a stationary
  one lands in `stationary_holds` and turns to face its target.

`NPC_MELEE_RANGE` is derived from the weapon table rather than invented.
`resources.items` carries `min_melee_range` / `max_melee_range` alongside
the ranged pair; across every item binding an `EVENT_ITEM_MELEE` ability the
melee maximum is 0, 2 or 3, so 3 is the largest reach any shipped weapon
expresses. `NPC_ATTACK_RANGE` (30) is the same table's dominant
`max_ranged_range`. Both constants live in
`crates/services/src/cell/combat/threat/aggro.rs`.

Not ported: the `ABILITY_Filtered` pre-pass. Cimmeria's selector does not
reject heals, buffs or non-`TCM_Single` abilities before partitioning. That
is inert while every NPC ability set holds only single-target hostile
auto-attacks, but it is a live hazard for the first content author who puts
a self-buff or an AoE into a mob set.

### Combat Tick

`doAiFightingAction()` runs each combat tick:

1. Call `getTopThreateningEntity()`. If `None`, set `AIState = AI_STATE_Idle` and return.
2. Call `lookAt(target)` to rotate the mob toward the target.
3. Call `selectHostileAbility(target)`. If an ability is returned, launch it.
4. If no ability is available (all on cooldown or no ammo), schedule a 0.5-second retry.

---

## Ammo Management

Mobs use the same `bandolier_items` / `Stat[AMMO_SLOT_1+slot]` model as players in principle. In practice the **Rust port skips the ammo gate for non-players**: the fire-gate in [`crates/services/src/cell/abilities/mod.rs:259-263`](../../crates/services/src/cell/abilities/mod.rs#L259) short-circuits with `entity.is_player && current_ammo < required_ammo`, so NPCs currently fire without consuming rounds and never need to reload. `triggerReload()` is not yet ported.

Legacy accessors and their Rust equivalents:

| Legacy (`SGWMob.py` / `SGWPlayer.py`) | Rust equivalent |
|----------------------------------------|-----------------|
| `getAmmoStat()` — stat ID for current slot | `crate::stats::AMMO_SLOT_1 + entity.active_bandolier_slot` |
| `getClipSize()` — max ammo from equipped weapon | [`CellEntity::active_clip_size()`](../../crates/entity/src/cell_entity/bandolier.rs#L19) |
| `getAmmoCount()` — current ammo | [`CellEntity::active_ammo()`](../../crates/entity/src/cell_entity/bandolier.rs#L12) |
| `consumeAmmo(amount)` | [`CellEntity::set_slot_ammo(slot, current - amount)`](../../crates/entity/src/cell_entity/bandolier.rs#L36) |
| `triggerReload()` | Not ported for NPCs (player path: [`handle_reload`](../../crates/services/src/cell/cell_methods/player/world/reload.rs#L71)) |

Legacy behavior: on spawn (`doAiSpawnAction`), the mob called `getClipSize()` on its equipped weapon and set its ammo stat to that value, representing a full reload at spawn. When `selectHostileAbility` found all abilities blocked by ammo, it called `triggerReload()`. The reload completed after a delay and refilled the clip, allowing the combat loop to resume.

If/when NPC reload is needed, the same machinery applies — but **all three** of the following are required together; partial work will silently leave NPCs stuck mid-reload:

1. Drop the `is_player` short-circuit in the fire-gate ([`abilities.rs`](../../crates/services/src/cell/abilities/mod.rs)).
2. Set `reload_complete_at` from an AI-driven path (an NPC equivalent of `requestReload`).
3. **Widen `reload_completion_tick`** ([`service.rs:610`](../../crates/services/src/cell/service.rs#L610)) — it currently iterates `space_mgr.all_player_entity_ids()` only, so an NPC's deadline would never be promoted. Add an `all_reloadable_entity_ids()` accessor or extend the existing one to include fighting NPCs.

See [weapon-ammo-reload.md](weapon-ammo-reload.md) for the full ammo and reload model.

---

## Tapping System (Kill Credit)

These properties are defined in `SGWMob.def` but have no Python implementation:

| Property | Type | Purpose |
|----------|------|---------|
| `tappedEntity` | INT32 | Entity ID with loot and XP rights |
| `tappedSquad` | INT32 | Squad ID with loot and XP rights |
| `tappedSquadMembers` | ARRAY<INT32> | Individual members of the tapped squad |

Tapping determines who receives loot drops and XP when the mob dies. Currently, loot generation on death runs without any tap check — all loot goes to whoever triggered the death event.

---

## Mob Properties Reference

Key properties from `SGWMob.def` (55 total), grouped by subsystem:

**Controller IDs** (C++ controller handles, stored as INT32):
`navControllerID`, `visionID`, `yawID`, `behaviorTimerID`, `despawnTimerID`, `investigateTimerID`, `grenadeDetectorID`, `trackControllerID`, `targetOverrideTimer`

**AI State:**
`AIState`, `POI` (VECTOR3 — investigate destination), `Home` (VECTOR3 — spawn/leash anchor), `lastNavigate`, `stateLock`, `stateChanges`, `stateHistory`, `disableBehaviorSystem`, `nextWanderTime`

**Combat:**
`MyAbilitySetID`, `LootTableID`, `Aggression`, `minIdealRange`, `maxIdealRange`, `isKillable`, `isTrackable`, `isWorthXP`

**Cover:**
`bCoverFromTarget`, `useCover`, `reservedCoverNode`, `CombatStance`

**Following:**
`currentlyFollowing`, `followTarget`, `followMinDistance`, `followMaxDistance`, `followAngle`, `followMovementType`

**Patrol:**
`patrolPaths` (dict), `currentPatrolPath`, `patrolMovementType`

**Hearing:**
`hearingRadius`

**Despawn:**
`despawnFlag`, `despawnTimerID`, `spawnTime`, `decayTimerID`

**Behavior Events:**
`mobBehaviorEventSet`

---

## Unimplemented States: Reconstruction Notes

### Leash and reset (NA12)

Implemented behaviour, decided in D-NA03 (corrected by D-NA10) of the [NPC AI restoration ledger](../analysis/npc-ai-restoration/README.md). Code: `crates/services/src/cell/service/npc_ai/leash/` and `fight_target.rs`.

**When an NPC gives up.** The leash is measured on the NPC's own horizontal distance from its spawn, never on the target's. The radius is `entity_templates.leash_distance`, or 50 u when the column is NULL (the seed sets none yet).

- Beyond the radius plus a 5 u hysteresis band, or more than 20 u above or below its spawn, the NPC always gives up (`trigger` `beyond_band` / `vertical_cap`).
- Inside the band it keeps fighting a target it can already hit. It gives up only if it would have to chase further from home (`chase_outward`).
- A target that dies, disconnects, or stays beyond the NPC's AoI radius for 5 s is dropped. When nobody is left, the NPC goes home (`target_lost`).
- A player who dies leaves every NPC's threat list at the moment of death (`abilities::death::resolve_death` → `purge_dead_player_from_threat`), and a target carrying `BSF_DEAD` counts as dead whatever its HEALTH reads. A dead player cannot `useItem` (answered with `onErrorCode(0, 0, CONDITION_FEEDBACK_NotLiving)`, the legacy `@mustBeAlive` reply), so a medkit in the Defeat Window can no longer revive the killer's interest (NA24).

The old metric was spawn-to-target in 3D. A player standing 49.9 u from the Cellblock Guard's spawn bounded every chase, and an aggressive NPC standing at its spawn leashed every 6 s against a player 60 u away (audit S3, S5).

**On giving up.** The NPC is removed from every player that lists it in `threatened_mobs`, so `BSF_InCombat` clears, the player is sent `onStateFieldUpdate` and regen resumes. Its threat list is cleared, cover is released, and `find_path(npc, spawn)` installs the route home.

**Walking home.** The movement tick walks the route. The client sees the walk from position and velocity only, because no server-to-client movement-type message exists (D-NA10). While Leashing the NPC evades: `generate_threat` refuses it, so damage adds no threat and does not put the attacker into combat (`npc_ai.leash event=damage_ignored`, DEBUG).

**Arrival.** Within 1.5 u of spawn with its route finished, the NPC heals to full, faces its authored spawn heading, clears its cooldowns and goes Idle. For 5 s afterwards the Idle auto-aggro scan ignores players.

**Snap fallback.** When no route can be planned, or the walk takes longer than 20 s, the NPC snaps to spawn through the grid-updating writer and resets the same way (`reason=leash_snap_fallback`). A follower (`follow_target_id` set) is reset where it stands and is not moved.

**Telemetry.** The leash reports through NA02's detectors (`npc_ai::detectors::leash`): `npc_ai.leash event=enter` (reason, trigger, `nav_path_len`, `npc_to_spawn`), `event=arrived` / `event=snap_fallback` (`arrival`, `walk_secs`, `snap_dist`), `event=loop` (should stay silent) and `event=damage_ignored`. The leash itself adds `event=replan` and `event=player_combat_exit`. NA02's `threat event=cleared_without_exit` and `npc_ai.idle_parked` should stay silent through a leash; tests pin both. The fight's `decision_outcome=leashed` row carries `trigger`, `npc_to_spawn` and `target_to_spawn`.

### Chase and unreachable targets (NA15)

Code: `crates/services/src/cell/service/npc_ai/chase/`. The Fighting handler hands a mobile NPC that is out of range or out of line of sight to the chase step.

**Stop distance.** A chase routes to the target moved `max(ability min_range, 1.0 u)` toward the NPC, capped at the ability's `max_range`. The walk ends short of the target, never inside it. Before NA15 a guard walked to the player's own point and stood 0.35-0.7 u from it (audit S10). A cover slot chosen by the cover step is routed as given.

**Repath.** The NPC keeps its route while the goal stays within 5 u horizontally and 1.5 u vertically of the goal the route was planned for (`decision_outcome=hold_no_repath`). A player walking down a ramp toward the NPC changes level quickly and gets a new route. The old test was 5 u in 3D against the last waypoint.

**Unreachable target.** When the target is on another mesh island, Detour returns a partial route. The NPC walks it to the island edge and then holds there, facing the target with zero velocity, and does not request a new route until the target moves (`decision_outcome=hold_unreachable`). After 8 s of holding it gives up and walks home (`npc_ai.transition reason=unreachable`, `decision_outcome=leashed trigger=unreachable`). A route that reaches the target, or an attack, resets the timer. A partial route *home* is walked to its end, and then the NPC snaps to spawn (`npc_ai.leash arrival=snap_partial_route`).

**Off-mesh start.** When the pathfinder cannot start from where the NPC stands (`no_start_poly`: hovering, sunk or a step off the mesh), the NPC is snapped onto the nearest polygon within 2 u horizontally and Â±4 u vertically, and the route is requested once more (`npc_ai.path event=off_mesh_snap`, `npc_ai.path_fail fallback=snapped_to_mesh`). With no polygon that close it goes home, and the leash tick snaps it to spawn.

**Off-mesh target.** A target the destination box cannot place (`no_end_poly`, typically a GM standing on unmeshed props, audit S14) is replaced by the nearest on-mesh point within 8 u horizontally and Â±4 u vertically (`fallback=nearest_on_mesh`). The route cannot reach the target, so the NPC holds at its end if it still cannot hit from there.

**Degenerate repath.** A route that comes back as a single point clears the stale route (`fallback=path_cleared`), and the NPC holds.

### Investigating (State 2)

A mob heard a noise or detected suspicious movement but has not confirmed a threat. It should navigate to `POI`, look around for a set duration, and return to `Home` if nothing is found.

Evidence: `POI` (VECTOR3) property holds the destination, `investigateTimerID` stores a C++ timer controller, `hearingRadius` controls detection range, `onNoise()` is a declared cell method that would set `POI` and transition to this state.

### Leashing (State 4)

The mob's current target has moved beyond pursuit range or out of LOS. The mob abandons the fight and returns to `Home`, clearing its threat list on arrival.

Evidence: `Home` (VECTOR3) stores the spawn anchor, `maxIdealRange` defines engagement distance. Standard MMO pattern: if distance to Home exceeds `maxIdealRange * 2`, cancel pursuit, navigate Home, clear `self.threat`, transition to Idle.

### Patrol (State 8)

The mob follows a scripted waypoint path between spawn locations.

Evidence: `patrolPaths` (dict) stores one or more named path definitions, `currentPatrolPath` tracks the active path index, `patrolMovementType` controls speed/animation. DB columns `patrol_path_id` and `patrol_point_delay` in `entity_templates`. C++ methods `startPatrol(path, delay)` and `cancelPatrol()` are declared on the cell entity.

### Wander (State 9)

Random movement within a radius of `Home`. The mob picks a random nearby point, navigates there, waits a random delay, then picks another point.

Evidence: `Home` property provides the anchor, `nextWanderTime` property stores the timestamp of the next wander move. Recommend: use `findPathTo()` with a randomly offset position from `Home`, bounded by `minIdealRange`.

### Follow (State 7)

The mob maintains a set distance and angle behind a target entity (used by pets and escort NPCs).

Evidence: `currentlyFollowing` (bool), `followTarget` (entity reference), `followMinDistance`, `followMaxDistance`, `followAngle`, `followMovementType` properties all defined in `.def`.

### Submit (State 10)

A controlled shutdown state for mobs that surrender rather than fight to the death (e.g., scripted encounters). The mob stops fighting and signals completion to the mission system before despawning.

Evidence: state is defined in the enum; no supporting properties are uniquely tied to this state.

### Error (State 11)

A diagnostic recovery state for when the AI reaches an inconsistent condition.

Evidence: Cell methods `enterErrorAIState()` and `leaveErrorAIState()` are declared. Properties `errorStateReason`, `errorStateDescription`, `errorAIState`, and `errorTime` are defined for logging the failure context.

### Despawning (State 6)

Controlled removal of the mob from the world, distinct from death. Allows animations and cleanup to complete before the entity is destroyed.

Evidence: `despawnFlag` (bool) property, `despawnTimerID` controller, `DespawnWhenFree()` cell method, `decayTimerID` for corpse removal after death.

---

## Navigation Integration

The C++ cell layer exposes these navigation methods to Python:

| Method | Purpose |
|--------|---------|
| `findPathTo(position)` | Compute and begin moving along a navmesh path |
| `findDetailedPathTo(position)` | Higher-fidelity path with full waypoint list |
| `addWaypoint(position)` | Append a waypoint to the current path |
| `cancelMovement()` | Stop all movement immediately |

None of these are called by the current Python mob AI. The `navControllerID` property is reserved for a C++ navigation controller that is never created. The only movement primitive used is `lookAt(target)`, which rotates the mob's yaw toward a target entity without translating.

The practical result is that all mobs are stationary during combat. They rotate to face their target and fire, but do not close distance, retreat to cover, or reposition.

### Rust: line of sight in the fight tick

The Python mob AI never checked line of sight: `classifyHostileAbility`
says `# TODO: Check distance, LOS`, and `AbilityManager` says
`# TODO: Do LOS checks on target`. The client still has the feedback codes
`CONDITION_FEEDBACK_LOS = 39` and `CONDITION_FEEDBACK_NoLOS = 40`, and the
GM surface has `testLOS`, `onLOSResult` and `toggleCombatLOS`. So the
shipped server probably did check it.

In a world without a `.occ` file, Cimmeria's occlusion source is the
navmesh raycast (`NavMesh::line_of_sight`), which returns `Clear`,
`Blocked` or `Unknown`. Every shipped client world has one now; see
[collision-geometry line of sight](#rust-collision-geometry-line-of-sight-na27).
It has no heights for the holes Recast cuts around furniture, so it cannot
tell a waist-high desk from a wall. Against the extracted Cellblock
collision geometry, at 1.5 m eye heights on one storey, 45% of its
`Blocked` verdicts were false and 0.15% of its `Clear` verdicts were
false.

The fight tick (`npc_ai/fight.rs`) calls
`SpaceManager::attack_line_of_sight`:

| Attacker | `Clear` | `Unknown` (endpoint off the mesh) | `Blocked` |
|---|---|---|---|
| Mobile NPC | fires | fires | paths toward the target |
| Mobile NPC at its cover slot (NA23) | fires | fires | the verdict is from the slot's peek point past the prop, or the NPC's own ray if that one is clear; `Blocked` holds fire (`cover_no_shot`) and gives the slot up after 3 s |
| Stationary NPC (`spawnlist.is_stationary`) | fires | fires | fires if the target is within 4 u of its height, otherwise holds (`stationary_holds`) |

A stationary NPC cannot walk around the obstacle, so a false `Blocked`
would silence it for the whole fight. This is what happened to the Find
Ambernol drone at the med-station desk (NPC AI restoration NA16, audit
S11, decision D-NA11). The cost is that a turret that is already fighting
can shoot through a real wall on its own storey.

The `npc_ai.tick` row carries both the navmesh verdict (`los`) and the rule
that acted on it (`los_policy`: `strict`, `stationary`,
`stationary_relaxed`, `stationary_other_storey`, or `cover_peek` for a
mobile NPC standing at its cover slot, NA23). A drone firing across
the desk logs `los=blocked los_policy=stationary_relaxed`; a guard in cover
fires on `los=clear los_policy=cover_peek` and holds on
`los=blocked los_policy=cover_peek`. NA22's `in_cover_slot` fired from a
slot whatever the verdict, and a guard shot a player through two walls
(UAT-1). Where a world ships an occluder, these navmesh rules are replaced
(NA27, below).

Ability launch (`use_ability/handle.rs`) checks range only. An NPC's line
of sight is checked by the fight tick in the same tick, just before the
launch. Players still get no line-of-sight check at fire time.

### Rust: collision-geometry line of sight (NA27)

A world that ships `data/spaces/<world>.occ` answers every NPC
line-of-sight question from its collision geometry, eye to eye at 1.5 m
(`space_manager/occlusion.rs`, decision D-NA13). The navmesh workarounds
above do not apply there:

| Check | With an occluder |
|---|---|
| Idle aggro scan and assist | the occluder verdict; `Unknown` (outside the trimmed explorable area) rejects `no_los` (D-NA08) |
| Fight tick attack check | `los_policy=occluder`: `Clear` fires, `Blocked` holds (a mobile NPC paths toward the target). No stationary relaxation (D-NA11). `Unknown` takes the navmesh rules above. |
| NPC at a cover slot | looks from its own eyes over the prop; no peek point (D-NA12) |

On the Castle_CellBlock and Castle sweeps it had no false clears, and
about 1% of truly clear pairs read blocked, mostly rays grazing a wall
edge. The Find Ambernol drone fires over the med-station desk, and
`Hallway01_Guard` sees over its counter to 13.6 u. `Hallway02_Guard` does
not see through the hallway walls. `npc_ai.los` rows say
`source=occluder` with `eye_height_used = 1.5`. The file is paged: only the
64 m pages near players are unpacked (`npc_ai.occluder event=residency`).
Details in
[the NA27 worknote](../analysis/npc-ai-restoration/worknotes/na27-occluder-phase1.md).

---

## Cover System

Cover nodes are spatial graph nodes placed in the world that provide defensive bonuses. The design supported mobs finding and reserving cover positions before or during combat.

Relevant properties: `useCover` (bool), `bCoverFromTarget` (bool direction flag), `CombatStance` (enum), `reservedCoverNode` (node reference).

Relevant cell methods: `onReserveCoverSlot()`.

The Python reference implemented none of this. The Rust server does (NA22); the design is in [architecture/cover-system.md](../architecture/cover-system.md). In short:

- **Who:** `entity_templates.use_cover`, or a hostile (`faction = 10`) NPC when it is NULL. Stationary NPCs, props and melee-only NPCs never take cover.
- **Spawned in cover:** an NPC authored within 1.5 u of a cover marker spawns holding that slot and keeps it while its target is in front of the cover and in range.
- **Seeking cover:** in a fight, an NPC takes the best free slot that reaches its target (within attack range less 2 u), whether or not it already has a shot. With a shot it walks at most 10 u, and after a seek that finds nothing it waits 4 s before looking again.
- **In cover:** on reaching the slot the NPC stops with zero velocity, gains Cover Stance (ability 1451, +100 `COVER_DEFENSE`), and fires from the slot without chasing.
- **Sight from cover (NA23):** an NPC at its slot looks from the slot's peek point past its prop, for aggro, assist and the shot alike. A wall past the cover still blocks. With no line it holds fire, and after 3 s gives the slot up. A slot is only picked if the NPC would have a shot from it.
- **Leaving:** the slot and the stance go when the target flanks the cover (20 degrees past side-on, NA23) or leaves attack range, after 3 s with no shot, and on leash, death or surrender. A slot left as flanked, blind or unreachable is not re-taken by the same NPC for 6 s.
- **Pose:** there is no server-to-client pose message. Whether the client crouches an NPC standing at a marker is an open owner experiment.

`CombatStance` is still set but not acted upon.

---

## Behavior Event System (Not Implemented)

`mobBehaviorEventSet` stores one or more named event sets that define data-driven behavior triggers. The design intent appears to be a table-driven system where events (e.g., "player enters radius", "health drops below 50%") trigger scripted responses (e.g., bark dialog, call for help, switch ability set).

Cell methods `addBehaviorSet(name)` and `removeBehaviorSet(name)` are declared for runtime modification of the active event sets. No behavior set logic is implemented.

---

## Mob Groups (Not Implemented)

`mobGroup` property and `mobJoinGroup()` cell method are defined for coordinating multiple mobs as a unit. This would enable pack behavior (all members assist when one is attacked), coordinated patrol paths, and shared threat lists. No group logic is implemented.

---

## Implementation Status Summary

| Feature | Status | Notes |
|---------|--------|-------|
| State machine tick loop | DONE | `doAiAction()` dispatches by state |
| Spawning state | DONE | Loads ammo, transitions to Idle |
| Idle state | DONE | Waits for `threatGenerated()` |
| Fighting state | DONE | Target selection, ability fire, 0.5s retry |
| Dead state | DONE | Loop exits cleanly |
| Threat accumulation | DONE | Damage -> threat formula, Idle->Fighting transition |
| Top-threat targeting | DONE | Linear scan with dead-entity pruning |
| Ability classification | DONE | Type/targeting/cooldown/ammo checks |
| Ammo management | DONE | Load on spawn, consume per shot, auto-reload |
| Combat exit | DONE | Threat empty -> Leashing (walk home) -> Idle, with the player-side combat drain. Python went Idle in place; NA12 diverges on purpose because Rust NPCs move (D-NA03). |
| Loot on death | DONE | Loot table referenced, no tap check |
| Aggression override | DONE (broadcast; timed revert still unported) | NA13: override (seed `spawnlist.aggression_override`, content, console), else the faction reaction. NA33: broadcast to witnesses on change and replayed on AoI entry. No timed revert (`overrideAggression`'s scheduled-revert helper, not ported); see [Wire: broadcast to witnesses](#wire-broadcast-to-witnesses-na33-d-na16). |
| lookAt() rotation | DONE | Mob faces target during combat |
| Leashing state | DONE | NA12: NPC-to-spawn leash radius with hysteresis and a per-template `leash_distance`, walk home with evade, heal / facing / cooldown reset on arrival, snap only as a fallback, player combat drained, 5 s re-aggro suppression. See [Leash and reset](#leash-and-reset-na12). |
| Chase path robustness | DONE | NA15: stop distance, level-aware repath, hold then give up at a partial route, partial route home, off-mesh start and target recovery, degenerate repath clears the route. See [Chase and unreachable targets](#chase-and-unreachable-targets-na15). |
| Proactive aggro detection | DONE | NA13: hostile Idle NPCs (override, else faction reaction) scan witnesses every 2 s through the radius (18 u default, `entity_templates.aggro_radius`), vertical band (4 u), fail-closed LoS and GM-switch gates, and seed 1.0 threat on the closest. See [Proximity aggro scan](#proximity-aggro-scan). |
| Navigation (findPathTo) | DONE | Detour FFI behind `space_mgr.find_path()` + `npc_movement_tick` consumes `nav_path` waypoints at 100 ms. See [#35](https://github.com/SandboxServers/Cimmeria/issues/35). |
| Per-ability range | DONE | `ability_ranges()` reads each ability's `min_range`/`max_range` from defs; fight tick gates on the chosen ability rather than a flat 30 m. See [#329](https://github.com/SandboxServers/Cimmeria/issues/329). |
| Three-bucket ability selection | DONE | `choose_npc_ability` partitions known abilities into usable / cooling / needs-ammo and picks the first off-cooldown ID. See [#342](https://github.com/SandboxServers/Cimmeria/issues/342). |
| Multi-ability sets | DONE | `ability_set_abilities` is keyed on `(ability_set_id, ability_id)`, so one set holds N abilities and the selector walks them all in ascending id order. Harset packet H09 widened the key and gave set 4 the staff pair (`584` ranged + `710` melee) and set 5 the ribbon pair (`711` melee + `712` ranged). |
| Melee reach gate | DONE | `choose_npc_ability_within_reach` will not select an `is_ranged = false` ability for a target beyond `NPC_MELEE_RANGE` (3 m), so an NPC never plays a weapon swing at a target it cannot touch. When nothing is in reach it falls back to the unfiltered pick, whose short `max_range` sends a mobile NPC down the chase arm and a stationary one into `stationary_holds`. See the section below. |
| `setMovementType` AoI broadcast | DONE | `broadcast_movement_type` fans the EMobMovementType byte to AoI witnesses on every state transition (CombatAdvance on Fighting entry, Leash on Leashing entry, clear on Idle). Dedup'd against `last_movement_type` so re-entry of same state is a wire no-op. Closes [#270](https://github.com/SandboxServers/Cimmeria/issues/270). |
| NPC respawn | DONE | `npc_respawn_tick` (1 Hz) reads `respawn_secs` (COALESCE `spawnlist`, `entity_templates`, minimum 3s enforced via CHECK). On NPC death the `combat::mark_npc_dead` helper stamps `respawn_at = now + respawn_secs`. Tick promotes Dead → Idle, restores HP / FOCUS / state / interaction-type / facing direction, snaps position to spawn, closes any open loot UIs on still-looting players, and broadcasts in wire order: EntityMoved → INTERACTION_TYPE → ON_STATE_FIELD_UPDATE → ON_STAT_UPDATE. `NULL` columns → one-shot mob (corpse persists). Effect-script-driven HP-to-0 paths that bypass `damage_apply` (e.g., `scripts::MeleeDamage`) also bypass respawn — future content using those paths must call `combat::mark_npc_dead` explicitly. |
| Investigating state | DONE | `npc_ai_investigate` handler routes the NPC to a content-set `poi`, dwells 5s (`INVESTIGATE_DWELL_SECS`), returns to Idle. Reached via the `SetNpcPoi` content action; the `onNoise` cell-method hook for in-game audio is deferred. |
| Patrol state | DONE | `npc_ai_patrol` walks the loop from `entity_templates.patrol_path_id` → `point_set_points`. Dwells `patrol_point_delay` at each waypoint. Threat preemption preserves `patrol_next_index` so the post-fight return resumes the route. |
| Wander state | DONE | `npc_ai_wander` samples a random point within `wander_radius` of `spawn_position`, validates against the navmesh, dwells a random duration in `[wander_min_dwell_secs, wander_max_dwell_secs]`. Off-mesh candidates fall back to `spawn_position`. |
| Follow state | DONE | `npc_ai_follow` maintains a distance band `[follow_min_distance, follow_max_distance]` to the target. Out of band → pathfind toward target; below min → hold (no back-away). Reached via the `SetFollowTarget` content action. A `being`-class follower (Col Marsh, template 10) is ticked too: `ai_driven_npc_entity_ids` admits a `being` in Follow / Patrol / Wander / Investigating / Despawning / Submit / Error, never in Idle (props) or Fighting (NA24). |
| Submit state | DONE | `npc_ai_submit` clears combat state (threat_list, BSF_IN_COMBAT, movement-type cache) and holds. Reached via the `SetNpcAiState` content action. |
| Error state | DONE | `npc_ai_error` is a quiescent diagnostic state — handler is a no-op per tick. Reached via the `SetNpcAiState` content action or the `enterErrorAIState` slash command. |
| Despawning state | DONE | `npc_ai_despawn` removes the entity from the space on entry; AoI fires the leave events to witnesses. Reached via the `SetNpcAiState` content action. |
| Cover system | DONE | `crates/services/src/cell/cover/` — world-space markers per world (NA21, `cover_extract`), uniform-grid spatial index, slot reservation with auto-release-prior semantics, and node scoring. NA22: `use_cover` from `entity_templates.use_cover`, the spawn hold, the in-range seek of a slot that reaches the target, arrival stop + Cover Stance (ability 1451), release (and stance removal) on flank, out of range, leash, death and surrender. See [architecture/cover-system.md](../architecture/cover-system.md). Pose on the client is unconfirmed. |
| NPC movement speed | PARTIAL | `move_speed` is a hardcoded `0.6` units per 100 ms tick (6 units/sec) set at construction (`crates/entity/src/cell_entity/construction.rs:84`) and never varied by AI state. `npc_movement_tick` reads it verbatim. So although the `EMobMovementType` byte broadcast to witnesses does change per state (Patrol vs CombatAdvance vs Leash), every NPC actually traverses at the same speed — the client plays a different gait animation over identical server-side motion. The seed data has distinct per-world speeds (`resources.worlds.walk_speed` ≈ 2.069, `run_speed` = 8.125) that nothing reads for NPCs. |
| Mob group coordination | NOT IMPL | mobGroup property, mobJoinGroup() declared; deferred. |
| Behavior event sets | NOT IMPL | addBehaviorSet/removeBehaviorSet declared; deferred. |
| Tapping (kill credit) | DONE | Content-engine kill chains supersede the Python tap design. |
| Group mate threat assist | NOT IMPL | Methods declared, no logic. |
