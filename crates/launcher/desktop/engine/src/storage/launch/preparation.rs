//! Validate the existing preparation destinations before reusing client setup.
use super::*;
pub(super) fn prepare(plan: &Plan) -> Result<PathBuf, IntentError> {
    let game = plan.installation.destination.join("game");
    if game.canonicalize().map_err(|_| StorageError::UnsafeFile)? != game {
        return Err(StorageError::UnsafeFile.into());
    }
    let binaries = crate::install_layout::binaries_dir(&game);
    let exe = crate::install_layout::sgw_exe(&game);
    let login = crate::client_setup::login_servers::path(&game);
    for path in [&binaries, &exe, &login] {
        contained(&game, path)?;
    }
    for target in crate::client_setup::stock_case::PATCH_TARGETS {
        let target = cimmeria_patchset::resolve_existing_case(&game, Path::new(target));
        contained(&game, &target)?;
    }
    crate::client_setup::prepare(&game, &plan.installation.login_servers)
        .map_err(|_| StorageError::Io)?;
    binaries
        .canonicalize()
        .map_err(|_| StorageError::UnsafeFile.into())
}
fn contained(root: &Path, target: &Path) -> Result<(), IntentError> {
    let relative = target
        .strip_prefix(root)
        .map_err(|_| StorageError::UnsafeFile)?;
    let mut path = root.to_path_buf();
    for component in relative.components() {
        if !matches!(component, std::path::Component::Normal(_)) {
            return Err(StorageError::UnsafeFile.into());
        }
        path.push(component);
        match std::fs::symlink_metadata(&path) {
            Ok(meta) => {
                #[cfg(windows)]
                {
                    use std::os::windows::fs::MetadataExt;
                    if meta.file_attributes() & 0x400 != 0 {
                        return Err(StorageError::UnsafeFile.into());
                    }
                }
                if meta.file_type().is_symlink() || (!meta.is_dir() && !meta.is_file()) {
                    return Err(StorageError::UnsafeFile.into());
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => break,
            Err(_) => return Err(StorageError::UnsafeFile.into()),
        }
    }
    Ok(())
}
#[cfg(all(test, unix))]
mod tests {
    use super::*;
    #[test]
    fn refuses_nested_link_before_legacy_preparation_can_write_through_it() {
        let root = tempfile::tempdir().unwrap();
        let root = root.path().canonicalize().unwrap();
        let game = root.join("game");
        let outside = root.join("outside");
        std::fs::create_dir(&game).unwrap();
        std::fs::create_dir(&outside).unwrap();
        std::os::unix::fs::symlink(&outside, game.join("Working")).unwrap();
        assert!(contained(&game, &game.join("Working/Binaries/SGW.exe")).is_err());
        assert!(contained(&game, &outside.join("file")).is_err());
        assert!(contained(&game, &game.join("new/file")).is_ok());
    }
}
