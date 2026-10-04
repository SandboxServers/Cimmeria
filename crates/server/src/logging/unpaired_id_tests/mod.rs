//! NT-03: Rule 6's ratchet. Every ID field on a log event is paired with its
//! name, and the files that still have unpaired ones may only get better.
//!
//! The scan reads the source of every crate in [`IN_PROCESS_CRATES`] (test
//! files skipped as in `target_scan_tests`, and each `#[cfg(test)]` item
//! blanked in place, so production code after a test module still counts),
//! finds each `trace!`, `debug!`,
//! `info!`, `warn!`, `error!` and `event!` call, and judges every ID-shaped
//! field against Rule 6 in `docs/architecture/instrumentation-discipline.md`:
//! paired, exempted by `// nt:id-only <reason>` on its line, or unpaired.
//! `pairing` has the key rules, `calls` the macro parser, `lexer` the masking
//! that keeps strings and comments out of both.
//!
//! `unpaired_id_baseline.txt` holds the unpaired count per file and a
//! `# total N` line. The guard fails when a file's count rises or a new file
//! appears (it prints the file's unpaired fields), and when a count falls but
//! the baseline still has the old number. Rewrite it with
//!
//! ```text
//! NT_BASELINE_BLESS=1 bash tools/build-lane/lane.sh cargo nextest run -p cimmeria-server unpaired_id
//! ```
//!
//! which accepts any per-file change, a moved or split file included, as long
//! as the workspace total doesn't rise, and refuses otherwise. (`force` is an
//! alias kept for old instructions; it refuses a rise too.) Blessing under
//! `CI` panics. Because the ratchet counts per file, pairing one field and
//! adding another unpaired one in the same file passes; the `file:line`
//! sites are only printed when a count rises.
//!
//! Failures that are never baselined: a marker with no reason (fewer than
//! two words and 10 characters), a marker on a line that holds more than one
//! ID field (a marker exempts exactly one), an event macro renamed in a
//! `use tracing::...` import, and an event macro that forwards `$(...)`
//! tokens (a `macro_rules!` wrapper) with no call site in the same file,
//! unless its forwarding line carries a marker. A wrapper's call sites in
//! its own file are judged as events, with its fixed fields counted toward
//! pairing.

mod calls;
mod fixture_tests;
mod lexer;
mod pairing;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::PathBuf;

use pairing::Verdict;

use super::target_scan_tests::{crates_dir, is_test_path, rs_files, IN_PROCESS_CRATES};

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
    /// Bad markers and hidden calls, as `file:line  what`. Never baselined.
    broken: Vec<String>,
    calls: usize,
    id_fields: usize,
}

/// Judges every event call in one source text, adding to `scan`.
fn scan_source(rel: &str, src: &str, scan: &mut Scan) {
    let mut masked = lexer::mask(src);
    lexer::blank_test_items(&mut masked.code);
    for (line, name) in calls::renamed_event_macros(&masked.code) {
        scan.broken.push(format!(
            "{rel}:{line}  `use tracing::{name} as ...` hides its calls from the scan; call `{name}!` by name"
        ));
    }
    let none = BTreeSet::new();
    let defs = calls::macro_rules_bodies(&masked.code);
    let mut judged = Vec::new();
    for call in calls::event_calls(src, &masked) {
        scan.calls += 1;
        judged.extend(pairing::judge(&call, &none, &masked.line_comments));
        let Some(line) = call.forwards_at else {
            continue;
        };
        // A `macro_rules!` wrapper forwarding `$(…)` into the event: judge
        // each of its call sites in this file as one more event, with the
        // wrapper's fixed fields available for pairing.
        let wrapper = defs
            .iter()
            .find(|(_, open, close)| (*open..*close).contains(&call.at));
        let sites: Vec<calls::Call> = wrapper.map_or_else(Vec::new, |(name, ..)| {
            calls::macro_calls(src, &masked, &[name.as_str()])
                .into_iter()
                .filter(|c| !defs.iter().any(|(_, o, e)| (*o..*e).contains(&c.at)))
                .collect()
        });
        if sites.is_empty() && pairing::marker_on(&masked.line_comments, line) != Some(true) {
            scan.broken.push(format!(
                "{rel}:{line}  an event macro forwarding `$(...)` tokens, with no call site in \
                 this file, hides its callers' fields; call the event macro directly, or mark \
                 this line `// {} <reason>`",
                pairing::MARKER
            ));
        }
        let fixed: BTreeSet<String> = call.fields.iter().map(|f| f.key.clone()).collect();
        for site in &sites {
            scan.calls += 1;
            judged.extend(pairing::judge(site, &fixed, &masked.line_comments));
        }
    }
    let mut per_line = BTreeMap::<usize, usize>::new();
    for (line, _, _) in &judged {
        *per_line.entry(*line).or_default() += 1;
    }
    for (line, key, verdict) in judged {
        scan.id_fields += 1;
        let at = format!("{rel}:{line}");
        match verdict {
            Verdict::Paired => {}
            Verdict::Exempt if per_line[&line] > 1 => scan.broken.push(format!(
                "{at}  `// {}` exempts one field, but this line holds {} ID fields; \
                 put `{key}` on its own line",
                pairing::MARKER,
                per_line[&line]
            )),
            Verdict::Exempt => {}
            Verdict::Unpaired => scan
                .unpaired
                .entry(rel.to_string())
                .or_default()
                .push(Site { at, key }),
            Verdict::MarkerWithoutReason => scan.broken.push(format!(
                "{at}  `// {}` on `{key}` needs a reason: two words or 10 characters",
                pairing::MARKER
            )),
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
            scan_source(&rel, &src, &mut scan);
        }
    }
    scan
}

