// Cimmeria Bible symbol-resolution lint.
//
// Scans every chapter under `docs/spec/` and emits warnings for:
//   (1) frontmatter `evidence_refs.rust:` entries that don't resolve to a real
//       Rust symbol in the workspace,
//   (2) body-prose `crates/...::Type::method` references in section 4 or 5 that
//       don't resolve, and
//   (3) violations of the no-line-numbers rule in section 4 or 5 (bare `.rs:N`
//       citations).
//
// Always exits 0 (warn-only). When run under GitHub Actions, emits
// `::warning file=...,line=...::message` annotations so warnings surface
// inline in PR diffs.
//
// The shape of each citation form is canonized in
// docs/spec/conventions.md §"Evidence-ref grammar" and §"The no-line-numbers
// rule for sections 4 and 5". This binary is the enforcement teeth — see #264.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use regex::Regex;
use walkdir::WalkDir;

// ---- types --------------------------------------------------------------

#[derive(Debug)]
struct Finding {
    file: PathBuf,
    line: usize,
    kind: FindingKind,
    detail: String,
}

#[derive(Debug)]
enum FindingKind {
    UnresolvedSymbol,
    LineNumberInRustSection,
    MalformedRef,
}

impl FindingKind {
    fn label(&self) -> &'static str {
        match self {
            Self::UnresolvedSymbol => "unresolved-symbol",
            Self::LineNumberInRustSection => "line-number-in-rust-section",
            Self::MalformedRef => "malformed-ref",
        }
    }
}

// ---- crate-name resolution ---------------------------------------------

/// Build a map from crate name (e.g. `cimmeria-services`) to its source root
/// (e.g. `crates/services/src`). Scans `crates/*/Cargo.toml` and `tools/*/Cargo.toml`.
fn build_crate_map(repo_root: &Path) -> HashMap<String, PathBuf> {
    let mut map = HashMap::new();
    let name_re = Regex::new(r#"(?m)^name\s*=\s*"([^"]+)""#).expect("static regex");

    for parent in ["crates", "tools"] {
        let parent_dir = repo_root.join(parent);
        let Ok(entries) = fs::read_dir(&parent_dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let crate_dir = entry.path();
            let manifest = crate_dir.join("Cargo.toml");
            let Ok(text) = fs::read_to_string(&manifest) else {
                continue;
            };
            // Only match the first `name = "..."` (the [package] block).
            // Workspace dependency entries that come later in the file
            // are not relevant — we want the crate's published name.
            if let Some(caps) = name_re.captures(&text) {
                let name = caps[1].to_string();
                let src = crate_dir.join("src");
                if src.is_dir() {
                    map.insert(name, src);
                }
            }
        }
    }

    map
}

// ---- markdown parsing ---------------------------------------------------

struct Chapter {
    frontmatter: String,
    body: String,
    body_offset: usize, // line number where `body` starts in the file (1-based)
}

fn parse_chapter(path: &Path) -> Option<Chapter> {
    let text = fs::read_to_string(path).ok()?;
    let mut lines = text.lines();

    // First non-blank line must be `---`.
    let first = lines.next()?;
    if first.trim() != "---" {
        return None;
    }

    let mut fm = String::new();
    let mut consumed = 1; // the opening `---`
    for line in lines.by_ref() {
        consumed += 1;
        if line.trim() == "---" {
            break;
        }
        fm.push_str(line);
        fm.push('\n');
    }

    let body: String = lines.collect::<Vec<_>>().join("\n");
    let body_offset = consumed + 1; // first body line is one past closing `---`

    Some(Chapter {
        frontmatter: fm,
        body,
        body_offset,
    })
}

/// Returns the chapter ID (`spec.X.Y`) from frontmatter, if present.
fn chapter_id(fm: &str) -> Option<&str> {
    for line in fm.lines() {
        if let Some(rest) = line.strip_prefix("chapter_id:") {
            return Some(rest.trim());
        }
    }
    None
}

/// Returns the entries under `evidence_refs.rust:` in the frontmatter.
/// Each entry is `(line_number_in_file, symbol)`.
fn extract_rust_evidence_refs(chapter: &Chapter) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let mut in_block = false;
    let mut block_indent: usize = 0;

    for (idx, line) in chapter.frontmatter.lines().enumerate() {
        let file_line = idx + 2; // +1 for opening `---`, +1 for 1-based

        // Detect `rust:` under any sub-indent.
        if let Some(stripped) = line.trim_start().strip_prefix("rust:") {
            // Only enter the block when the suffix is empty or a comment.
            // `rust: [some-inline-list]` would also be valid YAML but we
            // don't bother — current convention is block-style.
            if stripped.trim().is_empty() || stripped.trim_start().starts_with('#') {
                in_block = true;
                block_indent = line.len() - line.trim_start().len();
                continue;
            }
        }

        if in_block {
            let indent = line.len() - line.trim_start().len();
            // Continue while the line is more indented than the `rust:` line
            // and starts with `-`. Anything else closes the block.
            if line.trim().is_empty() {
                continue; // blank line inside the list — keep scanning
            }
            if indent <= block_indent {
                in_block = false;
                continue;
            }
            if let Some(item) = line.trim_start().strip_prefix('-') {
                let sym = item.trim().to_string();
                if !sym.is_empty() {
                    out.push((file_line, sym));
                }
            } else {
                // A `key:` at deeper indent — not a list entry.
                in_block = false;
            }
        }
    }

    out
}

