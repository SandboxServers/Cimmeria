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

/// One event macro call: its fields in order.
#[derive(Debug)]
pub(super) struct Call {
    pub(super) fields: Vec<Field>,
}

fn is_ident(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

fn line_at(code: &str, at: usize) -> usize {
    code[..at].matches('\n').count() + 1
}

pub(super) fn event_calls(src: &str, masked: &Masked) -> Vec<Call> {
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
        if preceded || !EVENT_MACROS.contains(&&code[start..i]) {
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
        out.push(Call {
            fields: fields(src, code, open + 1, close),
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
fn fields(src: &str, code: &str, from: usize, to: usize) -> Vec<Field> {
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
            out.extend(fields(src, code, inner_from, inner_from + inner.len()));
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
