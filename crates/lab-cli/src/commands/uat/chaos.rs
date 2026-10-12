//! `lab uat --chaos`: the batch-level helpers of lab-chaos CH-08 and CH-09 (the
//! header of a batch, a chaos run's ref in a divergence group, and the
//! divergence from the golden).

use std::sync::OnceLock;

use regex::Regex;

use crate::batch::RunRecord;

use super::golden::{merge_golden, GoldenSummary};

/// `seed=<digits>`: what differs between the runs' headers.
fn seed_digits() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"seed=\d+").expect("seed regex"))
}

/// `seed=<anything up to a space>`: a seed as the run ref shows it.
fn seed_token() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"seed=\S+").expect("seed token regex"))
}

fn header_of(run: &RunRecord) -> &str {
    run.chaos.as_deref().unwrap_or("")
}

/// The batch's chaos header (`Select-UatChaosHeader`): the runs' own headers
/// replace the typed one (`fallback`, kept only when no run returned one). Runs
/// differ in seed, so when the headers differ only by seed the seed is named once
/// as `<per run>`; headers that differ beyond that are counted, not listed.
pub fn select_header(runs: &[RunRecord], fallback: Option<&str>) -> Option<String> {
    let mut seen: Vec<&str> = Vec::new();
    for run in runs {
        let h = header_of(run);
        if !h.is_empty() && !seen.contains(&h) {
            seen.push(h);
        }
    }
    match seen.len() {
        0 => fallback.filter(|f| !f.is_empty()).map(str::to_string),
        1 => Some(seen[0].to_string()),
        _ => {
            let mut norm: Vec<String> = Vec::new();
            for h in &seen {
                let n = seed_digits().replace_all(h, "seed=<per run>").into_owned();
                if !norm.contains(&n) {
                    norm.push(n);
                }
            }
            if norm.len() == 1 {
                Some(norm.remove(0))
            } else {
                Some(format!(
                    "{} (+{} other spec(s) in batch.jsonl)",
                    norm[0],
                    norm.len() - 1
                ))
            }
        }
    }
}

/// The runs that ran under chaos (`chaos` true: they carry the runner's header)
/// or the rest (`chaos` false), in order, cloned.
pub fn select_runs(runs: &[RunRecord], chaos: bool) -> Vec<RunRecord> {
    runs.iter()
        .filter(|r| !header_of(r).is_empty() == chaos)
        .cloned()
        .collect()
}

/// A chaos run's ref in a divergence group (`Get-UatChaosRunRef`):
/// `<instance>#<index> seed=<seed>`, one seed per distinct value, joined with
/// `,`; a header with no seed is shown whole in parentheses; no header is the
/// plain `<instance>#<index>`.
pub fn run_ref(run: &RunRecord) -> String {
    let base = format!("{}#{}", run.instance, run.index);
    let header = header_of(run);
    let mut seeds: Vec<String> = Vec::new();
    for m in seed_token().find_iter(header) {
        let seed = m.as_str().trim_end_matches(';').to_string();
        if !seeds.contains(&seed) {
            seeds.push(seed);
        }
    }
    if !seeds.is_empty() {
        format!("{base} {}", seeds.join(","))
    } else if !header.is_empty() {
        format!("{base} ({header})")
    } else {
        base
    }
}

/// The golden comparison of a batch's chaos runs (`Merge-UatChaosDivergence`,
/// reported as `chaos_divergence` in place of `golden`): the same shape and
/// grouping as [`merge_golden`], each group's runs named by [`run_ref`]. `None`
/// only when no chaos run was compared.
pub fn merge_divergence(runs: &[RunRecord]) -> Option<GoldenSummary> {
    merge_golden(&select_runs(runs, true), &run_ref)
}
