//! `.bug <free text>` — a tester bookmark.
//!
//! Freezes the server's view of the world around the caller into telemetry at
//! the moment a human says "this looks wrong", so a playtest can be
//! reconstructed without an accompanying chat log or screenshot. See
//! `docs/analysis/playtests/2026-09-18-colo-castle/README.md` §9.2 for the
//! session that motivated it.
//!
//! Two event shapes share a `bookmark_id` correlator:
//!
//! - `playtest.bookmark` — one row: the caller, their selected target, mission
//!   and region state, and the free-text note.
//! - `playtest.bookmark.entity` — one row per nearby entity (NPC or player),
//!   with everything the server believes about it *and what it is telling
//!   clients* (`yaw_byte` is the byte that actually goes on the wire).
//!
//! [`capture`] is pure so tests can assert on the snapshot; [`emit`] is the only
//! place that touches `tracing`.

use std::time::{Instant, SystemTime, UNIX_EPOCH};

use cimmeria_common::Vector3;
use cimmeria_entity::cell_entity::CellEntity;
use cimmeria_entity::stats::{FOCUS, HEALTH};
use tokio::sync::mpsc;

use super::send_gm_feedback;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::mercury::aoi::pack_angle;

/// Entities farther than this from the caller are not captured.
pub(crate) const CAPTURE_RADIUS: f32 = 60.0;
/// Hard cap on per-entity rows, nearest first.
pub(crate) const CAPTURE_MAX_ENTITIES: usize = 32;

/// Everything captured for one entity near the caller.
#[derive(Debug, Clone)]
pub(crate) struct EntitySnapshot {
    pub entity_id: u32,
    pub is_player: bool,
    pub is_selected_target: bool,
    pub name: String,
    pub tag: String,
    pub template_id: i32,
    pub spawn_id: i32,
    pub level: u32,
    pub faction: u8,
    pub alignment: u8,
    pub pos: Vector3,
    pub velocity: [f32; 3],
    pub speed: f32,
    pub is_on_ground: bool,
    /// Facing the server holds, radians. `direction.y` is yaw.
    pub yaw_rad: f32,
    /// The byte `pack_angle` puts on the wire for that yaw — what clients render.
    pub yaw_byte: u8,
    /// Bearing from this entity to the caller, same convention as NPC movement
    /// (`atan2(dx, dz)`).
    pub bearing_to_caller_rad: f32,
    /// Absolute difference between the *transmitted* facing and the bearing to
    /// the caller, degrees, 0..=180. ~180 on a chasing NPC is the
    /// "walks at me backwards" signature.
    pub wire_facing_vs_caller_deg: f32,
    pub dist: f32,
    pub dist_xz: f32,
    /// This entity's Y minus the caller's Y.
    pub dy_vs_caller: f32,
    pub ground_y: Option<f32>,
    pub y_above_ground: Option<f32>,
    pub on_navmesh: bool,
    pub has_los_to_caller: bool,
    pub caller_witnesses_it: bool,
    pub witness_count: usize,
    pub health_cur: i32,
    pub health_max: i32,
    pub state_field: u32,
    pub interaction_type_flags: i64,
    pub ai_state: String,
    pub last_movement_type: String,
    pub nav_path_len: usize,
    pub next_wp: Option<Vector3>,
    pub final_wp: Option<Vector3>,
    pub move_speed: f32,
    pub is_stationary: bool,
    pub use_cover: bool,
    /// Effective `EMobAggressionLevel` toward players (1 = hostile, NA13).
    pub aggression: i32,
    /// The override behind it, `0` when faction-derived.
    pub aggression_override: i32,
    pub threat_count: usize,
    pub threat_top_id: u32,
    pub threat_top_value: f32,
    pub threat_on_caller: f32,
    pub follow_target_id: u32,
    pub follow_min_distance: f32,
    pub follow_max_distance: f32,
    pub spawn_pos: Option<Vector3>,
    pub dist_from_spawn: Option<f32>,
    pub spawn_yaw_rad: Option<f32>,
    pub patrol_len: usize,
    pub wander_radius: f32,
    pub respawn_secs: Option<u32>,
    pub respawn_in_secs: Option<f32>,
    pub active_effect_ids: Vec<i32>,
}

