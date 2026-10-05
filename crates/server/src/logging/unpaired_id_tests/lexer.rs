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

/// Blanks everything gated to test builds in a masked source: anything under
/// `#[cfg(test)]`, `#[cfg(any(test, …))]` or `#[cfg(all(test, …))]` (but not
/// `not(test)`). That covers a `mod tests { … }`, a test-only fn or impl, a
/// `mod tests;` declaration, a struct field, an enum variant, a match arm or
/// a statement. Only the gated part goes, so production code after a test
/// module is still scanned, and line numbers stay put.
pub(super) fn blank_test_items(code: &mut String) {
    let mut bytes = std::mem::take(code).into_bytes();
    let mut from = 0;
    while let Some(i) = find(&bytes[from..], b"#[cfg(") {
        let start = from + i;
        let Some(attr_end) = close_of(&bytes, start + 1) else {
            break;
        };
        from = attr_end + 1;
        if !is_test_cfg(&bytes[start + 6..attr_end]) {
            continue;
        }
        let mut j = attr_end + 1;
        // Further attributes on the same item.
        loop {
            while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                j += 1;
            }
            if bytes[j..].starts_with(b"#[") {
                j = close_of(&bytes, j + 1).map_or(bytes.len(), |c| c + 1);
            } else {
                break;
            }
        }
        let end = gated_end(&bytes, j);
        for b in &mut bytes[start..end] {
            if *b != b'\n' {
                *b = b' ';
            }
        }
        from = end.max(from);
    }
    *code = String::from_utf8(bytes).expect("blanking keeps UTF-8");
}

/// The cfg predicate (the text inside `cfg(…)`) names `test` and never
/// negates anything.
fn is_test_cfg(pred: &[u8]) -> bool {
    let pred = String::from_utf8_lossy(pred);
    let words: Vec<&str> = pred
        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .collect();
    words.contains(&"test") && !words.contains(&"not")
}

const ITEM_KEYWORDS: &[&str] = &[
    "fn",
    "mod",
    "impl",
    "struct",
    "enum",
    "use",
    "const",
    "static",
    "type",
    "trait",
    "pub",
    "async",
    "unsafe",
    "extern",
    "macro_rules",
    "union",
];

/// Where the gated thing that starts at `j` ends (exclusive). An item ends at
/// its first top-level `;` or with the block its first top-level `{` opens.
/// A field, variant, arm or parameter also ends at a top-level `,`, and
/// anything ends before a closing bracket it didn't open.
fn gated_end(bytes: &[u8], j: usize) -> usize {
    let word_len = bytes[j..]
        .iter()
        .position(|&b| !(b.is_ascii_alphanumeric() || b == b'_'))
        .unwrap_or(bytes.len() - j);
    let word = std::str::from_utf8(&bytes[j..j + word_len]).unwrap_or("");
    let is_item = ITEM_KEYWORDS.contains(&word);
    let mut depth = 0usize;
    for (k, &b) in bytes.iter().enumerate().skip(j) {
        match b {
            b'(' | b'[' => depth += 1,
            b'{' if depth == 0 => return close_of(bytes, k).map_or(bytes.len(), |c| c + 1),
            b'{' => depth += 1,
            b')' | b']' | b'}' if depth == 0 => return k,
            b')' | b']' | b'}' => depth -= 1,
            b';' if depth == 0 => return k + 1,
            b',' if depth == 0 && !is_item => return k + 1,
            _ => {}
        }
    }
    bytes.len()
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

/// Index of the bracket closing the one at `open`, in masked code.
fn close_of(bytes: &[u8], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (i, b) in bytes.iter().enumerate().skip(open) {
        match b {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}
