use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use super::items::items;
use super::lex::lex;
use super::module_tree::LibFn;
use super::*;

/// The fns of one synthetic file, as if it were the module `mod_path`.
fn lib_fns(mod_path: &str, src: &str) -> (Vec<LibFn>, Vec<(PathBuf, usize)>) {
    let found = items(&lex(src));
    let file = PathBuf::from("synthetic.rs");
    let fns = found
        .fns
        .into_iter()
        .map(|item| {
            let mut parts = vec![mod_path.to_string()];
            parts.extend(item.inline_mods.iter().cloned());
            parts.push(item.name.clone());
            LibFn {
                file: file.clone(),
                test_name: parts.join("::"),
                item,
            }
        })
        .collect();
    let stray = found
        .stray_gate_lines
        .into_iter()
        .map(|l| (file.clone(), l))
        .collect();
    (fns, stray)
}

fn names(v: &[Violation]) -> Vec<&str> {
    v.iter().filter_map(|v| v.test_name.as_deref()).collect()
}

/// The guard itself: every lib test in the workspace that reaches the
/// live-DB gate has `live_db` in its nextest name, so the `ci-live-db`
/// profile runs it in the serial `live-db` group.
#[test]
fn every_live_db_test_is_in_the_live_db_group() {
    let (report, _) = check_workspace();
    assert!(
        report.violations.is_empty(),
        "{} live-DB test(s) are outside the `live-db` test group \
         (`test(~{MARKER})` in .config/nextest.toml). They would get no \
         slot database and run against the shared template, in parallel \
         with other tests, colliding on shared rows. \
         Prefix the test fn with `{MARKER}_` or move it into a `{MARKER}` \
         module (TESTING.md, \"Live-DB tests\"):\n{}",
        report.violations.len(),
        report
            .violations
            .iter()
            .map(|v| format!("  {v}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
    // Not vacuous: the scan found the live-DB tests (well over a thousand
    // when this guard landed).
    assert!(
        report.db_tests.len() > 500,
        "the scan found only {} live-DB tests; is the lexer broken?",
        report.db_tests.len()
    );
}

/// The lexer-based scan and a plain line search agree on where every gate
/// call is, and every file with a gate call is part of a lib target, so a
/// lexer blind spot or an unreached module cannot hide a live-DB test.
#[test]
fn every_gate_call_is_attributed() {
    let (_, scans) = check_workspace();
    let mut attributed: BTreeMap<PathBuf, BTreeSet<usize>> = BTreeMap::new();
    let mut reached = BTreeSet::new();
    for scan in &scans {
        reached.extend(scan.files.iter().cloned());
        for f in &scan.fns {
            attributed
                .entry(f.file.clone())
                .or_default()
                .extend(f.item.gate_lines.iter().copied());
        }
        for (file, line) in &scan.stray_gates {
            attributed.entry(file.clone()).or_default().insert(*line);
        }
    }
    let mut problems = Vec::new();
    for src in crate::source_scan::rust_sources() {
        if src.src_rel.is_none() {
            continue; // integration tests: not in the `--lib` tier
        }
        if src
            .crates_rel
            .starts_with("test-support/src/live_db_group/")
        {
            continue; // this guard's own fixtures: gate calls inside strings
        }
        let naive: BTreeSet<usize> = naive_gate_lines(&src.read()).into_iter().collect();
        if naive.is_empty() {
            continue;
        }
        let path = src.path.canonicalize().expect("canonicalize a source file");
        if !reached.contains(&path) {
            problems.push(format!(
                "  crates/{}: calls the gate but is not reachable from its crate's src/lib.rs, \
                 and the live-DB tier runs lib tests only",
                src.crates_rel
            ));
            continue;
        }
        let lexed = attributed.remove(&path).unwrap_or_default();
        if lexed != naive {
            problems.push(format!(
                "  crates/{}: gate calls on lines {naive:?} by text search, {lexed:?} by the lexer",
                src.crates_rel
            ));
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

/// The guard checks names against `live_db`; this pins that the profile
/// really serialises that filter, so the two cannot drift apart.
#[test]
fn nextest_profile_groups_the_live_db_filter() {
    let path = crate::source_scan::crates_dir()
        .parent()
        .expect("workspace root")
        .join(".config/nextest.toml");
    let text = std::fs::read_to_string(&path).expect("read .config/nextest.toml");
    // Compare without whitespace and comments, so formatting can change.
    let flat: String = text
        .lines()
        .map(|l| l.split('#').next().unwrap_or(""))
        .collect::<String>()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    for want in [
        "[test-groups]live-db={max-threads=",
        "[[profile.ci-live-db.overrides]]filter=\"test(~live_db)\"test-group=\"live-db\"",
    ] {
        assert!(flat.contains(want), "{} lacks `{want}`", path.display());
    }
    assert_eq!(crate::LIVE_DB_GROUP, "live-db", "the slot resolver's group");
    // tools/test-live-db.sh and .ps1 read the slot count (how many database
    // clones to make) from this exact line shape.
    let slots = live_db_slots(&text).unwrap_or_else(|| {
        panic!(
            "{} has no `live-db = {{ max-threads = N }}` line the clone scripts can read",
            path.display()
        )
    });
    assert!(slots >= 1, "live-db max-threads must be at least 1");
    // The old serialise-everything override must be gone, or the group
    // buys nothing.
    assert!(
        !flat.contains("filter=\"all()\"threads-required=\"num-test-threads\""),
        "{} still serialises every test",
        path.display()
    );
}

/// The slot count as the clone scripts parse it: the integer in the line
/// `live-db = { max-threads = N }` (the scripts match
/// `^live-db = { max-threads = [0-9]+ }`).
fn live_db_slots(toml: &str) -> Option<u32> {
    toml.lines()
        .filter_map(|l| l.strip_prefix("live-db = { max-threads = "))
        .find_map(|rest| rest.trim_end().strip_suffix(" }")?.parse().ok())
}

#[test]
fn the_slot_count_parses_like_the_scripts_do() {
    assert_eq!(
        live_db_slots("[test-groups]\nlive-db = { max-threads = 8 }\n"),
        Some(8)
    );
    assert_eq!(live_db_slots("live-db = { max-threads = 8 }\r\n"), Some(8));
    assert_eq!(
        live_db_slots("live-db = { max-threads = \"num-cpus\" }"),
        None
    );
    assert_eq!(live_db_slots("# live-db = { max-threads = 8 }"), None);
}

/// The failure this guard exists for: a live-DB test with no `live_db`
/// in its name. Renaming `live_db_buys` back to `buys` must be caught.
#[test]
fn a_gate_calling_test_without_the_marker_is_a_violation() {
    let src = r#"
        #[tokio::test]
        async fn buys() {
            let pool = require_db_or_skip!();
        }
        #[tokio::test]
        async fn live_db_sells() {
            let pool = require_db_or_skip!();
        }
        #[test]
        fn parses() { assert_eq!(1, 1); }
    "#;
    let (fns, stray) = lib_fns("vendor::tests", src);
    let r = check(&fns, &stray);
    assert_eq!(names(&r.violations), ["vendor::tests::buys"]);
    assert_eq!(r.violations[0].line, 3);
    assert_eq!(
        r.db_tests,
        ["vendor::tests::buys", "vendor::tests::live_db_sells"]
    );
}

/// A `live_db` module covers every test in it, inline or out of line.
#[test]
fn a_live_db_module_covers_its_tests() {
    let src = r#"
        mod live_db_loaders {
            #[tokio::test]
            async fn loads() { let p = require_db_or_skip!(); }
        }
    "#;
    let (fns, stray) = lib_fns("spawner::tests", src);
    assert!(check(&fns, &stray).violations.is_empty());
    let (fns, stray) = lib_fns(
        "character::delete_live_db_tests",
        &src.replace("live_db_loaders", "m"),
    );
    let r = check(&fns, &stray);
    assert!(r.violations.is_empty(), "{:?}", r.violations);
    assert_eq!(r.db_tests, ["character::delete_live_db_tests::m::loads"]);
}

/// A test that reaches the gate through a helper (and a helper's helper)
/// is a live-DB test too. A test that only takes a pool argument-free
/// path is not.
#[test]
fn tests_reaching_the_gate_through_helpers_are_live_db_tests() {
    let src = r#"
        async fn assert_dialog_resolves(id: u32) {
            let pool = require_db_or_skip!();
        }
        async fn both() { assert_dialog_resolves(1).await; }
        #[tokio::test]
        async fn resolves_1112() { both().await; }
        #[tokio::test]
        async fn live_db_resolves_1113() { assert_dialog_resolves(2).await; }
        #[test]
        fn unrelated() {}
    "#;
    let (fns, stray) = lib_fns("chain_replay_tests::m638", src);
    let r = check(&fns, &stray);
    assert_eq!(
        names(&r.violations),
        ["chain_replay_tests::m638::resolves_1112"]
    );
    assert!(
        r.violations[0].why.contains("`both`"),
        "{}",
        r.violations[0].why
    );
    assert_eq!(r.db_tests.len(), 2);
}

/// Comments, doc examples, strings and the macro's own definition are not
/// gate calls; a call the guard cannot attribute to a fn is a violation.
#[test]
fn only_real_gate_calls_count() {
    let src = r##"
        //! let pool = require_db_or_skip!();
        /// ```
        /// let pool = require_db_or_skip!();
        /// ```
        #[test]
        fn mentions() {
            let s = "require_db_or_skip!()";
            let r = r#"require_db_or_skip!() " still raw"#;
            /* require_db_or_skip!() /* nested */ */
            let c = '{';
        }
        macro_rules! require_db_or_skip { () => {} }
        macro_rules! with_db { () => { let p = require_db_or_skip!(); } }
    "##;
    let (fns, stray) = lib_fns("m", src);
    let r = check(&fns, &stray);
    assert!(r.db_tests.is_empty(), "{:?}", r.db_tests);
    assert_eq!(r.violations.len(), 1, "{:?}", r.violations);
    assert_eq!(
        (r.violations[0].test_name.as_ref(), r.violations[0].line),
        (None, 14)
    );
}

#[test]
fn test_attributes_are_recognised() {
    let src = r#"
        #[test] fn a() {}
        #[tokio::test(flavor = "multi_thread", worker_threads = 2)] async fn b() {}
        #[rstest] fn c() {}
        #[cfg(test)] fn d() {}
        #[should_panic] fn e() {}
        fn f() {}
    "#;
    let found = items(&lex(src));
    let tests: Vec<&str> = found
        .fns
        .iter()
        .filter(|f| f.is_test)
        .map(|f| f.name.as_str())
        .collect();
    assert_eq!(tests, ["a", "b", "c"]);
}

/// `mod x;` resolution follows rustc: `x.rs` or `x/mod.rs` beside a
/// mod-rs file, `parent/x.rs` beside a non-mod-rs one, `#[path]` relative
/// to the declaring file's directory, and inline blocks as directories.
#[test]
fn the_module_walk_names_tests_like_rustc() {
    let root = std::env::temp_dir().join(format!("live_db_group_walk_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let write = |rel: &str, body: &str| {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, body).unwrap();
    };
    write("lib.rs", "mod a; mod b; mod inl { mod c; }");
    write("a.rs", "mod a1; #[path = \"elsewhere_tests.rs\"] mod a2;");
    write("a/a1.rs", "#[test] fn t1() {}");
    write("elsewhere_tests.rs", "#[test] fn t2() {}");
    write("b/mod.rs", "mod b1;");
    write("b/b1.rs", "#[test] fn t3() {}");
    write("inl/c.rs", "#[test] fn t4() {}");
    let scan = module_tree::scan_lib(&root.join("lib.rs"));
    let mut got: Vec<&str> = scan.fns.iter().map(|f| f.test_name.as_str()).collect();
    got.sort_unstable();
    assert_eq!(got, ["a::a1::t1", "a::a2::t2", "b::b1::t3", "inl::c::t4"]);
    let _ = std::fs::remove_dir_all(&root);
}

/// Every live-DB test process must reach its slot's database. Nothing in a
/// crate's `src/` may resolve the database URL except
/// `test_support::database_url()`: no direct `DATABASE_URL` read, no
/// hard-coded server URL, no `sqlx::test`.
#[test]
fn database_url_is_only_resolved_by_the_gate() {
    let mut problems = Vec::new();
    for src in crate::source_scan::rust_sources() {
        // Integration tests are outside the `--lib` tier; the guard's own
        // fixtures hold violations on purpose.
        if src.src_rel.is_none()
            || src
                .crates_rel
                .starts_with("test-support/src/live_db_group/")
        {
            continue;
        }
        for (line, why) in url_guard::url_violations(&src.crates_rel, &src.read()) {
            problems.push(format!("  crates/{}:{line}  {why}", src.crates_rel));
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

/// The guard trips on each way around the slot, and not on comments or
/// the unreachable pools.
#[test]
fn a_database_url_resolved_around_the_slot_is_a_violation() {
    let src = r#"
        // std::env::var("DATABASE_URL") in a comment is fine
        async fn wide_pool() -> PgPool {
            PgPool::connect(&std::env::var("DATABASE_URL").unwrap()).await.unwrap()
        }
        #[sqlx::test]
        async fn provisioned(pool: PgPool) {}
        fn fixed() -> &'static str { "postgres://w-testing:w-testing@localhost:5433/sgw" }
        fn dead() -> &'static str { "postgres://nobody:nobody@127.0.0.1:1/none" }
        fn slot() -> Option<String> { crate::test_support::database_url() }
    "#;
    let lines: Vec<usize> = url_guard::url_violations("x/src/y.rs", src)
        .into_iter()
        .map(|(l, _)| l)
        .collect();
    assert_eq!(lines, [4, 6, 8]);
    // The resolver itself may name the variable.
    assert!(url_guard::url_violations(
        "test-support/src/live_db_slot.rs",
        r#"std::env::var("DATABASE_URL")"#
    )
    .is_empty());
}