/// Returns `(section_4_body, section_5_body)` with line offsets relative to
/// the chapter file (1-based). Returns `None` for a section that's absent.
fn extract_rust_sections(chapter: &Chapter) -> (Option<(usize, String)>, Option<(usize, String)>) {
    let lines: Vec<&str> = chapter.body.lines().collect();
    let mut s4: Option<(usize, String)> = None;
    let mut s5: Option<(usize, String)> = None;

    let s4_re = Regex::new(r"(?i)^##\s*section\s*4\b").expect("static regex");
    let s5_re = Regex::new(r"(?i)^##\s*section\s*5\b").expect("static regex");
    let any_h2 = Regex::new(r"^##\s").expect("static regex");

    let mut idx = 0;
    while idx < lines.len() {
        let line = lines[idx];
        let opening_match = if s4_re.is_match(line) {
            Some(4)
        } else if s5_re.is_match(line) {
            Some(5)
        } else {
            None
        };

        if let Some(which) = opening_match {
            let body_line_in_file = chapter.body_offset + idx;
            let mut body = String::new();
            let mut j = idx + 1;
            while j < lines.len() && !any_h2.is_match(lines[j]) {
                body.push_str(lines[j]);
                body.push('\n');
                j += 1;
            }
            match which {
                4 => s4 = Some((body_line_in_file, body)),
                5 => s5 = Some((body_line_in_file, body)),
                _ => unreachable!(),
            }
            idx = j;
            continue;
        }
        idx += 1;
    }

    (s4, s5)
}

// ---- reference extraction & resolution ---------------------------------

/// Match a body-prose symbol reference like
/// `crates/services/src/cell/combat/threat.rs::ThreatList::add`.
fn body_ref_regex() -> Regex {
    Regex::new(r"crates/[A-Za-z0-9_./-]+\.rs(?:::[A-Za-z_][A-Za-z0-9_]*)+").expect("static regex")
}

/// Match a frontmatter-style symbol reference like
/// `cimmeria-services::cell::combat::threat::ThreatList::add`.
fn frontmatter_ref_regex() -> Regex {
    Regex::new(r"^cimmeria-[a-z0-9_-]+(?:::[A-Za-z_][A-Za-z0-9_]*)+$").expect("static regex")
}

/// Match a no-line-numbers rule violation: `foo.rs:NN` or `foo.rs:NN-MM` inside
/// a section-4/5 body. Counts as a violation only when the path is under
/// `crates/`, `src/`, or starts with a Rust filename — paths under `game/sgw/`,
/// `deprecated/`, etc., are out of scope for this rule (they are evidence
/// paths, not Rust paths, even if they appear inline in section 4/5 prose).
fn line_number_violation_regex() -> Regex {
    // Match `<token>.rs:<digits>` where the path token starts with `crates/`
    // or with a bare filename component followed by `.rs`. The lint is best-
    // effort; we want to catch obvious violations without false-firing on
    // evidence citations.
    Regex::new(r"(?:^|[\s`(\[])((?:crates/|[A-Za-z_])[A-Za-z0-9_./-]*\.rs:\d+(?:-\d+)?)")
        .expect("static regex")
}

