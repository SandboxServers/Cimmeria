use super::*;

/// Refuses a Copy-mode source whose layout is ambiguous: exactly one of the
/// full, working and flat layouts must hold, and the binaries directory must
/// sit under the source root.
pub(super) fn copy_layout(source_root: &Path, scanned: &inventory::Index) -> Result<(), Error> {
    let binaries = crate::install_layout::binaries_dir(source_root);
    let full = source_root.join("Working").is_dir();
    let working = std::fs::read_dir(source_root)?.any(|entry| {
        entry.is_ok_and(|entry| {
            entry.file_name().eq_ignore_ascii_case("binaries")
                && entry.file_type().is_ok_and(|kind| kind.is_dir())
        })
    });
    let flat = scanned.keys().any(|p| p.eq_ignore_ascii_case("SGW.exe"))
        || (!full && !working && source_root.join("SGWGame").is_dir());
    if [full, working, flat].into_iter().filter(|v| *v).count() != 1
        || !binaries.starts_with(source_root)
    {
        return Err(StorageError::InvalidDirectory.into());
    }
    Ok(())
}
