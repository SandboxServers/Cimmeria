//! `gmSetGodMode(UINT8 bTurnOn)` (SGWGmPlayer index 142, `/gmsetgodmode`):
//! the calling GM takes no Health or Focus loss while it is on.
//!
//! The def gives the method no target argument, so god mode is only ever
//! the caller's: a GM cannot make another player invulnerable through it.
//! The flag is `CellEntity::god_mode`; the damage seams read it through
//! `cimmeria_cell_combat`'s `combat::god_mode` (the hit and the pulse), so a
//! god-mode GM still receives every heal, buff and debuff an ability lands.
//! In memory only: a relog starts with it off, as `onPhysics` does.

use tokio::sync::mpsc;

use super::command_log::{log_gm_command, Outcome};
use super::feedback::send_gm_feedback;
use super::GM_SET_GOD_MODE;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

const CMD: &str = "gmSetGodMode";

/// `gmSetGodMode(UINT8 bTurnOn)`: any non-zero byte turns it on.
pub(super) async fn handle_set_god_mode(
    entity_id: u32,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    let Some(&turn_on) = args.first() else {
        log_gm_command(
            space_mgr,
            entity_id,
            CMD,
            GM_SET_GOD_MODE,
            "",
            Outcome::Refused("bad_args"),
        );
        send_gm_feedback(entity_id, "gmSetGodMode: missing UINT8 bTurnOn", tx).await;
        return true;
    };
    let on = turn_on != 0;
    let args_text = format!("bTurnOn={turn_on}");
    let Some(entity) = space_mgr.get_entity_mut(entity_id) else {
        log_gm_command(
            space_mgr,
            entity_id,
            CMD,
            GM_SET_GOD_MODE,
            &args_text,
            Outcome::Refused("caller_gone"),
        );
        send_gm_feedback(entity_id, "gmSetGodMode: caller entity not found", tx).await;
        return true;
    };
    entity.god_mode = on;
    log_gm_command(
        space_mgr,
        entity_id,
        CMD,
        GM_SET_GOD_MODE,
        &args_text,
        Outcome::Applied,
    );
    let text = if on {
        "gmSetGodMode: on. You take no Health or Focus damage until you turn it off or relog"
    } else {
        "gmSetGodMode: off. Damage lands normally"
    };
    send_gm_feedback(entity_id, text, tx).await;
    true
}
