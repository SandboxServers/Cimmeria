//! NT-03: Rule 6's ratchet. Every ID field on a log event is paired with its
//! name, and the files that still have unpaired ones may only get better.
//!
//! The scan reads the source of every crate in [`IN_PROCESS_CRATES`] (test
//! code skipped, as in `target_scan_tests`), finds each `trace!`, `debug!`,
//! `info!`, `warn!`, `error!` and `event!` call, and judges every ID-shaped
//! field against Rule 6 in `docs/architecture/instrumentation-discipline.md`:
//! paired, exempted by `// nt:id-only <reason>` on its line, or unpaired.
//! `pairing` has the key rules, `calls` the macro parser, `lexer` the masking
//! that keeps strings and comments out of both.
//!
//! `unpaired_id_baseline.txt` holds the unpaired count per file. The guard
//! fails when a file's count rises or a new file appears (it prints the
//! file's unpaired fields), and when a count falls but the baseline still
//! has the old number. Lower it with
//!
//! ```text
//! NT_BASELINE_BLESS=1 bash tools/build-lane/lane.sh cargo nextest run -p cimmeria-server unpaired_id
//! ```
//!
//! which rewrites the file and refuses while any count is above its
//! baseline. `NT_BASELINE_BLESS=force` writes the counts as they are; a
//! reviewer rejects a baseline that grows.

mod calls;
mod fixture_tests;
mod lexer;
mod pairing;

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::PathBuf;

use pairing::Verdict;

use super::target_scan_tests::{
    crates_dir, is_test_path, rs_files, strip_test_module, IN_PROCESS_CRATES,
};

const BASELINE: &str = "unpaired_id_baseline.txt";
const BLESS_VAR: &str = "NT_BASELINE_BLESS";

/// One unpaired ID field: `crates/…/file.rs:line`, and its key.
#[derive(Debug, Clone)]
struct Site {
    at: String,
    key: String,
}

#[derive(Default)]
struct Scan {
    /// Repo-relative path → its unpaired fields. Only files with at least one.
    unpaired: BTreeMap<String, Vec<Site>>,
    /// Markers with no reason. Never baselined.
    reasonless: Vec<Site>,
    calls: usize,
    id_fields: usize,
}

/// Judges every event call in one source text, adding to `scan`.
fn scan_source(rel: &str, src: &str, scan: &mut Scan) {
    let masked = lexer::mask(src);
    for call in calls::event_calls(src, &masked) {
        scan.calls += 1;
        for (line, key, verdict) in pairing::judge(&call, &masked.line_comments) {
            scan.id_fields += 1;
            let site = Site {
                at: format!("{rel}:{line}"),
                key,
            };
            match verdict {
                Verdict::Paired | Verdict::Exempt => {}
                Verdict::Unpaired => scan.unpaired.entry(rel.to_string()).or_default().push(site),
                Verdict::MarkerWithoutReason => scan.reasonless.push(site),
            }
        }
    }
}

fn scan_workspace() -> Scan {
    let root = crates_dir();
    let mut scan = Scan::default();
    for krate in IN_PROCESS_CRATES {
        let mut files = Vec::new();
        rs_files(&root.join(krate).join("src"), &mut files);
        files.sort();
        for f in files {
            let rel = f.strip_prefix(&root).unwrap();
            if is_test_path(rel) {
                continue;
            }
            let rel = format!("crates/{}", rel.to_string_lossy().replace('\\', "/"));
            let src = std::fs::read_to_string(&f).unwrap();
            scan_source(&rel, strip_test_module(&src), &mut scan);
        }
    }
    scan
}

fn baseline_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src/logging")
        .join(BASELINE)
}

/// `path count` lines; `#` lines and blank lines are comments.
fn parse_baseline(text: &str) -> BTreeMap<String, usize> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| {
            let (path, n) = l
                .rsplit_once(' ')
                .unwrap_or_else(|| panic!("{BASELINE}: `{l}` is not `path count`"));
            let n = n
                .parse()
                .unwrap_or_else(|_| panic!("{BASELINE}: `{l}` has no count"));
            (path.trim().to_string(), n)
        })
        .collect()
}

fn render_baseline(counts: &BTreeMap<String, usize>) -> String {
    let mut out = String::from(
        "# Unpaired ID fields per file (Rule 6, NT-03). Only ever lowered.\n\
         # Regenerate after pairing fields: NT_BASELINE_BLESS=1 cargo nextest run -p cimmeria-server unpaired_id\n\
         # See crates/server/src/logging/unpaired_id_tests/mod.rs.\n",
    );
    for (path, n) in counts {
        let _ = writeln!(out, "{path} {n}");
    }
    out
}

