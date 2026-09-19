use super::*;

fn argv(s: &[&str]) -> Vec<String> {
    s.iter().map(|x| x.to_string()).collect()
}

/// Minimal `XrcNav` with `n` one-metre quads, each its own island, laid out
/// along x at `spacing` metres. `cs = ch = 1`, `bmin = 0`, so grid units are
/// world metres and the quad at index `i` covers `x ∈ [i*spacing, +1]`,
/// `z ∈ [0, 1]` at `y = height`.
fn islands(n: u16, spacing: u16, height: u16) -> XrcNav {
    let mut verts: Vec<u16> = Vec::new();
    let mut polys: Vec<u16> = Vec::new();
    for i in 0..n {
        let x = i * spacing;
        for (dx, dz) in [(0, 0), (1, 0), (1, 1), (0, 1)] {
            verts.extend_from_slice(&[x + dx, height, dz]);
        }
        let b = i * 4;
        polys.extend_from_slice(&[b, b + 1, b + 2, b + 3, 0xffff, 0xffff, 0xffff, 0xffff]);
    }
    XrcNav {
        agent_height: 1.8,
        agent_climb: 0.6,
        agent_radius: 0.6,
        nverts: u32::from(n) * 4,
        npolys: u32::from(n),
        nvp: 4,
        border_size: 0,
        cs: 1.0,
        ch: 1.0,
        bmin: [0.0, 0.0, 0.0],
        bmax: [1000.0, 100.0, 1000.0],
        verts,
        polys,
        regs: vec![0; n as usize],
        flags: vec![1; n as usize],
        areas: vec![63; n as usize],
        detail_nmeshes: 0,
        detail_nverts: 0,
        detail_ntris: 0,
        detail_meshes: vec![],
        detail_verts: vec![],
        detail_tris: vec![],
    }
}

fn run_to_string(nav: &XrcNav, args: &Args) -> (u8, String, Vec<String>) {
    let mut out: Vec<u8> = Vec::new();
    let mut diag = Vec::new();
    let code = run(&mut out, nav, args, &mut diag).expect("writing to a Vec cannot fail");
    (code, String::from_utf8(out).unwrap(), diag)
}

#[test]
fn defaults_are_the_documented_ones() {
    let a = parse_args_from(&argv(&["mesh.nav"])).unwrap();
    assert_eq!(a.path, PathBuf::from("mesh.nav"));
    assert_eq!(a.h_tol, 2.0);
    assert_eq!(a.v_tol, 3.0);
    assert_eq!(a.gap_h, 3.0);
    assert_eq!(a.gap_v, 3.0);
    assert_eq!(a.gap_count, 5);
    assert!(!a.gaps);
    assert!(!a.quiet);
    assert!(a.max_components.is_none());
}

#[test]
fn flags_parse_into_the_fields_they_name() {
    let a = parse_args_from(&argv(&[
        "m.nav",
        "--probe",
        "cell=1,2,3",
        "--h-tol",
        "0.5",
        "--v-tol",
        "0.25",
        "--max-components",
        "7",
        "--gaps",
        "--gap-pair",
        "4, 9",
        "--gap-h",
        "6",
        "--gap-v",
        "8",
        "--gap-count",
        "2",
        "--quiet",
    ]))
    .unwrap();
    assert_eq!(a.probes.len(), 1);
    assert_eq!(a.probes[0].name, "cell");
    assert_eq!(a.probes[0].pos, [1.0, 2.0, 3.0]);
    assert_eq!(a.h_tol, 0.5);
    assert_eq!(a.v_tol, 0.25);
    assert_eq!(a.max_components, Some(7));
    assert!(a.gaps);
    assert_eq!(a.gap_pairs, vec![(4, 9)]);
    assert_eq!(a.gap_h, 6.0);
    assert_eq!(a.gap_v, 8.0);
    assert_eq!(a.gap_count, 2);
    assert!(a.quiet);
}

