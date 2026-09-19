//! Floor probe — "is there walkable geometry under this world point?"
//!
//! The extractor emits triangles in raw UE3 centimetres. The server
//! (and every coordinate in `db/resources/`) speaks BigWorld units with
//! **Y up**. Before a `.nav` built from these triangles can be trusted,
//! two things have to be true:
//!
//! 1. The UE3 → BigWorld axis mapping is right.
//! 2. There is actually a **floor** under the places players demonstrably
//!    stood — not just the walls around them.
//!
//! This module answers both at once by running the same probe under a
//! family of candidate [`AxisMapping`]s and reporting which one puts a
//! floor under the known-walkable points. The mapping that wins is the
//! mapping; if none wins but walls show up nearby under one of them,
//! the transform is right and the *floors* are missing (BSP/Terrain).
//! Distinguishing those two outcomes is the point of the exercise.
//!
//! # Prior evidence
//!
//! `docs/analysis/castle-rebuild/worknotes/ca05.md` §"Axis/scale
//! calibration" pins the mapping from two ground-truth actors whose
//! world positions are independently known from seed rows:
//!
//! ```text
//! server.x = raw.Y / 100     server.y = raw.Z / 100     server.z = raw.X / 100
//! ```
//!
//! which is [`AxisMapping::CA05`]. That is the **hypothesis**, not the
//! conclusion — [`AxisMapping::all`] enumerates all 48 permutation/sign
//! combinations so the probe confirms it against geometry rather than
//! assuming it.
//!
//! Note this is NOT what NavBuilder's `loadOBJ` currently does to a raw
//! UE3-cm OBJ ([`AxisMapping::NAVBUILDER_ON_RAW_UE3`], `x=obj.z, y=obj.y,
//! z=obj.x`) — under that one BigWorld's up axis is fed from UE3's
//! horizontal Y. Whether the fix belongs in the OBJ writer or in
//! NavBuilder is the `nav-axis` worker's call; this module only measures.

pub mod report;

use crate::geometry::TriangleSoup;

/// Centimetres per BigWorld unit — the `/ 100` in NavBuilder's `loadOBJ`.
pub const CM_PER_BW_UNIT: f32 = 100.0;

/// Which UE3 source axis (with sign) feeds one BigWorld axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AxisSource {
    /// 0 = UE3 X, 1 = UE3 Y, 2 = UE3 Z.
    pub ue3_axis: u8,
    pub negate: bool,
}

impl AxisSource {
    pub const fn new(ue3_axis: u8, negate: bool) -> Self {
        Self { ue3_axis, negate }
    }
}

/// A full UE3-cm → BigWorld-unit transform: an axis permutation, per-axis
/// signs, and the fixed cm→unit divide.
///
/// `bw[0]` sources BigWorld **x**, `bw[1]` BigWorld **y (up)**, `bw[2]`
/// BigWorld **z**.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AxisMapping {
    pub bw: [AxisSource; 3],
}

impl AxisMapping {
    /// The CA05 worknote's calibrated mapping: `bw = (ue.Y, ue.Z, ue.X) / 100`.
    pub const CA05: AxisMapping = AxisMapping {
        bw: [
            AxisSource::new(1, false),
            AxisSource::new(2, false),
            AxisSource::new(0, false),
        ],
    };

    /// What NavBuilder's `loadOBJ` swizzle produces when handed an OBJ
    /// written in raw UE3 cm: `bw = (ue.Z, ue.Y, ue.X) / 100`. BigWorld's
    /// up axis ends up sourced from UE3's horizontal Y.
    pub const NAVBUILDER_ON_RAW_UE3: AxisMapping = AxisMapping {
        bw: [
            AxisSource::new(2, false),
            AxisSource::new(1, false),
            AxisSource::new(0, false),
        ],
    };

    /// Map one UE3-cm point into BigWorld units.
    pub fn apply(&self, ue3_cm: [f32; 3]) -> [f32; 3] {
        let mut out = [0.0f32; 3];
        for (i, src) in self.bw.iter().enumerate() {
            let v = ue3_cm[src.ue3_axis as usize] / CM_PER_BW_UNIT;
            out[i] = if src.negate { -v } else { v };
        }
        out
    }

    /// Compact label, e.g. `+Y+Z+X` for [`AxisMapping::CA05`] — read as
    /// "BigWorld x from +UE3 Y, BigWorld y from +UE3 Z, BigWorld z from
    /// +UE3 X".
    pub fn label(&self) -> String {
        const AXIS: [char; 3] = ['X', 'Y', 'Z'];
        let mut s = String::with_capacity(6);
        for src in &self.bw {
            s.push(if src.negate { '-' } else { '+' });
            s.push(AXIS[src.ue3_axis as usize]);
        }
        s
    }

