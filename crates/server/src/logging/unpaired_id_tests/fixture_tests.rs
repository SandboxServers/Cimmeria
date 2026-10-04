//! The scanner's own tests, on fixture snippets. Each pins one rule so that
//! reverting the rule's code fails the test that names it.

use std::collections::BTreeMap;

use super::pairing::{name_key_for, Verdict};
use super::{calls, compare, lexer, pairing, parse_baseline, render_baseline, scan_source, Scan};

/// `(key, verdict)` for every ID-shaped field of every event call in `src`.
fn judged(src: &str) -> Vec<(String, Verdict)> {
    let mut masked = lexer::mask(src);
    lexer::blank_test_items(&mut masked.code);
    calls::event_calls(src, &masked)
        .iter()
        .flat_map(|c| pairing::judge(c, &masked.line_comments))
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
        scan.reasonless.len(),
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
    assert_eq!(parse_baseline(&render_baseline(&counts)), counts);
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
