//! The tags the seed defines: `spawnlist.tag` in the `INSERT`s under
//! `db/resources/Worlds/Seed`. A spec that names a tag the seed lacks
//! would test nothing, so [`check`] rejects it at load (README F16, D-SV10).
//!
//! The parser reads the seed's own SQL, not a copy: it is a small
//! tokenizer for the one statement shape the seed uses, and it returns an
//! error (never a panic) on anything it does not understand.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde_json::Value as Json;

use super::spec::{ActionSpec, SectionSpec};

/// Every (world_id, tag) the seed's spawnlist rows define.
#[derive(Debug, Clone, Default)]
pub struct SeedTags {
    pub tags: BTreeMap<String, BTreeSet<i32>>,
}

/// One SQL value: `NULL`, a quoted string, or a bare word (a number).
#[derive(Debug, PartialEq)]
enum Sql {
    Null,
    Text(String),
    Bare(String),
}

impl SeedTags {
    /// Parse every `*.sql` file directly under `dir` (sorted by name).
    pub fn load(dir: &Path) -> Result<SeedTags, String> {
        let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
            .map_err(|e| format!("read {}: {e}", dir.display()))?
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("sql")))
            .collect();
        files.sort();
        let mut seed = SeedTags::default();
        for f in files {
            let text =
                std::fs::read_to_string(&f).map_err(|e| format!("read {}: {e}", f.display()))?;
            Self::parse_sql(&text, &mut seed).map_err(|e| format!("{}: {e}", f.display()))?;
        }
        Ok(seed)
    }

    /// Add the tags of every `INSERT INTO spawnlist (<cols>) VALUES (...)`
    /// in `text`. An insert without a `tag` column adds nothing.
    ///
    /// One pass over the text: comments and string literals are skipped
    /// whole, so an insert inside either is not one, and `insert`, `into`
    /// and `[public.]spawnlist` are separate case-insensitive words with any
    /// whitespace or comments between them.
    pub fn parse_sql(text: &str, into: &mut SeedTags) -> Result<(), String> {
        let bytes = text.as_bytes();
        let mut pos = 0;
        loop {
            skip_ws(bytes, &mut pos)?;
            let Some(&b) = bytes.get(pos) else {
                return Ok(());
            };
            if b == b'\'' {
                read_value(text, &mut pos)?;
                continue;
            }
            if !is_word_byte(b) {
                pos += 1;
                continue;
            }
            if !read_word(text, &mut pos).eq_ignore_ascii_case("insert") {
                continue;
            }
            let after_insert = pos;
            if !spawnlist_target(text, &mut pos)? {
                pos = after_insert;
                continue;
            }
            Self::parse_insert(text, &mut pos, into)?;
        }
    }

    /// The rest of one insert, from just past the table name through its `;`.
    fn parse_insert(text: &str, pos: &mut usize, into: &mut SeedTags) -> Result<(), String> {
        let bytes = text.as_bytes();
        expect(bytes, pos, b'(')?;
        let cols = read_columns(text, pos)?;
        skip_ws(bytes, pos)?;
        let at = *pos;
        if !read_word(text, pos).eq_ignore_ascii_case("values") {
            return Err(format!("{at}: expected VALUES after the column list"));
        }
        let tag_col = cols.iter().position(|c| c == "tag");
        let world_col = cols.iter().position(|c| c == "world_id");
        if tag_col.is_some() && world_col.is_none() {
            return Err(format!("{pos}: insert has a tag column but no world_id"));
        }
        loop {
            skip_ws(bytes, pos)?;
            let start = *pos;
            let row = read_tuple(text, pos)?;
            if row.len() != cols.len() {
                return Err(format!(
                    "{start}: tuple has {} values for {} columns",
                    row.len(),
                    cols.len()
                ));
            }
            if let (Some(t), Some(w)) = (tag_col, world_col) {
                into.add(start, &row[t], &row[w])?;
            }
            skip_ws(bytes, pos)?;
            match bytes.get(*pos) {
                Some(b',') => *pos += 1,
                Some(b';') => {
                    *pos += 1;
                    return Ok(());
                }
                _ => return Err(format!("{pos}: expected ',' or ';' after a value tuple")),
            }
        }
    }

    /// Record one row's tag in `world`. A `NULL` tag adds nothing.
    fn add(&mut self, at: usize, tag: &Sql, world: &Sql) -> Result<(), String> {
        let tag = match tag {
            Sql::Null => return Ok(()),
            Sql::Text(t) => t,
            Sql::Bare(_) => return Err(format!("{at}: tag is not a quoted string")),
        };
        let world = match world {
            Sql::Bare(w) => w
                .parse::<i32>()
                .map_err(|_| format!("{at}: world_id {w} is not an integer"))?,
            _ => return Err(format!("{at}: world_id is not an integer")),
        };
        self.tags.entry(tag.clone()).or_default().insert(world);
        Ok(())
    }

    /// Whether the seed defines `tag` (exact spelling, in any world).
    pub fn contains(&self, tag: &str) -> bool {
        self.tags.contains_key(tag)
    }

    /// `<specs_dir>/../../../db/resources/Worlds/Seed`, when it is a directory.
    pub fn beside_specs(specs_dir: &Path) -> Option<PathBuf> {
        let dir = specs_dir.join("../../../db/resources/Worlds/Seed");
        dir.is_dir().then_some(dir)
    }
}

