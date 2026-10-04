//! The scanner's own tests, on fixture snippets. Each pins one rule so that
//! reverting the rule's code fails the test that names it.

use std::collections::{BTreeMap, BTreeSet};

use super::pairing::{name_key_for, Verdict};
use super::{
    baseline_total, bless_check, calls, compare, lexer, pairing, parse_baseline, render_baseline,
    scan_source, Scan,
};

/// `(key, verdict)` for every ID-shaped field of every event call in `src`.
fn judged(src: &str) -> Vec<(String, Verdict)> {
    let mut masked = lexer::mask(src);
    lexer::blank_test_items(&mut masked.code);
    calls::event_calls(src, &masked)
        .iter()
        .flat_map(|c| pairing::judge(c, &BTreeSet::new(), &masked.line_comments))
        .map(|(_, key, v)| (key, v))
        .collect()
}

fn keys(src: &str) -> Vec<Vec<String>> {
    let masked = lexer::mask(src);
    calls::event_calls(src, &masked)
        .into_iter()
        .map(|c| c.fields.into_iter().map(|f| f.key).collect())
        .collect()
}

fn v(key: &str, verdict: Verdict) -> (String, Verdict) {
    (key.to_string(), verdict)
}

#[test]
fn paired_call_passes() {
    let src = r#"tracing::info!(ability_id = id, ability_name = ?name, "cast");"#;
    assert_eq!(judged(src), [v("ability_id", Verdict::Paired)]);
}

#[test]
fn unpaired_call_is_counted_with_its_line() {
    let src = "fn f() {\n    warn!(\n        witness_id = 1,\n        npc_name = %n,\n        \"gone\"\n    );\n}\n";
    let mut scan = Scan::default();
    scan_source("crates/x/src/a.rs", src, &mut scan);
    let sites = &scan.unpaired["crates/x/src/a.rs"];
    assert_eq!(sites.len(), 1);
    assert_eq!(
        (sites[0].at.as_str(), sites[0].key.as_str()),
        ("crates/x/src/a.rs:3", "witness_id")
    );
}

#[test]
fn marker_with_reason_exempts_its_line_only() {
    let src = r#"info!(
        session_id = %sid, // nt:id-only generated UUID, nothing to name
        conn_id = c,
        "minted"
    );"#;
    assert_eq!(
        judged(src),
        [
            v("session_id", Verdict::Exempt),
            v("conn_id", Verdict::Unpaired)
        ]
    );
}

#[test]
fn marker_without_reason_fails_outright() {
    let src = "info!(\n    session_id = %sid, // nt:id-only   \n    \"minted\"\n);";
    assert_eq!(judged(src), [v("session_id", Verdict::MarkerWithoutReason)]);
    let mut scan = Scan::default();
    scan_source("crates/x/src/a.rs", src, &mut scan);
    assert_eq!(
        scan.broken.len(),
        1,
        "a reasonless marker is never baselined"
    );
    assert!(scan.unpaired.is_empty());
}

/// The exceptions table's name key is accepted, and the default rule's is
/// not: `space_id` needs `world`, never `space_name` or `world_name`.
#[test]
fn exception_table_keys_pair_with_their_table_name() {
    let src = r#"
        debug!(space_id = s, world = %w, "a");
        debug!(space_id = s, space_name = %w, world_name = %w, "b");
        debug!(opcode = op, msg_name = n, "c");
        debug!(method_index = m, method_name = n, "d");
        debug!(item_type_id = t, item_name = n, "e");
        debug!(dest_space_id = s, dest_world = %w, "f");
    "#;
    assert_eq!(
        judged(src),
        [
            v("space_id", Verdict::Paired),
            v("space_id", Verdict::Unpaired),
            v("opcode", Verdict::Paired),
            v("method_index", Verdict::Paired),
            v("item_type_id", Verdict::Paired),
            v("dest_space_id", Verdict::Paired),
        ]
    );
}

#[test]
fn span_constructors_are_not_scanned() {
    let src = r#"
        let s = tracing::info_span!("trade.execute", entity_id = e);
        let t = debug_span!("x", witness_id = w);
        #[tracing::instrument(fields(player_id, entity_id))]
        fn f() {}
    "#;
    assert!(judged(src).is_empty());
    assert!(keys(src).is_empty());
}

