//! Black Market sessions: which auctioneer opened the window for each
//! player, and whether the player's client ever answered (BM-02).
//!
//! # Authority (CWE-862)
//!
//! Cell methods 62-64 (`BMCreateAuction`, `BMPlaceBid`, `BMCancelAuction`)
//! move items and cash, so the cell forwards them only for a player the
//! server itself sent to an auctioneer. The `open_black_market` content
//! action records the auctioneer here ([`BlackMarketSessions::open`]);
//! [`black_market_access`] then requires, on every call:
//!
//! - an open session, and the player's current interaction target
//!   (`CellEntity::last_interaction_target`) still being that auctioneer, so
//!   talking to another NPC ends the right to trade;
//! - the auctioneer to exist, share the player's space and be within
//!   `MAX_INTERACT_DISTANCE`: the same [`interact_range`] rule the `interact`
//!   that opened the window passed;
//! - the NPC to be an auctioneer: `NpcInteractionType::Auctioneer`, which
//!   only a template's seeded `INT_AUCTION` bit gives, at spawn (BM-07).
//!
//! The pin is server-set only: the chain that runs `open_black_market` is
//! bound to the auctioneer's interact tag, so nothing the client sends can
//! name an auctioneer. The open itself runs [`auctioneer_check`] first, so a
//! chain bound to any other NPC opens nothing: authoring cannot turn a
//! quest giver into a Black Market terminal.
//!
//! # Lifetime
//!
//! Keyed by `player_id`, so the entry outlives gate travel (a space change
//! re-creates the `CellEntity`, whose fresh `last_interaction_target` is
//! `None`, so the old pin no longer passes). It ends on `DisconnectEntity`
//! ([`BlackMarketSessions::end`]), where the counts feed the
//! `bm.open_without_client_call` signal: a player who was sent `onBMOpen`
//! but whose client never called 61-66 is running without the client patch.

use std::collections::HashMap;

use cimmeria_entity::cell_entity::NpcInteractionType;

use super::space_manager::{interact_range, InteractRangeFail, SpaceManager};

/// Count one Black Market request on `bm_outcome_total{op, outcome}`. `op`
/// is the method (`search`, `create`, `bid`, `cancel`, `watch`, `settle`),
/// `outcome` is `ok` or a closed refusal label (an `onBMError` reason, or
/// `decode_failed`); never an id. The base counts on the same series.
pub fn count_bm_outcome(op: &'static str, outcome: &'static str) {
    cimmeria_observability::counter!(
        "bm_outcome_total",
        "op" => op,
        "outcome" => outcome,
    );
}

/// One player's Black Market state for this login.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BlackMarketSession {
    /// The auctioneer the last `onBMOpen` named.
    pub auctioneer_id: Option<u32>,
    /// How many times the window was opened this login.
    pub opens: u32,
    /// How many cell methods 61-66 arrived from the client this login.
    pub client_calls: u32,
}

impl BlackMarketSession {
    /// The server opened the window but the client never answered: the
    /// sign that the client patch is missing.
    pub fn open_without_client_call(&self) -> bool {
        self.opens > 0 && self.client_calls == 0
    }
}

/// Every player's session on this cell, keyed by `player_id`.
#[derive(Debug, Default)]
pub struct BlackMarketSessions {
    by_player: HashMap<i32, BlackMarketSession>,
}

impl BlackMarketSessions {
    pub fn new() -> Self {
        Self::default()
    }

    /// The player's session, if any.
    pub fn get(&self, player_id: i32) -> Option<&BlackMarketSession> {
        self.by_player.get(&player_id)
    }

    /// `onBMOpen` was sent naming `auctioneer_id`.
    pub fn open(&mut self, player_id: i32, auctioneer_id: u32) {
        let s = self.by_player.entry(player_id).or_default();
        s.auctioneer_id = Some(auctioneer_id);
        s.opens = s.opens.saturating_add(1);
    }

    /// A cell method 61-66 arrived from the player's client.
    pub fn note_client_call(&mut self, player_id: i32) {
        let s = self.by_player.entry(player_id).or_default();
        s.client_calls = s.client_calls.saturating_add(1);
    }

    /// The login ended: remove and return the session.
    pub fn end(&mut self, player_id: i32) -> Option<BlackMarketSession> {
        self.by_player.remove(&player_id)
    }
}