/// The full bookmark: the caller's own snapshot plus the scene around them.
#[derive(Debug, Clone)]
pub(crate) struct Bookmark {
    pub bookmark_id: u64,
    pub note: String,
    pub world_name: String,
    pub space_id: u32,
    pub navmesh_loaded: bool,
    /// Short content hash of the mesh this space is running, or `None`
    /// in a meshless space. `navmesh_loaded` alone says a mesh exists;
    /// this says *which* one — so a `.bug` filed against a bad snap-back
    /// can be attributed to a specific mesh build rather than to
    /// "Castle_CellBlock's navmesh, whichever was deployed that week".
    pub navmesh_hash: Option<String>,
    pub caller: EntitySnapshot,
    pub selected_target_id: u32,
    pub regions_inside: Vec<String>,
    pub missions_json: String,
    pub counters_json: String,
    pub threatened_mobs: Vec<u32>,
    pub in_aid_wait_or_dead: bool,
    /// Dialog ids offered to the caller and not yet answered, oldest
    /// first. A non-empty list on a stuck player is the "the server
    /// thinks a dialog is live that the client isn't showing" signal.
    pub offered_dialog_ids: Vec<i32>,
    pub last_interaction_target: u32,
    pub active_bandolier_slot: i32,
    pub weapon_visual: String,
    pub weapon_holstered: bool,
    pub movement_unrestricted: bool,
    /// `(cover_set_id, seconds inside)` per the server's proximity detection.
    pub cover_sets: Vec<(i32, f32)>,
    pub crouched: bool,
    /// Last journal entries for the tester, oldest first, as JSON.
    pub recent_events_json: String,
    pub speed_scale: f32,
    pub focus_cur: i32,
    pub focus_max: i32,
    pub entities: Vec<EntitySnapshot>,
    pub entities_in_radius: usize,
}

/// Smallest absolute angle between two headings, degrees, in `0..=180`.
pub(crate) fn angle_diff_deg(a_rad: f32, b_rad: f32) -> f32 {
    let tau = std::f32::consts::TAU;
    let d = (a_rad - b_rad).rem_euclid(tau);
    let d = if d > std::f32::consts::PI { tau - d } else { d };
    d.to_degrees()
}

/// The heading a client derives from a packed yaw byte (256 steps per turn).
pub(crate) fn unpack_yaw_byte(b: u8) -> f32 {
    f32::from(b) * (std::f32::consts::TAU / 256.0)
}

fn snapshot_entity(
    e: &CellEntity,
    caller: &CellEntity,
    selected: Option<u32>,
    space_mgr: &SpaceManager,
    now: Instant,
) -> EntitySnapshot {
    let eid = e.entity_id.0 as u32;
    let caller_eid = caller.entity_id.0 as u32;
    let dx = caller.position.x - e.position.x;
    let dz = caller.position.z - e.position.z;
    let bearing = dx.atan2(dz);
    let yaw_rad = e.direction.y;
    let yaw_byte = pack_angle(yaw_rad);
    let ground_y = space_mgr.get_navmesh_height(eid, e.position.x, e.position.y, e.position.z);
    let (threat_top_id, threat_top_value) = e
        .threat_list
        .iter()
        .max_by(|a, b| a.1.total_cmp(b.1))
        .map_or((0, 0.0), |(id, v)| (*id, *v));
    let health = e.stats.get(HEALTH);
    EntitySnapshot {
        entity_id: eid,
        is_player: e.is_player,
        is_selected_target: selected == Some(eid),
        name: e
            .npc_name
            .clone()
            .or_else(|| e.character_name.clone())
            .unwrap_or_default(),
        tag: e.tag.clone().unwrap_or_default(),
        template_id: e.template_id.unwrap_or(0),
        spawn_id: e.spawn_id.unwrap_or(0),
        level: e.level,
        faction: e.faction,
        alignment: e.alignment,
        pos: e.position,
        velocity: e.velocity,
        speed: (e.velocity[0].powi(2) + e.velocity[1].powi(2) + e.velocity[2].powi(2)).sqrt(),
        is_on_ground: e.is_on_ground,
        yaw_rad,
        yaw_byte,
        bearing_to_caller_rad: bearing,
        wire_facing_vs_caller_deg: angle_diff_deg(unpack_yaw_byte(yaw_byte), bearing),
        dist: e.position.distance_to(&caller.position),
        dist_xz: (dx * dx + dz * dz).sqrt(),
        dy_vs_caller: e.position.y - caller.position.y,
        ground_y,
        y_above_ground: ground_y.map(|g| e.position.y - g),
        on_navmesh: space_mgr.is_position_valid(eid, &e.position),
        has_los_to_caller: space_mgr.has_line_of_sight(eid, caller_eid),
        caller_witnesses_it: e.witnesses.contains(&caller.entity_id),
        witness_count: e.witnesses.len(),
        health_cur: health.map_or(0, |s| s.cur),
        health_max: health.map_or(0, |s| s.max),
        state_field: e.state_field,
        interaction_type_flags: e.interaction_type_flags,
        ai_state: format!("{:?}", e.ai_state()),
        last_movement_type: e
            .last_movement_type
            .map_or_else(|| "None".to_string(), |m| format!("{m:?}")),
        nav_path_len: e.nav_path.len(),
        next_wp: e.nav_path.front().copied(),
        final_wp: e.nav_path.back().copied(),
        move_speed: e.move_speed,
        is_stationary: e.is_stationary,
        use_cover: e.use_cover,
        aggression: crate::cell::combat::aggression_toward_players(e).level() as i32,
        aggression_override: e.aggro.override_level.map_or(0, |l| l.level() as i32),
        threat_count: e.threat_list.len(),
        threat_top_id,
        threat_top_value,
        threat_on_caller: e.threat_list.get(&caller_eid).copied().unwrap_or(0.0),
        follow_target_id: e.follow_target_id.unwrap_or(0),
        follow_min_distance: e.follow_min_distance,
        follow_max_distance: e.follow_max_distance,
        spawn_pos: e.spawn_position,
        dist_from_spawn: e.spawn_position.map(|s| s.distance_to(&e.position)),
        spawn_yaw_rad: e.spawn_direction.map(|d| d.y),
        patrol_len: e.patrol_path.len(),
        wander_radius: e.wander_radius,
        respawn_secs: e.respawn_secs,
        respawn_in_secs: e
            .respawn_at
            .map(|at| at.saturating_duration_since(now).as_secs_f32()),
        active_effect_ids: e.active_effects.iter().map(|fx| fx.effect_id).collect(),
    }
}