#[test]
fn bad_arguments_are_rejected_rather_than_defaulted() {
    assert!(parse_args_from(&argv(&[])).is_err(), "no file");
    assert!(
        parse_args_from(&argv(&["a.nav", "b.nav"])).is_err(),
        "two files"
    );
    assert!(parse_args_from(&argv(&["a.nav", "--nope"])).is_err());
    assert!(
        parse_args_from(&argv(&["a.nav", "--probe"])).is_err(),
        "no value"
    );
    assert!(parse_args_from(&argv(&["a.nav", "--probe", "noequals"])).is_err());
    assert!(parse_args_from(&argv(&["a.nav", "--probe", "p=1,2"])).is_err());
    assert!(parse_args_from(&argv(&["a.nav", "--probe", "p=1,2,x"])).is_err());
    assert!(parse_args_from(&argv(&["a.nav", "--h-tol", "wide"])).is_err());
    assert!(parse_args_from(&argv(&["a.nav", "--gap-pair", "4"])).is_err());
    assert!(parse_args_from(&argv(&["a.nav", "--gap-pair", "4,x"])).is_err());
    assert!(
        parse_args_from(&argv(&["a.nav", "--help"])).is_err(),
        "usage"
    );
}

/// `f32::from_str` accepts `inf` and `NaN`, and both are catastrophic
/// here rather than merely wrong: `--gap-h inf` puts every boundary
/// edge in the mesh into one grid cell, so every pair passes the
/// horizontal threshold and the gap graph goes quadratic in the edge
/// count. A `NaN` probe compares unordered against every polygon and
/// reports "NO POLYGON" about a perfectly good mesh.
#[test]
fn non_finite_and_negative_flag_values_are_refused() {
    let bad = |args: &[&str]| {
        let mut v = vec!["m.nav"];
        v.extend_from_slice(args);
        parse_args_from(&argv(&v)).map(|_| ())
    };
    for flag in ["--h-tol", "--v-tol", "--gap-h", "--gap-v"] {
        assert!(bad(&[flag, "inf"]).is_err(), "{flag} inf");
        assert!(bad(&[flag, "-inf"]).is_err(), "{flag} -inf");
        assert!(bad(&[flag, "NaN"]).is_err(), "{flag} NaN");
        assert!(bad(&[flag, "-1"]).is_err(), "{flag} -1");
        assert!(bad(&[flag, "0"]).is_ok(), "{flag} 0 is legitimate");
    }
    // Zero approaches per pair was silently normalised to one.
    assert!(bad(&["--gap-count", "0"]).is_err());
    assert!(bad(&["--gap-count", "-1"]).is_err());
    assert!(bad(&["--gap-count", "1"]).is_ok());
    // Probe coordinates go through the same gate, from both sources.
    assert!(bad(&["--probe", "p=1,inf,3"]).is_err());
    assert!(bad(&["--probe", "p=NaN,2,3"]).is_err());
}

