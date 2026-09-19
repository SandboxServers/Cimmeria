//! `UModel` (BSP) export bodies.
//!
//! Field order is the table in the `cimmeria_upk_objects::model`
//! module docs. The deserializer requires the declared fields to
//! consume the body **exactly**, so an encoder that drops or adds a
//! trailing scalar fails loudly instead of decoding garbage — which is
//! what makes this encoder worth trusting as a fixture.
//!
//! An empty `Model` encodes to exactly 108 bytes; `tests` pins that
//! against the same arithmetic the module docs quote.

use cimmeria_upk_objects::model::{FBSP_NODE_SIZE, FBSP_SURF_SIZE, FVERT_SIZE};

/// `FBoxSphereBounds`: 6 floats + radius.
const MODEL_BOUNDS_SIZE: usize = 28;

const NODE_OFF_IVERTPOOL: usize = 0x18;
const NODE_OFF_ISURF: usize = 0x1c;
const NODE_OFF_NUMVERTICES: usize = 0x3a;
const NODE_OFF_NODEFLAGS: usize = 0x3b;

const SURF_OFF_POLYFLAGS: usize = 0x04;
const SURF_OFF_VNORMAL: usize = 0x0c;

/// One `FBspNode`: a convex face over `num_vertices` entries of the
/// vertex pool starting at `i_vert_pool`, attributed to `i_surf`.
#[derive(Debug, Clone, Copy)]
pub struct NodeSpec {
    pub i_vert_pool: i32,
    pub i_surf: i32,
    pub num_vertices: u8,
    pub node_flags: u8,
}

/// One `FBspSurf`: the `PolyFlags` the collision filter reads and the
/// `Vectors` index of the authored surface normal.
#[derive(Debug, Clone, Copy)]
pub struct SurfSpec {
    pub poly_flags: u32,
    pub v_normal: i32,
}

/// A synthetic BSP model.
#[derive(Debug, Clone, Default)]
pub struct ModelPayload {
    /// Normal pool — `SurfSpec::v_normal` indexes this.
    pub vectors: Vec<[f32; 3]>,
    /// Vertex position pool.
    pub points: Vec<[f32; 3]>,
    pub nodes: Vec<NodeSpec>,
    pub surfs: Vec<SurfSpec>,
    /// `FVert::pVertex` — the vertex pool, each entry an index into
    /// `points`.
    pub verts: Vec<i32>,
}

impl ModelPayload {
    /// The 108-byte stub every Castle `Brush`-owned `Model` decodes to.
    pub fn empty() -> Self {
        Self::default()
    }

    /// One horizontal quad at UE3 `z`, spanning `x0..x1` by `y0..y1`,
    /// wound so the stored order's right-hand-rule normal points **up**
    /// (+Z) — the winding real BSP node pools use, and the one
    /// `bsp::EMIT_REVERSED` exists to flip.
    ///
    /// `poly_flags` rides on the surface so a fixture can exercise the
    /// collision filter; `surf_normal` is stored in the `Vectors` pool
    /// so `Model::surf_normal` returns the authored value rather than
    /// falling back to the (zeroed) plane.
    pub fn horizontal_quad(
        x0: f32,
        x1: f32,
        y0: f32,
        y1: f32,
        z: f32,
        poly_flags: u32,
        surf_normal: [f32; 3],
    ) -> Self {
        let mut m = Self::default();
        m.push_quad(
            [[x0, y0, z], [x1, y0, z], [x1, y1, z], [x0, y1, z]],
            poly_flags,
            surf_normal,
        );
        m
    }

    /// Append one convex face. Corners are consumed in the order given.
    pub fn push_quad(
        &mut self,
        corners: [[f32; 3]; 4],
        poly_flags: u32,
        surf_normal: [f32; 3],
    ) -> &mut Self {
        self.push_face(&corners, poly_flags, surf_normal)
    }

    /// Append a face of any corner count (3 or more).
    pub fn push_face(
        &mut self,
        corners: &[[f32; 3]],
        poly_flags: u32,
        surf_normal: [f32; 3],
    ) -> &mut Self {
        assert!(corners.len() >= 3, "a BSP face needs at least 3 corners");
        let i_vert_pool = self.verts.len() as i32;
        for c in corners {
            let p = self.points.len() as i32;
            self.points.push(*c);
            self.verts.push(p);
        }
        let v_normal = self.vectors.len() as i32;
        self.vectors.push(surf_normal);
        let i_surf = self.surfs.len() as i32;
        self.surfs.push(SurfSpec {
            poly_flags,
            v_normal,
        });
        self.nodes.push(NodeSpec {
            i_vert_pool,
            i_surf,
            num_vertices: corners.len() as u8,
            node_flags: 0,
        });
        self
    }

    /// Encode the export body.
    pub fn encode(&self) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(&0i32.to_le_bytes()); // NetIndex
        b.extend_from_slice(&0i32.to_le_bytes()); // FName "None" index
        b.extend_from_slice(&0i32.to_le_bytes()); // FName instance number
        b.resize(b.len() + MODEL_BOUNDS_SIZE, 0);

        push_vec3_array(&mut b, &self.vectors);
        push_vec3_array(&mut b, &self.points);

        b.extend_from_slice(&(self.nodes.len() as i32).to_le_bytes());
        for n in &self.nodes {
            let mut raw = vec![0u8; FBSP_NODE_SIZE];
            raw[NODE_OFF_IVERTPOOL..NODE_OFF_IVERTPOOL + 4]
                .copy_from_slice(&n.i_vert_pool.to_le_bytes());
            raw[NODE_OFF_ISURF..NODE_OFF_ISURF + 4].copy_from_slice(&n.i_surf.to_le_bytes());
            raw[NODE_OFF_NUMVERTICES] = n.num_vertices;
            raw[NODE_OFF_NODEFLAGS] = n.node_flags;
            b.extend_from_slice(&raw);
        }

        b.extend_from_slice(&0i32.to_le_bytes()); // ArVer > 0x140 objref

        b.extend_from_slice(&(self.surfs.len() as i32).to_le_bytes());
        for s in &self.surfs {
            let mut raw = vec![0u8; FBSP_SURF_SIZE];
            raw[SURF_OFF_POLYFLAGS..SURF_OFF_POLYFLAGS + 4]
                .copy_from_slice(&s.poly_flags.to_le_bytes());
            raw[SURF_OFF_VNORMAL..SURF_OFF_VNORMAL + 4].copy_from_slice(&s.v_normal.to_le_bytes());
            b.extend_from_slice(&raw);
        }

        b.extend_from_slice(&(self.verts.len() as i32).to_le_bytes());
        for v in &self.verts {
            let mut raw = vec![0u8; FVERT_SIZE];
            raw[0..4].copy_from_slice(&v.to_le_bytes());
            b.extend_from_slice(&raw);
        }

        for scalar in [
            0i32, // NumSharedSides
            0,    // NumZones — no Zones payload follows
            0,    // Polys objref
            0,    // LeafHulls count
            0,    // Leaves count
            1,    // RootOutside
            0,    // Linked
            0,    // PortalNodes count
            0,    // trailing array A count
            0,    // NumUniqueVertices
            0,    // trailing array B count
        ] {
            b.extend_from_slice(&scalar.to_le_bytes());
        }
        b
    }
}

fn push_vec3_array(b: &mut Vec<u8>, v: &[[f32; 3]]) {
    b.extend_from_slice(&(v.len() as i32).to_le_bytes());
    for e in v {
        for c in e {
            b.extend_from_slice(&c.to_le_bytes());
        }
    }
}
