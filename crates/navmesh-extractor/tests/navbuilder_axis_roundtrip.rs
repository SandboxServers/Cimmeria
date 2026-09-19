//! OBJ → NavBuilder → `.nav` round-trip that pins the axis convention.
//!
//! The extractor writes OBJ; the prebuilt C++ `NavBuilder_d.exe` reads it
//! and emits an XRC `.nav` in BigWorld coordinates. Nothing in either half
//! validates the other, and a mirrored or rotated mesh loads perfectly
//! happily — it just paths NPCs into walls. This test closes that loop on
//! a deliberately asymmetric fixture: an L-shaped floor plus a ramp, in a
//! non-square chunk (grid X=3, Z=10), authored from known UE3 centimetre
//! coordinates so an axis swap, a sign flip and a scale error all produce
//! visibly different numbers.
//!
//! Expected mapping (see `docs/engine/navmesh-build-pipeline.md`):
//!
//! ```text
//! OBJ line       : v <ue_x_cm> <ue_z_cm> <ue_y_cm>       (Z-up → Y-up)
//! NavBuilder     : bw = (obj_z/100, obj_y/100, obj_x/100)  (mesh.cpp:106-108)
//! net            : bw = (ue_y/100, ue_z/100, ue_x/100)
//! ```
//!
//! Self-skips (loudly) when `NavBuilder_d.exe` is absent — override the
//! location with `CIMMERIA_NAVBUILDER`.

use std::path::{Path, PathBuf};
use std::process::Command;

use cimmeria_navmesh_extractor::geometry::TriangleSoup;
use cimmeria_navmesh_extractor::nav_components::NavGraph;
use cimmeria_navmesh_extractor::nav_roundtrip::XrcNav;
use cimmeria_navmesh_extractor::obj::write_obj_into;

/// Chunk grid X = 3 (→ BW x ∈ [300, 400]), Z = 10 (→ BW z ∈ [1000, 1100]).
/// Deliberately not square and not zero so an X/Z swap is detectable.
const CHUNK_ID: u32 = 0x000a_0003;

// Fixture extents in UE3 centimetres.
const UX0: f32 = 100_000.0; // → BW z 1000
const UX1: f32 = 106_000.0; // → BW z 1060
const UX2: f32 = 107_500.0; // → BW z 1075 (ramp top)
const UY0: f32 = 30_000.0; //  → BW x 300
const UY1: f32 = 32_000.0; //  → BW x 320
const UY2: f32 = 34_000.0; //  → BW x 340
const UZ_FLOOR: f32 = 5_000.0; // → BW y 50
const UZ_RAMP: f32 = 5_300.0; //  → BW y 53

fn navbuilder_path() -> PathBuf {
    if let Ok(p) = std::env::var("CIMMERIA_NAVBUILDER") {
        return PathBuf::from(p);
    }
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for ancestor in manifest.ancestors().take(6) {
        let c = ancestor.join("bin64").join("NavBuilder_d.exe");
        if c.exists() {
            return c;
        }
    }
    PathBuf::from("bin64/NavBuilder_d.exe")
}