#[test]
fn probe_file_accepts_named_bare_and_commented_lines() {
    let dir = std::env::temp_dir().join(format!("cimmeria-navinspect-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let f = dir.join("probes.txt");
    std::fs::write(
        &f,
        "# a comment\n\ncell 1 2 3\n4 5 6\nlast 7 8 9  # trailing\n",
    )
    .unwrap();
    let p = parse_probe_file(&f).unwrap();
    assert_eq!(p.len(), 3);
    assert_eq!(p[0].name, "cell");
    assert_eq!(p[0].pos, [1.0, 2.0, 3.0]);
    assert_eq!(p[1].name, "probe1", "bare lines get a positional name");
    assert_eq!(p[1].pos, [4.0, 5.0, 6.0]);
    assert_eq!(p[2].pos, [7.0, 8.0, 9.0]);

    std::fs::write(&f, "cell 1 2\n").unwrap();
    assert!(parse_probe_file(&f).is_err(), "3 or 4 fields only");
    std::fs::write(&f, "cell 1 2 three\n").unwrap();
    assert!(parse_probe_file(&f).is_err());
    std::fs::write(&f, "cell 1 inf 3\n").unwrap();
    let e = parse_probe_file(&f).expect_err("inf is not a coordinate");
    assert!(e.contains(":1:"), "the error must name the line: {e}");
    assert!(parse_probe_file(&dir.join("missing.txt")).is_err());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn all_probes_in_one_component_exits_zero() {
    let nav = islands(1, 10, 5);
    let args = parse_args_from(&argv(&["m.nav", "--probe", "a=0.5,5,0.5", "--quiet"])).unwrap();
    let (code, text, diag) = run_to_string(&nav, &args);
    assert_eq!(code, 0, "{text}");
    assert!(diag.is_empty());
    assert!(text.contains("components  1"), "{text}");
    assert!(text.contains("a                    poly=0"), "{text}");
    assert!(text.contains("ok"), "{text}");
}

#[test]
fn a_probe_out_of_tolerance_exits_two() {
    let nav = islands(1, 10, 5);
    // 40 m away in x, far outside the 2 m default horizontal tolerance.
    let args = parse_args_from(&argv(&["m.nav", "--probe", "far=40,5,0.5", "--quiet"])).unwrap();
    let (code, text, diag) = run_to_string(&nav, &args);
    assert_eq!(code, EXIT_PROBE_OUT_OF_TOLERANCE, "{text}");
    assert!(text.contains("OUT OF TOLERANCE"), "{text}");
    assert_eq!(diag.len(), 1);
    assert!(diag[0].contains("out of tolerance"), "{:?}", diag);
}

#[test]
fn probes_in_different_components_exit_three() {
    let nav = islands(2, 50, 5);
    let args = parse_args_from(&argv(&[
        "m.nav",
        "--probe",
        "a=0.5,5,0.5",
        "--probe",
        "b=50.5,5,0.5",
        "--quiet",
    ]))
    .unwrap();
    let (code, text, diag) = run_to_string(&nav, &args);
    assert_eq!(code, EXIT_PROBES_DISCONNECTED, "{text}");
    assert_eq!(diag.len(), 1);
    assert!(diag[0].contains("span 2 components"), "{:?}", diag);
}

#[test]
fn max_components_is_checked_only_after_the_probe_gate() {
    let nav = islands(3, 50, 5);
    let ok = parse_args_from(&argv(&["m.nav", "--max-components", "3", "--quiet"])).unwrap();
    assert_eq!(run_to_string(&nav, &ok).0, 0);
    let too_many = parse_args_from(&argv(&["m.nav", "--max-components", "2", "--quiet"])).unwrap();
    let (code, _, diag) = run_to_string(&nav, &too_many);
    assert_eq!(code, EXIT_TOO_MANY_COMPONENTS);
    assert!(diag[0].contains("exceeds --max-components 2"), "{:?}", diag);
}

/// `--gaps` must name the gap between the components the probes landed in,
/// and the chain must list the intermediate island rather than one long jump.
#[test]
fn gaps_reports_the_chain_between_the_probe_components() {
    // Three islands 2 m apart end to end (quad is 1 m wide, spacing 2 ⇒ a
    // 1 m gap between neighbours, 3 m from the first to the last).
    let nav = islands(3, 2, 5);
    let args = parse_args_from(&argv(&[
        "m.nav",
        "--probe",
        "a=0.5,5,0.5",
        "--probe",
        "c=4.5,5,0.5",
        "--quiet",
        "--gaps",
        "--gap-h",
        "1.5",
    ]))
    .unwrap();
    let (code, text, _) = run_to_string(&nav, &args);
    assert_eq!(code, EXIT_PROBES_DISCONNECTED, "{text}");
    assert!(text.contains("gaps (search h<=1.50 m"), "{text}");
    assert!(text.contains("component 0 "), "{text}");
    assert!(
        text.contains("chain: 2 hop(s), widest 1.00 m, total 2.00 m"),
        "the two 1 m hops, not one 3 m jump:\n{text}"
    );
    assert!(
        text.contains("direct: nothing within the search radius"),
        "{text}"
    );
}

/// An explicit `--gap-pair` naming a component that does not exist must say
/// so instead of indexing out of range.
#[test]
fn gap_pair_with_an_unknown_component_is_reported_not_panicked() {
    let nav = islands(2, 50, 5);
    let args = parse_args_from(&argv(&["m.nav", "--quiet", "--gap-pair", "0,99"])).unwrap();
    let (code, text, _) = run_to_string(&nav, &args);
    assert_eq!(code, 0, "gap reporting does not change the exit code");
    assert!(text.contains("no such component (mesh has 2)"), "{text}");
}

/// The component table is the default and `--quiet` removes it.
#[test]
fn quiet_suppresses_the_component_table() {
    let nav = islands(2, 50, 5);
    let loud = parse_args_from(&argv(&["m.nav"])).unwrap();
    let (_, text, _) = run_to_string(&nav, &loud);
    assert!(text.contains("id   polys      area_m2"), "{text}");
    let quiet = parse_args_from(&argv(&["m.nav", "--quiet"])).unwrap();
    let (_, text, _) = run_to_string(&nav, &quiet);
    assert!(!text.contains("id   polys      area_m2"), "{text}");
}
