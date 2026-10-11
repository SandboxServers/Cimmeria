//! D-SV2 spec edits: rewrite one pose line in place, and show it as a diff.

use serde::Deserialize;

use crate::uat::spec::{self, PoseSpec, SectionSpec};

/// Context lines either side of a hunk, as `git diff` shows.
const CONTEXT: usize = 2;

/// Replace the one line holding pose `new.id` inside row `row_id` with
/// `new.to_inline_toml()`, keeping its indentation, trailing comma,
/// trailing comment and line ending (CRLF or LF). Errors: no such row, no
/// such pose line, or more than one.
pub fn rewrite_pose(text: &str, row_id: &str, new: &PoseSpec) -> Result<String, String> {
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    let at = pose_line(&lines, row_id, &new.id)?;
    let mut out = String::with_capacity(text.len());
    for (i, line) in lines.iter().enumerate() {
        if i == at {
            out.push_str(&replace_table(line, &new.to_inline_toml())?);
        } else {
            out.push_str(line);
        }
    }
    check_rewrite(text, &out, row_id, new)?;
    Ok(out)
}

/// Nothing but the pose may move, and the pose must load as `new` would.
/// The line search is textual, so this re-parse is what catches a line it
/// mis-scoped (a `[[row]]` inside a multi-line string, say).
fn check_rewrite(text: &str, out: &str, row_id: &str, new: &PoseSpec) -> Result<(), String> {
    let before = spec::parse(text).map_err(|e| format!("spec does not parse: {e}"))?;
    let after = spec::parse(out).map_err(|e| format!("rewrite does not parse: {e}"))?;
    if rest_of(&before, row_id, &new.id) != rest_of(&after, row_id, &new.id) {
        return Err(format!(
            "rewrite of {:?} changed more than its pose line",
            new.id
        ));
    }
    let row = after
        .rows
        .iter()
        .find(|r| r.id == row_id)
        .ok_or_else(|| format!("no row {row_id:?} after the rewrite"))?;
    let got = row
        .pose
        .iter()
        .find(|p| p.id == new.id)
        .ok_or_else(|| format!("no pose {:?} after the rewrite", new.id))?;
    if *got != round_trip(new)? {
        return Err(format!("pose {:?} would not reload as written", new.id));
    }
    Ok(())
}

/// A unified diff (`--- a/<path>`, `+++ b/<path>`, `@@ -l,n +l,n @@`, two
/// lines of context) that `git apply` accepts. Empty when old == new.
/// One hunk spans the first changed line to the last.
pub fn line_diff(path: &str, old: &str, new: &str) -> String {
    if old == new {
        return String::new();
    }
    let a: Vec<&str> = old.split_inclusive('\n').collect();
    let b: Vec<&str> = new.split_inclusive('\n').collect();
    let head = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    let room = a.len().min(b.len()) - head;
    let tail = a
        .iter()
        .rev()
        .zip(b.iter().rev())
        .take(room)
        .take_while(|(x, y)| x == y)
        .count();
    let (a_end, b_end) = (a.len() - tail, b.len() - tail);
    let ctx_start = head.saturating_sub(CONTEXT);
    let post = CONTEXT.min(tail);
    let (a_stop, b_stop) = (a_end + post, b_end + post);
    // git apply wants `/`; a Windows path would name a file called `a\b`.
    let path = path.replace('\\', "/");
    let mut out = format!("--- a/{path}\n+++ b/{path}\n");
    out += &format!(
        "@@ -{} +{} @@\n",
        hunk_range(ctx_start, a_stop - ctx_start),
        hunk_range(ctx_start, b_stop - ctx_start)
    );
    for line in &a[ctx_start..head] {
        push_line(&mut out, ' ', line);
    }
    for line in &a[head..a_end] {
        push_line(&mut out, '-', line);
    }
    for line in &b[head..b_end] {
        push_line(&mut out, '+', line);
    }
    for line in &a[a_end..a_stop] {
        push_line(&mut out, ' ', line);
    }
    out
}

