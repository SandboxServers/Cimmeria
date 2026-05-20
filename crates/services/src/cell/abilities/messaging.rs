//! Entity-method routing helpers for the cell-side wire dispatch.
//!
//! Three routing modes live here:
//!
//! - [`send_entity_method`] — entity-aware default. Player → self only; NPC →
//!   witnesses only. Used by paths where the state change is meaningful to
//!   one audience (the owner OR the observers, not both).
//! - [`send_entity_method_to_witnesses`] — strict witness-only fanout. Never
//!   sends to the entity's own client even if it's a player. Use when the
//!   event is for observers regardless of who owns the entity (e.g., a
//!   corpse-loot indicator, an NPC cleanup signal).
//! - [`send_entity_method_to_self_and_witnesses`] — owner + witnesses. Use for
//!   player state changes that must propagate to other players in AoI — the
//!   five cases in [#278](https://github.com/SandboxServers/Cimmeria/issues/278):
//!   `BSF_IN_COMBAT` flip, death/respawn state-field, `BSF_HOLSTER` posture,
//!   `BeingAppearance` equip recomposite, `setMovementType`. For NPC entities
//!   the "self" send is degenerate (NPCs have no client) and this collapses
//!   to the witnesses-only path.
//!
//! Also hosts the dirty-stat flush helper that pushes a queued `onStatUpdate`
//! to the attacker's client after an ammo decrement.

use tokio::sync::mpsc;

use super::super::messages::CellToBaseMsg;
use super::super::space_manager::SpaceManager;

/// Send an entity method call, routing to the entity's client if it's a player,
/// or broadcasting to all witnessing players if it's an NPC (ghost entity).
///
/// In BigWorld, method calls on ghost entities are forwarded to all players who
/// have that entity in their AoI. This is how players see NPC attack animations,
/// health changes, death states, etc.
///
/// This is the **entity-aware default**. If you need a player's state change
/// to also reach other players in AoI, use
/// [`send_entity_method_to_self_and_witnesses`] instead — this function alone
/// does not fan out for players.
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

/// Fan out an entity-method call to all AoI witnesses of `entity_id`.
///
/// Strict witness-only: never sends to `entity_id`'s own client even if it's
/// a player. Returns the witness count actually addressed so callers can
/// `tracing::debug!` the fanout shape without re-querying the space manager.
///
/// An empty result (no AoI witnesses) is a debug-level signal, not a warning —
/// many state changes legitimately have no observers (player alone in a space,
/// NPC outside any player's AoI, etc.). Use [`send_entity_method`] if you want
/// the "NPC with no witnesses" warning — that signal indicates a routing bug
/// for ghost entities, which doesn't apply when the caller has explicitly
/// asked for witness-only fanout.
//
// Allowed until #278's first child PR (#232, #249, #270, …) adopts this
// helper at a callsite. Tests in this file exercise it directly.
#[allow(dead_code)]
pub(crate) async fn send_entity_method_to_witnesses(
    entity_id: u32,
    method_index: u16,
    args: Vec<u8>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) -> usize {
    let witnesses = space_mgr.get_witnesses_of(entity_id);
    let count = witnesses.len();
    if count == 0 {
        tracing::debug!(
            entity_id,
            method_index,
            "send_entity_method_to_witnesses: no witnesses; nothing emitted"
        );
        return 0;
    }
    for witness_id in witnesses {
        let _ = tx
            .send(CellToBaseMsg::WitnessEntityMethod {
                witness_id,
                entity_id,
                method_index,
                args: args.clone(),
            })
            .await;
    }
    tracing::debug!(
        entity_id,
        method_index,
        witness_count = count,
        "send_entity_method_to_witnesses: fanned out"
    );
    count
}

