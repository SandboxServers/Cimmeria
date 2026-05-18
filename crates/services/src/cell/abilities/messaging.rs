//! Entity-method routing helpers for the abilities flow.
//!
//! Keeps the wire-routing decision (player → direct EntityMethodCall, NPC →
//! fan-out to witnesses) in one place so every callsite in `use_ability` and
//! `dispatch` ends up with identical behavior. Also hosts the dirty-stat
//! flush helper that pushes a queued `onStatUpdate` to the attacker's client
//! after an ammo decrement.

use tokio::sync::mpsc;

use super::super::messages::CellToBaseMsg;
use super::super::space_manager::SpaceManager;

/// Send an entity method call, routing to the entity's client if it's a player,
/// or broadcasting to all witnessing players if it's an NPC (ghost entity).
///
/// In BigWorld, method calls on ghost entities are forwarded to all players who
/// have that entity in their AoI. This is how players see NPC attack animations,
/// health changes, death states, etc.
pub(crate) async fn send_entity_method(
    entity_id: u32,
    method_index: u16,
    args: Vec<u8>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let is_player = space_mgr.get_entity(entity_id).is_some_and(|e| e.is_player);

    if is_player {
        let _ = tx
            .send(CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index,
                args,
            })
            .await;
    } else {
        let witnesses = space_mgr.get_witnesses_of(entity_id);
        if witnesses.is_empty() {
            tracing::warn!(
                entity_id,
                method_index,
                "send_entity_method: NPC has no witnesses, method dropped"
            );
        }
        for witness_id in witnesses {
            tracing::debug!(
                witness_id,
                entity_id,
                method_index,
                "send_entity_method: routing NPC method to witness"
            );
            let _ = tx
                .send(CellToBaseMsg::WitnessEntityMethod {
                    witness_id,
                    entity_id,
                    method_index,
                    args: args.clone(),
                })
                .await;
        }
    }
}

/// Send a `CellToBaseMsg::RefreshAppearance` for a player entity, reading
/// the player's current `weapon_holstered` state off the cell entity.
///
/// Phase 2 of the holster work (PR #338) — called from the combat
/// enter/exit broadcast sites after `onStateFieldUpdate` so a draw or
/// holster reaches the wire in the same dispatch burst as the BSF_InCombat
/// change. No-op (with a debug log) for non-player entities or for
/// players whose `player_id` (DB id) hasn't been populated yet — both
/// happen during transient world-entry races and we'd rather drop the
/// rebroadcast than send junk.
pub(crate) async fn request_appearance_refresh(
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let (player_id, holstered) = match space_mgr.get_entity(entity_id) {
        Some(e) if e.is_player => match e.player_id {
            Some(pid) => (pid, e.weapon_holstered),
            None => {
                tracing::debug!(
                    entity_id,
                    "request_appearance_refresh: player entity has no DB player_id (pre-load?), skipping"
                );
                return;
            }
        },
        Some(_) => {
            tracing::debug!(
                entity_id,
                "request_appearance_refresh: entity is not a player, skipping"
            );
            return;
        }
        None => {
            tracing::debug!(
                entity_id,
                "request_appearance_refresh: entity not found in space_mgr, skipping"
            );
            return;
        }
    };
    let _ = tx
        .send(CellToBaseMsg::RefreshAppearance {
            entity_id,
            player_id,
            holstered,
        })
        .await;
}

/// Drain the attacker's dirty stats and push `onStatUpdate` (method 20) to its
/// client. Used by `handle_use_ability` after a successful ammo consume — and
/// crucially before any early-return that follows the consume — so the client
/// always sees the AmmoSlot{N} decrement, even when downstream lookups fail.
pub(super) async fn flush_attacker_ammo_stat(
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let payload = match space_mgr.get_entity_mut(entity_id) {
        Some(e) => {
            let p = e.stats.serialize_dirty();
            e.stats.clear_dirty();
            p
        }
        None => Vec::new(),
    };
    if !payload.is_empty() {
        send_entity_method(entity_id, 20, payload, tx, space_mgr).await;
    }
}