/// Build the snapshot. Pure — no logging, no mutation.
pub(crate) fn capture(
    caller_id: u32,
    target_id: Option<u32>,
    note: &str,
    space_mgr: &SpaceManager,
) -> Option<Bookmark> {
    let caller = space_mgr.get_entity(caller_id)?;
    let now = Instant::now();
    let space_id = space_mgr.get_entity_space_id(caller_id).unwrap_or(0);
    let world_name = space_mgr
        .get_entity_world_name(caller_id)
        .unwrap_or_default();

    let mut entities: Vec<EntitySnapshot> = space_mgr
        .all_entity_ids()
        .into_iter()
        .filter(|id| *id != caller_id)
        .filter(|id| space_mgr.get_entity_space_id(*id) == Some(space_id))
        .filter_map(|id| space_mgr.get_entity(id))
        .filter(|e| {
            e.position.distance_to(&caller.position) <= CAPTURE_RADIUS
                || target_id == Some(e.entity_id.0 as u32)
        })
        .map(|e| snapshot_entity(e, caller, target_id, space_mgr, now))
        .collect();
    entities.sort_by(|a, b| {
        // The selected target always leads, then nearest first.
        b.is_selected_target
            .cmp(&a.is_selected_target)
            .then(a.dist.total_cmp(&b.dist))
    });
    let entities_in_radius = entities.len();
    entities.truncate(CAPTURE_MAX_ENTITIES);

    let regions_inside = space_mgr
        .regions_for_world(&world_name)
        .into_iter()
        .filter(|r| {
            crate::cell::playtest_friction::region_contains_xz(
                &r.points,
                caller.position.x,
                caller.position.z,
            )
        })
        .map(|r| r.tag.clone())
        .collect();

    let missions: Vec<serde_json::Value> = caller
        .missions
        .all_missions()
        .map(|m| {
            serde_json::json!({
                "mission_id": m.mission_id,
                "status": m.status,
                "step_id": m.current_step_id,
                "hidden": m.is_hidden,
                "objectives": m.active_objectives.iter().map(|o| serde_json::json!({
                    "id": o.objective_id, "status": o.status,
                    "optional": o.optional, "hidden": o.hidden,
                })).collect::<Vec<_>>(),
                "completed_objectives": m.completed_objectives,
                "completed_steps": m.completed_steps,
            })
        })
        .collect();

    let focus = caller.stats.get(FOCUS);
    let bookmark_id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64);

    Some(Bookmark {
        bookmark_id,
        note: note.to_string(),
        world_name,
        space_id,
        navmesh_loaded: space_mgr.space_has_navmesh(caller_id),
        navmesh_hash: space_mgr.navmesh_short_hash(caller_id).map(str::to_owned),
        caller: snapshot_entity(caller, caller, target_id, space_mgr, now),
        selected_target_id: target_id.unwrap_or(0),
        regions_inside,
        missions_json: serde_json::Value::Array(missions).to_string(),
        counters_json: serde_json::to_string(&caller.counters).unwrap_or_default(),
        threatened_mobs: caller.threatened_mobs.iter().copied().collect(),
        in_aid_wait_or_dead: caller.stats.get(HEALTH).is_some_and(|h| h.cur <= 0),
        offered_dialog_ids: caller.offered_dialogs(),
        last_interaction_target: caller.last_interaction_target.unwrap_or(0),
        active_bandolier_slot: caller.active_bandolier_slot,
        weapon_visual: caller.weapon_visual.clone().unwrap_or_default(),
        weapon_holstered: caller.weapon_holstered,
        movement_unrestricted: caller.movement_unrestricted,
        cover_sets: space_mgr
            .cover_detection
            .current_sets(caller.entity_id, now),
        recent_events_json: serde_json::Value::Array(
            crate::cell::player_journal::tail(caller_id, 24)
                .into_iter()
                .map(|(seq, ms_ago, kind, detail)| {
                    serde_json::json!({"seq": seq, "ms_ago": ms_ago, "kind": kind, "detail": detail})
                })
                .collect(),
        )
        .to_string(),
        crouched: caller.state_field & crate::cell::cell_methods::combatant::BSF_CROUCHING != 0,
        speed_scale: caller.stats.movement_speed_scale(),
        focus_cur: focus.map_or(0, |s| s.cur),
        focus_max: focus.map_or(0, |s| s.max),
        entities,
        entities_in_radius,
    })
}

