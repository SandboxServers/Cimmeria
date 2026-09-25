//! Whether an entity is in cover against an attacker when a hit resolves
//! (NA32).
//!
//! A hit gives cover its damage reduction only where the geometry says the
//! cover is between the two ([`crate::cell::combat::cover_reduction`], rated
//! by the node's quality and height). The test is the
//! one the NPC cover AI already uses to give a slot up: the entity stands at
//! a cover node, and the other side is not past the node's side-on line by
//! more than 20 degrees ([`crate::cell::cover::is_flanked`]). One rule for
//! both, so an NPC that the AI still considers covered is covered for the
//! hit too, and the flank that makes it leave its slot is the flank that
//! takes its bonus away.
//!
//! - **An NPC** is at cover when it holds a slot and stands within
//!   [`COVER_ARRIVE_RADIUS`] of it ([`SpaceManager::npc_cover_node`]). An NPC
//!   still walking to its slot is not in cover yet.
//! - **A player** holds no slot. It is at cover when a cover node of its own
//!   world is within [`COVER_ARRIVE_RADIUS`] horizontally and
//!   [`PLAYER_COVER_MAX_DY`] vertically, the nearest one counting. A player's
//!   `coverDefense` is zero unless a buff raised it, so for most players
//!   this changes nothing; it is what makes such a buff work.

use cimmeria_common::Vector3;

use super::SpaceManager;
use crate::cell::cover::{horizontal, is_flanked, CoverNode, COVER_ARRIVE_RADIUS};

/// Vertical band for the player test: the node is on the player's floor.
/// The same 2 u the player cover-set detection uses
/// (`cover::detection::sets_near`).
pub const PLAYER_COVER_MAX_DY: f32 = 2.0;

/// Where an entity stands relative to cover against one other entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoverStanding {
    /// Not at a cover node.
    Exposed,
    /// At a cover node that faces the other side.
    InCover,
    /// At a cover node, but the other side is past its side-on line.
    Flanked,
}

impl CoverStanding {
    /// Stable label for telemetry.
    pub fn label(self) -> &'static str {
        match self {
            Self::Exposed => "exposed",
            Self::InCover => "in_cover",
            Self::Flanked => "flanked",
        }
    }

    pub fn in_cover(self) -> bool {
        self == Self::InCover
    }
}

impl SpaceManager {
    /// The cover node `entity_id` stands at: its held slot for an NPC, the
    /// nearest node on its floor for a player. See the module docs.
    pub fn cover_node_at(&self, entity_id: u32) -> Option<CoverNode> {
        let entity = self.get_entity(entity_id)?;
        if !entity.is_player {
            return self.npc_cover_node(entity_id);
        }
        let world_id = self.get_entity_world_id(entity_id)?;
        let pos = entity.position;
        let idx = self
            .cover
            .index
            .nearby(
                world_id,
                &pos,
                COVER_ARRIVE_RADIUS,
                Some(PLAYER_COVER_MAX_DY),
            )
            .into_iter()
            .next()?;
        let node = self.cover.index.node(idx)?;
        (horizontal(&node.pos, &pos) <= COVER_ARRIVE_RADIUS).then(|| node.clone())
    }

    /// Where `entity_id` stands relative to cover against something at
    /// `other_pos`.
    pub fn cover_standing(&self, entity_id: u32, other_pos: Vector3) -> CoverStanding {
        self.cover_standing_node(entity_id, other_pos).0
    }

