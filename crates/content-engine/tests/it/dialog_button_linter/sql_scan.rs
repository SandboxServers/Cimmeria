//! A small, deliberately dumb SQL scanner for the seed files.
//!
//! `interact_tag_linter.rs` reads its seeds a line at a time. That works
//! for the chain files' `content_triggers` rows and would be wrong here
//! for two separate reasons, both of which cost rows silently rather
//! than loudly:
//!
//! * `db/resources/Dialogs/Seed/dialog_screens.sql` holds 1,013 rows
//!   whose `text` column contains a raw newline. A line-at-a-time scan
//!   drops every one of them — including screens that decide which
//!   screen of a dialog is last, which is the entire question this
//!   linter answers.
//! * The Castle chain seeds put apostrophes inside `--` comments
//!   (`Frost's`, `work-packets.md's`, `Ba'al`). A scanner that tracks
//!   quotes but not comments reads the first of those as an opening
//!   string literal and swallows every statement after it.
//!
//! So: one pass that understands single-quoted literals with the SQL
//! `''` escape and `--` line comments, and nothing else. There are no
//! dollar-quoted strings and no `/* */` comments anywhere in these files
//! (checked 2026-09-21); a literal `$$` does appear inside one
//! `dialog_screens` text value, which is why `$` is ordinary text here.

use std::collections::HashMap;

/// One decoded `INSERT` row: column name → raw field text, still
/// quoted and still `''`-escaped. Use [`unquote`] to read a string
/// column.
pub(crate) type Row = HashMap<String, String>;

/// Split a seed file into statements, stripping `--` line comments.
///
/// Semicolons and newlines inside a string literal do not split a
/// statement; a comment's apostrophes do not open one.
pub(crate) fn sql_statements(sql: &str) -> Vec<String> {
    let chars: Vec<char> = sql.chars().collect();
    let mut out = Vec::new();
    let mut buf = String::new();
    let mut in_string = false;
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        if in_string {
            if c == '\'' {
                if chars.get(i + 1) == Some(&'\'') {
                    buf.push_str("''");
                    i += 2;
                    continue;
                }
                in_string = false;
            }
            buf.push(c);
            i += 1;
            continue;
        }
        match c {
            '\'' => {
                in_string = true;
                buf.push(c);
                i += 1;
            }
            // Line comment: skip to the newline. The newline itself is
            // kept so tokens either side of a comment stay separated.
            '-' if chars.get(i + 1) == Some(&'-') => {
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
            }
            ';' => {
                push_trimmed(&mut out, &mut buf);
                i += 1;
            }
            _ => {
                buf.push(c);
                i += 1;
            }
        }
    }
    push_trimmed(&mut out, &mut buf);
    out
}

fn push_trimmed(out: &mut Vec<String>, buf: &mut String) {
    if !buf.trim().is_empty() {
        out.push(buf.trim().to_string());
    }
    buf.clear();
}

/// Decode `INSERT INTO <table> (cols...) VALUES (...), (...)` into
/// column-name-keyed rows. Returns empty for any other statement.
///
/// Rows are addressed by column NAME, not by position. Every seed
/// statement writes its column list out in full, and a name-keyed read
/// means a column added to the middle of a table cannot silently shift
/// this linter onto the wrong field — the failure would be a missing
/// key, not a wrong value.
pub(crate) fn insert_rows(stmt: &str, table: &str) -> Vec<Row> {
    let Some(rest) = stmt.strip_prefix(&format!("INSERT INTO {table} ")) else {
        return Vec::new();
    };
    let rest = rest.trim_start();
    if !rest.starts_with('(') {
        return Vec::new();
    }
    // The column list contains no quotes and no nested parens, so the
    // first `)` closes it.
    let Some(close) = rest.find(')') else {
        return Vec::new();
    };
    let columns: Vec<&str> = rest[1..close].split(',').map(str::trim).collect();

    let after_cols = &rest[close + 1..];
    let Some(values_at) = after_cols.find("VALUES") else {
        return Vec::new();
    };

    value_tuples(&after_cols[values_at + "VALUES".len()..])
        .into_iter()
        .filter(|tuple| tuple.len() == columns.len())
        .map(|tuple| {
            columns
                .iter()
                .map(|c| (*c).to_string())
                .zip(tuple)
                .collect::<Row>()
        })
        .collect()
}

