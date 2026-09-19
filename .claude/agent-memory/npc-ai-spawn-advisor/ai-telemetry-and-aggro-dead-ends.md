---
name: ai-telemetry-and-aggro-dead-ends
description: decision_outcome telemetry is split into two mutually invisible halves; proximity aggro is structurally impossible (no aggression column in seed); leash predicate measures spawn->target not spawn->NPC
metadata:
  type: project
---

> **Status 2026-09-19 — a dated snapshot.** These thirteen findings were verified against the
> 2026-09-18 playtest and code; several fixes merged the next day (#677, #680, #682, #709).
> Re-verified since: item 10 (leash is a raw field write) is **still true**. Item 12's "no
> turn-in-place branch" is **superseded** — `face_target` landed in #682 — though its point that no
> facing/arc gate exists in the attack decision was not re-checked. Item 5's Castle example is
> affected by `castle.nav` shipping in #709. The rest were not re-verified, and every line number is
> as of 2026-09-18. Confirm against the code before acting on any of them.

Verified against the 2026-09-18 colo playtest telemetry (session 23:55–01:35 UTC).

**1. `decision_outcome` has two disjoint, mutually invisible halves.**
`npc_ai/dispatch.rs:70-81` declares the `npc_ai.decision` span with an empty
`decision_outcome` field. `fight.rs` fills it as an **inline log field** on 10
`tracing::info!/debug!` sites (187, 273, 286, 298, 368, 408, 426, 457, 484,
497) — queryable as logs, **never increments `npc_ai_decisions_total`**. Every
other handler (`follow`/`patrol`/`wander`/`investigate`/`lifecycle`) calls
`record_decision_outcome()` (`npc_ai/mod.rs:90`) — span field + counter,
**emits no log at all**. So "zero `follow_band` / patrol / wander outcomes in
the logs" is a measurement artifact, never evidence of absence. Only
`attack_in_place`, `chase`, `stationary_holds`, `leashed`, `no_path`,
`move_to_cover`, `stay_in_cover`, `cover_released_flanked`, `min_range_backup`,
`no_ability` are log-queryable.

**2. Proximity / auto-aggro is structurally dead.**
`db/resources/Worlds/Seed/spawnlist.sql` has **no `aggression` column** at all
(cols: `spawn_id, x, y, z, heading, world_id, template_id, tag, set_name`), and
`entity_templates.sql` has none either. So `aggression == 0` everywhere, and
`dispatch.rs:59` excludes every Idle NPC from the tick snapshot before any
handler runs. `npc_ai_idle_auto_aggro` fired **zero** times in a 100-minute
session with 50 aggros — all 50 came from the player shooting first via
`generate_threat`. There is also **no assist/social aggro path anywhere**:
`threat/aggro.rs:50-102` only touches the single NPC that was hit, so a guard
standing next to the one you shot can never join. Only the `set_aggression`
content action can turn aggression on.

**3. The leash predicate measures the wrong distance.**
`fight.rs:178-180` computes `spawn.distance_to(&target_pos)` — spawn→**player**,
not spawn→**NPC**. A mob that chased you 300 units won't leash if you circle
back near its spawn; a mob that never moved *will* leash when you walk 50 units
from its spawn. `LEASH_DISTANCE = 50.0` is global
(`combat/threat/aggro.rs:11`), no per-template override. Zero `leashed` events
in the whole session.

Canonical reference has **no leash at all**: `deprecated/python/cell/SGWMob.py`
only assigns `AI_STATE_Spawning` (22), `AI_STATE_Fighting` (161),
`AI_STATE_Idle` (291, 305), `AI_STATE_Dead` (315). `AiState::Leashing` is a
Rust invention. When it does fire (`leash.rs:12-90`) it **teleports** to
`spawn_position` (skipped when `follow_target_id.is_some()`), heals to full,
clears threat + all cooldowns, sends methods 20 and 19.

**4. Nothing else moves an NPC home.** The only Fighting exits are target-dead
(`fight.rs:156`), target-gone (`fight.rs:168`), no-threat (`fight.rs:128`) and
leash. None restore position or heading — only `ticks/npc_respawn` does. There
is **no threat decay and no out-of-AoI disengage**, so a mob you shot and ran
from stays `Fighting` forever, parked where the chase ended.

**5. `follow.rs:99` silently swallows pathfinding failure.**
`find_path(...).unwrap_or_default()` → `path.len() > 1` false → `else` pushes
one raw straight-line waypoint. No log, no counter, no warn (unlike
`fight.rs:423`'s `no_path`). Proven live: npc 100112 (Castle Zerutska escort)
logged **exactly 54 `follow_routed` and exactly 54 `NPC reached waypoint`** — a
1:1 ratio only possible if every leg was the single-waypoint fallback. That is
the "walks through walls and floors" bug.

**6. Two Zerutskas are two seeded spawnlist rows**, not a runtime duplicate:
`Castle_Zuritska_Cell` (npc 100112, the escort) and `Castle_Zuritska_Comms`
(npc 100113, static at the terminal), both live in Castle space 65537 from
boot. Seed spells it *Zuritska*. Nothing despawns either.

**7. Escort survival depends on chain re-arming.** Chain 1302 re-fired
`set_follow_target` on Zerutska 10× in 65 s; chain 1174 fires once for Marsh.
Marsh (`Preparation_ColMarsh`, npc 100122/100150) had `resolved_target Some(..)`
both runs yet produced **zero** AI or movement events — consistent with the
silent `follow.rs:50-58` target-lost branch stranding him permanently, since an
Idle NPC with `aggression == 0` is never ticked again.

**8. Cover loads but is never used by NPCs.** 1,381 sets / 9,353 nodes load
clean at boot (`skipped_height=0, skipped_quality=0, skipped_tail=0`). All
`fire_cover_entered` events are *players* entering cover regions. NPC cover
needs `use_cover && !is_stationary && !in_range` (`fight.rs:256`); no NPC ever
produced a cover decision. Also: `cover_set_id` is always 1381, which equals
the set count — possible last-set/fallback lookup bug.

**9. Respawn works and is exact.** Castle hostiles: `respawn_secs = 120`,
measured death→respawn deltas 120.2–120.9 s. Snaps to `spawn_pos`, clears
`state_field` and `interaction_flags`, restores HP, notifies witnesses. Note
`crates/services/src/cell/combat/threat/player_combat.rs::clear_dead_npc_from_all_player_threat`
now exists — **#92 has landed**, contrary to older notes.

**10. Leash never walks home and never restores heading.** `leash.rs:48` writes
`npc.position = spawn_pos` as a **raw field write** — no `update_entity_position`,
no `EntityMoved` fan-out, and **no `spawn_dir` restore**. It sends only entity
methods 20 (stats) and 19 (state field), so the client never learns the NPC
moved and keeps rendering it at the chase-end position with the chase-end yaw.
`ticks/npc_respawn/mod.rs:286-320` is the correct reference (snap via
`update_entity_position`, restore `spawn_dir`, fan `EntityMoved` *before* the
state packets). Also: `Leashing` is missing from `generate_threat`'s preemption
list (`threat/aggro.rs:69-76`), so damage in the ≤2 s Leashing window
accumulates threat that `leash.rs:58` then discards.

**11. Three inconsistent nav_path write policies cause the "second leg" bug.**
`fight.rs:397-416` only assigns when `path.len() > 1` and has **no else**, so a
`find_path` that returns `Some(path)` with `len == 1` leaves the **stale path
installed** with no log — the NPC keeps walking toward where the player used to
be, and `npc_movement_tick:156` faces it along that stale heading ("walks
backwards facing backwards"). `follow.rs:99-111` instead clears and pushes a
straight-line `dest` whose **Y is interpolated toward the target's altitude**
(`follow.rs:90`), so leg 1 ends airborne/off-mesh and leg 2's `find_path` fails
from an off-mesh start — compounding float. Seven sites write `nav_path`
(`fight.rs` 305/397/448/472, `follow.rs:102`, `leash.rs` none, plus patrol/
wander/investigate); none of them set facing. Fix = one `issue_move_order`
choke point.

**12. No facing/arc gate exists in the attack decision.** The gate is only
`in_range` + `has_los` (`fight.rs:239-241`), and `has_line_of_sight`
(`spatial.rs:15-37`) is an orientation-independent navmesh raycast that **fails
open** (returns `true` with no navmesh). `npc.direction` is written ONLY by
`npc_movement_tick` (`:121`, `:187`) as a side effect of translation — no AI
handler ever sets it, and there is no turn-in-place branch. The "stops
attacking but keeps aggro" state is `fight.rs:436` returning when
`needs_repath == false` — **it emits no log at all** and is the non-stationary
twin of `stationary_holds` (which is gated on the `is_stationary` template flag
and unreachable for ordinary guards).

**13. `NPC_STEP_LOG_SAMPLE` is not a 10% sample.** `npc_movement.rs:171` uses
`npc_id.is_multiple_of(10)` — a fixed 10% *of NPCs* chosen by id parity. NPC
100112 is always sampled; 100122/100150 can never emit a step event.

Related: [[npc-follow-state-gaps]], [[npc-death-credit-and-respawn-gaps]],
[[castle-cellblock-navmesh-components]], [[submit-state-semantics]]
