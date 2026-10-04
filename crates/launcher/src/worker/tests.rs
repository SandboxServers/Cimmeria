//! Worker dispatch tests: adopt and the client-state wipes, plus the
//! shared helpers the other worker test files use.

use super::*;
use crate::manifest::{Manifest, SeedEntry};
use std::time::Duration;
use tokio::time::timeout;

/// 1-second deadline on every event-recv. If the worker dispatch
/// regresses to never-emit, the test fails loudly instead of
/// hanging the suite. Adjust upward only if a real platform-slow
/// path appears.
pub(super) const RECV_TIMEOUT: Duration = Duration::from_secs(1);

pub(super) fn fake_manifest() -> Manifest {
    Manifest {
        schema: 1,
        min_launcher: None,
        seed: SeedEntry {
            blob: "seed/x.zip".into(),
            size: 1,
            sha256: "manifest-seed-hash".into(),
        },
        patches: vec![],
    }
}

/// A worker whose process probe sees no running game, so a game on
/// the developer's machine never changes a test's outcome.
pub(super) fn make_worker() -> (Worker, Arc<Runtime>) {
    let rt = Arc::new(Runtime::new().unwrap());
    let mut worker = Worker::new(rt.clone(), no_waker());
    worker.activity = Activity::new(Arc::new(|_| Vec::new()));
    (worker, rt)
}

/// Pull the next event matching `pred` (skipping any unrelated
/// events on the channel) within `RECV_TIMEOUT`. Returns the first
/// match. The skip-and-match shape exists because `spawn_wipe`
/// emits `Wiped` *or* `WipeError` depending on the resolved path —
/// rather than asserting "no other events," we say what we want.
pub(super) async fn recv_matching<F: Fn(&Event) -> bool>(
    rx: &mut mpsc::UnboundedReceiver<Event>,
    pred: F,
) -> Event {
    loop {
        let ev = timeout(RECV_TIMEOUT, rx.recv())
            .await
            .expect("worker emitted no event before timeout")
            .expect("worker channel closed unexpectedly");
        if pred(&ev) {
            return ev;
        }
    }
}

// Adopt happy path: SGW.exe present + no prior state file → worker
// dispatches AdoptComplete and the on-disk launcher-installed.json
// carries the manifest's seed hash plus seed_adopted=true.
#[test]
fn spawn_adopt_emits_complete_and_writes_marker() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("SGW.exe"), b"fake-game").unwrap();
    let (mut worker, rt) = make_worker();
    worker.dispatch(Command::AdoptExisting {
        install_dir: dir.path().to_path_buf(),
        manifest: fake_manifest(),
    });
    let ev = rt.block_on(async {
        recv_matching(&mut worker.events_rx, |e| {
            matches!(e, Event::AdoptComplete | Event::AdoptError(_))
        })
        .await
    });
    assert!(
        matches!(ev, Event::AdoptComplete),
        "expected AdoptComplete, got {ev:?}"
    );
    let state = crate::state::InstalledState::load(dir.path());
    assert!(state.seed_adopted);
    assert_eq!(state.seed_sha256.as_deref(), Some("manifest-seed-hash"));
}

// Adopt error path: no SGW.exe → AdoptError carries the rendered
// message. Pinned shape: the worker must NOT silently swallow the
// failure (we wouldn't see it surface in the UI).
#[test]
fn spawn_adopt_emits_error_when_install_dir_empty() {
    let dir = tempfile::tempdir().unwrap();
    let (mut worker, rt) = make_worker();
    worker.dispatch(Command::AdoptExisting {
        install_dir: dir.path().to_path_buf(),
        manifest: fake_manifest(),
    });
    let ev = rt.block_on(async {
        recv_matching(&mut worker.events_rx, |e| {
            matches!(e, Event::AdoptComplete | Event::AdoptError(_))
        })
        .await
    });
    match ev {
        Event::AdoptError(msg) => assert!(
            msg.contains("SGW.exe"),
            "AdoptError message should mention the missing SGW.exe, got: {msg}"
        ),
        other => panic!("expected AdoptError, got {other:?}"),
    }
}