fn resolve_body_ref(
    repo_root: &Path,
    raw: &str,
    findings: &mut Vec<Finding>,
    file: &Path,
    line: usize,
) {
    // Split into file part + symbol chain.
    let Some((file_path, symbol_chain)) = raw.split_once(".rs::") else {
        findings.push(Finding {
            file: file.to_path_buf(),
            line,
            kind: FindingKind::MalformedRef,
            detail: format!("body ref `{raw}` is missing `.rs::` separator"),
        });
        return;
    };
    let file_path = format!("{file_path}.rs");
    let abs_path = repo_root.join(&file_path);

    if !abs_path.is_file() {
        findings.push(Finding {
            file: file.to_path_buf(),
            line,
            kind: FindingKind::UnresolvedSymbol,
            detail: format!("body ref `{raw}` — file `{file_path}` does not exist"),
        });
        return;
    }

    let Ok(source) = fs::read_to_string(&abs_path) else {
        findings.push(Finding {
            file: file.to_path_buf(),
            line,
            kind: FindingKind::UnresolvedSymbol,
            detail: format!("body ref `{raw}` — could not read `{file_path}`"),
        });
        return;
    };

    // Walk the symbol chain. For each leaf, search the file for a plausible
    // definition. Multi-segment chains check each segment as either a type
    // or a method.
    let segments: Vec<&str> = symbol_chain.split("::").collect();
    let leaf = segments[segments.len() - 1];
    let parent_type = if segments.len() >= 2 {
        Some(segments[segments.len() - 2])
    } else {
        None
    };

    if !symbol_exists(&source, leaf, parent_type) {
        findings.push(Finding {
            file: file.to_path_buf(),
            line,
            kind: FindingKind::UnresolvedSymbol,
            detail: format!("body ref `{raw}` — `{leaf}` not found in `{file_path}`"),
        });
    }
}

fn resolve_frontmatter_ref(
    crate_map: &HashMap<String, PathBuf>,
    raw: &str,
    findings: &mut Vec<Finding>,
    file: &Path,
    line: usize,
) {
    let segments: Vec<&str> = raw.split("::").collect();
    if segments.len() < 2 {
        findings.push(Finding {
            file: file.to_path_buf(),
            line,
            kind: FindingKind::MalformedRef,
            detail: format!("frontmatter ref `{raw}` has no path segments after crate name"),
        });
        return;
    }

    let crate_name = segments[0];
    let Some(crate_src) = crate_map.get(crate_name) else {
        findings.push(Finding {
            file: file.to_path_buf(),
            line,
            kind: FindingKind::UnresolvedSymbol,
            detail: format!("frontmatter ref `{raw}` — crate `{crate_name}` not in workspace"),
        });
        return;
    };

    // Walk module path, trying multiple file layouts at each step.
    // Given `cimmeria-services::cell::combat::threat::ThreatList::add`, we
    // strip the trailing 1-2 segments (the leaf symbol + optional parent type)
    // and try to resolve the remaining as a module path.
    //
    // We don't know in advance how many trailing segments are symbol vs.
    // module. Strategy: peel back one segment at a time from the right;
    // the first module path that resolves to an existing file wins, and the
    // remaining trailing segments are treated as the symbol chain.
    let inner = &segments[1..];

    for split_point in (1..=inner.len()).rev() {
        let module_segs = &inner[..split_point - 1];
        let symbol_segs = &inner[split_point - 1..];

        let candidates = module_file_candidates(crate_src, module_segs);
        for candidate in &candidates {
            if candidate.is_file() {
                let Ok(source) = fs::read_to_string(candidate) else {
                    continue;
                };
                let leaf = symbol_segs[symbol_segs.len() - 1];
                let parent_type = if symbol_segs.len() >= 2 {
                    Some(symbol_segs[symbol_segs.len() - 2])
                } else {
                    None
                };
                if symbol_exists(&source, leaf, parent_type) {
                    return; // resolved
                }
            }
        }
    }

    findings.push(Finding {
        file: file.to_path_buf(),
        line,
        kind: FindingKind::UnresolvedSymbol,
        detail: format!("frontmatter ref `{raw}` — symbol not found in crate `{crate_name}`"),
    });
}

