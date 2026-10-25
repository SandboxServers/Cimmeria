//! A reader for the part of Valve's text KeyValues format that Steam and Proton
//! write: quoted `"key" "value"` pairs, nested blocks and `//` comments.
//! Nesting is not modelled; blocks only mark where a pair cannot be.

use std::borrow::Cow;

/// One lexical item of a KeyValues file.
enum Token<'a> {
    Str(Cow<'a, str>),
    Open,
    Close,
}

/// Every quoted value whose quoted key equals `key`, at any depth, in file order.
pub(crate) fn values<'a>(text: &'a str, key: &str) -> Vec<Cow<'a, str>> {
    let tokens = tokens(text);
    let mut found = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        match &tokens[i..] {
            [Token::Str(k), Token::Str(v), ..] => {
                if k == key {
                    found.push(v.clone());
                }
                i += 2;
            }
            _ => i += 1,
        }
    }
    found
}

fn tokens(text: &str) -> Vec<Token<'_>> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => {
                out.push(Token::Open);
                i += 1;
            }
            b'}' => {
                out.push(Token::Close);
                i += 1;
            }
            // A comment runs to the end of its line.
            b'/' if bytes.get(i + 1) == Some(&b'/') => {
                i = match text[i..].find('\n') {
                    Some(newline) => i + newline,
                    None => bytes.len(),
                };
            }
            b'"' => {
                let (value, next) = quoted(text, i + 1);
                out.push(Token::Str(value));
                i = next;
            }
            _ => i += 1,
        }
    }
    out
}

/// Reads the body of a quoted string that starts at `start`. Returns the value
/// and the index just after the closing quote.
fn quoted(text: &str, start: usize) -> (Cow<'_, str>, usize) {
    let bytes = text.as_bytes();
    let mut end = start;
    let mut escaped = false;
    while end < bytes.len() && (escaped || bytes[end] != b'"') {
        escaped = !escaped && bytes[end] == b'\\';
        end += 1;
    }
    let body = &text[start..end];
    let value = if body.contains('\\') {
        Cow::Owned(unescape(body))
    } else {
        Cow::Borrowed(body)
    };
    (value, (end + 1).min(bytes.len()))
}

/// Turns `\\` into `\` and `\"` into `"`; any other backslash is kept.
fn unescape(body: &str) -> String {
    let mut out = String::with_capacity(body.len());
    let mut escaped = false;
    for c in body.chars() {
        if c == '\\' && !escaped {
            escaped = true;
            continue;
        }
        if escaped && c != '\\' && c != '"' {
            out.push('\\');
        }
        out.push(c);
        escaped = false;
    }
    if escaped {
        out.push('\\');
    }
    out
}
