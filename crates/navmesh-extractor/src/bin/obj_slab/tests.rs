use cimmeria_navmesh_extractor::geometry::TriangleSoup;
use cimmeria_navmesh_extractor::obj::write_obj;

use super::*;

fn argv(s: &[&str]) -> Vec<String> {
    s.iter().map(|x| x.to_string()).collect()
}

fn tmpdir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "cimmeria-objslab-bin-{tag}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// BigWorld metres → UE3 centimetres. `bw = (ue.y, ue.z, ue.x) / 100`,
/// so `ue = (bw.z, bw.x, bw.y) * 100`.
fn ue_cm(bw: [f32; 3]) -> [f32; 3] {
    [bw[2] * 100.0, bw[0] * 100.0, bw[1] * 100.0]
}

/// A flat floor quad at BigWorld height `y`, wound so Recast sees it
/// facing up (`recast_up > 0`, i.e. right-hand normal pointing *down*
/// in the emitted order — see `obj_slab::Surface::faces_up`).
fn floor(x0: f32, z0: f32, x1: f32, z1: f32, y: f32) -> [[[f32; 3]; 3]; 2] {
    [
        [ue_cm([x0, y, z0]), ue_cm([x1, y, z0]), ue_cm([x0, y, z1])],
        [ue_cm([x1, y, z0]), ue_cm([x1, y, z1]), ue_cm([x0, y, z1])],
    ]
}

/// A vertical wall panel along constant x.
fn wall_x(x: f32, z0: f32, z1: f32, y0: f32, y1: f32) -> [[[f32; 3]; 3]; 2] {
    [
        [ue_cm([x, y0, z0]), ue_cm([x, y0, z1]), ue_cm([x, y1, z1])],
        [ue_cm([x, y0, z0]), ue_cm([x, y1, z1]), ue_cm([x, y1, z0])],
    ]
}

/// Write `<stem>.obj` through the **real** OBJ writer, so the bin is
/// tested against the artifact `extract_map` actually produces.
fn write_chunk(dir: &Path, stem: &str, tris_ue_cm: &[[[f32; 3]; 3]]) {
    let mut soup = TriangleSoup::new(Some(format!("Chunk_{stem}")));
    for t in tris_ue_cm {
        soup.push(*t);
    }
    write_obj(&dir.join(format!("{stem}.obj")), &soup).unwrap();
}

fn run_to_string(args: &Args) -> (u8, String) {
    let mut out: Vec<u8> = Vec::new();
    let code = run(&mut out, args).expect("writing to a Vec cannot fail");
    (code, String::from_utf8(out).unwrap())
}

#[test]
fn at_defaults_to_an_8m_horizontal_12m_vertical_box() {
    let s = parse_at("g=280,43.4,875").unwrap();
    assert_eq!(s.name, "g");
    assert!((s.bmin[0] - 272.0).abs() < 1e-3);
    assert!((s.bmax[0] - 288.0).abs() < 1e-3);
    assert!((s.bmin[1] - 31.4).abs() < 1e-3);
    assert!((s.bmax[1] - 55.4).abs() < 1e-3);
}

#[test]
fn at_accepts_explicit_half_extents() {
    let s = parse_at("g=0,0,0,2,3").unwrap();
    assert_eq!(s.bmin, [-2.0, -3.0, -2.0]);
    assert_eq!(s.bmax, [2.0, 3.0, 2.0]);
}

#[test]
fn box_corners_are_sorted_so_either_order_works() {
    let a = parse_box("b=10,20,30,0,0,0").unwrap();
    let b = parse_box("b=0,0,0,10,20,30").unwrap();
    assert_eq!(a.bmin, b.bmin);
    assert_eq!(a.bmax, b.bmax);
    assert_eq!(a.bmin, [0.0, 0.0, 0.0]);
}

#[test]
fn malformed_specs_are_rejected_rather_than_defaulted() {
    assert!(parse_at("no-equals").is_err());
    assert!(parse_at("g=1,2").is_err(), "3 numbers minimum");
    assert!(parse_at("g=1,2,3,4,5,6").is_err(), "5 numbers maximum");
    assert!(parse_box("b=1,2,3").is_err());
    assert!(parse_box("b=1,2,3,4,5,x").is_err());
}

