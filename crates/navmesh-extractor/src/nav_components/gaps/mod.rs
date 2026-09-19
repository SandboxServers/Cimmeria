//! Where two walkable components *almost* touch, and what it would take to
//! join them.
//!
//! [`super::NavGraph`] answers "are these two points in the same component".
//! When the answer is no, the next question is always "why, and where" — and
//! the useless answer is "the two components are 300 m apart", which is what
//! you get by measuring centroid to centroid. A route from the Castle gate
//! room to Zuritska's cell is not broken in one place; it is broken wherever
//! a door mesh, a lift shaft or an un-extracted prefab interrupts it.
//!
//! So this module does two things:
//!
//! 1. [`NavGraph::component_gaps`] — index every *boundary* edge (a polygon
//!    edge with no in-mesh neighbour) and find, for every pair of components
//!    that come within `(h_max, v_max)` of each other, the closest approaches
//!    between their rims. Horizontal distance is measured in XZ, the vertical
//!    step separately, because the two have different causes: a horizontal
//!    gap is erosion or missing geometry, a vertical one is `agentClimb`.
//! 2. [`GapGraph::bottleneck_path`] — treat "gap under the thresholds" as a
//!    potential bridge and search for the cheapest chain of intermediate
//!    components linking A to B. A route broken in three places then reads as
//!    three short gaps with named waypoints instead of one impossible jump.
//!
//! The chain search minimises the **widest** gap on the route first and the
//! total second, because the actionable number is "the worst thing you have
//! to bridge", not the sum.

use std::collections::HashMap;

use super::NavGraph;

/// One polygon edge with no neighbour on the other side: the rim of a
/// walkable island.
#[derive(Debug, Clone, Copy)]
pub struct BoundaryEdge {
    pub poly: u32,
    pub component: u32,
    pub a: [f32; 3],
    pub b: [f32; 3],
}

/// The closest approach found between two components' rims.
#[derive(Debug, Clone, Copy)]
pub struct Approach {
    pub from: u32,
    pub to: u32,
    /// XZ distance between the two rims, metres. Zero when the two
    /// footprints overlap (stacked floors).
    pub horizontal: f32,
    /// `to.y - from.y` at the closest approach, metres. Signed: positive
    /// means `to` is the step *up*.
    pub vertical: f32,
    pub point_from: [f32; 3],
    pub point_to: [f32; 3],
}

impl Approach {
    /// How big a bridge this hop needs: the larger of the horizontal gap and
    /// the vertical step.
    ///
    /// Ranking on `horizontal` alone is wrong, and wrong in a way that hides
    /// the answer. Two floors of the same building stacked 12 m apart report
    /// `horizontal = 0.00` because their rims overlap in XZ, so a
    /// horizontal-only cost scores that storey jump as **free** and the chain
    /// search takes it in preference to any real route. On Castle that made
    /// `nav_inspect --gaps` answer "one hop, widest 0.00 m" for a pair whose
    /// actual connection is a stairwell 30 m to the east.
    ///
    /// `max` rather than `hypot` because the two are alternatives, not
    /// components: a 3 m drop and a 3 m gap are each about as hard to cross,
    /// and one of them being small does not help.
    pub fn bridge_size(&self) -> f32 {
        self.horizontal.max(self.vertical.abs())
    }

    fn flipped(&self) -> Self {
        Self {
            from: self.to,
            to: self.from,
            horizontal: self.horizontal,
            vertical: -self.vertical,
            point_from: self.point_to,
            point_to: self.point_from,
        }
    }
}

/// Every pair of components within the search thresholds, with the N best
/// approaches for each pair.
#[derive(Debug, Clone, Default)]
pub struct GapGraph {
    /// Keyed by `(min(a, b), max(a, b))`; the stored [`Approach`]es always
    /// run from the lower id to the higher one. Sorted best-first.
    pairs: HashMap<(u32, u32), Vec<Approach>>,
    pub h_max: f32,
    pub v_max: f32,
}