fn unique_tempdir(prefix: &str) -> PathBuf {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let dir = std::env::temp_dir().join(format!("{prefix}-{}-{nanos}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Two triangles covering an axis-aligned rectangle in the UE3 XY plane,
/// with the height interpolated along X so a ramp is one call.
///
/// Winding matches UE3's native (D3D, left-handed) front-face order for an
/// upward-facing surface: listed in this order the right-hand-rule normal
/// points at −Z in UE3. NavBuilder reverses every triangle on load
/// (`mesh.cpp:124-129`) and the Z↔Y column swap in the OBJ is a reflection,
/// so the two sign flips cancel and Recast sees a +Y normal.
fn floor_quad(x0: f32, y0: f32, x1: f32, y1: f32, z0: f32, z1: f32) -> [[[f32; 3]; 3]; 2] {
    let h = |x: f32| {
        if (x1 - x0).abs() < f32::EPSILON {
            z0
        } else {
            z0 + (z1 - z0) * (x - x0) / (x1 - x0)
        }
    };
    let a = [x0, y0, h(x0)];
    let b = [x1, y0, h(x1)];
    let c = [x1, y1, h(x1)];
    let d = [x0, y1, h(x0)];
    [[a, c, b], [a, d, c]]
}

/// The soup convention the extractor really uses: raw UE3 centimetres.
/// `obj::write_obj_into` owns the Y/Z column swap NavBuilder needs.
fn raw_ue3(v: [f32; 3]) -> [f32; 3] {
    v
}

/// Pre-swap Y and Z so the writer's own swap cancels out and the file on
/// disk carries raw UE3 column order — what `obj.rs` emitted before the
/// convention was pinned. Exists only to keep the failure shape guarded.
fn cancels_writer_swap(v: [f32; 3]) -> [f32; 3] {
    [v[0], v[2], v[1]]
}

fn fixture_soup(convention: fn([f32; 3]) -> [f32; 3]) -> TriangleSoup {
    let mut soup = TriangleSoup::new(Some(format!("Chunk_{CHUNK_ID:08x}")));
    let mut quads = Vec::new();
    // Long leg: 60 m × 20 m.
    quads.extend(floor_quad(UX0, UY0, UX1, UY1, UZ_FLOOR, UZ_FLOOR));
    // Stub making the footprint an L, at the low-X end only.
    quads.extend(floor_quad(UX0, UY1, UX0 + 2_000.0, UY2, UZ_FLOOR, UZ_FLOOR));
    // Ramp continuing past the long leg, rising 3 m over 15 m (≈11°).
    quads.extend(floor_quad(UX1, UY0, UX2, UY1, UZ_FLOOR, UZ_RAMP));
    for t in quads {
        soup.push([convention(t[0]), convention(t[1]), convention(t[2])]);
    }
    soup
}

/// Write the soup exactly as `obj::write_obj_into` does (Y/Z-swapped,
/// CRLF); `crlf = false` downgrades the line endings to reproduce the
/// face-dropping failure.
///
/// NavBuilder's face parser loops `while (pos < line.length() - 1)`
/// (`mesh.cpp:115`), so the last token on an `f` line needs one trailing
/// character to be consumed. The original C++ exporter wrote CRLF
/// (`mesh_exporter.cpp:56`) and the `\r` supplied it; with bare LF every
/// face whose third index is a single digit is silently dropped — the
/// first three faces of every file.
fn write_obj(path: &Path, soup: &TriangleSoup, crlf: bool) {
    let mut buf = Vec::new();
    write_obj_into(&mut buf, std::slice::from_ref(soup)).expect("write_obj_into");
    let text = String::from_utf8(buf).expect("obj is ascii");
    assert!(
        text.contains("\r\n"),
        "obj::write_obj_into must emit CRLF — NavBuilder drops faces otherwise"
    );
    let out = if crlf {
        text
    } else {
        text.replace("\r\n", "\n")
    };
    std::fs::write(path, out).expect("write obj");
}

/// `EXIT_BUILD_FAILED` from `build_params.hpp`'s exit-code table.
const EXIT_BUILD_FAILED: i32 = 3;

/// Returns `None` when NavBuilder produced no navmesh.
///
/// Two shapes count as "no navmesh": exit 0 with no file written (the
/// legacy behaviour — `exportNavmesh` logged `FAULT` and returned
/// `void`, which is the silent failure the exit-code table was added to
/// end), and exit 3, which says the same thing out loud. Any other code
/// is a real error and fails the test.
fn run_navbuilder(exe: &Path, chunk_dir: &Path, out: &Path) -> Option<XrcNav> {
    let _ = std::fs::remove_file(out);
    let status = Command::new(exe)
        .arg("chunked")
        .arg(chunk_dir)
        .arg(out)
        .arg("nav")
        .status()
        .expect("spawn NavBuilder");
    let code = status.code().expect("no signal on Windows");
    assert!(
        code == 0 || code == EXIT_BUILD_FAILED,
        "NavBuilder returned exit code {code}"
    );
    if code == EXIT_BUILD_FAILED {
        assert!(
            !out.exists(),
            "a reported build failure must not also leave a .nav behind"
        );
        return None;
    }
    if !out.exists() {
        return None;
    }
    let bytes = std::fs::read(out).expect("read nav");
    Some(XrcNav::read(&mut std::io::Cursor::new(&bytes)).expect("parse nav"))
}

fn build(
    exe: &Path,
    tag: &str,
    convention: fn([f32; 3]) -> [f32; 3],
    crlf: bool,
) -> Option<XrcNav> {
    let dir = unique_tempdir(&format!("cimmeria-navaxis-{tag}"));
    let chunk_dir = dir.join("chunks");
    std::fs::create_dir_all(&chunk_dir).unwrap();
    write_obj(
        &chunk_dir.join(format!("{CHUNK_ID:08x}o.obj")),
        &fixture_soup(convention),
        crlf,
    );
    run_navbuilder(exe, &chunk_dir, &dir.join("out.nav"))
}

#[test]
fn navbuilder_maps_ue3_cm_to_bigworld_metres() {
    let exe = navbuilder_path();
    if !exe.exists() {
        eprintln!(
            "SKIPPED navbuilder_maps_ue3_cm_to_bigworld_metres — NavBuilder not found at {}. \
             Set CIMMERIA_NAVBUILDER to run it.",
            exe.display()
        );
        return;
    }

    let nav = build(&exe, "swizzled", raw_ue3, true)
        .expect("NavBuilder produced no .nav for the swizzled fixture");
    let graph = NavGraph::from_nav(&nav);

    assert!(nav.npolys > 0, "swizzled fixture must produce polygons");
    assert_eq!(
        graph.component_count, 1,
        "the L + ramp is one contiguous surface; got {} components",
        graph.component_count
    );

    // Walkable extents, in BigWorld units. Recast erodes the walkable area
    // by agentRadius (0.6 m → 2 cells at cs = 0.3) and then simplifies the
    // contour, so the surface sits ~0.6–1.1 m inside the authored footprint.
    // The tolerance is one-sided: the mesh may shrink, never grow.
    let (mut lo, mut hi) = ([f32::INFINITY; 3], [f32::NEG_INFINITY; 3]);
    for v in &graph.verts {
        for k in 0..3 {
            lo[k] = lo[k].min(v[k]);
            hi[k] = hi[k].max(v[k]);
        }
    }
    let inset = |authored: f32, got: f32| (got - authored).abs() <= 1.5;

    // BW x comes from UE3 Y: [30000, 34000] cm → [300, 340].
    assert!(
        inset(300.0, lo[0]) && inset(340.0, hi[0]),
        "BW x = {lo:?}..{hi:?} — expected ≈300..340 (from UE3 Y)"
    );
    // BW y comes from UE3 Z: floor 5000 cm → 50, ramp top 5300 → 53.
    assert!(
        inset(50.0, lo[1]) && inset(53.0, hi[1]),
        "BW y = {lo:?}..{hi:?} — expected ≈50..53 (from UE3 Z)"
    );
    // BW z comes from UE3 X: [100000, 107500] cm → [1000, 1075].
    assert!(
        inset(1000.0, lo[2]) && inset(1075.0, hi[2]),
        "BW z = {lo:?}..{hi:?} — expected ≈1000..1075 (from UE3 X)"
    );

    // Asymmetry check: the L's stub only exists at low UE3 X, i.e. low BW z.
    // A probe in the stub must land on the mesh; the mirrored position at
    // high BW z must not.
    let in_stub = graph.locate([333.0, 50.0, 1010.0]).expect("mesh non-empty");
    assert_eq!(
        in_stub.horizontal_distance, 0.0,
        "BW (333, ~50, 1010) is inside the L's stub but resolved {} m away — \
         the footprint is mirrored along z",
        in_stub.horizontal_distance
    );
    let mirrored = graph.locate([333.0, 50.0, 1050.0]).expect("mesh non-empty");
    assert!(
        mirrored.horizontal_distance > 5.0,
        "BW (333, ~50, 1050) should be off-mesh (the stub does not extend that far \
         along z) but resolved {} m away",
        mirrored.horizontal_distance
    );

    // The ramp rises with BW z: sample the surface at both ends.
    let low = graph.locate([310.0, 50.0, 1005.0]).unwrap();
    let high = graph.locate([310.0, 53.0, 1070.0]).unwrap();
    assert_eq!(low.horizontal_distance, 0.0);
    assert_eq!(high.horizontal_distance, 0.0);
    assert!(
        high.closest[1] - low.closest[1] > 1.5,
        "ramp should climb ≈3 m from BW z 1005→1070; got {:.2} → {:.2}",
        low.closest[1],
        high.closest[1]
    );

    // The chunk id contributes bounds padding but NOT a vertex translation:
    // `MapChunk::exportVertices` passes offsetX = offsetZ = 0
    // (chunk.cpp:57). If it translated, BW x would be 300 + 300.
    assert!(
        hi[0] < 400.0,
        "chunk id must not offset vertices; BW x reached {}",
        hi[0]
    );
}

/// Raw-UE3 column order on disk (what `obj.rs` emitted before the swap
/// landed) puts UE3's up-axis on BW x. NavBuilder then finds no
/// upward-facing triangle and produces nothing usable — the exact
/// silent failure the swizzle test above exists to prevent regressing
/// into.
///
/// Two acceptable outcomes, and `run_navbuilder` has already rejected
/// every other exit code: an empty-but-structurally-valid `.nav` (the
/// legacy behaviour), or `EXIT_BUILD_FAILED` with nothing written (what
/// the exit-code table in `build_params.hpp` introduced). What must
/// never happen is a mesh with polygons in it.
#[test]
fn raw_ue3_column_order_yields_no_usable_navmesh() {
    let exe = navbuilder_path();
    if !exe.exists() {
        eprintln!(
            "SKIPPED raw_ue3_column_order_yields_no_usable_navmesh — NavBuilder not found at {}",
            exe.display()
        );
        return;
    }
    let Some(nav) = build(&exe, "raw", cancels_writer_swap, true) else {
        // Reported as a build failure. Nothing more to assert — the
        // absence of a `.nav` is the whole finding.
        return;
    };
    assert_eq!(
        nav.npolys, 0,
        "raw UE3 column order should rasterise the floor as a vertical wall"
    );
    // The up-axis ends up on BW x, pinned at UE3 Z / 100 = 50.
    assert!(
        (nav.bmax[0] - nav.bmin[0]) < 101.0,
        "with the raw ordering BW x collapses onto the chunk-padding span"
    );
}

/// Numeric parameters are range-checked *before* they are narrowed.
///
/// Every case here reached a cast whose result the target type cannot
/// represent, which is undefined behaviour, or was silently truncated
/// into something the operator did not ask for. The observable
/// contract is exit 1 (usage) with the parameter named on stderr —
/// never exit 2 (internal error), and never a navmesh built from a
/// mangled value.
///
/// **Opt-in.** Unlike the axis tests, this one asserts the behaviour of
/// C++ *in this working tree*, so a binary that merely exists proves
/// nothing: the shipped 2026-03 reference predates `BuildParams`
/// entirely, and any NavBuilder built before these checks landed
/// silently truncates `maxVertsPerPoly=3.9` to 3 and exits 0. There is
/// no capability probe that distinguishes them without being one of
/// the assertions, so the operator states it:
///
/// ```bash
/// tools/build-navbuilder.ps1 -Out $env:TEMP/NavBuilder.exe
/// CIMMERIA_NAVBUILDER=$env:TEMP/NavBuilder.exe \
/// CIMMERIA_NAVBUILDER_FROM_TREE=1 cargo test -p cimmeria-navmesh-extractor
/// ```
#[test]
fn out_of_range_parameters_are_refused_with_a_usage_error() {
    let exe = navbuilder_path();
    if !exe.exists() {
        eprintln!(
            "SKIPPED out_of_range_parameters_are_refused_with_a_usage_error — \
             NavBuilder not found at {}",
            exe.display()
        );
        return;
    }
    if std::env::var_os("CIMMERIA_NAVBUILDER_FROM_TREE").is_none() {
        eprintln!(
            "SKIPPED out_of_range_parameters_are_refused_with_a_usage_error — {} was not \
             declared to be built from this tree. Rebuild with tools/build-navbuilder.ps1 \
             and set CIMMERIA_NAVBUILDER_FROM_TREE=1 to run it.",
            exe.display()
        );
        return;
    }
    let dir = unique_tempdir("cimmeria-navparams");
    let chunk_dir = dir.join("chunks");
    std::fs::create_dir_all(&chunk_dir).unwrap();
    write_obj(
        &chunk_dir.join(format!("{CHUNK_ID:08x}o.obj")),
        &fixture_soup(raw_ue3),
        true,
    );
    let out = dir.join("out.nav");

    let run = |param: &str| -> i32 {
        let _ = std::fs::remove_file(&out);
        Command::new(&exe)
            .arg("chunked")
            .arg(&chunk_dir)
            .arg(&out)
            .arg("nav")
            .arg(param)
            .status()
            .expect("spawn NavBuilder")
            .code()
            .expect("no signal on Windows")
    };

    // Sanity check on the claim the env var makes: the default value
    // of a tunable parameter must be accepted and build as if it were
    // not passed at all.
    assert_eq!(
        run("maxVertsPerPoly=6"),
        0,
        "{} does not accept tunable build parameters, so it cannot have been built \
         from this tree",
        exe.display()
    );

    for param in [
        // Finite as a double, outside the float range: the old
        // `(float)strtod(...)` narrowed before checking.
        "cs=1e39",
        "cs=nan",
        "agentHeight=inf",
        // `(int)` of a float this large is undefined, and the cast ran
        // before `validate()` had a chance to reject it.
        "maxVertsPerPoly=1e10",
        // Truncated to 3 and accepted, silently.
        "maxVertsPerPoly=3.9",
        // In range on its own; sends the derived `walkableRadius`
        // cell count past INT_MAX.
        "cs=1e-30",
        // Squared into the region area: overflows above ~46341.
        "minRegionSize=1e9",
    ] {
        assert_eq!(run(param), 1, "{param} must be a usage error");
        assert!(!out.exists(), "{param} must not produce a navmesh");
    }

    // The control: a legitimate tweak of the same parameters still
    // builds, so the checks above are not just refusing everything.
    assert_eq!(run("maxVertsPerPoly=3"), 0, "a valid parameter must build");
    let _ = std::fs::remove_dir_all(&dir);
}

/// LF-terminated OBJ silently loses faces. Same geometry, same convention
/// — only the line ending differs, and the polygon count changes.
#[test]
fn lf_line_endings_drop_faces() {
    let exe = navbuilder_path();
    if !exe.exists() {
        eprintln!(
            "SKIPPED lf_line_endings_drop_faces — NavBuilder not found at {}",
            exe.display()
        );
        return;
    }
    let crlf = build(&exe, "crlf", raw_ue3, true).expect("crlf build");
    let lf = build(&exe, "lf", raw_ue3, false).expect("lf build");
    assert!(crlf.npolys > 0 && lf.npolys > 0);
    assert!(
        lf.npolys < crlf.npolys,
        "expected LF to lose geometry (mesh.cpp:115 needs a trailing char on \
         every `f` line); crlf npolys={} lf npolys={}",
        crlf.npolys,
        lf.npolys
    );
}

/// The cell counts Recast receives at DEFAULT parameters are part of the
/// byte-parity contract with `NavBuilder_d.exe`, the binary that built
/// every shipped 2013 mesh.
///
/// They are derived from float divisions, and the width of that
/// arithmetic is observable: `12.0f / 0.3f` rounds to exactly `40.0f`,
/// while the same division in `double` is `39.9999984` and truncates to
/// 39. Widening it (as a well-meant overflow hardening once did) changes
/// `maxEdgeLen`, and with it every navmesh built at defaults — silently,
/// since both meshes are valid. NavBuilder logs the derived values so
/// this can be pinned without diffing `.nav` files.
///
/// Needs a NavBuilder built from this tree (the log line is new); gated
/// the same way as the parameter-validation test above.
#[test]
fn default_parameters_derive_the_same_cell_counts_as_the_reference_builder() {
    let exe = navbuilder_path();
    if !exe.exists() || std::env::var_os("CIMMERIA_NAVBUILDER_FROM_TREE").is_none() {
        eprintln!(
            "SKIPPED default_parameters_derive_the_same_cell_counts_as_the_reference_builder —              needs a NavBuilder built from this tree: rebuild with tools/build-navbuilder.ps1              and set CIMMERIA_NAVBUILDER_FROM_TREE=1."
        );
        return;
    }
    let dir = unique_tempdir("cimmeria-navderived");
    let chunk_dir = dir.join("chunks");
    std::fs::create_dir_all(&chunk_dir).unwrap();
    write_obj(
        &chunk_dir.join(format!("{CHUNK_ID:08x}o.obj")),
        &fixture_soup(raw_ue3),
        true,
    );
    let output = Command::new(&exe)
        .arg("chunked")
        .arg(&chunk_dir)
        .arg(dir.join("out.nav"))
        .arg("nav")
        .output()
        .expect("spawn NavBuilder");
    let log = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let derived = log
        .lines()
        .find(|l| l.contains("Derived cells:"))
        .unwrap_or_else(|| {
            panic!(
                "NavBuilder logged no `Derived cells:` line:
{log}"
            )
        });
    for expected in [
        "walkableHeight=3",
        "walkableClimb=4",
        "walkableRadius=2",
        "maxEdgeLen=40",
        "minRegionArea=64",
        "mergeRegionArea=400",
    ] {
        assert!(
            derived.contains(expected),
            "expected `{expected}` at default parameters; got: {derived}"
        );
    }
}
