//! Linux launch preparation through Proton (PP-07). It finds the selected Proton
//! again, claims the private prefix, sets it up once, and builds the helper
//! command and the game request that the worker runs with `proton runinprefix`.
use super::resource::ProtonSelection;
use super::*;
use crate::failure::FailureCode;
use crate::game_telemetry::Session;
use crate::helper_supervisor::HelperCommand;
use crate::linux_proton::discovery::{self, ProtonInstall, SteamRoots};
use crate::linux_proton::prefix::{self, Prefix, ProtonError};
use crate::linux_proton::{directx, environment, invocation, paths, setup};
use crate::storage::display_options::{self, DisplayOptions, Placement};
use cimmeria_runtime_probe::game_launch::Request;

/// Where the system-wide Proton builds live (C-1).
const SYSTEM_TOOLS: &str = "/usr/share/steam/compatibilitytools.d";

/// The folders Proton searches, as the launcher sees them. Tests replace them.
struct Search {
    home: PathBuf,
    system_tools: PathBuf,
    /// The DirectX step (C-8). Production uses `directx::ensure`; the tests
    /// replace it, because the real pins need the game's own 4 MB DLL.
    directx: fn(&Prefix, &Path) -> Result<(), ProtonError>,
}

/// Prepares a Linux Play through the selected Proton. A coded refusal is
/// `Err(PrepareError(FailureCode::…))`; anything else is `launch_prepare_failed`.
pub(super) fn prepare(
    plan: &Plan,
    state_root: &Path,
    session: Option<&Session>,
    servers: &[LoginServer],
    display: DisplayOptions,
    placements: &[Placement],
) -> Result<(HelperCommand, Request, Prefix), PrepareError> {
    let home = std::env::var_os("HOME").ok_or(PrepareError(FailureCode::ProtonMissing))?;
    let search = Search {
        home: PathBuf::from(home),
        system_tools: PathBuf::from(SYSTEM_TOOLS),
        directx: directx::ensure,
    };
    prepare_in(
        plan, state_root, session, servers, display, placements, &search,
    )
}

/// `prepare` with the Steam folders as a parameter, so the tests use a fake tree.
fn prepare_in(
    plan: &Plan,
    state_root: &Path,
    session: Option<&Session>,
    servers: &[LoginServer],
    display: DisplayOptions,
    placements: &[Placement],
    search: &Search,
) -> Result<(HelperCommand, Request, Prefix), PrepareError> {
    let selection = plan
        .resources
        .proton
        .as_ref()
        .ok_or(PrepareError(FailureCode::ProtonMissing))?;
    let (install, roots) = selected_install(selection, search)?;
    let claimed = prefix::claim(state_root).map_err(|err| match err {
        ProtonError::Busy => PrepareError::from(ContractError::Busy),
        _ => PrepareError::from(StorageError::Io),
    })?;
    // Per-launch logs (PD-28): without the folder, the launch runs without logs.
    let logs = super::game_logs::folder(state_root, plan.id).ok();
    let log_file = logs.as_ref().map(|folder| folder.join("proton.log"));
    let log_guest = logs.as_ref().and_then(|folder| paths::guest(folder).ok());
    let base = environment::base(|key| std::env::var_os(key), &claimed, &roots.client);
    let setup_log = log_file.as_deref();
    tokio::runtime::Handle::current()
        .block_on(setup::ensure(&install, &claimed, &base, setup_log))
        .map_err(|_| PrepareError(FailureCode::ProtonSetupFailed))?;
    // Every Play: a Proton prefix upgrade may rewrite syswow64 (C-8).
    (search.directx)(&claimed, &plan.installation.content_root())
        .map_err(|_| PrepareError(FailureCode::ProtonSetupFailed))?;
    // PP-08 physx, PP-17 display
    let game = plan.installation.content_root();
    // Client setup, as `wine.rs` runs it: login servers, stock names, ASLR, containment.
    preparation::prepare(plan, servers)?;
    let binaries = cimmeria_patchset::resolve_existing_case(&game, Path::new("Working/Binaries"));
    let guest = |p: &Path| {
        paths::guest(p)
            .map(PathBuf::from)
            .map_err(|_| StorageError::UnsafeFile)
    };
    let telemetry = super::telemetry::injected(plan, session);
    let request = Request {
        schema_version: 1,
        operation_id: plan.id,
        exe: guest(&binaries.join("SGW.exe"))?,
        directory: guest(&binaries)?,
        // Patches first: the telemetry DLL shares their install lock.
        dlls: plan
            .resources
            .client_patches
            .iter()
            .chain(telemetry)
            .map(|p| guest(p.path()))
            .collect::<Result<_, _>>()?,
        args: super::display::arguments(display),
        // The worker fills it from the remembered monitor (PD-33).
        place_on: None,
    };
    // The frame cap (PD-35): `Display` follows the remembered or the main display.
    let refresh_hz =
        display_options::launch_refresh_hz(placements, &crate::display_targets::list());
    let frame_rate = display_options::frame_rate(display.frame_cap, refresh_hz);
    let mut game_env = environment::game(base, frame_rate, log_guest.as_deref());
    if session.is_some() {
        for (key, value) in crate::game_telemetry::passthrough_environment() {
            game_env.insert(key.into(), value.into());
        }
    }
    let helper =
        paths::guest(plan.resources.helper.path()).map_err(|_| StorageError::UnsafeFile)?;
    let inv = invocation::runinprefix(&install, &helper, &[]);
    let spec = HelperCommand {
        executable: inv.program,
        arguments: inv.args,
        directory: binaries,
        environment: game_env,
        stderr_log: log_file,
    };
    Ok((spec, request, claimed))
}

/// Finds the selected Proton again (step 1). A build that is gone, or whose
/// `version` file changed, is `proton_missing`.
fn selected_install(
    selection: &ProtonSelection,
    search: &Search,
) -> Result<(ProtonInstall, SteamRoots), PrepareError> {
    let missing = PrepareError(FailureCode::ProtonMissing);
    let roots = discovery::steam_roots(&search.home).ok_or(missing)?;
    let install = discovery::discover(&roots, &search.system_tools)
        .into_iter()
        .find(|install| install.id == selection.id && install.version == selection.version)
        .ok_or(missing)?;
    Ok((install, roots))
}

#[cfg(test)]
#[path = "proton_tests.rs"]
mod tests;
