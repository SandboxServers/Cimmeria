//! SGWPlayer world-interaction cell methods: auto-cycle toggle, loot,
//! region triggers, reload, ring-transporter destination, and system
//! options. Dispatch lives here; the per-feature state machines live in
//! the sibling submodules.

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use cimmeria_content_engine::chain::ChainEngine;
use tokio::sync::mpsc;

use super::constants::*;

mod auto_cycle;
mod item_sequence;
mod reload;
mod system_options;

// Re-export discipline: keep every cross-module call site's import path
// identical after the split. `reload`/`item_sequence` items are consumed
// from bandolier, base_messages, ticks, and use_ability via
// `cell_methods::player::world::<item>`.
pub(crate) use item_sequence::fire_item_sequence;
pub(crate) use reload::{handle_reload, maybe_trigger_reload_on_activate, UNHOLSTER_DRAW_DURATION};
// Also visible to the in-module test files (`tests.rs`,
// `system_options_tests.rs`) which reach these through `super::*`.
pub(crate) use reload::ABILITY_RELOAD_WEAPON;
pub(super) use system_options::parse_name_value_pairs;

pub async fn dispatch(
    entity_id: u32,
    method_index: u16,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) -> bool {
    match method_index {
        SET_AUTO_CYCLE => {
            auto_cycle::handle_set_auto_cycle(entity_id, args, tx, space_mgr, engine).await;
            true
        }

        LOOT_ITEM => {
            if args.len() >= 4 {
                let index = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                crate::cell::interactions::handle_loot_item(entity_id, index, tx, space_mgr).await;
            }
            true
        }

        TRIGGER_REGION => {
            if args.len() >= 17 {
                let region_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let b_entering = args[4] != 0;
                let _x = f32::from_le_bytes([args[5], args[6], args[7], args[8]]);
                let _y = f32::from_le_bytes([args[9], args[10], args[11], args[12]]);
                let _z = f32::from_le_bytes([args[13], args[14], args[15], args[16]]);

                // Region IDs are wire-encoded as i32 but stored as u32 internally.
                // Reject negative values up-front rather than sign-extending them
                // into a high u32 that no real region will match.
                let (region_tag, db_set_id) = match u32::try_from(region_id) {
                    Ok(rid) => match space_mgr.get_region(rid) {
                        Some(r) => (Some(r.tag.clone()), Some(r.db_set_id)),
                        None => (None, None),
                    },
                    Err(_) => {
                        tracing::warn!(
                            entity_id,
                            region_id,
                            "triggerClientHintedGenericRegion: negative region_id, ignoring"
                        );
                        (None, None)
                    }
                };

                if let Some(tag) = region_tag {
                    tracing::info!(entity_id, region_id, %tag, b_entering, "triggerClientHintedGenericRegion");

                    let player_id = space_mgr
                        .get_entity(entity_id)
                        .and_then(|e| e.player_id)
                        .unwrap_or(0);

                    if b_entering {
                        crate::cell::content::fire_enter_region(
                            entity_id, player_id, &tag, engine, tx, space_mgr,
                        )
                        .await;
                    } else {
                        crate::cell::content::fire_exit_region(
                            entity_id, player_id, &tag, engine, tx, space_mgr,
                        )
                        .await;
                    }

                    // Forward to the ring transporter FSM if this region is a
                    // ring pad (point_set_id matches a loaded ring region).
                    if let Some(set_id) = db_set_id {
                        crate::cell::ring_transport::handle_region_trigger(
                            set_id, b_entering, entity_id, tx, space_mgr, engine,
                        )
                        .await;
                    }
                } else {
                    tracing::warn!(
                        entity_id,
                        region_id,
                        "Unknown region ID in triggerClientHintedGenericRegion"
                    );
                }
            }
            true
        }

        REQUEST_RELOAD => {
            if !args.is_empty() {
                let _reload_type = args[0];
                tracing::debug!(entity_id, "requestReload");
                reload::handle_reload(entity_id, tx, space_mgr).await;
            }
            true
        }

        CHOSEN_REWARDS => {
            tracing::info!(entity_id, "UNIMPLEMENTED: chosenRewards");
            true
        }

        SET_RING_TRANSPORTER_DEST => {
            if args.len() >= 8 {
                let region_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let destination_id = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                tracing::info!(
                    entity_id,
                    region_id,
                    destination_id,
                    "setRingTransporterDestination"
                );
                crate::cell::ring_transport::handle_select_destination(
                    region_id,
                    destination_id,
                    entity_id,
                    tx,
                    space_mgr,
                    engine,
                )
                .await;
            }
            true
        }

        WORLD_INSTANCE_RESET => {
            tracing::info!(entity_id, "UNIMPLEMENTED: onWorldInstanceReset");
            true
        }

        UPDATE_SYSTEM_OPTIONS => {
            system_options::handle_update_system_options(entity_id, args, tx, space_mgr).await;
            true
        }

        _ => false,
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod system_options_tests;
