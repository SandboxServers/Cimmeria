//! Stale-target guard: every Rust module path a log directive names must
//! exist as a module in the workspace.
//!
//! `tracing-subscriber` matches a directive's target against an event's
//! `module_path!()` by string prefix. When a module moves crate, and the
//! services crate split moves most of them, its events carry a new module
//! path, and a directive naming the old one silently matches nothing: a
//! `logs/<file>` layer or a SigNoz index stops receiving those rows with no
//! error anywhere. This test resolves each module-path directive in
//! [`OTEL_FILTER`], [`FILE_LAYERS`] and the other directive strings against
//! the source tree (crate name to crate directory, then module directories
//! and files) and fails on any that no longer resolves. It checks the
//! module-path literals in `otel::is_network_noise_target` the same way.
//!
//! A directive is a module path when [`is_custom_target`] says it is not a
//! hand-named target: it contains `::` or starts with `cimmeria_`. Such a
//! directive that deliberately names no workspace module is listed in
//! [`NOT_WORKSPACE_MODULES`] with its reason.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::filters::{
    directive_pairs, is_custom_target, server_log_directives, FILE_LAYERS, OTEL_FILTER,
    WIRE_FIREHOSE_MUTED,
};

/// Directive targets shaped like module paths that name no module in this
/// workspace, each with the reason.
const NOT_WORKSPACE_MODULES: &[(&str, &str)] = &[(
    "sqlx::query",
    "sqlx's per-statement log target: a module of the external sqlx crate",
)];

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Library (or binary) crate name -> its root source file, for every package
/// under `crates/`. The name is `[lib] name`, else the package name with `-`
/// turned into `_`; the root is `[lib] path`, else `src/lib.rs`, else
/// `src/main.rs`.
fn crate_roots() -> BTreeMap<String, PathBuf> {
    let crates = workspace_root().join("crates");
    let mut out = BTreeMap::new();
    for entry in std::fs::read_dir(&crates).expect("read crates/").flatten() {
        let dir = entry.path();
        let Ok(manifest) = std::fs::read_to_string(dir.join("Cargo.toml")) else {
            continue;
        };
        let mut section = "";
        let (mut package, mut lib_name, mut lib_path) = (None, None, None);
        for line in manifest.lines().map(str::trim) {
            if line.starts_with('[') {
                section = line;
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let value = value.trim().trim_matches('"').to_string();
            match (section, key.trim()) {
                ("[package]", "name") => package = Some(value),
                ("[lib]", "name") => lib_name = Some(value),
                ("[lib]", "path") => lib_path = Some(value),
                _ => {}
            }
        }
        let Some(package) = package else { continue };
        let name = lib_name.unwrap_or_else(|| package.replace('-', "_"));
        let root = match lib_path {
            Some(p) => dir.join(p),
            None if dir.join("src/lib.rs").is_file() => dir.join("src/lib.rs"),
            None => dir.join("src/main.rs"),
        };
        out.insert(name, root);
    }
    out
}

/// `true` if `text` declares `mod name;` or `mod name { … }` outside a
/// line comment.
fn declares_module(text: &str, name: &str) -> bool {
    text.lines().any(|line| {
        let code = line.split("//").next().unwrap_or("");
        let mut from = 0;
        while let Some(i) = code[from..].find("mod ") {
            let at = from + i;
            from = at + 4;
            if at > 0 {
                let prev = code.as_bytes()[at - 1];
                if prev.is_ascii_alphanumeric() || prev == b'_' {
                    continue;
                }
            }
            let rest = code[from..].trim_start();
            if let Some(after) = rest.strip_prefix(name) {
                let after = after.trim_start();
                if after.starts_with(';') || after.starts_with('{') || after.is_empty() {
                    return true;
                }
            }
        }
        false
    })
}

/// `true` if `target` names a module of a workspace crate. A bare target
/// (no `::`) may also be a prefix of crate names, `cimmeria_cell` for every
/// `cimmeria_cell_*` crate, because that is how the filter matches it.
fn resolves(target: &str, roots: &BTreeMap<String, PathBuf>) -> bool {
    let mut segs = target.split("::");
    let krate = segs.next().unwrap_or("");
    let rest: Vec<&str> = segs.collect();
    let Some(root_file) = roots.get(krate) else {
        let prefix = format!("{krate}_");
        return rest.is_empty() && roots.keys().any(|k| k.starts_with(&prefix));
    };
    let mut file = root_file.clone();
    for (i, seg) in rest.iter().enumerate() {
        let text = std::fs::read_to_string(&file).unwrap_or_default();
        if !declares_module(&text, seg) {
            return false;
        }
        let is_mod_root = file
            .file_name()
            .is_some_and(|n| n == "lib.rs" || n == "main.rs" || n == "mod.rs");
        let dir = if is_mod_root {
            file.parent()
                .expect("source file has a parent")
                .to_path_buf()
        } else {
            file.with_extension("")
        };
        let flat = dir.join(format!("{seg}.rs"));
        let nested = dir.join(seg).join("mod.rs");
        if flat.is_file() {
            file = flat;
        } else if nested.is_file() {
            file = nested;
        } else {
            // An inline `mod seg { … }`: whatever follows is declared in the
            // same file.
            return rest[i + 1..].iter().all(|s| declares_module(&text, s));
        }
    }
    true
}

/// Every directive string the server builds a filter from, with a label.
fn directive_sources() -> Vec<(String, String)> {
    let mut out = vec![
        ("OTEL_FILTER".to_string(), OTEL_FILTER.to_string()),
        ("server.log".to_string(), server_log_directives()),
        (
            "WIRE_FIREHOSE_MUTED".to_string(),
            WIRE_FIREHOSE_MUTED.to_string(),
        ),
    ];
    for layer in FILE_LAYERS {
        out.push((
            format!("FILE_LAYERS {}", layer.file),
            layer.directives.to_string(),
        ));
    }
    out
}

fn is_exempt(target: &str) -> bool {
    NOT_WORKSPACE_MODULES.iter().any(|(t, _)| *t == target)
}

/// **The guard.** Fix a failure by renaming the directive to the module's
/// new path (`cimmeria_<crate>::…`); do not delete the row, or its events
/// stop reaching the file or index.
#[test]
fn every_module_path_directive_names_an_existing_module() {
    let roots = crate_roots();
    let mut checked = 0usize;
    let mut stale = Vec::new();
    for (source, directives) in directive_sources() {
        for (target, _level) in directive_pairs(&directives) {
            if is_custom_target(target) || is_exempt(target) {
                continue;
            }
            checked += 1;
            if !resolves(target, &roots) {
                stale.push(format!("{source}: {target}"));
            }
        }
    }
    assert!(
        checked > 20,
        "only {checked} module-path directives checked; is the classification broken?"
    );
    assert!(
        stale.is_empty(),
        "log directives name modules that do not exist (moved or deleted), so they \
         silently match nothing:\n{}",
        stale.join("\n")
    );
}

/// `otel::is_network_noise_target` routes these module paths to the
/// `cimmeria-network` index. A stale one sends a per-packet stream to the
/// primary index instead.
#[test]
fn network_noise_module_targets_exist() {
    let src = include_str!("../otel.rs");
    let start = src
        .find("pub fn is_network_noise_target")
        .expect("otel.rs defines is_network_noise_target");
    let body = &src[start..start + src[start..].find("\n}").expect("function end")];
    let roots = crate_roots();
    let mut found = 0usize;
    for literal in body.split('"').skip(1).step_by(2) {
        if !literal.starts_with("cimmeria_") {
            continue;
        }
        found += 1;
        let target = literal.trim_end_matches("::");
        assert!(
            resolves(target, &roots),
            "is_network_noise_target names `{literal}`, which is not a module"
        );
    }
    assert!(
        found >= 3,
        "found only {found} module paths in is_network_noise_target"
    );
}

/// An exemption must still be needed: it must be used by a directive and
/// must not resolve, or it could hide a real module path.
#[test]
fn not_workspace_module_exemptions_are_used_and_external() {
    let roots = crate_roots();
    let used: Vec<String> = directive_sources()
        .iter()
        .flat_map(|(_, d)| {
            directive_pairs(d)
                .map(|(t, _)| t.to_string())
                .collect::<Vec<_>>()
        })
        .collect();
    for (target, reason) in NOT_WORKSPACE_MODULES {
        assert!(
            used.iter().any(|t| t == target),
            "exemption `{target}` ({reason}) is no longer used by any directive; remove it"
        );
        assert!(
            !resolves(target, &roots),
            "exemption `{target}` resolves to a workspace module; check it like any other"
        );
    }
}

/// The resolver itself: the shapes the guard must reject and accept.
#[test]
fn resolver_rejects_missing_modules_and_accepts_real_ones() {
    let roots = crate_roots();
    for stale in [
        // The FILE_LAYERS row removed with this guard: world_entry_player.rs
        // was split into world_entry/methods/ long ago.
        "cimmeria_services::base::world_entry_player",
        "cimmeria_services::no_such_module",
        "cimmeria_services::cell::no_such_child",
        "cimmeria_no_such_crate",
        "cimmeria_no_such_crate::x",
    ] {
        assert!(!resolves(stale, &roots), "`{stale}` should not resolve");
    }
    for real in [
        "cimmeria_services",
        "cimmeria_mercury",
        "cimmeria_server::logging",
        "cimmeria_services::cell::space_manager",
        "cimmeria_services::base::world_entry",
        // `pub mod method_idx { … }` is inline in mercury/mod.rs.
        "cimmeria_services::mercury::method_idx",
        // A bare prefix covering cimmeria_client_launch and
        // cimmeria_client_telemetry.
        "cimmeria_client",
    ] {
        assert!(resolves(real, &roots), "`{real}` should resolve");
    }
}
