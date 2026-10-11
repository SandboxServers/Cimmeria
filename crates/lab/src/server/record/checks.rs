//! The record tools' argument checks (LR-07 review): each value that ends
//! up in a file name, a path or a draft's SQL is held to a fixed shape when
//! it arrives, so a bad one is refused at once (exit 2) rather than at stop,
//! and every answer that echoes it stays under the 400-character cap.

use std::path::PathBuf;

use super::run::{refused, Failure};

/// Longest section id. Every committed spec's id fits (`ability-mechanics`,
/// the longest today, is 17).
pub(super) const SECTION_MAX: usize = 48;
/// Longest `character_expr`.
pub(super) const CHARACTER_EXPR_MAX: usize = 64;

/// `^[a-z0-9][a-z0-9-]{0,47}$`, the shape of the `[section] id`s in
/// `docs/guides/uat-specs/*.toml`.
pub(super) fn check_section(s: &str) -> Result<(), Failure> {
    let ok = s.len() <= SECTION_MAX
        && s.bytes().next().is_some_and(|b| b.is_ascii_alphanumeric())
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-');
    if ok {
        Ok(())
    } else {
        Err(refused(format!(
            "section: give a section id, ^[a-z0-9][a-z0-9-]{{0,47}}$ (first-session); got {s:?}"
        )))
    }
}

/// A local directory that exists, canonicalized. UNC (`\\server\share`)
/// and verbatim (`\\?\`) paths are refused: the draft is written there, and
/// a share is not the lab's to write to.
pub(super) fn check_specs_dir(dir: Option<PathBuf>) -> Result<Option<PathBuf>, Failure> {
    let Some(dir) = dir else {
        return Ok(None);
    };
    let text = dir.to_string_lossy();
    if text.starts_with(r"\\?\") || text.starts_with("//?/") {
        return Err(refused("specs_dir: a verbatim path (\\\\?\\) is refused"));
    }
    if text.starts_with(r"\\") || text.starts_with("//") {
        return Err(refused(
            "specs_dir: a UNC path (\\\\server\\share) is refused",
        ));
    }
    if !dir.is_dir() {
        return Err(refused(format!(
            "specs_dir: {text} is not an existing directory"
        )));
    }
    dir.canonicalize()
        .map(Some)
        .map_err(|e| refused(format!("specs_dir: {text}: {e}")))
}

/// `^[A-Za-z0-9 _${}]{1,64}$`: it is pasted between the draft SQL's quotes
/// as it is (`Px${run_id}`), so no quote, backslash or semicolon.
pub(super) fn check_character_expr(e: Option<&str>) -> Result<(), Failure> {
    let Some(e) = e else {
        return Ok(());
    };
    let ok = !e.is_empty()
        && e.len() <= CHARACTER_EXPR_MAX
        && e.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b" _${}".contains(&b));
    if ok {
        Ok(())
    } else {
        Err(refused(
            "character_expr: give ^[A-Za-z0-9 _${}]{1,64}$ (Px${run_id})",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn section_ids_are_lower_case_dashed_and_short() {
        for ok in [
            "first-session",
            "ability-mechanics",
            "a",
            "0x",
            &"a".repeat(48),
        ] {
            assert!(check_section(ok).is_ok(), "{ok}");
        }
        for bad in [
            "",
            "-first",
            "First-Session",
            "first session",
            "first_session",
            "../escape",
            &"a".repeat(49),
        ] {
            let f = check_section(bad).unwrap_err();
            assert_eq!(f.exit, 2, "{bad}");
        }
    }

    /// Every committed spec's section id passes.
    #[test]
    fn every_committed_section_id_passes() {
        let dir =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/guides/uat-specs");
        let sections = crate::uat::load_sections(&dir, None).expect("specs load");
        assert!(!sections.is_empty());
        for s in sections {
            let id = &s.spec.section.id;
            assert!(check_section(id).is_ok(), "{id}");
        }
    }

    #[test]
    fn specs_dir_must_be_a_local_existing_directory() {
        assert_eq!(check_specs_dir(None).unwrap(), None);
        let root = tempfile::tempdir().unwrap();
        let got = check_specs_dir(Some(root.path().to_path_buf()))
            .unwrap()
            .unwrap();
        assert_eq!(got, root.path().canonicalize().unwrap());
        let missing = root.path().join("nope");
        assert!(check_specs_dir(Some(missing))
            .unwrap_err()
            .error
            .contains("not an existing directory"));
        let file = root.path().join("f.toml");
        std::fs::write(&file, "").unwrap();
        assert_eq!(check_specs_dir(Some(file)).unwrap_err().exit, 2);
        let e = check_specs_dir(Some(PathBuf::from(r"\\server\share\specs"))).unwrap_err();
        assert!(e.error.contains("UNC"), "{}", e.error);
        let e = check_specs_dir(Some(PathBuf::from(r"\\?\C:\specs"))).unwrap_err();
        assert!(e.error.contains("verbatim"), "{}", e.error);
    }

    #[test]
    fn character_expr_is_a_plain_template() {
        assert!(check_character_expr(None).is_ok());
        for ok in ["${character}", "Px${run_id}", "Lab one_2", &"a".repeat(64)] {
            assert!(check_character_expr(Some(ok)).is_ok(), "{ok}");
        }
        for bad in [
            "",
            "x'; DROP TABLE sgw_player; --",
            "a\"b",
            r"a\b",
            &"a".repeat(65),
        ] {
            assert_eq!(
                check_character_expr(Some(bad)).unwrap_err().exit,
                2,
                "{bad}"
            );
        }
    }
}
