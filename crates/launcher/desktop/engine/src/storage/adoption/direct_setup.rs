//! Direct-root setup writes through retained parent directory handles.
use super::direct_paths::Parents;
use super::*;

pub(super) fn prepare(plan: &Plan) -> Result<(), Error> {
    let root = &plan.installation.destination;
    for relative in crate::client_setup::stock_case::PATCH_TARGETS {
        let target = inventory::native(root, relative);
        // Do not create directories for absent optional patch outputs.
        let guard = Parents::new(plan, &[&target], false)?;
        let Ok(target) = guard.path(&target) else {
            continue;
        };
        let parent = target.parent().ok_or(StorageError::UnsafeFile)?;
        let name = target.file_name().ok_or(StorageError::UnsafeFile)?;
        let Ok(entries) = std::fs::read_dir(parent) else {
            continue;
        };
        let mut exact = false;
        let mut wrong = None;
        for entry in entries {
            let entry = entry?;
            if entry.file_name() == name {
                exact = true;
            } else if entry
                .file_name()
                .to_string_lossy()
                .eq_ignore_ascii_case(&name.to_string_lossy())
            {
                wrong = Some(entry.path());
            }
        }
        if let (false, Some(from)) = (exact, wrong) {
            super::in_place::present(&from)?;
            std::fs::rename(from, &target)?;
        }
    }
    let login = crate::client_setup::login_servers::path(root);
    let exe = crate::install_layout::sgw_exe(root);
    let ledger = root.join("launcher-installed.json");
    let guard = Parents::new(plan, &[&login, &exe, &ledger], true)?;
    let login = guard.path(&login)?;
    if std::fs::symlink_metadata(&login).is_ok() {
        super::in_place::present(&login)?;
    }
    crate::client_setup::login_servers::validate(&plan.installation.login_servers)
        .map_err(|_| StorageError::Corrupt)?;
    let bytes = crate::client_setup::login_servers::render(&plan.installation.login_servers);
    let unchanged = if super::in_place::present(&login)? {
        let mut file = inventory::open(&login)?;
        let mut previous = Vec::new();
        file.read_to_end(&mut previous)?;
        previous == bytes.as_bytes()
    } else {
        false
    };
    if !unchanged {
        atomic::write_bytes(
            login.parent().ok_or(StorageError::UnsafeFile)?,
            "LoginInternal.lua",
            bytes.as_bytes(),
            1024 * 1024,
        )?;
    }
    super::direct_paths::edit_file(&guard.path(&exe)?, |path| {
        crate::client_setup::aslr::disable(path)?;
        Ok(())
    })?;
    Ok(())
}