#[test]
fn every_field_form_is_read() {
    let src = r#"
        tracing::event!(target: "x", tracing::Level::WARN, a_id = %a, b_id = ?b, c_id, %d_id, ?e_id, f.g_id = 1, r#type = 2, "m {}", h_id);
        error!("only a message {x}", x = entity_id);
    "#;
    assert_eq!(
        keys(src),
        [
            vec!["a_id", "b_id", "c_id", "d_id", "e_id", "f.g_id", "type"],
            vec![],
        ]
    );
}

#[test]
fn dotted_keys_pair_under_the_same_path() {
    assert_eq!(
        name_key_for("npc.template_id"),
        Some(Some("npc.template_name".into()))
    );
    let src = r#"info!(npc.template_id = t, npc.template_name = n, other.template_id = u, "m");"#;
    assert_eq!(
        judged(src),
        [
            v("npc.template_id", Verdict::Paired),
            v("other.template_id", Verdict::Unpaired)
        ]
    );
}

#[test]
fn bare_entity_keys_and_generic_keys() {
    let src = r#"
        info!(target = %t, attacker = a, attacker_name = n, "hit");
        info!(type_id = t, type_name = n, design_id = d, "spawn");
    "#;
    assert_eq!(
        judged(src),
        [
            v("target", Verdict::Unpaired),
            v("attacker", Verdict::Paired),
            v("type_id", Verdict::Unpaired),
            v("design_id", Verdict::Unpaired),
        ]
    );
    // `target:` is the macro's directive, not a field; plain words aren't IDs.
    assert!(judged(r#"info!(target: "t", reason = r, id = 1, ids = v, "m");"#).is_empty());
}

#[test]
fn strings_and_comments_hide_lookalike_calls() {
    let src = r##"
        // info!(entity_id = e, "commented out");
        /* warn!(witness_id = w) */
        let s = "error!(item_id = i)";
        let r = r#"debug!(player_id = p, ")"#;
        let c = ')';
        info!(msg = "a ) , b_id = x", mission_id = m, "real");
    "##;
    assert_eq!(judged(src), [v("mission_id", Verdict::Unpaired)]);
}

#[test]
fn ratchet_fails_on_rise_new_file_fall_and_stale_line() {
    let scan_with = |files: &[(&str, usize)]| {
        let mut scan = Scan::default();
        for (path, n) in files {
            for _ in 0..*n {
                scan_source(path, "info!(entity_id = e, \"m\");", &mut scan);
            }
        }
        scan
    };
    let baseline = parse_baseline("# header\ncrates/a.rs 2\ncrates/b.rs 1\n\ncrates/gone.rs 4\n");

    let (rose, fell) = compare(
        &scan_with(&[
            ("crates/a.rs", 2),
            ("crates/b.rs", 1),
            ("crates/gone.rs", 4),
        ]),
        &baseline,
    );
    assert!(
        rose.is_empty() && fell.is_empty(),
        "equal counts pass: {rose:?} {fell:?}"
    );

    let (rose, fell) = compare(
        &scan_with(&[
            ("crates/a.rs", 3),
            ("crates/b.rs", 1),
            ("crates/new.rs", 1),
            ("crates/gone.rs", 4),
        ]),
        &baseline,
    );
    assert_eq!(rose.len(), 2, "a.rs rose and new.rs appeared: {rose:?}");
    assert!(rose[0].starts_with("crates/a.rs: 3") && rose[0].contains("crates/a.rs:1  entity_id"));
    assert!(rose[1].starts_with("crates/new.rs: 1 unpaired ID fields, baseline allows 0"));
    assert!(fell.is_empty());

    let (rose, fell) = compare(
        &scan_with(&[("crates/a.rs", 1), ("crates/b.rs", 1)]),
        &baseline,
    );
    assert!(rose.is_empty());
    assert_eq!(
        fell,
        [
            "crates/a.rs: 1 now, baseline says 2",
            "crates/gone.rs: 0 now, baseline says 4"
        ]
    );
}

#[test]
fn baseline_round_trips() {
    let counts = BTreeMap::from([
        ("crates/a.rs".to_string(), 3),
        ("crates/b c.rs".to_string(), 1),
    ]);
    let text = render_baseline(&counts);
    assert_eq!(parse_baseline(&text), counts);
    assert_eq!(baseline_total(&text), Some(4), "the total line is written");
}

/// Unpaired sites in `src` as `(line, key)`, through the full per-file path
/// (masking, test-item blanking, parsing, judging).
fn sites(src: &str) -> Vec<(usize, String)> {
    let mut scan = Scan::default();
    scan_source("crates/x/src/a.rs", src, &mut scan);
    scan.unpaired
        .remove("crates/x/src/a.rs")
        .unwrap_or_default()
        .into_iter()
        .map(|s| (s.at.rsplit(':').next().unwrap().parse().unwrap(), s.key))
        .collect()
}

#[test]
fn every_macro_delimiter_and_spacing_is_scanned() {
    let src = "info! { a_id = a }\ninfo![b_id = b]\ninfo! (c_id = c)\ntracing::warn ! ( d_id = d )\nerror! /* note */ (e_id = e)\n";
    assert_eq!(
        keys(src),
        [
            vec!["a_id"],
            vec!["b_id"],
            vec!["c_id"],
            vec!["d_id"],
            vec!["e_id"]
        ]
    );
    // An identifier that merely ends in a macro name is not one.
    assert!(keys("my_info!(a_id = 1); fn info() {}").is_empty());
}

#[test]
fn raw_string_keys_parse_and_raw_messages_stop_fields() {
    assert_eq!(
        keys(r###"info!(r#"player_id"# = p, account_id = a, "used");"###),
        [vec!["player_id", "account_id"]]
    );
    assert_eq!(
        keys(r###"info!(x_id = 1, r#"msg {}"#, y_id); warn!(r"m {}", z_id);"###),
        [vec!["x_id"], vec![]]
    );
}

#[test]
fn braced_field_sets_are_read_with_their_lines() {
    let src = "tracing::event!(\n    tracing::Level::INFO,\n    {\n        player_id = p,\n        npc_id = n,\n        npc_name = nn,\n    },\n    \"m\"\n);\n";
    assert_eq!(sites(src), [(4, "player_id".to_string())]);
}

/// Production code after a `#[cfg(test)] mod tests;` declaration or a test
/// module is still scanned, and keeps its line numbers.
#[test]
fn only_cfg_test_items_are_skipped() {
    let src = "\
fn a() { info!(a_id = 1); }
#[cfg(test)]
mod tests;
fn b() { info!(b_id = 1); }
#[cfg(test)]
#[allow(dead_code)]
mod t {
    fn f() { info!(c_id = 1); }
}
#[cfg(test)]
fn helper() { warn!(d_id = 1); }
fn e() { info!(e_id = 1); }
";
    assert_eq!(
        sites(src),
        [
            (1, "a_id".to_string()),
            (4, "b_id".to_string()),
            (12, "e_id".to_string())
        ]
    );
}

#[test]
fn prose_mentioning_the_marker_does_not_exempt() {
    let src = "info!(\n    entity_id = id, // do not use nt:id-only here\n    player_id = p, // nt:id-onlyish reason\n    \"m\"\n);";
    assert_eq!(
        judged(src),
        [
            v("entity_id", Verdict::Unpaired),
            v("player_id", Verdict::Unpaired)
        ]
    );
}

fn broken(src: &str) -> Vec<String> {
    let mut scan = Scan::default();
    scan_source("crates/x/src/a.rs", src, &mut scan);
    scan.broken
}

/// A marker exempts exactly one field: a marked line holding two ID fields,
/// in one call or two, fails outright instead of exempting both.
#[test]
fn marker_on_a_line_with_two_id_fields_fails() {
    let one_call = "info!(entity_id = e, player_id = p, \"m\"); // nt:id-only slot counter only\n";
    let two_calls =
        "info!(session_id = s, \"a\"); warn!(player_id = p, \"b\"); // nt:id-only generated UUID\n";
    for src in [one_call, two_calls] {
        let b = broken(src);
        assert_eq!(b.len(), 2, "{src}: {b:?}");
        assert!(b[0].contains("holds 2 ID fields"), "{b:?}");
    }
    assert!(
        broken("info!(\n    session_id = s, // nt:id-only generated UUID\n    \"a\"\n);")
            .is_empty()
    );
}

#[test]
fn marker_reason_needs_two_words_or_ten_characters() {
    let verdict = |reason: &str| {
        let src = format!("info!(\n    session_id = s, // nt:id-only {reason}\n    \"m\"\n);");
        judged(&src).remove(0).1
    };
    assert_eq!(verdict("x"), Verdict::MarkerWithoutReason);
    assert_eq!(verdict("TODO"), Verdict::MarkerWithoutReason);
    assert_eq!(verdict("generated UUID"), Verdict::Exempt);
    assert_eq!(verdict("correlation"), Verdict::Exempt);
}

/// `#[cfg(test)]` on a struct field or a match arm blanks only that field or
/// arm, never the production code after it.
#[test]
fn cfg_test_on_a_field_or_arm_stops_at_its_end() {
    let field = "\
struct S {
    #[cfg(test)]
    hook: u8,
}
fn prod() { info!(entity_id = e, \"x\"); }
";
    assert_eq!(sites(field), [(5, "entity_id".to_string())]);
    let last_field =
        "struct S {\n    #[cfg(test)]\n    hook: u8\n}\nfn prod() { info!(a_id = 1); }\n";
    assert_eq!(sites(last_field), [(5, "a_id".to_string())]);
    let arm = "\
fn f(n: u8) {
    match n {
        #[cfg(test)]
        9 => (),
        _ => info!(entity_id = e, \"next arm\"),
    }
    warn!(player_id = p, \"after\");
}
";
    assert_eq!(
        sites(arm),
        [(5, "entity_id".to_string()), (7, "player_id".to_string())]
    );
}

#[test]
fn test_only_cfg_predicates_are_skipped_and_not_test_is_not() {
    let src = "\
#[cfg(any(test, feature = \"test-support\"))]
fn helper() { info!(a_id = 1); }
#[cfg(all(test, unix))]
fn other() { info!(b_id = 1); }
#[cfg(not(test))]
fn prod() { info!(c_id = 1); }
";
    assert_eq!(sites(src), [(6, "c_id".to_string())]);
}

#[test]
fn renamed_event_macro_imports_fail() {
    for src in [
        "use tracing::warn as twarn;\nfn f() { twarn!(entity_id = e, \"x\"); }\n",
        "use tracing::{debug, info as i};\n",
    ] {
        let b = broken(src);
        assert_eq!(b.len(), 1, "{src}: {b:?}");
        assert!(b[0].contains("hides its calls"), "{b:?}");
    }
    assert!(broken("use tracing::{info, warn};\nuse std::io::Error as E;\n").is_empty());
}

/// A wrapper's call sites in the same file are judged as events, paired
/// against its fixed fields; a wrapper with none there fails unless marked.
#[test]
fn forwarding_wrappers_are_expanded_or_fail() {
    let local = "\
macro_rules! row {
    ($($extra:tt)*) => {
        tracing::debug!(entity_id = e, entity_name = n, $($extra)*)
    };
}
fn f() {
    row!(ability_id = a, \"x\");
    row!(item_id = i, item_name = m, witness_id = w, \"y\");
}
";
    assert_eq!(
        sites(local),
        [(7, "ability_id".to_string()), (8, "witness_id".to_string())]
    );
    assert!(broken(local).is_empty());
    let wrapper = "macro_rules! cell_warn {\n    ($($t:tt)*) => { tracing::warn!($($t)*) };\n}\n";
    let b = broken(wrapper);
    assert_eq!(b.len(), 1, "{b:?}");
    assert!(
        b[0].starts_with("crates/x/src/a.rs:2") && b[0].contains("forwarding"),
        "{b:?}"
    );
    let marked = "macro_rules! w {\n    ($($t:tt)*) => { tracing::warn!(\n        $($t)* // nt:id-only callers pass paired fields\n    ) };\n}\n";
    assert!(broken(marked).is_empty());
    // `$` in a value, or a single metavariable (an `event!` level), is not
    // forwarding.
    assert!(broken("macro_rules! m { ($e:expr) => { info!(a_name = $e, \"m\") }; }").is_empty());
    assert!(broken("macro_rules! m { ($l:expr) => { tracing::event!($l, \"m\") }; }").is_empty());
}

/// A bless may move counts between files (a rename or a split) but never
/// raise the total, unless there is no baseline yet.
#[test]
fn bless_accepts_moves_and_refuses_a_total_rise() {
    let scan_with = |files: &[(&str, usize)]| {
        let mut scan = Scan::default();
        for (path, n) in files {
            for _ in 0..*n {
                scan_source(path, "info!(entity_id = e, \"m\");", &mut scan);
            }
        }
        scan
    };
    let baseline = parse_baseline("crates/big.rs 5\n");
    assert!(bless_check(
        &scan_with(&[("crates/a.rs", 3), ("crates/b.rs", 2)]),
        &baseline
    )
    .is_ok());
    assert!(bless_check(&scan_with(&[("crates/a.rs", 4)]), &baseline).is_ok());
    let err = bless_check(
        &scan_with(&[("crates/a.rs", 3), ("crates/b.rs", 3)]),
        &baseline,
    )
    .unwrap_err();
    assert!(err.contains("rose from 5 to 6"), "{err}");
    assert!(bless_check(&scan_with(&[("crates/a.rs", 9)]), &BTreeMap::new()).is_ok());
}
