use super::*;

/// Refuses a Copy-mode source whose layout copy cannot map. A copy maps no paths,
/// so a flat folder, or a legacy import that names `Working` or `Binaries`, would
/// classify every file as Missing. The folder must be the installer's
/// `Working\Binaries` layout, and a root `SGW.exe` beside it is ambiguous.
pub(super) fn copy_layout(source_root: &Path, scanned: &inventory::Index) -> Result<(), Error> {
    if let Some(name) = scanned.keys().find(|p| p.eq_ignore_ascii_case("SGW.exe")) {
        let check = rejection::Rejection::new("exe_at_root")
            .entry(name.as_str())
            .expected("no SGW.exe at the root")
            .found("SGW.exe at the root");
        return Err(rejection::reject(
            StorageError::InvalidDirectory.into(),
            check,
        ));
    }
    if super::in_place_preview::full_layout(source_root) {
        return Ok(());
    }
    tracing::warn!(target: "adoption", "copy adoption refused: the folder is not in the installer's Working\\Binaries layout");
    Err(Error::UnsupportedLayout)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sgw_exe_at_the_root_is_refused_as_exe_at_root() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("SGW.exe"), b"not a game").unwrap();
        let scanned = inventory::scan(dir.path(), &CancellationToken::new()).unwrap();
        let (result, record) = rejection::scope(|| copy_layout(dir.path(), &scanned));
        assert!(matches!(
            result,
            Err(Error::Storage(StorageError::InvalidDirectory))
        ));
        let record = record.expect("a recorded rejection");
        assert_eq!(record.check, "exe_at_root");
        assert_eq!(record.entry.as_deref(), Some("SGW.exe"));
    }
}