/// Why [`black_market_access`] refused a call.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BlackMarketReject {
    /// The player entity is not in any space, or has no `player_id`.
    PlayerMissing,
    /// No `onBMOpen` was ever sent to this player this login.
    NoSession,
    /// The player's interaction target is no longer the auctioneer (another
    /// NPC was pinned since, or the space changed).
    NotInteracting,
    /// The auctioneer no longer exists.
    AuctioneerGone,
    /// The auctioneer is in another space.
    AuctioneerOtherSpace,
    /// The player walked out of interact distance.
    OutOfRange {
        /// The distance, in world units.
        dist: f32,
    },
    /// The NPC is not an auctioneer: its template carries no `INT_AUCTION`
    /// bit (BM-07).
    NotAnAuctioneer,
}

impl BlackMarketReject {
    /// Stable label, logged as `access` beside `reason = not_at_auctioneer`.
    pub fn label(self) -> &'static str {
        match self {
            Self::PlayerMissing => "player_missing",
            Self::NoSession => "no_bm_session",
            Self::NotInteracting => "not_interacting",
            Self::AuctioneerGone => "auctioneer_gone",
            Self::AuctioneerOtherSpace => "auctioneer_other_space",
            Self::OutOfRange { .. } => "auctioneer_out_of_range",
            Self::NotAnAuctioneer => "not_an_auctioneer",
        }
    }
}

/// May `entity_id` create, bid or cancel right now? Returns the auctioneer
/// on success. Pure: no logging, no sends.
pub fn black_market_access(
    entity_id: u32,
    space_mgr: &SpaceManager,
) -> Result<u32, BlackMarketReject> {
    let player = space_mgr
        .get_entity(entity_id)
        .ok_or(BlackMarketReject::PlayerMissing)?;
    let player_id = player.player_id.ok_or(BlackMarketReject::PlayerMissing)?;
    let auctioneer = space_mgr
        .black_market
        .get(player_id)
        .and_then(|s| s.auctioneer_id)
        .ok_or(BlackMarketReject::NoSession)?;
    if player.last_interaction_target != Some(auctioneer) {
        return Err(BlackMarketReject::NotInteracting);
    }
    auctioneer_check(entity_id, auctioneer, space_mgr)?;
    Ok(auctioneer)
}

