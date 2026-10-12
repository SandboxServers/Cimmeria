//! The fancy frame of `lab watch`'s live view: a pure function of the snapshot,
//! so tests can check every line without a terminal.

use crate::daemon::mcp::hide_lease_ids;
use crate::ui::table::Table;
use crate::ui::{skip_escape, visible_width, Mark, Tone, Ui};

use super::snapshot::{BatchState, BatchView, Snapshot};

/// The narrowest and widest progress bar, in cells.
const NARROWEST_BAR: usize = 10;
const WIDEST_BAR: usize = 40;

/// The frame as lines, each at most `width` visible columns. `height` is the
/// rows this frame may use (0 or 1: unknown, no cut). A frame that is too tall
/// is cut and ends with `… <n> more lines`.
pub fn render_frame(snapshot: &Snapshot, width: usize, height: usize, ui: &Ui) -> Vec<String> {
    let mut lines = vec![header_line(snapshot, ui)];
    if let Some(batch) = &snapshot.batch {
        lines.push(progress_line(batch, width, ui));
    }
    let table = instance_lines(snapshot, width, ui);
    if !table.is_empty() {
        lines.push(String::new());
        lines.extend(table);
    }
    if let Some(line) = failure_line(snapshot, ui) {
        lines.push(String::new());
        lines.push(line);
    }
    let footer = footer_lines(snapshot, ui);
    if !footer.is_empty() {
        lines.push(String::new());
        lines.extend(footer);
    }
    finish(lines, width, height, ui)
}

/// Hides lease ids, cuts every line to `width`, and cuts the frame to `height`.
fn finish(lines: Vec<String>, width: usize, height: usize, ui: &Ui) -> Vec<String> {
    let mut out: Vec<String> = lines
        .iter()
        .map(|line| fit_line(&hide_lease_ids(line), width))
        .collect();
    if height >= 2 && out.len() > height - 1 {
        let keep = height - 2; // the marker line takes the last kept row
        let hidden = out.len() - keep;
        out.truncate(keep);
        let marker = ui.paint(Tone::Dim, &format!("… {hidden} more lines"));
        out.push(fit_line(&marker, width));
    }
    out
}

/// `line` cut to `width` visible columns, ending in `…` (and a colour reset when
/// the line was painted) when it is cut.
pub fn fit_line(line: &str, width: usize) -> String {
    if visible_width(line) <= width {
        return line.to_string();
    }
    if width == 0 {
        return String::new();
    }
    let keep = width - 1;
    let mut out = String::new();
    let mut visible = 0;
    let mut styled = false;
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            styled = true;
            out.push(c);
            out.push_str(&skip_escape(&mut chars));
        } else if visible < keep {
            out.push(c);
            visible += 1;
        } else {
            break;
        }
    }
    out.push('…');
    if styled {
        out.push_str("\x1b[0m");
    }
    out
}

/// A glyph and a space, painted; empty when glyphs are off.
fn glyph(ui: &Ui, mark: Mark, tone: Tone) -> String {
    let g = ui.mark(mark);
    if g.is_empty() {
        String::new()
    } else {
        format!("{} ", ui.paint(tone, g))
    }
}

fn header_line(snapshot: &Snapshot, ui: &Ui) -> String {
    let Some(batch) = &snapshot.batch else {
        let mut line = "no batch".to_string();
        if !snapshot.daemon_up {
            line.push_str("  ");
            line.push_str(&ui.paint(Tone::Fail, "daemon down"));
        }
        return line;
    };
    let (mark, mark_tone, word_tone) = match batch.state {
        BatchState::Live => (Mark::Active, Tone::Cmd, Some(Tone::Ok)),
        BatchState::Done => (Mark::Pass, Tone::Ok, None),
        BatchState::Interrupted => (Mark::Fail, Tone::Fail, Some(Tone::Fail)),
        BatchState::Unknown => (Mark::Warn, Tone::Warn, Some(Tone::Dim)),
    };
    let word = batch.state.as_str();
    let word = match word_tone {
        Some(tone) => ui.paint(tone, word),
        None => word.to_string(),
    };
    let mut parts = vec![ui.paint(Tone::Bold, &batch.name)];
    if let Some(section) = &batch.section {
        parts.push(section.clone());
    }
    parts.push(word);
    format!("{}{}", glyph(ui, mark, mark_tone), parts.join("  "))
}