impl GapGraph {
    /// Approaches between `a` and `b`, oriented `a → b`, best first. Empty
    /// when the pair never came within the thresholds.
    pub fn approaches(&self, a: u32, b: u32) -> Vec<Approach> {
        let key = if a <= b { (a, b) } else { (b, a) };
        match self.pairs.get(&key) {
            None => Vec::new(),
            Some(v) if a <= b => v.clone(),
            Some(v) => v.iter().map(Approach::flipped).collect(),
        }
    }

    /// The single best approach between `a` and `b`, oriented `a → b`.
    pub fn best(&self, a: u32, b: u32) -> Option<Approach> {
        self.approaches(a, b).into_iter().next()
    }

    /// Every pair that came within the thresholds, as `(a, b)` with `a < b`.
    pub fn pair_keys(&self) -> Vec<(u32, u32)> {
        let mut keys: Vec<(u32, u32)> = self.pairs.keys().copied().collect();
        keys.sort_unstable();
        keys
    }

    pub fn pair_count(&self) -> usize {
        self.pairs.len()
    }

    /// Cheapest chain of bridges from `from` to `to`, as the ordered hops.
    ///
    /// "Cheapest" is lexicographic on
    /// `(widest bridge on the route, total bridge)`, where a hop's bridge is
    /// [`Approach::bridge_size`] — `max(horizontal, |vertical|)`, not the
    /// horizontal gap alone. The widest one decides whether the route is
    /// bridgeable at all; the total only breaks ties.
    ///
    /// Returns `None` when no chain exists under the thresholds, and an
    /// empty `Vec` when `from == to`.
    pub fn bottleneck_path(&self, from: u32, to: u32) -> Option<Vec<Approach>> {
        if from == to {
            return Some(Vec::new());
        }
        // Adjacency, both directions, one entry per pair.
        let mut adj: HashMap<u32, Vec<Approach>> = HashMap::new();
        for (key, v) in &self.pairs {
            let Some(best) = v.first() else { continue };
            adj.entry(key.0).or_default().push(*best);
            adj.entry(key.1).or_default().push(best.flipped());
        }

        // Dijkstra with a lexicographic (max, sum) key. The node count here
        // is the component count (~1k on Castle), so a linear scan for the
        // next node is cheaper than the bookkeeping a heap would need to
        // carry the two-part key.
        const INF: (f32, f32) = (f32::INFINITY, f32::INFINITY);
        let mut cost: HashMap<u32, (f32, f32)> = HashMap::new();
        let mut prev: HashMap<u32, Approach> = HashMap::new();
        let mut done: HashMap<u32, bool> = HashMap::new();
        cost.insert(from, (0.0, 0.0));

        loop {
            let mut cur = None;
            let mut cur_cost = INF;
            for (node, c) in &cost {
                if done.get(node).copied().unwrap_or(false) {
                    continue;
                }
                if better(*c, cur_cost) {
                    cur_cost = *c;
                    cur = Some(*node);
                }
            }
            let Some(node) = cur else { break };
            if node == to {
                break;
            }
            done.insert(node, true);
            for hop in adj.get(&node).into_iter().flatten() {
                let bridge = hop.bridge_size();
                let next = (cur_cost.0.max(bridge), cur_cost.1 + bridge);
                let known = cost.get(&hop.to).copied().unwrap_or(INF);
                if better(next, known) {
                    cost.insert(hop.to, next);
                    prev.insert(hop.to, *hop);
                }
            }
        }

        if !cost.contains_key(&to) {
            return None;
        }
        let mut chain = Vec::new();
        let mut node = to;
        while node != from {
            let hop = *prev.get(&node)?;
            chain.push(hop);
            node = hop.from;
            if chain.len() > self.pairs.len() + 1 {
                return None; // cycle guard; unreachable with a correct relax
            }
        }
        chain.reverse();
        Some(chain)
    }
}

/// Lexicographic `(widest, total)` comparison. `f32::total_cmp` so a NaN
/// cannot silently win.
fn better(a: (f32, f32), b: (f32, f32)) -> bool {
    match a.0.total_cmp(&b.0) {
        std::cmp::Ordering::Less => true,
        std::cmp::Ordering::Greater => false,
        std::cmp::Ordering::Equal => a.1.total_cmp(&b.1) == std::cmp::Ordering::Less,
    }
}

