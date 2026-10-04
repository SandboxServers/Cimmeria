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
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(i) = code[from..].find("!(") {
        let bang = from + i;
        from = bang + 2;
        let name_start = code[..bang]
            .rfind(|c: char| !is_ident(c))
            .map_or(0, |p| p + 1);
        if !EVENT_MACROS.contains(&&code[name_start..bang]) {
            continue;
        }
        let Some(close) = matching_paren(code, bang + 1) else {
            continue;
        };
        out.push(Call {
            fields: fields(src, code, bang + 2, close),
        });
    }
    out
}

/// Index of the `)` closing the `(` at `open`. Literals are already blank.
fn matching_paren(code: &str, open: usize) -> Option<usize> {
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
        let raw_literal = trimmed
            .strip_prefix('r')
            .is_some_and(|r| r.trim_start_matches('#').starts_with('"'));
        if trimmed.starts_with('"') || raw_literal {
            // A string-literal key (`"a.b" = x`) or the message. The mask
            // blanked the literal, so read it from the source.
            let len = trimmed[1..].find('"').map_or(trimmed.len(), |p| p + 2);
            if trimmed.starts_with('"') && is_assignment(&trimmed[len..]) {
                out.push(Field {
                    key: src[at + 1..at + len - 1].to_string(),
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
