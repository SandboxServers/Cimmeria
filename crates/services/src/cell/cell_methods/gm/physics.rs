//! GM `onPhysics` handler — flips the movement-validator bypass flag that
//! backs `/gmsetfly` and `/gmsetghost`.
//!
//! Both slash commands route through this single, currently-unimplemented
//! client method (`onPhysics(UINT8 bTurnOn)`, index 221 on `SGWGmPlayer`)
//! identically — the client has no way to tell the server which of the two
//! it invoked, and doesn't need to. The client also toggles its own pawn
//! physics mode locally and instantly the moment the GM types the command,
//! with zero dependency on a server round-trip: this `onPhysics` wire send
//! is a best-effort **notification**, not a gate on the client's own
//! movement. What it *does* gate is the server's movement validator — see
//! `crate::cell::space_manager::entities::apply_client_position_update_at`,
//! which would otherwise reject the GM's now-unrestricted movement as a
//! speed-hack / off-navmesh / out-of-bounds violation.
//!
//! **Confirmed wire polarity** (traced both client handler functions in the
//! binary): `bTurnOn=0` means normal physics/collision is now **disabled**
//! (the GM is flying or ghosting) → `CellEntity::movement_unrestricted =
//! true`. `bTurnOn=1` means normal physics is **restored** →
//! `movement_unrestricted = false`. This is inverted from "is fly/ghost on",
//! so read every branch below against that mapping, not against the raw
//! byte.

use tokio::sync::mpsc;

use super::feedback::send_gm_feedback;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// `onPhysics(UINT8 bTurnOn)` — see module docs for the inverted polarity.
pub(super) async fn handle_physics(
    entity_id: u32,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    let turn_on = match args.first() {
        Some(&b) => b,
        None => {
            tracing::warn!(entity_id, "onPhysics: truncated args (need UINT8 bTurnOn)");
            send_gm_feedback(entity_id, "onPhysics: missing UINT8 bTurnOn", tx).await;
            return true;
        }
    };

    // bTurnOn=0 -> normal physics OFF -> validator bypass ON.
    let unrestricted = turn_on == 0;

    match space_mgr.get_entity_mut(entity_id) {
        Some(e) => e.movement_unrestricted = unrestricted,
        None => {
            tracing::warn!(entity_id, "onPhysics: caller entity not found");
            send_gm_feedback(entity_id, "onPhysics: caller entity not found", tx).await;
            return true;
        }
    }

    tracing::info!(
        entity_id,
        turn_on,
        unrestricted,
        "onPhysics: movement validator bypass toggled"
    );
    let text = if unrestricted {
        "onPhysics: movement validation disabled (fly/ghost mode)"
    } else {
        "onPhysics: movement validation restored"
    };
    send_gm_feedback(entity_id, text, tx).await;
    true
}