/// The ratchet's verdict on `current` against `baseline`: what rose (with its
/// sites) and what fell without the baseline following.
fn compare(scan: &Scan, baseline: &BTreeMap<String, usize>) -> (Vec<String>, Vec<String>) {
    let mut rose = Vec::new();
    let mut fell = Vec::new();
    for (path, sites) in &scan.unpaired {
        let allowed = baseline.get(path).copied().unwrap_or(0);
        if sites.len() > allowed {
            let mut msg = format!(
                "{path}: {} unpaired ID fields, baseline allows {allowed}. Pair each with its name \
                 key or mark its line `// nt:id-only <reason>`:",
                sites.len()
            );
            for s in sites {
                let _ = write!(msg, "\n    {}  {}", s.at, s.key);
            }
            rose.push(msg);
        } else if sites.len() < allowed {
            fell.push(format!(
                "{path}: {} now, baseline says {allowed}",
                sites.len()
            ));
        }
    }
    for (path, n) in baseline {
        if !scan.unpaired.contains_key(path) {
            fell.push(format!("{path}: 0 now, baseline says {n}"));
        }
    }
    (rose, fell)
}

/// **The guard.** No reasonless marker anywhere; no file's unpaired count
/// above its baseline; no stale baseline line.
#[test]
fn unpaired_id_fields_only_shrink() {
    let scan = scan_workspace();
    let reasonless: Vec<String> = scan
        .reasonless
        .iter()
        .map(|s| format!("    {}  {}", s.at, s.key))
        .collect();
    assert!(
        reasonless.is_empty(),
        "`// {}` needs a reason after it (Rule 6):\n{}",
        pairing::MARKER,
        reasonless.join("\n")
    );

    let path = baseline_path();
    let baseline = parse_baseline(&std::fs::read_to_string(&path).unwrap_or_default());
    let (rose, fell) = compare(&scan, &baseline);
    let counts: BTreeMap<String, usize> = scan
        .unpaired
        .iter()
        .map(|(p, s)| (p.clone(), s.len()))
        .collect();
    match std::env::var(BLESS_VAR).as_deref() {
        Ok("force") => {
            std::fs::write(&path, render_baseline(&counts)).unwrap();
            return;
        }
        Ok("1") if rose.is_empty() => {
            std::fs::write(&path, render_baseline(&counts)).unwrap();
            return;
        }
        _ => {}
    }
    assert!(
        rose.is_empty(),
        "unpaired ID fields above the baseline in {BASELINE}:\n{}",
        rose.join("\n")
    );
    assert!(
        fell.is_empty(),
        "fewer unpaired ID fields than {BASELINE} says. Lower it with \
         `{BLESS_VAR}=1 cargo nextest run -p cimmeria-server unpaired_id` and commit it:\n{}",
        fell.join("\n")
    );
}

/// The scan has to see the code it guards, or the guard passes on nothing.
#[test]
fn scan_sees_the_workspace() {
    let scan = scan_workspace();
    assert!(scan.calls > 2000, "only {} event calls found", scan.calls);
    assert!(
        scan.id_fields > 1500,
        "only {} ID fields found",
        scan.id_fields
    );
}

/// Totals for the campaign ledger and the sweeps: run with
/// `cargo nextest run -p cimmeria-server unpaired_id_report --run-ignored only --no-capture`.
#[test]
#[ignore = "report, not a check"]
fn unpaired_id_report() {
    let scan = scan_workspace();
    let total: usize = scan.unpaired.values().map(Vec::len).sum();
    println!(
        "calls={} id_fields={} unpaired={total} files={}",
        scan.calls,
        scan.id_fields,
        scan.unpaired.len()
    );
    let top = |counts: BTreeMap<String, usize>, n: usize| {
        let mut v: Vec<_> = counts.into_iter().collect();
        v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        v.truncate(n);
        v.iter()
            .map(|(k, c)| format!("  {c:5} {k}"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let mut keys = BTreeMap::new();
    let mut crates = BTreeMap::new();
    for (path, sites) in &scan.unpaired {
        let krate = path.split('/').nth(1).unwrap_or("").to_string();
        *crates.entry(krate.clone()).or_insert(0) += sites.len();
        for s in sites {
            *keys.entry(s.key.clone()).or_insert(0) += 1;
        }
    }
    println!("top keys:\n{}", top(keys, 25));
    println!("by crate:\n{}", top(crates.clone(), 40));
    let files = scan
        .unpaired
        .iter()
        .map(|(p, s)| (p.clone(), s.len()))
        .collect();
    println!("top files:\n{}", top(files, 25));
    for krate in crates.keys() {
        let mut ks = BTreeMap::new();
        let mut fs = BTreeMap::new();
        for (path, sites) in scan
            .unpaired
            .iter()
            .filter(|(p, _)| p.split('/').nth(1) == Some(krate))
        {
            fs.insert(path.clone(), sites.len());
            for s in sites {
                *ks.entry(s.key.clone()).or_insert(0) += 1;
            }
        }
        println!(
            "== {krate}\nkeys:\n{}\nfiles:\n{}",
            top(ks, 10),
            top(fs, 10)
        );
    }
}
