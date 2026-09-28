//! Pull the items the guard needs out of one file's tokens: its `fn`s
//! (name, inline-module path, test attribute, gate calls, identifiers the
//! body mentions) and its out-of-line `mod name;` declarations.

use super::lex::{Tok, Token};

/// The live-DB gate macro. A call is this identifier followed by `!`.
pub(crate) const GATE: &str = "require_db_or_skip";

#[derive(Debug, Clone)]
pub(crate) struct FnItem {
    pub(crate) name: String,
    /// Inline `mod x { .. }` blocks between the file's module and the fn.
    pub(crate) inline_mods: Vec<String>,
    pub(crate) line: usize,
    /// Carries `#[test]`, `#[tokio::test]`, `#[rstest]` or `#[test_case]`.
    pub(crate) is_test: bool,
    /// Lines of the gate calls in this fn's own body (not nested fns').
    pub(crate) gate_lines: Vec<usize>,
    /// Every identifier in this fn's own body.
    pub(crate) idents: Vec<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct ModDecl {
    pub(crate) name: String,
    pub(crate) inline_mods: Vec<String>,
    /// The `#[path = "..."]` value, if any.
    pub(crate) path_attr: Option<String>,
}

#[derive(Debug, Default)]
pub(crate) struct FileItems {
    pub(crate) fns: Vec<FnItem>,
    pub(crate) mods: Vec<ModDecl>,
    /// Gate calls outside any fn body (inside a `macro_rules!`, say): the
    /// guard cannot name the test they end up in.
    pub(crate) stray_gate_lines: Vec<usize>,
}

enum Frame {
    Mod,
    Fn(usize),
    Other,
}

pub(crate) fn items(toks: &[Token]) -> FileItems {
    let mut out = FileItems::default();
    let mut frames: Vec<Frame> = Vec::new();
    let mut inline_mods: Vec<String> = Vec::new();
    let mut attrs: Vec<Vec<Tok>> = Vec::new();
    // What the next `{` opens, set by `mod x` / `fn x` headers.
    let mut pending: Option<Frame> = None;
    let mut pending_mod_name: Option<String> = None;

    let ident = |i: usize| match toks.get(i).map(|t| &t.tok) {
        Some(Tok::Ident(s)) => Some(s.as_str()),
        _ => None,
    };
    let punct =
        |i: usize, p: char| matches!(toks.get(i).map(|t| &t.tok), Some(Tok::Punct(q)) if *q == p);
    let current_fn = |frames: &[Frame]| {
        frames.iter().rev().find_map(|f| match f {
            Frame::Fn(ix) => Some(*ix),
            _ => None,
        })
    };

    let mut i = 0;
    while i < toks.len() {
        // Attributes: `#[...]` (outer, kept for the next item) and `#![...]`.
        if punct(i, '#') && (punct(i + 1, '[') || (punct(i + 1, '!') && punct(i + 2, '['))) {
            let inner = punct(i + 1, '!');
            let open = if inner { i + 2 } else { i + 1 };
            let close = matching(toks, open, '[', ']');
            if !inner {
                attrs.push(
                    toks[open + 1..close]
                        .iter()
                        .map(|t| t.tok.clone())
                        .collect(),
                );
            }
            i = close + 1;
            continue;
        }
        match &toks[i].tok {
            Tok::Ident(kw) if kw == "mod" && ident(i + 1).is_some() => {
                let name = ident(i + 1).unwrap().to_string();
                if punct(i + 2, ';') {
                    out.mods.push(ModDecl {
                        name,
                        inline_mods: inline_mods.clone(),
                        path_attr: path_attr(&attrs),
                    });
                    attrs.clear();
                    i += 3;
                    continue;
                }
                pending = Some(Frame::Mod);
                pending_mod_name = Some(name);
                attrs.clear();
                i += 2;
                continue;
            }
            Tok::Ident(kw) if kw == "fn" && ident(i + 1).is_some() => {
                // Find the body: the first `{` (or a `;` for a bodiless
                // declaration) outside the signature's parens and brackets.
                let mut j = i + 2;
                let mut depth = 0i32;
                while j < toks.len() {
                    match toks[j].tok {
                        Tok::Punct('(') | Tok::Punct('[') => depth += 1,
                        Tok::Punct(')') | Tok::Punct(']') => depth -= 1,
                        Tok::Punct('{') | Tok::Punct(';') if depth == 0 => break,
                        _ => {}
                    }
                    j += 1;
                }
                if punct(j, '{') {
                    out.fns.push(FnItem {
                        name: ident(i + 1).unwrap().to_string(),
                        inline_mods: inline_mods.clone(),
                        line: toks[i].line,
                        is_test: attrs.iter().any(|a| is_test_attr(a)),
                        gate_lines: Vec::new(),
                        idents: Vec::new(),
                    });
                    pending = Some(Frame::Fn(out.fns.len() - 1));
                }
                attrs.clear();
                i = j;
                continue;
            }
            Tok::Punct('{') => {
                let frame = pending.take().unwrap_or(Frame::Other);
                if let Frame::Mod = frame {
                    inline_mods.push(pending_mod_name.take().unwrap_or_default());
                }
                frames.push(frame);
                attrs.clear();
            }
            Tok::Punct('}') => {
                if let Some(Frame::Mod) = frames.pop() {
                    inline_mods.pop();
                }
                attrs.clear();
            }
            Tok::Punct(';') => attrs.clear(),
            Tok::Ident(name) => {
                let is_gate = name == GATE && punct(i + 1, '!') && !prev_is(toks, i, '!');
                match current_fn(&frames) {
                    Some(ix) => {
                        let f = &mut out.fns[ix];
                        if is_gate {
                            f.gate_lines.push(toks[i].line);
                        }
                        f.idents.push(name.clone());
                    }
                    None if is_gate => out.stray_gate_lines.push(toks[i].line),
                    None => {}
                }
            }
            _ => {}
        }
        i += 1;
    }
    out
}

fn prev_is(toks: &[Token], i: usize, p: char) -> bool {
    i > 0 && toks[i - 1].tok == Tok::Punct(p)
}

/// Index of the token closing the group opened at `open`.
fn matching(toks: &[Token], open: usize, o: char, c: char) -> usize {
    let mut depth = 0;
    for (j, t) in toks.iter().enumerate().skip(open) {
        if t.tok == Tok::Punct(o) {
            depth += 1;
        } else if t.tok == Tok::Punct(c) {
            depth -= 1;
            if depth == 0 {
                return j;
            }
        }
    }
    toks.len() - 1
}

/// `test`, `tokio::test(..)`, `rstest`, `test_case(..)`: the attribute's
/// leading path ends in one of these.
fn is_test_attr(attr: &[Tok]) -> bool {
    let mut last = None;
    for t in attr {
        match t {
            Tok::Ident(s) => last = Some(s.as_str()),
            Tok::Punct(':') => {}
            _ => break,
        }
    }
    matches!(last, Some("test" | "rstest" | "test_case"))
}

fn path_attr(attrs: &[Vec<Tok>]) -> Option<String> {
    attrs.iter().find_map(|a| match a.as_slice() {
        [Tok::Ident(p), Tok::Punct('='), Tok::Str(s)] if p == "path" => Some(s.clone()),
        _ => None,
    })
}