fn baseline_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src/logging")
        .join(BASELINE)
}

const TOTAL_PREFIX: &str = "# total ";

/// The `# total N` line, if the baseline has one.
fn baseline_total(text: &str) -> Option<usize> {
    text.lines()
        .find_map(|l| l.trim().strip_prefix(TOTAL_PREFIX))
        .map(|n| {
            n.trim()
                .parse()
                .unwrap_or_else(|_| panic!("{BASELINE}: bad `{TOTAL_PREFIX}{n}` line"))
        })
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
    let _ = writeln!(out, "{TOTAL_PREFIX}{}", counts.values().sum::<usize>());
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

/// Whether a bless may rewrite `baseline` with `scan`'s counts: yes when the
/// workspace total doesn't rise (moves and splits shift counts between files)
/// or there is no baseline yet. `Err` names the rise.
fn bless_check(scan: &Scan, baseline: &BTreeMap<String, usize>) -> Result<(), String> {
    let now: usize = scan.unpaired.values().map(Vec::len).sum();
    let before: usize = baseline.values().sum();
    if baseline.is_empty() || now <= before {
        Ok(())
    } else {
        Err(format!(
            "refusing to bless: the total of unpaired ID fields rose from {before} to {now}. \
             Pair the new fields or mark them; a bless may move counts between files but never \
             raise the total"
        ))
    }
}

/// **The guard.** No bad marker or hidden call anywhere; a `# total` line
/// that matches the rows; no file's unpaired count above its baseline; no
/// stale baseline line.
#[test]
fn unpaired_id_fields_only_shrink() {
    let scan = scan_workspace();
    assert!(
        scan.broken.is_empty(),
        "Rule 6 scan failures (never baselined):\n    {}",
        scan.broken.join("\n    ")
    );

    let path = baseline_path();
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let baseline = parse_baseline(&text);
    let (rose, fell) = compare(&scan, &baseline);
    if let Ok(mode) = std::env::var(BLESS_VAR) {
        assert!(
            std::env::var_os("CI").is_none(),
            "{BLESS_VAR} is set under CI: blessing there would turn the guard off"
        );
        assert!(mode == "1" || mode == "force", "{BLESS_VAR}={mode}: use 1");
        if let Err(why) = bless_check(&scan, &baseline) {
            panic!("{why}:\n{}", rose.join("\n"));
        }
        let counts: BTreeMap<String, usize> = scan
            .unpaired
            .iter()
            .map(|(p, s)| (p.clone(), s.len()))
            .collect();
        std::fs::write(&path, render_baseline(&counts)).unwrap();
        for moved in &rose {
            println!("blessed a per-file rise (the total did not rise): {moved}");
        }
        return;
    }
    let sum: usize = baseline.values().sum();
    assert_eq!(
        baseline_total(&text),
        Some(sum),
        "the `{TOTAL_PREFIX}N` line in {BASELINE} must equal the sum of its rows; re-bless it"
    );
    assert!(
        rose.is_empty(),
        "unpaired ID fields above the baseline in {BASELINE}. A moved or split file can be \
         blessed with `{BLESS_VAR}=1` while the total doesn't rise:\n{}",
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
    // About 95% of the counts when this was written (2,915 calls, 7,087 ID
    // fields): a scanner that starts missing calls fails here before its
    // falling counts get blessed in as progress.
    assert!(scan.calls > 2770, "only {} event calls found", scan.calls);
    assert!(
        scan.id_fields > 6730,
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