/// The one line of row `row_id` that holds pose `pose_id`. A row block runs
/// from its `[[row]]` header to the next one, and only the lines of its
/// `pose = [ ... ]` array are searched (an expect clause may share the id).
fn pose_line(lines: &[&str], row_id: &str, pose_id: &str) -> Result<usize, String> {
    let headers: Vec<usize> = (0..lines.len())
        .filter(|&i| is_row_header(lines[i]))
        .collect();
    let mut blocks = Vec::new();
    for (k, &start) in headers.iter().enumerate() {
        let end = headers.get(k + 1).copied().unwrap_or(lines.len());
        if first_id(&lines[start..end]) == Some(row_id) {
            blocks.push((start, end));
        }
    }
    let (start, end) = match blocks.as_slice() {
        [] => return Err(format!("no row {row_id:?}")),
        [one] => *one,
        _ => return Err(format!("{} rows are named {row_id:?}", blocks.len())),
    };
    let (start, end) = pose_array(&lines[start..end])
        .map(|(s, e)| (start + s, start + e))
        .ok_or_else(|| format!("row {row_id:?} has no pose array"))?;
    let prefix = format!("{{ id = \"{pose_id}\"");
    let hits: Vec<usize> = (start..end)
        .filter(|&i| lines[i].trim_start().starts_with(&prefix))
        .collect();
    match hits.as_slice() {
        [] => Err(format!("row {row_id:?} has no pose line for {pose_id:?}")),
        [one] => Ok(*one),
        _ => Err(format!(
            "row {row_id:?} has {} pose lines for {pose_id:?}",
            hits.len()
        )),
    }
}

/// A `[[row]]` table header, spaces inside the brackets and a trailing
/// comment allowed.
fn is_row_header(line: &str) -> bool {
    let code = line.split('#').next().unwrap_or("");
    code.split_whitespace().collect::<String>() == "[[row]]"
}

/// The line range `[first, past)` of a row block's `pose = [ ... ]` array:
/// from its `pose = [` line to the line holding the `]` that closes it.
/// Brackets inside strings and comments don't count.
fn pose_array(block: &[&str]) -> Option<(usize, usize)> {
    let first = block.iter().position(|l| {
        l.trim_start()
            .strip_prefix("pose")
            .and_then(|r| r.trim_start().strip_prefix('='))
            .is_some_and(|r| r.trim_start().starts_with('['))
    })?;
    let mut depth = 0usize;
    for (i, line) in block.iter().enumerate().skip(first) {
        let mut quote: Option<char> = None;
        let mut escaped = false;
        for c in line.chars() {
            if let Some(q) = quote {
                if escaped {
                    escaped = false;
                } else if c == '\\' && q == '"' {
                    escaped = true;
                } else if c == q {
                    quote = None;
                }
                continue;
            }
            match c {
                '"' | '\'' => quote = Some(c),
                '#' => break,
                '[' => depth += 1,
                ']' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        return Some((first, i + 1));
                    }
                }
                _ => {}
            }
        }
    }
    // Unclosed: the spec would not have parsed, but stay inside the block.
    Some((first, block.len()))
}

/// The value of the first `id = "..."` line in a row block.
fn first_id<'a>(block: &[&'a str]) -> Option<&'a str> {
    let rest = block
        .iter()
        .copied()
        .find_map(|l| l.trim().strip_prefix("id = \""))?;
    rest.split('"').next()
}

/// `line` with its inline table `{ ... }` swapped for `table`. The
/// indentation, the comma, any comment and the line ending stay.
fn replace_table(line: &str, table: &str) -> Result<String, String> {
    let body = line.trim_end_matches(['\r', '\n']);
    let ending = &line[body.len()..];
    let indent = body.len() - body.trim_start().len();
    let rest = &body[indent..];
    let end = table_end(rest).ok_or("pose line has no closing brace")?;
    Ok(format!(
        "{}{table}{}{ending}",
        &body[..indent],
        &rest[end + 1..]
    ))
}

/// Byte index of the `}` that closes the inline table `s` starts with,
/// skipping quoted strings (a `calibrated` note may hold a brace).
fn table_end(s: &str) -> Option<usize> {
    let mut quote: Option<char> = None;
    let mut escaped = false;
    for (i, c) in s.char_indices() {
        if let Some(q) = quote {
            if escaped {
                escaped = false;
            } else if c == '\\' && q == '"' {
                escaped = true;
            } else if c == q {
                quote = None;
            }
        } else if c == '"' || c == '\'' {
            quote = Some(c);
        } else if c == '}' {
            return Some(i);
        }
    }
    None
}

/// The spec with one pose taken out of one row: what must not change.
fn rest_of(spec: &SectionSpec, row_id: &str, pose_id: &str) -> String {
    let mut rows = spec.rows.clone();
    for row in rows.iter_mut().filter(|r| r.id == row_id) {
        row.pose.retain(|p| p.id != pose_id);
    }
    format!("{rows:?}")
}

/// A pose line as the file will hold it: `to_inline_toml`, loaded again.
fn round_trip(p: &PoseSpec) -> Result<PoseSpec, String> {
    #[derive(Deserialize)]
    struct OnePose {
        pose: Vec<PoseSpec>,
    }
    let text = format!("pose = [{}]", p.to_inline_toml());
    let one: OnePose = toml::from_str(&text).map_err(|e| format!("pose line: {e}"))?;
    one.pose
        .into_iter()
        .next()
        .ok_or_else(|| "pose line: no pose".to_string())
}

