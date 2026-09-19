//! Byte-exact wire-format tests for the `UModel` / `UPolys`
//! deserializers in [`super`].
//!
//! Every fixture is hand-assembled from the layout table in the
//! [`crate::model`] module docs, so a field-order regression shows up
//! as a decode mismatch or an exactness-contract failure rather than as
//! plausible-looking garbage.

use super::*;
use crate::model::types::CollisionFilter;

/// Minimal name table: index 0 is `None`, which terminates an empty
/// tagged-property stream.
fn names() -> Vec<cimmeria_upk::NameEntry> {
    vec![cimmeria_upk::NameEntry {
        name: "None".to_string(),
        flags: 0,
    }]
}

/// Little-endian byte pusher for fixture assembly.
#[derive(Default)]
struct Buf(Vec<u8>);

impl Buf {
    fn i32(&mut self, v: i32) -> &mut Self {
        self.0.extend_from_slice(&v.to_le_bytes());
        self
    }
    fn u32(&mut self, v: u32) -> &mut Self {
        self.0.extend_from_slice(&v.to_le_bytes());
        self
    }
    fn f32(&mut self, v: f32) -> &mut Self {
        self.0.extend_from_slice(&v.to_le_bytes());
        self
    }
    fn vec3(&mut self, v: [f32; 3]) -> &mut Self {
        self.f32(v[0]).f32(v[1]).f32(v[2])
    }
    fn zeros(&mut self, n: usize) -> &mut Self {
        self.0.resize(self.0.len() + n, 0);
        self
    }
    /// NetIndex prefix + a `None`-terminated (empty) property stream.
    fn object_header(&mut self) -> &mut Self {
        self.i32(0); // NetIndex
        self.i32(0).i32(0) // FName "None": name index 0, instance 0
    }
    fn bounds_zero(&mut self) -> &mut Self {
        self.zeros(MODEL_BOUNDS_SIZE)
    }
}

/// Build a `Model` payload whose geometry arrays the caller supplies as
/// pre-encoded element blobs. Everything after `Verts` is emitted as
/// the minimum legal encoding (zero counts) so the total lands exactly
/// on the buffer end.
struct ModelFixture {
    vectors: Vec<[f32; 3]>,
    points: Vec<[f32; 3]>,
    nodes: Vec<Vec<u8>>,
    surfs: Vec<Vec<u8>>,
    verts: Vec<i32>,
}

impl ModelFixture {
    fn empty() -> Self {
        Self {
            vectors: vec![],
            points: vec![],
            nodes: vec![],
            surfs: vec![],
            verts: vec![],
        }
    }

    fn build(&self) -> Vec<u8> {
        let mut b = Buf::default();
        b.object_header();
        b.bounds_zero();

        b.i32(self.vectors.len() as i32);
        for v in &self.vectors {
            b.vec3(*v);
        }
        b.i32(self.points.len() as i32);
        for p in &self.points {
            b.vec3(*p);
        }
        b.i32(self.nodes.len() as i32);
        for n in &self.nodes {
            assert_eq!(n.len(), FBSP_NODE_SIZE, "node fixture must be 68 bytes");
            b.0.extend_from_slice(n);
        }
        b.i32(0); // ArVer > 0x140 unidentified objref
        b.i32(self.surfs.len() as i32);
        for s in &self.surfs {
            assert_eq!(s.len(), FBSP_SURF_SIZE, "surf fixture must be 56 bytes");
            b.0.extend_from_slice(s);
        }
        b.i32(self.verts.len() as i32);
        for v in &self.verts {
            b.i32(*v).zeros(FVERT_SIZE - 4);
        }

        b.i32(0); // NumSharedSides
        b.i32(0); // NumZones (no Zones payload follows)
        b.i32(1234); // Polys objref
        b.i32(0); // LeafHulls count
        b.i32(0); // Leaves count
        b.i32(1); // RootOutside
        b.i32(0); // Linked
        b.i32(0); // PortalNodes count
        b.i32(0); // trailing array A count
        b.i32(0); // NumUniqueVertices
        b.i32(0); // trailing array B count
        b.0
    }
}

