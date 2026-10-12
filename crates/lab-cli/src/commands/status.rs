//! `lab status`: the daemon and each lab instance, from the daemon's status
//! endpoint. LX-02 gives it its clap arguments and help page; the body is
//! still the PowerShell fallback.

use std::path::Path;

use serde_json::Value;

use crate::commands::Ctx;
use crate::daemon::env_file::{self, EnvLine};
use crate::daemon::paths;
use crate::daemon::status::{self, Endpoint, StatusReply};
use crate::ui::table::Table;
use crate::ui::{Mark, Tone, Ui};

const LONG: &str = concat!(
    "Reads the daemon's GET /status and prints the daemon and each lab instance: account, client, port, lease, seed.",
    "\n\n",
    "When the daemon or its endpoint is down, prints the instances named by labd.env's CIMMERIA_LAB_INSTANCES with the SEED column only, and exits 1.",
    "\n\n",
    "SEED is 'seeded' when the instance's profile folder exists under the profile root, else '-'.",
);

const EXAMPLES: &str = "Examples:\n  lab status";

// Arguments of `lab status`.
#[derive(clap::Args, Debug)]
#[command(
    about = "Show the daemon and each instance: account, client, port, lease, seed",
    long_about = LONG,
    after_help = EXAMPLES
)]
pub struct Args {}

/// The plain-mode column names, in the PowerShell order.
pub const COLUMNS: [&str; 6] = ["INSTANCE", "ACCOUNT", "CLIENT", "PORT", "LEASE", "SEED"];

/// The plain-mode first line when the daemon does not answer.
pub const DAEMON_DOWN: &str = "daemon: not running (or status endpoint unavailable)";

/// A duration as `12s`, `5m` or `2h 3m` (minutes and hours truncate, so 90 s
/// is `1m`). Port of `Format-Ago`.
pub fn format_ago(seconds: u64) -> String {
    if seconds < 60 {
        format!("{seconds}s")
    } else if seconds < 3600 {
        format!("{}m", seconds / 60)
    } else {
        format!("{}h {}m", seconds / 3600, (seconds % 3600) / 60)
    }
}

/// A table cell: the text, or `-` when it is missing or empty.
fn dash(value: Option<&str>) -> String {
    match value {
        Some(text) if !text.is_empty() => text.to_string(),
        _ => "-".to_string(),
    }
}

/// Seconds left on a lease: `remaining_s`, or 0 when it is missing or negative.
fn remaining_s(held: &Value) -> u64 {
    held.get("remaining_s")
        .and_then(|value| {
            value
                .as_u64()
                .or_else(|| value.as_f64().map(|seconds| seconds.max(0.0) as u64))
        })
        .unwrap_or(0)
}

/// The LEASE cell. `free` when the lease is missing or not held; `held` when
/// it is held but the daemon sent no lease object. Held: plain mode is
/// `<owner> (<purpose>, <ago> left)`, fancy mode `<owner> (<purpose>) · <ago> left`.
/// A missing owner or purpose is `-`. Never shows the lease id. Port of
/// `Get-LeaseText`.
pub fn lease_text(lease: Option<&Value>, fancy: bool) -> String {
    let Some(lease) = lease else {
        return "free".to_string();
    };
    if !lease.get("held").and_then(Value::as_bool).unwrap_or(false) {
        return "free".to_string();
    }
    let Some(held) = lease.get("lease").filter(|value| value.is_object()) else {
        return "held".to_string();
    };
    let owner = dash(held.get("owner").and_then(Value::as_str));
    let purpose = dash(held.get("purpose").and_then(Value::as_str));
    let left = format_ago(remaining_s(held));
    if fancy {
        format!("{owner} ({purpose}) · {left} left")
    } else {
        format!("{owner} ({purpose}, {left} left)")
    }
}

/// Whether the instance's profile folder exists:
/// `<root>\<label>\profile\Documents\My Games\Firesky\SGWGame`. Port of
/// `Get-SeedText`.
pub fn is_seeded(profile_root: &Path, label: &str) -> bool {
    profile_root
        .join(label)
        .join("profile")
        .join("Documents")
        .join("My Games")
        .join("Firesky")
        .join("SGWGame")
        .exists()
}