/// `start,count` in a hunk header: 1-based, and an empty range names the
/// line before it.
fn hunk_range(start: usize, count: usize) -> String {
    let first = if count == 0 { start } else { start + 1 };
    format!("{first},{count}")
}

/// One hunk line: its tag, its text, and a newline (git's marker when the
/// file has none).
fn push_line(out: &mut String, tag: char, line: &str) {
    out.push(tag);
    out.push_str(line.strip_suffix('\n').unwrap_or(line));
    out.push('\n');
    if !line.ends_with('\n') {
        out.push_str("\\ No newline at end of file\n");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::uat::spec::parse;

    const HEAD: &str = "schema = 1\n[section]\nid = \"cal\"\nsystem = \"calibration\"\n\
        guide = \"g.md#c\"\nledger = \"l.md\"\n";

    const FROST: &str = "  { id = \"frost\", tag = \"ArmYourself_FrostBody\", radius_m = 4.16, \
        bearing_deg = 322.5, dy_m = 0.13, pitch_deg = -28.8, yaw_offset_deg = 25.3, \
        zoom = 250.0, calibrated = \"2026-10-10 hand\" },\n";

    /// A row with the given pose lines (each ending in a newline).
    fn row(id: &str, poses: &str) -> String {
        format!(
            "\n[[row]]\nid = \"{id}\"\ntitle = \"t\"\nexpected = \"e\"\npose = [\n{poses}]\n\
             step = [{{ chat = \".help\", label = \"help\" }}]\n\
             [[row.expect]]\nid = \"help\"\ntext = \"t\"\nsource = \"chat\"\n\
             since = \"help\"\ncontains = \"help\"\n"
        )
    }

    /// The pose `pose_id` of row `row_id`, as the spec parses it.
    fn pose_of(text: &str, row_id: &str, pose_id: &str) -> PoseSpec {
        let spec = parse(text).unwrap();
        let row = spec.rows.iter().find(|r| r.id == row_id).unwrap();
        row.pose.iter().find(|p| p.id == pose_id).unwrap().clone()
    }

    #[test]
    fn rewrites_only_the_pose_line() {
        let text = format!(
            "{HEAD}{}{}",
            row("R1", FROST),
            row("R2", &FROST.replace("radius_m = 4.16", "radius_m = 3.0"))
        );
        let mut new = pose_of(&text, "R2", "frost");
        new.bearing_deg = 330.0;
        let out = rewrite_pose(&text, "R2", &new).unwrap();
        let changed: Vec<(&str, &str)> = text
            .lines()
            .zip(out.lines())
            .filter(|(a, b)| a != b)
            .collect();
        assert_eq!(changed.len(), 1, "{changed:?}");
        assert!(changed[0].1.contains("bearing_deg = 330.0"), "{changed:?}");
        // Row 1's frost line is the one still at 4.16.
        assert_eq!(out.matches("radius_m = 4.16").count(), 1);
    }

    #[test]
    fn keeps_crlf_and_the_trailing_comment() {
        let poses = FROST.replace("},\n", "}, # keep me\n");
        let text = format!("{HEAD}{}", row("R1", &poses)).replace('\n', "\r\n");
        let mut new = pose_of(&text, "R1", "frost");
        new.bearing_deg = 330.0;
        let out = rewrite_pose(&text, "R1", &new).unwrap();
        assert!(out.contains("\r\n  { id = \"frost\", tag = "), "{out}");
        assert!(out.contains("bearing_deg = 330.0"), "{out}");
        assert!(out.contains("}, # keep me\r\n"), "{out}");
        assert_eq!(out.matches('\n').count(), out.matches("\r\n").count());
    }

    #[test]
    fn refuses_a_missing_or_duplicated_pose_line() {
        let text = format!("{HEAD}{}", row("R1", FROST));
        let new = pose_of(&text, "R1", "frost");
        let missing = rewrite_pose(
            &text,
            "R1",
            &PoseSpec {
                id: "guard".into(),
                ..new.clone()
            },
        );
        assert!(missing.unwrap_err().contains("no pose line"));
        assert!(rewrite_pose(&text, "R9", &new)
            .unwrap_err()
            .contains("no row"));
        let twice = format!("{HEAD}{}", row("R1", &format!("{FROST}{FROST}")));
        assert!(rewrite_pose(&twice, "R1", &new)
            .unwrap_err()
            .contains("2 pose lines"));
    }

    #[test]
    fn refuses_a_mis_scoped_line_that_changes_another_row() {
        // A `[[row]]` line inside R1's multi-line title looks like a header to
        // the line search, so "R2" lands on R1's frost line. Only the re-parse
        // can tell; R1's pose must not move.
        let r1 = row("R1", FROST).replace(
            "title = \"t\"",
            "title = \"\"\"\n[[row]]\nid = \"R2\"\n\"\"\"",
        );
        let text = format!("{HEAD}{r1}");
        let mut new = pose_of(&text, "R1", "frost");
        new.bearing_deg = 330.0;
        let err = rewrite_pose(&text, "R2", &new).unwrap_err();
        assert!(err.contains("changed more than its pose line"), "{err}");
    }

    #[test]
    fn refuses_a_rewrite_that_would_not_reload_as_written() {
        let text = format!("{HEAD}{}", row("R1", FROST));
        let mut new = pose_of(&text, "R1", "frost");
        new.bearing_deg = 330.0;
        let good = rewrite_pose(&text, "R1", &new).unwrap();
        check_rewrite(&text, &good, "R1", &new).unwrap();
        // The file holds a different pose than the one proposed.
        let mut other = new.clone();
        other.bearing_deg = 300.0;
        let wrong = rewrite_pose(&text, "R1", &other).unwrap();
        let err = check_rewrite(&text, &wrong, "R1", &new).unwrap_err();
        assert!(err.contains("would not reload as written"), "{err}");
    }

    #[test]
    fn finds_a_row_whose_header_has_a_comment() {
        let guard = FROST.replace("id = \"frost\"", "id = \"guard\"");
        let r2 = row("R2", FROST).replace("[[row]]\n", "[[ row ]] # note\n");
        let text = format!("{HEAD}{}{r2}", row("R1", &guard));
        let mut new = pose_of(&text, "R2", "frost");
        new.bearing_deg = 330.0;
        let out = rewrite_pose(&text, "R2", &new).unwrap();
        assert_eq!(pose_of(&out, "R2", "frost").bearing_deg, 330.0);
        assert_eq!(pose_of(&out, "R1", "guard").bearing_deg, 322.5);
    }

    #[test]
    fn an_expect_clause_with_the_pose_id_is_not_a_pose_line() {
        let r1 = format!(
            "\n[[row]]\nid = \"R1\"\ntitle = \"t\"\nexpected = \"e\"\npose = [\n{FROST}]\n\
             step = [{{ chat = \".help\", label = \"help\" }}]\n\
             expect = [\n  {{ id = \"frost\", text = \"t\", source = \"chat\", \
             since = \"help\", contains = \"help\" }},\n]\n"
        );
        let text = format!("{HEAD}{r1}");
        let mut new = pose_of(&text, "R1", "frost");
        new.bearing_deg = 330.0;
        let out = rewrite_pose(&text, "R1", &new).unwrap();
        assert_eq!(pose_of(&out, "R1", "frost").bearing_deg, 330.0);
        assert!(out.contains("  { id = \"frost\", text = \"t\""), "{out}");
    }

    #[test]
    fn the_diff_applies() {
        let old = "a\nb\nc\nd\ne\nf\n";
        let new = "a\nb\nX\nd\ne\nf\n";
        assert_eq!(
            line_diff("x.toml", old, new),
            "--- a/x.toml\n+++ b/x.toml\n@@ -1,5 +1,5 @@\n a\n b\n-c\n+X\n d\n e\n"
        );
        assert_eq!(line_diff("x.toml", old, old), "");
    }

    #[test]
    fn the_diff_at_the_first_and_last_line() {
        let old = "a\nb\nc\nd\n";
        assert_eq!(
            line_diff("x.toml", old, "X\nb\nc\nd\n"),
            "--- a/x.toml\n+++ b/x.toml\n@@ -1,3 +1,3 @@\n-a\n+X\n b\n c\n"
        );
        assert_eq!(
            line_diff("x.toml", old, "a\nb\nc\nX\n"),
            "--- a/x.toml\n+++ b/x.toml\n@@ -2,3 +2,3 @@\n b\n c\n-d\n+X\n"
        );
    }

    #[test]
    fn the_diff_marks_a_missing_final_newline() {
        assert_eq!(
            line_diff("x.toml", "a\nb", "a\nc"),
            "--- a/x.toml\n+++ b/x.toml\n@@ -1,2 +1,2 @@\n a\n-b\n\
             \\ No newline at end of file\n+c\n\\ No newline at end of file\n"
        );
        // Adding the newline is a change too.
        assert_eq!(
            line_diff("x.toml", "a\nb", "a\nb\n"),
            "--- a/x.toml\n+++ b/x.toml\n@@ -1,2 +1,2 @@\n a\n-b\n\
             \\ No newline at end of file\n+b\n"
        );
    }

    #[test]
    fn the_diff_keeps_crlf_lines_and_uses_forward_slashes() {
        assert_eq!(
            line_diff(r"data\lab\x.toml", "a\r\nb\r\nc\r\n", "a\r\nX\r\nc\r\n"),
            "--- a/data/lab/x.toml\n+++ b/data/lab/x.toml\n@@ -1,3 +1,3 @@\n \
             a\r\n-b\r\n+X\r\n c\r\n"
        );
    }
}
