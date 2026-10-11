//! GM travel handlers — `gmGotoXYZ` (same-space snap), `gmGotoLocation`
//! (cross-world reload), and `gmDHD` (stargate dial).
//!
//! `gmGotoXYZ` is the verified same-space teleport. `gmGotoLocation`
//! reuses the cross-world `GateTravel` path, and `gmDHD` reuses the canonical
//! [`crate::cell::gate_travel::handle_dial_gate`] primitive.

mod dhd;
mod goto_location;
mod goto_summon;
mod goto_xyz;

pub(super) use dhd::handle_dhd;
pub(super) use goto_location::handle_goto_location;
pub(super) use goto_summon::{handle_goto, handle_summon};
pub(super) use goto_xyz::handle_goto_xyz;

use crate::cell::space_manager::SpaceManager;
use crate::mercury::read_wstring;

/// Parse a leading `WSTRING aNameOrID` as a positive numeric entity id.
/// Returns `None` (after a warn) on malformed/non-numeric input.
fn parse_target_id(
    entity_id: u32,
    args: &[u8],
    cmd: &str,
    space_mgr: &SpaceManager,
) -> Option<u32> {
    let name_or_id = match read_wstring(args, 0) {
        Ok((s, _)) => s,
        Err(e) => {
            tracing::warn!(
                entity_id,
                entity_name = space_mgr.entity_label(entity_id),
                error = %e,
                cmd,
                "GM goto/summon: malformed NameOrID WSTRING",
            );
            return None;
        }
    };
    match name_or_id.trim().parse::<u32>() {
        Ok(id) if id > 0 => Some(id),
        _ => {
            tracing::warn!(
                entity_id,
                entity_name = space_mgr.entity_label(entity_id),
                name_or_id_arg = %name_or_id,
                cmd,
                "GM goto/summon: NameOrID is not a positive numeric id — name resolution not wired in the cell",
            );
            None
        }
    }
}

/// Read three consecutive little-endian `f32`s at `offset`, or `None` if the
/// slice is too short.
fn read_xyz(args: &[u8], offset: usize) -> Option<[f32; 3]> {
    let b = args.get(offset..offset + 12)?;
    Some([
        f32::from_le_bytes([b[0], b[1], b[2], b[3]]),
        f32::from_le_bytes([b[4], b[5], b[6], b[7]]),
        f32::from_le_bytes([b[8], b[9], b[10], b[11]]),
    ])
}
