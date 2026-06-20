//! Unit tests for the byte-level StaticMesh deserializer in [`super`].

use super::*;

#[test]
fn unpack_normal_center() {
    // 127 should map to approximately 0.0 (slightly negative due to integer mapping)
    let n = unpack_normal(&[127, 127, 127, 0]);
    assert!(n[0].abs() < 0.01);
    assert!(n[1].abs() < 0.01);
    assert!(n[2].abs() < 0.01);
}

#[test]
fn unpack_normal_positive() {
    // 255 should map to approximately +1.0
    let n = unpack_normal(&[255, 127, 127, 0]);
    assert!((n[0] - 1.0).abs() < 0.01);
}

#[test]
fn unpack_normal_negative() {
    // 0 should map to approximately -1.0
    let n = unpack_normal(&[0, 127, 127, 0]);
    assert!((n[0] + 1.0).abs() < 0.01);
}

#[test]
fn parse_bounds_roundtrip() {
    let mut buf = vec![0u8; 28];
    // origin
    LittleEndian::write_f32(&mut buf[0..], 1.0);
    LittleEndian::write_f32(&mut buf[4..], 2.0);
    LittleEndian::write_f32(&mut buf[8..], 3.0);
    // extent
    LittleEndian::write_f32(&mut buf[12..], 10.0);
    LittleEndian::write_f32(&mut buf[16..], 20.0);
    LittleEndian::write_f32(&mut buf[20..], 30.0);
    // radius
    LittleEndian::write_f32(&mut buf[24..], 37.4);

    let mut pos = 0;
    let bb = read_bounds(&buf, &mut pos).unwrap();
    assert_eq!(pos, 28);
    assert_eq!(bb.origin, [1.0, 2.0, 3.0]);
    assert_eq!(bb.extent, [10.0, 20.0, 30.0]);
    assert!((bb.sphere_radius - 37.4).abs() < 0.001);
}

#[test]
fn read_empty_kdop_tree() {
    // 0 nodes, 0 triangles
    let mut buf = vec![0u8; 8];
    LittleEndian::write_i32(&mut buf[0..], 0);
    LittleEndian::write_i32(&mut buf[4..], 0);

    let mut pos = 0;
    let tris = read_kdop_tree(&buf, &mut pos).unwrap();
    assert_eq!(pos, 8);
    assert!(tris.is_empty());
}

#[test]
fn read_small_kdop_tree() {
    // 2 nodes (64 bytes) + 3 triangles (24 bytes) = 96 bytes total
    let mut buf = vec![0u8; 4 + 64 + 4 + 24];
    LittleEndian::write_i32(&mut buf[0..], 2);
    LittleEndian::write_i32(&mut buf[68..], 3);
    // Triangle 0 starts at offset 72: indices (0, 1, 2) material 7
    LittleEndian::write_u16(&mut buf[72..], 0);
    LittleEndian::write_u16(&mut buf[74..], 1);
    LittleEndian::write_u16(&mut buf[76..], 2);
    LittleEndian::write_u16(&mut buf[78..], 7);
    // Triangle 1: (3, 4, 5) material 0
    LittleEndian::write_u16(&mut buf[80..], 3);
    LittleEndian::write_u16(&mut buf[82..], 4);
    LittleEndian::write_u16(&mut buf[84..], 5);
    // Triangle 2: (6, 7, 8) material 0
    LittleEndian::write_u16(&mut buf[88..], 6);
    LittleEndian::write_u16(&mut buf[90..], 7);
    LittleEndian::write_u16(&mut buf[92..], 8);

    let mut pos = 0;
    let tris = read_kdop_tree(&buf, &mut pos).unwrap();
    assert_eq!(pos, 96);
    assert_eq!(tris.len(), 3);
    assert_eq!(tris[0].v0, 0);
    assert_eq!(tris[0].v1, 1);
    assert_eq!(tris[0].v2, 2);
    assert_eq!(tris[0].material, 7);
    assert_eq!(tris[1].v0, 3);
    assert_eq!(tris[2].v2, 8);
}

#[test]
fn read_kdop_tree_rejects_oversized_tri_count() {
    // Regression guard for the allocation-bomb class of bug: a
    // malicious file that declares `tri_count = i32::MAX` must be
    // rejected before the parser hits `Vec::with_capacity(...)`,
    // not after a ~16GB allocation attempt.
    //
    // Encode: 0 nodes, then `tri_count = 0x7FFFFFFF`. We don't bother
    // including the (impossible) triangle payload — the rejection
    // must happen before any of those bytes are read.
    let mut buf = vec![0u8; 8];
    LittleEndian::write_i32(&mut buf[0..], 0); // 0 kDOP nodes
    LittleEndian::write_i32(&mut buf[4..], i32::MAX); // malicious tri_count

    let mut pos = 0;
    let result = read_kdop_tree(&buf, &mut pos);
    let err = result.expect_err("oversized tri_count must error, not panic");
    let msg = format!("{err}");
    assert!(
        msg.contains("Unreasonable kDOP triangle count"),
        "error must call out the bounds-check failure; got: {msg}"
    );
}

