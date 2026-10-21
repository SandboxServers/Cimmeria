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