#[test]
fn defaults_are_the_documented_ones() {
    let a = parse_args_from(&argv(&["chunks", "--at", "g=0,0,0"])).unwrap();
    assert_eq!(a.dir, PathBuf::from("chunks"));
    assert_eq!(a.cell, 0.1);
    assert_eq!(a.tilt, 45.0);
    assert_eq!(a.margin, 60.0);
    assert!(a.levels.is_none());
    assert!(a.line.is_none());
    assert!(a.band.is_none());
    assert!(a.columns.is_empty());
}

#[test]
fn flags_parse_into_the_fields_they_name() {
    let a = parse_args_from(&argv(&[
        "chunks",
        "--box",
        "b=0,0,0,1,1,1",
        "--column",
        "2,3",
        "--line",
        "1,2,3,4",
        "--band",
        "0.5,2.5",
        "--levels",
        "0.5",
        "--cell",
        "0.2",
        "--tilt",
        "30",
        "--margin",
        "5",
    ]))
    .unwrap();
    assert_eq!(a.columns, vec![[2.0, 3.0]]);
    assert_eq!(a.line, Some([1.0, 2.0, 3.0, 4.0]));
    assert_eq!(a.band, Some([0.5, 2.5]));
    assert_eq!(a.levels, Some(0.5));
    assert_eq!(a.cell, 0.2);
    assert_eq!(a.tilt, 30.0);
    assert_eq!(a.margin, 5.0);
}

#[test]
fn bad_arguments_are_rejected_rather_than_defaulted() {
    assert!(parse_args_from(&argv(&[])).is_err(), "no dir, no box");
    assert!(
        parse_args_from(&argv(&["chunks"])).is_err(),
        "a box is required"
    );
    assert!(parse_args_from(&argv(&["a", "b", "--at", "g=0,0,0"])).is_err());
    assert!(parse_args_from(&argv(&["chunks", "--nope"])).is_err());
    assert!(parse_args_from(&argv(&["chunks", "--at"])).is_err());
    assert!(parse_args_from(&argv(&["chunks", "--help"])).is_err());
    assert!(parse_args_from(&argv(&["chunks", "--at", "g=0,0,0", "--column", "1"])).is_err());
    assert!(parse_args_from(&argv(&["chunks", "--at", "g=0,0,0", "--line", "1,2,3"])).is_err());
    assert!(parse_args_from(&argv(&["chunks", "--at", "g=0,0,0", "--band", "1"])).is_err());
}

/// `--cell 0` divides through to an infinite occupancy-grid width, and
/// `inf`/`NaN` poison every bound they touch while the report still
/// prints a plausible-looking answer. Both must be refused at parse.
#[test]
fn non_finite_and_non_positive_measurements_are_refused() {
    let with = |flag: &str, v: &str| {
        parse_args_from(&argv(&["chunks", "--at", "g=0,0,0", flag, v])).map(|_| ())
    };
    for flag in ["--cell", "--tilt", "--levels"] {
        assert!(with(flag, "0").is_err(), "{flag} 0");
        assert!(with(flag, "-1").is_err(), "{flag} -1");
        assert!(with(flag, "inf").is_err(), "{flag} inf");
        assert!(with(flag, "NaN").is_err(), "{flag} NaN");
    }
    assert!(with("--margin", "-1").is_err(), "negative margin");
    assert!(with("--margin", "inf").is_err(), "infinite margin");
    assert!(with("--margin", "0").is_ok(), "zero margin is legitimate");
    // The comma-separated forms go through the same finiteness gate.
    assert!(parse_at("g=0,inf,0").is_err());
    assert!(parse_box("b=0,0,0,1,NaN,1").is_err());
    assert!(parse_args_from(&argv(&["chunks", "--at", "g=0,0,0", "--band", "0,inf"])).is_err());
}

