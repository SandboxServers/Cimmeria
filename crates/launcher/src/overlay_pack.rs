//! Pack the client-patches UI overlay into a manifest patch (BM-06).
//!
//! The overlay lives in `crates/client-patches/overlay/`, laid out like the
//! client tree below `Working/SGWGame/`, with a `MANIFEST.txt` naming each
//! file to ship. This module turns it into:
//!
//! - a patch zip whose entries are those paths, extracted by the launcher
//!   into the client's `SGWGame/` directory (`"root": "sgw_game"`);
//! - the manifest entry for it, with an id that carries the zip's hash, so
//!   a changed overlay is a new patch (the launcher applies each id once)
//!   and an unchanged one is the same patch.
//!
//! The zip is deterministic (sorted entries, fixed timestamps), so the same
//! overlay always packs to the same bytes and the same id.
//!
//! Compiled into the `pack-client-overlay` tool, and into the launcher only
//! for its tests.

use std::io::Write;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::manifest::{Manifest, ManifestError, PatchEntry, PatchRoot};

/// The file in the overlay directory that lists what ships.
pub const LIST_FILE: &str = "MANIFEST.txt";

/// Files in the overlay directory that document it and never ship.
const NOT_SHIPPED: [&str; 2] = [LIST_FILE, "README.md"];

/// Top-level directory of the overlay's own tests, which never ships.
const TEST_DIR: &str = "test/";

pub use crate::overlay_meta::{DEFAULT_ID_PREFIX, OVERLAY_DESCRIPTION, OVERLAY_TITLE};

