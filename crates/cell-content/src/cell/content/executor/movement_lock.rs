//! `Action::MovementLock`: a content chain pins the acting player to a seeded
//! anchor (cellblock-boot-lock D-BL3). A `hold` inserts a [`MovementHold`]
//! while the tagged prop is live and the player stands near it; a `release`
//! removes it and says the release line either way.
//!
//! **Fail-open.** A hold whose anchor prop is missing, or whose player is too
//! far from the anchor, is skipped and logged; the player is never locked
//! without a reason. **Flag.** A hold with `client_flag` sets the ref-counted
//! `BSF_MOVEMENT_LOCK` once, and its release clears it once.

use tokio::sync::mpsc;

use cimmeria_content_engine::movement_lock::MovementLockOp;
use cimmeria_wire::cell::chat::{serialize_on_player_communication, CHAN_FEEDBACK};

use crate::cell::client_methods::communicator::ON_PLAYER_COMMUNICATION;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::ring_transport::wire_helpers::{update_state_flag, BSF_MOVEMENT_LOCK};
use crate::cell::space_manager::{MovementHold, SpaceManager};

/// Run one `movement_lock` action for `entity_id`.
pub(super) async fn run(
    op: MovementLockOp,
    entity_id: u32,
    chain_id: i64,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let event = match op {
        MovementLockOp::Hold { .. } => "movement_lock.hold",
        MovementLockOp::Release { .. } => "movement_lock.release",
    };
    if !space_mgr.get_entity(entity_id).is_some_and(|e| e.is_player) {
        tracing::warn!(
            target: "content",
            event,
            decision_outcome = "refused",
            reason = "not_a_player",
            entity_id,
            entity_name = space_mgr.entity_label(entity_id),
            chain_id,
            chain_name = cimmeria_names::book().chain(chain_id),
            op = op.as_str(),
            "movement_lock fired for an entity that is not a player; nothing changed"
        );
        return;
    }
    match op {
        MovementLockOp::Hold {
            anchor_tag,
            at,
            max_distance,
            client_flag,
            message,
        } => {
            // A hold left by a previous space is stale: drop it and fall
            // through to the normal hold path, so the new anchor is used.
            let current_space_id = space_mgr.get_entity_space_id(entity_id);
            let held_space_id = space_mgr
                .get_entity(entity_id)
                .and_then(|e| e.extensions.get::<MovementHold>())
                .map(|h| h.space_id);
            match held_space_id {
                Some(space_id) if Some(space_id) == current_space_id => {
                    tracing::debug!(
                        target: "content",
                        event,
                        decision_outcome = "already_held",
                        entity_id,
                        entity_name = space_mgr.entity_label(entity_id),
                        chain_id,
                        chain_name = cimmeria_names::book().chain(chain_id),
                        op = "hold",
                        "movement_lock hold: already held; nothing changed"
                    );
                    return;
                }
                Some(_) => {
                    if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
                        entity.extensions.remove::<MovementHold>();
                    }
                    tracing::info!(
                        target: "content",
                        event,
                        decision_outcome = "stale_hold_replaced",
                        entity_id,
                        entity_name = space_mgr.entity_label(entity_id),
                        chain_id,
                        chain_name = cimmeria_names::book().chain(chain_id),
                        op = "hold",
                        "movement_lock hold: dropped a stale hold from another space"
                    );
                }
                None => {}
            }
            if space_mgr
                .find_entity_by_tag(entity_id, &anchor_tag)
                .is_none()
            {
                tracing::info!(
                    target: "content",
                    event,
                    decision_outcome = "skipped",
                    reason = "hold_skipped_no_anchor",
                    anchor_tag = %anchor_tag,
                    entity_id,
                    entity_name = space_mgr.entity_label(entity_id),
                    chain_id,
                    chain_name = cimmeria_names::book().chain(chain_id),
                    op = "hold",
                    "movement_lock hold skipped: no live prop with the anchor tag"
                );
                return;
            }
            let Some((px, pz)) = space_mgr
                .get_entity(entity_id)
                .map(|e| (e.position.x, e.position.z))
            else {
                return;
            };
            let distance = ((px - at[0]).powi(2) + (pz - at[2]).powi(2)).sqrt();
            if distance > max_distance {
                tracing::info!(
                    target: "content",
                    event,
                    decision_outcome = "skipped",
                    reason = "hold_skipped_far_from_anchor",
                    distance,
                    max_distance,
                    entity_id,
                    entity_name = space_mgr.entity_label(entity_id),
                    chain_id,
                    chain_name = cimmeria_names::book().chain(chain_id),
                    op = "hold",
                    "movement_lock hold skipped: the player is too far from the anchor"
                );
                return;
            }
            let Some(&space_id) = space_mgr.entity_space.get(&entity_id) else {
                return;
            };
            let Some(entity) = space_mgr.get_entity_mut(entity_id) else {
                return;
            };
            entity.extensions.insert(MovementHold {
                anchor: at,
                space_id,
                chain_id,
                client_flag,
                message: message.clone(),
                last_snap: None,
                last_feedback: message.is_some().then(std::time::Instant::now),
            });
            if client_flag {
                update_state_flag(entity_id, BSF_MOVEMENT_LOCK, true, tx, space_mgr).await;
            }
            if let Some(text) = &message {
                send_line(entity_id, text, tx).await;
            }
            let who = space_mgr.player_identity(entity_id);
            tracing::info!(
                target: "content",
                event,
                decision_outcome = "held",
                client_flag,
                anchor_tag = %anchor_tag,
                entity_id,
                entity_name = who.player_name,
                account_id = who.account_id,
                account_name = who.account_name,
                player_id = who.player_id,
                player_name = who.player_name,
                chain_id,
                chain_name = cimmeria_names::book().chain(chain_id),
                op = "hold",
                "movement_lock hold: the player is pinned to the anchor"
            );
        }
        MovementLockOp::Release { message } => {
            let removed = space_mgr
                .get_entity_mut(entity_id)
                .and_then(|e| e.extensions.remove::<MovementHold>());
            let (outcome, client_flag) = match &removed {
                Some(hold) => ("released", hold.client_flag),
                None => ("nothing_held", false),
            };
            if client_flag {
                update_state_flag(entity_id, BSF_MOVEMENT_LOCK, false, tx, space_mgr).await;
            }
            if let Some(text) = &message {
                send_line(entity_id, text, tx).await;
            }
            let who = space_mgr.player_identity(entity_id);
            tracing::info!(
                target: "content",
                event,
                decision_outcome = outcome,
                client_flag,
                entity_id,
                entity_name = who.player_name,
                account_id = who.account_id,
                account_name = who.account_name,
                player_id = who.player_id,
                player_name = who.player_name,
                chain_id,
                chain_name = cimmeria_names::book().chain(chain_id),
                op = "release",
                "movement_lock release"
            );
        }
    }
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
            event = "movement_lock_feedback_send_failed",
            reason = "base_channel_closed",
            entity_id, // nt:id-only shutdown path; the hold's own row names the player
            "movement_lock feedback line not queued"
        );
    }
}
