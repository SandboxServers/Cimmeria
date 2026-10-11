//! SGWPlayer vendor cell methods: ability training, the trainer respec,
//! plus the purchase / sell / buyback / repair / recharge op group.
//! Dispatch lives here; the wire decoders, session validation, and training
//! validation live in the sibling submodules. The respec's method index
//! (72) sits in the combat range, so `combat::dispatch` calls
//! [`handle_reset_my_abilities`].

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use tokio::sync::mpsc;

use super::constants::*;

mod respec;
#[cfg(test)]
mod respec_tests;
mod session;
mod store_ops;
#[cfg(test)]
mod store_ops_tests;
mod train;
mod train_feedback;
#[cfg(test)]
mod train_feedback_tests;
#[cfg(test)]
mod train_spend_tests;
#[cfg(test)]
mod train_trainer_agreement;
mod wire;

pub(crate) use respec::handle_reset_my_abilities;
use store_ops::handle_store_op;
use train::handle_train_ability;

pub async fn dispatch(
    entity_id: u32,
    method_index: u16,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    match method_index {
        TRAIN_ABILITY => {
            if args.len() >= 4 {
                let ability_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                handle_train_ability(entity_id, ability_id, tx, space_mgr).await;
            } else {
                tracing::warn!(
                    entity_id,
                    entity_name = space_mgr.entity_label(entity_id),
                    args_len = args.len(),
                    "trainAbility: truncated args (need 4 bytes ability_id)"
                );
            }
            true
        }

        PURCHASE_ITEMS | SELL_ITEMS | BUYBACK_ITEMS | REPAIR_ITEMS | RECHARGE_ITEMS => {
            handle_store_op(entity_id, method_index, args, tx, space_mgr).await;
            true
        }

        _ => false,
    }
}
