//! The cover peek point: where an NPC holding a cover slot looks from
//! (NA23, decision D-NA12).
//!
//! A cover prop is a hole in the navmesh. An NPC standing behind one casts
//! its navmesh line-of-sight ray into the hole's edge within half a metre
//! and reads `Blocked` against every target in front of the cover (UAT-1:
//! `Hallway01_Guard`'s ray stopped 0.34-0.42 u from it, `Hallway02_Guard`'s
//! at 0.0 u). NA22 answered that by not checking the attack at all, which
//! let a guard shoot a player through two walls. The peek point replaces
//! both: the ray starts where the NPC's shot would clear the prop, so the
//! prop no longer blocks and every wall beyond it still does.
//!
//! Two kinds of peek are searched on the navmesh from the node, in order:
//!
//! 1. **Over the prop** ([`PeekKind::Over`]): along the node's facing
//!    (toward the defended side), from [`PEEK_FORWARD_MIN`] to
//!    [`PEEK_FORWARD_MAX`] in [`PEEK_FORWARD_STEP`] steps: the far side of
//!    the prop, which an NPC firing over waist- or chest-high cover shoots
//!    across. Every seeded height (`Low`, `Mid`, `High`) is fired over; a
//!    `Los` marker (2.52 m, none are seeded) only peeks round.
//! 2. **Round its side** ([`PeekKind::Around`]): [`PEEK_LATERAL_MARGIN`]
//!    past each end of the marker (`width / 2`), level with the node and
//!    then half a step forward.
//!
//! A candidate is accepted when:
//!
//! - it is on the mesh: a polygon within [`PEEK_SNAP_RADIUS`] horizontally
//!   and [`PEEK_SNAP_HALF_HEIGHT`] vertically (the snapped point is used);
//! - it is past the prop: a navmesh ray from it along the facing runs at
//!   least [`PEEK_MIN_CLEARANCE`] before it hits anything, so a point still
//!   pressed against the prop, or on the NPC's own side of a deep one, is
//!   not taken;
//! - for a side peek only, the NPC can step there: a full navmesh route from
//!   where it stands is at most [`PEEK_MAX_WALK`] long, so a side search
//!   that ended up in the next room is rejected. An over-the-prop peek is
//!   not walked to (`Hallway01_Guard`'s counter is 5 u wide and the walk
//!   round it is 9.4 u); its guard is the short forward reach instead.
//!
//! No candidate means no peek point: the slot sees only what the NPC's own
//! ray sees ([`sight_from_slot`]).
//!
//! The numbers are measured on `castle_cellblock.nav` against the world-12
//! cover seed (the NA23 geometry probe, recorded in
//! `docs/analysis/npc-ai-restoration/worknotes/uat-1.md`): the first point
//! past the prop is 1.08 u over `Hallway01_Guard`'s node 1200037/3 and
//! 1.30 u over `Hallway02_Guard`'s 1200034/0; the mess tables are deeper,
//! 3.36 u over `MessHall_Guard2`'s 1200053/0 and 3.46 u over
//! `MessHall_Guard1`'s 1200046/0. Of the 236 world-12 markers, 150 find an
//! over-the-prop peek (0.5-3.6 u out, median 1.6 u), 17 a side peek and 69
//! none. Every over-the-prop peek has a full navmesh route from behind its
//! marker (none is on another mesh island), the longest 22 u round a long
//! counter.

use cimmeria_common::Vector3;
use cimmeria_entity::navigation::{LineOfSight, LosProbe, NavMesh, PathStatus};

use super::types::{CoverHeight, CoverNode};

/// The first forward sample. Inside it every Cellblock marker is still in
/// its own prop.
pub const PEEK_FORWARD_MIN: f32 = 0.5;
/// The last forward sample. The deepest Cellblock prop, a mess table
/// (1200046/0), clears 3.25 u out along the facing.
pub const PEEK_FORWARD_MAX: f32 = 3.5;
pub const PEEK_FORWARD_STEP: f32 = 0.25;
/// How far a sample may be from the mesh horizontally and still count as on
/// it. Smaller than a step, so a sample in the middle of a prop does not snap
/// to the prop's near edge.
pub const PEEK_SNAP_RADIUS: f32 = 0.3;
pub const PEEK_SNAP_HALF_HEIGHT: f32 = 1.0;
/// The clear run along the facing that makes a candidate "past the prop".
pub const PEEK_MIN_CLEARANCE: f32 = 1.0;
/// The ray cast for the clearance test.
const PEEK_CLEARANCE_RAY: f32 = 3.0;
/// How far past the end of the marker a side peek stands.
pub const PEEK_LATERAL_MARGIN: f32 = 0.6;
/// The longest navmesh walk from the NPC to a side peek. Stepping round the
/// end of a prop is a few metres; a route through a door into the next room
/// is much longer.
pub const PEEK_MAX_WALK: f32 = 6.0;
/// How far the NPC's standing point may be snapped to find the route's
/// start: a guard authored in a prop's footprint stands up to 0.6 u off the
/// mesh (`Hallway02_Guard`, 0.59 u).
const STAND_SNAP_RADIUS: f32 = 1.5;
const STAND_SNAP_HALF_HEIGHT: f32 = 2.0;

/// Which way a peek clears the prop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeekKind {
    /// Across the prop, along the node's facing.
    Over,
    /// Round one end of the prop.
    Around,
}

/// A peek point and how it was found.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Peek {
    pub point: Vector3,
    pub kind: PeekKind,
}

/// The node's facing, `(cos, sin)` in BW `(x, z)`.
fn facing(node: &CoverNode) -> (f32, f32) {
    (node.orient.cos(), node.orient.sin())
}

