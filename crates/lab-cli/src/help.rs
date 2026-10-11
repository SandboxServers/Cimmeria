//! The root help screen and the "did you mean" suggestions.

use crate::commands::{Group, COMMANDS};
use crate::ui::{Tone, Ui};

/// Example lines: the command, then its one-line meaning.
const EXAMPLES: [(&str, &str); 4] = [
    ("lab status", "what is running, and who holds each lease"),
    ("lab doctor", "check the setup"),
    ("lab logs --follow", "follow the daemon log"),
    (
        "lab uat first-session --rows FS-01,FS-02 --plan-only",
        "plan a UAT run without driving anything",
    ),
];

/// The column the example meanings start at, counted from the indent.
const EXAMPLE_COLUMN: usize = 54;

/// The root help screen: usage, every command by group, examples and a footer.
pub fn root_help(ui: &Ui) -> String {
    let longest = COMMANDS.iter().map(|c| c.name.len()).max().unwrap_or(0);
    let name_column = longest + 2;
    let mut lines = vec![
        "lab - drive the Cimmeria Live Research Lab".to_string(),
        String::new(),
        ui.paint(Tone::Heading, "Usage"),
        "  lab <command> [options]".to_string(),
        "  lab help <command>      full help for one command (or: lab <command> -h)".to_string(),
    ];
    for group in Group::ALL {
        lines.push(String::new());
        lines.push(ui.paint(Tone::Heading, group.title()));
        for info in COMMANDS.iter().filter(|c| c.group == group) {
            let pad = " ".repeat(name_column - info.name.len());
            let summary = cut_summary(ui, info.summary, 2 + name_column);
            lines.push(format!(
                "  {}{pad}{summary}",
                ui.paint(Tone::Cmd, info.name)
            ));
        }
    }
    lines.push(String::new());
    lines.push(ui.paint(Tone::Heading, "Examples"));
    for (command, meaning) in EXAMPLES {
        let pad = " ".repeat(EXAMPLE_COLUMN.saturating_sub(command.len()));
        lines.push(format!("  {}{pad}{meaning}", ui.paint(Tone::Cmd, command)));
    }
    lines.push(String::new());
    lines.push(ui.paint(
        Tone::Dim,
        &format!(
            "lab {} - docs/guides/live-research-lab.md#the-lab-command",
            env!("CARGO_PKG_VERSION")
        ),
    ));
    lines.join("\n")
}

/// `summary` cut to the terminal width with `…`, in fancy mode only. `room`
/// is the columns already taken before the summary starts.
fn cut_summary(ui: &Ui, summary: &str, room: usize) -> String {
    match ui.width {
        Some(width) if ui.fancy && summary.chars().count() > width.saturating_sub(room) => {
            let keep = width.saturating_sub(room).saturating_sub(1);
            let head: String = summary.chars().take(keep).collect();
            format!("{head}…")
        }
        _ => summary.to_string(),
    }
}

/// Command names that `name` could mean: the names that start with it (when
/// it is two characters or more), else the names within edit distance 2.
pub fn suggest(name: &str) -> Vec<&'static str> {
    let names = COMMANDS.iter().map(|c| c.name);
    if name.chars().count() >= 2 {
        let prefix: Vec<&'static str> = names.clone().filter(|n| n.starts_with(name)).collect();
        if !prefix.is_empty() {
            return prefix;
        }
    }
    names.filter(|n| edit_distance(n, name) <= 2).collect()
}

/// Levenshtein distance between two strings, by characters.
fn edit_distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut row = vec![i + 1; b.len() + 1];
        for (j, cb) in b.iter().enumerate() {
            let subst = prev[j] + usize::from(ca != *cb);
            row[j + 1] = subst.min(prev[j + 1] + 1).min(row[j] + 1);
        }
        prev = row;
    }
    prev[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_help_lists_every_command_under_its_group_in_order() {
        let text = root_help(&Ui::plain());
        let mut at = 0;
        for group in Group::ALL {
            let heading = text[at..]
                .find(group.title())
                .unwrap_or_else(|| panic!("missing heading {}", group.title()));
            at += heading;
            for info in COMMANDS.iter().filter(|c| c.group == group) {
                let found = text[at..]
                    .find(&format!("  {}", info.name))
                    .unwrap_or_else(|| panic!("missing {}", info.name));
                at += found;
            }
        }
        assert!(text.starts_with("lab - drive the Cimmeria Live Research Lab"));
        assert!(text.contains("lab help <command>"));
    }

    #[test]
    fn suggestions_follow_prefix_then_distance() {
        let stauts = suggest("stauts");
        assert!(stauts.contains(&"status"));
        assert!(stauts.contains(&"start"));
        assert_eq!(suggest("gold"), ["golden"]);
        assert!(suggest("zzzz").is_empty());
    }
}
