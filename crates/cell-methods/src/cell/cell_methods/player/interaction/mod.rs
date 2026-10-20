//! SGWPlayer interaction cell methods: `who`, `interact` (right-click),
//! `dialogButtonChoice`, and `initialResponse`. Dispatch lives here; the
//! per-method handlers live in the sibling submodules.

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use cimmeria_content_engine::chain::ChainEngine;
use tokio::sync::mpsc;

use super::constants::*;

#[cfg(test)]
mod bank_dispatch_tests;
#[cfg(test)]
mod castle_defense_q1_range_tests;
#[cfg(test)]
mod castle_medic_q2_range_tests;
#[cfg(test)]
mod castle_medic_range_tests;
#[cfg(test)]
mod castle_mohkatan_range_tests;
#[cfg(test)]
mod crafting_hub_station_tests;
#[cfg(test)]
mod debug_hub_dispatch_tests;
mod dialog;
#[cfg(test)]
mod dialog_choice_gate_tests;
mod hostile_attack;
#[cfg(test)]
mod hostile_attack_tests;
mod interact;
#[cfg(test)]
mod interact_pin_tests;
#[cfg(test)]
mod npc_routing_tests;
#[cfg(test)]
mod registrar_dispatch_tests;

pub async fn dispatch(
    entity_id: u32,
    method_index: u16,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) -> bool {
    match method_index {
        WHO => {
            tracing::info!(
                entity_id,
                entity_name = space_mgr.entity_label(entity_id),
                "UNIMPLEMENTED: who"
            );
            true
        }

        INTERACT => {
            interact::handle_interact(entity_id, args, tx, space_mgr, engine).await;
            true
        }

        DIALOG_BUTTON_CHOICE => {
            dialog::handle_dialog_button_choice(entity_id, args, tx, space_mgr, engine).await;
            true
        }

        INITIAL_RESPONSE => {
            dialog::handle_initial_response(entity_id, args, tx, space_mgr, engine).await;
            true
        }

        _ => false,
    }
}
