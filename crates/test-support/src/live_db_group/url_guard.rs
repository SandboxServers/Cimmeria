//! The second half of the per-slot contract: a live-DB test process talks
//! to its slot's database only if nothing resolves the database URL except
//! [`crate::database_url`]. This scan flags the ways around it.

use super::lex::{lex, Tok};

/// Files allowed to name `DATABASE_URL`: the resolver itself.
const URL_READERS: &[&str] = &["test-support/src/live_db_slot.rs"];

/// Files allowed to hold a `postgres://` literal with a real host: pure
/// string conversion (`libpq_to_url`, the slot suffix) and their unit
/// tests; neither connects.
const URL_LITERAL_FILES: &[&str] = &[
    "services/src/database.rs",
    "test-support/src/live_db_slot.rs",
];

/// `(line, why)` for each way `src` could reach a database other than the
/// slot's. `crates_rel` is the file's path under `crates/`.
pub(crate) fn url_violations(crates_rel: &str, src: &str) -> Vec<(usize, String)> {
    let toks = lex(src);
    let mut out = Vec::new();
    for (i, t) in toks.iter().enumerate() {
        match &t.tok {
            Tok::Str(s) if s == "DATABASE_URL" && !URL_READERS.contains(&crates_rel) => {
                out.push((
                    t.line,
                    "reads DATABASE_URL directly; use `test_support::database_url()`, \
                     which resolves the live-DB slot"
                        .into(),
                ));
            }
            Tok::Str(s) if is_real_postgres_url(s) && !URL_LITERAL_FILES.contains(&crates_rel) => {
                out.push((
                    t.line,
                    format!(
                        "hard-coded database URL {s:?}; a live-DB test must use \
                         `test_support::database_url()` (an unreachable pool may use \
                         127.0.0.1:1)"
                    ),
                ));
            }
            Tok::Ident(s) if s == "sqlx" => {
                let path: Vec<&Tok> = toks[i + 1..].iter().take(3).map(|t| &t.tok).collect();
                if matches!(path.as_slice(), [Tok::Punct(':'), Tok::Punct(':'), Tok::Ident(n)] if n == "test")
                {
                    out.push((
                        t.line,
                        "`sqlx::test` provisions databases from DATABASE_URL itself, \
                         outside the live-DB slots; use `require_db_or_skip!`"
                            .into(),
                    ));
                }
            }
            _ => {}
        }
    }
    out
}

/// A `postgres://` or `postgresql://` URL with a host, other than the
/// never-listening `127.0.0.1:1` the infrastructure-failure tests use.
fn is_real_postgres_url(s: &str) -> bool {
    let rest = s
        .strip_prefix("postgres://")
        .or_else(|| s.strip_prefix("postgresql://"));
    matches!(rest, Some(r) if !r.is_empty() && !r.contains("@127.0.0.1:1/"))
}
