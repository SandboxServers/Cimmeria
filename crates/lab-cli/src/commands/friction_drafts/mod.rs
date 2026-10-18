//! `lab friction-drafts`: turn exported human friction rows into bug cards.
//!
//! [`card`] renders one cluster as markdown. [`run`] reads the JSONL export
//! (see [`crate::human`]), keeps the clusters that repeat, and writes one card
//! per cluster into `--out`. It never writes into the spec tree.

pub mod card;

#[cfg(test)]
mod tests;

use std::path::PathBuf;

use serde_json::json;

use crate::commands::Ctx;
use crate::human::friction::{cluster_friction, FrictionCluster};
use crate::human::rows::read_jsonl;

const LONG: &str = concat!(
    "Reads an export of telemetry rows (JSONL, one JSON object per line) and writes one markdown bug card per friction cluster into --out. Export the rows with the ClickHouse JSONEachRow recipe in docs/operations/signoz/interaction-pose-views.md.",
    "\n\n",
    "Only rows with session_kind = player count. Lab and unknown sessions are dropped. A cluster groups rows that share a signal, reason, world and subject; only clusters with at least --min-count rows are kept, and at most --top of them, most frequent first.",
    "\n\n",
    "Each card holds the facts of its cluster and a draft row for the spec. The draft row is UNCONFIRMED: every TOML line in a card is commented out, and a person must confirm and uncomment it. The cards are proposals only.",
    "\n\n",
    "The command never writes into the spec tree (docs/guides/uat-specs/). Output: one line per card (file, rows, players, signal at subject (world)), or with --json one compact JSON array of {file, signal, world, subject, count, players}.",
    "\n\n",
    "Exit codes: 0 the cards were written (or there were none to write); 1 the export could not be read or a card could not be written.",
);

const EXAMPLES: &str = "Examples:\n  lab friction-drafts friction.jsonl --out drafts\n  lab friction-drafts friction.jsonl --out drafts --top 5 --json";

/// Turn exported human friction rows into bug cards
#[derive(clap::Args, Debug)]
#[command(about, long_about = LONG, after_help = EXAMPLES)]
pub struct Args {
    /// The JSONL export of telemetry rows
    pub rows: PathBuf,
    /// The folder the cards are written to (created if missing)
    #[arg(long)]
    pub out: PathBuf,
    /// Write at most this many cards, most frequent first
    #[arg(long, default_value_t = 20)]
    pub top: usize,
    /// Keep only clusters with at least this many rows
    #[arg(long, default_value_t = 2)]
    pub min_count: usize,
    /// Print one JSON array instead of one line per card
    #[arg(long)]
    pub json: bool,
}

/// Runs `lab friction-drafts` and returns its exit code: 0 the cards were
/// written, 1 the export could not be read or a card could not be written.
pub fn run(args: &Args, ctx: &Ctx) -> u8 {
    let rows = match read_jsonl(&args.rows) {
        Ok(rows) => rows,
        Err(msg) => {
            ctx.ui.error(&msg, None);
            return 1;
        }
    };
    let clusters: Vec<FrictionCluster> = cluster_friction(&rows)
        .into_iter()
        .filter(|c| c.count >= args.min_count)
        .take(args.top)
        .collect();
    if let Err(e) = std::fs::create_dir_all(&args.out) {
        ctx.ui.error(&format!("{}: {e}", args.out.display()), None);
        return 1;
    }
    let mut written = Vec::new();
    for (index, cluster) in clusters.iter().enumerate() {
        let path = args.out.join(format!(
            "bug-{:02}-{}.md",
            index + 1,
            file_slug(&cluster.signal)
        ));
        if let Err(e) = std::fs::write(&path, card::render_card(cluster)) {
            ctx.ui.error(&format!("{}: {e}", path.display()), None);
            return 1;
        }
        written.push((path, cluster));
    }
    if args.json {
        let list: Vec<serde_json::Value> = written
            .iter()
            .map(|(path, c)| {
                json!({
                    "file": path.display().to_string(),
                    "signal": c.signal,
                    "world": c.world,
                    "subject": c.subject,
                    "count": c.count,
                    "players": c.players,
                })
            })
            .collect();
        anstream::println!("{}", serde_json::Value::Array(list));
    } else {
        for (path, c) in &written {
            anstream::println!(
                "{}  {} rows, {} players  {} at {} ({})",
                path.display(),
                c.count,
                c.players,
                c.signal,
                c.subject,
                c.world
            );
        }
    }
    0
}

/// The signal as a file-name part: ASCII letters, digits, `-` and `_`; any other character becomes `-`.
fn file_slug(signal: &str) -> String {
    signal
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect()
}