    /// Parse a label produced by [`AxisMapping::label`]. Returns `None`
    /// for a malformed string or a non-permutation (repeated axis).
    pub fn from_label(label: &str) -> Option<Self> {
        let chars: Vec<char> = label.trim().chars().collect();
        if chars.len() != 6 {
            return None;
        }
        let mut bw = [AxisSource::new(0, false); 3];
        let mut seen = [false; 3];
        for i in 0..3 {
            let negate = match chars[i * 2] {
                '+' => false,
                '-' => true,
                _ => return None,
            };
            let axis = match chars[i * 2 + 1].to_ascii_uppercase() {
                'X' => 0u8,
                'Y' => 1,
                'Z' => 2,
                _ => return None,
            };
            if seen[axis as usize] {
                return None;
            }
            seen[axis as usize] = true;
            bw[i] = AxisSource::new(axis, negate);
        }
        Some(Self { bw })
    }

    /// All 48 candidates: 6 axis permutations × 8 sign combinations.
    pub fn all() -> Vec<AxisMapping> {
        const PERMS: [[u8; 3]; 6] = [
            [0, 1, 2],
            [0, 2, 1],
            [1, 0, 2],
            [1, 2, 0],
            [2, 0, 1],
            [2, 1, 0],
        ];
        let mut out = Vec::with_capacity(48);
        for perm in PERMS {
            for signs in 0u8..8 {
                out.push(AxisMapping {
                    bw: [
                        AxisSource::new(perm[0], signs & 1 != 0),
                        AxisSource::new(perm[1], signs & 2 != 0),
                        AxisSource::new(perm[2], signs & 4 != 0),
                    ],
                });
            }
        }
        out
    }
}

/// How much we trust a probe point's world coordinate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Confidence {
    /// Live telemetry: a player's avatar was observed standing here.
    High,
    /// Reconstructed from seed rows / map landmarks — the room is right,
    /// the exact spot inside it is an estimate.
    Medium,
}

impl Confidence {
    pub fn as_str(self) -> &'static str {
        match self {
            Confidence::High => "HIGH",
            Confidence::Medium => "MEDIUM",
        }
    }
}

/// One known-walkable world point, in BigWorld units (y up).
#[derive(Debug, Clone)]
pub struct ProbePoint {
    pub label: String,
    pub confidence: Confidence,
    pub bw: [f32; 3],
    /// Where the coordinate came from — printed in the report so a
    /// reader can re-check it without grepping.
    pub source: String,
}

impl ProbePoint {
    pub fn new(label: &str, confidence: Confidence, bw: [f32; 3], source: &str) -> Self {
        Self {
            label: label.to_string(),
            confidence,
            bw,
            source: source.to_string(),
        }
    }
}

/// Tolerances for "is this triangle the floor under that point?".
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProbeConfig {
    /// How far below the point a triangle may sit and still count as its
    /// floor, in BigWorld units.
    pub below: f32,
    /// How far *above* the point a triangle may sit and still count —
    /// absorbs the difference between an avatar's origin and its feet,
    /// and rounding in reconstructed seed coordinates.
    pub above: f32,
    /// Minimum `normal.y` (on a unit normal) for a triangle to count as
    /// walkable. 0.7071 == a 45° slope limit, matching NavBuilder's
    /// current Recast `walkableSlopeAngle`.
    pub min_up: f32,
    /// Radius, in BigWorld units, for the "is there ANY geometry around
    /// this point?" neighbourhood counters.
    pub neighbourhood: f32,
}

impl Default for ProbeConfig {
    fn default() -> Self {
        Self {
            below: 1.5,
            above: 0.5,
            // cos(45°).
            min_up: std::f32::consts::FRAC_1_SQRT_2,
            neighbourhood: 5.0,
        }
    }
}

