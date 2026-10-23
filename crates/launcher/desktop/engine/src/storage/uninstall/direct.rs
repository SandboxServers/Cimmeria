//! Selective direct-root uninstall: preserve the chosen folder and user extras.
use super::*;
use crate::storage::adoption::direct_paths::Parents;
use std::collections::BTreeSet;

fn storage(error: adoption::Error) -> StorageError {
    match error {
        adoption::Error::Storage(error) => error,
        _ => StorageError::Corrupt,
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Selection {
    plan_digest: [u8; 32],
    paths: BTreeSet<PathBuf>,
}
impl DesktopState {
    pub(super) fn remove_direct(&mut self, plan: &Plan) -> Result<(), IntentError> {
        let root = &plan.installation.destination;
        let marker = root.join(".cimmeria-install.json");
        let name = format!("uninstall-direct-files-{}.json", plan.id);
        if !marker.try_exists().map_err(|_| StorageError::Io)? {
            // The marker is always removed last; finishing after that is safe.
            self.forget_installed_content(&plan.installation)?;
            return Ok(());
        }
        let owner = inspect_tree(
            &self.directory.root,
            root,
            &plan.installation,
            plan.release_identity(),
            true,
            true,
        )?
        .ok_or(StorageError::Corrupt)?;
        let selection = match read::<Selection>(&self.directory.root.join(&name))? {
            Some(saved) if saved.plan_digest == plan.digest()? => saved,
            Some(_) => return Err(StorageError::Corrupt.into()),
            None => {
                let mut paths: BTreeSet<PathBuf> = self
                    .direct_adopted_files(&plan.installation)?
                    .into_iter()
                    .collect();
                // Completed reconstruction journals register newly installed release
                // files without treating the user's directory as a managed tree.
                for entry in
                    std::fs::read_dir(&self.directory.root).map_err(|_| StorageError::Io)?
                {
                    let entry = entry.map_err(|_| StorageError::Io)?;
                    let filename = entry.file_name();
                    let filename = filename.to_string_lossy();
                    if filename.starts_with("repair-plan-") {
                        let saved: repair::Plan =
                            read(&entry.path())?.ok_or(StorageError::Corrupt)?;
                        if saved.installation != plan.installation {
                            continue;
                        }
                        if let Some(committed) = read::<repair::commit::Record>(
                            &self
                                .directory
                                .root
                                .join(format!("repair-commit-{}.json", saved.id)),
                        )? {
                            if committed.plan == saved
                                && committed.phase == repair::commit::Phase::Published
                            {
                                paths.extend(repair::direct_files::paths(
                                    &saved,
                                    &saved.work_directory(),
                                )?);
                            }
                        }
                    } else if filename.starts_with("update-plan-") {
                        let saved: update::Plan =
                            read(&entry.path())?.ok_or(StorageError::Corrupt)?;
                        if saved.owner != plan.installation {
                            continue;
                        }
                        if let Some(committed) = read::<update::commit::Record>(
                            &self
                                .directory
                                .root
                                .join(format!("update-commit-{}.json", saved.id)),
                        )? {
                            if committed.plan == saved
                                && committed.phase == update::commit::Phase::Published
                            {
                                paths.extend(repair::direct_files::paths(
                                    &saved,
                                    &saved.work_directory(),
                                )?);
                            }
                        }
                    }
                }
                let saved = Selection {
                    plan_digest: plan.digest()?,
                    paths,
                };
                let written = atomic::write(&self.directory.root, &name, &saved);
                self.preferences_uncertain |= written == Err(StorageError::PersistenceUncertain);
                written?;
                saved
            }
        };
        let parents = remove_selected(root, &selection.paths, &owner, || Ok(()))?;
        if let Some(id) = installed_content::adopted_work(&self.directory.root, &plan.installation)?
        {
            let work = root.join(format!(".cimmeria-adopt-{id}"));
            if repair::directory_or_absent(&work)? {
                // The exact work ID comes from this installation's retained
                // provenance. Original-file backups and other folders stay.
                failed_cleanup::validate_tree(&work)?;
                parents.remove_tree(&work).map_err(storage)?;
                sync(root)?;
            }
        }
        parents
            .remove(&root.join("content-ready.json"), false)
            .map_err(storage)?;
        drop(owner);
        parents.remove(&marker, false).map_err(storage)?;
        sync(root)?;
        self.forget_installed_content(&plan.installation)?;
        Ok(())
    }
}
fn validate(root: &Path, relative: &Path) -> Result<(), StorageError> {
    if relative.as_os_str().is_empty()
        || relative
            .components()
            .any(|part| !matches!(part, std::path::Component::Normal(_)))
        || relative
            .components()
            .next()
            .is_some_and(|part| part.as_os_str().to_string_lossy().starts_with(".cimmeria"))
        || relative == Path::new("content-ready.json")
    {
        return Err(StorageError::UnsafeFile);
    }
    let mut path = root.to_path_buf();
    for part in relative.components() {
        path.push(part);
        match std::fs::symlink_metadata(&path) {
            Ok(metadata) => {
                repair::ordinary(&metadata)?;
                if path == root.join(relative) && !metadata.is_file() {
                    return Err(StorageError::UnsafeFile);
                }
            }
            Err(error) if error.kind() == ErrorKind::NotFound => break,
            Err(_) => return Err(StorageError::Io),
        }
    }
    Ok(())
}

// Keep ordinary parent handles from selection validation through every unlink.
fn remove_selected(
    root: &Path,
    paths: &BTreeSet<PathBuf>,
    owner: &File,
    after_validation: impl FnOnce() -> Result<(), StorageError>,
) -> Result<Parents, StorageError> {
    let absolute: Vec<_> = paths.iter().map(|path| root.join(path)).collect();
    let borrowed: Vec<_> = absolute.iter().map(PathBuf::as_path).collect();
    let parents = Parents::for_owner(root, &borrowed, false, owner).map_err(storage)?;
    for relative in paths {
        validate(root, relative)?;
    }
    after_validation()?;
    for path in &absolute {
        parents.remove(path, false).map_err(storage)?;
        // Preserve the existing unlink durability boundary on ordinary paths.
        sync(path.parent().ok_or(StorageError::UnsafeFile)?)?;
    }
    Ok(parents)
}
#[cfg(test)]
mod tests;
