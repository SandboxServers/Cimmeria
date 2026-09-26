//! Player-ghost identity join — the base half of introducing one player to
//! another.
//!
//! When a player enters a witness's AoI the cell ships the live state it owns
//! ([`PlayerAoIData`]: state field, target, public stats, ammo type). What a
//! witness actually *sees* — name, level, archetype, alignment, the
//! `BeingAppearance` body and tint — lives on the observee's base session,
//! where holster / equip / bandolier changes keep `cached_appearance_args`
//! current. This module reads that session at emit time and picks the phase-2
//! cascade body: the player-ghost cascade when both halves are present, the
//! bare cascade (with a WARN) when the session half cannot be resolved.
//!
//! Joining at emit time rather than when the cell fires the event matters on
//! the deferred path: a witness still loading the map has its `EnteredAoI`
//! buffered for seconds, and the observee may re-holster in the meantime.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use crate::cell::messages::{NpcAoIData, PlayerAoIData};
use crate::mercury::{
    compose_create_entity_cascade_body, compose_player_ghost_cascade_body, PlayerGhostCascade,
};

use super::super::super::ConnectedClientState;

/// The observee's session-owned identity, cloned out from under the
/// `connected` lock so the cascade can be composed without holding it.
#[derive(Debug, Clone)]
pub(super) struct PlayerGhostIdentity {
    name: String,
    level: i32,
    archetype: i32,
    alignment: u8,
    appearance_args: Option<Vec<u8>>,
    tint_args: Option<Vec<u8>>,
}

impl PlayerGhostIdentity {
    /// Borrow this identity together with the cell's live snapshot as the
    /// composer's input.
    pub(super) fn cascade<'a>(&'a self, live: &'a PlayerAoIData) -> PlayerGhostCascade<'a> {
        PlayerGhostCascade {
            name: &self.name,
            level: self.level,
            archetype: self.archetype,
            alignment: self.alignment,
            appearance_args: self.appearance_args.as_deref(),
            tint_args: self.tint_args.as_deref(),
            live,
        }
    }
}

/// Resolve the session identity of the player `entity_id` for a witness.
///
/// `None` (with a WARN) when the observee has no resolvable session — the
/// cell still holds its entity but the base has already torn the session
/// down, or never registered it. A resolved identity with no cached
/// appearance is returned as-is and WARNed: the ghost will carry a name and
/// stats but no body, which is strictly more useful than dropping it.
pub(super) fn resolve_identity(
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    witness_id: u32,
    entity_id: u32,
) -> Option<PlayerGhostIdentity> {
    let addr = entity_to_addr
        .lock()
        .ok()
        .and_then(|m| m.get(&entity_id).copied());
    let identity = addr.and_then(|addr| {
        let clients = connected.lock().ok()?;
        let c = clients.get(&addr)?;
        Some(PlayerGhostIdentity {
            name: c.player_name.clone().unwrap_or_default(),
            level: c.player_level.unwrap_or(1),
            archetype: c.player_archetype.unwrap_or(0),
            alignment: c.player_alignment.unwrap_or(0) as u8,
            appearance_args: c.cached_appearance_args.clone(),
            tint_args: c.cached_tint_args.clone(),
        })
    });

    match &identity {
        None => tracing::warn!(
            target: "aoi.player_ghost_incomplete",
            witness_id,
            entity_id,
            addr_resolved = addr.is_some(),
            reason = "observee_session_unresolved",
            "Player entered AoI but its base session could not be resolved -- \
             witness gets the bare cascade (no name, no appearance)"
        ),
        Some(id) if id.appearance_args.is_none() => tracing::warn!(
            target: "aoi.player_ghost_incomplete",
            witness_id,
            entity_id,
            reason = "no_cached_appearance",
            "Player entered AoI with no cached BeingAppearance -- witness will see \
             a named entity with no body until the next appearance rebroadcast"
        ),
        Some(_) => {}
    }
    identity
}