/// The progress bar's width in cells for a frame `width` columns wide.
pub fn bar_width(width: usize) -> usize {
    (width / 4).clamp(NARROWEST_BAR, WIDEST_BAR)
}

/// Filled cells of a bar `width` cells wide after `runs` of `total` runs:
/// `runs * width / total`, never above `width`; 0 when `total` is 0.
pub fn bar_cells(runs: u32, total: u32, width: usize) -> usize {
    if total == 0 {
        return 0;
    }
    let done = u64::from(runs.min(total));
    let cells = done * u64::try_from(width).unwrap_or(u64::MAX) / u64::from(total);
    usize::try_from(cells).unwrap_or(width).min(width)
}

fn progress_line(batch: &BatchView, width: usize, ui: &Ui) -> String {
    let head = match batch.total {
        Some(total) if total > 0 => {
            let cells = bar_width(width);
            let filled = bar_cells(batch.runs, total, cells);
            let (full, empty) = if ui.fancy { ('█', '░') } else { ('#', '-') };
            format!(
                "{}{} {}",
                ui.paint(Tone::Ok, &full.to_string().repeat(filled)),
                ui.paint(Tone::Dim, &empty.to_string().repeat(cells - filled)),
                batch.runs_text()
            )
        }
        _ => format!("{} runs", batch.runs),
    };
    let passed = format!("passed {}", batch.passed);
    let passed = if batch.passed > 0 {
        ui.paint(Tone::Ok, &passed)
    } else {
        passed
    };
    let mut failed = format!("failed {}", batch.failed);
    if batch.failed > 0 {
        failed = ui.paint(Tone::Fail, &failed);
    }
    if batch.known > 0 {
        failed.push_str(&ui.paint(Tone::Dim, &format!(" ({} known)", batch.known)));
    }
    format!("{head}  {passed}  {failed}")
}

/// The table header, each instance's row, and under a row its lane's last
/// progress line (indented four spaces). Empty without instances.
fn instance_lines(snapshot: &Snapshot, width: usize, ui: &Ui) -> Vec<String> {
    if snapshot.instances.is_empty() {
        return Vec::new();
    }
    let mut table = Table::new(&["INSTANCE", "CLIENT", "LEASE", "RUN", "ROW", "SHOT"]);
    for i in &snapshot.instances {
        let client = i
            .client_pid
            .map_or_else(|| "-".to_string(), |pid| pid.to_string());
        let lease = match i.lease.as_deref().filter(|l| !l.is_empty()) {
            Some("free") | None => ui.paint(Tone::Dim, i.lease.as_deref().unwrap_or("-")),
            Some(text) => text.to_string(),
        };
        let run = if i.run.contains("braked") {
            ui.paint(Tone::Fail, &i.run)
        } else {
            i.run.clone()
        };
        let shot = i
            .shot_s
            .map_or_else(|| "-".to_string(), |s| format!("{s}s"));
        table.row(vec![
            ui.paint(Tone::Cmd, &i.instance),
            client,
            lease,
            run,
            i.row_cell(),
            shot,
        ]);
    }
    let sized = Ui {
        width: Some(width),
        ..*ui
    };
    let text = table.render(&sized);
    let mut rendered = text.lines();
    let mut out = Vec::new();
    if let Some(header) = rendered.next() {
        out.push(header.to_string());
    }
    for (line, instance) in rendered.zip(&snapshot.instances) {
        out.push(line.to_string());
        if let Some(last) = instance.last.as_deref().filter(|l| !l.is_empty()) {
            out.push(format!("    {}", ui.paint(Tone::Dim, last)));
        }
    }
    out
}