/// Emit an entity-method call to `entity_id`'s own client AND fan out to all
/// AoI witnesses.
///
/// This is the helper [#278](https://github.com/SandboxServers/Cimmeria/issues/278)
/// names — use it for player state changes that must also propagate to other
/// players who can see them (BSF_IN_COMBAT, death/respawn flips, holster pose,
/// equip BeingAppearance, movement-type transitions).
///
/// For an NPC entity (`is_player == false`), the "self" path degenerates —
/// NPCs don't have a client — and this collapses to a witness-only fanout,
/// equivalent to [`send_entity_method_to_witnesses`]. That keeps the helper
/// callable from paths that don't statically know whether the entity is a
/// player without forcing a branch at every callsite.
///
/// Returns the witness count actually addressed (excludes the self send).
//
// Allowed until #278's first child PR (#232, #249, #270, …) adopts this
// helper at a callsite. Tests in this file exercise it directly.
#[allow(dead_code)]
pub(crate) async fn send_entity_method_to_self_and_witnesses(
    entity_id: u32,
    method_index: u16,
    args: Vec<u8>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) -> usize {
    let is_player = space_mgr.get_entity(entity_id).is_some_and(|e| e.is_player);

    if is_player {
        let _ = tx
            .send(CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index,
                args: args.clone(),
            })
            .await;
    }
    // For NPCs the self send is a no-op; the witness fanout below is the only
    // path that does anything. For players the fanout sits on top of the self
    // send above. Either way the witnesses-only helper handles the empty case
    // (no warning, debug-only) so an alone-in-space player doesn't log noise.
    send_entity_method_to_witnesses(entity_id, method_index, args, tx, space_mgr).await
}

