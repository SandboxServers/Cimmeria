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

/// The start of every seed insert, lowercase (the search is case-insensitive).
const INSERT: &str = "insert into spawnlist";

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
            .filter(|p| p.extension().is_some_and(|x| x == "sql"))
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
    pub fn parse_sql(text: &str, into: &mut SeedTags) -> Result<(), String> {
        // ASCII lowercasing keeps every byte offset, so the search runs on
        // the copy and the values are read from the original.
        let lower = text.to_ascii_lowercase();
        let bytes = text.as_bytes();
        let mut from = 0;
        while let Some(found) = lower.get(from..).and_then(|s| s.find(INSERT)) {
            let mut pos = from + found + INSERT.len();
            from = pos;
            // Not `insert into spawnlist_debug`: the name must end there.
            if !matches!(bytes.get(pos), Some(b) if b.is_ascii_whitespace() || *b == b'(') {
                continue;
            }
            skip_ws(bytes, &mut pos);
            expect(bytes, &mut pos, b'(')?;
            let cols = read_columns(text, &mut pos)?;
            skip_ws(bytes, &mut pos);
            if !lower
                .as_bytes()
                .get(pos..)
                .is_some_and(|s| s.starts_with(b"values"))
            {
                return Err(format!("{pos}: expected VALUES after the column list"));
            }
            pos += "values".len();
            let tag_col = cols.iter().position(|c| c == "tag");
            let world_col = cols.iter().position(|c| c == "world_id");
            if tag_col.is_some() && world_col.is_none() {
                return Err(format!("{pos}: insert has a tag column but no world_id"));
            }
            loop {
                skip_ws(bytes, &mut pos);
                let start = pos;
                let row = read_tuple(text, &mut pos)?;
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
                skip_ws(bytes, &mut pos);
                match bytes.get(pos) {
                    Some(b',') => pos += 1,
                    Some(b';') => {
                        pos += 1;
                        break;
                    }
                    _ => return Err(format!("{pos}: expected ',' or ';' after a value tuple")),
                }
            }
            from = pos;
        }
        Ok(())
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

fn skip_ws(bytes: &[u8], pos: &mut usize) {
    while bytes.get(*pos).is_some_and(|b| b.is_ascii_whitespace()) {
        *pos += 1;
    }
}

fn expect(bytes: &[u8], pos: &mut usize, want: u8) -> Result<(), String> {
    skip_ws(bytes, pos);
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
        skip_ws(bytes, pos);
        let start = *pos;
        while bytes
            .get(*pos)
            .is_some_and(|b| !b.is_ascii_whitespace() && !matches!(b, b',' | b')'))
        {
            *pos += 1;
        }
        let name = text.get(start..*pos).unwrap_or("");
        if name.is_empty() {
            return Err(format!("{start}: expected a column name"));
        }
        cols.push(name.to_ascii_lowercase());
        skip_ws(bytes, pos);
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
        skip_ws(bytes, pos);
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
    skip_ws(bytes, pos);
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
        !b.is_ascii_whitespace() && !matches!(b, b',' | b')' | b';' | b'(' | b'\'')
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

    /// Fails if the parser misses a file or a column in the committed seed.
    #[test]
    fn the_committed_seed_defines_the_first_session_targets() {
        let dir = Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../db/resources/Worlds/Seed"
        ));
        let seed = SeedTags::load(dir).unwrap();
        for tag in [
            "ArmYourself_FrostBody",
            "ArmYourself_GuardBody",
            "SGCW1_GenHammond",
            "SGC_W1_Tealc",
            "SGC_W1_ElevatorButton1",
            "SGC_W1_FirearmBody",
        ] {
            assert!(seed.contains(tag), "{tag} is not seeded");
        }
        assert!(!seed.contains("SGC_W1_GenHammond"));
    }
}