/// Accumulated evidence for one probe point under one mapping.
#[derive(Debug, Clone, Default)]
pub struct PointProbe {
    /// Walkable triangles directly under the point inside the y window.
    pub floor_hits: u64,
    /// Highest qualifying floor surface, in BigWorld y.
    pub best_floor_y: Option<f32>,
    /// Triangles whose XZ footprint contains the point, any orientation,
    /// any height. Zero here means the point is outside the geometry's
    /// horizontal footprint entirely — a transform problem, not a
    /// missing-floor problem.
    pub column_tris: u64,
    /// ...of which walkable.
    pub column_walkable: u64,
    /// Highest walkable triangle anywhere below the point, ignoring the
    /// y window. The gap to the point's y is the vertical error a wrong
    /// scale or offset would produce.
    pub column_walkable_best_below_y: Option<f32>,
    /// Triangles whose bounding box comes within `neighbourhood` of the
    /// point.
    pub near_tris: u64,
    /// ...that are walkable (floor-ish).
    pub near_walkable: u64,
    /// ...that are near-vertical (`|normal.y| < 0.3`) — i.e. walls.
    pub near_vertical: u64,
    /// Distance, in BigWorld units, to the nearest triangle bounding box
    /// seen anywhere — 0 when the point is inside one.
    ///
    /// Measured against the AABB rather than the vertices on purpose: a
    /// single Castle wall section is tens of units across, so its
    /// *vertices* can all be 10+ units from a point standing right
    /// against it. Vertex distance would report "nothing near here" for
    /// a point with a wall in its face, which is precisely the reading
    /// this probe must not get wrong.
    pub nearest_dist: Option<f32>,
}

impl PointProbe {
    /// The headline answer: is there a floor under this point?
    pub fn has_floor(&self) -> bool {
        self.floor_hits > 0
    }

    /// Vertical gap from the point down to the best walkable surface in
    /// its column, ignoring the acceptance window. `None` when the
    /// column holds no walkable triangle below the point at all.
    pub fn column_gap(&self, point_y: f32) -> Option<f32> {
        self.column_walkable_best_below_y.map(|y| point_y - y)
    }
}

/// Runs one [`AxisMapping`] over a stream of triangle soups.
///
/// Soups are fed in UE3 cm (exactly what [`crate::obj::read_obj`] returns
/// for an extractor-written OBJ); the mapping is applied per triangle, so
/// one pass over the map can drive all 48 candidates without ever
/// materialising 48 copies of the geometry.
#[derive(Debug, Clone)]
pub struct ProbeRun {
    pub mapping: AxisMapping,
    pub config: ProbeConfig,
    pub points: Vec<ProbePoint>,
    results: Vec<PointProbe>,
}

impl ProbeRun {
    pub fn new(mapping: AxisMapping, config: ProbeConfig, points: Vec<ProbePoint>) -> Self {
        let results = vec![PointProbe::default(); points.len()];
        Self {
            mapping,
            config,
            points,
            results,
        }
    }

    /// Feed a whole chunk's worth of triangles (UE3 cm).
    pub fn add_soup(&mut self, soup: &TriangleSoup) {
        for face in &soup.faces {
            // 1-based OBJ indices.
            let (a, b, c) = (
                face[0] as usize - 1,
                face[1] as usize - 1,
                face[2] as usize - 1,
            );
            let (Some(&va), Some(&vb), Some(&vc)) = (
                soup.vertices.get(a),
                soup.vertices.get(b),
                soup.vertices.get(c),
            ) else {
                continue;
            };
            self.add_triangle_ue3([va, vb, vc]);
        }
    }

    /// Feed one triangle in UE3 cm.
    pub fn add_triangle_ue3(&mut self, tri_ue3: [[f32; 3]; 3]) {
        let tri = [
            self.mapping.apply(tri_ue3[0]),
            self.mapping.apply(tri_ue3[1]),
            self.mapping.apply(tri_ue3[2]),
        ];
        self.add_triangle_bw(tri);
    }

    /// Feed one triangle already in BigWorld units, in the **emitted**
    /// (UE3-native) index order.
    pub fn add_triangle_bw(&mut self, tri: [[f32; 3]; 3]) {
        let up = recast_up(&tri);
        let walkable = up >= self.config.min_up;
        let vertical = up.abs() < 0.3;

        for (point, out) in self.points.iter().zip(self.results.iter_mut()) {
            accumulate(&tri, up, walkable, vertical, point.bw, &self.config, out);
        }
    }

    /// Per-point results, parallel to `self.points`.
    pub fn results(&self) -> &[PointProbe] {
        &self.results
    }

    /// How many probe points found a floor.
    pub fn points_with_floor(&self) -> usize {
        self.results.iter().filter(|r| r.has_floor()).count()
    }

    /// How many HIGH-confidence probe points found a floor. This is the
    /// score that picks the winning mapping — MEDIUM points are
    /// reconstructions and may legitimately sit inside a wall.
    pub fn high_confidence_hits(&self) -> usize {
        self.points
            .iter()
            .zip(self.results.iter())
            .filter(|(p, r)| p.confidence == Confidence::High && r.has_floor())
            .count()
    }