#[test]
fn read_kdop_tree_rejects_oversized_node_count() {
    // Companion guard for the kDOP node array — same allocation-bomb
    // shape, different field. Catches a malicious file that uses the
    // node-array side to drive the parser into a multi-GB allocation.
    let mut buf = vec![0u8; 4];
    LittleEndian::write_i32(&mut buf[0..], i32::MAX);
    let mut pos = 0;
    let result = read_kdop_tree(&buf, &mut pos);
    let err = result.expect_err("oversized node_count must error");
    assert!(format!("{err}").contains("Unreasonable kDOP node count"));
}

#[test]
fn read_kdop_tree_rejects_negative_tri_count() {
    // Negative counts can't survive a `Vec::with_capacity` cast to
    // usize cleanly — guard explicitly. Without the bounds check the
    // multiplication `tri_count as usize * KDOP_TRI_SIZE` overflows
    // and panics on debug builds.
    let mut buf = vec![0u8; 8];
    LittleEndian::write_i32(&mut buf[0..], 0);
    LittleEndian::write_i32(&mut buf[4..], -1);
    let mut pos = 0;
    let result = read_kdop_tree(&buf, &mut pos);
    assert!(result.is_err(), "negative tri_count must error");
}

#[test]
fn validate_kdop_indices_passes_when_all_in_range() {
    let tris = vec![
        KdopTriangle {
            v0: 0,
            v1: 1,
            v2: 2,
            material: 0,
        },
        KdopTriangle {
            v0: 99,
            v1: 50,
            v2: 25,
            material: 0,
        },
    ];
    assert!(validate_kdop_indices(&tris, Some(100)).is_ok());
}

#[test]
fn validate_kdop_indices_rejects_out_of_range() {
    // A mesh with 100 LOD0 verts can only legally reference indices
    // 0..=99. Index 100 is out of range and could be a u16 wrap from
    // a >65535-vertex mesh — fail fast at parse so the navmesh
    // extractor doesn't see the same triangle later and silently
    // drop it (leaving a hole in the navmesh).
    let tris = vec![
        KdopTriangle {
            v0: 0,
            v1: 1,
            v2: 2,
            material: 0,
        },
        KdopTriangle {
            v0: 0,
            v1: 1,
            v2: 100,
            material: 0,
        },
    ];
    let err = validate_kdop_indices(&tris, Some(100)).expect_err("out-of-range index must error");
    let msg = format!("{err}");
    assert!(
        msg.contains("u16 index wrap"),
        "error message must explain the u16-wrap class of failure; got: {msg}"
    );
}

#[test]
fn validate_kdop_indices_rejects_u16_wrap_scenario() {
    // The literal scenario: a mesh with >65535 verts. The wire can't
    // represent index 70000 (>65535), so the cooker would either
    // have wrapped (silent corruption) or — if the cook chain is
    // sane — never emitted a kDOP for that mesh in the first place.
    // Either way, if we see an out-of-range index that could have
    // come from a u16 wrap, reject.
    let tris = vec![KdopTriangle {
        v0: 0,
        v1: 1,
        v2: 65535,
        material: 0,
    }];
    assert!(validate_kdop_indices(&tris, Some(100)).is_err());
}

#[test]
fn validate_kdop_indices_with_no_lod_is_lenient() {
    // No LOD0 vertex buffer to validate against → pass through. The
    // caller's "no LOD models" path will surface the unusability.
    let tris = vec![KdopTriangle {
        v0: 99,
        v1: 99,
        v2: 99,
        material: 0,
    }];
    assert!(validate_kdop_indices(&tris, None).is_ok());
}

#[test]
fn parse_single_vertex_40byte() {
    let stride = 40;
    let mut buf = vec![0u8; stride];
    // Position: (100.0, 200.0, 300.0)
    LittleEndian::write_f32(&mut buf[0..], 100.0);
    LittleEndian::write_f32(&mut buf[4..], 200.0);
    LittleEndian::write_f32(&mut buf[8..], 300.0);
    // TangentX W (bitangent sign)
    LittleEndian::write_f32(&mut buf[12..], 1.0);
    // TangentX packed: (255, 127, 127, 0) => (+1, 0, 0)
    buf[16] = 255;
    buf[17] = 127;
    buf[18] = 127;
    buf[19] = 0;
    // TangentY packed
    buf[20] = 127;
    buf[21] = 255;
    buf[22] = 127;
    buf[23] = 0;
    // TangentZ packed: (127, 127, 255, 0) => (0, 0, +1)
    buf[24] = 127;
    buf[25] = 127;
    buf[26] = 255;
    buf[27] = 0;
    // Color (white)
    buf[28] = 0xFF;
    buf[29] = 0xFF;
    buf[30] = 0xFF;
    buf[31] = 0xFF;
    // UV: (0.5, 0.75)
    LittleEndian::write_f32(&mut buf[32..], 0.5);
    LittleEndian::write_f32(&mut buf[36..], 0.75);

    let verts = parse_vertices(&buf, 1, stride).unwrap();
    assert_eq!(verts.len(), 1);
    let v = &verts[0];
    assert_eq!(v.position, [100.0, 200.0, 300.0]);
    assert!((v.tangent[3] - 1.0).abs() < 0.01);
    assert!((v.uv[0] - 0.5).abs() < 0.001);
    assert!((v.uv[1] - 0.75).abs() < 0.001);
    // Normal should be approximately (0, 0, 1) from TangentZ
    assert!(v.normal[2] > 0.9);
}