/// Is `auctioneer_id` an auctioneer `entity_id` may trade at right now: it
/// exists, shares the player's space, is within interact distance, and is
/// `NpcInteractionType::Auctioneer`. The `open_black_market` action runs
/// this before it sends `onBMOpen`; [`black_market_access`] runs it again
/// on every create, bid and cancel. Pure: no logging, no sends.
pub fn auctioneer_check(
    entity_id: u32,
    auctioneer_id: u32,
    space_mgr: &SpaceManager,
) -> Result<(), BlackMarketReject> {
    interact_range(entity_id, auctioneer_id, space_mgr).map_err(|fail| match fail {
        InteractRangeFail::PlayerMissing => BlackMarketReject::PlayerMissing,
        InteractRangeFail::TargetMissing => BlackMarketReject::AuctioneerGone,
        InteractRangeFail::OtherSpace => BlackMarketReject::AuctioneerOtherSpace,
        InteractRangeFail::TooFar { dist } => BlackMarketReject::OutOfRange { dist },
    })?;
    let is_auctioneer = space_mgr
        .get_entity(auctioneer_id)
        .is_some_and(|npc| npc.interaction_type == Some(NpcInteractionType::Auctioneer));
    if !is_auctioneer {
        return Err(BlackMarketReject::NotAnAuctioneer);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLAYER: u32 = 1;
    const PLAYER_ID: i32 = 12;

    /// Two worlds, the player in Agnos at the origin, and an auctioneer at
    /// `pos` in `world`.
    fn setup(world: &str, pos: [f32; 3]) -> (SpaceManager, u32) {
        let mut mgr = SpaceManager::new(1);
        mgr.parse_spaces_xml(
            r#"<?xml version="1.0"?><Spaces>
<Space WorldName="Agnos" Instanced="false" MinX="-500" MaxX="500" MinY="-500" MaxY="500" />
<Space WorldName="Harset" Instanced="false" MinX="-500" MaxX="500" MinY="-500" MaxY="500" />
</Spaces>"#,
        )
        .unwrap();
        mgr.create_startup_spaces(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /><Space WorldName="Harset" /></Spaces>"#,
        )
        .unwrap();
        mgr.create_entity(PLAYER, "Agnos", [0.0; 3], [0.0; 3])
            .unwrap();
        mgr.get_entity_mut(PLAYER).unwrap().player_id = Some(PLAYER_ID);
        let npc = mgr.allocate_npc_id();
        mgr.spawn_npc(npc, world, pos, [0.0; 3]).unwrap();
        mgr.get_entity_mut(npc).unwrap().interaction_type = Some(NpcInteractionType::Auctioneer);
        (mgr, npc)
    }

    fn open_at(mgr: &mut SpaceManager, npc: u32) {
        mgr.get_entity_mut(PLAYER).unwrap().last_interaction_target = Some(npc);
        mgr.black_market.open(PLAYER_ID, npc);
    }

    #[test]
    fn open_session_at_an_auctioneer_in_range_is_allowed() {
        let (mut mgr, npc) = setup("Agnos", [3.0, 0.0, 0.0]);
        open_at(&mut mgr, npc);
        assert_eq!(black_market_access(PLAYER, &mgr), Ok(npc));
    }

    /// The load-bearing refusal: a client that was never sent to an
    /// auctioneer cannot create, bid or cancel.
    #[test]
    fn no_session_is_refused() {
        let (mut mgr, npc) = setup("Agnos", [3.0, 0.0, 0.0]);
        // Standing next to the auctioneer and even pinned to it is not
        // enough: only the server's open records a session.
        mgr.get_entity_mut(PLAYER).unwrap().last_interaction_target = Some(npc);
        assert_eq!(
            black_market_access(PLAYER, &mgr),
            Err(BlackMarketReject::NoSession)
        );
    }

    #[test]
    fn a_new_interaction_target_ends_the_right_to_trade() {
        let (mut mgr, npc) = setup("Agnos", [3.0, 0.0, 0.0]);
        open_at(&mut mgr, npc);
        mgr.get_entity_mut(PLAYER).unwrap().last_interaction_target = Some(npc + 1);
        assert_eq!(
            black_market_access(PLAYER, &mgr),
            Err(BlackMarketReject::NotInteracting)
        );
    }

    #[test]
    fn out_of_range_other_space_and_gone_are_refused() {
        let (mut mgr, npc) = setup("Agnos", [30.0, 0.0, 0.0]);
        open_at(&mut mgr, npc);
        assert!(matches!(
            black_market_access(PLAYER, &mgr),
            Err(BlackMarketReject::OutOfRange { dist }) if (dist - 30.0).abs() < 0.01
        ));

        let (mut mgr, npc) = setup("Harset", [1.0, 0.0, 0.0]);
        open_at(&mut mgr, npc);
        assert_eq!(
            black_market_access(PLAYER, &mgr),
            Err(BlackMarketReject::AuctioneerOtherSpace)
        );

        let (mut mgr, npc) = setup("Agnos", [1.0, 0.0, 0.0]);
        open_at(&mut mgr, npc);
        mgr.destroy_entity(npc);
        assert_eq!(
            black_market_access(PLAYER, &mgr),
            Err(BlackMarketReject::AuctioneerGone)
        );
    }

    /// BM-07: an open session at an NPC that is not an auctioneer grants
    /// nothing. Fails if `auctioneer_check` drops the interaction-type test.
    #[test]
    fn an_npc_that_is_not_an_auctioneer_is_refused() {
        for other in [
            None,
            Some(NpcInteractionType::Vendor),
            Some(NpcInteractionType::Dialog { dialog_id: 1 }),
        ] {
            let (mut mgr, npc) = setup("Agnos", [3.0, 0.0, 0.0]);
            mgr.get_entity_mut(npc).unwrap().interaction_type = other.clone();
            open_at(&mut mgr, npc);
            assert_eq!(
                black_market_access(PLAYER, &mgr),
                Err(BlackMarketReject::NotAnAuctioneer),
                "{other:?}"
            );
            assert_eq!(
                auctioneer_check(PLAYER, npc, &mgr),
                Err(BlackMarketReject::NotAnAuctioneer)
            );
        }
    }

    #[test]
    fn session_counts_and_end() {
        let mut s = BlackMarketSessions::new();
        s.open(7, 100);
        assert!(s.get(7).unwrap().open_without_client_call());
        s.note_client_call(7);
        let ended = s.end(7).unwrap();
        assert_eq!((ended.opens, ended.client_calls), (1, 1));
        assert!(!ended.open_without_client_call());
        assert!(s.get(7).is_none());
    }
}
