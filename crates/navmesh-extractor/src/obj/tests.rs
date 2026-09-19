//! Unit tests for the OBJ writer and reader.
//!
//! Split out of `mod.rs` at the `#[cfg(test)]` seam to keep that module
//! under the repo's 500-line soft cap. Most of these are byte-exact
//! pins on things NavBuilder punishes silently -- the Y/Z column swap,
//! CRLF endings, and the `Terrain_` group prefix.

use super::*;
use crate::geometry::TriangleSoup;

/// Vertices go out as `v <ue.X> <ue.Z> <ue.Y>`. Emitting raw
/// `(X, Y, Z)` instead makes NavBuilder rasterise every floor as a
/// wall and emit an empty 72-byte `.nav`.
#[test]
fn emits_vertices_with_ue3_y_and_z_swapped() {
    let mut soup = TriangleSoup::new(None);
    soup.push([[1.0, 2.0, 3.0], [4.0, 5.0, 6.0], [7.0, 8.0, 9.0]]);

    let mut buf = Vec::new();
    write_obj_into(&mut buf, &[soup]).unwrap();
    let s = String::from_utf8(buf).unwrap();

    assert!(s.contains("v 1 3 2\r\n"), "{s}");
    assert!(s.contains("v 4 6 5\r\n"), "{s}");
    assert!(s.contains("v 7 9 8\r\n"), "{s}");
    assert!(s.contains("f 1 2 3\r\n"), "{s}");
}

/// Every line must end CRLF. NavBuilder's `f` parser loops
/// `while (pos < length - 1)` and silently drops the final index of
/// an LF-terminated face line whose last token is one digit.
#[test]
fn every_emitted_line_ends_crlf() {
    let mut soup = TriangleSoup::new(Some("Chunk_000a0002".to_string()));
    soup.push([[1.0, 2.0, 3.0], [4.0, 5.0, 6.0], [7.0, 8.0, 9.0]]);

    let mut buf = Vec::new();
    write_obj_into(&mut buf, &[soup]).unwrap();
    let s = String::from_utf8(buf).unwrap();

    assert!(s.ends_with("\r\n"));
    for (i, line) in s.split("\r\n").enumerate() {
        assert!(
            !line.contains('\n') && !line.contains('\r'),
            "line {i} has a bare newline: {line:?}"
        );
    }
    // A lone-LF file would have fewer CRLF splits than lines.
    assert_eq!(
        s.matches("\r\n").count(),
        // 3 comments + 1 group + 3 vertices + 1 face
        8,
        "{s}"
    );
}

