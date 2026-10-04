//! No ability use while stunned or knocked down (ability mechanics AB-09a).
//!
//! Python's `SGWBeing` lists `PLAYER_STATE_Stun` as "BSF_MovementLock + No
//! ability/item use". The client reads `BSF_MovementLock` for movement
//! only and does not block a cast, so the server refuses the launch. The
//! test is a timed-effect entry holding the lock, not the bare bit: ring
//! transport and death set the bit too, and they have their own gates.
//!
//! A player's press is answered like AB-12's: `onErrorCode` plus a
//! `CHAN_FEEDBACK` line, no cooldown, one `abilities` row. The auto-cycle
//! loop's own relaunch is refused silently, so a stunned auto-attacker is
//! not sent a line every tick, and the loop resumes when the lock clears.
//! An NPC's cast is refused silently: its AI already holds while stunned.

use tokio::sync::mpsc;

use cimmeria_entity::cell_entity::CellEntity;
use cimmeria_wire::state_field::BSF_MOVEMENT_LOCK;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// The feedback line a refused press shows.
pub(crate) const INCAPACITATED_TEXT: &str = "You cannot do that while stunned.";

/// `CONDITION_FEEDBACK_Deprecated` (15): the code python's `canUseAbility`
/// returned for a launch it refused for the caster's own state. The client
/// enum has no stun value and no Lua renders `onErrorCode`; the chat line
/// is what the player reads.
pub(crate) const INCAPACITATED_ERROR_CODE: u16 = 15;

/// Whether a stun or knockdown entry holds `entity`'s movement lock.
pub(crate) fn is_incapacitated(entity: &CellEntity) -> bool {
    entity.holds_ledger_flag(BSF_MOVEMENT_LOCK)
}

/// Refuse `entity_id`'s launch of `ability_id`: the row, then (for a
/// player's own press) the feedback. The caller returns before the
/// cooldown.
pub(super) async fn refuse_while_incapacitated(
    entity_id: u32,
    ability_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let Some(entity) = space_mgr.get_entity(entity_id) else {
        return;
    };
    let is_player = entity.is_player;
    let loop_relaunch =
        entity.abilities.auto_cycle && entity.abilities.auto_cycle_ability_id == Some(ability_id);
    if loop_relaunch {
        // The auto-cycle tick relaunches every 100 ms; the loop just waits.
        tracing::trace!(target: "abilities", entity_id, ability_id, "auto-cycle paused: stunned");
        return;
    }
    let id = space_mgr.player_identity(entity_id);
    // DEBUG: a stunned client can press at will.
    tracing::debug!(
        target: "abilities",
        event = "incapacitated_refused",
        decision_outcome = "refused",
        reason = "incapacitated",
        entity_id,
        account_id = id.account_id,
        player_id = id.player_id,
        ability_id,
        is_player,
        "useAbility: the caster is stunned or knocked down; refused, no cooldown charged"
    );
    if !is_player {
        return;
    }
    super::no_mechanics::send_ability_refusal(
        entity_id,
        ability_id,
        INCAPACITATED_ERROR_CODE,
        INCAPACITATED_TEXT,
        tx,
    )
    .await;
}