/// Send a `CellToBaseMsg::RefreshAppearance` for a player entity, reading
/// the player's current `weapon_holstered` state off the cell entity.
///
/// Called from the combat enter/exit broadcast sites after
/// `onStateFieldUpdate` so a draw or holster reaches the wire in the same
/// dispatch burst as the BSF_InCombat change. No-op (with a debug log)
/// for non-player entities or for players whose `player_id` (DB id) hasn't
/// been populated yet — both happen during transient world-entry races
/// and we'd rather drop the rebroadcast than send junk.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::messages::CellToBaseMsg;
    use crate::cell::space_manager::SpaceManager;

    /// Two players + one NPC in the same Castle space. Both players are
    /// connected and AoI is computed, so each player sees the others +
    /// the NPC. Returns the manager + an mpsc rx for asserting on emitted
    /// `CellToBaseMsg` traffic.
    fn make_mgr_two_players_and_npc() -> (SpaceManager, mpsc::Receiver<CellToBaseMsg>) {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
        )
        .unwrap();
        // Two players + one NPC, all co-located so AoI naturally captures all.
        mgr.create_entity(1, "Castle", [0.0; 3], [0.0; 3]).unwrap();
        mgr.create_entity(2, "Castle", [0.0; 3], [0.0; 3]).unwrap();
        mgr.create_entity(3, "Castle", [0.0; 3], [0.0; 3]).unwrap();
        if let Some(p) = mgr.get_entity_mut(1) {
            p.is_player = true;
            p.player_id = Some(100);
        }
        if let Some(p) = mgr.get_entity_mut(2) {
            p.is_player = true;
            p.player_id = Some(200);
        }
        // entity 3 stays an NPC.
        mgr.connect_entity(1);
        mgr.connect_entity(2);
        let _ = mgr.compute_aoi_changes();
        let (_tx, rx) = mpsc::channel(64);
        (mgr, rx)
    }

    fn drain(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<CellToBaseMsg> {
        let mut out = Vec::new();
        while let Ok(msg) = rx.try_recv() {
            out.push(msg);
        }
        out
    }

    /// Witness-only fanout for a player who has one other player in AoI:
    /// emits exactly one `WitnessEntityMethod` to that other player, and
    /// zero `EntityMethodCall` to self.
    #[tokio::test]
    async fn witnesses_only_fanout_skips_self_and_addresses_each_observer() {
        let (mgr, _rx) = make_mgr_two_players_and_npc();
        let (tx, mut rx) = mpsc::channel(64);
        let count = send_entity_method_to_witnesses(
            1,
            19, // ON_STATE_FIELD_UPDATE — arbitrary; the helper is method-agnostic
            vec![0xDE, 0xAD],
            &tx,
            &mgr,
        )
        .await;
        // Player 2 sees player 1, so exactly one witness.
        assert_eq!(count, 1);
        let msgs = drain(&mut rx);
        assert_eq!(msgs.len(), 1);
        match &msgs[0] {
            CellToBaseMsg::WitnessEntityMethod {
                witness_id,
                entity_id,
                method_index,
                args,
            } => {
                assert_eq!(*witness_id, 2);
                assert_eq!(*entity_id, 1);
                assert_eq!(*method_index, 19);
                assert_eq!(args, &vec![0xDE, 0xAD]);
            }
            other => panic!("expected WitnessEntityMethod, got {other:?}"),
        }
    }

    /// Witness-only with no observers: returns 0, emits nothing, does NOT
    /// log a warning. This is the path a player alone in a space hits when
    /// their state flips — the helper must stay silent rather than spam.
    #[tokio::test]
    async fn witnesses_only_with_no_observers_is_a_clean_zero() {
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
        let (tx, mut rx) = mpsc::channel(64);

        let count = send_entity_method_to_witnesses(1, 19, vec![], &tx, &mgr).await;
        assert_eq!(count, 0);
        assert!(drain(&mut rx).is_empty());
    }

    /// Self + witnesses for a player: one `EntityMethodCall` to self, one
    /// `WitnessEntityMethod` per observer. Returns the witness count
    /// (not counting the self send).
    #[tokio::test]
    async fn self_and_witnesses_for_player_sends_both() {
        let (mgr, _rx) = make_mgr_two_players_and_npc();
        let (tx, mut rx) = mpsc::channel(64);

        let witness_count =
            send_entity_method_to_self_and_witnesses(1, 19, vec![0xBE, 0xEF], &tx, &mgr).await;
        assert_eq!(witness_count, 1);

        let msgs = drain(&mut rx);
        let self_sends: Vec<_> = msgs
            .iter()
            .filter(|m| matches!(m, CellToBaseMsg::EntityMethodCall { entity_id: 1, .. }))
            .collect();
        let witness_sends: Vec<_> = msgs
            .iter()
            .filter(|m| matches!(m, CellToBaseMsg::WitnessEntityMethod { entity_id: 1, .. }))
            .collect();
        assert_eq!(self_sends.len(), 1, "expected exactly one self send");
        assert_eq!(witness_sends.len(), 1, "expected exactly one witness send");
    }

    /// Self + witnesses for an NPC: the "self" path is a no-op (NPCs have
    /// no client), and the helper collapses to witnesses-only. Verifies
    /// no `EntityMethodCall` is emitted for the NPC.
    #[tokio::test]
    async fn self_and_witnesses_for_npc_skips_self_send() {
        let (mgr, _rx) = make_mgr_two_players_and_npc();
        let (tx, mut rx) = mpsc::channel(64);

        // Entity 3 is the NPC; both players witness it.
        let witness_count =
            send_entity_method_to_self_and_witnesses(3, 19, vec![], &tx, &mgr).await;
        // Both players are co-located and see the NPC.
        assert_eq!(witness_count, 2);

        let msgs = drain(&mut rx);
        let self_sends: Vec<_> = msgs
            .iter()
            .filter(|m| matches!(m, CellToBaseMsg::EntityMethodCall { entity_id: 3, .. }))
            .collect();
        let witness_sends: Vec<_> = msgs
            .iter()
            .filter(|m| matches!(m, CellToBaseMsg::WitnessEntityMethod { entity_id: 3, .. }))
            .collect();
        assert!(
            self_sends.is_empty(),
            "NPC must not receive an EntityMethodCall — NPCs have no client"
        );
        assert_eq!(witness_sends.len(), 2);
    }

    /// Behavior parity: `send_entity_method` for an NPC fans out to the same
    /// witness set `send_entity_method_to_witnesses` would. This pins that
    /// the new witness-only helper is a non-disruptive extension — paths
    /// that already use the entity-aware default keep their existing
    /// behavior unchanged when the new helper lands.
    #[tokio::test]
    async fn npc_send_entity_method_matches_witnesses_only_helper() {
        let (mgr, _rx) = make_mgr_two_players_and_npc();
        let (tx_a, mut rx_a) = mpsc::channel(64);
        let (tx_b, mut rx_b) = mpsc::channel(64);

        send_entity_method(3, 19, vec![1, 2, 3], &tx_a, &mgr).await;
        let count_b = send_entity_method_to_witnesses(3, 19, vec![1, 2, 3], &tx_b, &mgr).await;

        let msgs_a = drain(&mut rx_a);
        let msgs_b = drain(&mut rx_b);
        assert_eq!(msgs_a.len(), msgs_b.len());
        assert_eq!(msgs_a.len(), count_b);
    }
}
