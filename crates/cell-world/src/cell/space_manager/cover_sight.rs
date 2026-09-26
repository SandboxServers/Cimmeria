//! An NPC's line of sight, seen from its cover peek point when it stands at
//! a cover slot (NA23, D-NA12).
//!
//! Every NPC-looks-at-someone check goes through
//! [`SpaceManager::npc_line_of_sight`]: the Idle aggro scan and the assist
//! check (`npc_ai::aggro_gates::same_room`), the fight tick's attack check
//! (`attack_line_of_sight`) and the `npc_ai.tick` row. For an NPC that holds
//! a cover slot and stands within [`COVER_ARRIVE_RADIUS`] of it, the answer
//! is [`crate::cell::cover::sight_from_slot`]: the ray from the slot's peek
//! point past the prop, or the NPC's own ray when that one is clear. The
//! prop in front of the NPC is a navmesh hole that would otherwise block
//! every ray, and every wall past the prop still does. A slot with no peek
//! point on the mesh sees only what the NPC's own ray sees. The slot is
//! reserved from spawn, so this covers an Idle guard authored in cover as
//! well as one that walked to its slot in a fight.

use cimmeria_common::{EntityId, Vector3};
use cimmeria_entity::navigation::{LineOfSight, NavMesh};

use super::SpaceManager;
use crate::cell::cover::{
    find_peek_point, horizontal, sight_from_slot, stand_behind, CoverNode, COVER_ARRIVE_RADIUS,
};

/// Where an NPC's line-of-sight ray started.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SightOrigin {
    /// The NPC's own position (not in cover, or no navmesh).
    Npc,
    /// The NPC stands at a cover slot whose peek point is here.
    CoverPeek(Vector3),
    /// The NPC stands at a cover slot that has no peek point on the mesh.
    CoverNoPeek,
}

impl SightOrigin {
    /// Stable label for the `origin` field of `npc_ai.los`.
    pub fn label(self) -> &'static str {
        match self {
            Self::Npc => "npc",
            Self::CoverPeek(_) => "cover_peek",
            Self::CoverNoPeek => "cover_no_peek",
        }
    }

    /// Whether the NPC stood at a cover slot.
    pub fn from_cover(self) -> bool {
        !matches!(self, Self::Npc)
    }
}

/// A navmesh line-of-sight verdict and where its ray started.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NpcSight {
    pub los: LineOfSight,
    pub origin: SightOrigin,
}

impl From<LineOfSight> for NpcSight {
    /// A verdict from the NPC's own position.
    fn from(los: LineOfSight) -> Self {
        Self {
            los,
            origin: SightOrigin::Npc,
        }
    }
}

impl SpaceManager {
    /// The cover node `npc_id` holds, when it stands within
    /// [`COVER_ARRIVE_RADIUS`] of it horizontally. A held slot the NPC is
    /// still walking to does not count: the NPC looks from where it is.
    pub fn npc_cover_node(&self, npc_id: u32) -> Option<CoverNode> {
        let npc = self.get_entity(npc_id)?;
        let slot = {
            let r = match self.cover.reservations.lock() {
                Ok(g) => g,
                Err(p) => p.into_inner(),
            };
            r.slot_for_entity(EntityId(npc_id as i32))?
        };
        let node = self.cover.index.node_by_key(slot)?;
        (horizontal(&node.pos, &npc.position) <= COVER_ARRIVE_RADIUS).then(|| node.clone())
    }

    /// Where `npc_id` looks from: see [`SightOrigin`]. Always its own eyes
    /// in a space with an occluder (NA27), which sees over the prop.
    pub fn npc_sight_origin(&self, npc_id: u32) -> SightOrigin {
        if self.space_has_occluder(npc_id) {
            return SightOrigin::Npc;
        }
        let (Some(navmesh), Some(node), Some(npc)) = (
            self.navmesh_of(npc_id),
            self.npc_cover_node(npc_id),
            self.get_entity(npc_id),
        ) else {
            return SightOrigin::Npc;
        };
        match find_peek_point(navmesh, &node, npc.position) {
            Some(p) => SightOrigin::CoverPeek(p),
            None => SightOrigin::CoverNoPeek,
        }
    }

