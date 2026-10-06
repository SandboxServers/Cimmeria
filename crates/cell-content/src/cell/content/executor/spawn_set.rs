//! `Action::SpawnSet`: the Visual NPC Lineup attendants (DA-10). A
//! right-click on an attendant fires its `interact_tag` chain, which shows
//! one lineup group (switching off whichever was on) or clears the lineup.
//!
//! **Authority.** GM-gated on the clicking player's account access level, the
//! same gate as the ability granter: a non-GM gets a refusal line and nothing
//! changes. **Feedback.** Every click gets a line on the first press: what is
//! now showing and how many actors, "already showing", "nothing to clear", or
//! the refusal. A repeat click inside the chain debounce is ignored (the
//! first already answered).

use tokio::sync::mpsc;

use cimmeria_cell_world::cell::dispatch::is_gm;
use cimmeria_content_engine::actions::SpawnSetOp;
use cimmeria_wire::cell::chat::{serialize_on_player_communication, CHAN_FEEDBACK};

use crate::cell::client_methods::communicator::ON_PLAYER_COMMUNICATION;
use crate::cell::content::spawn_sets::{
    clear_spawn_set_kind, hide_spawn_set, log_switch, show_spawn_set,
};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// The line a non-GM gets.
pub(crate) const NOT_GM_LINE: &str = "Only a GM can switch the lineup. Nothing changed.";

/// Run one `spawn_set` action for `entity_id`.
pub(super) async fn run(
    op: SpawnSetOp,
    entity_id: u32,
    chain_id: i64,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(player) = space_mgr.get_entity(entity_id).filter(|e| e.is_player) else {
        tracing::warn!(
            target: "content",
            event = "spawn_set.switched",
            decision_outcome = "refused",
            reason = "not_a_player",
            entity_id,
            entity_name = space_mgr.entity_label(entity_id),
            chain_id,
            chain_name = cimmeria_names::book().chain(chain_id),
            op = op.as_str(),
            "spawn_set fired for an entity that is not a player; nothing changed"
        );
        return;
    };
    let access_level = player.access_level;
    if !space_mgr.chain_debounce(entity_id, chain_id, std::time::Instant::now()) {
        tracing::debug!(
            event = "spawn_set.switched",
            decision_outcome = "debounced",
            entity_id,
            entity_name = space_mgr.entity_label(entity_id),
            chain_id,
            chain_name = cimmeria_names::book().chain(chain_id),
            op = op.as_str(),
            "lineup attendant: a repeat click inside the debounce window; ignored"
        );
        return;
    }
    if !is_gm(access_level) {
        let who = space_mgr.player_identity(entity_id);
        tracing::info!(
            target: "content",
            event = "spawn_set.switched",
            door = "attendant",
            decision_outcome = "refused",
            reason = "not_gm",
            entity_id,
            entity_name = who.player_name,
            account_id = who.account_id,
            account_name = who.account_name,
            player_id = who.player_id,
            player_name = who.player_name,
            access_level,
            chain_id,
            chain_name = cimmeria_names::book().chain(chain_id),
            op = op.as_str(),
            "lineup attendant refused: the player is not a GM"
        );
        send_line(entity_id, NOT_GM_LINE, tx).await;
        return;
    }
    let (set_id, switch) = match &op {
        SpawnSetOp::Show { set_id } => {
            (Some(*set_id), show_spawn_set(*set_id, tx, space_mgr).await)
        }
        SpawnSetOp::Hide { set_id } => {
            (Some(*set_id), hide_spawn_set(*set_id, tx, space_mgr).await)
        }
        SpawnSetOp::Clear { kind, world_id } => (
            None,
            clear_spawn_set_kind(kind, *world_id, tx, space_mgr).await,
        ),
    };
    log_switch("attendant", entity_id, set_id, &switch, space_mgr);
    send_line(entity_id, &switch.line(), tx).await;
}

/// One `SYSTEM` feedback line to the player.
async fn send_line(entity_id: u32, text: &str, tx: &mpsc::Sender<CellToBaseMsg>) {
    let args = serialize_on_player_communication("SYSTEM", 0, CHAN_FEEDBACK, text);
    if tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: ON_PLAYER_COMMUNICATION,
            args,
        })
        .await
        .is_err()
    {
        tracing::warn!(
            target: "content",
            event = "spawn_set_feedback_send_failed",
            reason = "base_channel_closed",
            entity_id, // nt:id-only shutdown path; the switch's own row names the player
            "lineup attendant feedback line not queued"
        );
    }
}
