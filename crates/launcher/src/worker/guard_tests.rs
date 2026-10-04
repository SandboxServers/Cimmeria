//! The worker refuses conflicting commands and reports the game's exit.
//! Each test fails if its guard or notification is removed.

use std::path::Path;

use super::tests::{fake_manifest, make_worker, recv_matching};
use super::{Busy, Command, Event, LaunchSgwRequest};
use crate::config::{ClientPatchesSettings, LauncherConfig};

fn plain_launch(dir: &Path) -> Command {
    Command::LaunchSgw(LaunchSgwRequest {
        prep: None,
        install_dir: dir.to_path_buf(),
        client_patches: ClientPatchesSettings {
            enabled: false,
            dll_override: None,
        },
        telemetry: None,
    })
}

fn is_launch_outcome(e: &Event) -> bool {
    matches!(
        e,
        Event::Launched(..) | Event::LaunchError(_) | Event::Refused { .. }
    )
}

// Bug shape: a failed launch left the game slot claimed, so every later
// Play was refused as "already starting".
#[test]
fn a_failed_launch_frees_the_game_slot() {
    let dir = tempfile::tempdir().unwrap();
    let (mut worker, rt) = make_worker();
    for attempt in 0..2 {
        worker.dispatch(plain_launch(dir.path()));
        let ev = rt.block_on(recv_matching(&mut worker.events_rx, is_launch_outcome));
        assert!(
            matches!(ev, Event::LaunchError(_)),
            "attempt {attempt}: expected the missing-game error, got {ev:?}"
        );
    }
}

#[test]
fn a_launch_while_one_is_starting_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let (mut worker, rt) = make_worker();
    worker.activity.begin_launch(dir.path()).unwrap();
    worker.dispatch(plain_launch(dir.path()));
    let ev = rt.block_on(recv_matching(&mut worker.events_rx, is_launch_outcome));
    match ev {
        Event::Refused { action, reason } => {
            assert_eq!(action, Busy::Launch);
            assert!(reason.contains("already starting"), "{reason}");
        }
        other => panic!("expected Refused, got {other:?}"),
    }
}

// Install, and adopting an install, rewrite files under the game.
#[test]
fn install_and_adopt_are_refused_while_the_game_runs() {
    let dir = tempfile::tempdir().unwrap();
    let (mut worker, rt) = make_worker();
    worker.activity.begin_launch(dir.path()).unwrap();
    worker.activity.game_started(4242);

    let config = LauncherConfig {
        install_path: dir.path().to_path_buf(),
        ..LauncherConfig::default()
    };
    worker.dispatch(Command::Install {
        config,
        manifest: fake_manifest(),
    });
    worker.dispatch(Command::AdoptExisting {
        install_dir: dir.path().to_path_buf(),
        manifest: fake_manifest(),
    });
    for _ in 0..2 {
        let ev = rt.block_on(recv_matching(&mut worker.events_rx, |e| {
            matches!(
                e,
                Event::Refused { .. }
                    | Event::InstallStarted
                    | Event::AdoptComplete
                    | Event::AdoptError(_)
            )
        }));
        match ev {
            Event::Refused { action, reason } => {
                assert_eq!(action, Busy::Install);
                assert!(reason.contains("4242"), "{reason}");
            }
            other => panic!("expected Refused, got {other:?}"),
        }
    }
}

