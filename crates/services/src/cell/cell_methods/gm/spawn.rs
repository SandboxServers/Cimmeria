//! GM spawn handler — `gmSpawnByCmd`.
//!
//! Unlike the other give/world handlers, this is a base round-trip: the cell
//! has no template cache, so it cannot build a `SpawnRecord` for an arbitrary
//! `template_id`. It validates the request, resolves the caller's world/space/
//! position, computes the spawn position, and hands the request to the base via
//! [`CellToBaseMsg::GmSpawnNpc`]. The base queries `resources.entity_templates`,
//! builds the record, and replies with `BaseToCellMsg::GmSpawnNpcReady`, which
//! the cell turns into an actual spawn. This mirrors the
//! `TrainAbility` → `AbilityGranted` round-trip.

use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::mercury::read_wstring;

/// `gmSpawnByCmd(WSTRING DesignId, FLOAT XOffset, FLOAT ZOffset)` — spawn an
/// NPC of the given template at the caller's position plus an X/Z offset.
///
/// `DesignId` is the numeric template id (the def types it WSTRING; the GM
/// console accepts a numeric id or an internal name, but only the numeric form
/// is wired here). The two FLOATs are world-unit offsets applied to the
/// caller's current position (Y is left unchanged — these are ground-plane
/// offsets). The computed spawn position must be finite.
pub(super) async fn handle_spawn_by_cmd(
    entity_id: u32,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    let (design_id_str, consumed) = match read_wstring(args, 0) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(entity_id, error = %e, "gmSpawnByCmd: malformed DesignId WSTRING");
            return true;
        }
    };

    // DesignId resolution: numeric template id only. Reject anything that
    // isn't a positive i32 rather than query a bogus template_id.
    let template_id = match design_id_str.trim().parse::<i32>() {
        Ok(id) if id > 0 => id,
        _ => {
            tracing::warn!(
                entity_id,
                design_id = %design_id_str,
                "gmSpawnByCmd: DesignId is not a positive numeric template id — \
                 internal-name resolution is not wired in the cell; rejecting"
            );
            return true;
        }
    };

    let x_off = match read_f32(args, consumed) {
        Some(v) => v,
        None => {
            tracing::warn!(
                entity_id,
                args_len = args.len(),
                "gmSpawnByCmd: truncated args (missing FLOAT XOffset)"
            );
            return true;
        }
    };
    let z_off = match read_f32(args, consumed + 4) {
        Some(v) => v,
        None => {
            tracing::warn!(
                entity_id,
                args_len = args.len(),
                "gmSpawnByCmd: truncated args (missing FLOAT ZOffset)"
            );
            return true;
        }
    };

    // Resolve the caller's world, space, and position.
    let (space_id, caller_pos) = match space_mgr.get_entity(entity_id) {
        Some(e) => (
            e.space_id.0 as u32,
            [e.position.x, e.position.y, e.position.z],
        ),
        None => {
            tracing::warn!(entity_id, "gmSpawnByCmd: caller entity not found");
            return true;
        }
    };
    let world_name = match space_mgr.get_entity_world_name(entity_id) {
        Some(w) => w,
        None => {
            tracing::warn!(
                entity_id,
                "gmSpawnByCmd: caller has no resolvable world name"
            );
            return true;
        }
    };

    // Ground-plane offset: X and Z shift, Y stays at the caller's height.
    let position = [caller_pos[0] + x_off, caller_pos[1], caller_pos[2] + z_off];
    if !position.iter().all(|c| c.is_finite()) {
        tracing::warn!(
            entity_id,
            ?position,
            x_off,
            z_off,
            "gmSpawnByCmd: non-finite spawn position rejected"
        );
        return true;
    }

    tracing::info!(
        entity_id,
        template_id,
        space_id,
        %world_name,
        ?position,
        "gmSpawnByCmd: requesting NPC spawn from base"
    );
    let _ = tx
        .send(CellToBaseMsg::GmSpawnNpc {
            entity_id,
            template_id,
            space_id,
            world_name,
            position,
        })
        .await;
    true
}

/// Read a little-endian `f32` at `offset`, or `None` if the slice is too short.
fn read_f32(args: &[u8], offset: usize) -> Option<f32> {
    args.get(offset..offset + 4)
        .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}