/// Given a module path like `["cell", "combat", "threat"]` and a crate src
/// dir, return all candidate file paths to check.
fn module_file_candidates(crate_src: &Path, module_path: &[&str]) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if module_path.is_empty() {
        // Root of the crate — symbol must be in lib.rs / main.rs.
        candidates.push(crate_src.join("lib.rs"));
        candidates.push(crate_src.join("main.rs"));
        return candidates;
    }

    let last = module_path[module_path.len() - 1];
    let parent_chain: PathBuf = module_path[..module_path.len() - 1]
        .iter()
        .fold(crate_src.to_path_buf(), |acc, seg| acc.join(seg));

    candidates.push(parent_chain.join(format!("{last}.rs")));
    candidates.push(parent_chain.join(last).join("mod.rs"));

    // Also try the parent file — the symbol may be defined inline in an
    // `mod` block in the parent. E.g. `foo::bar::Baz` might live in
    // `foo.rs` inside a `pub mod bar { struct Baz; }` block.
    if module_path.len() >= 2 {
        let parent_last = module_path[module_path.len() - 2];
        let grandparent_chain: PathBuf = module_path[..module_path.len() - 2]
            .iter()
            .fold(crate_src.to_path_buf(), |acc, seg| acc.join(seg));
        candidates.push(grandparent_chain.join(format!("{parent_last}.rs")));
        candidates.push(grandparent_chain.join(parent_last).join("mod.rs"));
    }

    candidates
}

/// Best-effort: does the given symbol appear as a definition in this source?
/// `leaf` is the symbol name; `parent_type` is the type it might be a method
/// on (when relevant).
fn symbol_exists(source: &str, leaf: &str, parent_type: Option<&str>) -> bool {
    // Defining-form patterns. Order matters only for performance.
    let patterns = [
        format!(r"\bfn\s+{}\b", regex::escape(leaf)),
        format!(r"\bstruct\s+{}\b", regex::escape(leaf)),
        format!(r"\benum\s+{}\b", regex::escape(leaf)),
        format!(r"\btrait\s+{}\b", regex::escape(leaf)),
        format!(r"\btype\s+{}\b", regex::escape(leaf)),
        format!(r"\bunion\s+{}\b", regex::escape(leaf)),
        format!(r"\bconst\s+{}\b", regex::escape(leaf)),
        format!(r"\bstatic\s+{}\b", regex::escape(leaf)),
        // Macro definitions: `macro_rules! foo`
        format!(r"\bmacro_rules!\s+{}\b", regex::escape(leaf)),
    ];
    for pat in &patterns {
        if Regex::new(pat).expect("static regex").is_match(source) {
            // For `fn` leaves with a `parent_type`, also check the type
            // exists in the same file. If neither the type nor an impl
            // block for it appears, the ref probably points elsewhere.
            if let Some(pt) = parent_type {
                let type_pat = format!(
                    r"\b(?:struct|enum|trait|type|union|impl(?:\s*<[^>]*>)?\s+(?:[A-Za-z0-9_:<>,\s]+\s+for\s+)?){}\b",
                    regex::escape(pt)
                );
                if !Regex::new(&type_pat)
                    .expect("static regex")
                    .is_match(source)
                {
                    // Symbol exists but on a type that isn't here — keep
                    // looking via other candidate files. Treat as not found.
                    continue;
                }
            }
            return true;
        }
    }
    false
}

// ---- main flow ---------------------------------------------------------

