//! SGWPlayer world-interaction cell methods: auto-cycle toggle, loot,
//! region triggers, reload, ring-transporter destination, and system
//! options. Dispatch lives here; the per-feature state machines live in
//! the sibling submodules.

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::{SpaceManager, REGION_FLAG_STARGATE};
use crate::cell::spawner::GENERIC_REGION_CHECK_THRESHOLD;
use cimmeria_content_engine::chain::ChainEngine;
use tokio::sync::mpsc;

use super::constants::*;

mod auto_cycle;
mod item_sequence;
mod region_registration;
mod reload;
mod system_options;

// Re-export discipline: keep every cross-module call site's import path
// identical after the split. `reload`/`item_sequence` items are consumed
// from bandolier, base_messages, ticks, and use_ability via
// `cell_methods::player::world::<item>`.
pub(crate) use item_sequence::fire_item_sequence;
pub(crate) use region_registration::{send_client_hinted_regions, ClearFirst};
pub(crate) use reload::{handle_reload, maybe_trigger_reload_on_activate, UNHOLSTER_DRAW_DURATION};
// Only the in-module test files (`tests.rs`, `system_options_tests.rs`)
// reach these through `super::*`; gate the re-exports so the non-test build
// doesn't flag them unused.
#[cfg(test)]
pub(crate) use reload::ABILITY_RELOAD_WEAPON;
#[cfg(test)]
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
                // The client's own idea of where it was when it crossed the
                // volume. Deliberately unread: `resolve_hinted_region` tests
                // the server-known position instead, because these three
                // floats are exactly what an attacker would forge.
                let _x = f32::from_le_bytes([args[5], args[6], args[7], args[8]]);
                let _y = f32::from_le_bytes([args[9], args[10], args[11], args[12]]);
                let _z = f32::from_le_bytes([args[13], args[14], args[15], args[16]]);

                if let Some((tag, db_set_id, region_flags)) =
                    resolve_hinted_region(entity_id, region_id, b_entering, space_mgr)
                {
                    tracing::info!(entity_id, region_id, %tag, b_entering, "triggerClientHintedGenericRegion");
                    crate::cell::playtest_friction::region_hint(entity_id, region_id as u32);
                    crate::cell::player_journal::note(
                        entity_id,
                        crate::cell::player_journal::kinds::REGION_HINT,
                        format!("{tag} entering={b_entering}"),
                    );

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
                    crate::cell::ring_transport::handle_region_trigger(
                        db_set_id, b_entering, entity_id, tx, space_mgr, engine,
                    )
                    .await;

                    // `REGION_FLAG_Stargate` volumes additionally drive
                    // `stargatePassed` (`GenericRegion.py:174-176`). Flag-keyed,
                    // never region-id-keyed: all twelve gate regions route the
                    // same way. Runs LAST because a successful crossing
                    // destroys the cell entity, and anything after it would be
                    // acting on a torn-down entity.
                    //
                    // Harset packet H01 owns the long-term home of this
                    // routing; when the branches meet, its flag-2 entry point
                    // calls `gate_travel::on_stargate_passage` and this block
                    // goes away. The exit case has no handler in the 2009
                    // server either.
                    if b_entering && region_flags & REGION_FLAG_STARGATE != 0 {
                        crate::cell::gate_travel::handle_stargate_region_entered(
                            entity_id, tx, space_mgr, engine,
                        )
                        .await;
                    }
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

/// Refuse one client region hint: a WARN on the negative-logging seam
/// (`docs/architecture/negative-logging-convention.md`) plus a journal note,
/// then `None`.
///
/// Both halves are load-bearing, and the 2026-09-18 Castle playtest is why.
/// The symptom of a *false* reject is "the door does nothing", which from
/// the outside is indistinguishable from the client never having sent the
/// hint at all. A silent `return None` here would make the two
/// indistinguishable from the inside too. The WARN carries the server-known
/// position so a reject can be plotted against the volume that refused it,
/// and the journal note puts it in the player's own `.bug` bookmark, where
/// the report is actually written.
fn refuse_hinted_region(
    entity_id: u32,
    region_id: i32,
    region_tag: &str,
    position: Option<[f32; 3]>,
    reason: &'static str,
    detail: &str,
) -> Option<(String, i32, i32)> {
    // `Option<f32>` tracing fields are omitted entirely when `None`, which
    // is the honest rendering: an entity we cannot find has no position,
    // and a `0.0` there would read as "standing at the origin".
    tracing::warn!(
        entity_id,
        region_id,
        region_tag,
        pos_x = position.map(|p| p[0]),
        pos_y = position.map(|p| p[1]),
        pos_z = position.map(|p| p[2]),
        reason,
        "triggerClientHintedGenericRegion refused: {detail}"
    );
    crate::cell::player_journal::note(
        entity_id,
        crate::cell::player_journal::kinds::REGION_HINT_REFUSED,
        format!("{region_id} {region_tag} {reason}"),
    );
    None
}

/// Resolve a client-supplied `triggerClientHintedGenericRegion` id to the
/// `(tag, db_set_id, flags)` the dispatch arm acts on, refusing anything the
/// caller has no business triggering. `None` means "already logged and
/// journaled, do nothing".
///
/// Three gates, all of which the 2009 server had and Cimmeria had lost:
///
/// 1. **Non-negative id.** Wire-encoded `i32`, stored `u32`; rejecting up
///    front beats sign-extending into a high `u32` that could collide with
///    a real runtime id.
/// 2. **Caller's world.** [`SpaceManager::get_region`] is a world-*global*
///    map keyed on the id the client hands us, while `RegionData` knows the
///    world its point set was seeded against. In 2009 this scope was
///    structural — `GenericRegionManager.load(worldId)` built one manager
///    per space, so a region id simply did not exist outside its world.
///    Cimmeria flattened that, which let a client in any world name any
///    region in the game and fire its chains. (H10 worknote IR-1.)
/// 3. **Server-known containment**, on entry only. Port of
///    `deprecated/python/cell/GenericRegion.py:167-170`, which tests
///    `entity.position` — the position the *server* accepted — never the
///    x/y/z the RPC carried. Without it a forged `(region_id, entering)`
///    pair fires an `enter_region` chain, and its `cross_world_teleport`,
///    from anywhere on the map. Exits stay ungated, matching the Python's
///    own `# TODO: Check !entering and isPointOutsideRegion() too`: a
///    player who leaves a volume is by definition outside it, and gating
///    the exit would strand every "on leave" chain.
///
/// All three run **above** the flag dispatch, so they also cover the ring
/// forwarding below and Castle CA10's `REGION_FLAG_STARGATE` branch when it
/// lands. Keep this one call at the top of the arm: that is what makes the
/// CA10 rebase mechanical.
fn resolve_hinted_region(
    entity_id: u32,
    region_id: i32,
    b_entering: bool,
    space_mgr: &SpaceManager,
) -> Option<(String, i32, i32)> {
    // Read once, up front: every refusal below wants it for the log, and
    // the containment gate wants it for the test itself.
    let position = space_mgr
        .get_entity(entity_id)
        .map(|e| [e.position.x, e.position.y, e.position.z]);

    let Ok(runtime_id) = u32::try_from(region_id) else {
        return refuse_hinted_region(
            entity_id,
            region_id,
            "",
            position,
            "region_id_negative",
            "negative region id — no chain, ring transition or gate crossing fires",
        );
    };

    let Some(region) = space_mgr.get_region(runtime_id) else {
        return refuse_hinted_region(
            entity_id,
            region_id,
            "",
            position,
            "region_unknown",
            "no region with this runtime id — no chain, ring transition or gate \
             crossing fires",
        );
    };

    let Some(caller_world) = space_mgr.get_entity_world_name(entity_id) else {
        return refuse_hinted_region(
            entity_id,
            region_id,
            &region.tag,
            position,
            "region_caller_unspaced",
            "caller is bound to no space — no chain, ring transition or gate \
             crossing fires",
        );
    };

    if region.world_name != caller_world {
        return refuse_hinted_region(
            entity_id,
            region_id,
            &region.tag,
            position,
            "region_world_mismatch",
            &format!(
                "region belongs to world {} but the caller is in {caller_world} — a \
                 client cannot trigger a region it is not standing in",
                region.world_name
            ),
        );
    }

    if b_entering {
        let Some(position) = position else {
            return refuse_hinted_region(
                entity_id,
                region_id,
                &region.tag,
                position,
                "region_caller_unspaced",
                "caller has a space binding but no cell entity — no chain, ring \
                 transition or gate crossing fires",
            );
        };
        if !crate::cell::spawner::is_point_in_region(&region.points, position) {
            return refuse_hinted_region(
                entity_id,
                region_id,
                &region.tag,
                Some(position),
                "region_containment_failed",
                &format!(
                    "server-known position is outside the {}-corner volume (plus \
                     {GENERIC_REGION_CHECK_THRESHOLD} units of slop) — refusing the \
                     enter event",
                    region.points.len()
                ),
            );
        }
    }

    Some((region.tag.clone(), region.db_set_id, region.flags))
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod system_options_tests;
