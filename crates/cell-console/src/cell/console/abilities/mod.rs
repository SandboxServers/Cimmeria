//! Ability lab commands (ability-mechanics AB-L2): the tools the ability UAT
//! needs that the client never had a native command for (D-AU7).
//!
//! - [`effects`] — `.effects`: the AB-T5 ability state of the selected
//!   target (else the caller), as a few concise chat lines.
//! - [`cooldowns`] — `.cooldowns [reset [abilityId]]`: list the caller's
//!   cooldowns, or clear them and send the client the clear timer, so its
//!   hotbar sweep stops with the server's.
//! - [`dummy`] — `.dummy [hostile|friendly|clear] [templateId]`: a lab
//!   target that never fights back (D-AU6), and the sweeps that take it
//!   away after ten minutes or when its owner logs out.
//! - [`clear_effects`] — `.cleareffects`: strip the selected target's (else
//!   the caller's) ledger entries and pulsing effects, reason `cleansed`,
//!   with the client's icon clears.
//!
//! `.qr` (D-AU2) is not here: it waits on the owner.
//!
//! **Authority.** The chat gate admits only `access_level >= GameMaster`, and
//! the dispatcher's target contract applies: `[target]` is the caller's
//! selection, and only when it is in the caller's space and view (#844).
//! `.cooldowns` acts on the caller alone, and `.dummy clear` removes only the
//! caller's own dummies. Every accepted line gets the dispatcher's audit row;
//! each state change also logs one `abilities.gm` INFO row naming the GM and
//! the entity it changed.

mod clear_effects;
pub(crate) mod cooldowns;
pub(crate) mod dummy;
pub(crate) mod effects;

use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

pub use dummy::{despawn_lab_dummies_of, lab_dummy_tick};

/// Route one validated AB-L2 command. `target_id` is the dispatcher's
/// resolved selection (`Target::None`: in view, else `None`).
pub(super) async fn dispatch(
    name: &str,
    caller_id: u32,
    args: &[&str],
    target_id: Option<u32>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let subject = target_id.unwrap_or(caller_id);
    match name {
        "effects" => effects::show(caller_id, subject, tx, space_mgr).await,
        "cooldowns" => cooldowns::run(caller_id, args, tx, space_mgr).await,
        "dummy" => dummy::run(caller_id, args, tx, space_mgr).await,
        "cleareffects" => clear_effects::run(caller_id, subject, tx, space_mgr).await,
        _ => {}
    }
}

/// `character_name` for a player, `npc_name` for an NPC, else `entity <id>`.
fn display_name(space_mgr: &SpaceManager, entity_id: u32) -> String {
    space_mgr
        .get_entity(entity_id)
        .and_then(|e| e.character_name.clone().or_else(|| e.npc_name.clone()))
        .unwrap_or_else(|| format!("entity {entity_id}"))
}
