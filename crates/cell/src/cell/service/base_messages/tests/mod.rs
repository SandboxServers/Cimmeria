use super::*;
use crate::cell::space_manager::SpaceManager;
use cimmeria_entity::cell_entity::BandolierItem;

/// Build a `DisconnectEntity` for a test that doesn't care about the
/// teardown-confirmation reply (`BaseToCellMsg::DisconnectEntity`'s
/// `reply_tx`, issue #999) — the receiver is dropped immediately, and
/// `handle_base_message`'s `let _ = reply_tx.send(())` silently no-ops.
pub(super) fn disconnect_entity_msg(entity_id: u32) -> BaseToCellMsg {
    let (reply_tx, _reply_rx) = tokio::sync::oneshot::channel();
    BaseToCellMsg::DisconnectEntity {
        entity_id,
        reply_tx,
    }
}

mod ability_granted_burst;
mod ability_granted_trainer_resend;
mod bandolier_sync;
mod bandolier_sync_ammo_type;
mod bandolier_sync_reload;
mod bandolier_update;
mod bank;
mod bank_org;
mod broadcast_to_witnesses;
mod create_entity_instance;
mod disconnect_ability_snapshot;
mod disconnect_persist_position;
mod duel;
mod general;
mod gm_abilities;
mod gm_ability_granted;
mod gm_spawn_ready;
mod identity_propagation;
mod ignore;
mod item_events;
mod lab_ability_state;
mod lab_console;
mod lab_query;
mod minigame;
mod movement;
mod org;
mod passive_abilities;
mod receipt_seq;
mod request_entity_update;
mod respec_burst;
mod stat_passives;
mod telemetry_fields;
mod trade_disconnect;
