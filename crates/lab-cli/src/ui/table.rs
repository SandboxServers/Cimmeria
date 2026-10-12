//! A text table whose columns align by visible width, so painted cells line up.

use super::{cell_width, skip_escape, text_width, Tone, Ui};

/// Spaces between columns.
const SEP: &str = "  ";

/// A table: a header row and rows of cells.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Table {
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
}

impl Table {
    /// Starts a table with these column headers.
    pub fn new(headers: &[&str]) -> Self {
        Self {
            headers: headers.iter().map(|h| (*h).to_string()).collect(),
            rows: Vec::new(),
        }
    }

    /// Adds a row. Cells may carry [`Ui::paint`] codes.
    pub fn row(&mut self, cells: Vec<String>) {
        self.rows.push(cells);
    }

    /// Renders the header and the rows, one line each. Headers are dim and
    /// upper case in fancy mode. The last column is cut to `ui.width` with `…`.
    pub fn render(&self, ui: &Ui) -> String {
        let cols = self.headers.len();
        if cols == 0 {
            return String::new();
        }
        let labels: Vec<String> = self
            .headers
            .iter()
            .map(|h| {
                if ui.fancy {
                    h.to_uppercase()
                } else {
                    h.clone()
                }
            })
            .collect();
        let mut widths: Vec<usize> = labels.iter().map(|l| text_width(l)).collect();
        for row in &self.rows {
            for (i, cell) in row.iter().enumerate().take(cols) {
                widths[i] = widths[i].max(text_width(cell));
            }
        }
        let last = cols - 1;
        if let Some(total) = ui.width {
            let used = widths[..last].iter().sum::<usize>() + SEP.len() * last;
            widths[last] = widths[last].min(total.saturating_sub(used).max(1));
        }

        let mut out = String::new();
        let header: Vec<String> = labels.iter().map(|l| ui.paint(Tone::Dim, l)).collect();
        push_line(&mut out, &header, &widths);
        for row in &self.rows {
            let cells: Vec<String> = (0..cols)
                .map(|i| row.get(i).cloned().unwrap_or_default())
                .collect();
            push_line(&mut out, &cells, &widths);
        }
        out
    }
}

/// Appends one line: every cell padded to its column but the last, which is cut.
fn push_line(out: &mut String, cells: &[String], widths: &[usize]) {
    let last = cells.len() - 1;
    let parts: Vec<String> = cells
        .iter()
        .enumerate()
        .map(|(i, cell)| {
            if i == last {
                fit(cell, widths[i])
            } else {
                pad(cell, widths[i])
            }
        })
        .collect();
    out.push_str(&parts.join(SEP));
    out.push('\n');
}

/// `cell` followed by spaces up to `width` visible columns.
fn pad(cell: &str, width: usize) -> String {
    let spaces = width.saturating_sub(text_width(cell));
    format!("{cell}{}", " ".repeat(spaces))
}

/// `cell` cut to `width` visible columns, ending in `…` when it is cut.
fn fit(cell: &str, width: usize) -> String {
    if text_width(cell) <= width {
        return cell.to_string();
    }
    let keep = width.saturating_sub(1);
    let mut out = String::new();
    let mut visible = 0;
    let mut styled = false;
    let mut chars = cell.chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            styled = true;
            out.push(c);
            out.push_str(&skip_escape(&mut chars));
        } else if visible + cell_width(c) <= keep {
            out.push(c);
            visible += cell_width(c);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn columns_align_by_text_width() {
        let ui = Ui::plain();
        let mut table = Table::new(&["Name", "State"]);
        table.row(vec!["\x1b[36muat\x1b[0m".to_string(), "idle".to_string()]);
        table.row(vec!["calibrate".to_string(), "busy".to_string()]);
        let text = table.render(&ui);
        assert_eq!(
            text,
            "Name       State\n\x1b[36muat\x1b[0m        idle\ncalibrate  busy\n"
        );
    }

    #[test]
    fn last_column_is_cut_to_the_width() {
        let ui = Ui {
            color: false,
            fancy: false,
            width: Some(16),
        };
        let mut table = Table::new(&["Name", "Summary"]);
        table.row(vec![
            "uat".to_string(),
            "Run a UAT spec on the lab".to_string(),
        ]);
        let text = table.render(&ui);
        let line = text.lines().nth(1).expect("row line");
        assert_eq!(line, "uat   Run a UAT…");
        assert_eq!(text_width(line), 16);
    }

    #[test]
    fn painted_last_column_is_cut_by_visible_width_and_reset() {
        let ui = Ui {
            color: true,
            fancy: true,
            width: Some(12),
        };
        let mut table = Table::new(&["A", "B"]);
        table.row(vec![
            "x".to_string(),
            ui.paint(Tone::Cmd, "abcdefghijklmnop"),
        ]);
        let text = table.render(&ui);
        let line = text.lines().nth(1).expect("row line");
        assert_eq!(text_width(line), 12);
        assert!(line.ends_with("…[0m"), "{line:?}");
    }

    // F-LX6: columns align by terminal cells, not characters.
    #[test]
    fn wide_cells_align_by_terminal_columns() {
        let ui = Ui::plain();
        let mut table = Table::new(&["Name", "State"]);
        table.row(vec!["日本".to_string(), "x".to_string()]);
        table.row(vec!["abcd".to_string(), "y".to_string()]);
        assert_eq!(table.render(&ui), "Name  State\n日本  x\nabcd  y\n");
    }

    #[test]
    fn a_wide_last_column_is_cut_by_cells() {
        let ui = Ui {
            color: false,
            fancy: false,
            width: Some(8),
        };
        let mut table = Table::new(&["A", "B"]);
        table.row(vec!["x".to_string(), "日本語日本語".to_string()]);
        let text = table.render(&ui);
        let line = text.lines().nth(1).expect("row line");
        assert_eq!(line, "x  日本…");
        assert!(text_width(line) <= 8);
    }

    #[test]
    fn fancy_headers_are_upper_case() {
        let ui = Ui {
            color: false,
            fancy: true,
            width: None,
        };
        let mut table = Table::new(&["Name", "State"]);
        table.row(vec!["a".to_string(), "b".to_string()]);
        assert!(table.render(&ui).starts_with(
            "NAME  STATE
"
        ));
    }
}
