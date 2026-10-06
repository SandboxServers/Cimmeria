//! `activateSpawnSet(INT32)` (214) and `deactivateSpawnSet(INT32)` (215):
//! switch a spawn set on or off for everyone in its world.
//!
//! The def's comment calls the argument a "SpawnSet EntityID": the 2009
//! server had a SpawnSet entity per set. Cimmeria has no such entity, so the
//! argument is the `resources.spawn_sets.set_id` (the Debug Area's Visual NPC
//! Lineup groups are 1301-1305). Showing a set first switches off the other
//! sets of its kind in its world, so the lineup shows one group at a time;
//! the Lineup attendants and `.spawnset` call the same
//! [`show_spawn_set`] / [`hide_spawn_set`].
//!
//! GM-gated at the dispatch layer (index >= 109). Every press answers with a
//! feedback line, a refusal included, and writes one `spawn_set.switched`
//! row.

use tokio::sync::mpsc;

use cimmeria_cell_content::cell::content::spawn_sets::{
    hide_spawn_set, log_switch, show_spawn_set,
};

use super::feedback::send_gm_feedback;
use super::read_i32;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// `activateSpawnSet(INT32)` (`on = true`) or `deactivateSpawnSet(INT32)`.
pub(super) async fn handle_switch(
    entity_id: u32,
    args: &[u8],
    on: bool,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    let cmd = if on {
        "activateSpawnSet"
    } else {
        "deactivateSpawnSet"
    };
    let Some(set_id) = read_i32(args, 0) else {
        let who = space_mgr.player_identity(entity_id);
        tracing::info!(
            target: "content",
            event = "spawn_set.switched",
            door = "gm_native",
            decision_outcome = "refused",
            reason = "bad_args",
            cmd,
            entity_id,
            entity_name = who.player_name,
            account_id = who.account_id,
            account_name = who.account_name,
            player_id = who.player_id,
            player_name = who.player_name,
            args_len = args.len(),
            "GM spawn-set switch refused: missing INT32 set id"
        );
        send_gm_feedback(entity_id, &format!("{cmd}: missing INT32 spawn set id"), tx).await;
        return true;
    };
    let switch = if on {
        show_spawn_set(set_id, tx, space_mgr).await
    } else {
        hide_spawn_set(set_id, tx, space_mgr).await
    };
    log_switch("gm_native", entity_id, Some(set_id), &switch, space_mgr);
    send_gm_feedback(entity_id, &switch.line(), tx).await;
    true
}
