//! CLI for the append-only package patcher.
//!
//! Usage:
//!   upk-patch roundtrip <in> <out>
//!   upk-patch audit-names <package> [--from FIRST_EXPORT_INDEX] [--strict]
//!   (clone-objects also audits its new objects, and fails on a property name
//!   the client would read as None; --strict also fails on an object whose
//!   property list the audit could not follow)
//!   upk-patch clone-objects <target_in> <source> <out> --roots A,B,C
//!             [--map SRC:DST,...] [--first-at X,Y,Z ... | --offset DX,DY,DZ | --anchor SRC:DST]
//!             [--yaw-degrees 0|90|180|270 with --first-at]
//!             [--strip-lightmaps]
//!
//! `roundtrip` rewrites a package uncompressed with no content change.
//! `clone-objects` copies each root (a 0-based export index in <source>) and
//! everything outered to it into <target_in>. `--map` redirects refs to a source
//! export onto an existing target export instead of cloning it. `--anchor` places
//! the clones relative to target actor DST as they sit relative to source actor
//! SRC, yaw included. Coordinates are UE units. <source> may be <target_in>.
//! `--first-at` may be repeated: each point gets its own copy of the whole root
//! set in one session, so the tables are written once and each copy's root
//! sequence takes its own instance number (`_Seq`, `_Seq_0`, `_Seq_1`, ...).
//! Both refuse to overwrite <in>, and both re-open the output to verify it.

use cimmeria_upk::patcher::name_audit::audit_client_names;
use cimmeria_upk::patcher::{
    clone_objects_with_options, CloneReport, CloneRequest, PatchSession, Placement,
};
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
        "Usage:\n  upk-patch roundtrip <in> <out>\n  upk-patch retain-level-actors <in> <out> --classes WorldInfo,Brush\n  upk-patch audit-names <package> [--from N]\n  upk-patch clone-objects <target_in> <source> <out> --roots A,B,C [--map SRC:DST,...] [--first-at X,Y,Z ... | --offset DX,DY,DZ | --anchor SRC:DST] [--yaw-degrees 0|90|180|270] [--strip-lightmaps]"
    );
    process::exit(1);
}

fn flag<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    flags(args, name).into_iter().next()
}

/// Every value of a repeatable flag, in command-line order.
fn flags<'a>(args: &'a [String], name: &str) -> Vec<&'a str> {
    args.windows(2)
        .filter(|w| w[0] == name)
        .map(|w| w[1].as_str())
        .collect()
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
    // The client's name-table loader stores an entry that does not load on the
    // client as `None`; a property tag named by one ends its list early and the
    // rest of the object is misread. A structural parse cannot see that, so
    // audit the names of the new objects.
    let new_exports = before.exports.len()..after.exports.len();
    let audit = audit_client_names(&after, new_exports)
        .unwrap_or_else(|e| fail(&format!("name audit: {e}")));
    for b in audit.unloadable.iter().take(10) {
        eprintln!(
            "  export {} names '{}' (name table entry {}), which the client reads as None",
            b.export, b.name, b.name_index
        );
    }
    if !audit.unloadable.is_empty() {
        fail(&format!(
            "{} property name(s) in new objects would read as None on the client",
            audit.unloadable.len()
        ));
    }
    if !audit.not_audited.is_empty() {
        fail(&format!(
            "{} new object(s) have a property list the name audit cannot follow (exports {:?})",
            audit.not_audited.len(),
            &audit.not_audited[..audit.not_audited.len().min(10)]
        ));
    }
    println!(
        "verify: every property name in the {} new client-loaded objects loads on the client",
        audit.audited
    );
}