/// Whether a `--` or `/*` comment starts at `pos`.
fn at_comment(bytes: &[u8], pos: usize) -> bool {
    matches!(
        (bytes.get(pos), bytes.get(pos + 1)),
        (Some(b'-'), Some(b'-')) | (Some(b'/'), Some(b'*'))
    )
}

/// Skip whitespace and comments (`--` to the end of the line, `/* */`):
/// SQL reads a comment as whitespace, so every gap between tokens may hold one.
fn skip_ws(bytes: &[u8], pos: &mut usize) -> Result<(), String> {
    loop {
        match (bytes.get(*pos), bytes.get(*pos + 1)) {
            (Some(b), _) if b.is_ascii_whitespace() => *pos += 1,
            (Some(b'-'), Some(b'-')) => {
                while bytes.get(*pos).is_some_and(|b| *b != b'\n') {
                    *pos += 1;
                }
            }
            (Some(b'/'), Some(b'*')) => {
                let start = *pos;
                *pos += 2;
                loop {
                    match (bytes.get(*pos), bytes.get(*pos + 1)) {
                        (None, _) => return Err(format!("{start}: unterminated comment")),
                        (Some(b'*'), Some(b'/')) => {
                            *pos += 2;
                            break;
                        }
                        _ => *pos += 1,
                    }
                }
            }
            _ => return Ok(()),
        }
    }
}

/// A byte of a bare SQL word (keyword, identifier, `schema.table`).
fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.')
}

/// The word at `pos` (possibly empty), moving `pos` past it.
fn read_word<'a>(text: &'a str, pos: &mut usize) -> &'a str {
    let bytes = text.as_bytes();
    let start = *pos;
    while bytes.get(*pos).is_some_and(|b| is_word_byte(*b)) {
        *pos += 1;
    }
    text.get(start..*pos).unwrap_or("")
}

/// After an `insert` word: whether `into [public.]spawnlist` follows. The
/// table must be the whole word, so `spawnlist_debug` is not it.
fn spawnlist_target(text: &str, pos: &mut usize) -> Result<bool, String> {
    let bytes = text.as_bytes();
    skip_ws(bytes, pos)?;
    if !read_word(text, pos).eq_ignore_ascii_case("into") {
        return Ok(false);
    }
    skip_ws(bytes, pos)?;
    let name = read_word(text, pos);
    let name = match name.get(..7) {
        Some(p) if p.eq_ignore_ascii_case("public.") => &name[7..],
        _ => name,
    };
    Ok(name.eq_ignore_ascii_case("spawnlist"))
}

fn expect(bytes: &[u8], pos: &mut usize, want: u8) -> Result<(), String> {
    skip_ws(bytes, pos)?;
    if bytes.get(*pos) == Some(&want) {
        *pos += 1;
        Ok(())
    } else {
        Err(format!("{pos}: expected '{}'", char::from(want)))
    }
}

