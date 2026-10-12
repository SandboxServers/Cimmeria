//! What `lab uat` prints: progress lines while it runs (plain: the PowerShell's
//! text; fancy: the contract's mock-up) and the fancy summary tables.

use std::io::Write;

use crate::batch::{format_progress_line, PlanLane, RunRecord};
use crate::ui::table::Table;
use crate::ui::{Mark, Tone, Ui};

use super::summary::{FailureGroup, Summary};

/// Where progress lines go: LX-15's `args::Progress` under this module's name
/// (`Host`, `Stderr`, `None`), so the pre-flight and the batch agree and there
/// is one enum. `progress_mode` is LX-15's port of `Get-UatProgressMode`
/// (`--progress-to-stderr` wins, `--json` or `--quiet` silence, else the console).
pub use super::args::{progress_mode, Progress as ProgressMode};

/// `Write-UatProgress`: one line to the stream `mode` names; write errors are ignored.
pub fn write_line(mode: ProgressMode, message: &str, out: &mut dyn Write, err: &mut dyn Write) {
    let _ = match mode {
        ProgressMode::Host => writeln!(out, "{message}"),
        ProgressMode::Stderr => writeln!(err, "{message}"),
        ProgressMode::None => Ok(()),
    };
}

/// A thread-safe emitter that writes one whole line per call to the real streams.
pub fn stdio_emit(mode: ProgressMode) -> impl Fn(&str) + Sync {
    move |line: &str| {
        let out = std::io::stdout();
        let err = std::io::stderr();
        write_line(mode, line, &mut out.lock(), &mut err.lock());
    }
}

/// The plain header line of a batch.
pub fn plain_header(
    section: &str,
    lanes: &[PlanLane],
    runs_per_lease: u32,
    row_ids: &[String],
) -> String {
    let instances: Vec<&str> = lanes.iter().map(|l| l.instance.as_str()).collect();
    format!(
        "lab uat {section}: {n} lease(s) x {runs_per_lease} run(s) = {total} run(s) on {on}; rows {rows}",
        n = lanes.len(),
        total = lanes.len() as u32 * runs_per_lease,
        on = instances.join(", "),
        rows = row_ids.join(", ")
    )
}

fn count(n: usize, noun: &str) -> String {
    if n == 1 {
        format!("1 {noun}")
    } else {
        format!("{n} {noun}s")
    }
}

/// The fancy header: `▶ <section> · <n> rows · <n> lanes × <n> runs · <batch>`.
pub fn fancy_header(
    ui: &Ui,
    section: &str,
    rows: usize,
    lanes: usize,
    runs_per_lease: u32,
    batch_name: &str,
) -> String {
    format!(
        "{} {} · {} · {} × {} · {batch_name}",
        ui.paint(Tone::Cmd, ui.mark(Mark::Active)),
        ui.paint(Tone::Bold, section),
        count(rows, "row"),
        count(lanes, "lane"),
        count(runs_per_lease as usize, "run"),
    )
}

