//! SGWCombatant interface exposed CellMethods (indices 5–7).
//!
//! These methods handle player-initiated state changes (crouch, weapon holster)
//! and must reflect the updated stateField back to the client via
//! `onStateFieldUpdate` so the animation system transitions correctly.
//!
//! Reference: `python/cell/SGWBeing.py:746-770`

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use tokio::sync::mpsc;

/// Set crouched state.
pub const SET_CROUCHED: u16 = 5;
/// Toggle heal debug overlay.
pub const TOGGLE_HEAL_DEBUG: u16 = 6;
/// Request holster/unholster weapon.
pub const REQUEST_HOLSTER_WEAPON: u16 = 7;

/// Being State Field bit positions (from Atrea.enums BSF_*).
const BSF_CROUCHING: u32 = 1 << 2;

pub async fn dispatch(
    entity_id: u32,
    method_index: u16,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    match method_index {
        SET_CROUCHED => {
            if args.is_empty() {
                return true;
            }
            let crouched = args[0] as i8;
            tracing::debug!(entity_id, crouched, "setCrouched");

            if let Some(e) = space_mgr.get_entity_mut(entity_id) {
                let old = e.state_field;
                if crouched != 0 {
                    e.state_field |= BSF_CROUCHING;
                } else {
                    e.state_field &= !BSF_CROUCHING;
                }
                if e.state_field != old {
                    let new_state = e.state_field;
                    let _ = tx
                        .send(CellToBaseMsg::EntityMethodCall {
                            entity_id,
                            method_index: 19, // onStateFieldUpdate
                            args: new_state.to_le_bytes().to_vec(),
                        })
                        .await;
                    // TODO: also send to witnesses via AoI broadcast
                }
            }
            true
        }
        TOGGLE_HEAL_DEBUG => {
            tracing::debug!(entity_id, "toggleHealDebug (stub)");
            true
        }
        REQUEST_HOLSTER_WEAPON => {
            if args.is_empty() {
                return true;
            }
            let holstered = args[0] != 0;
            tracing::debug!(entity_id, holstered, "requestHolsterWeapon");

            // Update server-side holster state on `CellEntity` rather
            // than the long-defunct `BSF_HOLSTER` bit of `state_field`
            // — the SGW client only dispatches on bits 0-7 of
            // `bStateField` (`GameBeing_OnStateFieldUpdate` at
            // `ghidra://SGW.exe@0x00e01c90`), so any write to bit 8
            // was historically a no-op. The wire effect comes from
            // `BeingAppearance.ComponentList` dropping (or keeping)
            // the weapon visual — the client's appearance compositor
            // (`ghidra://SGW.exe@0x00ec0840`) picks the weapon-pose
            // vs. holstered-pose animation key from the resulting
            // list. See `CellEntity::set_weapon_holstered` and
            // `docs/architecture/state-field-bits.md`.
            let should_rebroadcast = space_mgr
                .get_entity_mut(entity_id)
                .map(|e| e.set_weapon_holstered(holstered))
                .unwrap_or(false);
            if should_rebroadcast {
                // Phase 2 of the holster work (PR #338): re-emit
                // `BeingAppearance` to the owner + AoI witnesses with
                // the freshly-toggled `ComponentList` so the holster
                // toggle becomes visible.
                crate::cell::abilities::request_appearance_refresh(entity_id, tx, space_mgr).await;
            }
            true
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One-player Castle space — small enough fixture for cell-method
    /// dispatch tests that only need a single entity to mutate.
    fn make_mgr_with_player() -> SpaceManager {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
        )
        .unwrap();
        mgr.create_entity(1, "Castle", [0.0; 3], [0.0; 3]).unwrap();
        if let Some(p) = mgr.get_entity_mut(1) {
            p.is_player = true;
            p.player_id = Some(100);
        }
        mgr.connect_entity(1);
        let _ = mgr.compute_aoi_changes();
        mgr
    }

    /// `requestHolsterWeapon(1)` flips the entity's `weapon_holstered`
    /// from the spawn-default `true` (already holstered) by first
    /// drawing the weapon, then re-holstering. Pins that the cell
    /// method actually reaches `set_weapon_holstered` — without this,
    /// a future refactor could replace the dispatch body with a stub
    /// and the test suite wouldn't notice.
    #[tokio::test]
    async fn request_holster_weapon_toggles_entity_state() {
        let mut mgr = make_mgr_with_player();
        // Give the entity a weapon visual so the holster filter actually
        // has something to toggle (set_weapon_holstered's "signal a
        // rebroadcast" path requires weapon_visual.is_some()).
        if let Some(e) = mgr.get_entity_mut(1) {
            e.weapon_visual = Some("BS_Gun.Pistol".into());
            e.weapon_holstered = true;
        }

        let (tx, _rx) = mpsc::channel(8);

        // Draw the weapon: args[0] = 0 (not holstered).
        let handled = dispatch(1, REQUEST_HOLSTER_WEAPON, &[0u8], &tx, &mut mgr).await;
        assert!(handled, "dispatch must recognise REQUEST_HOLSTER_WEAPON");
        assert!(
            !mgr.get_entity(1).unwrap().weapon_holstered,
            "args[0] = 0 must clear weapon_holstered",
        );

        // Holster the weapon: args[0] = 1.
        let handled = dispatch(1, REQUEST_HOLSTER_WEAPON, &[1u8], &tx, &mut mgr).await;
        assert!(handled);
        assert!(
            mgr.get_entity(1).unwrap().weapon_holstered,
            "args[0] = 1 must set weapon_holstered",
        );
    }

    /// Phase 2 (PR #338): a real holster toggle (state actually changed,
    /// weapon visual present) must dispatch a `RefreshAppearance` so the
    /// AoI rebroadcast lands. The bug shape this catches: someone keeps
    /// the `set_weapon_holstered` call but drops the rebroadcast — the
    /// server state is correct but no wire packet ever ships, and the
    /// client keeps rendering the prior pose forever (the exact symptom
    /// that drove Phase 2).
    #[tokio::test]
    async fn request_holster_weapon_dispatches_refresh_appearance() {
        let mut mgr = make_mgr_with_player();
        if let Some(e) = mgr.get_entity_mut(1) {
            e.weapon_visual = Some("BS_Gun.Pistol".into());
            e.weapon_holstered = true;
        }

        let (tx, mut rx) = mpsc::channel(8);

        // Draw the weapon — this is the state-change branch.
        let _ = dispatch(1, REQUEST_HOLSTER_WEAPON, &[0u8], &tx, &mut mgr).await;

        let msg = rx
            .try_recv()
            .expect("dispatch must send a RefreshAppearance on state change");
        match msg {
            CellToBaseMsg::RefreshAppearance {
                entity_id,
                player_id,
                holstered,
            } => {
                assert_eq!(entity_id, 1);
                assert_eq!(
                    player_id, 100,
                    "must carry the DB player_id from the entity"
                );
                assert!(
                    !holstered,
                    "drawing the weapon must send holstered=false on the wire",
                );
            }
            other => panic!("expected RefreshAppearance, got {other:?}"),
        }
    }

    /// Inverse: a no-op call (already in requested state) must NOT
    /// dispatch a refresh — that would waste bandwidth and risk
    /// flickering the model on receipt. The `set_weapon_holstered`
    /// helper's change-detection is what gates the dispatch.
    #[tokio::test]
    async fn request_holster_weapon_no_op_skips_refresh_appearance() {
        let mut mgr = make_mgr_with_player();
        if let Some(e) = mgr.get_entity_mut(1) {
            e.weapon_visual = Some("BS_Gun.Pistol".into());
            e.weapon_holstered = true;
        }

        let (tx, mut rx) = mpsc::channel(8);

        // Request holster=true when already holstered — same state, no change.
        let _ = dispatch(1, REQUEST_HOLSTER_WEAPON, &[1u8], &tx, &mut mgr).await;
        assert!(
            rx.try_recv().is_err(),
            "no-op holster toggle must NOT dispatch RefreshAppearance",
        );
    }

    /// Empty payload is a benign no-op: the handler returns `true`
    /// (it recognised the method) but doesn't touch entity state.
    /// The previous in-combat client sometimes emitted these as part
    /// of state-sync; we don't crash on them.
    #[tokio::test]
    async fn request_holster_weapon_with_empty_args_is_noop() {
        let mut mgr = make_mgr_with_player();
        if let Some(e) = mgr.get_entity_mut(1) {
            e.weapon_visual = Some("BS_Gun.Pistol".into());
            e.weapon_holstered = true;
        }

        let (tx, _rx) = mpsc::channel(8);
        let handled = dispatch(1, REQUEST_HOLSTER_WEAPON, &[], &tx, &mut mgr).await;
        assert!(handled);
        assert!(
            mgr.get_entity(1).unwrap().weapon_holstered,
            "empty args must NOT touch weapon_holstered",
        );
    }

    /// Unknown method indices fall through — the dispatch returns
    /// `false` so the outer router can try the next handler.
    #[tokio::test]
    async fn dispatch_returns_false_for_unhandled_method() {
        let mut mgr = make_mgr_with_player();
        let (tx, _rx) = mpsc::channel(8);
        let handled = dispatch(1, 999, &[0u8], &tx, &mut mgr).await;
        assert!(
            !handled,
            "unknown method indices must return false so the router can chain",
        );
    }
}