/// The names in `(a, b, c)`; `pos` is just past the `(`.
fn read_columns(text: &str, pos: &mut usize) -> Result<Vec<String>, String> {
    let bytes = text.as_bytes();
    let mut cols = Vec::new();
    loop {
        skip_ws(bytes, pos)?;
        let start = *pos;
        while bytes.get(*pos).is_some_and(|b| {
            !b.is_ascii_whitespace() && !matches!(b, b',' | b')') && !at_comment(bytes, *pos)
        }) {
            *pos += 1;
        }
        let name = text.get(start..*pos).unwrap_or("");
        if name.is_empty() {
            return Err(format!("{start}: expected a column name"));
        }
        cols.push(name.to_ascii_lowercase());
        skip_ws(bytes, pos)?;
        match bytes.get(*pos) {
            Some(b',') => *pos += 1,
            Some(b')') => {
                *pos += 1;
                return Ok(cols);
            }
            _ => return Err(format!("{pos}: expected ',' or ')' in the column list")),
        }
    }
}

/// One `(v, v, ...)` tuple; `pos` is at its `(`.
fn read_tuple(text: &str, pos: &mut usize) -> Result<Vec<Sql>, String> {
    let bytes = text.as_bytes();
    expect(bytes, pos, b'(')?;
    let mut row = Vec::new();
    loop {
        row.push(read_value(text, pos)?);
        skip_ws(bytes, pos)?;
        match bytes.get(*pos) {
            Some(b',') => *pos += 1,
            Some(b')') => {
                *pos += 1;
                return Ok(row);
            }
            _ => return Err(format!("{pos}: expected ',' or ')' in a value tuple")),
        }
    }
}

/// One value: a `'...'` string (`''` is a quote), `NULL`, or a bare word.
fn read_value(text: &str, pos: &mut usize) -> Result<Sql, String> {
    let bytes = text.as_bytes();
    skip_ws(bytes, pos)?;
    if bytes.get(*pos) == Some(&b'\'') {
        let start = *pos;
        *pos += 1;
        let mut buf = Vec::new();
        loop {
            match bytes.get(*pos) {
                None => return Err(format!("{start}: unterminated string")),
                Some(b'\'') if bytes.get(*pos + 1) == Some(&b'\'') => {
                    buf.push(b'\'');
                    *pos += 2;
                }
                Some(b'\'') => {
                    *pos += 1;
                    break;
                }
                Some(&b) => {
                    buf.push(b);
                    *pos += 1;
                }
            }
        }
        // Only ASCII quote bytes were removed, so the bytes stay UTF-8.
        return String::from_utf8(buf)
            .map(Sql::Text)
            .map_err(|_| format!("{start}: string is not UTF-8"));
    }
    let start = *pos;
    while bytes.get(*pos).is_some_and(|b| {
        !b.is_ascii_whitespace()
            && !matches!(b, b',' | b')' | b';' | b'(' | b'\'')
            && !at_comment(bytes, *pos)
    }) {
        *pos += 1;
    }
    let word = text.get(start..*pos).unwrap_or("");
    if word.is_empty() {
        return Err(format!("{start}: expected a value"));
    }
    Ok(if word.eq_ignore_ascii_case("null") {
        Sql::Null
    } else {
        Sql::Bare(word.to_string())
    })
}

/// Every tag a section names: pose tags, and `tag` / `face_tag` in any
/// action's args (setup, step, teardown, fallbacks).
pub fn spec_tags(spec: &SectionSpec) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for row in &spec.rows {
        for pose in &row.pose {
            out.push((row.id.clone(), pose.tag.clone()));
        }
        for action in row.setup.iter().chain(&row.steps).chain(&row.teardown) {
            action_tags(action, &row.id, &mut out);
        }
    }
    out
}

fn action_tags(action: &ActionSpec, row: &str, out: &mut Vec<(String, String)>) {
    if let Some(args) = &action.args {
        for key in ["tag", "face_tag"] {
            if let Some(tag) = args.get(key).and_then(Json::as_str) {
                out.push((row.to_string(), tag.to_string()));
            }
        }
    }
    for fallback in &action.fallback {
        action_tags(fallback, row, out);
    }
}

