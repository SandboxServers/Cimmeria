//! LR-05: the draft-spec transform and the draft file name (D-LR5, D-LR8,
//! D-LR10). A recording becomes a section file through one fixed table
//! ([`row`]); a fact outside it is a `# TODO` line, and a row that cannot be
//! written validly is written whole as comments under a `# TODO` header. The
//! output is checked with [`spec::parse`] before it is returned, so a draft
//! that would not load is an error, never a file.

use crate::uat::spec::{self, PoseSpec, SectionMeta};

use super::clauses::{key, ClauseOut, Inline};
use super::fixture;
use super::log::{RecordLog, RecordStart};
use super::segment::segment;

#[path = "emit_row.rs"]
mod row;
#[path = "emit_ticks.rs"]
mod ticks;

use row::draft_row;

/// Bumped when a rule changes what a log turns into; written in the header,
/// so a draft names the transform that wrote it.
pub const TRANSFORM_VERSION: u32 = 1;

/// What the transform needs besides the log.
#[derive(Debug, Clone)]
pub struct TransformOptions {
    /// The named section's meta, when it exists: `system`, `guide`,
    /// `ledger`, `account` and `character` are copied from it.
    pub base: Option<SectionMeta>,
    /// The section's `account` when `base` is None (`gm` or `non-gm`).
    pub account: String,
    /// The section's `character` when `base` is None (`lab`).
    pub character: String,
    /// Replaces the recorded character in SQL (`Px${run_id}`); default
    /// `${character}`. Pasted between SQL quotes as it is.
    pub character_expr: Option<String>,
    /// The fixture profile (LR-09): a `[section.profiles]` key of `base`.
    /// Set, each row gets a `[row.fixture]` and the draft copies the
    /// profile; None writes the fixture as comments.
    pub profile: Option<String>,
    /// At most this many actions across all rows (LR-15).
    #[allow(dead_code)] // LR-15 reads it.
    pub max_actions: Option<usize>,
    /// At most this many bytes of draft (LR-15).
    #[allow(dead_code)] // LR-15 reads it.
    pub max_bytes: Option<usize>,
    /// Rows anchored before this tick are dropped (the rolling window's
    /// first tick, LR-15).
    pub from_tick: Option<u32>,
}

/// `account = "gm"` and `character = "lab"`, so a transform with no named
/// section still writes a `[section]` the validator accepts.
impl Default for TransformOptions {
    fn default() -> Self {
        TransformOptions {
            base: None,
            account: "gm".into(),
            character: "lab".into(),
            character_expr: None,
            profile: None,
            max_actions: None,
            max_bytes: None,
            from_tick: None,
        }
    }
}

/// A draft spec, ready to write.
#[derive(Debug, Clone, PartialEq)]
pub struct Draft {
    /// `<section>-<yyyyMMdd-HHmm>.toml` ([`draft_file_name`]).
    pub file_name: String,
    /// The TOML, ending in one `\n`; it parses with [`spec::parse`].
    pub text: String,
    /// Runnable rows (not the commented ones).
    pub rows: usize,
    /// Setup and step actions in the runnable rows.
    pub actions: usize,
    /// `# TODO` lines: header faults, row and fixture lines, and one per
    /// commented row.
    pub todos: usize,
    /// Rows left out by `from_tick`.
    pub dropped_rows: usize,
}

/// Turn a recording into a draft spec. Pure: the same log and options give
/// the same bytes. An `Err` names why the draft would not load.
pub fn transform(log: &RecordLog, opts: &TransformOptions) -> Result<Draft, String> {
    let character_sql = opts.character_expr.as_deref().unwrap_or("${character}");
    let mut text = header(log);
    text.push_str(&section(log, opts));
    text.push_str(&profile_table(opts)?);

    let mut todos = 0;
    for fault in faults(log) {
        text.push_str(&format!("# TODO {fault}\n"));
        todos += 1;
    }

    let recorded = segment(log);
    let kept: Vec<_> = recorded
        .iter()
        .filter(|r| opts.from_tick.is_none_or(|from| r.anchor.tick >= from))
        .collect();
    let dropped_rows = recorded.len() - kept.len();

    let (mut rows, mut actions) = (0, 0);
    for (n, recorded_row) in kept.into_iter().enumerate() {
        let id = format!("REC-{:02}", n + 1);
        let d = draft_row(log, recorded_row, &id, character_sql);
        let mut body = render_row(&d);
        let (fixture, fixture_todos) =
            fixture::row_block(log, recorded_row, opts.profile.as_deref());
        for line in fixture {
            body.push_str(&line);
            body.push('\n');
        }
        text.push('\n');
        match &d.blocked {
            None => {
                text.push_str(&body);
                rows += 1;
                actions += d.actions();
                todos += d.todos() + fixture_todos;
            }
            Some(reason) => {
                text.push_str(&format!("# TODO {id} not runnable: {}\n", one_line(reason)));
                for line in body.lines() {
                    text.push_str(format!("# {line}").trim_end());
                    text.push('\n');
                }
                todos += 1;
            }
        }
    }

    spec::parse(&text).map_err(|e| format!("the draft does not parse: {e}"))?;
    Ok(Draft {
        file_name: draft_file_name(&log.start),
        text,
        rows,
        actions,
        todos,
        dropped_rows,
    })
}