// Wipe cache: with USERPROFILE pointed at a temp dir containing
// populated Cache.en-US, WipeClientCache emits Wiped{kind=Cache.en-US}
// and the cache contents are gone.
//
// Env-mutation is serialized via the env_lock pattern used in other
// launcher tests; here we just save/restore around the body because
// there's only one test that mutates USERPROFILE per run path and
// tests in the same process are parallel-but-distinct via this
// restore-on-drop guard.
#[test]
fn spawn_wipe_cache_clears_cache_subdir() {
    let _g = crate::client_paths::env_test_lock().lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let cache = dir
        .path()
        .join("Documents/My Games/Firesky/SGWGame/Cache.en-US");
    std::fs::create_dir_all(&cache).unwrap();
    std::fs::write(cache.join("override.pak"), b"stale").unwrap();

    let prev_profile = std::env::var("USERPROFILE").ok();
    let prev_home = std::env::var("HOME").ok();
    std::env::set_var("USERPROFILE", dir.path());
    std::env::remove_var("HOME");

    let (mut worker, rt) = make_worker();
    worker.dispatch(Command::WipeClientCache);
    let ev = rt.block_on(async {
        recv_matching(&mut worker.events_rx, |e| {
            matches!(e, Event::Wiped { .. } | Event::WipeError(_))
        })
        .await
    });

    // Restore env BEFORE asserting so a panic doesn't leak global state.
    match prev_profile {
        Some(v) => std::env::set_var("USERPROFILE", v),
        None => std::env::remove_var("USERPROFILE"),
    }
    if let Some(v) = prev_home {
        std::env::set_var("HOME", v);
    }

    match ev {
        Event::Wiped { kind, report } => {
            assert_eq!(kind, "Cache.en-US");
            assert_eq!(report.entries_removed, 1);
            assert!(report.bytes_freed >= 5);
        }
        other => panic!("expected Wiped, got {other:?}"),
    }
    assert!(
        cache.exists(),
        "cache dir itself survives — only contents wiped"
    );
    assert!(
        std::fs::read_dir(&cache).unwrap().next().is_none(),
        "cache contents must be empty",
    );
}

// Wipe all client state: with USERPROFILE set, WipeAllClientState
// emits Wiped{kind=Firesky} and the Firesky tree is empty after.
#[test]
fn spawn_wipe_all_clears_firesky_tree() {
    let _g = crate::client_paths::env_test_lock().lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let firesky = dir.path().join("Documents/My Games/Firesky");
    std::fs::create_dir_all(firesky.join("SGWGame/Config")).unwrap();
    std::fs::write(firesky.join("SGWGame/Config/user.ini"), b"keybinds=...").unwrap();

    let prev_profile = std::env::var("USERPROFILE").ok();
    let prev_home = std::env::var("HOME").ok();
    std::env::set_var("USERPROFILE", dir.path());
    std::env::remove_var("HOME");

    let (mut worker, rt) = make_worker();
    worker.dispatch(Command::WipeAllClientState);
    let ev = rt.block_on(async {
        recv_matching(&mut worker.events_rx, |e| {
            matches!(e, Event::Wiped { .. } | Event::WipeError(_))
        })
        .await
    });

    match prev_profile {
        Some(v) => std::env::set_var("USERPROFILE", v),
        None => std::env::remove_var("USERPROFILE"),
    }
    if let Some(v) = prev_home {
        std::env::set_var("HOME", v);
    }

    match ev {
        Event::Wiped { kind, .. } => assert_eq!(kind, "Firesky"),
        other => panic!("expected Wiped{{kind=Firesky}}, got {other:?}"),
    }
    // The Firesky dir itself survives — only contents go — so a
    // running watcher in the client doesn't lose its handle.
    assert!(firesky.exists());
    assert!(std::fs::read_dir(&firesky).unwrap().next().is_none());
}

// Resolution failure path: neither USERPROFILE nor HOME set →
// WipeError instead of Wiped. Important so a CI runner with a
// weird env doesn't silently nuke files relative to cwd.
#[test]
fn spawn_wipe_emits_error_when_no_profile_env() {
    let _g = crate::client_paths::env_test_lock().lock().unwrap();
    let prev_profile = std::env::var("USERPROFILE").ok();
    let prev_home = std::env::var("HOME").ok();
    std::env::remove_var("USERPROFILE");
    std::env::remove_var("HOME");

    let (mut worker, rt) = make_worker();
    worker.dispatch(Command::WipeClientCache);
    let ev = rt.block_on(async {
        recv_matching(&mut worker.events_rx, |e| {
            matches!(e, Event::Wiped { .. } | Event::WipeError(_))
        })
        .await
    });

    if let Some(v) = prev_profile {
        std::env::set_var("USERPROFILE", v);
    }
    if let Some(v) = prev_home {
        std::env::set_var("HOME", v);
    }

    match ev {
        Event::WipeError(msg) => assert!(
            msg.contains("USERPROFILE") || msg.contains("HOME"),
            "WipeError should explain which env var was missing, got: {msg}"
        ),
        other => panic!("expected WipeError, got {other:?}"),
    }
}