/// `text` cut to `max` characters, ending in `…` when cut.
pub fn cut(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let mut out: String = text.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

/// `45s`, `1m02s`, `4m12s`.
pub fn fmt_duration(seconds: u64) -> String {
    if seconds < 60 {
        format!("{seconds}s")
    } else {
        format!("{}m{:02}s", seconds / 60, seconds % 60)
    }
}

/// The width of the widest `<instance> <i>/<total>` label of the lanes.
pub fn label_width(lanes: &[PlanLane], total: u32) -> usize {
    lanes
        .iter()
        .map(|l| format!("{} {total}/{total}", l.instance).chars().count())
        .max()
        .unwrap_or(0)
}

/// The width the fail text is cut to (the mock-up's `FS-02: Login_PasswordEdit
/// holds 0 ch…` is 37).
const FAIL_TEXT_WIDTH: usize = 37;

/// The fancy line of one finished run:
/// `  <label>  ✓ pass  <p>/<n> rows  <time>` or
/// `  <label>  ✗ fail  <row>: <reason…>  [#issue]  <time>` or
/// `  <label>  ✗ error  <error…>  <time>`. The label is padded to `label_width`.
pub fn fancy_run_line(ui: &Ui, rec: &RunRecord, total: u32, label_width: usize) -> String {
    let label = format!("{} {}/{}", rec.instance, rec.index, total);
    let pad = " ".repeat(label_width.saturating_sub(label.chars().count()));
    let time = fmt_duration(rec.seconds);
    let body = if rec.ok {
        let passed = rec
            .rows
            .iter()
            .filter(|r| r.result.eq_ignore_ascii_case("PASS"))
            .count();
        format!(
            "{}  {}/{} rows  {time}",
            ui.paint(Tone::Ok, &format!("{} pass", ui.mark(Mark::Pass))),
            passed,
            rec.rows.len()
        )
    } else if let Some(error) = rec.error.as_deref().filter(|e| !e.is_empty()) {
        format!(
            "{}  {}  {time}",
            ui.paint(Tone::Fail, &format!("{} error", ui.mark(Mark::Fail))),
            cut(error, FAIL_TEXT_WIDTH)
        )
    } else {
        let verdict = rec.verdict.as_ref();
        let row = verdict.and_then(|v| v.row.as_deref()).unwrap_or("");
        let reason = verdict.and_then(|v| v.reason.as_deref()).unwrap_or("");
        let tag = verdict
            .and_then(|v| v.issue.as_deref())
            .filter(|i| !i.is_empty())
            .map(|i| format!("  {}", ui.paint(Tone::Warn, &format!("[{i}]"))))
            .unwrap_or_default();
        format!(
            "{}  {}{tag}  {time}",
            ui.paint(Tone::Fail, &format!("{} fail", ui.mark(Mark::Fail))),
            cut(&format!("{row}: {reason}"), FAIL_TEXT_WIDTH)
        )
    };
    format!("  {label}{pad}  {body}")
}

/// The line that says a lane stopped (`rest` is `stopped after 3 failed runs in a
/// row` or `its lease did not clear after the timeout; lane stopped`): plain
/// `[<instance>] <rest>`, fancy `  ! <instance>  <rest>`.
pub fn lane_notice(ui: &Ui, fancy: bool, instance: &str, rest: &str) -> String {
    if fancy {
        format!(
            "  {} {instance}  {rest}",
            ui.paint(Tone::Warn, ui.mark(Mark::Warn))
        )
    } else {
        format!("[{instance}] {rest}")
    }
}

/// `3/4  75%`, coloured: 100 % green, 0 % red, else yellow (no runs: dim). The
/// percentage is rounded down.
pub fn rate_cell(ui: &Ui, pass: u32, total: u32) -> String {
    let pct = (pass * 100).checked_div(total).unwrap_or(0);
    let tone = if total == 0 {
        Tone::Dim
    } else if pass == total {
        Tone::Ok
    } else if pass == 0 {
        Tone::Fail
    } else {
        Tone::Warn
    };
    ui.paint(tone, &format!("{pass}/{total}  {pct}%"))
}

/// The fancy summary: verdict line, the failed count, a lane table, a row table,
/// the failure groups with their known-issue tags, the batch folder. Every line
/// ends in `\n`.
pub fn fancy_summary(ui: &Ui, summary: &Summary, title: &str, dir: &str) -> String {
    let (mark, tone, word) = if summary.ok {
        (Mark::Pass, Tone::Ok, "PASS")
    } else {
        (Mark::Fail, Tone::Fail, "FAIL")
    };
    let mut out = format!(
        "{} {}\n",
        ui.paint(tone, ui.mark(mark)),
        ui.paint(
            Tone::Bold,
            &format!(
                "{title}: {word} ({}/{} runs passed)",
                summary.passed, summary.runs
            )
        )
    );
    if summary.failed > 0 {
        out.push_str(&ui.paint(
            Tone::Warn,
            &format!(
                "Failed: {} ({} known issue, {} new)",
                summary.failed, summary.failed_known, summary.failed_new
            ),
        ));
        out.push('\n');
    }
    out.push('\n');
    let mut lanes = Table::new(&["Lane", "Instance", "Stopped", "Passed"]);
    for l in &summary.lanes {
        lanes.row(vec![
            l.lane.to_string(),
            l.instance.clone(),
            if l.braked {
                ui.paint(Tone::Warn, "yes")
            } else {
                ui.paint(Tone::Dim, "-")
            },
            rate_cell(ui, l.passed, l.runs),
        ]);
    }
    out.push_str(&lanes.render(ui));
    out.push('\n');
    let mut rows = Table::new(&["Row", "Passed"]);
    for (id, stat) in &summary.rows.0 {
        rows.row(vec![id.clone(), rate_cell(ui, stat.pass, stat.total)]);
    }
    out.push_str(&rows.render(ui));
    if !summary.failures.is_empty() {
        out.push('\n');
        out.push_str(&ui.paint(Tone::Heading, "Failures, grouped by the first failing row:"));
        out.push('\n');
        let mut sorted: Vec<&FailureGroup> = summary.failures.iter().collect();
        sorted.sort_by_key(|f| std::cmp::Reverse(f.count));
        for f in sorted {
            let tag = f
                .issue
                .as_deref()
                .filter(|i| !i.is_empty())
                .map(|i| format!("  {}", ui.paint(Tone::Warn, &format!("[{i}]"))))
                .unwrap_or_default();
            out.push_str(&format!(
                "  {}x {}{tag}  runs: {}\n",
                f.count,
                f.signature,
                f.runs.join(", ")
            ));
        }
    }
    out.push('\n');
    out.push_str(&format!("Batch: {}\n", ui.paint(Tone::Cmd, dir)));
    out
}

/// The one line `--quiet` prints at the end.
pub fn quiet_line(title: &str, summary: &Summary, dir: &str) -> String {
    format!("{title}: {}/{} passed; {dir}", summary.passed, summary.runs)
}

/// Prints batch progress through an emitter, in the mode's style. Whole lines
/// only, so lanes never interleave inside a line.
pub struct Progress<'a> {
    ui: Ui,
    mode: ProgressMode,
    fancy: bool,
    lane_count: usize,
    per_lane: u32,
    width: usize,
    emit: &'a (dyn Fn(&str) + Sync),
}