/// Every tag the section names must be in the seed. One message per unknown
/// tag, joined with "; "; a case-only near miss adds "did you mean ...?".
pub fn check(spec: &SectionSpec, seed: &SeedTags) -> Result<(), String> {
    let mut errors = Vec::new();
    for (row, tag) in spec_tags(spec) {
        if seed.contains(&tag) {
            continue;
        }
        let mut msg = format!("{row}: tag {tag} is not in the seed (db/resources/Worlds/Seed)");
        if let Some(near) = seed.tags.keys().find(|k| k.eq_ignore_ascii_case(&tag)) {
            msg.push_str(&format!(", did you mean {near}?"));
        }
        errors.push(msg);
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> Result<SeedTags, String> {
        let mut seed = SeedTags::default();
        SeedTags::parse_sql(text, &mut seed)?;
        Ok(seed)
    }

    fn pairs(seed: &SeedTags) -> Vec<(String, i32)> {
        seed.tags
            .iter()
            .flat_map(|(tag, worlds)| worlds.iter().map(move |w| (tag.clone(), *w)))
            .collect()
    }

    #[test]
    fn parses_single_and_multi_row_inserts() {
        let sql = "\
INSERT INTO spawnlist (spawn_id, x, world_id, tag) VALUES (1, 2.5, 58, 'ArmYourself_FrostBody');
insert into spawnlist (spawn_id, x, world_id, tag, set_name) VALUES
  (2, 0, 12, 'DebugArea_Ring', NULL),
  (3, 0, 12, NULL, NULL),
  (4, -1.5, 61, 'O''Neill_Body', 'set, one (x)');
INSERT INTO spawnlist (tag, world_id, spawn_id) VALUES ('SGC_W1_Tealc', 58, 5);
INSERT INTO spawnlist (spawn_id, world_id) VALUES (6, 12);
";
        let seed = parse(sql).unwrap();
        assert_eq!(
            pairs(&seed),
            vec![
                ("ArmYourself_FrostBody".to_string(), 58),
                ("DebugArea_Ring".to_string(), 12),
                ("O'Neill_Body".to_string(), 61),
                ("SGC_W1_Tealc".to_string(), 58),
            ]
        );
    }

    #[test]
    fn a_short_tuple_is_an_error() {
        let err =
            parse("INSERT INTO spawnlist (spawn_id, world_id, tag) VALUES (1, 58);").unwrap_err();
        assert!(err.contains("tuple has 2 values for 3 columns"), "{err}");
        assert!(err.starts_with("55:"), "{err}");
    }

    #[test]
    fn a_truncated_insert_is_an_error_not_a_panic() {
        let full = "INSERT INTO spawnlist (spawn_id, world_id, tag) VALUES (1, 58, 'Abc''d');";
        for cut in 0..full.len() {
            let text = &full[..cut];
            // Every prefix either parses to nothing or fails with an error.
            let _ = parse(text);
        }
        assert!(parse(&full[..full.len() - 1]).is_err());
    }

    #[test]
    fn keywords_may_be_split_by_any_whitespace_and_schema_qualified() {
        let sql = "\
INSERT  INTO  spawnlist (world_id, tag) VALUES (1, 'Double');
insert
  into
spawnlist (world_id, tag) values (2, 'Newlines');
INSERT INTO public.spawnlist (world_id, tag) VALUES (3, 'Public');
INSERT /* c */ INTO -- c
  PUBLIC.SpawnList (world_id, tag) VALUES (4, 'Commented');
";
        let seed = parse(sql).unwrap();
        assert_eq!(
            pairs(&seed),
            vec![
                ("Commented".to_string(), 4),
                ("Double".to_string(), 1),
                ("Newlines".to_string(), 2),
                ("Public".to_string(), 3),
            ]
        );
    }

    #[test]
    fn inserts_inside_comments_and_strings_are_not_inserts() {
        let sql = "\
-- INSERT INTO spawnlist (world_id, tag) VALUES (9, 'LineComment');
/* INSERT INTO spawnlist (world_id, tag) VALUES (9, 'BlockComment'); */
-- prose: an insert into spawnlist needs a tag column
SELECT 'insert into spawnlist (oops';
INSERT INTO spawnlist_debug (world_id, tag) VALUES (9, 'OtherTable');
INSERT INTO spawnlist (world_id, tag) VALUES (1, 'Real');
";
        let seed = parse(sql).unwrap();
        assert_eq!(pairs(&seed), vec![("Real".to_string(), 1)]);
    }

    #[test]
    fn comments_between_tuples_are_whitespace() {
        let sql = "\
INSERT INTO spawnlist (world_id, -- the map
  tag) VALUES
  (1, 'First'), -- a trailing note, with 'quotes' and (parens)
-- a whole line between tuples
  (2, /* inline */ 'Second'),
  (-3/**/, 'Third') /* last */ ;
";
        let seed = parse(sql).unwrap();
        assert_eq!(
            pairs(&seed),
            vec![
                ("First".to_string(), 1),
                ("Second".to_string(), 2),
                ("Third".to_string(), -3),
            ]
        );
    }

    #[test]
    fn an_unterminated_block_comment_is_an_error() {
        let err = parse("INSERT INTO spawnlist (world_id, tag) /* VALUES (1, 'x');").unwrap_err();
        assert!(err.contains("unterminated comment"), "{err}");
    }

    /// One message per unknown tag, from every place a spec can name one.
    /// Fails if any extraction path in `spec_tags` / `action_tags` is lost.
    #[test]
    fn check_reports_unknown_tags_from_every_source() {
        let spec: SectionSpec = toml::from_str(
            r#"
schema = 1
[section]
id = "tags"
system = "s"
guide = "g"
ledger = "l"

[[row]]
id = "R"
title = "t"
expected = "e"
pose = [{ id = "p", tag = "PoseTag", radius_m = 4.0, bearing_deg = 0.0 }]
setup = [{ tool = "a", args = { tag = "SetupTag" } }]
step = [
  { tool = "b", args = { tag = "StepTag" } },
  { tool = "c", args = { face_tag = "FaceTag" } },
  { tool = "d", fallback = [{ tool = "e", args = { tag = "FallbackTag" } }] },
  { tool = "f", args = { tag = "Known" } },
]
teardown = [{ tool = "g", args = { tag = "TeardownTag" } }]
"#,
        )
        .unwrap();
        let mut seed = SeedTags::default();
        seed.tags.entry("Known".to_string()).or_default().insert(1);

        let err = check(&spec, &seed).unwrap_err();
        let msgs: Vec<&str> = err.split("; ").collect();
        let expected: Vec<String> = [
            "PoseTag",
            "SetupTag",
            "StepTag",
            "FaceTag",
            "FallbackTag",
            "TeardownTag",
        ]
        .iter()
        .map(|t| format!("R: tag {t} is not in the seed (db/resources/Worlds/Seed)"))
        .collect();
        assert_eq!(msgs, expected, "{err}");
    }

    /// Fails if `load` drops a seed file or the parser misses a column in the
    /// committed seed. No total is pinned: every zone campaign adds spawn tags
    /// (608 on 2026-10-10). Instead every tag a single file defines must reach
    /// the loaded union, and the union holds the first-session targets plus one
    /// tag from each per-zone spawn file.
    #[test]
    fn the_committed_seed_defines_the_first_session_targets() {
        let dir = Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../db/resources/Worlds/Seed"
        ));
        let seed = SeedTags::load(dir).unwrap();

        let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
            .unwrap()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("sql")))
            .collect();
        files.sort();
        let mut tagged_files = 0;
        for f in &files {
            let one = parse(&std::fs::read_to_string(f).unwrap())
                .unwrap_or_else(|e| panic!("{}: {e}", f.display()));
            tagged_files += usize::from(!one.tags.is_empty());
            for (tag, worlds) in &one.tags {
                assert!(
                    seed.tags.get(tag).is_some_and(|w| w.is_superset(worlds)),
                    "{}: tag {tag} (worlds {worlds:?}) is missing from SeedTags::load",
                    f.display()
                );
            }
        }
        assert!(
            tagged_files >= 3,
            "only {tagged_files} seed files define tags"
        );

        assert!(
            seed.tags.len() >= 608,
            "distinct seeded tags fell to {}",
            seed.tags.len()
        );
        for tag in [
            "ArmYourself_FrostBody",
            "ArmYourself_GuardBody",
            "SGCW1_GenHammond",
            "SGC_W1_Tealc",
            "SGC_W1_ElevatorButton1",
            "SGC_W1_FirearmBody",
            // spawnlist_tollana.sql (four-zone campaign)
            "Tollana_Copplemann",
            // spawnlist_dakara_e1.sql (Dakara E1 rebuild)
            "Dakara_E1_TentFlap_ToCommand",
        ] {
            assert!(seed.contains(tag), "{tag} is not seeded");
        }
        assert!(!seed.contains("SGC_W1_GenHammond"));
    }
}
