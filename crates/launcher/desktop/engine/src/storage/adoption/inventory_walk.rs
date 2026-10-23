//! Bounded filesystem traversal shared by source and direct-root content scans.
use super::inventory::*;
use super::*;
use std::collections::BTreeMap;
pub(super) fn scan(
    root: &Path,
    cancel: &CancellationToken,
    on_file: &mut dyn FnMut(u64),
    skip: &dyn Fn(&Path) -> bool,
) -> Result<Index, Error> {
    fn walk(
        root: &Path,
        path: &Path,
        depth: usize,
        out: &mut Index,
        folded: &mut BTreeMap<String, String>,
        cancel: &CancellationToken,
        on_file: &mut dyn FnMut(u64),
        skip: &dyn Fn(&Path) -> bool,
    ) -> Result<(), Error> {
        check_cancel(cancel)?;
        if depth > MAX_DEPTH {
            let check = rejection::Rejection::new("folder_too_deep")
                .entry(rejection::relative(root, path))
                .expected(MAX_DEPTH.to_string())
                .found(depth.to_string());
            return Err(rejection::reject(StorageError::TooLarge.into(), check));
        }
        if out.len() > MAX_FILES {
            return Err(too_large(root, path, out.len()));
        }
        identity(path)?;
        for child in std::fs::read_dir(path)? {
            if out.len() >= MAX_FILES || folded.len() >= MAX_FILES {
                return Err(too_large(root, path, folded.len()));
            }
            let child = child?;
            let path = child.path();
            if skip(&path) {
                continue;
            }
            identity(&path)?;
            // Index names use `/` on every host, so a Windows scan compares
            // with the reference and with a plan written elsewhere.
            let mut parts = Vec::new();
            for part in path
                .strip_prefix(root)
                .map_err(|_| StorageError::UnsafeFile)?
                .components()
            {
                match part {
                    std::path::Component::Normal(part) => {
                        parts.push(part.to_str().ok_or(StorageError::UnsafeFile)?)
                    }
                    _ => return Err(StorageError::UnsafeFile.into()),
                }
            }
            let name = parts.join("/");
            if !scan_name_allowed(&name) {
                return Err(StorageError::UnsafeFile.into());
            }
            if folded
                .insert(name.to_ascii_lowercase(), name.clone())
                .is_some()
            {
                return Err(StorageError::UnsafeFile.into());
            }
            if child.file_type()?.is_dir() {
                walk(root, &path, depth + 1, out, folded, cancel, on_file, skip)?;
            } else {
                let entry = hash(&path, cancel)?;
                let size = entry.size;
                out.insert(name, entry);
                on_file(size);
            }
        }
        Ok(())
    }
    let before = identity(root)?;
    let mut index = Index::new();
    walk(
        root,
        root,
        0,
        &mut index,
        &mut BTreeMap::new(),
        cancel,
        on_file,
        skip,
    )?;
    if identity(root)? != before {
        let check = rejection::Rejection::new("folder_changed_during_check")
            .entry(".")
            .expected("same folder")
            .found("replaced");
        return Err(rejection::reject(Error::SourceChanged, check));
    }
    Ok(index)
}
