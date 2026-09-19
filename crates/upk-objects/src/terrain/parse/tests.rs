//! Byte-exact unit tests for the `Terrain` deserializer in [`super`].
//!
//! Every fixture is assembled from raw bytes by [`TerrainFixture`] so the
//! tests pin the on-disk layout, not just the struct shape.

use super::*;
use crate::terrain::TID_VISIBILITY_OFF;

/// Name-table indices used by the fixtures. Index 0 is deliberately
/// `None` so a truncated stream that reads a zeroed FName terminates
/// like real data does.
const NAMES: &[&str] = &[
    "None",
    "IntProperty",
    "StructProperty",
    "FloatProperty",
    "Vector",
    "Rotator",
    "NumPatchesX",
    "NumPatchesY",
    "NumVerticesX",
    "NumVerticesY",
    "NumSectionsX",
    "NumSectionsY",
    "Location",
    "Rotation",
    "DrawScale",
    "DrawScale3D",
    "AlphaXSize",
    "AlphaYSize",
];

fn names() -> Vec<NameEntry> {
    NAMES
        .iter()
        .map(|n| NameEntry {
            name: (*n).to_string(),
            flags: 0,
        })
        .collect()
}

fn name_idx(n: &str) -> i32 {
    NAMES.iter().position(|x| *x == n).expect("test name") as i32
}

/// Builds a synthetic `Terrain` export byte-for-byte.
struct TerrainFixture {
    buf: Vec<u8>,
}

impl TerrainFixture {
    /// Starts with the 32-byte actor header.
    fn new() -> Self {
        Self {
            buf: vec![0u8; ACTOR_HEADER_SIZE],
        }
    }

    fn i32(&mut self, v: i32) -> &mut Self {
        self.buf.extend_from_slice(&v.to_le_bytes());
        self
    }

    fn f32(&mut self, v: f32) -> &mut Self {
        self.buf.extend_from_slice(&v.to_le_bytes());
        self
    }

    fn u16(&mut self, v: u16) -> &mut Self {
        self.buf.extend_from_slice(&v.to_le_bytes());
        self
    }

    /// An FName is an i32 name index followed by an i32 instance number.
    fn fname(&mut self, n: &str) -> &mut Self {
        self.i32(name_idx(n)).i32(0)
    }

    fn int_prop(&mut self, name: &str, value: i32) -> &mut Self {
        self.fname(name)
            .fname("IntProperty")
            .i32(4)
            .i32(0)
            .i32(value)
    }

    fn float_prop(&mut self, name: &str, value: f32) -> &mut Self {
        self.fname(name)
            .fname("FloatProperty")
            .i32(4)
            .i32(0)
            .f32(value)
    }

    fn vector_prop(&mut self, name: &str, v: [f32; 3]) -> &mut Self {
        self.fname(name)
            .fname("StructProperty")
            .i32(12)
            .i32(0)
            .fname("Vector")
            .f32(v[0])
            .f32(v[1])
            .f32(v[2])
    }

    fn rotator_prop(&mut self, name: &str, v: [i32; 3]) -> &mut Self {
        self.fname(name)
            .fname("StructProperty")
            .i32(12)
            .i32(0)
            .fname("Rotator")
            .i32(v[0])
            .i32(v[1])
            .i32(v[2])
    }

    /// Writes the `None` terminator that ends the property stream.
    fn end_props(&mut self) -> &mut Self {
        self.fname("None")
    }

    fn bytes(&self) -> &[u8] {
        &self.buf
    }
}

/// A 2×2-patch (3×3-vertex) terrain with a full trailer.
///
/// `heights` and `info` are 9 entries each. `extra_tail` bytes of
/// undecoded lighting trailer are appended.
fn two_by_two(heights: [u16; 9], info: [u8; 9], extra_tail: usize) -> Vec<u8> {
    let mut f = TerrainFixture::new();
    f.int_prop("NumPatchesX", 2)
        .int_prop("NumPatchesY", 2)
        .int_prop("NumVerticesX", 3)
        .int_prop("NumVerticesY", 3)
        .int_prop("NumSectionsX", 1)
        .int_prop("NumSectionsY", 1)
        .int_prop("AlphaXSize", 8)
        .int_prop("AlphaYSize", 8)
        .vector_prop("Location", [1000.0, 2000.0, 300.0])
        .rotator_prop("Rotation", [0, 16384, 0])
        .end_props();

    f.i32(9); // Heights.Num
    for h in heights {
        f.u16(h);
    }
    f.i32(9); // InfoData.Num
    f.buf.extend_from_slice(&info);
    f.i32(8) // AlphaXSize binary copy
        .i32(8) // AlphaYSize binary copy
        .i32(1) // WeightedTextureMaps.Num
        .i32(4); // WeightedTextureMaps[0].Num
    f.buf.extend_from_slice(&[0xAA; 4]);
    f.i32(0); // WeightMapTextures.Num
    f.buf.extend_from_slice(&vec![0x5A; extra_tail]);
    f.bytes().to_vec()
}