#[derive(Debug, Error)]
pub enum PackError {
    #[error("{LIST_FILE} line {line}: {reason}: {path:?}")]
    BadPath {
        line: usize,
        path: String,
        reason: &'static str,
    },
    #[error("{LIST_FILE} lists {0:?} twice")]
    Duplicate(String),
    #[error("{LIST_FILE} lists {0:?}, which is not a file in the overlay directory")]
    Missing(String),
    #[error("{0:?} is in the overlay directory but not in {LIST_FILE}; list it or remove it")]
    Unlisted(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("zip error: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("the merged manifest is invalid: {0}")]
    Manifest(#[from] ManifestError),
}

/// Parse `MANIFEST.txt`: one path per line, relative to the overlay root
/// (`Working/SGWGame/` in the client). Blank lines and `#` comments are
/// skipped, `\` is read as `/`, and a leading `SGWGame/` is dropped so a
/// list written relative to `Working/` means the same thing.
pub fn parse_list(text: &str) -> Result<Vec<String>, PackError> {
    let mut out: Vec<String> = Vec::new();
    for (i, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let bad = |reason| PackError::BadPath {
            line: i + 1,
            path: line.to_string(),
            reason,
        };
        let mut path = line.replace('\\', "/");
        if path
            .get(..8)
            .is_some_and(|p| p.eq_ignore_ascii_case("SGWGame/"))
            && path.len() > 8
        {
            path.drain(..8);
        }
        if path.starts_with('/') || path.contains(':') {
            return Err(bad("absolute path"));
        }
        if path
            .split('/')
            .any(|c| c.is_empty() || c == "." || c == "..")
        {
            return Err(bad("empty, `.` or `..` component"));
        }
        if out.iter().any(|p| p.eq_ignore_ascii_case(&path)) {
            return Err(PackError::Duplicate(path));
        }
        out.push(path);
    }
    Ok(out)
}

/// A packed overlay.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Packed {
    pub id: String,
    pub zip: Vec<u8>,
    pub sha256: String,
    pub files: Vec<String>,
}

/// Pack the overlay in `dir`. `Ok(None)` when there is nothing to pack
/// (no directory, no list, or an empty list): the release step is then a
/// no-op, not a failure, until the overlay lands.
pub fn pack(dir: &Path, id_prefix: &str) -> Result<Option<Packed>, PackError> {
    let list_path = dir.join(LIST_FILE);
    let text = match std::fs::read_to_string(&list_path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    let mut files = parse_list(&text)?;
    if files.is_empty() {
        return Ok(None);
    }
    for f in &files {
        if !dir.join(f).is_file() {
            return Err(PackError::Missing(f.clone()));
        }
    }
    // Everything shipped is reviewed: a stray file in the overlay tree is
    // an error, not silently left out or silently shipped.
    for on_disk in walk_files(dir)? {
        let listed = files.iter().any(|f| f.eq_ignore_ascii_case(&on_disk));
        let not_shipped = NOT_SHIPPED.iter().any(|n| n.eq_ignore_ascii_case(&on_disk))
            || on_disk
                .get(..TEST_DIR.len())
                .is_some_and(|p| p.eq_ignore_ascii_case(TEST_DIR));
        if !listed && !not_shipped {
            return Err(PackError::Unlisted(on_disk));
        }
    }
    files.sort();
    let zip = write_zip(dir, &files)?;
    let sha256 = hex(&Sha256::digest(&zip));
    Ok(Some(Packed {
        id: format!("{id_prefix}-{}", &sha256[..12]),
        zip,
        sha256,
        files,
    }))
}

/// The manifest entry for `packed`, served from `blob_url`.
pub fn entry(packed: &Packed, blob_url: String, after: Option<String>) -> PatchEntry {
    PatchEntry {
        id: packed.id.clone(),
        blob: blob_url,
        size: packed.zip.len() as u64,
        sha256: packed.sha256.clone(),
        after,
        root: PatchRoot::SgwGame,
        title: Some(OVERLAY_TITLE.to_string()),
        description: Some(OVERLAY_DESCRIPTION.to_string()),
    }
}

/// What [`merge`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Merged {
    /// Added at the end, `after` the previous last patch.
    Appended,
    /// An entry with this id is already there: the same overlay bytes
    /// were published before. The manifest is unchanged.
    AlreadyPresent,
}

/// Add `entry` to `manifest`. Unless the entry names its own `after`, it
/// goes after the manifest's current last patch, so it applies on top of
/// everything published so far. Earlier overlay entries stay: installs
/// that applied them keep their record, and a fresh install applies them
/// in order with the newest last.
pub fn merge(manifest: &mut Manifest, mut entry: PatchEntry) -> Result<Merged, PackError> {
    if manifest.patches.iter().any(|p| p.id == entry.id) {
        return Ok(Merged::AlreadyPresent);
    }
    if entry.after.is_none() {
        entry.after = manifest.patches.last().map(|p| p.id.clone());
    }
    manifest.patches.push(entry);
    if let Err(e) = manifest.validate() {
        manifest.patches.pop();
        return Err(e.into());
    }
    Ok(Merged::Appended)
}

/// Every file below `dir`, as `/`-separated relative paths, sorted.
fn walk_files(dir: &Path) -> std::io::Result<Vec<String>> {
    let mut out = Vec::new();
    for entry in walkdir::WalkDir::new(dir).sort_by_file_name() {
        let entry = entry.map_err(std::io::Error::other)?;
        if !entry.file_type().is_file() {
            continue;
        }
        let rel = entry
            .path()
            .strip_prefix(dir)
            .map_err(std::io::Error::other)?;
        let parts: Vec<String> = rel
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect();
        out.push(parts.join("/"));
    }
    Ok(out)
}

fn write_zip(dir: &Path, files: &[String]) -> Result<Vec<u8>, PackError> {
    use zip::write::SimpleFileOptions;

    // A fixed timestamp (the zip epoch) keeps the bytes, and so the id,
    // a function of the overlay's content alone.
    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .last_modified_time(zip::DateTime::default())
        .unix_permissions(0o644);
    let mut zw = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for f in files {
        let bytes = std::fs::read(dir.join(PathBuf::from(f)))?;
        zw.start_file(f.as_str(), options)?;
        zw.write_all(&bytes)?;
    }
    Ok(zw.finish()?.into_inner())
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::SeedEntry;

    /// A stand-in overlay with the path contract of the real one (UI
    /// files below `Working/SGWGame/`), until the overlay lands on main.
    fn fixture(dir: &Path) {
        let files = [
            ("UI/Scripts/BlackMarket/CimmeriaBM.lua", "CimmeriaBM = {}\n"),
            ("UI/Scripts/BlackMarket/strings.lua", "-- error text\n"),
        ];
        for (rel, body) in files {
            let p = dir.join(rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, body).unwrap();
        }
        std::fs::write(
            dir.join(LIST_FILE),
            "# Black Market overlay\r\n\
             UI/Scripts/BlackMarket/strings.lua\r\n\
             \r\n\
             UI\\Scripts\\BlackMarket\\CimmeriaBM.lua\r\n",
        )
        .unwrap();
        std::fs::write(dir.join("README.md"), "docs, never shipped").unwrap();
        std::fs::create_dir_all(dir.join("test")).unwrap();
        std::fs::write(dir.join("test/overlay_spec.lua"), "-- tests, never shipped").unwrap();
    }

    fn manifest(patches: Vec<PatchEntry>) -> Manifest {
        Manifest {
            schema: 1,
            seed: SeedEntry {
                blob: "seed.zip".into(),
                size: 1,
                sha256: "s".into(),
            },
            patches,
        }
    }

    fn plain_patch(id: &str, after: Option<&str>) -> PatchEntry {
        PatchEntry {
            id: id.into(),
            blob: format!("{id}.zip"),
            size: 1,
            sha256: "h".into(),
            after: after.map(str::to_string),
            root: PatchRoot::InstallDir,
            title: None,
            description: None,
        }
    }

    #[test]
    fn list_skips_comments_and_normalises_paths() {
        let list = parse_list("# c\n\n  a/b.lua  \nSGWGame\\c\\d.lua\n").unwrap();
        assert_eq!(list, vec!["a/b.lua", "c/d.lua"]);
    }

    #[test]
    fn list_refuses_escapes_and_duplicates() {
        for bad in ["../x.lua", "/abs.lua", "C:/x.lua", "a//b.lua", "a/./b.lua"] {
            assert!(
                matches!(parse_list(bad), Err(PackError::BadPath { .. })),
                "{bad} must be refused"
            );
        }
        assert!(matches!(
            parse_list("a.lua\nA.lua\n"),
            Err(PackError::Duplicate(_))
        ));
    }

    #[test]
    fn packs_listed_files_with_a_content_hash_id() {
        let dir = tempfile::tempdir().unwrap();
        fixture(dir.path());
        let packed = pack(dir.path(), DEFAULT_ID_PREFIX).unwrap().unwrap();
        assert_eq!(
            packed.files,
            vec![
                "UI/Scripts/BlackMarket/CimmeriaBM.lua",
                "UI/Scripts/BlackMarket/strings.lua"
            ]
        );
        assert_eq!(packed.id, format!("bm-ui-overlay-{}", &packed.sha256[..12]));
        assert_eq!(packed.sha256, hex(&Sha256::digest(&packed.zip)));

        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(&packed.zip)).unwrap();
        let names: Vec<String> = archive.file_names().map(str::to_string).collect();
        assert_eq!(
            names, packed.files,
            "README.md, MANIFEST.txt and test/ never ship"
        );
        let mut body = String::new();
        std::io::Read::read_to_string(
            &mut archive
                .by_name("UI/Scripts/BlackMarket/CimmeriaBM.lua")
                .unwrap(),
            &mut body,
        )
        .unwrap();
        assert_eq!(body, "CimmeriaBM = {}\n");
    }

    /// Overlay files ship byte for byte: CRLF line endings stay CRLF.
    #[test]
    fn files_ship_byte_for_byte() {
        let dir = tempfile::tempdir().unwrap();
        let body = b"-- a\r\n<Layout/>\r\n\xff";
        std::fs::create_dir_all(dir.path().join("Content/UI")).unwrap();
        std::fs::write(dir.path().join("Content/UI/BM.layout"), body).unwrap();
        std::fs::write(dir.path().join(LIST_FILE), "Content/UI/BM.layout\n").unwrap();
        let packed = pack(dir.path(), "x").unwrap().unwrap();
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(&packed.zip)).unwrap();
        let mut out = Vec::new();
        std::io::Read::read_to_end(
            &mut archive.by_name("Content/UI/BM.layout").unwrap(),
            &mut out,
        )
        .unwrap();
        assert_eq!(out, body);
    }