fn v3(v: Option<Vector3>) -> String {
    v.map_or_else(String::new, |p| format!("{:.3},{:.3},{:.3}", p.x, p.y, p.z))
}

fn emit_entity(bookmark_id: u64, rank: usize, s: &EntitySnapshot) {
    tracing::info!(
        target: "playtest.bookmark.entity",
        bookmark_id,
        rank,
        entity_id = s.entity_id,
        is_player = s.is_player,
        is_selected_target = s.is_selected_target,
        npc_name = %s.name,
        tag = %s.tag,
        template_id = s.template_id,
        spawn_id = s.spawn_id,
        level = s.level,
        faction = s.faction,
        alignment = s.alignment,
        x = s.pos.x,
        y = s.pos.y,
        z = s.pos.z,
        vx = s.velocity[0],
        vy = s.velocity[1],
        vz = s.velocity[2],
        speed = s.speed,
        is_on_ground = s.is_on_ground,
        yaw_rad = s.yaw_rad,
        yaw_byte = s.yaw_byte,
        bearing_to_caller_rad = s.bearing_to_caller_rad,
        wire_facing_vs_caller_deg = s.wire_facing_vs_caller_deg,
        dist = s.dist,
        dist_xz = s.dist_xz,
        dy_vs_caller = s.dy_vs_caller,
        ground_y = ?s.ground_y,
        y_above_ground = ?s.y_above_ground,
        on_navmesh = s.on_navmesh,
        has_los_to_caller = s.has_los_to_caller,
        caller_witnesses_it = s.caller_witnesses_it,
        witness_count = s.witness_count,
        health = s.health_cur,
        health_max = s.health_max,
        state_field = s.state_field,
        crouched = s.state_field & crate::cell::cell_methods::combatant::BSF_CROUCHING != 0,
        interaction_flags = s.interaction_type_flags,
        ai_state = %s.ai_state,
        last_movement_type = %s.last_movement_type,
        nav_path_len = s.nav_path_len,
        next_wp = %v3(s.next_wp),
        final_wp = %v3(s.final_wp),
        move_speed = s.move_speed,
        is_stationary = s.is_stationary,
        use_cover = s.use_cover,
        aggression = s.aggression,
        aggression_override = s.aggression_override,
        threat_count = s.threat_count,
        threat_top_id = s.threat_top_id,
        threat_top_value = s.threat_top_value,
        threat_on_caller = s.threat_on_caller,
        follow_target_id = s.follow_target_id,
        follow_min_distance = s.follow_min_distance,
        follow_max_distance = s.follow_max_distance,
        spawn_pos = %v3(s.spawn_pos),
        dist_from_spawn = ?s.dist_from_spawn,
        spawn_yaw_rad = ?s.spawn_yaw_rad,
        patrol_len = s.patrol_len,
        wander_radius = s.wander_radius,
        respawn_secs = ?s.respawn_secs,
        respawn_in_secs = ?s.respawn_in_secs,
        active_effect_ids = ?s.active_effect_ids,
        "playtest bookmark: entity near the tester at the moment of the report"
    );
}