/// The instances named by labd.env's `CIMMERIA_LAB_INSTANCES` (comma list,
/// each trimmed, blanks dropped), or `default` when there are none.
pub fn instance_labels(lines: &[EnvLine]) -> Vec<String> {
    let raw = env_file::get(lines, "CIMMERIA_LAB_INSTANCES").unwrap_or("");
    let labels: Vec<String> = raw
        .split(',')
        .map(|label| label.trim().to_string())
        .filter(|label| !label.is_empty())
        .collect();
    if labels.is_empty() {
        vec!["default".to_string()]
    } else {
        labels
    }
}

/// One instance row, ready to print in either mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusRow {
    pub instance: String,
    pub account: String,
    pub client: String,
    pub port: String,
    /// The LEASE cell in plain mode.
    pub lease_plain: String,
    /// The LEASE cell in fancy mode.
    pub lease_fancy: String,
    pub seeded: bool,
}

/// Rows from the daemon's reply, in its order.
pub fn rows_from_reply(reply: &StatusReply, profile_root: &Path) -> Vec<StatusRow> {
    reply
        .instances
        .iter()
        .map(|inst| StatusRow {
            instance: inst.instance.clone(),
            account: dash(inst.account.as_deref()),
            client: inst
                .client_pid
                .map_or_else(|| "-".to_string(), |pid| pid.to_string()),
            port: inst
                .bridge_port
                .map_or_else(|| "-".to_string(), |port| port.to_string()),
            lease_plain: lease_text(inst.lease.as_ref(), false),
            lease_fancy: lease_text(inst.lease.as_ref(), true),
            seeded: is_seeded(profile_root, &inst.instance),
        })
        .collect()
}

/// Rows for the daemon-down case: only the instance and the seed are known.
pub fn rows_from_labels(labels: &[String], profile_root: &Path) -> Vec<StatusRow> {
    labels
        .iter()
        .map(|label| StatusRow {
            instance: label.clone(),
            account: "-".to_string(),
            client: "-".to_string(),
            port: "-".to_string(),
            lease_plain: "-".to_string(),
            lease_fancy: "-".to_string(),
            seeded: is_seeded(profile_root, label),
        })
        .collect()
}

/// The first line. Plain: `daemon: pid <pid>, up <ago>, version <version>`, or
/// [`DAEMON_DOWN`]. Fancy: `lab daemon  ● running  pid <pid> · up <ago> · <version>`,
/// or `lab daemon  ○ not running  (or status endpoint unavailable)`.
pub fn daemon_line(ui: &Ui, reply: Option<&StatusReply>) -> String {
    match reply {
        Some(reply) => {
            let ago = format_ago(reply.daemon.uptime_s.unwrap_or(0));
            if ui.fancy {
                format!(
                    "{}  {} {}  pid {} · up {} · {}",
                    ui.paint(Tone::Bold, "lab daemon"),
                    ui.paint(Tone::Ok, ui.mark(Mark::Running)),
                    ui.paint(Tone::Ok, "running"),
                    reply.daemon.pid,
                    ago,
                    reply.daemon.version
                )
            } else {
                format!(
                    "daemon: pid {}, up {}, version {}",
                    reply.daemon.pid, ago, reply.daemon.version
                )
            }
        }
        None if ui.fancy => format!(
            "{}  {} {}  {}",
            ui.paint(Tone::Bold, "lab daemon"),
            ui.paint(Tone::Fail, ui.mark(Mark::Stopped)),
            ui.paint(Tone::Fail, "not running"),
            ui.paint(Tone::Dim, "(or status endpoint unavailable)")
        ),
        None => DAEMON_DOWN.to_string(),
    }
}

