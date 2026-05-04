//! Death-transition wire protocol — the ordered burst of methods sent to the
//! client when an entity's HEALTH drops to zero. Extracted from `use_ability`
//! so the protocol ordering invariants are visible in one place.
//!
//! The order on the wire is **load-bearing**:
//!
//! 1. (attacker only) `onTargetUpdate(0)` — drop the targeting reticle.
//! 2. (attacker only) `onStateFieldUpdate` with `BSF_InCombat` cleared — stops
//!    the client routing right-click on selected entities to `useAbility`.
//! 3. (NPC target only) `generate_loot_on_death` + `InteractionType` — the
//!    `InteractionType` update MUST land before the dead-state bit, otherwise
//!    the client locks in "shootable" cursor state on dead-state arrival and
//!    ignores the later flag change. Mirrors python `SGWMob.onDead()` which
//!    calls `setInteractionType` before the state field flip propagates.
//! 4. `onStateFieldUpdate` with the corpse's new state (dead bit set) — flips
//!    visuals + cursor.
//!
//! Caller is responsible for the death-side state mutations on the entity
//! itself (HEALTH=0, BSF_Dead set, AI state Dead, etc.) — this module only
//! handles outbound messages and the attacker's BSF_InCombat clear.

use tokio::sync::mpsc;

use super::super::messages::CellToBaseMsg;
use super::super::space_manager::SpaceManager;
use super::loot_drop::generate_loot_on_death;
use super::messaging::send_entity_method;