/// `<section>-<yyyyMMdd-HHmm>.toml` from the start line's recording id.
pub fn draft_file_name(start: &RecordStart) -> String {
    format!(
        "{}-{}.toml",
        slug(&start.section),
        stamp(&start.recording_id)
    )
}

/// One line of a `setup` or `step` array.
#[derive(Debug, Clone, PartialEq)]
enum Line {
    /// An inline action table.
    Action(Inline),
    /// A `# TODO` comment, without the `# TODO ` prefix.
    Todo(String),
}

/// A row after the mapping table, before rendering.
#[derive(Debug, Clone)]
struct RowDraft {
    id: String,
    title: String,
    notes: String,
    poses: Vec<PoseSpec>,
    setup: Vec<Line>,
    steps: Vec<Line>,
    clauses: Vec<ClauseOut>,
    /// Why the row cannot run; it is then written as comments.
    blocked: Option<String>,
}

impl RowDraft {
    /// Setup and step actions.
    fn actions(&self) -> usize {
        self.lines()
            .filter(|l| matches!(l, Line::Action(_)))
            .count()
    }

    /// `# TODO` lines.
    fn todos(&self) -> usize {
        self.lines().filter(|l| matches!(l, Line::Todo(_))).count()
    }

    fn lines(&self) -> impl Iterator<Item = &Line> {
        self.setup.iter().chain(&self.steps)
    }

    /// Keep the first reason a row cannot run.
    fn block(&mut self, reason: impl Into<String>) {
        self.blocked.get_or_insert_with(|| reason.into());
    }
}

/// The row as TOML, from `[[row]]` to its last clause, ending in `\n`:
/// each pose and action on one line, TODOs as comment lines in their array.
fn render_row(d: &RowDraft) -> String {
    let string = |v: &str| Inline::Str(v.to_string()).render();
    let expected = if d.clauses.is_empty() {
        "Nothing was recorded to check.".to_string()
    } else {
        let texts: Vec<&str> = d.clauses.iter().map(|c| c.text.as_str()).collect();
        format!("{}.", texts.join("; "))
    };
    let mut out = String::from("[[row]]\n");
    for (k, v) in [
        ("id", d.id.as_str()),
        ("title", &d.title),
        ("expected", &expected),
        ("state", "any"),
        ("notes", &d.notes),
    ] {
        out.push_str(&format!("{k} = {}\n", string(v)));
    }
    if !d.poses.is_empty() {
        out.push_str("pose = [\n");
        for p in &d.poses {
            out.push_str(&format!("  {},\n", p.to_inline_toml()));
        }
        out.push_str("]\n");
    }
    for (key, lines) in [("setup", &d.setup), ("step", &d.steps)] {
        if lines.is_empty() && key == "setup" {
            continue;
        }
        out.push_str(&format!("{key} = [\n"));
        for l in lines {
            match l {
                Line::Action(a) => out.push_str(&format!("  {},\n", a.render())),
                Line::Todo(t) => out.push_str(&format!("  # TODO {t}\n")),
            }
        }
        out.push_str("]\n");
    }
    for c in &d.clauses {
        out.push_str("[[row.expect]]\n");
        out.push_str(&format!("id = {}\n", string(&c.id)));
        out.push_str(&format!("text = {}\n", string(&c.text)));
        for (k, v) in &c.fields {
            out.push_str(&format!("{k} = {}\n", v.render()));
        }
    }
    out
}

/// The UNCONFIRMED banner (D-LR12), the recording line, and the schema.
fn header(log: &RecordLog) -> String {
    let s = &log.start;
    format!(
        "# Recorded draft: UNCONFIRMED until `lab golden record` agrees on it 5 times\n\
         # (docs/analysis/lab-record/README.md, D-LR12).\n\
         # recording {} on {}, {}, {} ticks, transform {TRANSFORM_VERSION}\n\
         schema = 1\n",
        one_line(&s.recording_id),
        one_line(&s.instance),
        one_line(&s.date),
        log.ticks.len(),
    )
}

