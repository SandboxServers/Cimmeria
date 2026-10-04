//! Finds every tracing event macro call in a masked source and reads the
//! field keys it records.
//!
//! Event macros only: `trace!`, `debug!`, `info!`, `warn!`, `error!` and
//! `event!`, with or without a `tracing::` path. Span constructors
//! (`info_span!`, `#[instrument]`) are outside Rule 6, and their names never
//! match here.

use super::lexer::Masked;

const EVENT_MACROS: &[&str] = &["trace", "debug", "info", "warn", "error", "event"];

/// One field of one call: the key as tracing records it (`a.b` for a dotted
/// key) and the 1-based line the key starts on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Field {
    pub(super) key: String,
    pub(super) line: usize,
}

/// One macro call: where its name starts, its fields in order, and the line
/// of the first argument that forwards a `$( … )` repetition (a
/// `macro_rules!` wrapper, whose remaining fields are at its call sites).
#[derive(Debug)]
pub(super) struct Call {
    pub(super) at: usize,
    pub(super) fields: Vec<Field>,
    pub(super) forwards_at: Option<usize>,
}

fn is_ident(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

fn line_at(code: &str, at: usize) -> usize {
    code[..at].matches('\n').count() + 1
}

pub(super) fn event_calls(src: &str, masked: &Masked) -> Vec<Call> {
    macro_calls(src, masked, EVENT_MACROS)
}

/// Every call of a macro named in `names`, parsed like an event macro.
pub(super) fn macro_calls(src: &str, masked: &Masked, names: &[&str]) -> Vec<Call> {
    let code = masked.code.as_str();
    let bytes = code.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if !is_ident(char::from(bytes[i])) {
            i += 1;
            continue;
        }
        let start = i;
        while i < bytes.len() && is_ident(char::from(bytes[i])) {
            i += 1;
        }
        let preceded = start > 0 && is_ident(char::from(bytes[start - 1]));
        if preceded || !names.contains(&&code[start..i]) {
            continue;
        }
        // `info!(…)`, `info! {…}`, `info![…]`: whitespace (and masked
        // comments) may sit on either side of the `!`.
        let rest = code[i..].trim_start();
        let Some(rest) = rest.strip_prefix('!') else {
            continue;
        };
        let open = code.len() - rest.trim_start().len();
        if !matches!(bytes.get(open), Some(b'(' | b'[' | b'{')) {
            continue;
        }
        let Some(close) = matching_delimiter(code, open) else {
            continue;
        };
        let mut forwards_at = None;
        out.push(Call {
            at: start,
            fields: fields(src, code, open + 1, close, &mut forwards_at),
            forwards_at,
        });
        i = open + 1;
    }
    out
}