/// One placement's clone summary.
fn print_report(report: &CloneReport) {
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
    if let Some((from, to)) = report.level_actor_count {
        println!("level actors {from} -> {to}");
    }
    for (parent, seq) in &report.sequences_attached {
        println!("sequence ref {seq} attached to SequenceObjects of export {parent}");
    }
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
        Some("audit-names") if args.len() >= 3 => {
            // Property names, in client-loaded exports FROM.., that the client
            // reads as None. An unmodified package audits clean.
            let package = Package::open(&args[2]).unwrap_or_else(|e| fail(&e.to_string()));
            let from: usize = flag(&args, "--from")
                .map_or(0, |v| v.parse().unwrap_or_else(|_| fail("bad --from")));
            let audit = audit_client_names(&package, from..package.exports.len())
                .unwrap_or_else(|e| fail(&e.to_string()));
            for b in &audit.unloadable {
                println!(
                    "export {} names '{}' (name table entry {})",
                    b.export, b.name, b.name_index
                );
            }
            println!(
                "{} unloadable property name(s); {} client-loaded export(s) audited, {} not audited",
                audit.unloadable.len(),
                audit.audited,
                audit.not_audited.len()
            );
            if !audit.unloadable.is_empty() {
                process::exit(2);
            }
            if !audit.not_audited.is_empty() && args.iter().any(|a| a == "--strict") {
                eprintln!("not audited: exports {:?}", audit.not_audited);
                process::exit(3);
            }
        }
        Some("retain-level-actors") if args.len() >= 5 => {
            let (input, output) = (&args[2], &args[3]);
            refuse_in_place(input, output);
            let classes: Vec<&str> = flag(&args, "--classes")
                .unwrap_or_else(|| usage())
                .split(',')
                .filter(|name| !name.is_empty())
                .collect();
            if classes.is_empty() {
                fail("at least one retained actor class is required");
            }
            let mut session = PatchSession::open(input).unwrap_or_else(|e| fail(&e.to_string()));
            let level_index = session
                .level_export_index()
                .unwrap_or_else(|e| fail(&e.to_string()));
            let (before, after) = session
                .retain_level_actors(&classes)
                .unwrap_or_else(|e| fail(&e.to_string()));
            let bytes = session.finish().unwrap_or_else(|e| fail(&e.to_string()));
            std::fs::write(output, bytes).unwrap_or_else(|e| fail(&e.to_string()));
            println!(
                "level actor refs: {before} -> {after}; retained classes: {}",
                classes.join(",")
            );
            verify(input, output, &[level_index]);
        }
        Some("clone-objects") if args.len() >= 5 => {
            let (input, source, output) = (&args[2], &args[3], &args[4]);
            refuse_in_place(input, output);
            let roots = parse_indices(flag(&args, "--roots").unwrap_or_else(|| usage()));
            let mapped = parse_pairs(flag(&args, "--map").unwrap_or(""));
            let first_at = flags(&args, "--first-at");
            let yaw = flag(&args, "--yaw-degrees").map(|raw| match raw {
                "0" => 0,
                "90" => 16384,
                "180" => 32768,
                "270" => 49152,
                _ => fail("--yaw-degrees must be 0, 90, 180 or 270"),
            });
            let placements: Vec<Placement> = match (
                first_at.is_empty(),
                flag(&args, "--offset"),
                flag(&args, "--anchor"),
            ) {
                (false, None, None) => first_at
                    .iter()
                    .map(|p| match yaw {
                        Some(yaw) => Placement::FirstActorAtYaw {
                            position: parse_vec3(p),
                            yaw,
                        },
                        None => Placement::FirstActorAt(parse_vec3(p)),
                    })
                    .collect(),
                (true, Some(d), None) => vec![Placement::Offset(parse_vec3(d))],
                (true, None, Some(a)) => match parse_pairs(a)[..] {
                    [(source, target)] => vec![Placement::Anchor { source, target }],
                    _ => fail("--anchor takes one SRC:DST pair"),
                },
                (true, None, None) => vec![Placement::Offset([0.0; 3])],
                _ => usage(),
            };
            if yaw.is_some() && first_at.is_empty() {
                fail("--yaw-degrees requires --first-at");
            }

            let src = PatchSession::open(source).unwrap_or_else(|e| fail(&e.to_string()));
            let mut dst = PatchSession::open(input).unwrap_or_else(|e| fail(&e.to_string()));
            // One session for every copy: the tables are written once, and each
            // copy's root sequence takes the next free instance number.
            let mut changed = Vec::new();
            for placement in placements {
                let request = CloneRequest {
                    roots: &roots,
                    mapped: &mapped,
                    placement,
                };
                let report = clone_objects_with_options(
                    &mut dst,
                    &src,
                    &request,
                    args.iter().any(|arg| arg == "--strip-lightmaps"),
                )
                .unwrap_or_else(|e| fail(&e.to_string()));
                print_report(&report);
                if report.level_actor_count.is_some() {
                    changed.push(
                        dst.level_export_index()
                            .unwrap_or_else(|e| fail(&e.to_string())),
                    );
                }
                changed.extend(report.sequences_attached.iter().map(|(parent, _)| *parent));
            }
            changed.sort_unstable();
            changed.dedup();
            let bytes = dst.finish().unwrap_or_else(|e| fail(&e.to_string()));
            std::fs::write(output, &bytes).unwrap_or_else(|e| fail(&e.to_string()));
            println!("wrote {output} ({} bytes, uncompressed)", bytes.len());
            verify(input, output, &changed);
        }
        _ => usage(),
    }
}
