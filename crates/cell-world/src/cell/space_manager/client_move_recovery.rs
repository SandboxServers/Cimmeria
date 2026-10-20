//! Off-mesh recovery target for the client-authoritative position path.
//!
//! Split out of `client_move.rs` to keep that file under the line cap. This
//! is the terminal-fallback resolver: it picks the point a rejected entity is
//! relocated to, and every candidate it returns passes the same layers the
//! validator applies to a client position.

use cimmeria_common::Vector3;
use cimmeria_entity::movement_validation::{position_within_bounds, SpaceBounds};

use super::SpaceManager;

impl SpaceManager {
    /// Resolve a position an entity stuck at `from` can be safely placed
    /// at. `None` means there is nothing better than where it already is.
    ///
    /// Ordered by how little it disturbs the player:
    ///
    /// 1. The nearest walkable navmesh point. This is the Z-clamp answer —
    ///    a player who ended up a metre inside the floor comes back out on
    ///    the surface they were standing on, not across the map.
    /// 2. The world's nearest authored respawn point. Already the
    ///    server's answer to "where is it safe to put this player", so
    ///    reusing it needs no new content.
    /// 3. The space AABB, clamped — the last resort when neither of the
    ///    above answers.
    ///
    /// **Every candidate is tested against the same layers that reject a
    /// client position before it is returned.** That is what makes the
    /// recovery terminal: the outcome is written through and clears the
    /// correction budget, so handing back a point the validator would itself
    /// reject just restarts the rubber-band loop one position over, with
    /// nothing left to spend. `None` — and the resulting
    /// [`ClientMoveOutcome::CorrectionSuppressed`] — is the correct answer
    /// when nothing passes.
    pub(super) fn resolve_recovery_position(
        &self,
        space_id: u32,
        from: Vector3,
        bounds: &SpaceBounds,
    ) -> Option<[f32; 3]> {
        let space = self.spaces.get(&space_id)?;
        // The mesh in its *containment* role — `None` for a meshless space and
        // for an advisory world alike. Bound once and used by all three
        // candidate branches below so they cannot disagree about the mode.
        let containment_nav = self.containment_navmesh(space_id);

        // Reprojection onto the mesh. Deliberately gated on the mode rather
        // than on `space.navmesh`: `get_nearest_point` is only as good as the
        // mesh's coverage, and in an advisory world the coverage is exactly
        // what is not trusted. On `harset.nav` the nearest polygon to a point
        // in the Command-Center corridor is on a different floor entirely, so
        // "recovering" a player there would teleport them through the map to
        // fix a position that was never a problem. An advisory world falls
        // through to the world's authored respawner instead — a coordinate a
        // human chose.
        if let Some(nav) = containment_nav {
            let projected = nav.get_nearest_point(&from);
            if nav.is_point_valid(&projected) && position_within_bounds(projected, bounds) {
                return Some([projected.x, projected.y, projected.z]);
            }
        }

        let world = space.world_name.as_str();
        // `load_respawners` copies the DB columns in with no validation, so an
        // authored-bad respawn point is exactly as unusable a snap target as
        // the position being recovered *from* — and relocating there would
        // still `note_authorized_teleport`, clearing the correction budget and
        // leaving the next reject with nothing left to spend. The shared
        // helper applies the same soundness test the navmesh branch above
        // does, so the fallback can only ever be a step towards a position the
        // validator accepts, plus the all-zero placeholder guard this path
        // used to be missing (PR #662 review, finding 8).
        let nearest_respawner = crate::cell::respawner_fallback::nearest_valid_respawner(
            &self.respawners,
            world,
            from,
            // Containment flavour again: in an advisory world an authored
            // respawner that happens to sit in a hole in the mesh is still a
            // place a human said a player may stand, and discarding it here
            // would leave the AABB clamp as the only candidate.
            containment_nav,
            bounds,
        );
        if let Some(p) = nearest_respawner {
            return Some([p.x, p.y, p.z]);
        }

        let clamped = [
            from.x.clamp(bounds.min[0], bounds.max[0]),
            from.y.clamp(bounds.min[1], bounds.max[1]),
            from.z.clamp(bounds.min[2], bounds.max[2]),
        ];
        if clamped == [from.x, from.y, from.z] {
            return None;
        }
        // A clamp answers the bounds layer by construction and says nothing
        // about walkability. On a navmesh-less space that is the whole test;
        // on a navmesh-backed one it is reachable whenever the reprojection
        // above failed (a point too far outside the mesh for Detour's
        // nearest-poly search), and pulling the entity to the AABB face lands
        // it inside whatever geometry happens to be there.
        //
        // Advisory worlds take the navmesh-less branch, which is the whole
        // point of the mode and matters most here: this is the last
        // candidate, and answering `None` returns `CorrectionSuppressed` — a
        // player left exactly where the validator refuses to move them from.
        // Holding an out-of-AABB player hostage to a mesh the server has
        // already declared untrustworthy is the worst of both policies.
        let clamped_pos = Vector3::new(clamped[0], clamped[1], clamped[2]);
        containment_nav
            .is_none_or(|nav| nav.is_point_valid(&clamped_pos))
            .then_some(clamped)
    }
}