/// `[section]`: the draft's own id, the rest from the named section when
/// there is one, else the recorder's defaults and the fallbacks.
fn section(log: &RecordLog, opts: &TransformOptions) -> String {
    let id = format!(
        "rec-{}-{}",
        slug(&log.start.section),
        stamp(&log.start.recording_id)
    );
    let (system, guide, ledger, account, character) = match &opts.base {
        Some(b) => (
            b.system.as_str(),
            b.guide.as_str(),
            b.ledger.as_str(),
            b.account.as_str(),
            b.character.as_str(),
        ),
        None => (
            "recorded",
            "docs/guides/automated-uat.md",
            "docs/analysis/lab-record/README.md",
            opts.account.as_str(),
            opts.character.as_str(),
        ),
    };
    // A fresh character is made per run and named from the run id; a
    // recording plays one existing character, so it is never copied.
    let character = if character == "fresh" {
        "lab"
    } else {
        character
    };
    let mut out = String::from("\n[section]\n");
    for (k, v) in [
        ("id", id.as_str()),
        ("system", system),
        ("guide", guide),
        ("ledger", ledger),
        ("account", account),
        ("character", character),
    ] {
        out.push_str(&format!("{k} = {}\n", Inline::Str(v.to_string()).render()));
    }
    out
}

/// `[section.profiles.<profile>]` copied from the named section, so the
/// rows' `[row.fixture]` names a profile the validator finds; empty with no
/// profile. A profile the named section does not define is an error: the
/// draft would not load. So is a profile with no named section, and a
/// profile beside `character_expr`: the runner binds `${character}` to the
/// fixture's character, so a second expression would grade another name.
fn profile_table(opts: &TransformOptions) -> Result<String, String> {
    let Some(name) = &opts.profile else {
        return Ok(String::new());
    };
    if opts.character_expr.is_some() {
        return Err(
            "-Profile and -CharacterExpr exclude each other: the fixture's character is ${character}"
                .into(),
        );
    }
    let Some(base) = &opts.base else {
        return Err("-Profile needs a named section to copy the profile from".into());
    };
    let Some(p) = base.profiles.get(name) else {
        return Err(format!(
            "profile {name:?} is not a [section.profiles] key of the named section"
        ));
    };
    let mut out = format!("\n[section.profiles.{}]\n", key(name));
    for (k, v) in [
        ("alignment", &p.alignment),
        ("archetype", &p.archetype),
        ("gender", &p.gender),
        ("first", &p.first),
    ] {
        out.push_str(&format!("{k} = {}\n", Inline::Str(v.clone()).render()));
    }
    Ok(out)
}

/// What went wrong while recording, for the header's TODO lines.
fn faults(log: &RecordLog) -> Vec<String> {
    let mut out = Vec::new();
    let dropped: u64 = log.ticks.iter().map(|t| t.tap_dropped).sum();
    if dropped > 0 {
        out.push(format!("tap dropped {dropped} messages"));
    }
    if !log.has_stop() {
        out.push("recording has no stop line".into());
    }
    let ui = log.ticks.iter().filter(|t| t.windows.is_none()).count();
    if ui > 0 {
        out.push(format!("{ui} UI read failures"));
    }
    out
}

/// `yyyyMMdd-HHmm` from a `yyyyMMdd-HHmmss-<instance>` recording id; any
/// other id as a [`slug`].
fn stamp(recording_id: &str) -> String {
    let b = recording_id.as_bytes();
    let shaped = b.len() >= 13
        && b[..8].iter().all(u8::is_ascii_digit)
        && b[8] == b'-'
        && b[9..13].iter().all(u8::is_ascii_digit);
    if shaped {
        recording_id[..13].to_string()
    } else {
        slug(recording_id)
    }
}

/// Lower-case `[a-z0-9-]`, anything else a `-`: safe in a file name and a
/// section id.
fn slug(s: &str) -> String {
    s.chars()
        .map(|c| match c.to_ascii_lowercase() {
            c @ ('a'..='z' | '0'..='9' | '-') => c,
            _ => '-',
        })
        .collect()
}

/// Control characters as spaces, so a recorded string stays on its comment line.
fn one_line(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect()
}

#[cfg(test)]
#[path = "emit_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "emit_mapping_tests.rs"]
mod mapping_tests;

#[cfg(test)]
#[path = "emit_click_tests.rs"]
mod click_tests;