    /// [`Self::cover_standing`] and the node it was decided on, which carries
    /// the quality and height the damage reduction is rated from.
    pub fn cover_standing_node(
        &self,
        entity_id: u32,
        other_pos: Vector3,
    ) -> (CoverStanding, Option<CoverNode>) {
        match self.cover_node_at(entity_id) {
            None => (CoverStanding::Exposed, None),
            Some(node) if is_flanked(node.pos, node.orient, other_pos) => {
                (CoverStanding::Flanked, Some(node))
            }
            Some(node) => (CoverStanding::InCover, Some(node)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::cover::{Cover, CoverHeight, CoverQuality, CoverSlotKey, TEST_WORLD_ID};
    use cimmeria_common::EntityId;

    const NPC: u32 = 2;
    const PLAYER: u32 = 1;

    fn node(x: f32, z: f32, orient: f32) -> CoverNode {
        CoverNode {
            chunk_id: 50,
            node_id: 0,
            world_id: TEST_WORLD_ID,
            pos: Vector3::new(x, 0.0, z),
            orient,
            height: CoverHeight::Mid,
            quality: CoverQuality::Best,
            width: 1.0,
            tail: [0; 4],
        }
    }

    /// NPC at (0,0,0), player at `player`, one node at the NPC facing +X.
    fn mgr(player: [f32; 3]) -> SpaceManager {
        let mut mgr = SpaceManager::new(1);
        mgr.parse_spaces_xml(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#,
        )
        .unwrap();
        mgr.create_startup_spaces(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
        )
        .unwrap();
        mgr.stamp_world_rows(&std::collections::HashMap::from([(
            "Castle".to_string(),
            crate::cell::spawner::WorldRow::enforcing(TEST_WORLD_ID),
        )]));
        mgr.create_entity(PLAYER, "Castle", player, [0.0; 3])
            .unwrap();
        mgr.get_entity_mut(PLAYER).unwrap().is_player = true;
        mgr.create_entity(NPC, "Castle", [0.0; 3], [0.0; 3])
            .unwrap();
        mgr.cover = Cover::from_loaded(Vec::new(), vec![node(0.0, 0.0, 0.0)]);
        mgr
    }

    fn hold(mgr: &SpaceManager) {
        mgr.cover
            .reservations
            .lock()
            .unwrap()
            .reserve_for_entity(
                EntityId(NPC as i32),
                CoverSlotKey {
                    chunk_id: 50,
                    node_id: 0,
                },
            )
            .unwrap();
    }

    /// An NPC at its held slot is in cover against a threat in front and
    /// flanked against one behind; without the reservation it is exposed.
    #[test]
    fn npc_standing_follows_its_slot_and_the_flank_rule() {
        let mgr = mgr([20.0, 0.0, 0.0]);
        let front = Vector3::new(20.0, 0.0, 0.0);
        let behind = Vector3::new(-20.0, 0.0, 3.0);
        assert_eq!(
            mgr.cover_standing(NPC, front),
            CoverStanding::Exposed,
            "an NPC standing at a node it does not hold is not in cover"
        );
        hold(&mgr);
        assert_eq!(mgr.cover_standing(NPC, front), CoverStanding::InCover);
        assert_eq!(mgr.cover_standing(NPC, behind), CoverStanding::Flanked);
        // 10 degrees past side-on is still inside the release band.
        let (s, c) = (100f32.to_radians().sin(), 100f32.to_radians().cos());
        let near_side = Vector3::new(20.0 * c, 0.0, 20.0 * s);
        assert_eq!(mgr.cover_standing(NPC, near_side), CoverStanding::InCover);
    }

    /// An NPC walking to its slot (more than 1.5 u away) is not in cover yet.
    #[test]
    fn npc_short_of_its_slot_is_exposed() {
        let mut mgr = mgr([20.0, 0.0, 0.0]);
        hold(&mgr);
        mgr.get_entity_mut(NPC).unwrap().position = Vector3::new(-3.0, 0.0, 0.0);
        assert_eq!(
            mgr.cover_standing(NPC, Vector3::new(20.0, 0.0, 0.0)),
            CoverStanding::Exposed
        );
    }

    /// A player needs no reservation: the node under it counts, on its own
    /// floor only.
    #[test]
    fn player_standing_uses_the_nearest_node_on_its_floor() {
        let attacker = Vector3::new(20.0, 0.0, 0.0);
        let at_node = mgr([1.0, 0.0, 0.0]);
        assert_eq!(
            at_node.cover_standing(PLAYER, attacker),
            CoverStanding::InCover
        );
        assert_eq!(
            at_node.cover_standing(PLAYER, Vector3::new(-20.0, 0.0, 0.0)),
            CoverStanding::Flanked
        );
        let far = mgr([4.0, 0.0, 0.0]);
        assert_eq!(far.cover_standing(PLAYER, attacker), CoverStanding::Exposed);
        let upstairs = mgr([1.0, 5.0, 0.0]);
        assert_eq!(
            upstairs.cover_standing(PLAYER, attacker),
            CoverStanding::Exposed
        );
    }
}