/// Index of the delimiter closing the one at `open`. Literals are already
/// blank, so every bracket left is code.
fn matching_delimiter(code: &str, open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (i, b) in code.bytes().enumerate().skip(open) {
        match b {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

/// The arguments between `from` and `to`, split at top-level commas, as
/// `(start offset, masked text)`.
fn arguments(code: &str, from: usize, to: usize) -> Vec<(usize, &str)> {
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut start = from;
    for (i, b) in code[from..to].bytes().enumerate() {
        match b {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth = depth.saturating_sub(1),
            b',' if depth == 0 => {
                out.push((start, &code[start..from + i]));
                start = from + i + 1;
            }
            _ => {}
        }
    }
    out.push((start, &code[start..to]));
    out
}

/// The fields of one call. Directives (`target:`, `parent:`, `name:`) and the
/// `Level` of `event!` are skipped; reading stops at the message literal,
/// because what follows it are format arguments, not fields.
fn fields(
    src: &str,
    code: &str,
    from: usize,
    to: usize,
    forwards_at: &mut Option<usize>,
) -> Vec<Field> {
    let mut out = Vec::new();
    for (start, arg) in arguments(code, from, to) {
        let trimmed = arg.trim_start();
        let at = start + (arg.len() - trimmed.len());
        if trimmed.is_empty() {
            continue;
        }
        if let Some(inner) = trimmed
            .strip_prefix('{')
            .and_then(|t| t.trim_end().strip_suffix('}'))
        {
            // A braced field set: `event!(Level::INFO, { a_id = x }, "m")`.
            let inner_from = at + 1;
            out.extend(fields(
                src,
                code,
                inner_from,
                inner_from + inner.len(),
                forwards_at,
            ));
            continue;
        }
        if let Some((open_len, close_len)) = literal_delimiters(trimmed) {
            // A string-literal key (`"a.b" = x`, `r#"a.b"# = x`) or the
            // message. The mask blanked the literal, so read the key from the
            // source; the mask kept the delimiters, so find the end there.
            let body = open_len
                + trimmed[open_len..]
                    .find('"')
                    .unwrap_or(trimmed.len() - open_len);
            let end = (body + close_len).min(trimmed.len());
            if is_assignment(&trimmed[end..]) {
                out.push(Field {
                    key: src[at + open_len..at + body].to_string(),
                    line: line_at(code, at),
                });
                continue;
            }
            break;
        }
        if trimmed.starts_with("$(") {
            forwards_at.get_or_insert(line_at(code, at));
            continue;
        }
        if let Some(key) = field_key(trimmed) {
            out.push(Field {
                key,
                line: line_at(code, at),
            });
        }
    }
    out
}

/// `(opening, closing)` delimiter lengths when `arg` starts with a string
/// literal: `"…"` is `(1, 1)`, `r##"…"##` is `(4, 3)`.
fn literal_delimiters(arg: &str) -> Option<(usize, usize)> {
    if arg.starts_with('"') {
        return Some((1, 1));
    }
    let hashes = arg.strip_prefix('r')?;
    let n = hashes.len() - hashes.trim_start_matches('#').len();
    hashes[n..].starts_with('"').then_some((n + 2, n + 1))
}

/// `= value` with a single `=` (not `==` or `=>`).
fn is_assignment(rest: &str) -> bool {
    let rest = rest.trim_start();
    rest.starts_with('=') && !rest.starts_with("==") && !rest.starts_with("=>")
}

/// The key of `key = v`, `key = %v`, `key = ?v`, `key`, `%key`, `?key` or
/// any of those with a dotted key; `None` for anything else (`Level::INFO`,
/// `target: "…"`, a `$($arg)*` repetition).
fn field_key(arg: &str) -> Option<String> {
    let sigil = arg.starts_with(['%', '?']);
    let body = if sigil { arg[1..].trim_start() } else { arg };
    let mut key = String::new();
    let mut rest = body;
    loop {
        let part = rest.strip_prefix("r#").unwrap_or(rest);
        let len = part.find(|c: char| !is_ident(c)).unwrap_or(part.len());
        if len == 0 || part.starts_with(|c: char| c.is_ascii_digit()) {
            return None;
        }
        key.push_str(&part[..len]);
        rest = &part[len..];
        match rest.strip_prefix('.') {
            Some(r) => {
                key.push('.');
                rest = r;
            }
            None => break,
        }
    }
    let rest = rest.trim();
    if rest.is_empty() || (!sigil && is_assignment(rest)) {
        Some(key)
    } else {
        None
    }
}

/// `(line, macro)` for every `use` of `tracing` that renames an event macro
/// (`use tracing::warn as twarn;`, `use tracing::{info as i, debug};`). A
/// renamed macro's calls are invisible to the scan, so the rename itself
/// fails the build.
pub(super) fn renamed_event_macros(code: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(i) = code[from..].find("use") {
        let at = from + i;
        from = at + 3;
        let bounded = !code[..at].ends_with(is_ident) && !code[from..].starts_with(is_ident);
        if !bounded {
            continue;
        }
        let end = code[at..].find(';').map_or(code.len(), |p| at + p);
        let words: Vec<&str> = code[at..end]
            .split(|c: char| !is_ident(c))
            .filter(|w| !w.is_empty())
            .collect();
        if !words.contains(&"tracing") {
            continue;
        }
        for pair in words.windows(2) {
            if EVENT_MACROS.contains(&pair[0]) && pair[1] == "as" {
                out.push((line_at(code, at), pair[0].to_string()));
            }
        }
    }
    out
}

/// `(name, body start, body end)` for every `macro_rules! name { … }` (or
/// `( … )` / `[ … ]`) definition in a masked source.
pub(super) fn macro_rules_bodies(code: &str) -> Vec<(String, usize, usize)> {
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(i) = code[from..].find("macro_rules!") {
        let after = from + i + "macro_rules!".len();
        from = after;
        let rest = code[after..].trim_start();
        let name_len = rest.find(|c: char| !is_ident(c)).unwrap_or(rest.len());
        let name = &rest[..name_len];
        let tail = rest[name_len..].trim_start();
        let open = code.len() - tail.len();
        if name.is_empty() || !matches!(code.as_bytes().get(open), Some(b'{' | b'(' | b'[')) {
            continue;
        }
        if let Some(close) = matching_delimiter(code, open) {
            out.push((name.to_string(), open, close));
        }
    }
    out
}
