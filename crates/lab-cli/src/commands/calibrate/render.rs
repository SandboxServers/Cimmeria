//! Fancy-mode rendering of the pose lines and the diff. In plain mode both
//! return the PowerShell text byte for byte.

use crate::daemon::mcp::hide_lease_ids;
use crate::ui::{Mark, Tone, Ui};

use super::outcome::{changes, format_line, score_text, Outcome, CONFIRM_RUNS};

/// One pose's line.
///
/// Fancy:
/// ```text
/// ✓ frost  score 8312 · confirmed 3/3 · bearing 322.5 → 330.0
/// · supply  unchanged · score 7400 · confirmed 3/3
/// ✗ frost  no proposal: <reason> (best score 5100, confirmed 1/3)
/// ```
/// The pose is bold, `✓` green, `✗` bold red, separators and details dim
/// where shown above in the tail. Plain: [`format_line`].
pub fn render_line(ui: &Ui, o: &Outcome) -> String {
    if !ui.fancy {
        return format_line(o);
    }
    let reason = hide_lease_ids(&o.reason);
    let confirmed = format!("confirmed {}/{CONFIRM_RUNS}", o.confirmed);
    let pose = ui.paint(Tone::Bold, &o.pose);
    if !o.ok {
        let mut line = format!(
            "{} {pose}  no proposal",
            ui.paint(Tone::Fail, ui.mark(Mark::Fail))
        );
        if !reason.is_empty() {
            line.push_str(&format!(": {reason}"));
        }
        if let Some(score) = o.score {
            let tail = format!(" (best score {}, {confirmed})", score_text(score));
            line.push_str(&ui.paint(Tone::Dim, &tail));
        }
        return line;
    }
    let dot = ui.paint(Tone::Dim, " · ");
    let mut parts: Vec<String> = Vec::new();
    if let Some(score) = o.score {
        parts.push(format!("score {}", score_text(score)));
    }
    parts.push(confirmed);
    let changes = changes(o);
    let mut line = if changes.is_empty() {
        format!(
            "{} {pose}  unchanged{dot}{}",
            ui.paint(Tone::Dim, "·"),
            ui.paint(Tone::Dim, &parts.join(" · "))
        )
    } else {
        for (label, text) in &changes {
            parts.push(format!("{label} {}", text.replace(" -> ", " → ")));
        }
        format!(
            "{} {pose}  {}",
            ui.paint(Tone::Ok, ui.mark(Mark::Pass)),
            parts.join(dot.as_str())
        )
    };
    if !reason.is_empty() {
        line.push_str(&ui.paint(Tone::Dim, &format!(" ({reason})")));
    }
    line
}

/// The diff text: plain unchanged; fancy with `+` lines green, `-` lines red,
/// `@@` lines cyan and the `---`/`+++` headers bold. Lines are joined with
/// `\n`, with no trailing newline.
pub fn render_diff(ui: &Ui, text: &str) -> String {
    if !ui.fancy {
        return text.to_string();
    }
    text.lines()
        .map(|l| {
            if l.starts_with("+++") || l.starts_with("---") {
                ui.paint(Tone::Bold, l)
            } else if l.starts_with("@@") {
                ui.paint(Tone::Cmd, l)
            } else if l.starts_with('+') {
                ui.paint(Tone::Ok, l)
            } else if l.starts_with('-') {
                ui.paint(Tone::Fail, l)
            } else {
                l.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}