fn flat() -> [u16; 9] {
    [0x8000; 9]
}

#[test]
fn parses_a_two_by_two_terrain_byte_exactly() {
    let data = two_by_two(flat(), [0; 9], 0);
    let t = deserialize_terrain(&data, &names()).expect("parse");

    assert_eq!((t.num_patches_x, t.num_patches_y), (2, 2));
    assert_eq!((t.num_vertices_x, t.num_vertices_y), (3, 3));
    assert_eq!((t.num_sections_x, t.num_sections_y), (1, 1));
    assert_eq!(t.location, [1000.0, 2000.0, 300.0]);
    assert_eq!(t.rotation, [0, 16384, 0]);
    assert_eq!(t.draw_scale, 1.0);
    assert_eq!(t.heights.len(), 9);
    assert_eq!(t.info_data.len(), 9);
    assert_eq!((t.alpha_x_size, t.alpha_y_size), (8, 8));
    assert_eq!(t.weighted_texture_map_count, 1);
    assert_eq!(t.weight_map_texture_count, 0);
    // Whole export consumed: no lighting trailer in this fixture.
    assert_eq!(t.lighting_trailer_bytes, 0);
    assert_eq!(t.quad_count(), 4);
    assert_eq!(t.hole_quad_count(), 0);
}

#[test]
fn absent_draw_scale_3d_defaults_to_the_sgw_terrain_class_default() {
    // The fixture never writes DrawScale3D. A generic UE3 actor default
    // of (1,1,1) would collapse every patch to 1 cm — the SGW Terrain
    // class default is 100 cm per patch.
    let data = two_by_two(flat(), [0; 9], 0);
    let t = deserialize_terrain(&data, &names()).expect("parse");
    assert_eq!(t.draw_scale_3d, SGW_TERRAIN_DEFAULT_DRAW_SCALE_3D);
    assert_eq!(t.draw_scale_3d, [100.0, 100.0, 100.0]);
}

#[test]
fn explicit_draw_scale_3d_overrides_the_default() {
    let mut f = TerrainFixture::new();
    f.int_prop("NumPatchesX", 2)
        .int_prop("NumPatchesY", 2)
        .int_prop("NumVerticesX", 3)
        .int_prop("NumVerticesY", 3)
        .vector_prop("DrawScale3D", [100.0, 100.0, 200.0])
        .float_prop("DrawScale", 2.0)
        .end_props();
    f.i32(9);
    for _ in 0..9 {
        f.u16(0x8000);
    }
    f.i32(9);
    f.buf.extend_from_slice(&[0u8; 9]);
    f.i32(0).i32(0).i32(0).i32(0);

    let t = deserialize_terrain(f.bytes(), &names()).expect("parse");
    assert_eq!(t.draw_scale_3d, [100.0, 100.0, 200.0]);
    assert_eq!(t.draw_scale, 2.0);
}

#[test]
fn neutral_height_maps_to_local_zero() {
    let data = two_by_two(flat(), [0; 9], 0);
    let t = deserialize_terrain(&data, &names()).expect("parse");
    for j in 0..3 {
        for i in 0..3 {
            assert_eq!(t.local_vertex(i, j), Some([i as f32, j as f32, 0.0]));
        }
    }
}

#[test]
fn height_above_and_below_neutral_scales_by_one_over_128() {
    let mut h = flat();
    h[0] = 0x8000 + 128; // +1 local unit
    h[8] = 0x8000 - 256; // -2 local units
    let data = two_by_two(h, [0; 9], 0);
    let t = deserialize_terrain(&data, &names()).expect("parse");
    assert_eq!(t.local_vertex(0, 0).unwrap()[2], 1.0);
    assert_eq!(t.local_vertex(2, 2).unwrap()[2], -2.0);
}

#[test]
fn info_data_bit0_marks_the_quad_at_its_lower_left_corner() {
    let mut info = [0u8; 9];
    // Vertex (1,0) is the lower-left corner of quad (1,0).
    info[1] = TID_VISIBILITY_OFF;
    let data = two_by_two(flat(), info, 0);
    let t = deserialize_terrain(&data, &names()).expect("parse");

    assert!(t.quad_visible(0, 0));
    assert!(!t.quad_visible(1, 0));
    assert!(t.quad_visible(0, 1));
    assert!(t.quad_visible(1, 1));
    assert_eq!(t.hole_quad_count(), 1);
}

#[test]
fn final_row_and_column_info_flags_never_gate_a_quad() {
    // Vertex (2,2) is the top-right heightmap corner — it is not the
    // lower-left of any quad, so flagging it must not remove geometry.
    let mut info = [0u8; 9];
    info[8] = TID_VISIBILITY_OFF;
    let data = two_by_two(flat(), info, 0);
    let t = deserialize_terrain(&data, &names()).expect("parse");
    assert_eq!(t.hole_quad_count(), 0);
}

