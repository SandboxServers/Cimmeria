//! Content validity and selected files for direct-root adoption.
use super::*;
impl DesktopState {
    pub(crate) fn direct_adopted_files(
        &self,
        intent: &InstallIntent,
    ) -> Result<Vec<PathBuf>, StorageError> {
        let record: Record = read(&self.directory.root.join(NAME))?.ok_or(StorageError::Corrupt)?;
        if record.intent != *intent || intent.schema_version != 2 {
            return Err(StorageError::Corrupt);
        }
        let provenance = adoption_shape(&record)?.ok_or(StorageError::Corrupt)?;
        let plan = crate::storage::adoption::published_plan(self, intent, provenance)?;
        let mut paths: Vec<PathBuf> =
            if provenance.check == crate::storage::adoption::FileCheck::Light {
                // Light review lists the whole source, including unverified user files.
                // Only signed replacement outputs and setup's fixed files are ours.
                let mut paths: Vec<PathBuf> = plan
                    .replacements
                    .iter()
                    .map(|file| file.path.split('/').collect())
                    .collect();
                paths.push(crate::install_layout::sgw_exe(Path::new("")));
                paths.push(crate::client_setup::login_servers::path(Path::new("")));
                paths
            } else {
                plan.report
                    .files
                    .iter()
                    .filter(|file| {
                        file.classification != crate::storage::adoption::Classification::Extra
                    })
                    .map(|file| file.path.split('/').collect())
                    .collect()
            };
        paths.push(crate::state::InstalledState::path(Path::new("")).to_path_buf());
        Ok(paths)
    }
    /// Content validity is independent of runtime prerequisites, launcher minimum,
    /// and Play admission. The authenticated content root selects the layout.
    pub fn has_valid_install(&self) -> Result<bool, StorageError> {
        match self.installed_for_launch() {
            Ok(Some((installed, _))) => Ok(install_worker::content_valid(
                &installed.intent.content_root(),
                &installed.release,
            )),
            Ok(None) | Err(StorageError::Busy) => Ok(false),
            Err(error) => Err(error),
        }
    }
}