// Bug shape: a launch without telemetry dropped the game's exit
// future, so the launcher never learned the game had closed and could
// not return to Play from real evidence.
#[cfg(windows)]
#[test]
fn a_plain_launch_reports_the_exit_and_frees_the_slot() {
    let dir = tempfile::tempdir().unwrap();
    let sys = std::path::PathBuf::from(std::env::var_os("SystemRoot").unwrap()).join("System32");
    std::fs::copy(sys.join("HOSTNAME.EXE"), dir.path().join("SGW.exe")).unwrap();
    let (mut worker, rt) = make_worker();
    worker.dispatch(plain_launch(dir.path()));
    let launched = rt.block_on(recv_matching(&mut worker.events_rx, is_launch_outcome));
    let Event::Launched(_, pid) = launched else {
        panic!("expected Launched, got {launched:?}");
    };
    let exited = rt.block_on(async {
        tokio::time::timeout(std::time::Duration::from_secs(10), async {
            loop {
                if let Some(Event::GameExited { pid, exit_code }) = worker.events_rx.recv().await {
                    return (pid, exit_code);
                }
            }
        })
        .await
        .expect("no GameExited within 10 s")
    });
    assert_eq!(exited.0, pid);
    assert_eq!(exited.1, Some(0));
    // The slot is free again: the next launch is not refused.
    worker.dispatch(plain_launch(dir.path()));
    let again = rt.block_on(recv_matching(&mut worker.events_rx, is_launch_outcome));
    assert!(matches!(again, Event::Launched(..)), "{again:?}");
}

// The client-state resets delete per-user files the running client
// reads; they wait for the game like any other file job.
#[test]
fn client_state_resets_are_refused_while_the_game_runs() {
    let (mut worker, rt) = make_worker();
    worker
        .activity
        .begin_launch(std::path::Path::new(""))
        .unwrap();
    for cmd in [Command::WipeClientCache, Command::WipeAllClientState] {
        worker.dispatch(cmd);
        let ev = rt.block_on(recv_matching(&mut worker.events_rx, |e| {
            matches!(
                e,
                Event::Refused { .. } | Event::Wiped { .. } | Event::WipeError(_)
            )
        }));
        assert!(
            matches!(
                ev,
                Event::Refused {
                    action: Busy::Files,
                    ..
                }
            ),
            "{ev:?}"
        );
    }
}

// Bug shape: a failed wait on the game reported "the game closed", which
// re-enabled Play and file actions while the game might still run.
#[test]
fn a_lost_wait_is_untracked_not_an_exit() {
    use crate::telemetry::process_watch::{ExitWaiter, WatchError};
    let (mut worker, rt) = make_worker();
    let lost: ExitWaiter = Box::pin(async { Err(WatchError::JoinPanic) });
    let wrapped =
        super::launch_sgw::notify_exit(lost, 77, worker.events_tx.clone(), worker.activity.clone());
    let ev = rt.block_on(async {
        let _ = wrapped.await;
        recv_matching(&mut worker.events_rx, |e| {
            matches!(e, Event::GameExited { .. } | Event::GameUntracked { .. })
        })
        .await
    });
    assert!(matches!(ev, Event::GameUntracked { pid: 77 }), "{ev:?}");
}

// Bug shape: client setup (renames, LoginInternal.lua, ASLR) ran on the
// UI thread before the worker's claim, so it could rewrite files under a
// running game. Now a refused launch touches nothing.
#[test]
fn a_refused_launch_runs_no_client_setup() {
    let dir = tempfile::tempdir().unwrap();
    let (mut worker, rt) = make_worker();
    worker.activity.begin_launch(dir.path()).unwrap();
    worker.dispatch(Command::LaunchSgw(LaunchSgwRequest {
        prep: Some(super::ClientPrep {
            install_root: dir.path().to_path_buf(),
            login_servers: crate::client_setup::login_servers::default_servers(),
        }),
        install_dir: dir.path().to_path_buf(),
        client_patches: ClientPatchesSettings::default(),
        telemetry: None,
    }));
    let ev = rt.block_on(recv_matching(&mut worker.events_rx, |e| {
        matches!(
            e,
            Event::Refused { .. } | Event::SetupNote(_) | Event::LaunchError(_)
        )
    }));
    assert!(matches!(ev, Event::Refused { .. }), "{ev:?}");
    assert!(
        std::fs::read_dir(dir.path()).unwrap().next().is_none(),
        "no file may be written by a refused launch"
    );
}