fn lint_chapter(
    repo_root: &Path,
    crate_map: &HashMap<String, PathBuf>,
    path: &Path,
    findings: &mut Vec<Finding>,
) {
    let Some(chapter) = parse_chapter(path) else {
        return; // no frontmatter — not a chapter
    };

    // Skip meta apparatus (README, conventions, glossary, how-to-{read,write}).
    if let Some(id) = chapter_id(&chapter.frontmatter) {
        if id.starts_with("spec.meta.") {
            return;
        }
    }

    // (1) frontmatter `evidence_refs.rust:` entries
    let fm_refs = extract_rust_evidence_refs(&chapter);
    let fm_re = frontmatter_ref_regex();
    for (line, raw) in fm_refs {
        if !fm_re.is_match(&raw) {
            findings.push(Finding {
                file: path.to_path_buf(),
                line,
                kind: FindingKind::MalformedRef,
                detail: format!(
                    "frontmatter `evidence_refs.rust:` entry `{raw}` does not match \
                    crate-prefixed `cimmeria-<name>::module::path::Symbol` form"
                ),
            });
            continue;
        }
        resolve_frontmatter_ref(crate_map, &raw, findings, path, line);
    }

    // (2) body-prose symbol refs in sections 4 and 5
    // (3) line-number violations in sections 4 and 5
    let (s4, s5) = extract_rust_sections(&chapter);
    let body_re = body_ref_regex();
    let line_no_re = line_number_violation_regex();

    for (section_num, section) in [(4u8, &s4), (5u8, &s5)] {
        let Some((start_line, body)) = section else {
            continue;
        };
        for (offset, body_line) in body.lines().enumerate() {
            let file_line = start_line + 1 + offset;

            for mat in body_re.find_iter(body_line) {
                resolve_body_ref(repo_root, mat.as_str(), findings, path, file_line);
            }
            for caps in line_no_re.captures_iter(body_line) {
                if let Some(m) = caps.get(1) {
                    findings.push(Finding {
                        file: path.to_path_buf(),
                        line: file_line,
                        kind: FindingKind::LineNumberInRustSection,
                        detail: format!(
                            "section {section_num}: `{}` contains a line number — \
                            cite by symbol instead (see conventions.md §\"The \
                            no-line-numbers rule\")",
                            m.as_str()
                        ),
                    });
                }
            }
        }
    }
}

fn emit_findings(findings: &[Finding], gh_actions: bool, repo_root: &Path) {
    for f in findings {
        let rel = f
            .file
            .strip_prefix(repo_root)
            .unwrap_or(&f.file)
            .display()
            .to_string()
            .replace('\\', "/");
        if gh_actions {
            // GitHub Actions warning annotation. The `title=` makes the
            // finding kind visible in the PR Files Changed view.
            println!(
                "::warning file={file},line={line},title=spec-lint {kind}::{detail}",
                file = rel,
                line = f.line,
                kind = f.kind.label(),
                detail = f.detail,
            );
        } else {
            println!(
                "{file}:{line}: [{kind}] {detail}",
                file = rel,
                line = f.line,
                kind = f.kind.label(),
                detail = f.detail,
            );
        }
    }
}

fn print_summary(findings: &[Finding]) {
    let total = findings.len();
    if total == 0 {
        println!("spec-lint: 0 findings.");
        return;
    }
    let mut by_kind: HashMap<&'static str, usize> = HashMap::new();
    for f in findings {
        *by_kind.entry(f.kind.label()).or_default() += 1;
    }
    let mut parts: Vec<String> = by_kind
        .into_iter()
        .map(|(k, n)| format!("{k}={n}"))
        .collect();
    parts.sort();
    println!("spec-lint: {total} findings ({}).", parts.join(", "));
}

fn main() -> ExitCode {
    // Repo root: the parent that contains a `docs/spec` directory. We default
    // to CWD; running from anywhere inside the workspace works because we
    // walk upward until we find docs/spec.
    let repo_root = find_repo_root().unwrap_or_else(|| PathBuf::from("."));
    let spec_root = repo_root.join("docs").join("spec");
    if !spec_root.is_dir() {
        eprintln!(
            "spec-lint: docs/spec/ not found under {} — nothing to lint",
            repo_root.display()
        );
        return ExitCode::SUCCESS;
    }

    let crate_map = build_crate_map(&repo_root);
    let mut findings: Vec<Finding> = Vec::new();

    for entry in WalkDir::new(&spec_root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|s| s.to_str())
                .is_some_and(|s| s.eq_ignore_ascii_case("md"))
        })
    {
        lint_chapter(&repo_root, &crate_map, entry.path(), &mut findings);
    }

    let gh_actions = std::env::var("GITHUB_ACTIONS").is_ok();
    emit_findings(&findings, gh_actions, &repo_root);
    print_summary(&findings);

    // Warn-only. Always exits 0 so CI never fails on lint output.
    ExitCode::SUCCESS
}

fn find_repo_root() -> Option<PathBuf> {
    let mut cur = std::env::current_dir().ok()?;
    loop {
        if cur.join("docs").join("spec").is_dir() {
            return Some(cur);
        }
        if !cur.pop() {
            return None;
        }
    }
}
