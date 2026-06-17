//! GM stat handlers — `gmSetHealth` / `gmSetHealthMax` / `gmSetFocus` /
//! `gmSetFocusMax`. Each takes `(INT32 Amount, INT64 TargetId)`; the target
//! defaults to the caller (the common "set my own health" dev case) and may
//! name any entity in the caller's space (see [`super::resolve_self_or_target`]).
//!
//! Mutation reuses the canonical [`Stat::set_current`] / [`Stat::set_max`]
//! primitives, then pushes the change via the `onStatUpdate` wire path — to the
//! owning client for a player target, or to AoI witnesses for an NPC target
//! (handled by [`crate::cell::abilities::send_entity_method`]).

use cimmeria_entity::stats::{FOCUS, HEALTH};
use tokio::sync::mpsc;

use super::{read_i32, read_i64, resolve_self_or_target};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::mercury::method_idx::ON_STAT_UPDATE;

/// `gmSetHealth(INT32 Amount, INT64 TargetId)` (`is_max == false`) or
/// `gmSetHealthMax` (`is_max == true`).
pub(super) async fn handle_set_health(
    entity_id: u32,
    args: &[u8],
    is_max: bool,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    set_stat(
        entity_id,
        args,
        HEALTH,
        is_max,
        "gmSetHealth",
        tx,
        space_mgr,
    )
    .await
}

/// `gmSetFocus(INT32 Amount, INT64 TargetId)` (`is_max == false`) or
/// `gmSetFocusMax` (`is_max == true`).
pub(super) async fn handle_set_focus(
    entity_id: u32,
    args: &[u8],
    is_max: bool,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    set_stat(entity_id, args, FOCUS, is_max, "gmSetFocus", tx, space_mgr).await
}

/// Shared body for the four set-stat commands: parse `(INT32 Amount, INT64
/// TargetId)`, resolve the target, mutate the stat, and broadcast `onStatUpdate`.
async fn set_stat(
    entity_id: u32,
    args: &[u8],
    stat_id: i32,
    is_max: bool,
    cmd: &str,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    let amount = match read_i32(args, 0) {
        Some(v) => v,
        None => {
            tracing::warn!(
                entity_id,
                args_len = args.len(),
                cmd,
                "GM set-stat: truncated args (need INT32 Amount)"
            );
            return true;
        }
    };
    let target_raw = match read_i64(args, 4) {
        Some(v) => v,
        None => {
            tracing::warn!(
                entity_id,
                args_len = args.len(),
                cmd,
                "GM set-stat: truncated args (missing INT64 TargetId)"
            );
            return true;
        }
    };
    // A negative value is nonsensical for health/focus (current or max);
    // `set_current` would clamp it to `min` anyway, and a negative max would
    // collapse the bar. Reject so the GM gets a clear no-op + warn.
    if amount < 0 {
        tracing::warn!(
            entity_id,
            amount,
            cmd,
            is_max,
            "GM set-stat: negative amount rejected"
        );
        return true;
    }

    let Some(target_eid) = resolve_self_or_target(entity_id, target_raw, space_mgr, cmd) else {
        return true;
    };

    // Mutate the stat in place; capture whether the target is a player so we
    // pick the right serialization (full for the owning HUD, public-only for
    // an NPC's witnesses).
    let (changed, is_player) = {
        let Some(target) = space_mgr.get_entity_mut(target_eid) else {
            tracing::warn!(
                entity_id,
                target_eid,
                cmd,
                "GM set-stat: target vanished before mutate"
            );
            return true;
        };
        let is_player = target.is_player;
        match target.stats.get_mut(stat_id) {
            Some(stat) => {
                if is_max {
                    stat.set_max(amount);
                } else {
                    stat.set_current(amount);
                }
                (true, is_player)
            }
            None => {
                tracing::warn!(
                    entity_id,
                    target_eid,
                    stat_id,
                    cmd,
                    "GM set-stat: target has no such stat"
                );
                (false, is_player)
            }
        }
    };
    if !changed {
        return true;
    }

    // Serialize + clear the dirty set, then broadcast onStatUpdate.
    let payload = {
        let target = space_mgr
            .get_entity_mut(target_eid)
            .expect("target existence checked above");
        let p = if is_player {
            target.stats.serialize_dirty()
        } else {
            target.stats.serialize_dirty_public()
        };
        target.stats.clear_dirty();
        p
    };

    tracing::info!(
        entity_id,
        target_eid,
        stat_id,
        amount,
        is_max,
        "GM set-stat: applied"
    );
    if !payload.is_empty() {
        crate::cell::abilities::send_entity_method(
            target_eid,
            ON_STAT_UPDATE,
            payload,
            tx,
            space_mgr,
        )
        .await;
    }
    true
}