/// The plain table, as `Format-Table -AutoSize | Out-String` prints it: a
/// blank line, the header, a dash row, the rows, a blank line. Columns are one
/// space apart and padded to their widest cell; the last column is not padded.
/// No rows is an empty string.
pub fn plain_table(rows: &[StatusRow]) -> String {
    if rows.is_empty() {
        return String::new();
    }
    let data: Vec<[String; 6]> = rows
        .iter()
        .map(|row| {
            [
                row.instance.clone(),
                row.account.clone(),
                row.client.clone(),
                row.port.clone(),
                row.lease_plain.clone(),
                if row.seeded { "seeded" } else { "-" }.to_string(),
            ]
        })
        .collect();
    let mut widths = COLUMNS.map(str::len);
    for row in &data {
        for (i, cell) in row.iter().enumerate() {
            widths[i] = widths[i].max(cell.chars().count());
        }
    }
    let line = |cells: &[String; 6]| -> String {
        cells
            .iter()
            .enumerate()
            .map(|(i, cell)| {
                if i == 5 {
                    cell.clone()
                } else {
                    format!("{cell:<width$}", width = widths[i])
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    };
    let header = COLUMNS.map(String::from);
    let dashes = COLUMNS.map(|name| "-".repeat(name.len()));
    let mut out = String::from("\n");
    out.push_str(&line(&header));
    out.push('\n');
    out.push_str(&line(&dashes));
    out.push('\n');
    for row in &data {
        out.push_str(&line(row));
        out.push('\n');
    }
    out.push('\n');
    out
}

/// The fancy table of the mock-up: `INSTANCE ACCOUNT CLIENT PORT LEASE PROFILE`,
/// two spaces between columns, instance names cyan, `-` and `free` dim,
/// `seeded` green, `not seeded` yellow. No rows is an empty string.
pub fn fancy_table(ui: &Ui, rows: &[StatusRow]) -> String {
    if rows.is_empty() {
        return String::new();
    }
    let dim_dash = |text: &str| {
        if text == "-" {
            ui.paint(Tone::Dim, text)
        } else {
            text.to_string()
        }
    };
    let mut table = Table::new(&["Instance", "Account", "Client", "Port", "Lease", "Profile"]);
    for row in rows {
        let lease = if row.lease_fancy == "free" || row.lease_fancy == "-" {
            ui.paint(Tone::Dim, &row.lease_fancy)
        } else {
            row.lease_fancy.clone()
        };
        let profile = if row.seeded {
            ui.paint(Tone::Ok, "seeded")
        } else {
            ui.paint(Tone::Warn, "not seeded")
        };
        table.row(vec![
            ui.paint(Tone::Cmd, &row.instance),
            dim_dash(&row.account),
            dim_dash(&row.client),
            dim_dash(&row.port),
            lease,
            profile,
        ]);
    }
    table.render(ui)
}

/// The whole output and the exit code: 0 with a reply; 1 without one (the
/// `labd.env` instances are listed with the seed column only).
pub fn render(
    ui: &Ui,
    reply: Option<&StatusReply>,
    env_lines: &[EnvLine],
    profile_root: &Path,
) -> (String, u8) {
    let (rows, code) = match reply {
        Some(reply) => (rows_from_reply(reply, profile_root), 0),
        None => (
            rows_from_labels(&instance_labels(env_lines), profile_root),
            1,
        ),
    };
    let head = daemon_line(ui, reply);
    let text = if ui.fancy {
        let table = fancy_table(ui, &rows);
        if table.is_empty() {
            format!("{head}\n")
        } else {
            format!("{head}\n\n{table}")
        }
    } else {
        format!("{head}\n{}", plain_table(&rows))
    };
    (text, code)
}

/// Runs `lab status`: asks the daemon's `GET /status`, prints, exits 0 (the
/// daemon answered) or 1 (it did not).
pub fn run(args: &Args, ctx: &Ctx) -> u8 {
    let _ = args;
    let reply = status::fetch(&Endpoint::discover());
    let env_lines = env_file::read_or_empty(&paths::labd_env_path());
    let (text, code) = render(&ctx.ui, reply.as_ref(), &env_lines, &paths::profile_root());
    anstream::print!("{text}");
    code
}

#[cfg(test)]
mod tests {
    use crate::commands::{parse_ok, try_parse, Command};

    /// The `.EXAMPLE` line of status.ps1 parses.
    #[test]
    fn examples_parse() {
        assert!(matches!(parse_ok("status", &[]), Command::Status(_)));
        assert!(try_parse("status", &["extra"]).is_err());
    }
}