/// Compose the phase-2 cascade body for an entity entering `witness_id`'s
/// AoI: the player-ghost cascade for a player whose session resolves, the
/// NPC / bare cascade otherwise.
pub(super) fn compose_cascade_body(
    witness_id: u32,
    entity_id: u32,
    class_id: u8,
    level: u32,
    npc_data: Option<&NpcAoIData>,
    player_data: Option<&PlayerAoIData>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) -> Vec<u8> {
    if let Some(live) = player_data {
        if let Some(identity) = resolve_identity(connected, entity_to_addr, witness_id, entity_id) {
            return compose_player_ghost_cascade_body(entity_id, &identity.cascade(live));
        }
    }
    compose_create_entity_cascade_body(entity_id, class_id, level, npc_data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base::world_entry::cell_dispatch::aoi;
    use crate::test_support::{test_default_connected_client_state, LogCapture, TestTransport};
    use cimmeria_mercury::encryption::MercuryEncryption;
    use cimmeria_mercury::transport::Transport;
    use tracing::Level;

    const WITNESS: u32 = 100;
    const OBSERVEE: u32 = 200;

    type Connected = Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>;
    type EntityToAddr = Arc<Mutex<HashMap<u32, SocketAddr>>>;

    fn witness_addr() -> SocketAddr {
        "127.0.0.1:50300".parse().unwrap()
    }

    fn observee_addr() -> SocketAddr {
        "127.0.0.1:50301".parse().unwrap()
    }

    /// An in-world observee session: named, levelled, with a cached
    /// appearance, which is the state `play_character` + `map_loaded` leave
    /// behind.
    fn observee_state() -> ConnectedClientState {
        let mut s = test_default_connected_client_state();
        s.player_name = Some("Lomiada".to_string());
        s.player_level = Some(9);
        s.player_archetype = Some(4);
        s.player_alignment = Some(2);
        s.cached_appearance_args = Some(vec![0xAA, 0xBB]);
        s.cached_tint_args = Some(vec![0; 12]);
        s
    }

    fn two_sessions(observee: Option<ConnectedClientState>) -> (Connected, EntityToAddr) {
        let mut sessions = HashMap::from([(witness_addr(), test_default_connected_client_state())]);
        let mut addrs = HashMap::from([(WITNESS, witness_addr())]);
        if let Some(state) = observee {
            sessions.insert(observee_addr(), state);
            addrs.insert(OBSERVEE, observee_addr());
        }
        (Arc::new(Mutex::new(sessions)), Arc::new(Mutex::new(addrs)))
    }

    fn live() -> PlayerAoIData {
        PlayerAoIData {
            state_field: 0b1000,
            target_id: 77,
            ammo_type_id: 12,
            ..PlayerAoIData::default()
        }
    }

    /// Decrypt a test-transport packet (all-zero session key) down to its
    /// message body: strip the flags byte and the 4-byte sequence footer.
    fn body_of(pkt: &[u8]) -> Vec<u8> {
        let pt = MercuryEncryption::from_session_key([0u8; 32])
            .decrypt(pkt)
            .unwrap();
        pt[1..pt.len() - 4].to_vec()
    }

    /// Fan-out byte guard for the bug this module fixes: when a player
    /// enters another player's AoI, the cascade packet that reaches the
    /// WITNESS carries the OBSERVEE session's identity (name, appearance,
    /// level, ...) joined with the cell's live state, not the bare cascade.
    /// Reverting the join in `aoi::entered_aoi` sends the bare body and
    /// trips the equality below.
    #[tokio::test]
    async fn player_entering_aoi_sends_the_witness_the_observee_identity() {
        let transport = Arc::new(TestTransport::new());
        let dyn_transport: Arc<dyn Transport> = transport.clone();
        let (connected, entity_to_addr) = two_sessions(Some(observee_state()));
        let live = live();

        aoi::entered_aoi(
            WITNESS,
            OBSERVEE,
            0x02,
            [1.0, 2.0, 3.0],
            [0.0; 3],
            1, // cell-side level is never set for players; the session value wins
            None,
            Some(live.clone()),
            &dyn_transport,
            &connected,
            &entity_to_addr,
        )
        .await;

        let sent = transport.drain();
        assert_eq!(sent.len(), 2, "create_base + cascade");
        assert!(
            sent.iter().all(|(addr, _)| *addr == witness_addr()),
            "both packets go to the witness, never to the observee"
        );

        let expected = compose_player_ghost_cascade_body(
            OBSERVEE,
            &PlayerGhostCascade {
                name: "Lomiada",
                level: 9,
                archetype: 4,
                alignment: 2,
                appearance_args: Some(&[0xAA, 0xBB]),
                tint_args: Some(&[0; 12]),
                live: &live,
            },
        );
        assert_eq!(body_of(&sent[1].1), expected);
        assert_ne!(
            expected,
            compose_create_entity_cascade_body(OBSERVEE, 0x02, 1, None),
            "test invariant: the ghost cascade differs from the bare one"
        );
    }

    /// The join happens at compose time, so a witness whose `EnteredAoI` sat
    /// in the deferred buffer gets the observee appearance as of the flush,
    /// not as of the cell event.
    #[test]
    fn cascade_body_reads_the_observee_appearance_at_compose_time() {
        let (connected, entity_to_addr) = two_sessions(Some(observee_state()));
        let live = live();
        let compose = || {
            compose_cascade_body(
                WITNESS,
                OBSERVEE,
                0x02,
                1,
                None,
                Some(&live),
                &connected,
                &entity_to_addr,
            )
        };

        let before = compose();
        connected
            .lock()
            .unwrap()
            .get_mut(&observee_addr())
            .unwrap()
            .cached_appearance_args = Some(vec![0xCC, 0xDD, 0xEE]);
        let after = compose();

        assert_ne!(before, after);
        assert!(after.windows(3).any(|w| w == [0xCC, 0xDD, 0xEE]));
    }

    /// NPCs never touch the session join: `player_data: None` composes the
    /// NPC cascade byte-for-byte as before, with no WARN.
    #[tokio::test]
    async fn npc_cascade_is_unchanged_and_silent() {
        let capture = LogCapture::install();
        let (connected, entity_to_addr) = two_sessions(None);
        let npc = NpcAoIData::default();

        let body = compose_cascade_body(
            WITNESS,
            300,
            0x04,
            5,
            Some(&npc),
            None,
            &connected,
            &entity_to_addr,
        );

        assert_eq!(
            body,
            compose_create_entity_cascade_body(300, 0x04, 5, Some(&npc))
        );
        assert!(capture
            .find_event(
                Level::WARN,
                "Player entered AoI",
                "observee_session_unresolved"
            )
            .is_none());
    }

    /// Negative-log seam: a player observee whose base session is gone
    /// degrades to the bare cascade and says so, naming both ends.
    #[tokio::test]
    async fn unresolved_observee_session_warns_and_falls_back_to_bare_cascade() {
        let capture = LogCapture::install();
        let (connected, entity_to_addr) = two_sessions(None);

        let body = compose_cascade_body(
            WITNESS,
            OBSERVEE,
            0x02,
            1,
            None,
            Some(&live()),
            &connected,
            &entity_to_addr,
        );

        assert_eq!(
            body,
            compose_create_entity_cascade_body(OBSERVEE, 0x02, 1, None)
        );
        let warn = capture
            .find_event(
                Level::WARN,
                "Player entered AoI",
                "observee_session_unresolved",
            )
            .expect("unresolved observee session must WARN");
        assert!(warn.has_field("entity_id", "200"), "{warn:#?}");
        assert!(warn.has_field("witness_id", "100"), "{warn:#?}");
        assert!(warn.has_field("addr_resolved", "false"), "{warn:#?}");
    }

    /// Negative-log seam: a resolved session with no cached appearance still
    /// introduces the player (name + stats beat nothing) but WARNs that the
    /// ghost has no body.
    #[tokio::test]
    async fn missing_cached_appearance_warns_but_still_sends_identity() {
        let capture = LogCapture::install();
        let mut state = observee_state();
        state.cached_appearance_args = None;
        let (connected, entity_to_addr) = two_sessions(Some(state));

        let identity = resolve_identity(&connected, &entity_to_addr, WITNESS, OBSERVEE)
            .expect("session resolves");

        assert_eq!(identity.name, "Lomiada");
        assert!(identity.appearance_args.is_none());
        let warn = capture
            .find_event(Level::WARN, "Player entered AoI", "no_cached_appearance")
            .expect("missing appearance must WARN");
        assert!(warn.has_field("entity_id", "200"), "{warn:#?}");
    }
}
