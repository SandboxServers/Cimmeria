//! A small Rust lexer: enough to find items, attributes and macro calls
//! without pulling `syn` and `proc-macro2` (with `span-locations`, which
//! would change the workspace-hack) into every test build.
//!
//! Comments are dropped, so a `require_db_or_skip!` in a doc example is
//! not a call. String contents are kept only for plain `"..."` literals,
//! which is where `#[path = "..."]` puts its file name.

/// One token, with the 1-based line it starts on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Tok {
    Ident(String),
    Punct(char),
    /// A string literal's contents (escapes left as written).
    Str(String),
    /// Any other literal: number, char, byte, raw or byte string.
    Lit,
}

#[derive(Debug, Clone)]
pub(crate) struct Token {
    pub(crate) tok: Tok,
    pub(crate) line: usize,
}

pub(crate) fn lex(src: &str) -> Vec<Token> {
    let c: Vec<char> = src.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    let mut line = 1;
    let at = |i: usize| c.get(i).copied().unwrap_or('\0');
    while i < c.len() {
        let ch = c[i];
        let start_line = line;
        if ch == '\n' {
            line += 1;
            i += 1;
        } else if ch.is_whitespace() {
            i += 1;
        } else if ch == '/' && at(i + 1) == '/' {
            while i < c.len() && c[i] != '\n' {
                i += 1;
            }
        } else if ch == '/' && at(i + 1) == '*' {
            let mut depth = 0;
            while i < c.len() {
                if c[i] == '/' && at(i + 1) == '*' {
                    depth += 1;
                    i += 2;
                } else if c[i] == '*' && at(i + 1) == '/' {
                    depth -= 1;
                    i += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    if c[i] == '\n' {
                        line += 1;
                    }
                    i += 1;
                }
            }
        } else if ch == '"' {
            let (end, text, nl) = quoted(&c, i + 1);
            line += nl;
            i = end;
            out.push(Token {
                tok: Tok::Str(text),
                line: start_line,
            });
        } else if let Some(end) = raw_or_prefixed_literal(&c, i) {
            line += c[i..end].iter().filter(|&&x| x == '\n').count();
            i = end;
            out.push(Token {
                tok: Tok::Lit,
                line: start_line,
            });
        } else if ch == '\'' {
            // A char literal ('a', '\n', '{') or a lifetime ('a, 'static).
            if at(i + 1) == '\\' {
                i += 2;
                while i < c.len() && c[i] != '\'' {
                    i += 1;
                }
                i += 1;
                out.push(Token {
                    tok: Tok::Lit,
                    line: start_line,
                });
            } else if at(i + 2) == '\'' {
                i += 3;
                out.push(Token {
                    tok: Tok::Lit,
                    line: start_line,
                });
            } else {
                i += 1;
                while i < c.len() && is_ident_char(c[i]) {
                    i += 1;
                }
            }
        } else if ch.is_ascii_digit() {
            while i < c.len() && is_ident_char(c[i]) {
                i += 1;
            }
            out.push(Token {
                tok: Tok::Lit,
                line: start_line,
            });
        } else if is_ident_start(ch) {
            if ch == 'r' && at(i + 1) == '#' && is_ident_start(at(i + 2)) {
                i += 2; // raw identifier: r#type is the ident `type`
            }
            let s = i;
            while i < c.len() && is_ident_char(c[i]) {
                i += 1;
            }
            out.push(Token {
                tok: Tok::Ident(c[s..i].iter().collect()),
                line: start_line,
            });
        } else {
            out.push(Token {
                tok: Tok::Punct(ch),
                line: start_line,
            });
            i += 1;
        }
    }
    out
}

fn is_ident_start(ch: char) -> bool {
    ch == '_' || ch.is_alphabetic()
}

fn is_ident_char(ch: char) -> bool {
    ch == '_' || ch.is_alphanumeric()
}

/// Scan a `"..."` body starting just after the opening quote. Returns the
/// index after the closing quote, the raw contents, and the newline count.
fn quoted(c: &[char], mut i: usize) -> (usize, String, usize) {
    let s = i;
    let mut nl = 0;
    while i < c.len() && c[i] != '"' {
        if c[i] == '\\' {
            i += 1;
        }
        if c.get(i) == Some(&'\n') {
            nl += 1;
        }
        i += 1;
    }
    (i + 1, c[s..i.min(c.len())].iter().collect(), nl)
}

/// Raw strings (`r"..."`, `r#"..."#`), byte and C strings (`b"..."`,
/// `br#"..."#`, `c"..."`) and byte chars (`b'x'`) starting at `i`: the
/// index just past the literal, or `None` if `i` starts none of them.
fn raw_or_prefixed_literal(c: &[char], i: usize) -> Option<usize> {
    let at = |j: usize| c.get(j).copied().unwrap_or('\0');
    // Only a literal prefix when it is not the tail of a longer identifier.
    if i > 0 && is_ident_char(c[i - 1]) {
        return None;
    }
    let mut j = i;
    match at(j) {
        'b' | 'c' => {
            j += 1;
            if at(i) == 'b' && at(j) == '\'' {
                j += 1;
                if at(j) == '\\' {
                    j += 1;
                }
                j += 1;
                while j < c.len() && c[j] != '\'' {
                    j += 1;
                }
                return Some(j + 1);
            }
            if at(j) == '"' {
                return Some(quoted(c, j + 1).0);
            }
            if at(j) != 'r' {
                return None;
            }
            j += 1;
        }
        'r' => j += 1,
        _ => return None,
    }
    // Raw string: r, then zero or more #, then ".
    let mut hashes = 0;
    while at(j) == '#' {
        hashes += 1;
        j += 1;
    }
    if at(j) != '"' {
        return None;
    }
    j += 1;
    loop {
        if j >= c.len() {
            return Some(j);
        }
        if c[j] == '"' && (1..=hashes).all(|k| at(j + k) == '#') {
            return Some(j + 1 + hashes);
        }
        j += 1;
    }
}
