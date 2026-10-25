//! One `command_failed` record for every Tauri command that returns a `JobError`
//! to the window (PD-13). The fields are closed codes: the command's name, the
//! error's code and its failure taxonomy code (C-PD1), never a path or a value
//! the player typed.
use super::JobError;

/// The launcher area a command belongs to; it picks the event's target.
// Only Install, Launch, GameUpdate and Settings commands return a `JobError`
// today; the other kinds are ready for the commands that move to it.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CommandKind {
    Install,
    Repair,
    Launch,
    GameUpdate,
    Adoption,
    Settings,
    LauncherUpdate,
    Other,
}

/// Logs that `command` returned `error` to the window. `busy` and
/// `stale_revision` are routine double-clicks and log at `info`; every other
/// error logs at `warn`.
pub(crate) fn log(kind: CommandKind, command: &'static str, error: JobError) {
    let code = error.code();
    let failure_code = error.failure().code();
    let routine = matches!(error, JobError::Busy | JobError::StaleRevision);
    // tracing needs a literal target and level, so each pair is its own call.
    macro_rules! emit {
        ($target:literal) => {
            if routine {
                tracing::info!(
                    target: $target,
                    event = "command_failed",
                    command,
                    error = code,
                    failure_code,
                    "launcher command: {command} refused; the launcher returned {code} to the window; nothing started"
                )
            } else {
                tracing::warn!(
                    target: $target,
                    event = "command_failed",
                    command,
                    error = code,
                    failure_code,
                    "launcher command: {command} refused; the launcher returned {code} to the window; nothing started"
                )
            }
        };
    }
    match kind {
        CommandKind::Install => emit!("launcher.install"),
        CommandKind::Repair => emit!("launcher.repair"),
        CommandKind::Launch => emit!("launcher.launch"),
        CommandKind::GameUpdate => emit!("launcher.game_update"),
        CommandKind::Adoption => emit!("launcher.adoption"),
        CommandKind::Settings => emit!("launcher.settings"),
        CommandKind::LauncherUpdate => emit!("launcher.update"),
        CommandKind::Other => emit!("launcher.ui"),
    }
}

#[cfg(test)]
#[path = "command_failure_tests.rs"]
mod tests;