    /// Same overlay, same bytes, same id: republishing is a no-op.
    #[test]
    fn packing_is_deterministic() {
        let a = tempfile::tempdir().unwrap();
        let b = tempfile::tempdir().unwrap();
        fixture(a.path());
        fixture(b.path());
        assert_eq!(
            pack(a.path(), "x").unwrap().unwrap(),
            pack(b.path(), "x").unwrap().unwrap()
        );
    }

    #[test]
    fn a_changed_file_changes_the_id() {
        let dir = tempfile::tempdir().unwrap();
        fixture(dir.path());
        let before = pack(dir.path(), "x").unwrap().unwrap();
        std::fs::write(
            dir.path().join("UI/Scripts/BlackMarket/strings.lua"),
            "-- new text\n",
        )
        .unwrap();
        let after = pack(dir.path(), "x").unwrap().unwrap();
        assert_ne!(before.id, after.id);
    }

    #[test]
    fn nothing_to_pack_is_a_no_op() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(pack(&dir.path().join("absent"), "x").unwrap(), None);
        assert_eq!(pack(dir.path(), "x").unwrap(), None, "no list file");
        std::fs::write(dir.path().join(LIST_FILE), "# nothing yet\n").unwrap();
        assert_eq!(pack(dir.path(), "x").unwrap(), None, "empty list");
    }

    #[test]
    fn a_listed_file_that_is_missing_fails() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(LIST_FILE), "UI/gone.lua\n").unwrap();
        assert!(matches!(
            pack(dir.path(), "x"),
            Err(PackError::Missing(p)) if p == "UI/gone.lua"
        ));
    }

    #[test]
    fn an_unlisted_file_fails() {
        let dir = tempfile::tempdir().unwrap();
        fixture(dir.path());
        std::fs::write(dir.path().join("UI/stray.lua"), "x").unwrap();
        assert!(matches!(
            pack(dir.path(), "x"),
            Err(PackError::Unlisted(p)) if p == "UI/stray.lua"
        ));
    }

    #[test]
    fn entry_extracts_into_sgwgame() {
        let dir = tempfile::tempdir().unwrap();
        fixture(dir.path());
        let packed = pack(dir.path(), DEFAULT_ID_PREFIX).unwrap().unwrap();
        let e = entry(&packed, "https://example.test/o.zip".into(), None);
        assert_eq!(e.root, PatchRoot::SgwGame);
        assert_eq!(e.size, packed.zip.len() as u64);
        assert_eq!(e.sha256, packed.sha256);
        let json = serde_json::to_value(&e).unwrap();
        assert_eq!(json["root"], "sgw_game");
        assert_eq!(json["id"], packed.id.as_str());
        // The launcher's "Changes to your client" list reads these.
        assert_eq!(json["title"], OVERLAY_TITLE);
        assert_eq!(json["description"], OVERLAY_DESCRIPTION);
    }

    #[test]
    fn merge_appends_after_the_last_patch() {
        let mut m = manifest(vec![
            plain_patch("001-base", None),
            plain_patch("002-x", Some("001-base")),
        ]);
        let e = PatchEntry {
            root: PatchRoot::SgwGame,
            ..plain_patch("bm-ui-overlay-aaaaaaaaaaaa", None)
        };
        assert_eq!(merge(&mut m, e).unwrap(), Merged::Appended);
        let last = m.patches.last().unwrap();
        assert_eq!(last.after.as_deref(), Some("002-x"));
        assert_eq!(last.root, PatchRoot::SgwGame);
    }

    #[test]
    fn merge_into_a_manifest_with_no_patches_has_no_after() {
        let mut m = manifest(vec![]);
        merge(&mut m, plain_patch("o", None)).unwrap();
        assert_eq!(m.patches[0].after, None);
    }

    #[test]
    fn merge_of_a_published_id_is_a_no_op() {
        let mut m = manifest(vec![plain_patch("o", None)]);
        let before = m.patches.clone();
        assert_eq!(
            merge(&mut m, plain_patch("o", None)).unwrap(),
            Merged::AlreadyPresent
        );
        assert_eq!(m.patches, before);
    }

    #[test]
    fn merge_refuses_an_after_that_does_not_exist() {
        let mut m = manifest(vec![plain_patch("a", None)]);
        let err = merge(&mut m, plain_patch("o", Some("nope"))).unwrap_err();
        assert!(matches!(err, PackError::Manifest(_)), "{err}");
    }
}
