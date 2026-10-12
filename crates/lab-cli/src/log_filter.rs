//! The daemon log filter behind `lab logs`: the instance and level filters,
//! continuation lines kept with their entry, and token masking.

use std::sync::OnceLock;

use regex::Regex;

/// The level words, lowest first. A line's level is its second
/// whitespace-separated field, compared ignoring case.
pub const LEVELS: [&str; 5] = ["trace", "debug", "info", "warn", "error"];

/// The rank of a level word (`trace` 0 .. `error` 4), ignoring case.
pub fn level_rank(word: &str) -> Option<u8> {
    let word = word.to_ascii_lowercase();
    LEVELS
        .iter()
        .position(|level| *level == word)
        .map(|rank| rank as u8)
}

fn entry_start() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^[0-9]{4}-[0-9]{2}-[0-9]{2}T").expect("valid regex"))
}

fn bearer() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)bearer\s+\S+").expect("valid regex"))
}

fn hex64() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)\b[0-9a-f]{64}\b").expect("valid regex"))
}

/// Whether `line` starts a log entry: it begins with `yyyy-MM-ddT`. Every
/// other line (a panic body, a backtrace) is a continuation line.
pub fn is_entry_start(line: &str) -> bool {
    entry_start().is_match(line)
}

/// `line` with a bearer token and any 64-hex token replaced by `<redacted>`
/// (the bearer pattern first, then the hex one).
pub fn redact(line: &str) -> String {
    let masked = bearer().replace_all(line, "<redacted>");
    hex64().replace_all(&masked, "<redacted>").into_owned()
}

/// The filter state: feed it every line of the log in order.
pub struct LogFilter {
    instance: Option<Regex>,
    min_rank: Option<u8>,
    keep: bool,
}

impl LogFilter {
    /// `instance` keeps entries that name it (`instance="x"` or
    /// `instance=Some("x")`, ignoring case, the whole name). `level` keeps
    /// entries at or above it. An empty string or an unknown level word is no
    /// filter. With no filter every line is kept, even before the first entry.
    pub fn new(instance: Option<&str>, level: Option<&str>) -> Self {
        let instance = instance.filter(|name| !name.is_empty()).map(|name| {
            let pattern = format!(r#"(?i)instance=(?:Some\()?"{}""#, regex::escape(name));
            Regex::new(&pattern).expect("escaped instance name is a valid regex")
        });
        let min_rank = level.filter(|word| !word.is_empty()).and_then(level_rank);
        // Before the first entry a continuation line has no entry to follow,
        // so with a filter on it is dropped.
        let keep = instance.is_none() && min_rank.is_none();
        Self {
            instance,
            min_rank,
            keep,
        }
    }

    /// `Some(masked line)` when `line` passes, else `None`. A line that is not
    /// an entry start is kept or dropped with the entry above it, so a level
    /// word inside a continuation line never ranks it on its own.
    pub fn accept(&mut self, line: &str) -> Option<String> {
        if is_entry_start(line) {
            self.keep = true;
            if let Some(instance) = &self.instance {
                if !instance.is_match(line) {
                    self.keep = false;
                }
            }
            if self.keep {
                if let Some(min) = self.min_rank {
                    let rank = line.split_whitespace().nth(1).and_then(level_rank);
                    match rank {
                        Some(rank) if rank >= min => {}
                        _ => self.keep = false,
                    }
                }
            }
        }
        if self.keep {
            Some(redact(line))
        } else {
            None
        }
    }
}

/// The lines of `lines` that pass the filters, masked.
pub fn filter_lines<'a>(
    lines: impl IntoIterator<Item = &'a str>,
    instance: Option<&str>,
    level: Option<&str>,
) -> Vec<String> {
    let mut filter = LogFilter::new(instance, level);
    lines
        .into_iter()
        .filter_map(|line| filter.accept(line))
        .collect()
}

/// The last `count` lines of `text` that pass the filters, masked (what a
/// plain `lab logs` prints).
pub fn tail_lines(
    text: &str,
    instance: Option<&str>,
    level: Option<&str>,
    count: usize,
) -> Vec<String> {
    let mut kept = filter_lines(text.lines(), instance, level);
    let skip = kept.len().saturating_sub(count);
    kept.drain(..skip);
    kept
}
