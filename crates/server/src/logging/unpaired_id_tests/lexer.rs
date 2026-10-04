//! Masks a Rust source file so the call scanner sees only code.
//!
//! Comments and the insides of string and char literals become spaces, so a
//! `)` or `,` in a message or a commented-out `info!(` never reaches the call
//! parser. Byte offsets and newlines are kept, so an offset in the mask is the
//! same offset in the source. Line comments are collected on the way, because
//! the `// nt:id-only` exemption lives in one.

/// The masked source plus every `//` comment, keyed by its 1-based line.
pub(super) struct Masked {
    pub(super) code: String,
    pub(super) line_comments: Vec<(usize, String)>,
}

fn is_ident(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

pub(super) fn mask(src: &str) -> Masked {
    let s = src.as_bytes();
    let mut out = s.to_vec();
    let mut line_comments = Vec::new();
    let mut line = 1;
    let blank = |out: &mut Vec<u8>, from: usize, to: usize| {
        for b in &mut out[from..to] {
            if *b != b'\n' {
                *b = b' ';
            }
        }
    };
    let mut i = 0;
    while i < s.len() {
        let b = s[i];
        let next = s.get(i + 1).copied();
        if b == b'\n' {
            line += 1;
            i += 1;
        } else if b == b'/' && next == Some(b'/') {
            let end = s[i..]
                .iter()
                .position(|&c| c == b'\n')
                .map_or(s.len(), |p| i + p);
            line_comments.push((line, src[i + 2..end].trim_end_matches('\r').to_string()));
            blank(&mut out, i, end);
            i = end;
        } else if b == b'/' && next == Some(b'*') {
            let mut depth = 1;
            let mut j = i + 2;
            while j < s.len() && depth > 0 {
                if s[j] == b'/' && s.get(j + 1) == Some(&b'*') {
                    depth += 1;
                    j += 2;
                } else if s[j] == b'*' && s.get(j + 1) == Some(&b'/') {
                    depth -= 1;
                    j += 2;
                } else {
                    line += usize::from(s[j] == b'\n');
                    j += 1;
                }
            }
            blank(&mut out, i, j);
            i = j;
        } else if let Some((open, hashes)) = raw_string_start(s, i) {
            // r"…", r#"…"#, br"…": no escapes, ends at `"` plus the same hashes.
            let mut j = open + 1;
            let end = loop {
                if j >= s.len() {
                    break s.len();
                }
                if s[j] == b'"'
                    && s[j + 1..]
                        .iter()
                        .take(hashes)
                        .filter(|&&c| c == b'#')
                        .count()
                        == hashes
                {
                    break j;
                }
                line += usize::from(s[j] == b'\n');
                j += 1;
            };
            blank(&mut out, open + 1, end);
            i = (end + 1 + hashes).min(s.len());
        } else if b == b'"' {
            let mut j = i + 1;
            while j < s.len() && s[j] != b'"' {
                if s[j] == b'\\' {
                    j += 1;
                }
                line += usize::from(s.get(j) == Some(&b'\n'));
                j += 1;
            }
            blank(&mut out, i + 1, j.min(s.len()));
            i = j + 1;
        } else if b == b'\'' {
            i = char_literal_end(src, i).map_or(i + 1, |end| {
                blank(&mut out, i + 1, end);
                end + 1
            });
        } else {
            i += 1;
        }
    }
    Masked {
        // Only ASCII bytes were written, over whole literals and comments, so
        // every multi-byte character is either intact or fully blanked.
        code: String::from_utf8(out).expect("masking keeps UTF-8"),
        line_comments,
    }
}

/// `Some((index of the opening quote, hash count))` when a raw string starts
/// at `i` (`r"`, `r#"`, `br"`, `br#"`), and `r`/`br` is not the tail of an
/// identifier.
fn raw_string_start(s: &[u8], i: usize) -> Option<(usize, usize)> {
    if i > 0 && is_ident(s[i - 1]) {
        return None;
    }
    let mut j = i;
    if s.get(j) == Some(&b'b') {
        j += 1;
    }
    if s.get(j) != Some(&b'r') {
        return None;
    }
    j += 1;
    let hashes = s[j..].iter().take_while(|&&c| c == b'#').count();
    (s.get(j + hashes) == Some(&b'"')).then_some((j + hashes, hashes))
}

/// The index of the closing `'` when a char literal opens at `i`; `None` for
/// a lifetime or label (`'a`, `'static`).
fn char_literal_end(src: &str, i: usize) -> Option<usize> {
    let rest = &src[i + 1..];
    if rest.starts_with('\\') {
        // '\n', '\'', '\u{1F600}': the first unescaped quote closes it.
        let skip = 2 + rest[2..].find('\'')?;
        return Some(i + 1 + skip);
    }
    let c = rest.chars().next()?;
    let after = i + 1 + c.len_utf8();
    (src.as_bytes().get(after) == Some(&b'\'')).then_some(after)
}
