//! CLI for the append-only package patcher.
//!
//! Usage:
//!   upk-patch roundtrip <in> <out>
//!   upk-patch clone-objects <target_in> <source> <out> --roots A,B,C
//!             [--map SRC:DST,...] [--first-at X,Y,Z | --offset DX,DY,DZ | --anchor SRC:DST]
//!
//! `roundtrip` rewrites a package uncompressed with no content change.
//! `clone-objects` copies each root (a 0-based export index in <source>) and
//! everything outered to it into <target_in>. `--map` redirects refs to a source
//! export onto an existing target export instead of cloning it. `--anchor` places
//! the clones relative to target actor DST as they sit relative to source actor
//! SRC, yaw included. Coordinates are UE units. <source> may be <target_in>.
//! Both refuse to overwrite <in>, and both re-open the output to verify it.

use cimmeria_upk::patcher::{clone_objects, CloneRequest, PatchSession, Placement};
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
        "Usage:\n  upk-patch roundtrip <in> <out>\n  upk-patch clone-objects <target_in> <source> <out> --roots A,B,C [--map SRC:DST,...] [--first-at X,Y,Z | --offset DX,DY,DZ | --anchor SRC:DST]"
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

fn parse_indices(s: &str) -> Vec<usize> {
    s.split(',')
        .filter(|p| !p.trim().is_empty())
        .map(|p| {
            p.trim()
                .parse()
                .unwrap_or_else(|_| fail("bad export index"))
        })
        .collect()
}

fn parse_pairs(s: &str) -> Vec<(usize, usize)> {
    s.split(',')
        .filter(|p| !p.trim().is_empty())
        .map(|p| match parse_indices(&p.replace(':', ","))[..] {
            [a, b] => (a, b),
            _ => fail("expected SRC:DST"),
        })
        .collect()
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
        Some("clone-objects") if args.len() >= 5 => {
            let (input, source, output) = (&args[2], &args[3], &args[4]);
            refuse_in_place(input, output);
            let roots = parse_indices(flag(&args, "--roots").unwrap_or_else(|| usage()));
            let mapped = parse_pairs(flag(&args, "--map").unwrap_or(""));
            let placement = match (
                flag(&args, "--first-at"),
                flag(&args, "--offset"),
                flag(&args, "--anchor"),
            ) {
                (Some(p), None, None) => Placement::FirstActorAt(parse_vec3(p)),
                (None, Some(d), None) => Placement::Offset(parse_vec3(d)),
                (None, None, Some(a)) => match parse_pairs(a)[..] {
                    [(source, target)] => Placement::Anchor { source, target },
                    _ => fail("--anchor takes one SRC:DST pair"),
                },
                (None, None, None) => Placement::Offset([0.0; 3]),
                _ => usage(),
            };

            let src = PatchSession::open(source).unwrap_or_else(|e| fail(&e.to_string()));
            let mut dst = PatchSession::open(input).unwrap_or_else(|e| fail(&e.to_string()));
            let request = CloneRequest {
                roots: &roots,
                mapped: &mapped,
                placement,
            };
            let report =
                clone_objects(&mut dst, &src, &request).unwrap_or_else(|e| fail(&e.to_string()));
            for o in report.objects.iter().filter(|o| o.is_root) {
                match o.location {
                    Some(l) => println!(
                        "cloned source export {} ({}) -> ref {} '{}' at UE ({:.1}, {:.1}, {:.1}) = game ({:.3}, {:.3}, {:.3})",
                        o.source_index,
                        o.class,
                        o.target_ref,
                        o.name,
                        l[0],
                        l[1],
                        l[2],
                        l[1] / 100.0,
                        l[2] / 100.0,
                        l[0] / 100.0
                    ),
                    None => println!(
                        "cloned source export {} ({}) -> ref {} '{}'",
                        o.source_index, o.class, o.target_ref, o.name
                    ),
                }
            }
            println!(
                "added {} name(s), {} import(s), {} export(s) from {} root(s)",
                report.names_added,
                report.imports_added,
                report.exports_added,
                report.objects.iter().filter(|o| o.is_root).count()
            );
            let mut changed = Vec::new();
            if let Some((from, to)) = report.level_actor_count {
                println!("level actors {from} -> {to}");
                changed.push(
                    dst.level_export_index()
                        .unwrap_or_else(|e| fail(&e.to_string())),
                );
            }
            for (parent, seq) in &report.sequences_attached {
                println!("sequence ref {seq} attached to SequenceObjects of export {parent}");
                changed.push(*parent);
            }
            let bytes = dst.finish().unwrap_or_else(|e| fail(&e.to_string()));
            std::fs::write(output, &bytes).unwrap_or_else(|e| fail(&e.to_string()));
            println!("wrote {output} ({} bytes, uncompressed)", bytes.len());
            verify(input, output, &changed);
        }
        _ => usage(),
    }
}