/// The happy path end to end: two storeys and a wall in one chunk, read
/// back through the real OBJ writer, reported with every optional
/// section switched on.
#[test]
fn a_populated_box_reports_levels_columns_and_the_clear_width() {
    let dir = tmpdir("populated");
    let mut tris: Vec<[[f32; 3]; 3]> = Vec::new();
    tris.extend(floor(0.0, 0.0, 20.0, 20.0, 0.0));
    tris.extend(floor(0.0, 0.0, 20.0, 20.0, 12.0));
    // Wall at x = 10 with a 1.2 m opening at z ∈ [9, 10.2].
    tris.extend(wall_x(10.0, 0.0, 9.0, 0.0, 3.0));
    tris.extend(wall_x(10.0, 10.2, 20.0, 0.0, 3.0));
    write_chunk(&dir, "00000000o", &tris);

    let args = parse_args_from(&argv(&[
        dir.to_str().unwrap(),
        "--box",
        "well=-1,-1,-1,21,20,21",
        "--column",
        "5,5",
        "--line",
        "10,0,10,20",
        "--band",
        "0.2,1.8",
        "--levels",
        "1.0",
    ]))
    .unwrap();
    let (code, text) = run_to_string(&args);

    assert_eq!(code, 0, "{text}");
    assert!(text.contains("chunks     1 read, 0 skipped"), "{text}");
    assert!(text.contains("box well"), "{text}");
    assert!(text.contains("triangles  8"), "{text}");
    assert!(text.contains("from       00000000o:8"), "{text}");
    // 2 x 400 m² of floor, all of it dead flat, so it lands in the
    // `<=5deg` bucket; the vertical walls contribute no XZ footprint.
    assert!(text.contains("flat<=5deg 800.0 m^2"), "floor area:\n{text}");
    assert!(text.contains("steep>60 0.0 m^2"), "{text}");
    // Both storeys show up as their own height bucket, and the column
    // sees both floors facing up.
    assert!(text.contains("y[   0.00,   1.00]      400.0 m^2"), "{text}");
    assert!(text.contains("y[  12.00,  13.00]      400.0 m^2"), "{text}");
    assert_eq!(
        text.matches("faces_up=true").count(),
        2,
        "both storeys are floors, not ceilings:\n{text}"
    );
    assert!(text.contains("step_from_below=+12.00 m"), "{text}");

    // The doorway measurement itself. Parsed rather than string-matched
    // because the occupancy raster quantises the opening to the 0.1 m
    // cell, so the printed figure is 1.1–1.2 depending on where the
    // wall edges land in the grid.
    let clear = text
        .lines()
        .filter_map(|l| l.trim().strip_prefix("clear "))
        .filter_map(|r| r.split_whitespace().next())
        .filter_map(|n| n.parse::<f32>().ok())
        .fold(0.0f32, f32::max);
    assert!(
        (clear - 1.2).abs() < 0.25,
        "expected a ~1.2 m opening, measured {clear:.2} m:\n{text}"
    );
}

/// An empty box is a finding, not an error: exit 2 and say so.
#[test]
fn an_empty_box_exits_two_and_says_so() {
    let dir = tmpdir("empty");
    write_chunk(&dir, "00000000o", &floor(0.0, 0.0, 5.0, 5.0, 0.0));
    let args = parse_args_from(&argv(&[
        dir.to_str().unwrap(),
        "--at",
        "nowhere=900,50,900,2,2",
    ]))
    .unwrap();
    let (code, text) = run_to_string(&args);
    assert_eq!(code, EXIT_EMPTY, "{text}");
    assert!(text.contains("EMPTY — no source geometry"), "{text}");
    assert!(
        text.contains("1 skipped by the grid pre-filter"),
        "the far chunk is pre-filtered out:\n{text}"
    );
}

/// A directory that is not there is a usage error, not a panic or an
/// empty-but-successful report.
#[test]
fn a_missing_chunk_directory_is_an_error() {
    let args = parse_args_from(&argv(&["no-such-directory-here", "--at", "g=0,0,0"])).unwrap();
    let mut out: Vec<u8> = Vec::new();
    let err = run(&mut out, &args).expect_err("a missing directory must fail");
    assert!(err.contains("no-such-directory-here"), "{err}");
}

/// A corrupt OBJ must fail the run rather than silently under-report.
#[test]
fn a_corrupt_obj_fails_the_run() {
    let dir = tmpdir("corrupt");
    std::fs::write(dir.join("00000000o.obj"), "v 0 0 0\r\nv 1 junk 1\r\n").unwrap();
    let args = parse_args_from(&argv(&[dir.to_str().unwrap(), "--at", "g=0,0,0,50,50"])).unwrap();
    let mut out: Vec<u8> = Vec::new();
    let err = run(&mut out, &args).expect_err("a malformed vertex must fail");
    assert!(err.contains("malformed vertex"), "{err}");
}