/// Split the text after `VALUES` into its parenthesised tuples, each
/// into its raw fields.
///
/// Multi-row `VALUES (...), (...), (...)` is used throughout the chain
/// seeds — chains 1205 and 1234 of `castle_701_chains.sql` among them —
/// so a scanner that stopped at the first tuple would miss most of the
/// `display_dialog` actions.
pub(crate) fn value_tuples(s: &str) -> Vec<Vec<String>> {
    let chars: Vec<char> = s.chars().collect();
    let mut tuples = Vec::new();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] != '(' {
            i += 1;
            continue;
        }
        let mut fields: Vec<String> = Vec::new();
        let mut cur = String::new();
        let mut depth = 0usize;
        let mut in_string = false;

        while i < chars.len() {
            let c = chars[i];
            if in_string {
                if c == '\'' {
                    if chars.get(i + 1) == Some(&'\'') {
                        cur.push_str("''");
                        i += 2;
                        continue;
                    }
                    in_string = false;
                }
                cur.push(c);
                i += 1;
                continue;
            }
            match c {
                '\'' => {
                    in_string = true;
                    cur.push(c);
                }
                '(' => {
                    depth += 1;
                    if depth > 1 {
                        cur.push(c);
                    }
                }
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        fields.push(cur.trim().to_string());
                        i += 1;
                        break;
                    }
                    cur.push(c);
                }
                ',' if depth == 1 => {
                    fields.push(cur.trim().to_string());
                    cur.clear();
                }
                _ => cur.push(c),
            }
            i += 1;
        }
        tuples.push(fields);
    }
    tuples
}

/// Strip the surrounding single quotes from a SQL literal and undo the
/// `''` escape. `None` for an unquoted field (`NULL`, a number, `true`).
pub(crate) fn unquote(field: &str) -> Option<String> {
    let inner = field.strip_prefix('\'')?.strip_suffix('\'')?;
    Some(inner.replace("''", "'"))
}

/// Read a string column, or `None` if it is absent or unquoted.
pub(crate) fn text(row: &Row, column: &str) -> Option<String> {
    row.get(column).map(String::as_str).and_then(unquote)
}

/// Read an integer column, or `None` if it is absent or not an integer.
pub(crate) fn int(row: &Row, column: &str) -> Option<i32> {
    row.get(column)?.parse().ok()
}

// Not gated on `#[cfg(test)]`: this file is already inside a test
// target, and a stray gate that failed to apply would silently drop
// these guards from the run.
mod scanner_guards {
    use super::*;

    #[test]
    fn statements_survive_newlines_and_escaped_quotes_inside_a_literal() {
        // Shape taken from dialog_screens.sql row (77, 8818, ...): a raw
        // newline inside `text`, plus the `''` escape. The semicolon is
        // there too, because a naive split on `;` would cut the row in
        // half at a point where the tail has no INSERT prefix and simply
        // disappears.
        let sql = "INSERT INTO dialog_screens (dialog_id, screen_id, text, speaker_id, index) \
                   VALUES (77, 8818, 'Rebels\nare supported; don''t tell', 0, 0);\n\
                   INSERT INTO dialog_screens (dialog_id, screen_id, text, speaker_id, index) \
                   VALUES (77, 8819, 'Second', 0, 1);\n";
        let stmts = sql_statements(sql);
        assert_eq!(stmts.len(), 2, "got {stmts:?}");

        let rows: Vec<Row> = stmts
            .iter()
            .flat_map(|s| insert_rows(s, "dialog_screens"))
            .collect();
        assert_eq!(rows.len(), 2, "got {rows:?}");
        assert_eq!(
            text(&rows[0], "text").unwrap(),
            "Rebels\nare supported; don't tell"
        );
        assert_eq!(int(&rows[0], "index"), Some(0));
        assert_eq!(int(&rows[1], "screen_id"), Some(8819));
        assert_eq!(int(&rows[1], "index"), Some(1));
    }

