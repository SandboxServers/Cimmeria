//! Validate the existing preparation destinations before reusing client setup.
use super::*;
use crate::client_setup::login_servers;
use crate::storage::adoption::Provenance;
use crate::storage::InstalledContent;
impl DesktopState {
    /// The login servers Play writes into `LoginInternal.lua`. A legacy adoption
    /// keeps the list its adoption reviewed; any other install uses the saved
    /// preference, or the defaults when that is empty. Checked again here, so a
    /// list that reached `preferences.json` another way never reaches the client.
    pub(crate) fn launch_login_servers(
        &self,
        installation: &InstallIntent,
    ) -> Result<Vec<LoginServer>, IntentError> {
        let (installed, adoption) = self.installed_for_launch()?.ok_or(StorageError::Corrupt)?;
        if installed.intent != *installation {
            return Err(StorageError::Corrupt.into());
        }
        let servers = self.pick_login_servers(Some((&installed, adoption.as_ref())));
        login_servers::validate(&servers).map_err(|_| StorageError::InvalidServerList)?;
        Ok(servers)
    }

    /// The list Play writes: a legacy adoption's reviewed list, else the saved
    /// preference, else the defaults (C18.2).
    fn pick_login_servers(
        &self,
        installed: Option<(&InstalledContent, Option<&Provenance>)>,
    ) -> Vec<LoginServer> {
        match installed {
            Some((content, Some(p))) if p.legacy_install_id.is_some() => {
                content.intent.login_servers.clone()
            }
            _ if self.preferences.login_servers.is_empty() => login_servers::default_servers(),
            _ => self.preferences.login_servers.clone(),
        }
    }

    /// The effective list without a launch intent: what sign-in offers (C18.2).
    pub(crate) fn effective_login_servers(&self) -> Result<Vec<LoginServer>, StorageError> {
        let installed = self.installed_for_launch()?;
        Ok(self.pick_login_servers(installed.as_ref().map(|(c, p)| (c, p.as_ref()))))
    }
}
pub(super) fn prepare(plan: &Plan, servers: &[LoginServer]) -> Result<PathBuf, IntentError> {
    let game = plan.installation.content_root();
    if game.canonicalize().map_err(|_| StorageError::UnsafeFile)? != game {
        return Err(StorageError::UnsafeFile.into());
    }
    let binaries = crate::install_layout::binaries_dir(&game);
    let exe = crate::install_layout::sgw_exe(&game);
    let login = login_servers::path(&game);
    for path in [&binaries, &exe, &login] {
        contained(&game, path)?;
    }
    for target in crate::client_setup::stock_case::PATCH_TARGETS {
        let target = cimmeria_patchset::resolve_existing_case(&game, Path::new(target));
        contained(&game, &target)?;
    }
    crate::client_setup::prepare(&game, servers).map_err(|_| StorageError::Io)?;
    binaries
        .canonicalize()
        .map_err(|_| StorageError::UnsafeFile.into())
}
pub(super) fn contained(root: &Path, target: &Path) -> Result<(), IntentError> {
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