#[test]
fn lighting_trailer_bytes_reports_the_undecoded_tail_exactly() {
    for tail in [0usize, 1, 92, 152, 3304] {
        let data = two_by_two(flat(), [0; 9], tail);
        let t = deserialize_terrain(&data, &names()).expect("parse");
        assert_eq!(
            t.lighting_trailer_bytes, tail,
            "tail size {tail} misreported"
        );
    }
}

#[test]
fn truncated_height_array_is_an_error() {
    let full = two_by_two(flat(), [0; 9], 0);
    // Cut mid-heightmap: the Heights.Num field survives but its payload
    // does not.
    let cut = full.len() - (9 + 4 * 5 + 4 + 9); // drop info + alpha + wtm
    let err = deserialize_terrain(&full[..cut - 4], &names()).expect_err("must fail");
    assert!(
        matches!(err, ObjectError::InvalidData(_)),
        "expected InvalidData, got {err:?}"
    );
}

#[test]
fn truncated_before_weight_map_textures_is_an_error() {
    let full = two_by_two(flat(), [0; 9], 0);
    let err = deserialize_terrain(&full[..full.len() - 4], &names()).expect_err("must fail");
    assert!(
        matches!(err, ObjectError::InvalidData(_)),
        "expected InvalidData, got {err:?}"
    );
}

#[test]
fn overlong_weighted_texture_map_payload_is_an_error() {
    let mut f = TerrainFixture::new();
    f.int_prop("NumPatchesX", 2)
        .int_prop("NumPatchesY", 2)
        .int_prop("NumVerticesX", 3)
        .int_prop("NumVerticesY", 3)
        .end_props();
    f.i32(9);
    for _ in 0..9 {
        f.u16(0x8000);
    }
    f.i32(9);
    f.buf.extend_from_slice(&[0u8; 9]);
    f.i32(0).i32(0).i32(1).i32(1_000_000); // declares 1 MB of payload
    let err = deserialize_terrain(f.bytes(), &names()).expect_err("must fail");
    assert!(matches!(err, ObjectError::InvalidData(_)), "got {err:?}");
}

#[test]
fn grid_mismatch_between_patches_and_vertices_is_an_error() {
    let mut f = TerrainFixture::new();
    f.int_prop("NumPatchesX", 2)
        .int_prop("NumPatchesY", 2)
        .int_prop("NumVerticesX", 2) // should be 3
        .int_prop("NumVerticesY", 3)
        .end_props();
    let err = deserialize_terrain(f.bytes(), &names()).expect_err("must fail");
    assert!(matches!(err, ObjectError::InvalidData(_)), "got {err:?}");
}

#[test]
fn missing_grid_property_is_a_missing_property_error() {
    let mut f = TerrainFixture::new();
    f.int_prop("NumPatchesX", 2).end_props();
    let err = deserialize_terrain(f.bytes(), &names()).expect_err("must fail");
    assert!(
        matches!(err, ObjectError::MissingProperty(ref p) if p == "NumPatchesY"),
        "got {err:?}"
    );
}

#[test]
fn alpha_size_disagreement_between_property_and_trailer_is_an_error() {
    let mut f = TerrainFixture::new();
    f.int_prop("NumPatchesX", 2)
        .int_prop("NumPatchesY", 2)
        .int_prop("NumVerticesX", 3)
        .int_prop("NumVerticesY", 3)
        .int_prop("AlphaXSize", 8)
        .end_props();
    f.i32(9);
    for _ in 0..9 {
        f.u16(0x8000);
    }
    f.i32(9);
    f.buf.extend_from_slice(&[0u8; 9]);
    f.i32(9) // binary copy disagrees with the property's 8
        .i32(8)
        .i32(0)
        .i32(0);
    let err = deserialize_terrain(f.bytes(), &names()).expect_err("must fail");
    assert!(matches!(err, ObjectError::InvalidData(_)), "got {err:?}");
}

#[test]
fn export_smaller_than_the_actor_header_is_an_error() {
    let err = deserialize_terrain(&[0u8; 16], &names()).expect_err("must fail");
    assert!(matches!(err, ObjectError::InvalidData(_)), "got {err:?}");
}

#[test]
fn negative_array_count_is_rejected_before_allocation() {
    let mut f = TerrainFixture::new();
    f.int_prop("NumPatchesX", 2)
        .int_prop("NumPatchesY", 2)
        .int_prop("NumVerticesX", 3)
        .int_prop("NumVerticesY", 3)
        .end_props();
    f.i32(-1); // Heights.Num
    let err = deserialize_terrain(f.bytes(), &names()).expect_err("must fail");
    assert!(matches!(err, ObjectError::InvalidData(_)), "got {err:?}");
}

#[test]
fn oversized_grid_is_rejected_before_allocation() {
    let mut f = TerrainFixture::new();
    f.int_prop("NumPatchesX", 100_000)
        .int_prop("NumPatchesY", 100_000)
        .int_prop("NumVerticesX", 100_001)
        .int_prop("NumVerticesY", 100_001)
        .end_props();
    let err = deserialize_terrain(f.bytes(), &names()).expect_err("must fail");
    assert!(matches!(err, ObjectError::InvalidData(_)), "got {err:?}");
}