/// How far apart two kept approaches for the same pair must be in XZ before
/// they count as separate places rather than the same doorway twice.
const DEFAULT_MIN_SEPARATION: f32 = 4.0;

impl NavGraph {
    /// Every polygon edge with no in-mesh neighbour.
    pub fn boundary_edges(&self) -> Vec<BoundaryEdge> {
        let mut out = Vec::new();
        for (p, poly) in self.polys.iter().enumerate() {
            let n = poly.verts.len();
            for i in 0..n {
                if poly.neighbours[i].is_some() {
                    continue;
                }
                out.push(BoundaryEdge {
                    poly: p as u32,
                    component: self.component[p],
                    a: self.verts[poly.verts[i] as usize],
                    b: self.verts[poly.verts[(i + 1) % n] as usize],
                });
            }
        }
        out
    }

    /// Find the closest approaches between every pair of components whose
    /// rims come within `h_max` horizontally and `v_max` vertically.
    ///
    /// `per_pair` approaches are kept for each pair, each at least
    /// [`DEFAULT_MIN_SEPARATION`] apart in XZ so the list describes distinct
    /// places rather than the same doorway from five adjacent edges.
    pub fn component_gaps(&self, h_max: f32, v_max: f32, per_pair: usize) -> GapGraph {
        self.component_gaps_filtered(h_max, v_max, per_pair, |_| true)
    }

    /// As [`Self::component_gaps`], but only considering components for which
    /// `keep` returns true. Used to cut the search down when only a handful
    /// of components matter; the chain search needs the unfiltered graph.
    pub fn component_gaps_filtered(
        &self,
        h_max: f32,
        v_max: f32,
        per_pair: usize,
        keep: impl Fn(u32) -> bool,
    ) -> GapGraph {
        let edges: Vec<BoundaryEdge> = self
            .boundary_edges()
            .into_iter()
            .filter(|e| keep(e.component))
            .collect();

        // Uniform XZ hash grid. Cell size is the search radius, so a
        // candidate is always within the 3x3 ring around any cell an edge
        // touches. Long contour edges (maxEdgeLen defaults to 12 m) span
        // several cells; that is fine, they are just inserted several times.
        let cell = h_max.max(0.5);
        let mut grid: HashMap<(i32, i32), Vec<u32>> = HashMap::new();
        for (i, e) in edges.iter().enumerate() {
            for c in cells_of(e, cell) {
                grid.entry(c).or_default().push(i as u32);
            }
        }

        let mut pairs: HashMap<(u32, u32), Vec<Approach>> = HashMap::new();
        for (i, e) in edges.iter().enumerate() {
            for (cx, cz) in cells_of(e, cell) {
                for dz in -1..=1 {
                    for dx in -1..=1 {
                        let Some(bucket) = grid.get(&(cx + dx, cz + dz)) else {
                            continue;
                        };
                        for &j in bucket {
                            // Each unordered pair is visited from both sides;
                            // taking only i < j halves the work and keeps the
                            // result identical.
                            if j as usize <= i {
                                continue;
                            }
                            let f = &edges[j as usize];
                            if f.component == e.component {
                                continue;
                            }
                            let Some(app) = approach(e, f, h_max, v_max) else {
                                continue;
                            };
                            insert_approach(&mut pairs, app, per_pair, DEFAULT_MIN_SEPARATION);
                        }
                    }
                }
            }
        }

        GapGraph {
            pairs,
            h_max,
            v_max,
        }
    }
}

/// Grid cells an edge's XZ bounding box touches.
fn cells_of(e: &BoundaryEdge, cell: f32) -> Vec<(i32, i32)> {
    let x0 = (e.a[0].min(e.b[0]) / cell).floor() as i32;
    let x1 = (e.a[0].max(e.b[0]) / cell).floor() as i32;
    let z0 = (e.a[2].min(e.b[2]) / cell).floor() as i32;
    let z1 = (e.a[2].max(e.b[2]) / cell).floor() as i32;
    let mut out = Vec::with_capacity(((x1 - x0 + 1) * (z1 - z0 + 1)).max(1) as usize);
    for z in z0..=z1 {
        for x in x0..=x1 {
            out.push((x, z));
        }
    }
    out
}