    /// How many points have any geometry within `neighbourhood`,
    /// regardless of whether it is a floor. Separates "wrong transform"
    /// (nothing anywhere near) from "missing floors" (walls, no floor).
    pub fn points_with_nearby_geometry(&self) -> usize {
        self.results.iter().filter(|r| r.near_tris > 0).count()
    }
}

/// Accumulate one triangle's contribution to one point's evidence.
///
/// Free function rather than a method so the hot loop doesn't re-borrow
/// `self` per point.
fn accumulate(
    tri: &[[f32; 3]; 3],
    up: f32,
    walkable: bool,
    vertical: bool,
    point: [f32; 3],
    cfg: &ProbeConfig,
    out: &mut PointProbe,
) {
    let _ = up;

    // --- neighbourhood (point-to-AABB distance) ---
    let nearest = point_to_triangle_aabb_distance(tri, point);
    if out.nearest_dist.is_none_or(|d| nearest < d) {
        out.nearest_dist = Some(nearest);
    }
    if nearest <= cfg.neighbourhood {
        out.near_tris += 1;
        if walkable {
            out.near_walkable += 1;
        }
        if vertical {
            out.near_vertical += 1;
        }
    }

    // --- vertical column ---
    let Some(surface_y) = xz_height_at(tri, point[0], point[2]) else {
        return;
    };
    out.column_tris += 1;
    if !walkable {
        return;
    }
    out.column_walkable += 1;

    if surface_y <= point[1] + cfg.above
        && out
            .column_walkable_best_below_y
            .is_none_or(|y| surface_y > y)
    {
        out.column_walkable_best_below_y = Some(surface_y);
    }

    let drop = point[1] - surface_y;
    if drop <= cfg.below && drop >= -cfg.above {
        out.floor_hits += 1;
        if out.best_floor_y.is_none_or(|y| surface_y > y) {
            out.best_floor_y = Some(surface_y);
        }
    }
}

/// Euclidean distance from `point` to a triangle's axis-aligned
/// bounding box, in the same units as the inputs. Zero when the point is
/// inside the box.
///
/// A lower bound on the true point-to-triangle distance, which is all
/// the "is there geometry near here?" counters need — and unlike a
/// vertex-distance test it does not blow up on the large wall and floor
/// sections Castle is built from.
pub fn point_to_triangle_aabb_distance(tri: &[[f32; 3]; 3], point: [f32; 3]) -> f32 {
    let mut d2 = 0.0f32;
    for axis in 0..3 {
        let lo = tri[0][axis].min(tri[1][axis]).min(tri[2][axis]);
        let hi = tri[0][axis].max(tri[1][axis]).max(tri[2][axis]);
        let p = point[axis];
        let d = if p < lo {
            lo - p
        } else if p > hi {
            p - hi
        } else {
            0.0
        };
        d2 += d * d;
    }
    d2.sqrt()
}

/// The up component of the normal **Recast will compute** for a
/// triangle the extractor emitted in this index order.
///
/// NavBuilder's `loadOBJ` pushes each face as
/// `(faces[i], faces[i-1], faces[0])` — `mesh.cpp:123-128` — so Recast
/// sees the reverse of the order we wrote, and its normal is the
/// negation of [`triangle_up`]. The axis swizzle itself does not flip
/// anything: `bw = (ue.Y, ue.Z, ue.X)` is a cyclic (even) permutation
/// with determinant +1.
///
/// Getting this sign wrong does not fail loudly — it silently reports
/// ceilings as floors, which for a multi-storey interior looks
/// entirely plausible.
pub fn recast_up(tri: &[[f32; 3]; 3]) -> f32 {
    -triangle_up(tri)
}

/// Unit-normal Y component of a triangle in the order given — positive
/// means the face points up under the standard right-hand rule.
/// Returns 0 for a degenerate triangle.
///
/// This is *not* what Recast sees for an extractor-written OBJ; use
/// [`recast_up`] for that.
pub fn triangle_up(tri: &[[f32; 3]; 3]) -> f32 {
    let e1 = [
        tri[1][0] - tri[0][0],
        tri[1][1] - tri[0][1],
        tri[1][2] - tri[0][2],
    ];
    let e2 = [
        tri[2][0] - tri[0][0],
        tri[2][1] - tri[0][1],
        tri[2][2] - tri[0][2],
    ];
    let n = [
        e1[1] * e2[2] - e1[2] * e2[1],
        e1[2] * e2[0] - e1[0] * e2[2],
        e1[0] * e2[1] - e1[1] * e2[0],
    ];
    let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
    if len <= f32::EPSILON {
        0.0
    } else {
        n[1] / len
    }
}

