//! Per-ability range, min-range backup, retry-on-failure coverage, plus
//! the `handle_use_ability`-returns-false negative-logging guard.
//!
//! The AI tick previously used a flat `NPC_ATTACK_RANGE = 30.0` for the
//! "am I in range" check, ignoring each ability's own `max_range` /
//! `min_range`. Three regressions the issue identified:
//!   1. Ability with `max_range < 30` (e.g. grenade 15) — NPC walks
//!      into the flat-30 ring but can't actually fire; mob looks stuck.
//!   2. Ability with `min_range > 0` (e.g. sniper 5) — target inside
//!      the dead zone never gets fired on; mob stops in place.
//!   3. `handle_use_ability` launch-failure (target died between range
//!      check and launch, animation lock) — mob waits a full 2-second
//!      AI tick to retry, visible as a dead frame.
//!
//! The guards below pin all three fixes plus the `max_range = 0`
//! fallback to `NPC_ATTACK_RANGE` so prior NPCs (no def or `max_range`
//! = 0 sentinel) keep working at the flat 30.

use super::{make_ai_fixture, seed_default_ability, seed_target_with_threat};
use cimmeria_entity::stats::HEALTH;
use tokio::sync::mpsc;

// ──────────────────────────────────────────────────────────────────────
// Negative-logging regression guard: handle_use_ability returning false.
//
// The bug shape: NPC AI ticks but the ability never fires (mob picked
// an ability that the pre-consume guard then rejected on cooldown, no
// ammo, or any other reason). Historically this was silent — the mob
// would appear stuck. The guard pins WARN level + reason field so removing
// either trips the test.
// ──────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn npc_ai_fight_warns_when_handle_use_ability_returns_false() {
    use crate::test_support::LogCapture;
    use tracing::Level;

    // Stage: NPC in Fighting with a live in-range player on the threat
    // list. The NPC has NO ability_defs loaded at all, so
    // `handle_use_ability` will reject the call ("ability missing")
    // and return false — exactly the rejection shape the WARN guards.
    let mut mgr = make_ai_fixture([0.0; 3], [10.0, 0.0, 0.0]);
    mgr.create_entity(100, "Castle", [11.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(p) = mgr.get_entity_mut(100) {
        p.is_player = true;
        p.player_id = Some(1);
        if let Some(h) = p.stats.get_mut(HEALTH) {
            h.update(0, 100, 100);
            h.clear_dirty();
        }
    }
    if let Some(npc) = mgr.get_entity_mut(200) {
        npc.threat_list.insert(100, 1.0);
        // Stage the rejection shape that triggers the WARN:
        // - `npc.abilities` is empty (no `add_ability` call).
        // - `choose_npc_ability` documents an empty-bucket fallback to
        //   NPC_DEFAULT_ABILITY ("so a misconfigured template doesn't
        //   wedge silently") — so the selector returns Some.
        // - `handle_use_ability` then rejects on the
        //   `entity.abilities.has_ability(ability_id)` check (the
        //   entity doesn't know the fallback ability) and returns
        //   false. That false-return is the seam the new WARN guards.
    }

    let capture = LogCapture::install();
    let (tx, _rx) = mpsc::channel(8);
    crate::cell::service::npc_ai::npc_ai_tick(
        &tx,
        &mut mgr,
        &cimmeria_content_engine::chain::ChainEngine::new(),
    )
    .await;

    assert!(
        capture
            .find_event(
                Level::WARN,
                "NPC AI: attack tick produced no ability fire",
                "handle_use_ability_returned_false"
            )
            .is_some(),
        "negative-logging convention: NPC AI must WARN when handle_use_ability returns false; \
         reverting to bare `.await` (ignoring the bool) breaks mob-stuck \
         diagnosability. Captured: {:#?}",
        capture.all()
    );
}

/// Per-ability max_range gates the in-range fire path. With
/// `max_range = 15`, a target at distance 20 must NOT trigger the
/// fire path (no ON_ABILITY_TIMER / ON_SEQUENCE in the message
/// stream) — instead the NPC repaths toward the target.
///
/// The bug shape this guards: the flat-30 constant lets the NPC fire
/// at distance 20 even though the ability's own `max_range = 15`
/// would refuse — `use_ability` rejects, NPC stands there.
#[tokio::test]
async fn npc_ai_fight_with_short_max_range_holds_fire_when_target_outside() {
    let mut mgr = make_ai_fixture([0.0; 3], [0.0; 3]);
    seed_default_ability(&mut mgr, /* min */ 0, /* max */ 15);
    // Target at distance 20 — past the ability's 15 but inside the old
    // flat-30 constant. Spawn close enough to spawn point (within
    // LEASH_DISTANCE = 50) that the leash check doesn't fire.
    seed_target_with_threat(&mut mgr, 200, 100, [20.0, 0.0, 0.0]);
    if let Some(npc) = mgr.get_entity_mut(200) {
        npc.abilities
            .add_ability(crate::cell::combat::NPC_DEFAULT_ABILITY);
    }

    let (tx, _rx) = mpsc::channel(16);
    crate::cell::service::npc_ai::npc_ai_tick(
        &tx,
        &mut mgr,
        &cimmeria_content_engine::chain::ChainEngine::new(),
    )
    .await;

    // Post-fix the AI's own range gate keeps the NPC out of the fire
    // path entirely — observable via the side effects:
    //   - ability cooldown is NOT started (use_ability never ran)
    //   - `ai_retry_at` is None (no launch-failure retry was scheduled
    //     by the launch-failure retry hook because handle_use_ability
    //     never returned false — it was never called).
    //
    // Pre-fix the flat-30 constant let in_range = true at distance 20,
    // use_ability ran, its own range check rejected (max_range = 15),
    // and the negative-logging WARN + retry-schedule path fired.
    // So `ai_retry_at == Some(...)` would be the pre-fix signature
    // here. The None assertion below is the canary.
    let npc = mgr.get_entity(200).unwrap();
    assert!(
        !npc.abilities
            .is_on_cooldown(crate::cell::combat::NPC_DEFAULT_ABILITY),
        "per-ability range gate: NPC with max_range=15 must NOT fire at distance 20 \
         — pre-fix flat-30 entered the fire path and started the cooldown despite \
         use_ability rejecting. Cooldown started means we shipped the bug shape."
    );
    assert!(
        npc.ai_retry_at.is_none(),
        "out-of-range gate must not schedule a retry — it's not a launch failure. \
         A `Some(...)` here indicates the AI entered the fire path (pre-fix shape) \
         and handle_use_ability returned false."
    );
}

/// Same setup but target at distance 14: now inside `max_range = 15`,
/// the fire path runs. Pairs with the previous test to prove the gate
/// is bidirectional — not "always reject."
#[tokio::test]
async fn npc_ai_fight_with_short_max_range_fires_when_target_inside() {
    let mut mgr = make_ai_fixture([0.0; 3], [0.0; 3]);
    seed_default_ability(&mut mgr, /* min */ 0, /* max */ 15);
    seed_target_with_threat(&mut mgr, 200, 100, [14.0, 0.0, 0.0]);
    if let Some(npc) = mgr.get_entity_mut(200) {
        npc.abilities
            .add_ability(crate::cell::combat::NPC_DEFAULT_ABILITY);
    }

    let (tx, _rx) = mpsc::channel(64);
    crate::cell::service::npc_ai::npc_ai_tick(
        &tx,
        &mut mgr,
        &cimmeria_content_engine::chain::ChainEngine::new(),
    )
    .await;

    // Successful fire signature: handle_use_ability started the
    // ability's cooldown (it ran end-to-end without rejection).
    let npc = mgr.get_entity(200).unwrap();
    assert!(
        npc.abilities
            .is_on_cooldown(crate::cell::combat::NPC_DEFAULT_ABILITY),
        "per-ability range gate: NPC with max_range=15 must enter the fire path at distance \
         14 (cooldown is started end-to-end). If this trips post-fix, the AI \
         range gate is over-tight and refusing in-range fires."
    );
}

/// `max_range = 0` is the "use server default" sentinel — the AI must
/// fall back to `NPC_ATTACK_RANGE = 30.0` so legacy NPCs without a
/// per-ability range continue to work. Without this regression guard,
/// a future refactor that takes `max_range` literally would
/// silently freeze every legacy NPC.
#[tokio::test]
async fn npc_ai_fight_max_range_zero_falls_back_to_npc_attack_range() {
    let mut mgr = make_ai_fixture([0.0; 3], [0.0; 3]);
    // max_range = 0 → fall back to combat::NPC_ATTACK_RANGE (30.0).
    seed_default_ability(&mut mgr, /* min */ 0, /* max */ 0);
    // Target at distance 25 — outside `max_range = 0` (if taken
    // literally), but inside the 30.0 fallback.
    seed_target_with_threat(&mut mgr, 200, 100, [25.0, 0.0, 0.0]);
    if let Some(npc) = mgr.get_entity_mut(200) {
        npc.abilities
            .add_ability(crate::cell::combat::NPC_DEFAULT_ABILITY);
    }

    let (tx, _rx) = mpsc::channel(64);
    crate::cell::service::npc_ai::npc_ai_tick(
        &tx,
        &mut mgr,
        &cimmeria_content_engine::chain::ChainEngine::new(),
    )
    .await;

    let npc = mgr.get_entity(200).unwrap();
    assert!(
        npc.abilities
            .is_on_cooldown(crate::cell::combat::NPC_DEFAULT_ABILITY),
        "max_range=0 sentinel regression: ability must fall back to \
         NPC_ATTACK_RANGE (30.0); a target at distance 25 must still enter \
         the fire path and start the cooldown end-to-end"
    );
}

/// Target inside the ability's `min_range` triggers a backup
/// waypoint: the NPC's `nav_path` gets a single waypoint at
/// `min_range + 1.0` from the target along the target→NPC vector.
/// Sniper-style abilities work this way — the AI must step out of
/// the dead zone before firing.
#[tokio::test]
async fn npc_ai_fight_target_inside_min_range_schedules_backup_waypoint() {
    use cimmeria_common::Vector3;

    // Spawn NPC well inside the navmesh so the leash check doesn't
    // fire and the in-range/LOS check passes the standard 30 default
    // from the seeded ability's max_range.
    let mut mgr = make_ai_fixture([0.0; 3], [3.0, 0.0, 0.0]);
    seed_default_ability(&mut mgr, /* min */ 5, /* max */ 30);
    // Target at the origin: NPC at (3,0,0), target at (0,0,0) →
    // distance 3, inside `min_range = 5`.
    seed_target_with_threat(&mut mgr, 200, 100, [0.0; 3]);
    if let Some(npc) = mgr.get_entity_mut(200) {
        npc.abilities
            .add_ability(crate::cell::combat::NPC_DEFAULT_ABILITY);
    }

    let (tx, _rx) = mpsc::channel(16);
    crate::cell::service::npc_ai::npc_ai_tick(
        &tx,
        &mut mgr,
        &cimmeria_content_engine::chain::ChainEngine::new(),
    )
    .await;

    let npc = mgr.get_entity(200).unwrap();
    assert_eq!(
        npc.nav_path.len(),
        1,
        "min-range backup must enqueue exactly one waypoint"
    );
    let backup = npc.nav_path[0];
    let dist_from_target = Vector3::new(0.0, 0.0, 0.0).distance_to(&backup);
    assert!(
        dist_from_target >= 6.0,
        "backup waypoint must be at distance >= min_range + 1.0 from the \
         target (min_range = 5, expected >= 6.0, got {dist_from_target})"
    );
    // The backup vector should point AWAY from the target — the NPC
    // is at (3,0,0) and the target is at the origin, so the backup
    // must have positive x (continuing in the NPC's away-from-target
    // direction).
    assert!(
        backup.x > 0.0,
        "backup waypoint must continue along the target→NPC direction \
         (target at origin, NPC at +x — backup must also be +x): got {backup:?}"
    );
}

/// `handle_use_ability` returning `false` schedules `ai_retry_at` to
/// approximately `now + 500ms` (the
/// `AI_LAUNCH_FAILURE_RETRY_DELAY` constant). The retry-sweep tick
/// (every 100ms from `message_loop`) consumes the slot when the
/// deadline passes and re-runs the fight pass. The guard pins:
///
///   - `ai_retry_at` becomes `Some(...)` on launch failure.
///   - The scheduled deadline lands in `[now + 400ms, now + 600ms]`
///     (loose bounds to absorb scheduling jitter and the cost of the
///     tick itself).
///
/// Reverting either the retry-schedule branch or the constant would
/// trip this guard. The actual sweep-consumes-deadline behavior is
/// covered by `npc_ai_retry_sweep_processes_due_npc_and_clears_slot`.
#[tokio::test]
async fn npc_ai_fight_schedules_retry_on_handle_use_ability_failure() {
    // Same setup as `npc_ai_fight_warns_when_handle_use_ability_returns_false`:
    // empty known_abilities + missing ability_defs → choose_npc_ability
    // returns NPC_DEFAULT_ABILITY via the empty-bucket fallback, then
    // handle_use_ability rejects on the has_ability check and returns
    // false.
    let mut mgr = make_ai_fixture([0.0; 3], [10.0, 0.0, 0.0]);
    mgr.create_entity(100, "Castle", [11.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(p) = mgr.get_entity_mut(100) {
        p.is_player = true;
        p.player_id = Some(1);
        if let Some(h) = p.stats.get_mut(HEALTH) {
            h.update(0, 100, 100);
            h.clear_dirty();
        }
    }
    if let Some(npc) = mgr.get_entity_mut(200) {
        npc.threat_list.insert(100, 1.0);
        // `ai_retry_at` starts None; the fight pass must populate it.
        assert!(npc.ai_retry_at.is_none());
    }

    let before = std::time::Instant::now();
    let (tx, _rx) = mpsc::channel(16);
    crate::cell::service::npc_ai::npc_ai_tick(
        &tx,
        &mut mgr,
        &cimmeria_content_engine::chain::ChainEngine::new(),
    )
    .await;
    let after = std::time::Instant::now();

    let retry_at = mgr.get_entity(200).unwrap().ai_retry_at.expect(
        "launch-failure retry hook: handle_use_ability returning false must schedule a \
             retry via ai_retry_at — without this the NPC waits up to 2s for \
             the natural cadence and the player sees a 'dead frame' stutter",
    );

    // Loose bounds — the constant is 500ms; jitter from the tick body
    // can push the observed deadline a few ms in either direction.
    let min_deadline = before + std::time::Duration::from_millis(400);
    let max_deadline = after + std::time::Duration::from_millis(600);
    assert!(
        retry_at >= min_deadline,
        "retry deadline must be at least 400ms after the tick fired (got {retry_at:?})"
    );
    assert!(
        retry_at <= max_deadline,
        "retry deadline must be at most 600ms after the tick returned (got {retry_at:?})"
    );
}

/// `npc_ai_retry_sweep` is what actually consumes the `ai_retry_at`
/// slot — runs every AoI tick (100ms) and fires `npc_ai_fight` on
/// NPCs whose deadline has passed. The slot must be cleared even if
/// the re-fired fight pass fails again (otherwise the same NPC would
/// keep re-firing every tick).
///
/// Setup: NPC with `ai_retry_at = Some(past)` and the same
/// launch-failure shape as the previous test. After the sweep:
///   - `ai_retry_at` is either None OR a fresh deadline (>= now)
///     because the re-fired fight also failed and rescheduled.
#[tokio::test]
async fn npc_ai_retry_sweep_processes_due_npc_and_clears_slot() {
    let mut mgr = make_ai_fixture([0.0; 3], [10.0, 0.0, 0.0]);
    mgr.create_entity(100, "Castle", [11.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(p) = mgr.get_entity_mut(100) {
        p.is_player = true;
        p.player_id = Some(1);
        if let Some(h) = p.stats.get_mut(HEALTH) {
            h.update(0, 100, 100);
            h.clear_dirty();
        }
    }
    if let Some(npc) = mgr.get_entity_mut(200) {
        npc.threat_list.insert(100, 1.0);
        // Backdate the deadline so the sweep treats this NPC as due
        // immediately, no real wall-clock delay required.
        npc.ai_retry_at = Some(std::time::Instant::now() - std::time::Duration::from_millis(10));
    }
    // The sweep iterates `pending_ai_retries` (not all NPCs), so the
    // test must mirror the schedule-side bookkeeping by inserting
    // here. In production the `npc_ai_fight` failure branch handles
    // this; here we bypass that branch and set the field directly,
    // so we owe the set update.
    mgr.pending_ai_retries.insert(200);

    let (tx, _rx) = mpsc::channel(16);
    crate::cell::service::npc_ai::npc_ai_retry_sweep(
        &tx,
        &mut mgr,
        &cimmeria_content_engine::chain::ChainEngine::new(),
    )
    .await;

    // The sweep clears the slot BEFORE calling npc_ai_fight; if the
    // re-fired fight failed (it will, same has_ability rejection),
    // the fight pass MAY have re-set ai_retry_at to a future deadline.
    // Either way the original past-due deadline must be gone.
    let post = mgr.get_entity(200).unwrap().ai_retry_at;
    assert!(
        post.is_none_or(|t| t > std::time::Instant::now()),
        "retry sweep must clear the past-due ai_retry_at slot \
         (either to None or to a fresh future deadline from another retry \
         schedule); leaving the past-due value would re-fire the fight pass \
         every 100ms AoI tick. Got: {post:?}"
    );
}

/// Missing `AbilityDef` (entity has the ability id in `known_abilities`
/// but `space_mgr.ability_defs` has no entry for it) falls back to
/// `NPC_ATTACK_RANGE`. Distinct from `max_range = 0` — that exercise
/// is `max_range_zero_falls_back_to_npc_attack_range`. This one
/// exercises the `None` branch of `chosen_ability.and_then(|id|
/// space_mgr.ability_defs.get(&id))`.
///
/// Bug shape this guards: a future "fail closed on missing def"
/// refactor would silently freeze every NPC whose ability isn't in
/// the def cache (e.g. server starts before def loader finishes).
/// Pre-#329 had no per-ability path, so this case literally couldn't
/// regress; post-#329 it's a real branch.
#[tokio::test]
async fn npc_ai_fight_missing_ability_def_falls_back_to_npc_attack_range() {
    let mut mgr = make_ai_fixture([0.0; 3], [0.0; 3]);
    // Deliberately DO NOT seed an AbilityDef. The NPC knows the
    // ability id (added below), but `space_mgr.ability_defs.get(...)`
    // returns None — exercising the missing-def fallback path.
    seed_target_with_threat(&mut mgr, 200, 100, [25.0, 0.0, 0.0]);
    if let Some(npc) = mgr.get_entity_mut(200) {
        npc.abilities
            .add_ability(crate::cell::combat::NPC_DEFAULT_ABILITY);
    }

    let (tx, _rx) = mpsc::channel(64);
    crate::cell::service::npc_ai::npc_ai_tick(
        &tx,
        &mut mgr,
        &cimmeria_content_engine::chain::ChainEngine::new(),
    )
    .await;

    // With no def, `ability_ranges` returns `(NPC_ATTACK_RANGE,
    // 0.0)`. Distance 25 < 30 = in_range = true → fire path. But
    // `handle_use_ability` ALSO needs the def for its own range
    // check; it shares the same fallback so the cooldown should
    // still start.
    let npc = mgr.get_entity(200).unwrap();
    assert!(
        npc.abilities
            .is_on_cooldown(crate::cell::combat::NPC_DEFAULT_ABILITY),
        "missing-def regression: ability with no AbilityDef in the cache \
         must fall back to NPC_ATTACK_RANGE (30.0); target at distance 25 \
         must still enter the fire path and start the cooldown end-to-end"
    );
}

/// `compute_backup_waypoint` returns `None` when NPC and target share
/// a position — the target→NPC vector has no direction to step back
/// along. Caller treats `None` as "no backup possible, fall through";
/// pinning this avoids a future "use a default direction" refactor
/// that would silently teleport the NPC to (0, 0, 0) or similar.
///
/// Pure unit test on the helper — no SpaceManager needed.
#[test]
fn compute_backup_waypoint_returns_none_for_co_located_target() {
    use cimmeria_common::Vector3;

    // We're calling a private fn from outside its module via the
    // sibling tests directory — only legal because Rust allows
    // `super`-relative paths inside test modules. If this assertion
    // shape ever changes, expose a thin wrapper rather than going
    // `pub`.
    //
    // Both at the origin. EPSILON guard fires.
    let result = crate::cell::service::npc_ai::compute_backup_waypoint_for_test(
        Vector3::new(0.0, 0.0, 0.0),
        Vector3::new(0.0, 0.0, 0.0),
        5.0,
    );
    assert!(
        result.is_none(),
        "co-located NPC+target must return None — the helper has no \
         direction to back away along. Pre-fix returning a NaN-laden \
         waypoint would corrupt nav_path and crash the path follower."
    );

    // Sanity: a non-degenerate input still returns Some(...) so the
    // negative-case assertion above isn't a false positive.
    let result = crate::cell::service::npc_ai::compute_backup_waypoint_for_test(
        Vector3::new(3.0, 0.0, 0.0),
        Vector3::new(0.0, 0.0, 0.0),
        5.0,
    );
    assert!(
        result.is_some(),
        "non-degenerate input must produce a Some — verifies the helper \
         isn't always returning None (which would make the co-located \
         assertion trivially pass)"
    );
}

/// Stationary NPC with `is_stationary = true` must NOT receive a
/// min-range backup waypoint even if the target is inside the dead
/// zone. The PR body explicitly carved this out ("Stationary NPCs
/// skip the backup — they're pinned by design"); pin it in code so
/// a future "always back off" refactor doesn't violate the design
/// invariant.
///
/// Mirrors `npc_ai_stationary_does_not_pathfind_when_out_of_range`
/// (the pre-existing nav-skip guard for out-of-range) but for the
/// NEW min-range branch.
#[tokio::test]
async fn npc_ai_fight_stationary_does_not_back_off_inside_min_range() {
    let mut mgr = make_ai_fixture([0.0; 3], [3.0, 0.0, 0.0]);
    seed_default_ability(&mut mgr, /* min */ 5, /* max */ 30);
    seed_target_with_threat(&mut mgr, 200, 100, [0.0; 3]);
    if let Some(npc) = mgr.get_entity_mut(200) {
        npc.abilities
            .add_ability(crate::cell::combat::NPC_DEFAULT_ABILITY);
        // Pin the NPC in place. A non-stationary NPC at this same
        // position would enqueue a backup waypoint — see
        // `npc_ai_fight_target_inside_min_range_schedules_backup_waypoint`.
        npc.is_stationary = true;
    }

    let (tx, _rx) = mpsc::channel(16);
    crate::cell::service::npc_ai::npc_ai_tick(
        &tx,
        &mut mgr,
        &cimmeria_content_engine::chain::ChainEngine::new(),
    )
    .await;

    let npc = mgr.get_entity(200).unwrap();
    assert!(
        npc.nav_path.is_empty(),
        "stationary NPC must NOT enqueue a min-range backup waypoint — \
         turrets / fixed defenders are pinned by design. Pre-fix would \
         have populated nav_path here just like a mobile NPC; the \
         is_stationary guard is the canary. Got: {:?}",
        npc.nav_path
    );
}