/// Write the bookmark to telemetry: one header row, one row per entity.
pub(crate) fn emit(b: &Bookmark, account_id: u32, player_id: i32, access_level: u32) {
    let c = &b.caller;
    tracing::info!(
        target: "playtest.bookmark",
        bookmark_id = b.bookmark_id,
        note = %b.note,
        note_len = b.note.chars().count(),
        account_id,
        player_id,
        access_level,
        entity_id = c.entity_id,
        name = %c.name,
        level = c.level,
        world_name = %b.world_name,
        space_id = b.space_id,
        navmesh_loaded = b.navmesh_loaded,
        navmesh_hash = b.navmesh_hash.as_deref(),
        x = c.pos.x,
        y = c.pos.y,
        z = c.pos.z,
        vx = c.velocity[0],
        vy = c.velocity[1],
        vz = c.velocity[2],
        speed = c.speed,
        is_on_ground = c.is_on_ground,
        movement_unrestricted = b.movement_unrestricted,
        crouched = b.crouched,
        in_cover = !b.cover_sets.is_empty(),
        cover_sets = ?b.cover_sets,
        speed_scale = b.speed_scale,
        yaw_rad = c.yaw_rad,
        yaw_byte = c.yaw_byte,
        ground_y = ?c.ground_y,
        y_above_ground = ?c.y_above_ground,
        on_navmesh = c.on_navmesh,
        health = c.health_cur,
        health_max = c.health_max,
        focus = b.focus_cur,
        focus_max = b.focus_max,
        dead = b.in_aid_wait_or_dead,
        state_field = c.state_field,
        witness_count = c.witness_count,
        target_id = b.selected_target_id,
        last_interaction_target = b.last_interaction_target,
        offered_dialog_ids = ?b.offered_dialog_ids,
        active_bandolier_slot = b.active_bandolier_slot,
        weapon_visual = %b.weapon_visual,
        weapon_holstered = b.weapon_holstered,
        threatened_mobs = ?b.threatened_mobs,
        active_effect_ids = ?c.active_effect_ids,
        regions_inside = ?b.regions_inside,
        missions = %b.missions_json,
        counters = %b.counters_json,
        recent_events = %b.recent_events_json,
        entities_captured = b.entities.len(),
        entities_in_radius = b.entities_in_radius,
        capture_radius = CAPTURE_RADIUS,
        "playtest bookmark: tester flagged this moment"
    );
    for (rank, s) in b.entities.iter().enumerate() {
        emit_entity(b.bookmark_id, rank, s);
    }
}

/// `.bug <free text>` handler.
pub(crate) async fn bug(
    caller_id: u32,
    target_id: Option<u32>,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let note = args.join(" ");
    let Some(b) = capture(caller_id, target_id, &note, space_mgr) else {
        send_gm_feedback(caller_id, ".bug: caller entity not found", tx).await;
        return;
    };
    let id = space_mgr.player_identity(caller_id);
    let access_level = space_mgr
        .get_entity(caller_id)
        .map_or(0, |e| e.access_level);
    emit(
        &b,
        id.account_id.unwrap_or(0),
        id.player_id.unwrap_or(0),
        access_level,
    );
    send_gm_feedback(
        caller_id,
        &format!(
            "Bookmark {} recorded: {} of {} entities within {:.0}u captured{}",
            b.bookmark_id,
            b.entities.len(),
            b.entities_in_radius,
            CAPTURE_RADIUS,
            if b.selected_target_id != 0 {
                format!(", target {}", b.selected_target_id)
            } else {
                String::new()
            }
        ),
        tx,
    )
    .await;
}