    #[test]
    fn apostrophes_inside_line_comments_do_not_open_a_string() {
        // Verbatim shapes from the Castle chain seeds. Without comment
        // handling the first apostrophe opens a literal and both INSERTs
        // vanish into it.
        let sql = "-- Mission 1360: accept Frost's Letter on the body loot\n\
                   INSERT INTO content_triggers (chain_id, event_type, event_key, scope, \
                   once, sort_order) VALUES (1237, 'dialog_choice', '2576', 'player', \
                   false, 0);\n\
                   -- CmdCenter_Ba'al: per-player bind, dsm 5151\n\
                   INSERT INTO content_triggers (chain_id, event_type, event_key, scope, \
                   once, sort_order) VALUES (1238, 'dialog_choice', '2576', 'player', \
                   false, 0);\n";
        let rows: Vec<Row> = sql_statements(sql)
            .iter()
            .flat_map(|s| insert_rows(s, "content_triggers"))
            .collect();
        assert_eq!(rows.len(), 2, "got {rows:?}");
        assert_eq!(int(&rows[0], "chain_id"), Some(1237));
        assert_eq!(int(&rows[1], "chain_id"), Some(1238));
        assert_eq!(text(&rows[1], "event_key").unwrap(), "2576");
    }

    #[test]
    fn a_double_dash_inside_a_literal_is_not_a_comment() {
        let sql = "INSERT INTO dialog_screens (dialog_id, screen_id, text, speaker_id, index) \
                   VALUES (1, 2, 'a -- b', 0, 0);";
        let rows: Vec<Row> = sql_statements(sql)
            .iter()
            .flat_map(|s| insert_rows(s, "dialog_screens"))
            .collect();
        assert_eq!(rows.len(), 1, "got {rows:?}");
        assert_eq!(text(&rows[0], "text").unwrap(), "a -- b");
    }

    #[test]
    fn every_tuple_of_a_multi_row_values_list_is_read() {
        let sql = "INSERT INTO content_actions (chain_id, action_type, target_id, target_key, \
                   params, delay_ms, sort_order) VALUES\n\
                   (1205, 'accept_mission', 701, NULL, '{}', 0, 0),\n\
                   (1205, 'display_dialog', 5862, NULL, '{\"qty\": 1, \"x\": 2}', 0, 3);\n";
        let rows: Vec<Row> = sql_statements(sql)
            .iter()
            .flat_map(|s| insert_rows(s, "content_actions"))
            .collect();
        assert_eq!(rows.len(), 2, "both tuples must be read: {rows:?}");
        assert_eq!(text(&rows[1], "action_type").unwrap(), "display_dialog");
        assert_eq!(int(&rows[1], "target_id"), Some(5862));
        // The params JSON carries a comma inside quotes; if that split
        // the tuple, the field count would no longer match the column
        // count and the whole row would be dropped instead.
        assert_eq!(int(&rows[1], "sort_order"), Some(3));
        assert_eq!(rows[1]["target_key"], "NULL");
        assert_eq!(text(&rows[1], "target_key"), None, "NULL is not a string");
    }

    #[test]
    fn insert_rows_ignores_statements_for_other_tables() {
        let sql = "INSERT INTO content_chains (chain_id, description, scope_type, scope_id, \
                   enabled, priority) VALUES (1237, 'x', 'mission', 701, true, 0);";
        let stmts = sql_statements(sql);
        assert_eq!(stmts.len(), 1);
        assert!(insert_rows(&stmts[0], "content_triggers").is_empty());
        assert_eq!(insert_rows(&stmts[0], "content_chains").len(), 1);
    }
}