fn failure_line(snapshot: &Snapshot, ui: &Ui) -> Option<String> {
    let last = snapshot.batch.as_ref()?.last_failure.as_ref()?;
    let issue = last
        .issue
        .as_ref()
        .map(|tag| format!("{}  ", ui.paint(Tone::Warn, &format!("[{tag}]"))))
        .unwrap_or_default();
    Some(format!(
        "{}{}  {}  {}{}",
        glyph(ui, Mark::Fail, Tone::Fail),
        ui.paint(Tone::Dim, "last failure"),
        last.run,
        issue,
        last.why
    ))
}

fn footer_lines(snapshot: &Snapshot, ui: &Ui) -> Vec<String> {
    let mut parts: Vec<String> = Vec::new();
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
        parts.push(format!(
            "orphan SGW.exe: {}{more} (#1342?)",
            shown.join(", ")
        ));
    }
    if let Some(logs) = &snapshot.logs {
        if !logs.relaunches.is_empty() {
            let each: Vec<String> = logs
                .relaunches
                .iter()
                .map(|(name, count)| format!("{name} {count}"))
                .collect();
            parts.push(format!("relaunches: {}", each.join(", ")));
        }
        if logs.login_retries > 0 {
            parts.push(format!("login retries: {} (#1343)", logs.login_retries));
        }
    }
    let mut lines = Vec::new();
    if !parts.is_empty() {
        lines.push(ui.paint(Tone::Dim, &parts.join("  ")));
    }
    if snapshot.old_daemon {
        lines.push(ui.paint(
            Tone::Warn,
            "daemon has no run progress: lab install, then lab restart",
        ));
    }
    if !snapshot.daemon_up && snapshot.batch.is_some() {
        lines.push(ui.paint(Tone::Warn, "daemon down: showing the batch files only"));
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::watch::snapshot::{InstanceView, LastFailure, LogCounts};

    /// Glyphs, no colour: strings compare as plain text.
    fn fancy() -> Ui {
        Ui {
            color: false,
            fancy: true,
            width: None,
        }
    }

    fn colour() -> Ui {
        Ui {
            color: true,
            fancy: true,
            width: None,
        }
    }

    fn batch(state: BatchState) -> BatchView {
        BatchView {
            name: "batch-20261011-013906".into(),
            state,
            section: Some("first-session".into()),
            runs: 4,
            total: Some(6),
            passed: 3,
            failed: 1,
            known: 1,
            last_failure: None,
        }
    }

    fn instance(n: u32) -> InstanceView {
        InstanceView {
            instance: format!("p{n}"),
            client_pid: Some(1000 + n),
            lease: Some("free".into()),
            run: "1/3".into(),
            row: "-".into(),
            row_s: None,
            shot_s: Some(5),
            last: None,
        }
    }

    /// A live batch with `lanes` instances, no failure, no footer: its frame is
    /// `4 + lanes` lines (header, progress, blank, table header, one per lane).
    fn sample(lanes: u32) -> Snapshot {
        Snapshot {
            daemon_up: true,
            old_daemon: false,
            batch: Some(batch(BatchState::Live)),
            instances: (1..=lanes).map(instance).collect(),
            orphans: Vec::new(),
            logs: None,
        }
    }

    /// Everything on one screen: a long lease, a long last line, a long failure,
    /// a footer and a warning.
    fn busy() -> Snapshot {
        let mut s = sample(2);
        s.instances[0].lease =
            Some("claude-agent-a1 (first-session UAT with a very long purpose text)".into());
        s.instances[0].last = Some(
            "p1 1/3  ✗ fail  FS-02: Login_PasswordEdit holds 0 characters when read back  [#1343]  1m02s"
                .into(),
        );
        let b = s.batch.as_mut().unwrap();
        b.last_failure = Some(LastFailure {
            run: "default#2".into(),
            why: "FS-01 BLOCKED: setup failed: lab_login: Login_PasswordEdit holds 0 ch...".into(),
            issue: Some("#1343".into()),
        });
        s.orphans = vec![63904, 2, 3, 4, 5];
        s.logs = Some(LogCounts {
            relaunches: vec![("p2".into(), 1)],
            login_retries: 2,
        });
        s.old_daemon = true;
        s
    }

    // bar_cells: runs * width / total, capped, 0 for an empty plan.
    // Fails if reverted: a changed formula (rounding up, no cap, no zero guard) breaks a row.
    #[test]
    fn bar_cells_are_runs_times_width_over_total() {
        assert_eq!(bar_cells(4, 6, 11), 7);
        assert_eq!(bar_cells(0, 6, 11), 0);
        assert_eq!(bar_cells(6, 6, 11), 11);
        assert_eq!(bar_cells(9, 6, 11), 11);
        assert_eq!(bar_cells(3, 0, 11), 0);
    }

    // bar_width: a quarter of the width, between 10 and 40 cells.
    // Fails if reverted: a fixed width, or no clamp, breaks 20, 44 or 400.
    #[test]
    fn bar_width_is_a_quarter_of_the_width_clamped() {
        assert_eq!(bar_width(20), 10);
        assert_eq!(bar_width(44), 11);
        assert_eq!(bar_width(400), 40);
    }

    // The progress line at width 47 (an 11-cell bar; the line is 45 columns, so 44 would cut it).
    // Fails if reverted: a wrong fill (7 of 11 for 4/6), glyph or count text.
    #[test]
    fn the_progress_line_has_the_bar_and_the_counts() {
        let frame = render_frame(&sample(1), 47, 0, &fancy());
        assert_eq!(frame[1], "███████░░░░ 4/6  passed 3  failed 1 (1 known)");
    }

    // Plain (no glyphs, a terminal with NO_COLOR): the bar uses # and -.
    // Fails if reverted: block characters leak into a glyph-free frame.
    #[test]
    fn without_glyphs_the_bar_is_hashes_and_dashes() {
        let ui = Ui {
            color: false,
            fancy: false,
            width: None,
        };
        let frame = render_frame(&sample(1), 47, 0, &ui);
        assert_eq!(frame[1], "#######---- 4/6  passed 3  failed 1 (1 known)");
    }

    // Without a plan total there is no bar.
    // Fails if reverted: a division by zero or a bar of nothing.
    #[test]
    fn a_batch_without_a_plan_shows_a_bare_run_count() {
        let mut s = sample(1);
        let b = s.batch.as_mut().unwrap();
        b.total = None;
        b.state = BatchState::Unknown;
        let frame = render_frame(&s, 80, 0, &fancy());
        assert_eq!(frame[1], "4 runs  passed 3  failed 1 (1 known)");
    }

    // The header: glyph, name, section, state word; the word is coloured.
    // Fails if reverted: a missing state, or live not green / interrupted not red.
    #[test]
    fn the_header_names_the_state_in_its_colour() {
        let mut s = sample(0);
        let line = render_frame(&s, 100, 0, &fancy())[0].clone();
        assert_eq!(line, "▶ batch-20261011-013906  first-session  live");
        let live = render_frame(&s, 100, 0, &colour())[0].clone();
        assert!(live.contains("\x1b[32mlive\x1b[0m"), "{live:?}");
        s.batch.as_mut().unwrap().state = BatchState::Done;
        let done = render_frame(&s, 100, 0, &colour())[0].clone();
        assert!(done.ends_with("  done"), "{done:?}");
        s.batch.as_mut().unwrap().state = BatchState::Interrupted;
        let dead = render_frame(&s, 100, 0, &colour())[0].clone();
        assert!(dead.contains("\x1b[1;31minterrupted\x1b[0m"), "{dead:?}");
    }

    // No batch: one line, and "daemon down" only when it is.
    // Fails if reverted: a panic on the missing batch, or the daemon note lost.
    #[test]
    fn no_batch_is_one_line_and_no_table() {
        let mut s = sample(0);
        s.batch = None;
        s.daemon_up = true;
        assert_eq!(render_frame(&s, 80, 0, &fancy()), ["no batch"]);
        s.daemon_up = false;
        assert_eq!(render_frame(&s, 80, 0, &fancy()), ["no batch  daemon down"]);
    }

    // The table, with each lane's last line indented four spaces under its row.
    // Fails if reverted: last lines dropped, or put under the wrong row.
    #[test]
    fn a_lanes_last_line_sits_under_its_row() {
        let mut s = sample(2);
        s.instances[0].last = Some("p1 1/3  ✓ pass  2/2 rows  4m12s".into());
        let frame = render_frame(&s, 100, 0, &fancy());
        assert!(
            frame[3].starts_with("INSTANCE  CLIENT  LEASE  RUN  ROW  SHOT"),
            "{:?}",
            frame[3]
        );
        assert!(frame[4].starts_with("p1"), "{:?}", frame[4]);
        assert_eq!(frame[5], "    p1 1/3  ✓ pass  2/2 rows  4m12s");
        assert!(frame[6].starts_with("p2"), "{:?}", frame[6]);
        assert_eq!(frame.len(), 7);
    }

    // The last failure carries its issue tag ahead of the reason.
    // Fails if reverted: no tag, or the tag after the reason (cut first at 60).
    #[test]
    fn the_last_failure_keeps_its_issue_tag_at_a_narrow_width() {
        let frame = render_frame(&busy(), 60, 0, &fancy());
        let line = frame
            .iter()
            .find(|l| l.contains("last failure"))
            .expect("a failure line");
        assert!(
            line.starts_with("✗ last failure  default#2  [#1343]  FS-01"),
            "{line:?}"
        );
        assert!(line.ends_with('…'), "{line:?}");
    }

    // The footer.
    // Fails if reverted: a missing +N, relaunch or retry text, or either warning.
    #[test]
    fn the_footer_lists_orphans_relaunches_retries_and_warnings() {
        let frame = render_frame(&busy(), 160, 0, &fancy());
        assert!(frame.contains(
            &"orphan SGW.exe: 63904, 2, 3 +2 (#1342?)  relaunches: p2 1  login retries: 2 (#1343)"
                .to_string()
        ));
        assert!(frame
            .contains(&"daemon has no run progress: lab install, then lab restart".to_string()));
        let mut down = busy();
        down.daemon_up = false;
        assert!(render_frame(&down, 160, 0, &fancy())
            .contains(&"daemon down: showing the batch files only".to_string()));
    }

    // Every line fits the width, painted or not, at the three widths of the packet.
    // Fails if reverted: dropping fit_line (or the table's own cut) lets the long
    // lease, last line and footer overflow 60 columns.
    #[test]
    fn every_line_fits_the_width() {
        for width in [60, 100, 160] {
            for ui in [fancy(), colour()] {
                let frame = render_frame(&busy(), width, 0, &ui);
                assert!(frame.len() > 8, "{width}: {frame:?}");
                for line in &frame {
                    assert!(visible_width(line) <= width, "{width}: {line:?}");
                }
            }
        }
    }

    // fit_line cuts by visible width, keeps the colour code, and resets it.
    // Fails if reverted: it counts escape bytes as columns, or leaves colour on.
    #[test]
    fn fit_line_cuts_painted_text_by_visible_width() {
        let painted = colour().paint(Tone::Cmd, "abcdefghij");
        let cut = fit_line(&painted, 6);
        assert_eq!(cut, "\x1b[36mabcde…\x1b[0m");
        assert_eq!(visible_width(&cut), 6);
        assert_eq!(fit_line("short", 6), "short");
        assert_eq!(fit_line("anything", 0), "");
    }

    // The cut: a 20-line frame in a 10-row window keeps 8 lines and a marker.
    // Fails if reverted: no cut (20 lines), or an off-by-one in the count.
    #[test]
    fn a_tall_frame_is_cut_with_a_more_lines_marker() {
        let s = sample(16); // 20 lines
        assert_eq!(render_frame(&s, 100, 0, &fancy()).len(), 20);
        let cut = render_frame(&s, 100, 10, &fancy());
        assert_eq!(cut.len(), 9);
        assert_eq!(cut[8], "… 12 more lines");
        assert!(cut[7].starts_with("p4"), "{:?}", cut[7]);
        // The boundary: 20 lines fit a 21-row window, not a 20-row one.
        assert_eq!(render_frame(&s, 100, 21, &fancy()).len(), 20);
        let edge = render_frame(&s, 100, 20, &fancy());
        assert_eq!(edge.len(), 19);
        assert_eq!(edge[18], "… 2 more lines");
    }
}