/// One `FBspNode`: a face over `num_vertices` verts starting at
/// `i_vert_pool`, pointing at `i_surf`.
fn node_bytes(i_vert_pool: i32, i_surf: i32, num_vertices: u8, node_flags: u8) -> Vec<u8> {
    let mut n = vec![0u8; FBSP_NODE_SIZE];
    n[NODE_OFF_IVERTPOOL..NODE_OFF_IVERTPOOL + 4].copy_from_slice(&i_vert_pool.to_le_bytes());
    n[NODE_OFF_ISURF..NODE_OFF_ISURF + 4].copy_from_slice(&i_surf.to_le_bytes());
    n[NODE_OFF_NUMVERTICES] = num_vertices;
    n[NODE_OFF_NODEFLAGS] = node_flags;
    n
}

/// One `FBspSurf` with the given PolyFlags and normal-pool index.
fn surf_bytes(poly_flags: u32, v_normal: i32) -> Vec<u8> {
    let mut s = vec![0u8; FBSP_SURF_SIZE];
    s[SURF_OFF_POLYFLAGS..SURF_OFF_POLYFLAGS + 4].copy_from_slice(&poly_flags.to_le_bytes());
    s[SURF_OFF_VNORMAL..SURF_OFF_VNORMAL + 4].copy_from_slice(&v_normal.to_le_bytes());
    s
}

// ---------------------------------------------------------------- Model

#[test]
fn empty_model_is_exactly_108_bytes() {
    // The arithmetic self-check from the module docs: NetIndex(4) +
    // `None`(8) + Bounds(28) + seventeen 4-byte scalar/count fields
    // = 108. Every stub `Model` export in Castle-000a0002.umap is
    // exactly this size, so a field added or dropped from the layout
    // moves this number and trips the test.
    let data = ModelFixture::empty().build();
    assert_eq!(data.len(), 108, "empty Model must be 108 bytes on the wire");

    let m = deserialize_model(&data, &names()).expect("empty model parses");
    assert!(m.nodes.is_empty());
    assert!(m.points.is_empty());
    assert!(m.surfs.is_empty());
    assert!(m.verts.is_empty());
    assert_eq!(m.polys_ref, 1234);
    assert!(m.root_outside);
    assert!(!m.linked);
}

#[test]
fn model_bounds_decode_from_the_28_byte_header() {
    let mut b = Buf::default();
    b.object_header();
    b.vec3([1.0, 2.0, 3.0]) // Origin
        .vec3([10.0, 20.0, 30.0]) // BoxExtent
        .f32(37.5); // SphereRadius

    // Vectors, Points, Nodes, objref, Surfs, Verts, NumSharedSides,
    // NumZones, Polys, LeafHulls, Leaves = 11 zero-valued fields…
    for _ in 0..11 {
        b.i32(0);
    }
    // …then RootOutside, Linked, PortalNodes, arrayA,
    // NumUniqueVertices, arrayB = 6 more.
    for _ in 0..6 {
        b.i32(0);
    }
    let m = deserialize_model(&b.0, &names()).expect("parses");
    assert_eq!(m.bounds.origin, [1.0, 2.0, 3.0]);
    assert_eq!(m.bounds.box_extent, [10.0, 20.0, 30.0]);
    assert_eq!(m.bounds.sphere_radius, 37.5);
}

#[test]
fn minimal_model_one_node_three_verts_triangulates() {
    // Points form a triangle in the XY plane; Verts index them in
    // order; one node fans them into a single triangle.
    let mut f = ModelFixture::empty();
    f.vectors = vec![[0.0, 0.0, 1.0]];
    f.points = vec![[0.0, 0.0, 0.0], [100.0, 0.0, 0.0], [0.0, 100.0, 0.0]];
    f.verts = vec![0, 1, 2];
    f.surfs = vec![surf_bytes(0x0000_0E00, 0)];
    f.nodes = vec![node_bytes(0, 0, 3, 0)];
    let data = f.build();

    let m = deserialize_model(&data, &names()).expect("parses");
    assert_eq!(m.nodes.len(), 1);
    assert_eq!(m.nodes[0].i_vert_pool, 0);
    assert_eq!(m.nodes[0].i_surf, 0);
    assert_eq!(m.nodes[0].num_vertices, 3);
    assert_eq!(m.surfs.len(), 1);
    assert_eq!(m.surfs[0].poly_flags, 0x0000_0E00);
    assert_eq!(m.verts.len(), 3);
    assert_eq!(m.points.len(), 3);

    let t = m.triangulate(CollisionFilter::default());
    assert_eq!(t.triangles.len(), 1);
    assert_eq!(
        t.triangles[0],
        [[0.0, 0.0, 0.0], [100.0, 0.0, 0.0], [0.0, 100.0, 0.0]]
    );
    assert_eq!(t.triangle_surf, vec![0]);
    assert_eq!(t.nodes_out_of_range, 0);
}