/// Closest approach between two boundary edges, oriented low-component-id
/// first, or `None` when it is outside the thresholds.
fn approach(e: &BoundaryEdge, f: &BoundaryEdge, h_max: f32, v_max: f32) -> Option<Approach> {
    let (s, t, h) = closest_params_xz(e.a, e.b, f.a, f.b);
    if h > h_max {
        return None;
    }
    let pe = lerp3(e.a, e.b, s);
    let pf = lerp3(f.a, f.b, t);
    let dy = pf[1] - pe[1];
    if dy.abs() > v_max {
        return None;
    }
    let app = Approach {
        from: e.component,
        to: f.component,
        horizontal: h,
        vertical: dy,
        point_from: pe,
        point_to: pf,
    };
    Some(if e.component <= f.component {
        app
    } else {
        app.flipped()
    })
}

fn insert_approach(
    pairs: &mut HashMap<(u32, u32), Vec<Approach>>,
    app: Approach,
    per_pair: usize,
    min_sep: f32,
) {
    let slot = pairs.entry((app.from, app.to)).or_default();
    // Same place as one we already kept? Keep whichever needs the smaller
    // bridge.
    for kept in slot.iter_mut() {
        if xz_dist(kept.point_from, app.point_from) < min_sep {
            if app.bridge_size() < kept.bridge_size() {
                *kept = app;
                sort_by_bridge(slot);
            }
            return;
        }
    }
    slot.push(app);
    sort_by_bridge(slot);
    slot.truncate(per_pair.max(1));
}

fn sort_by_bridge(slot: &mut [Approach]) {
    slot.sort_by(|a, b| {
        a.bridge_size()
            .total_cmp(&b.bridge_size())
            .then(a.horizontal.total_cmp(&b.horizontal))
    });
}

fn xz_dist(a: [f32; 3], b: [f32; 3]) -> f32 {
    (a[0] - b[0]).hypot(a[2] - b[2])
}

fn lerp3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

/// Closest approach between segments `p0→p1` and `q0→q1` projected onto XZ.
/// Returns `(s, t, distance)` where `s`/`t` are the parameters along each
/// segment. Intersecting segments give distance 0, which is the right answer
/// for stacked floors: the horizontal gap is nil and the whole obstacle is
/// the vertical step.
fn closest_params_xz(p0: [f32; 3], p1: [f32; 3], q0: [f32; 3], q1: [f32; 3]) -> (f32, f32, f32) {
    let (px, pz) = (p1[0] - p0[0], p1[2] - p0[2]);
    let (qx, qz) = (q1[0] - q0[0], q1[2] - q0[2]);
    let (wx, wz) = (p0[0] - q0[0], p0[2] - q0[2]);
    let a = px * px + pz * pz;
    let b = px * qx + pz * qz;
    let c = qx * qx + qz * qz;
    let d = px * wx + pz * wz;
    let e = qx * wx + qz * wz;
    let denom = a * c - b * b;

    // Ericson's clamped segment-segment solve: unclamped `s`, clamp it,
    // re-solve `t` for that `s` and clamp, then re-solve `s` for that `t`.
    // Exact for segments (unlike a single pass, which is only exact when
    // neither parameter clamps). Parallel segments pin `s = 0` and slide `t`.
    let mut s = if denom.abs() < 1e-12 {
        0.0
    } else {
        ((b * e - c * d) / denom).clamp(0.0, 1.0)
    };
    let t = if c > 1e-12 {
        ((e + s * b) / c).clamp(0.0, 1.0)
    } else {
        0.0
    };
    s = if a > 1e-12 {
        ((t * b - d) / a).clamp(0.0, 1.0)
    } else {
        0.0
    };

    let cx = (p0[0] + px * s) - (q0[0] + qx * t);
    let cz = (p0[2] + pz * s) - (q0[2] + qz * t);
    (s, t, cx.hypot(cz))
}

#[cfg(test)]
mod tests;
