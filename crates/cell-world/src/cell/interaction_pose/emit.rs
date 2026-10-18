//! Writes the `interaction.pose` row for one successful interaction, with a
//! per-actor throttle.
//!
//! Volume budget: humans interact a few times a minute, and the throttle caps
//! a click-spammer at 4 rows/s (one row per actor per 250 ms).

use super::capture::InteractionPose;
use super::geometry::{InteractionKind, InteractionRoute};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{LazyLock, Mutex, PoisonError};
use std::time::{Duration, Instant};

/// Minimum gap between two admitted rows for the same actor (D-PC5).
pub const POSE_MIN_INTERVAL: Duration = Duration::from_millis(250);

/// Map size at which entries older than [`POSE_MIN_INTERVAL`] are dropped.
const PRUNE_AT: usize = 2048;

/// The last admitted row time per actor entity id. Process-global.
static LAST_ROW: LazyLock<Mutex<HashMap<u32, Instant>>> = LazyLock::new(Mutex::default);

/// The `pose_seq` source. Starts at 1 and advances only when a row is admitted.
static POSE_SEQ: AtomicU64 = AtomicU64::new(1);

/// Optional content ids that only some interaction routes know. An absent
/// id stays absent on the row.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PoseExtra {
    /// The dialog the interaction opened, if any.
    pub dialog_id: Option<i32>,
    /// The item type the interaction used, if any.
    pub item_type_id: Option<i32>,
}

/// Records whether a row for `entity_id` may be written at `now`, and if so
/// stamps the time.
fn admit(entity_id: u32, now: Instant) -> bool {
    let mut map = LAST_ROW.lock().unwrap_or_else(PoisonError::into_inner);
    if map.len() >= PRUNE_AT {
        map.retain(|_, at| now.saturating_duration_since(*at) < POSE_MIN_INTERVAL);
    }
    match map.get(&entity_id) {
        Some(at) if now.saturating_duration_since(*at) < POSE_MIN_INTERVAL => false,
        _ => {
            map.insert(entity_id, now);
            true
        }
    }
}

/// Writes the `interaction.pose` row now. Returns the row's `pose_seq`, or
/// `None` when the row was throttled.
pub fn emit_interaction_pose(
    pose: &InteractionPose,
    kind: InteractionKind,
    route: InteractionRoute,
    extra: PoseExtra,
) -> Option<u64> {
    emit_interaction_pose_at(pose, kind, route, extra, Instant::now())
}

/// Writes the `interaction.pose` row as of `now`. Returns the row's
/// `pose_seq`, or `None` when the row was throttled (no log line then).
///
/// When the pose carries an approach trail, a second `interaction.approach`
/// row follows the pose row with the same `pose_seq`. A throttled pose
/// writes neither row.
pub fn emit_interaction_pose_at(
    pose: &InteractionPose,
    kind: InteractionKind,
    route: InteractionRoute,
    extra: PoseExtra,
    now: Instant,
) -> Option<u64> {
    if !admit(pose.entity_id, now) {
        return None;
    }
    let seq = POSE_SEQ.fetch_add(1, Ordering::Relaxed);
    let names = cimmeria_names::book();
    let target_template_name = pose.target_template_id.and_then(|t| names.template(t));
    let dialog_name = extra.dialog_id.and_then(|d| names.dialog(d));
    let item_name = extra.item_type_id.and_then(|i| names.item(i));
    let geometry = pose.geometry;
    tracing::info!(
        target: "interaction.pose",
        event = "interaction_pose",
        kind = kind.as_str(),
        route = route.as_str(),
        cimmeria.session_kind = pose.session_kind,
        entity_id = pose.entity_id, // nt:id-only privacy: no personal names on interaction.pose (D-PC2)
        account_id = pose.account_id, // nt:id-only privacy: no personal names on interaction.pose (D-PC2)
        player_id = pose.player_id, // nt:id-only privacy: no personal names on interaction.pose (D-PC2)
        space_id = pose.space_id, // nt:id-only named by space instance
        world_id = pose.world_id, // nt:id-only named by world
        world = %pose.world,
        player_x = pose.player_pos[0],
        player_y = pose.player_pos[1],
        player_z = pose.player_pos[2],
        player_yaw_deg = pose.player_yaw_deg,
        wire_pitch_deg = pose.wire_pitch_deg,
        target_entity_id = pose.target_entity_id, // nt:id-only privacy: no personal names on interaction.pose (D-PC2)
        target_template_id = pose.target_template_id,
        target_template_name = target_template_name,
        target_tag = pose.target_tag.as_deref(),
        target_yaw_deg = pose.target_yaw_deg,
        radius_m = geometry.radius_m,
        bearing_deg = geometry.bearing_deg,
        dy_m = geometry.dy_m,
        facing_error_deg = geometry.facing_error_deg,
        dialog_id = extra.dialog_id,
        dialog_name = dialog_name,
        item_type_id = extra.item_type_id,
        item_name = item_name,
        pose_seq = seq,
        "interaction.pose: player pose at a successful interaction"
    );
    if let Some(trail) = pose.approach.as_ref() {
        tracing::info!(
            target: "interaction.approach",
            event = "interaction_approach",
            pose_seq = seq,
            cimmeria.session_kind = pose.session_kind,
            entity_id = pose.entity_id, // nt:id-only privacy: no personal names (D-PC2)
            player_id = pose.player_id, // nt:id-only privacy: no personal names (D-PC2)
            target_entity_id = pose.target_entity_id, // nt:id-only privacy: no personal names (D-PC2)
            world = %pose.world,
            target_tag = pose.target_tag.as_deref(),
            target_template_id = pose.target_template_id,
            target_template_name = target_template_name,
            n_points = trail.points.len(),
            span_s = trail.span_s(),
            path_len_m = trail.path_len_m(),
            points = %trail.encode(),
            "interaction.approach: the player's path to a successful interaction"
        );
    }
    Some(seq)
}