impl<'a> Progress<'a> {
    /// `ui.fancy` makes the lines fancy only when they go to the console.
    pub fn new(
        ui: Ui,
        mode: ProgressMode,
        lanes: &[PlanLane],
        per_lane: u32,
        emit: &'a (dyn Fn(&str) + Sync),
    ) -> Self {
        Self {
            ui,
            mode,
            fancy: ui.fancy && mode == ProgressMode::Host,
            lane_count: lanes.len(),
            per_lane,
            width: label_width(lanes, per_lane),
            emit,
        }
    }

    /// Whether the fancy style is on.
    pub fn is_fancy(&self) -> bool {
        self.fancy
    }

    /// Sends one line, unless the mode is `None`.
    pub fn say(&self, line: &str) {
        if self.mode != ProgressMode::None {
            (self.emit)(line);
        }
    }

    /// The batch's header line.
    pub fn header(
        &self,
        section: &str,
        lanes: &[PlanLane],
        row_ids: &[String],
        batch_name: &str,
    ) -> String {
        if self.fancy {
            fancy_header(
                &self.ui,
                section,
                row_ids.len(),
                self.lane_count,
                self.per_lane,
                batch_name,
            )
        } else {
            plain_header(section, lanes, self.per_lane, row_ids)
        }
    }

    /// The line of one finished run.
    pub fn run_line(&self, rec: &RunRecord) -> String {
        if self.fancy {
            fancy_run_line(&self.ui, rec, self.per_lane, self.width)
        } else {
            format_progress_line(rec, self.per_lane)
        }
    }

    /// The line that says a lane stopped.
    pub fn notice(&self, instance: &str, rest: &str) -> String {
        lane_notice(&self.ui, self.fancy, instance, rest)
    }
}