#[test]
fn node_flags_are_masked_to_low_five_bits_on_load() {
    // `UModel::Serialize`'s IsLoading fixup does `NodeFlags &= 0x1f`.
    // Reproducing it matters because a filter written against observed
    // client behaviour can only ever see bits 0..4.
    let mut f = ModelFixture::empty();
    f.points = vec![[0.0; 3]];
    f.verts = vec![0];
    f.surfs = vec![surf_bytes(0, 0)];
    f.nodes = vec![node_bytes(0, 0, 1, 0xFF)];
    let m = deserialize_model(&f.build(), &names()).expect("parses");
    assert_eq!(m.nodes[0].node_flags, 0x1f);
}

#[test]
fn truncated_model_buffer_errors_instead_of_truncating() {
    let full = ModelFixture::empty().build();
    let truncated = &full[..full.len() - 4];
    let err = deserialize_model(truncated, &names())
        .expect_err("a short buffer must error, not decode a partial Model");
    let msg = format!("{err}");
    assert!(
        msg.contains("requires 4 bytes"),
        "error must name the field that ran out of bytes; got: {msg}"
    );
}

#[test]
fn model_buffer_with_trailing_bytes_errors() {
    // The exactness contract. Every field past `Verts` is skipped by
    // declared size, so an upstream off-by-one surfaces *only* as a
    // non-zero remainder — accepting it would let a mis-parsed Nodes
    // array reach the navmesh looking plausible.
    let mut data = ModelFixture::empty().build();
    data.extend_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD]);
    let err = deserialize_model(&data, &names()).expect_err("trailing bytes must error");
    let msg = format!("{err}");
    assert!(
        msg.contains("not consumed exactly") && msg.contains("4 bytes remaining"),
        "error must report the remainder; got: {msg}"
    );
}

#[test]
fn model_rejects_negative_array_count() {
    let mut b = Buf::default();
    b.object_header();
    b.bounds_zero();
    b.i32(-1); // Vectors count
    let err = deserialize_model(&b.0, &names()).expect_err("negative count must error");
    assert!(format!("{err}").contains("negative element count"));
}

#[test]
fn model_rejects_oversized_array_count_before_allocating() {
    // Allocation-bomb guard: `Nodes` count of i32::MAX would be a
    // ~146GB Vec. The bounds check against the real buffer length must
    // fire before `Vec::with_capacity`.
    let mut b = Buf::default();
    b.object_header();
    b.bounds_zero();
    b.i32(0); // Vectors
    b.i32(0); // Points
    b.i32(i32::MAX); // Nodes
    let err = deserialize_model(&b.0, &names()).expect_err("oversized count must error");
    let msg = format!("{err}");
    assert!(
        msg.contains("Nodes requires"),
        "must fail the Nodes bounds check; got: {msg}"
    );
}

#[test]
fn model_rejects_num_zones_above_the_fixed_array_size() {
    let mut b = Buf::default();
    b.object_header();
    b.bounds_zero();
    b.i32(0).i32(0).i32(0); // Vectors, Points, Nodes
    b.i32(0); // objref
    b.i32(0).i32(0); // Surfs, Verts
    b.i32(0); // NumSharedSides
    b.i32(65); // NumZones — Zones is a fixed [64]
    let err = deserialize_model(&b.0, &names()).expect_err("NumZones > 64 must error");
    assert!(format!("{err}").contains("NumZones 65"));
}

// ---------------------------------------------------------------- Polys

/// One `FPoly` with `verts.len()` vertices — `88 + 12 * N` bytes.
fn poly_bytes(verts: &[[f32; 3]], poly_flags: u32) -> Vec<u8> {
    let mut b = Buf::default();
    b.vec3([1.0, 2.0, 3.0]) // Base
        .vec3([0.0, 0.0, 1.0]) // Normal
        .vec3([1.0, 0.0, 0.0]) // TextureU
        .vec3([0.0, 1.0, 0.0]); // TextureV
    b.i32(verts.len() as i32);
    for v in verts {
        b.vec3(*v);
    }
    b.u32(poly_flags); // PolyFlags
    b.i32(-7); // Actor objref
    b.i32(0).i32(0); // ItemName FName
    b.i32(-9); // Material objref
    b.i32(0).i32(0).i32(0).i32(0); // four unidentified trailing INTs
    assert_eq!(b.0.len(), 88 + 12 * verts.len());
    b.0
}

