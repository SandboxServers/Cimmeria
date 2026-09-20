//! CLI for the append-only package patcher.
//!
//! Usage:
//!   upk-patch roundtrip <in> <out>
//!   upk-patch clone-actors <target_in> <source> <out> --actors A,B,C
//!             (--first-at X,Y,Z | --offset DX,DY,DZ)
//!
//! `roundtrip` rewrites a package uncompressed with no content change.
//! `clone-actors` copies placed actors (0-based export indices in <source>) and
//! their components into <target_in>. Coordinates are UE units.
//! Both refuse to overwrite <in>, and both re-open the output to verify it.

use cimmeria_upk::patcher::{clone_actors, PatchSession, Placement};
use cimmeria_upk::{extract_actors, Package};
use std::env;
use std::path::Path;
use std::process;

fn fail(msg: &str) -> ! {
    eprintln!("ERROR: {msg}");
    process::exit(1);
}

fn usage() -> ! {
    eprintln!(
        "Usage:\n  upk-patch roundtrip <in> <out>\n  upk-patch clone-actors <target_in> <source> <out> --actors A,B,C (--first-at X,Y,Z | --offset DX,DY,DZ)"
    );
    process::exit(1);
}

fn flag<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.windows(2)
        .find(|w| w[0] == name)
        .map(|w| w[1].as_str())
}

fn parse_vec3(s: &str) -> [f32; 3] {
    let v: Vec<f32> = s
        .split(',')
        .map(|p| p.trim().parse().unwrap_or_else(|_| fail("bad coordinate")))
        .collect();
    match v[..] {
        [x, y, z] => [x, y, z],
        _ => fail("expected three comma-separated numbers"),
    }
}

fn refuse_in_place(input: &str, output: &str) {
    let same = Path::new(input).canonicalize().ok() == Path::new(output).canonicalize().ok()
        && Path::new(output).exists();
    if same {
        fail("output path is the input file; write to a new file and install it deliberately");
    }
}

/// Re-open `output` and check every original export still reads back
/// byte-identical, apart from `changed` (0-based export indices).
fn verify(original: &str, output: &str, changed: &[usize]) {
    let before = Package::open(original).unwrap_or_else(|e| fail(&format!("reopen input: {e}")));
    let after = Package::open(output).unwrap_or_else(|e| fail(&format!("reopen output: {e}")));
    if after.header.is_compressed() {
        fail("output still claims to be compressed");
    }
    for (i, n) in before.names.iter().enumerate() {
        if after.names.get(i).map(|a| &a.name) != Some(&n.name) {
            fail(&format!("name {i} changed"));
        }
    }
    for (i, imp) in before.imports.iter().enumerate() {
        let a = &after.imports[i];
        if a.object_name != imp.object_name || a.package_index != imp.package_index {
            fail(&format!("import {i} changed"));
        }
    }
    let mut identical = 0usize;
    for (i, exp) in before.exports.iter().enumerate() {
        let a = &after.exports[i];
        if a.object_name != exp.object_name || a.class_index != exp.class_index {
            fail(&format!("export {i} entry changed"));
        }
        if changed.contains(&i) {
            continue;
        }
        let old = before.read_export_data(exp).unwrap_or_default();
        let new = after.read_export_data(a).unwrap_or_default();
        if old != new {
            fail(&format!("export {i} data changed"));
        }
        identical += 1;
    }
    println!(
        "verify: {} names (+{}), {} imports (+{}), {} exports (+{}); {identical} original exports byte-identical, {} intentionally changed",
        after.names.len(),
        after.names.len() - before.names.len(),
        after.imports.len(),
        after.imports.len() - before.imports.len(),
        after.exports.len(),
        after.exports.len() - before.exports.len(),
        changed.len()
    );
    println!(
        "verify: actors parsed from output: {} (input had {})",
        extract_actors(&after).len(),
        extract_actors(&before).len()
    );
}

fn main() {
    let args: Vec<String> = env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("roundtrip") if args.len() >= 4 => {
            let (input, output) = (&args[2], &args[3]);
            refuse_in_place(input, output);
            let session = PatchSession::open(input).unwrap_or_else(|e| fail(&e.to_string()));
            let bytes = session.finish().unwrap_or_else(|e| fail(&e.to_string()));
            std::fs::write(output, &bytes).unwrap_or_else(|e| fail(&e.to_string()));
            println!("wrote {output} ({} bytes, uncompressed)", bytes.len());
            verify(input, output, &[]);
        }
        Some("clone-actors") if args.len() >= 5 => {
            let (input, source, output) = (&args[2], &args[3], &args[4]);
            refuse_in_place(input, output);
            let actors: Vec<usize> = flag(&args, "--actors")
                .unwrap_or_else(|| usage())
                .split(',')
                .map(|s| s.trim().parse().unwrap_or_else(|_| fail("bad actor index")))
                .collect();
            let placement = match (flag(&args, "--first-at"), flag(&args, "--offset")) {
                (Some(p), None) => Placement::FirstActorAt(parse_vec3(p)),
                (None, Some(d)) => Placement::Offset(parse_vec3(d)),
                _ => usage(),
            };

            let src = PatchSession::open(source).unwrap_or_else(|e| fail(&e.to_string()));
            let mut dst = PatchSession::open(input).unwrap_or_else(|e| fail(&e.to_string()));
            let level = dst
                .level_export_index()
                .unwrap_or_else(|e| fail(&e.to_string()));
            let report = clone_actors(&mut dst, &src, &actors, placement)
                .unwrap_or_else(|e| fail(&e.to_string()));
            for a in &report.actors {
                println!(
                    "cloned source export {} ({}) -> ref {} at UE ({:.1}, {:.1}, {:.1}) = game ({:.3}, {:.3}, {:.3}), {} component(s)",
                    a.source_index,
                    a.class,
                    a.target_ref,
                    a.location[0],
                    a.location[1],
                    a.location[2],
                    a.location[1] / 100.0,
                    a.location[2] / 100.0,
                    a.location[0] / 100.0,
                    a.components
                );
            }
            println!(
                "added {} name(s), {} import(s), {} export(s); level actors {} -> {}",
                report.names_added,
                report.imports_added,
                report.exports_added,
                report.level_actor_count.0,
                report.level_actor_count.1
            );
            let bytes = dst.finish().unwrap_or_else(|e| fail(&e.to_string()));
            std::fs::write(output, &bytes).unwrap_or_else(|e| fail(&e.to_string()));
            println!("wrote {output} ({} bytes, uncompressed)", bytes.len());
            verify(input, output, &[level]);
        }
        _ => usage(),
    }
}
