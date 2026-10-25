use super::*;
use crate::test_capture::{capture, Captured};
use tracing::Level;

/// The one `command_failed` record `log` writes for `error`.
fn logged(kind: CommandKind, command: &'static str, error: JobError) -> Captured {
    // A second live Dispatch keeps tracing's callsite interest per dispatcher; with one,
    // a callsite first hit by another test with no subscriber stays disabled.
    let _second = tracing::Dispatch::new(tracing_subscriber::registry());
    let ((), events) = capture(|| {
        tracing::callsite::rebuild_interest_cache();
        log(kind, command, error)
    });
    let failed: Vec<Captured> = events
        .into_iter()
        .filter(|e| e.field("event") == Some("command_failed"))
        .collect();
    assert_eq!(failed.len(), 1);
    failed.into_iter().next().unwrap()
}

#[test]
fn an_invalid_directory_logs_a_warning_with_its_codes() {
    let event = logged(
        CommandKind::Install,
        "install_command",
        JobError::InvalidDirectory,
    );
    assert_eq!(event.target, "launcher.install");
    assert_eq!(event.level, Level::WARN);
    assert_eq!(event.field("command"), Some("install_command"));
    assert_eq!(event.field("error"), Some("invalid_directory"));
    assert_eq!(event.field("failure_code"), Some("game_folder_invalid"));
}

#[test]
fn routine_double_clicks_log_at_info() {
    for error in [JobError::Busy, JobError::StaleRevision] {
        let event = logged(CommandKind::Launch, "launch_command", error);
        assert_eq!(event.level, Level::INFO, "{}", error.code());
        assert_eq!(event.field("error"), Some(error.code()));
    }
    let event = logged(CommandKind::Launch, "launch_command", JobError::Busy);
    assert_eq!(event.field("failure_code"), Some("operation_busy"));
}

#[test]
fn every_kind_logs_on_its_own_target() {
    let rows = [
        (CommandKind::Install, "launcher.install"),
        (CommandKind::Repair, "launcher.repair"),
        (CommandKind::Launch, "launcher.launch"),
        (CommandKind::GameUpdate, "launcher.game_update"),
        (CommandKind::Adoption, "launcher.adoption"),
        (CommandKind::Settings, "launcher.settings"),
        (CommandKind::LauncherUpdate, "launcher.update"),
        (CommandKind::Other, "launcher.ui"),
    ];
    for (kind, target) in rows {
        let event = logged(kind, "test_command", JobError::Io);
        assert_eq!(event.target, target);
        assert_eq!(event.level, Level::WARN);
        assert_eq!(event.field("failure_code"), Some("local_io"));
    }
}