#[test]
fn fpoly_wire_size_is_88_plus_12_per_vertex() {
    for n in [3usize, 4, 5, 8] {
        let verts: Vec<[f32; 3]> = (0..n).map(|i| [i as f32, 0.0, 0.0]).collect();
        assert_eq!(poly_bytes(&verts, 0).len(), 88 + 12 * n);
    }
}

#[test]
fn polys_decodes_mixed_vertex_counts() {
    // A multi-element export with differing NumVertices is the case
    // that catches a wrong FPoly stride: get the +0x58 gap wrong and
    // element 1 lands four bytes off, so its vertex count decodes as
    // garbage and the exactness check fires.
    let quad = [
        [0.0, 0.0, 0.0],
        [100.0, 0.0, 0.0],
        [100.0, 100.0, 0.0],
        [0.0, 100.0, 0.0],
    ];
    let tri = [[0.0, 0.0, 50.0], [10.0, 0.0, 50.0], [0.0, 10.0, 50.0]];

    let mut b = Buf::default();
    b.object_header();
    b.i32(2); // Count
    b.i32(2); // legacy Max (discarded)
    b.i32(931); // legacy objref (discarded)
    b.0.extend_from_slice(&poly_bytes(&quad, 0x0000_0200));
    b.0.extend_from_slice(&poly_bytes(&tri, 0x0000_0E00));

    let p = deserialize_polys(&b.0, &names()).expect("parses");
    assert_eq!(p.elements.len(), 2);
    assert_eq!(p.elements[0].vertices.len(), 4);
    assert_eq!(p.elements[0].vertices[2], [100.0, 100.0, 0.0]);
    assert_eq!(p.elements[0].poly_flags, 0x0000_0200);
    assert_eq!(p.elements[0].actor_ref, -7);
    assert_eq!(p.elements[0].material_ref, -9);
    assert_eq!(p.elements[0].base, [1.0, 2.0, 3.0]);
    assert_eq!(p.elements[1].vertices.len(), 3);
    assert_eq!(p.elements[1].poly_flags, 0x0000_0E00);
    // Fan triangulation: quad -> 2 tris, tri -> 1.
    assert_eq!(p.triangle_count(), 3);
    assert_eq!(p.elements[0].triangles().len(), 2);
    assert_eq!(p.elements[1].triangles().len(), 1);
}

#[test]
fn empty_polys_is_exactly_24_bytes() {
    // NetIndex(4) + `None`(8) + the three-i32 Element header(12).
    let mut b = Buf::default();
    b.object_header();
    b.i32(0).i32(0).i32(0);
    assert_eq!(b.0.len(), 24);
    let p = deserialize_polys(&b.0, &names()).expect("parses");
    assert!(p.elements.is_empty());
    assert_eq!(p.triangle_count(), 0);
}

#[test]
fn truncated_polys_buffer_errors() {
    let quad = [
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
    ];
    let mut b = Buf::default();
    b.object_header();
    b.i32(1).i32(1).i32(0);
    let mut poly = poly_bytes(&quad, 0);
    poly.truncate(poly.len() - 8);
    b.0.extend_from_slice(&poly);
    let err = deserialize_polys(&b.0, &names()).expect_err("short FPoly must error");
    assert!(format!("{err}").contains("requires"));
}

#[test]
fn polys_buffer_with_trailing_bytes_errors() {
    let mut b = Buf::default();
    b.object_header();
    b.i32(0).i32(0).i32(0);
    b.i32(0xDEAD_BEEFu32 as i32);
    let err = deserialize_polys(&b.0, &names()).expect_err("trailing bytes must error");
    assert!(format!("{err}").contains("not consumed exactly"));
}

#[test]
fn polys_rejects_count_larger_than_the_buffer_can_hold() {
    let mut b = Buf::default();
    b.object_header();
    b.i32(i32::MAX).i32(0).i32(0);
    let err = deserialize_polys(&b.0, &names()).expect_err("oversized count must error");
    assert!(format!("{err}").contains("needs at least"));
}
