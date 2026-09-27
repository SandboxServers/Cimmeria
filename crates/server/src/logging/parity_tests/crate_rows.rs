//! Two guards on the module-path rows of the filters, which every per-crate
//! guard of the services crate split rests on
//! (`docs/architecture/services-crate-split.md`):
//!
//! - **Every in-process crate has its own `OTEL_FILTER` row**: its crate
//!   name, or one row per top-level module (`cimmeria_base::base`). A crate
//!   without one exports nothing below the filter's leading `info`, so its
//!   DEBUG rows would never reach SigNoz and nothing would fail. Each wave of
//!   the split closed that gap for its new crate by hand; this makes the
//!   next crate close it too, or say in [`NO_OWN_ROW`] why it does not.
//! - **No row reaches into two crates.** A directive matches by string
//!   prefix, so a row naming `cimmeria_cell` would also match every
//!   `cimmeria_cell_*` crate. Removing one of those crates' own rows would
//!   then change nothing, and the guard that pins it could never fail again.
//!   That is why the base and cell crates' rows name `cimmeria_base::base`
//!   and `cimmeria_cell::cell`, and wire's rows name its modules.
//!
//! Both check the filters through `EnvFilter` itself, with probe events, not
//! a model of its matching rule.

use std::collections::BTreeMap;
use std::sync::Arc;

use tracing::{Dispatch, Level};
use tracing_subscriber::layer::{Layer, SubscriberExt};
use tracing_subscriber::EnvFilter;

use super::{recorder, sinks_for, Hits, OTLP_SERVER};
use crate::logging::filters::{directive_pairs, is_custom_target, FILE_LAYERS, OTEL_FILTER};
use crate::logging::stale_target_tests::crate_roots;
use crate::logging::target_scan_tests::IN_PROCESS_CRATES;

/// In-process crates with no `OTEL_FILTER` row of their own, each with the
/// reason. Their rows ride the leading `info`: INFO and up reach SigNoz,
/// DEBUG does not. Giving one a row would export its DEBUG rows, which is a
/// decision about that crate, not a repair.
const NO_OWN_ROW: &[(&str, &str)] = &[
    ("admin-api", NEVER_EXPORTED_BELOW_INFO),
    ("commands", NEVER_EXPORTED_BELOW_INFO),
    ("common", NEVER_EXPORTED_BELOW_INFO),
    ("content-engine", NEVER_EXPORTED_BELOW_INFO),
    ("defs", NEVER_EXPORTED_BELOW_INFO),
    ("entity", NEVER_EXPORTED_BELOW_INFO),
    ("game", NEVER_EXPORTED_BELOW_INFO),
    ("lab-mcp", NEVER_EXPORTED_BELOW_INFO),
    ("observability", NEVER_EXPORTED_BELOW_INFO),
    ("occluder", NEVER_EXPORTED_BELOW_INFO),
    (
        "server",
        "the binary itself: its startup and shutdown rows are INFO and up, and \
         its DEBUG rows were never exported",
    ),
];

/// Reason shared by the pre-split crates in [`NO_OWN_ROW`].
const NEVER_EXPORTED_BELOW_INFO: &str = "a crate that predates the services split and \
     was never under `cimmeria_services=debug`: its DEBUG rows have never been exported";

/// The workspace crates by crate name (`cimmeria_cell_world`), each with its
/// directory under `crates/` and its root source file.
fn workspace_crates() -> BTreeMap<String, (String, std::path::PathBuf)> {
    crate_roots()
        .into_iter()
        .map(|(name, root)| {
            let dir = root
                .ancestors()
                .find(|a| {
                    a.parent()
                        .and_then(|p| p.file_name())
                        .is_some_and(|n| n == "crates")
                })
                .and_then(|a| a.file_name())
                .unwrap_or_else(|| panic!("{} is not under crates/", root.display()))
                .to_string_lossy()
                .into_owned();
            (name, (dir, root))
        })
        .collect()
}

/// A dispatch that records every event `directives` enables.
fn probe(directives: &str) -> (Dispatch, Hits) {
    let hits: Hits = Arc::default();
    let dispatch = Dispatch::new(
        tracing_subscriber::registry()
            .with(recorder(OTLP_SERVER.into(), &hits).with_filter(EnvFilter::new(directives))),
    );
    (dispatch, hits)
}