/// `f 1 2 3` is the exact shape the LF bug eats — the trailing
/// index is one character. Pin that the writer never emits it
/// without a two-character terminator.
#[test]
fn single_digit_face_index_still_has_two_trailing_chars() {
    let mut soup = TriangleSoup::new(None);
    soup.push([[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]);
    let mut buf = Vec::new();
    write_obj_into(&mut buf, &[soup]).unwrap();
    let s = String::from_utf8(buf).unwrap();
    let face = s.lines().find(|l| l.starts_with("f ")).unwrap();
    assert_eq!(face.trim_end_matches('\r'), "f 1 2 3");
    assert!(s.contains("f 1 2 3\r\n"));
}

#[test]
fn emits_group_label_before_geometry() {
    let mut soup = TriangleSoup::new(Some("Chunk_00000000A".to_string()));
    soup.push([[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]);

    let mut buf = Vec::new();
    write_obj_into(&mut buf, &[soup]).unwrap();
    let s = String::from_utf8(buf).unwrap();

    let group_pos = s.find("o Chunk_00000000A").expect("group line present");
    let first_v = s.find("\nv ").expect("vertex line present");
    assert!(
        group_pos < first_v,
        "group line must precede the first vertex"
    );
}

#[test]
fn combined_obj_rebases_indices_across_soups() {
    // Two soups, each with a single triangle. The combined OBJ
    // must address the second soup's triangle via vertex indices
    // 4, 5, 6 — not the local 1, 2, 3.
    let mut a = TriangleSoup::new(None);
    a.push([[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]);
    let mut b = TriangleSoup::new(None);
    b.push([[2.0, 2.0, 2.0], [3.0, 2.0, 2.0], [2.0, 3.0, 2.0]]);

    let mut buf = Vec::new();
    write_obj_into(&mut buf, &[a, b]).unwrap();
    let s = String::from_utf8(buf).unwrap();

    assert!(s.contains("f 1 2 3\r\n"));
    assert!(s.contains("f 4 5 6\r\n"));
}

#[test]
fn ue3_obj_swizzle_is_self_inverse() {
    let v = [1.0f32, 2.0, 3.0];
    assert_eq!(ue3_to_obj(v), [1.0, 3.0, 2.0]);
    assert_eq!(obj_to_ue3(ue3_to_obj(v)), v);
}

// ----- reader -----

/// `read_obj_from` returns OBJ coordinates verbatim, so a raw
/// round-trip comes back Y/Z-swapped. `read_obj_as_ue3` is the one
/// that undoes it — see the next test.
#[test]
fn write_then_read_round_trips_faces_in_obj_coordinates() {
    let mut soup = TriangleSoup::new(Some("Chunk_000a0002".to_string()));
    soup.push([[1.5, -2.25, 3.0], [4.0, 5.0, 6.0], [7.0, 8.0, 9.5]]);
    soup.push([[10.0, 11.0, 12.0], [13.0, 14.0, 15.0], [16.0, 17.0, 18.0]]);

    let mut buf = Vec::new();
    write_obj_into(&mut buf, std::slice::from_ref(&soup)).unwrap();
    let back = read_obj_from(buf.as_slice()).unwrap();

    assert_eq!(back.triangle_count(), 2);
    assert_eq!(back.faces, soup.faces);
    assert_eq!(back.group.as_deref(), Some("Chunk_000a0002"));
    let expected: Vec<[f32; 3]> = soup.vertices.iter().map(|v| ue3_to_obj(*v)).collect();
    assert_eq!(back.vertices, expected);
}

#[test]
fn read_obj_as_ue3_undoes_the_writer_swizzle() {
    let mut soup = TriangleSoup::new(None);
    soup.push([[1.5, -2.25, 3.0], [4.0, 5.0, 6.0], [7.0, 8.0, 9.5]]);

    let dir = std::env::temp_dir().join(format!(
        "cimmeria-obj-roundtrip-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("00000000o.obj");
    write_obj(&path, &soup).unwrap();

    let back = read_obj_as_ue3(&path).unwrap();
    assert_eq!(back.vertices, soup.vertices);
    assert_eq!(back.faces, soup.faces);

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn reader_skips_comments_normals_and_unknown_keywords() {
    let src = b"# header\nvt 0 0\nvn 0 1 0\nusemtl foo\ns off\n\
                v 0 0 0\nv 1 0 0\nv 0 0 1\nf 1 2 3\n";
    let soup = read_obj_from(&src[..]).unwrap();
    assert_eq!(soup.triangle_count(), 1);
    assert_eq!(soup.vertices.len(), 3);
}

#[test]
fn reader_strips_slash_suffixes_from_face_tokens() {
    let src = b"v 0 0 0\nv 1 0 0\nv 0 0 1\nf 1/1/1 2/2/1 3//1\n";
    let soup = read_obj_from(&src[..]).unwrap();
    assert_eq!(soup.triangle_count(), 1);
    assert_eq!(soup.vertices[1], [1.0, 0.0, 0.0]);
}

#[test]
fn reader_fan_triangulates_a_quad() {
    let src = b"v 0 0 0\nv 1 0 0\nv 1 0 1\nv 0 0 1\nf 1 2 3 4\n";
    let soup = read_obj_from(&src[..]).unwrap();
    assert_eq!(soup.triangle_count(), 2);
    // Fan from vertex 0: (0,1,2) then (0,2,3).
    assert_eq!(soup.vertices[3], [0.0, 0.0, 0.0]);
    assert_eq!(soup.vertices[4], [1.0, 0.0, 1.0]);
    assert_eq!(soup.vertices[5], [0.0, 0.0, 1.0]);
}

#[test]
fn reader_resolves_negative_relative_indices() {
    let src = b"v 0 0 0\nv 1 0 0\nv 0 0 1\nf -3 -2 -1\n";
    let soup = read_obj_from(&src[..]).unwrap();
    assert_eq!(soup.triangle_count(), 1);
    assert_eq!(soup.vertices[0], [0.0, 0.0, 0.0]);
    assert_eq!(soup.vertices[2], [0.0, 0.0, 1.0]);
}

/// A truncated OBJ must fail loudly. Silently dropping the face
/// would make the floor probe report "no geometry here" for what is
/// really a broken file — the exact kind of false negative this
/// spike must not produce.
#[test]
fn reader_errors_on_a_face_index_past_the_vertex_list() {
    let src = b"v 0 0 0\nv 1 0 0\nf 1 2 3\n";
    let err = read_obj_from(&src[..]).unwrap_err();
    assert!(
        format!("{err}").contains("exceeds"),
        "unexpected error: {err}"
    );
}

#[test]
fn reader_errors_on_a_zero_face_index() {
    let src = b"v 0 0 0\nv 1 0 0\nv 0 0 1\nf 0 1 2\n";
    assert!(read_obj_from(&src[..]).is_err());
}

#[test]
fn reader_errors_on_a_short_vertex_line() {
    let src = b"v 0 0\n";
    assert!(read_obj_from(&src[..]).is_err());
}

/// The bug shape the positional parse exists for: a junk token in
/// the middle of a vertex line, with a surplus token after it. A
/// `filter_map(parse.ok())` reader drops `abc`, shifts `2` and `3`
/// left, reads three coordinates, and accepts the line.
#[test]
fn reader_errors_on_a_junk_coordinate_with_a_surplus_token() {
    let src = b"v 1 abc 2 3\nv 1 0 0\nv 0 1 0\nf 1 2 3\n";
    let err = read_obj_from(&src[..]).unwrap_err();
    assert!(
        format!("{err}").contains("malformed vertex"),
        "unexpected error: {err}"
    );
}

/// A junk token in the last coordinate slot must fail too — the
/// length check alone would have caught this one, the positional
/// parse catches it for the right reason.
#[test]
fn reader_errors_on_a_junk_final_coordinate() {
    let src = b"v 1 2 NaNsense\n";
    assert!(read_obj_from(&src[..]).is_err());
}

/// Surplus *valid* tokens (a `v x y z w` rational vertex) are still
/// accepted, reading only the first three — dropping that
/// tolerance would reject legitimate OBJ.
#[test]
fn reader_accepts_a_four_component_vertex_and_reads_the_first_three() {
    let src = b"v 1 2 3 1.0\nv 4 5 6\nv 7 8 9\nf 1 2 3\n";
    let soup = read_obj_from(&src[..]).unwrap();
    assert_eq!(soup.vertices[0], [1.0, 2.0, 3.0]);
}

/// `Terrain_*` groups are dropped wholesale by NavBuilder's
/// `loadOBJ`, so the writer must refuse rather than emit an OBJ
/// that silently rasterises to nothing.
#[test]
fn writer_refuses_a_terrain_prefixed_group() {
    let mut soup = TriangleSoup::new(Some("Terrain_000a0002".to_string()));
    soup.push([[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]);
    let mut buf = Vec::new();
    let err = write_obj_into(&mut buf, &[soup]).unwrap_err();
    assert!(
        format!("{err}").contains("Terrain_"),
        "unexpected error: {err}"
    );
}

/// The near-miss must still be allowed: only the exact `Terrain_`
/// prefix is special to NavBuilder.
#[test]
fn writer_allows_a_group_merely_containing_terrain() {
    let mut soup = TriangleSoup::new(Some("Chunk_Terrain_000a0002".to_string()));
    soup.push([[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]);
    let mut buf = Vec::new();
    write_obj_into(&mut buf, &[soup]).unwrap();
    let s = String::from_utf8(buf).unwrap();
    assert!(s.contains("o Chunk_Terrain_000a0002\r\n"), "{s}");
}