    /// `npc_id`'s navmesh line of sight to `target_id`: from its cover slot
    /// ([`sight_from_slot`]) when it stands at one, otherwise from itself
    /// ([`Self::line_of_sight`]). A non-clear answer is reported through the
    /// sampled `npc_ai.los` row with the origin it used.
    ///
    /// In a space with an occluder (NA27) the NPC always looks from its own
    /// eyes: the collision geometry sees over the cover prop, which is the
    /// hole the peek point exists to step past.
    pub fn npc_line_of_sight(&self, npc_id: u32, target_id: u32) -> NpcSight {
        if self.space_has_occluder(npc_id) {
            return self.line_of_sight(npc_id, target_id).into();
        }
        self.npc_navmesh_sight(npc_id, target_id)
    }

    /// [`Self::npc_line_of_sight`] from the navmesh alone (the cover peek
    /// point, else the NPC's own ray), whether or not the space has an
    /// occluder. The attack check's fallback for an occluder `Unknown`.
    pub(crate) fn npc_navmesh_sight(&self, npc_id: u32, target_id: u32) -> NpcSight {
        let (Some(navmesh), Some(node), Some(npc), Some(target)) = (
            self.navmesh_of(npc_id),
            self.npc_cover_node(npc_id),
            self.get_entity(npc_id),
            self.get_entity(target_id),
        ) else {
            return self.navmesh_line_of_sight(npc_id, target_id).into();
        };
        let sight = sight_from_slot(navmesh, &node, npc.position, target.position);
        let origin = match sight.peek {
            Some(p) => SightOrigin::CoverPeek(p),
            None => SightOrigin::CoverNoPeek,
        };
        crate::cell::service::npc_ai::detectors::los::report(
            self,
            npc_id,
            target_id,
            sight.from,
            target.position,
            &sight.probe,
            crate::cell::service::npc_ai::detectors::los::LosSource::Navmesh(navmesh.short_hash()),
            origin.label(),
            std::time::Instant::now(),
        );
        NpcSight {
            los: sight.los,
            origin,
        }
    }

    /// Whether an NPC standing at `node` would have a line of sight to
    /// `target_pos` ([`sight_from_slot`] from a step behind the marker). The
    /// cover pick's shot check: a slot is a firing position (D-NA05), so one
    /// it cannot see its target from is not taken. `true` in a space with no
    /// navmesh, where there is nothing to check.
    ///
    /// With an occluder (NA27) the shot is the eye-to-eye segment from the
    /// stand point behind the marker.
    pub fn slot_has_shot(&self, npc_id: u32, node: &CoverNode, target_pos: Vector3) -> bool {
        if let Some(occ) = self.occluder_of(npc_id) {
            // The NPC's own eye (NA31); the far end is a bare position, so
            // it takes the default.
            let eye = self
                .get_entity(npc_id)
                .map_or(super::DEFAULT_EYE_HEIGHT, |e| self.eye_height_of(e));
            return super::occluder_probe(
                occ,
                stand_behind(node),
                eye,
                target_pos,
                super::DEFAULT_EYE_HEIGHT,
            )
            .result
            .is_clear_or_unknown();
        }
        let Some(navmesh) = self.navmesh_of(npc_id) else {
            return true;
        };
        sight_from_slot(navmesh, node, stand_behind(node), target_pos)
            .los
            .is_clear_or_unknown()
    }

    /// The navmesh of the space `entity_id` is in.
    fn navmesh_of(&self, entity_id: u32) -> Option<&NavMesh> {
        let space_id = self.entity_space.get(&entity_id)?;
        self.spaces.get(space_id)?.navmesh.as_ref()
    }
}
