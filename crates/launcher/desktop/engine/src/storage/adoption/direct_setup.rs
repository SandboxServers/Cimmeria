//! Direct-root setup writes through retained parent directory handles.
use super::direct_paths::Parents;
use super::*;

pub(super) fn prepare(plan: &Plan) -> Result<(), Error> {
    super::setup_expectations::check(plan)?;
    let root = &plan.installation.destination;
    for relative in crate::client_setup::stock_case::PATCH_TARGETS {
        let target = inventory::native(root, relative);
        let guard = Parents::new(plan, &[&target], false)?;
        let parent = target.parent().ok_or(StorageError::UnsafeFile)?;
        let name = target.file_name().ok_or(StorageError::UnsafeFile)?;
        let Ok(names) = guard.names(parent) else {
            continue;
        };
        if !names.iter().any(|entry| entry == name) {
            if let Some(wrong) = names.into_iter().find(|entry| {
                entry
                    .to_string_lossy()
                    .eq_ignore_ascii_case(&name.to_string_lossy())
            }) {
                let source = parent.join(wrong);
                guard.present(&source)?;
                guard.rename(&source, &target, true)?;
            }
        }
    }
    let login = crate::client_setup::login_servers::path(root);
    let exe = crate::install_layout::sgw_exe(root);
    let guard = Parents::new(plan, &[&login, &exe], true)?;
    crate::client_setup::login_servers::validate(&plan.installation.login_servers)
        .map_err(|_| StorageError::Corrupt)?;
    let bytes = crate::client_setup::login_servers::render(&plan.installation.login_servers);
    if !guard.present(&login)? || guard.read(&login)? != bytes.as_bytes() {
        guard.write(&login, bytes.as_bytes())?;
    }
    crate::client_setup::aslr::disable_open(&mut guard.open(&exe, true, false)?)?;
    Ok(())
}