/// Apply the death-transition message sequence for a target that just died.
///
/// Ordering and side effects are described at the module level. `target_state`
/// is the corpse's already-mutated `state_field` (with `BSF_Dead` set), passed
/// in by the caller because the caller already had a mutable borrow.
pub(super) async fn apply_death_transition(
    target_eid: u32,
    attacker_id: u32,
    target_state: u32,
    attacker_is_player: bool,
    target_is_player: bool,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    // 1. Attacker side: clear targeting reticle.
    if attacker_is_player {
        send_entity_method(
            attacker_id,
            crate::mercury::method_idx::ON_TARGET_UPDATE,
            0i32.to_le_bytes().to_vec(),
            tx,
            space_mgr,
        )
        .await;
    }

    // 2. Drop the dying NPC from EVERY player's threatened_mobs set —
    //    not just the killer's. Multiple players can have the same mob on
    //    their threat lists; clearing only the killer's BSF_InCombat would
    //    leave others stuck in combat-ready cursor mode after the only mob
    //    they were threatened by died. Mirrors python `SGWPlayer.on
    //    RemovedFromThreatList` fanout from `SGWMob.onDead`.
    if !target_is_player {
        let to_broadcast =
            crate::cell::combat::clear_dead_npc_from_all_player_threat(space_mgr, target_eid);
        for (player_id, new_state) in to_broadcast {
            tracing::debug!(
                player_id,
                dying_npc = target_eid,
                new_state,
                "death: clearing player BSF_InCombat (last threatened mob died)"
            );
            send_entity_method(
                player_id,
                crate::mercury::method_idx::ON_STATE_FIELD_UPDATE,
                new_state.to_le_bytes().to_vec(),
                tx,
                space_mgr,
            )
            .await;
        }
    }

    // 3. Target side: roll loot then push interaction flags. Player targets
    //    don't loot or change interaction type.
    if !target_is_player {
        generate_loot_on_death(target_eid, space_mgr);

        let interaction_flags = space_mgr
            .get_entity(target_eid)
            .map_or(0i64, |e| e.interaction_type_flags);
        send_entity_method(
            target_eid,
            crate::mercury::method_idx::INTERACTION_TYPE,
            (interaction_flags as u64).to_le_bytes().to_vec(),
            tx,
            space_mgr,
        )
        .await;
    }

    // 4. Flip dead-state bit on the corpse — visuals + cursor change client-side.
    send_entity_method(
        target_eid,
        crate::mercury::method_idx::ON_STATE_FIELD_UPDATE,
        target_state.to_le_bytes().to_vec(),
        tx,
        space_mgr,
    )
    .await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::space_manager::SpaceManager;
    use crate::mercury::method_idx;

    /// Build a `SpaceManager` with one player at id=1 and one NPC at id=2
    /// in the SAME startup space. We use the non-instanced "Castle" world
    /// because instanced worlds (like Castle_CellBlock) allocate a fresh
    /// space on every `create_entity` call — putting the player and NPC
    /// in different spaces, where AoI never sees them.
    ///
    /// `connect_entity(1)` + an AoI tick populates the player's witness
    /// set so messages addressed to the NPC fan out via
    /// `WitnessEntityMethod` instead of being dropped at the empty-witness
    /// branch of `send_entity_method`.
    fn make_mgr_with_player_and_npc() -> SpaceManager {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
        )
        .unwrap();
        mgr.create_entity(1, "Castle", [0.0; 3], [0.0; 3]).unwrap();
        mgr.create_entity(2, "Castle", [0.0; 3], [0.0; 3]).unwrap();
        if let Some(p) = mgr.get_entity_mut(1) {
            p.is_player = true;
            p.player_id = Some(100);
        }
        mgr.connect_entity(1);
        let _ = mgr.compute_aoi_changes();
        mgr
    }

    /// Drain everything currently sitting in the channel into a Vec.
    fn drain(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<CellToBaseMsg> {
        let mut out = Vec::new();
        while let Ok(m) = rx.try_recv() {
            out.push(m);
        }
        out
    }

    /// Extract `(entity_id, method_index)` pairs for both player-direct and
    /// NPC-witness routes. Tests compare ordering on this projection so the
    /// exact wire enum variant doesn't matter — only that the right method
    /// targeted the right entity in the right order.
    fn methods(msgs: &[CellToBaseMsg]) -> Vec<(u32, u16)> {
        msgs.iter()
            .filter_map(|m| match m {
                CellToBaseMsg::EntityMethodCall {
                    entity_id,
                    method_index,
                    ..
                } => Some((*entity_id, *method_index)),
                CellToBaseMsg::WitnessEntityMethod {
                    entity_id,
                    method_index,
                    ..
                } => Some((*entity_id, *method_index)),
                _ => None,
            })
            .collect()
    }

    /// NPC target killed by player attacker — full burst:
    ///   1. `onTargetUpdate(0)` to attacker (drop reticle)
    ///   2. INTERACTION_TYPE on the corpse
    ///   3. `onStateFieldUpdate` on the corpse with dead bit
    ///
    /// (No threatened-mob clear because the player wasn't actually threatened
    /// by this NPC in this fixture — combat::clear_dead_npc_from_all_player_threat
    /// returns an empty vec when nobody had it on their threat list.)
    ///
    /// The INTERACTION_TYPE-before-state-update ordering is load-bearing per
    /// the module-level docs; this test pins it.
    #[tokio::test]
    async fn npc_target_player_attacker_emits_full_burst_in_order() {
        let mut mgr = make_mgr_with_player_and_npc();
        if let Some(npc) = mgr.get_entity_mut(2) {
            npc.interaction_type_flags = 1 << 5; // pre-existing bit must survive
        }
        let target_state = 0x80; // BSF_DEAD bit pretend-set
        let (tx, mut rx) = mpsc::channel(32);

        apply_death_transition(2, 1, target_state, true, false, &tx, &mut mgr).await;

        let msgs = drain(&mut rx);
        let pairs = methods(&msgs);
        // Locate the three load-bearing entries in order.
        let ix_target = pairs
            .iter()
            .position(|p| *p == (1, method_idx::ON_TARGET_UPDATE))
            .expect("attacker should receive onTargetUpdate(0)");
        let ix_int = pairs
            .iter()
            .position(|p| *p == (2, method_idx::INTERACTION_TYPE))
            .expect("corpse should receive INTERACTION_TYPE");
        let ix_state = pairs
            .iter()
            .position(|p| *p == (2, method_idx::ON_STATE_FIELD_UPDATE))
            .expect("corpse should receive onStateFieldUpdate");
        assert!(
            ix_target < ix_int && ix_int < ix_state,
            "ordering must be onTargetUpdate -> INTERACTION_TYPE -> onStateFieldUpdate; got {pairs:?}"
        );
    }

    /// NPC attacker killing an NPC: no reticle drop on a non-player attacker.
    /// The corpse-side burst still fires.
    #[tokio::test]
    async fn npc_attacker_skips_on_target_update() {
        let mut mgr = make_mgr_with_player_and_npc();
        // Re-flag entity 1 as an NPC so `attacker_is_player = false`.
        if let Some(p) = mgr.get_entity_mut(1) {
            p.is_player = false;
            p.player_id = None;
        }
        let (tx, mut rx) = mpsc::channel(32);

        apply_death_transition(2, 1, 0x80, false, false, &tx, &mut mgr).await;

        let pairs = methods(&drain(&mut rx));
        assert!(
            !pairs.contains(&(1, method_idx::ON_TARGET_UPDATE)),
            "non-player attacker must not receive onTargetUpdate; got {pairs:?}"
        );
    }

    /// Player target dying: skip both the threatened-mob clear (target is a
    /// player, not a dying NPC) and the corpse-side INTERACTION_TYPE update
    /// (player corpses don't loot). The reticle drop and state-field flip
    /// still fire.
    #[tokio::test]
    async fn player_target_skips_interaction_type_and_threat_clear() {
        let mut mgr = make_mgr_with_player_and_npc();
        // entity 2 is the dying target; promote it to a player.
        if let Some(t) = mgr.get_entity_mut(2) {
            t.is_player = true;
            t.player_id = Some(200);
        }
        let (tx, mut rx) = mpsc::channel(32);

        apply_death_transition(2, 1, 0x80, true, true, &tx, &mut mgr).await;

        let pairs = methods(&drain(&mut rx));
        assert!(
            !pairs.contains(&(2, method_idx::INTERACTION_TYPE)),
            "player target must not receive INTERACTION_TYPE; got {pairs:?}"
        );
        assert!(
            pairs.contains(&(2, method_idx::ON_STATE_FIELD_UPDATE)),
            "player target must still receive onStateFieldUpdate; got {pairs:?}"
        );
    }

    /// INTERACTION_TYPE payload is the entity's `interaction_type_flags`
    /// re-cast to `u64` and serialized little-endian. A refactor that
    /// truncates to u32 (the column is `i64`) would silently lose the
    /// high-bit `INT_NormalLoot` (1<<62), so we pin the byte layout.
    #[tokio::test]
    async fn interaction_type_payload_is_little_endian_u64_of_full_flags() {
        let mut mgr = make_mgr_with_player_and_npc();
        let high_bit_flag: i64 = 1 << 62; // INT_NormalLoot equivalent
        if let Some(npc) = mgr.get_entity_mut(2) {
            npc.interaction_type_flags = high_bit_flag;
        }
        let (tx, mut rx) = mpsc::channel(32);

        apply_death_transition(2, 1, 0x80, true, false, &tx, &mut mgr).await;

        let msgs = drain(&mut rx);
        let int_msg = msgs
            .iter()
            .find_map(|m| match m {
                CellToBaseMsg::WitnessEntityMethod {
                    entity_id,
                    method_index,
                    args,
                    ..
                }
                | CellToBaseMsg::EntityMethodCall {
                    entity_id,
                    method_index,
                    args,
                } if *entity_id == 2 && *method_index == method_idx::INTERACTION_TYPE => {
                    Some(args.clone())
                }
                _ => None,
            })
            .expect("INTERACTION_TYPE must be sent for an NPC target");
        assert_eq!(
            int_msg.len(),
            8,
            "INTERACTION_TYPE payload must be exactly 8 bytes"
        );
        let u = u64::from_le_bytes(int_msg.try_into().unwrap());
        assert_eq!(
            u, high_bit_flag as u64,
            "payload must reproduce the full i64 flag bits as little-endian u64 — preserves the high INT_NormalLoot bit"
        );
    }

    /// `ON_STATE_FIELD_UPDATE` for the corpse carries the caller's already-
    /// mutated `target_state`. Pin the byte layout so a refactor that
    /// re-reads from the entity (after the threat-clear step has run!)
    /// can't silently drop the BSF_Dead bit.
    #[tokio::test]
    async fn corpse_state_field_update_carries_caller_supplied_state() {
        let mut mgr = make_mgr_with_player_and_npc();
        let (tx, mut rx) = mpsc::channel(32);
        let target_state: u32 = 0x1234_5678;

        apply_death_transition(2, 1, target_state, true, false, &tx, &mut mgr).await;

        let msgs = drain(&mut rx);
        // Find the LAST onStateFieldUpdate addressed at the corpse — the
        // module ships several on-state-field-update messages in this flow
        // (one optional per-player threat-clear, then the final corpse one);
        // the corpse one is the load-bearing tail.
        let final_state_args = msgs
            .iter()
            .rev()
            .find_map(|m| match m {
                CellToBaseMsg::WitnessEntityMethod {
                    entity_id,
                    method_index,
                    args,
                    ..
                }
                | CellToBaseMsg::EntityMethodCall {
                    entity_id,
                    method_index,
                    args,
                } if *entity_id == 2 && *method_index == method_idx::ON_STATE_FIELD_UPDATE => {
                    Some(args.clone())
                }
                _ => None,
            })
            .expect("corpse must receive onStateFieldUpdate");
        assert_eq!(final_state_args.len(), 4);
        assert_eq!(
            u32::from_le_bytes(final_state_args.try_into().unwrap()),
            target_state
        );
    }
}