/// The peek point for `node`, for an NPC standing at `stand`, or `None`.
pub fn find_peek_point(navmesh: &NavMesh, node: &CoverNode, stand: Vector3) -> Option<Vector3> {
    find_peek(navmesh, node, stand).map(|p| p.point)
}

/// [`find_peek_point`] with the kind of peek it found.
pub fn find_peek(navmesh: &NavMesh, node: &CoverNode, stand: Vector3) -> Option<Peek> {
    let (fx, fz) = facing(node);
    let at = |along: f32, side: f32| {
        Vector3::new(
            node.pos.x + along * fx - side * fz,
            node.pos.y,
            node.pos.z + along * fz + side * fx,
        )
    };
    let on_mesh =
        |c: Vector3| navmesh.nearest_point_within(&c, PEEK_SNAP_RADIUS, PEEK_SNAP_HALF_HEIGHT);
    if node.height != CoverHeight::Los {
        let mut d = PEEK_FORWARD_MIN;
        while d <= PEEK_FORWARD_MAX + 1e-3 {
            if let Some(p) = on_mesh(at(d, 0.0)) {
                if past_the_prop(navmesh, p, fx, fz) {
                    return Some(Peek {
                        point: p,
                        kind: PeekKind::Over,
                    });
                }
            }
            d += PEEK_FORWARD_STEP;
        }
    }
    let start = navmesh.nearest_point_within(&stand, STAND_SNAP_RADIUS, STAND_SNAP_HALF_HEIGHT);
    let side = node.width.max(0.0) / 2.0 + PEEK_LATERAL_MARGIN;
    [0.0, PEEK_FORWARD_MIN]
        .into_iter()
        .flat_map(|along| [at(along, side), at(along, -side)])
        .find_map(|c| {
            let p = on_mesh(c)?;
            (past_the_prop(navmesh, p, fx, fz) && reachable(navmesh, start, p)).then_some(Peek {
                point: p,
                kind: PeekKind::Around,
            })
        })
}

/// A ray from `p` along the facing runs [`PEEK_MIN_CLEARANCE`] before
/// hitting anything.
fn past_the_prop(navmesh: &NavMesh, p: Vector3, fx: f32, fz: f32) -> bool {
    let end = Vector3::new(
        p.x + PEEK_CLEARANCE_RAY * fx,
        p.y,
        p.z + PEEK_CLEARANCE_RAY * fz,
    );
    let probe = navmesh.line_of_sight_probe(&p, &end);
    match (probe.result, probe.hit, probe.from) {
        (LineOfSight::Clear, _, _) => true,
        (LineOfSight::Blocked, Some(hit), Some(from)) => {
            let (dx, dz) = (hit[0] - from[0], hit[2] - from[2]);
            (dx * dx + dz * dz).sqrt() >= PEEK_MIN_CLEARANCE
        }
        _ => false,
    }
}

/// A full navmesh route from `start` to `p` no longer than
/// [`PEEK_MAX_WALK`]. A standing point with no polygon near it cannot be
/// checked and passes: the peek candidate is already on the mesh next to the
/// node.
fn reachable(navmesh: &NavMesh, start: Option<Vector3>, p: Vector3) -> bool {
    let Some(start) = start else {
        return true;
    };
    let outcome = navmesh.find_path(&start, &p);
    if outcome.status != PathStatus::Ok {
        return false;
    }
    let len: f32 = outcome
        .waypoints
        .windows(2)
        .map(|w| w[0].distance_to(&w[1]))
        .sum();
    len <= PEEK_MAX_WALK
}

/// Where an NPC stands to use `node` when it is not there yet: a step
/// behind the marker, on the side the prop shields. Used to judge a slot at
/// pick time.
pub fn stand_behind(node: &CoverNode) -> Vector3 {
    let (fx, fz) = facing(node);
    Vector3::new(
        node.pos.x - STAND_BEHIND * fx,
        node.pos.y,
        node.pos.z - STAND_BEHIND * fz,
    )
}

/// How far behind its marker an NPC stands. `Hallway01_Guard` is authored
/// 1.21 u behind node 1200037/3, `MessHall_Guard2` 0.96 u behind 1200053/0.
const STAND_BEHIND: f32 = 1.0;

/// Line of sight from a cover slot to `target`, for an NPC standing at
/// `stand` behind `node`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SlotSight {
    /// `Clear` when the ray from the peek point or the ray from `stand` is
    /// clear; `Blocked` when neither is, and always when there is no peek
    /// point and the ray from `stand` is not clear.
    pub los: LineOfSight,
    /// The peek point, when the slot has one.
    pub peek: Option<Vector3>,
    /// The ray that decided a non-clear answer, and where it started (for
    /// the `npc_ai.los` row).
    pub probe: LosProbe,
    pub from: Vector3,
}

/// See [`SlotSight`]. A target beside or behind the prop is seen past it,
/// and a clear ray from where the NPC stands crosses nothing, so taking the
/// better of the two rays never adds a shot through a wall.
pub fn sight_from_slot(
    navmesh: &NavMesh,
    node: &CoverNode,
    stand: Vector3,
    target: Vector3,
) -> SlotSight {
    let peek = find_peek_point(navmesh, node, stand);
    let own = navmesh.line_of_sight_probe(&stand, &target);
    if own.result == LineOfSight::Clear {
        return SlotSight {
            los: LineOfSight::Clear,
            peek,
            probe: own,
            from: stand,
        };
    }
    let Some(p) = peek else {
        return SlotSight {
            los: LineOfSight::Blocked,
            peek,
            probe: own,
            from: stand,
        };
    };
    let probe = navmesh.line_of_sight_probe(&p, &target);
    SlotSight {
        los: probe.result,
        peek,
        probe,
        from: p,
    }
}