/// Height of a triangle's plane at `(x, z)`, or `None` if `(x, z)` is
/// outside the triangle's XZ projection (or the triangle is vertical, so
/// has no XZ area).
///
/// Winding-agnostic: a floor triangle must register whether it was
/// emitted clockwise or counter-clockwise, and the kDOP collision lists
/// the extractor reads are not normalised either way.
pub fn xz_height_at(tri: &[[f32; 3]; 3], x: f32, z: f32) -> Option<f32> {
    let (ax, az) = (tri[0][0], tri[0][2]);
    let (bx, bz) = (tri[1][0], tri[1][2]);
    let (cx, cz) = (tri[2][0], tri[2][2]);

    let det = (bz - cz) * (ax - cx) + (cx - bx) * (az - cz);
    if det.abs() < 1e-9 {
        // No XZ area — a vertical wall, or a degenerate sliver.
        return None;
    }
    let l1 = ((bz - cz) * (x - cx) + (cx - bx) * (z - cz)) / det;
    let l2 = ((cz - az) * (x - cx) + (ax - cx) * (z - cz)) / det;
    let l3 = 1.0 - l1 - l2;

    // Small negative tolerance so a point exactly on a shared edge is
    // claimed by both neighbours rather than falling through the crack.
    const EDGE_EPS: f32 = -1e-4;
    if l1 < EDGE_EPS || l2 < EDGE_EPS || l3 < EDGE_EPS {
        return None;
    }
    Some(l1 * tri[0][1] + l2 * tri[1][1] + l3 * tri[2][1])
}

/// The Castle (World 8) probe set.
///
/// HIGH points come from live telemetry captured during the 2026-09-18
/// colo playtest — a player avatar stood on each. MEDIUM points are
/// `db/resources/Worlds/Seed/spawnlist.sql` rows for `world_id = 8`,
/// whose heights were reconstructed from map landmarks in
/// `docs/analysis/castle-rebuild/worknotes/ca05.md`; the room is
/// evidence-backed but the exact spot inside it is not.
pub fn castle_probe_points() -> Vec<ProbePoint> {
    use Confidence::{High, Medium};
    vec![
        ProbePoint::new(
            "Zuritska_cell",
            High,
            [268.0, 66.79, 1042.59],
            "playtest telemetry; also spawnlist 238 Castle_Zuritska_Cell",
        ),
        ProbePoint::new(
            "Romney_corridor_end",
            High,
            [244.0, 66.79, 1036.0],
            "playtest telemetry; also spawnlist 240 Castle_Romney",
        ),
        ProbePoint::new(
            "Level5_comms_room",
            High,
            [271.7, 55.2, 858.0],
            "playtest telemetry; also spawnlist 239 Castle_Zuritska_Comms",
        ),
        ProbePoint::new(
            "Comms_terminal",
            Medium,
            [271.7, 55.2, 855.0],
            "spawnlist 246 Castle_CommsTerminal",
        ),
        ProbePoint::new(
            "Checkpoint_Alpha_DHD",
            Medium,
            [806.27, 55.10, 517.24],
            "spawnlist 2 Castle_DHD",
        ),
        ProbePoint::new(
            "Checkpoint_Alpha_ColMarsh",
            Medium,
            [810.73, 55.20, 515.01],
            "spawnlist 118 Castle_ColMarsh",
        ),
        ProbePoint::new(
            "Interior_Coppleman",
            Medium,
            [352.69, 70.27, 952.32],
            "spawnlist 87 Castle_Coppleman",
        ),
        ProbePoint::new(
            "Interior_SgtGerschon",
            Medium,
            [429.64, 70.11, 996.56],
            "spawnlist 112 Castle_SgtGerschon",
        ),
        ProbePoint::new(
            "Bunker_Muelbach",
            Medium,
            [1008.0, 48.0, 414.0],
            "spawnlist 241 Castle_Muelbach (bunker floor y = 48.00)",
        ),
        ProbePoint::new(
            "Exterior_CheckpointBravo",
            Medium,
            [962.41, 24.80, 469.57],
            "spawnlist 101 Castle_NidGuard9 — OUTDOOR control point; \
             expected to stand on Terrain, which the StaticMesh path does not decode",
        ),
        ProbePoint::new(
            "Exterior_NidGuard16",
            Medium,
            [758.17, 29.88, 421.44],
            "spawnlist 110 Castle_NidGuard16 — second OUTDOOR control point",
        ),
        ProbePoint::new(
            "Interior_AccessPanel",
            Medium,
            [330.49, 41.18, 653.11],
            "spawnlist 92 Castle_AccessPanel",
        ),
    ]
}

#[cfg(test)]
mod tests;
