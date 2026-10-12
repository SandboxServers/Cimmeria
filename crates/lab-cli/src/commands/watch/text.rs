//! The plain-text frame of `lab watch`: the `--once` output.

use crate::clip::clip_text;
use crate::daemon::mcp::hide_lease_ids;

use super::snapshot::Snapshot;

const COLUMNS: [&str; 6] = ["INSTANCE", "CLIENT", "LEASE", "RUN", "ROW", "SHOT"];

/// The frame as lines, each at most `width` characters.
pub fn render_text(snapshot: &Snapshot, width: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();

    match &snapshot.batch {
        None => lines.push(
            if snapshot.daemon_up {
                "no batch"
            } else {
                "no batch  daemon down"
            }
            .to_string(),
        ),
        Some(batch) => {
            let mut failed = format!("failed {}", batch.failed);
            if batch.known > 0 {
                failed.push_str(&format!(" ({} known)", batch.known));
            }
            let mut parts = vec![batch.name.clone()];
            if let Some(section) = &batch.section {
                parts.push(section.clone());
            }
            parts.push(batch.state.as_str().to_string());
            parts.push(format!("runs {}", batch.runs_text()));
            parts.push(format!("passed {}", batch.passed));
            parts.push(failed);
            lines.push(parts.join("  "));
            if let Some(last) = &batch.last_failure {
                let mut line = format!("last failure  {}  {}", last.run, last.why);
                if let Some(issue) = &last.issue {
                    line.push_str(&format!("  [{issue}]"));
                }
                lines.push(line);
            }
        }
    }
    lines.push(String::new());

    // The first six cells of a row are the columns; the seventh is LAST.
    let mut rows: Vec<[String; 7]> = vec![[
        COLUMNS[0].to_string(),
        COLUMNS[1].to_string(),
        COLUMNS[2].to_string(),
        COLUMNS[3].to_string(),
        COLUMNS[4].to_string(),
        COLUMNS[5].to_string(),
        "LAST".to_string(),
    ]];
    for i in &snapshot.instances {
        rows.push([
            i.instance.clone(),
            i.client_pid
                .map_or_else(|| "-".to_string(), |p| p.to_string()),
            i.lease
                .clone()
                .filter(|l| !l.is_empty())
                .unwrap_or_else(|| "-".to_string()),
            i.run.clone(),
            i.row_cell(),
            i.shot_s
                .map_or_else(|| "-".to_string(), |s| format!("{s}s")),
            i.last
                .as_ref()
                .map_or_else(String::new, |l| format!("    {l}")),
        ]);
    }
    let widths: Vec<usize> = (0..5)
        .map(|c| rows.iter().map(|r| r[c].chars().count()).max().unwrap_or(0) + 2)
        .collect();
    for (n, row) in rows.iter().enumerate() {
        let mut text = String::new();
        for c in 0..5 {
            text.push_str(&row[c]);
            let pad = widths[c].saturating_sub(row[c].chars().count());
            text.push_str(&" ".repeat(pad));
        }
        text.push_str(&row[5]);
        lines.push(text);
        if n > 0 && !row[6].is_empty() {
            lines.push(row[6].clone());
        }
    }

    let mut foot: Vec<String> = Vec::new();
    if !snapshot.orphans.is_empty() {
        let shown: Vec<String> = snapshot
            .orphans
            .iter()
            .take(3)
            .map(|p| p.to_string())
            .collect();
        let more = if snapshot.orphans.len() > 3 {
            format!(" +{}", snapshot.orphans.len() - 3)
        } else {
            String::new()
        };
        foot.push(format!(
            "orphan SGW.exe: {}{more} (#1342?)",
            shown.join(", ")
        ));
    }
    if let Some(logs) = &snapshot.logs {
        if !logs.relaunches.is_empty() {
            let each: Vec<String> = logs
                .relaunches
                .iter()
                .map(|(n, c)| format!("{n} {c}"))
                .collect();
            foot.push(format!("relaunches: {}", each.join(", ")));
        }
        if logs.login_retries > 0 {
            foot.push(format!("login retries: {} (#1343)", logs.login_retries));
        }
    }
    if !foot.is_empty() {
        lines.push(String::new());
        lines.push(foot.join("  "));
    }
    if snapshot.old_daemon {
        lines.push("daemon has no run progress: lab install, then lab restart".to_string());
    }
    if !snapshot.daemon_up && snapshot.batch.is_some() {
        lines.push("daemon down: showing the batch files only".to_string());
    }

    lines
        .into_iter()
        .map(|line| clip_text(&hide_lease_ids(&line), width))
        .collect()
}