/// `src` with every `//` comment removed.
fn strip_comments(src: &str) -> String {
    src.lines()
        .map(|l| l.split("//").next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Each `mod` declared at the top level of `code` (comments stripped), with
/// its body when it is inline. Modules behind `cfg(test)` or the
/// `test-support` feature are skipped: they are never in the server.
fn top_level_modules(code: &str) -> Vec<(String, Option<String>)> {
    let bytes = code.as_bytes();
    let mut out = Vec::new();
    let (mut depth, mut i) = (0usize, 0usize);
    while i < bytes.len() {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => depth = depth.saturating_sub(1),
            b'm' if depth == 0
                && code[i..].starts_with("mod ")
                && (i == 0 || !(bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'_')) =>
            {
                let rest = &code[i + 4..];
                let name: String = rest
                    .trim_start()
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                    .collect();
                // The attributes sit between the previous item's end and here.
                let attrs = &code[code[..i].rfind([';', '}']).map_or(0, |p| p + 1)..i];
                let gated = attrs.contains("cfg(test)")
                    || attrs.contains("cfg(any(test")
                    || attrs.contains("test-support");
                let after = rest.trim_start()[name.len()..].trim_start();
                let body = after.strip_prefix('{').map(|inner| {
                    let mut d = 1usize;
                    let end = inner
                        .char_indices()
                        .find(|&(_, c)| {
                            match c {
                                '{' => d += 1,
                                '}' => d -= 1,
                                _ => {}
                            }
                            d == 0
                        })
                        .map_or(inner.len(), |(p, _)| p);
                    inner[..end].to_string()
                });
                // Past the inline body's closing brace, or onto the `;`.
                let skip = body.as_ref().map_or(0, |b| b.len() + 2);
                if !gated {
                    out.push((name, body));
                }
                i = code.len() - after.len() + skip;
                continue;
            }
            _ => {}
        }
        i += 1;
    }
    out
}

/// The top-level modules of a crate root that can emit events: file-backed
/// ones, and inline ones that declare a module or a function. An inline
/// module of `use` lines only (the split crates' `mod cell { pub(crate) use
/// …; }` skeletons) re-exports other crates' items, whose events carry their
/// own crate's path.
fn event_modules(root_src: &str) -> Vec<String> {
    top_level_modules(&strip_comments(root_src))
        .into_iter()
        .filter(|(_, body)| {
            body.as_deref()
                .is_none_or(|b| b.contains("fn ") || !top_level_modules(b).is_empty())
        })
        .map(|(name, _)| name)
        .collect()
}

/// Every in-process crate that `filter` gives no row of its own, or whose
/// own rows leave a top-level module uncovered.
fn own_row_violations(filter: &str) -> Vec<String> {
    let crates = workspace_crates();
    let mut out = Vec::new();
    for (name, (dir, root)) in &crates {
        if !IN_PROCESS_CRATES.contains(&dir.as_str()) || NO_OWN_ROW.iter().any(|(d, _)| d == dir) {
            continue;
        }
        let module_prefix = format!("{name}::");
        let own: Vec<&str> = directive_pairs(filter)
            .filter(|(t, level)| {
                !level.eq_ignore_ascii_case("off") && (t == name || t.starts_with(&module_prefix))
            })
            .map(|(t, _)| t)
            .collect();
        if own.is_empty() {
            out.push(format!(
                "{name} (crates/{dir}) has no OTEL_FILTER row of its own; add \
                 `{name}=debug` or list it in NO_OWN_ROW with the reason"
            ));
            continue;
        }
        // Only the crate's own rows, each at `trace`: which of its modules
        // do they reach at all?
        let (dispatch, hits) = probe(&format!(
            "off,{}",
            own.iter()
                .map(|t| format!("{t}=trace"))
                .collect::<Vec<_>>()
                .join(",")
        ));
        let src = std::fs::read_to_string(root).expect("read the crate root");
        let modules = event_modules(&src);
        assert!(
            !modules.is_empty(),
            "{name}: found no top-level module in {}; is the scanner broken?",
            root.display()
        );
        for module in modules {
            let target = format!("{name}::{module}::parity_probe");
            if sinks_for(&dispatch, &hits, &target, Level::TRACE).is_empty() {
                out.push(format!(
                    "{name}::{module} is covered by none of {name}'s own rows {own:?}; \
                     add `{name}::{module}=debug`"
                ));
            }
        }
    }
    out
}

/// Every module-path directive in `sources` that matches more than one
/// workspace crate's events.
fn shared_prefix_violations(sources: &[(&str, &str)]) -> Vec<String> {
    let names: Vec<String> = workspace_crates().into_keys().collect();
    let mut out = Vec::new();
    for (source, directives) in sources {
        for (target, level) in directive_pairs(directives) {
            if is_custom_target(target) || level.eq_ignore_ascii_case("off") {
                continue;
            }
            let (dispatch, hits) = probe(&format!("off,{target}=trace"));
            let reached: Vec<&String> = names
                .iter()
                .filter(|n| {
                    !sinks_for(
                        &dispatch,
                        &hits,
                        &format!("{n}::parity_probe"),
                        Level::TRACE,
                    )
                    .is_empty()
                })
                .collect();
            if reached.len() > 1 {
                out.push(format!(
                    "{source}: `{target}` reaches the events of {} crates {reached:?}; \
                     name a module (`{target}::<module>`) instead",
                    reached.len()
                ));
            }
        }
    }
    out
}

/// Every directive string the guard checks, with a label.
fn all_directive_sources() -> Vec<(&'static str, &'static str)> {
    let mut out = vec![("OTEL_FILTER", OTEL_FILTER)];
    out.extend(FILE_LAYERS.iter().map(|l| (l.file, l.directives)));
    out
}

/// **Guard (a).** Every in-process crate has its own `OTEL_FILTER` row that
/// reaches each of its top-level modules. The second half is the guard's own
/// revert-proof: a crate row, or one module row of a crate that names its
/// modules, removed from the real filter is reported.
#[test]
fn every_in_process_crate_has_its_own_otel_row() {
    let v = own_row_violations(OTEL_FILTER);
    assert!(v.is_empty(), "{}", v.join("\n"));

    for (row, reported) in [
        (
            "cimmeria_minigame=debug,",
            "cimmeria_minigame (crates/minigame)",
        ),
        ("cimmeria_cell::cell=debug,", "cimmeria_cell (crates/cell)"),
        (
            "cimmeria_wire::containers=debug,",
            "cimmeria_wire::containers",
        ),
    ] {
        let without = OTEL_FILTER.replace(row, "");
        assert_ne!(
            without, OTEL_FILTER,
            "OTEL_FILTER no longer carries `{row}`; update this test"
        );
        let v = own_row_violations(&without);
        assert!(
            v.len() == 1 && v[0].starts_with(reported),
            "dropping `{row}` must report exactly {reported}, got {v:#?}"
        );
    }
}

/// Every exemption is still needed: the crate is in-process, and it has no
/// row of its own (or the exemption would hide a real one's removal).
#[test]
fn no_own_row_exemptions_are_in_process_and_rowless() {
    let crates = workspace_crates();
    for (dir, reason) in NO_OWN_ROW {
        assert!(reason.len() > 20, "{dir}: every exemption needs its reason");
        assert!(
            IN_PROCESS_CRATES.contains(dir),
            "{dir} is in NO_OWN_ROW but not in IN_PROCESS_CRATES; remove it"
        );
        let (name, _) = crates
            .iter()
            .find(|(_, (d, _))| d == dir)
            .unwrap_or_else(|| panic!("no crate at crates/{dir}"));
        let prefix = format!("{name}::");
        assert!(
            !directive_pairs(OTEL_FILTER).any(|(t, _)| t == name || t.starts_with(&prefix)),
            "{name} has an OTEL_FILTER row now; remove its NO_OWN_ROW entry"
        );
    }
}

/// **Guard (b).** No module-path row of `OTEL_FILTER` or a file layer reaches
/// the events of two crates. The second half proves the guard can fail: the
/// bare crate names the base and cell rows avoid are each reported.
#[test]
fn no_filter_row_reaches_two_crates() {
    let v = shared_prefix_violations(&all_directive_sources());
    assert!(v.is_empty(), "{}", v.join("\n"));

    for (row, bare) in [
        ("cimmeria_cell::cell=debug", "cimmeria_cell=debug"),
        ("cimmeria_base::base=debug", "cimmeria_base=debug"),
        ("cimmeria_wire::mercury=debug", "cimmeria_wire=debug"),
    ] {
        let widened = OTEL_FILTER.replace(row, bare);
        assert_ne!(
            widened, OTEL_FILTER,
            "OTEL_FILTER no longer carries `{row}`; update this test"
        );
        let v = shared_prefix_violations(&[("OTEL_FILTER", &widened)]);
        assert!(
            v.len() == 1 && v[0].contains(&format!("`{}`", bare.trim_end_matches("=debug"))),
            "`{bare}` in place of `{row}` must be reported, got {v:#?}"
        );
    }
}

/// The module scanner behind guard (a): the shapes the crate roots use.
#[test]
fn top_level_module_scan_reads_the_root_shapes() {
    let src = r#"
        //! pub mod in_a_doc_comment;
        #![warn(unreachable_pub)]
        pub mod base;
        pub(crate) use other::mercury;
        mod cell {
            pub(crate) use other::cell::{a, b};
            #[cfg(test)]
            mod content_tests;
        }
        pub mod ability_tree {
            pub mod points_property;
        }
        pub mod helpers {
            pub fn f() {}
        }
        #[cfg(test)]
        mod test_support {
            pub(crate) use x::*;
        }
        #[cfg(any(test, feature = "test-support"))]
        #[doc(hidden)]
        pub mod test_fixtures;
        mod tail;
    "#;
    assert_eq!(
        event_modules(src),
        ["base", "ability_tree", "helpers", "tail"]
    );
}
