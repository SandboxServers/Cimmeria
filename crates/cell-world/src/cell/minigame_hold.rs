//! The "playing a minigame" hold: `BSF_PlayingMinigame` plus one counted
//! `BSF_MovementLock` reference (D-MG15).
//!
//! The launch (`StartMinigame` content action) takes the hold; every end path
//! releases it through the cell's `MinigameResult` handler, whatever the result
//! code. Both flags go through the ref-counted entity helpers: a stun or a
//! death that already holds the movement lock keeps its own reference, so the
//! minigame's release cannot free a stunned player.
//!
//! One `onStateFieldUpdate` per transition, not one per flag: the client reads
//! the whole word, and two sends back to back would flash the lock on without
//! the playing bit.

use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::mercury::method_idx::ON_STATE_FIELD_UPDATE;
use cimmeria_wire::state_field::{BSF_MOVEMENT_LOCK, BSF_PLAYING_MINIGAME, STATE_FLAGS};

/// Take the hold for a launching player. Idempotent: a launch while the bit is
/// already set (a second click, or a reopen before the old result arrived)
/// takes no second reference, so one release always frees it.
///
/// Returns whether this call took the hold.
pub async fn hold_for_minigame(
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    let Some(e) = space_mgr.get_entity_mut(entity_id) else {
        return false;
    };
    if e.state_field & BSF_PLAYING_MINIGAME != 0 {
        return false;
    }
    e.set_state_flag(BSF_PLAYING_MINIGAME);
    e.set_state_flag(BSF_MOVEMENT_LOCK);
    let state = e.state_field;
    send_state(entity_id, state, "hold", tx, space_mgr).await;
    true
}

/// Release the hold on any minigame result. A no-op when the bit is clear, so
/// a late or duplicate result cannot drop a lock someone else took.
///
/// Returns whether this call released it.
pub async fn release_minigame_hold(
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    let Some(e) = space_mgr.get_entity_mut(entity_id) else {
        return false;
    };
    if e.state_field & BSF_PLAYING_MINIGAME == 0 {
        return false;
    }
    let before = e.state_field;
    e.unset_state_flag(BSF_PLAYING_MINIGAME);
    e.unset_state_flag(BSF_MOVEMENT_LOCK);
    let state = e.state_field;
    if state != before {
        send_state(entity_id, state, "release", tx, space_mgr).await;
    }
    true
}

async fn send_state(
    entity_id: u32,
    state: u32,
    action: &'static str,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    if let Err(e) = tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: ON_STATE_FIELD_UPDATE,
            args: state.to_le_bytes().to_vec(),
        })
        .await
    {
        // The next state-field write resends the whole word, but until then
        // the client keeps its old view of the lock.
        tracing::warn!(
            entity_id,
            entity_name = space_mgr.entity_label(entity_id),
            action,
            state_flags = %STATE_FLAGS.render(state),
            error = %e,
            reason = "cell_to_base_send_failed",
            "Minigame hold: onStateFieldUpdate send failed"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mgr_with_player() -> SpaceManager {
        let mut mgr = SpaceManager::new(1);
        mgr.parse_spaces_xml(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#,
        )
        .unwrap();
        mgr.create_startup_spaces(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
        )
        .unwrap();
        mgr.create_entity(1, "Castle", [0.0; 3], [0.0; 3]).unwrap();
        mgr.get_entity_mut(1).unwrap().is_player = true;
        mgr
    }

    const BOTH: u32 = BSF_PLAYING_MINIGAME | BSF_MOVEMENT_LOCK;

    #[tokio::test]
    async fn hold_sets_both_bits_and_release_clears_both() {
        let mut mgr = mgr_with_player();
        let (tx, mut rx) = mpsc::channel(8);

        assert!(hold_for_minigame(1, &tx, &mut mgr).await);
        assert_eq!(mgr.get_entity(1).unwrap().state_field & BOTH, BOTH);
        assert!(release_minigame_hold(1, &tx, &mut mgr).await);
        assert_eq!(mgr.get_entity(1).unwrap().state_field & BOTH, 0);

        // One update per transition, carrying the whole word.
        for want in [BOTH, 0] {
            match rx.try_recv() {
                Ok(CellToBaseMsg::EntityMethodCall { args, .. }) => {
                    assert_eq!(args, want.to_le_bytes().to_vec());
                }
                other => panic!("expected onStateFieldUpdate, got {other:?}"),
            }
        }
        assert!(rx.try_recv().is_err());
    }

    /// A stun that took the movement lock first keeps its own reference:
    /// the release must leave bit 6 set. Revert: raw `&= !BOTH` in release.
    #[tokio::test]
    async fn release_leaves_a_lock_another_source_holds() {
        let mut mgr = mgr_with_player();
        mgr.get_entity_mut(1)
            .unwrap()
            .set_state_flag(BSF_MOVEMENT_LOCK);
        let (tx, _rx) = mpsc::channel(8);

        hold_for_minigame(1, &tx, &mut mgr).await;
        release_minigame_hold(1, &tx, &mut mgr).await;

        let state = mgr.get_entity(1).unwrap().state_field;
        assert_eq!(state & BSF_PLAYING_MINIGAME, 0);
        assert_eq!(
            state & BSF_MOVEMENT_LOCK,
            BSF_MOVEMENT_LOCK,
            "stun lock survives"
        );
    }

    /// A second launch takes no second reference, so one release frees it;
    /// a release with nothing held is a silent no-op.
    #[tokio::test]
    async fn hold_is_idempotent_and_release_without_hold_is_a_no_op() {
        let mut mgr = mgr_with_player();
        let (tx, mut rx) = mpsc::channel(8);

        assert!(!release_minigame_hold(1, &tx, &mut mgr).await);
        assert!(hold_for_minigame(1, &tx, &mut mgr).await);
        assert!(!hold_for_minigame(1, &tx, &mut mgr).await);
        assert!(release_minigame_hold(1, &tx, &mut mgr).await);

        assert_eq!(mgr.get_entity(1).unwrap().state_field & BOTH, 0);
        let mut sent = 0;
        while rx.try_recv().is_ok() {
            sent += 1;
        }
        assert_eq!(sent, 2, "one hold update, one release update");
    }
}
