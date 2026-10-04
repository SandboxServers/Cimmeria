//! Reducer and guard tests for the Install/Play surface. Each
//! drives the production [`PlayState`] the way the app does.

use super::*;
use crate::manifest::{PatchEntry, PatchRoot, SeedEntry};

const URL: &str = "https://example.test/manifest.json";

fn manifest(patches: &[&str]) -> Manifest {
    Manifest {
        schema: 1,
        min_launcher: None,
        seed: SeedEntry {
            blob: "seed.zip".into(),
            size: 1,
            sha256: "seed-hash".into(),
        },
        patches: patches
            .iter()
            .map(|id| PatchEntry {
                id: (*id).into(),
                blob: format!("{id}.zip"),
                size: 1,
                sha256: "h".into(),
                after: None,
                root: PatchRoot::InstallDir,
                title: None,
                description: None,
            })
            .collect(),
    }
}

fn loaded() -> PlayState {
    let mut s = PlayState::default();
    s.manifest.begin_fetch(URL);
    s.apply(
        &Event::ManifestFetched {
            url: URL.into(),
            manifest: manifest(&["a"]),
        },
        URL,
    );
    s
}

fn inputs(status: InstallStatus) -> Inputs {
    Inputs {
        status,
        sgw_present: true,
        writable: true,
        launcher_blocked: false,
    }
}

#[test]
fn install_runs_from_click_to_complete_and_asks_for_a_refresh() {
    let mut s = loaded();
    let i = inputs(InstallStatus::NotInstalled);
    assert_eq!(primary_action(&s, &i), Primary::Install);
    s.click_install();
    assert_eq!(primary_action(&s, &i), Primary::Installing);
    assert!(file_action_block(&s).is_some());
    s.apply(&Event::InstallStarted, URL);
    s.apply(
        &Event::Progress(Progress::Downloading {
            label: "seed".into(),
            downloaded: 1,
            total: 2,
        }),
        URL,
    );
    assert!(s.progress.is_some());
    let fx = s.apply(&Event::InstallComplete, URL);
    assert!(fx.refresh_install, "the ledger decides the next state");
    assert_eq!(s.operation, None);
    assert_eq!(s.progress, None);
    assert!(file_action_block(&s).is_none());
}

// A failed or cancelled install maps back to the real state: the
// operation ends and the ledger is re-read, so Install/Update offers to
// pick up where it stopped.
#[test]
fn failure_and_cancel_end_the_install_and_refresh() {
    for (ev, error) in [
        (Event::InstallError("disk full".into()), true),
        (Event::InstallCancelled, false),
    ] {
        let mut s = loaded();
        s.click_install();
        s.apply(&Event::InstallStarted, URL);
        let fx = s.apply(&ev, URL);
        assert!(fx.refresh_install, "{ev:?}");
        assert_eq!(s.operation, None, "{ev:?}");
        assert_eq!(s.notice.as_ref().unwrap().error, error, "{ev:?}");
    }
}

// Bug shape: a refused Install left the surface stuck on "Installing".
#[test]
fn a_refused_install_returns_to_the_install_button() {
    let mut s = loaded();
    s.click_install();
    s.apply(
        &Event::Refused {
            action: Busy::Install,
            reason: "the game is running".into(),
        },
        URL,
    );
    assert_eq!(s.operation, None);
    assert_eq!(
        primary_action(&s, &inputs(InstallStatus::NotInstalled)),
        Primary::Install
    );
    assert!(s.notice.unwrap().text.contains("game is running"));
}

#[test]
fn launch_runs_to_running_and_back_to_play_on_exit() {
    for telemetry in [false, true] {
        let mut s = loaded();
        let i = inputs(InstallStatus::UpToDate);
        assert_eq!(primary_action(&s, &i), Primary::Play);
        s.click_play(telemetry);
        assert_eq!(primary_action(&s, &i), Primary::Launching);
        assert!(launch_block(&s, &i).is_some(), "no duplicate launch");
        s.apply(&Event::Launched("SGW.exe".into(), 77), URL);
        assert_eq!(primary_action(&s, &i), Primary::Running);
        assert_eq!(
            s.activity(),
            GameActivity::Running {
                telemetry: Some(telemetry)
            }
        );
        assert!(file_action_block(&s).is_some());
        s.apply(
            &Event::GameExited {
                pid: 77,
                exit_code: Some(0),
            },
            URL,
        );
        assert_eq!(primary_action(&s, &i), Primary::Play);
        assert!(file_action_block(&s).is_none());
    }
}

// Only the followed game's exit returns to Play, and a crash says so.
#[test]
fn another_pids_exit_is_ignored_and_a_crash_is_reported() {
    let mut s = loaded();
    s.click_play(false);
    s.apply(&Event::Launched("SGW.exe".into(), 5), URL);
    s.apply(
        &Event::GameExited {
            pid: 6,
            exit_code: Some(0),
        },
        URL,
    );
    assert!(matches!(s.game, Lifecycle::Running { pid: 5, .. }));
    s.apply(
        &Event::GameExited {
            pid: 5,
            exit_code: Some(-1073741819),
        },
        URL,
    );
    assert_eq!(s.game, Lifecycle::Idle);
    assert!(s.notice.unwrap().error);
}

#[test]
fn a_launch_error_or_refusal_returns_to_play() {
    for ev in [
        Event::LaunchError("SGW.exe missing".into()),
        Event::Refused {
            action: Busy::Launch,
            reason: "already starting".into(),
        },
    ] {
        let mut s = loaded();
        s.click_play(false);
        s.apply(&ev, URL);
        assert_eq!(s.game, Lifecycle::Idle, "{ev:?}");
        assert!(s.notice.as_ref().unwrap().error);
    }
}

// A game the launcher did not start (reopened window, Atera bat, an
// untracked launch) still counts as running for every guard.
#[test]
fn a_probed_game_blocks_play_and_file_actions() {
    let mut s = loaded();
    s.probed_pids = vec![9];
    let i = inputs(InstallStatus::UpToDate);
    assert_eq!(primary_action(&s, &i), Primary::Running);
    assert_eq!(s.activity(), GameActivity::Running { telemetry: None });
    assert!(launch_block(&s, &i).is_some());
    assert!(file_action_block(&s).is_some());
}

#[test]
fn an_untracked_launch_hands_over_to_the_probe() {
    let mut s = loaded();
    s.click_play(true);
    s.apply(&Event::Launched("SGW.exe".into(), 3), URL);
    s.apply(&Event::GameUntracked { pid: 3 }, URL);
    assert_eq!(s.game, Lifecycle::Idle);
    s.probed_pids = vec![3];
    assert_eq!(s.activity(), GameActivity::Running { telemetry: None });
}

// The min_launcher gate wins over every install state, as before.
#[test]
fn the_launcher_gate_blocks_install_update_and_play() {
    let s = loaded();
    for status in [
        InstallStatus::NotInstalled,
        InstallStatus::NeedsUpdate {
            seed: false,
            patches: 1,
        },
        InstallStatus::UpToDate,
    ] {
        let mut i = inputs(status);
        i.launcher_blocked = true;
        assert!(matches!(primary_action(&s, &i), Primary::Unavailable(_)));
        assert!(launch_block(&s, &i).is_some());
    }
}

#[test]
fn update_offers_play_anyway_only_when_the_game_is_there() {
    let s = loaded();
    let mut i = inputs(InstallStatus::NeedsUpdate {
        seed: false,
        patches: 2,
    });
    let p = primary_action(&s, &i);
    assert_eq!(p, Primary::Update);
    assert!(offers_play_anyway(&p, &i));
    i.sgw_present = false;
    assert!(!offers_play_anyway(&p, &i));
}

#[test]
fn a_not_writable_folder_cannot_install() {
    let s = loaded();
    let mut i = inputs(InstallStatus::NotInstalled);
    i.writable = false;
    assert!(matches!(primary_action(&s, &i), Primary::Unavailable(_)));
}

#[test]
fn tab_and_settings_changes_do_not_touch_progress() {
    // Tabs and the gear live in the app, not here; this pins that a
    // progress tick is kept until the install ends, whatever is shown.
    let mut s = loaded();
    s.click_install();
    s.apply(&Event::InstallStarted, URL);
    let tick = Progress::Extracting {
        label: "seed".into(),
        current: 3,
        total: 9,
        filename: "x".into(),
    };
    s.apply(&Event::Progress(tick.clone()), URL);
    assert_eq!(s.progress, Some(tick));
    assert_eq!(s.operation, Some(Operation::Installing));
}

// A manifest fetched for a URL no longer in use is not trusted for this
// one: its blob paths belong to the other host.
#[test]
fn a_manifest_for_a_stale_url_is_ignored() {
    let mut s = loaded();
    let other = "https://other.test/manifest.json";
    s.manifest.begin_fetch(other);
    assert!(
        s.manifest.manifest.is_none(),
        "changing URL drops the old one"
    );
    s.apply(
        &Event::ManifestFetched {
            url: URL.into(),
            manifest: manifest(&["late"]),
        },
        other,
    );
    assert!(s.manifest.manifest.is_none());
    assert!(s.manifest.fetching);
}

// A refresh of the same URL keeps the verified manifest on screen while
// it runs, and a failed refresh keeps it too (the error is shown beside).
#[test]
fn a_failed_refresh_keeps_the_verified_manifest() {
    let mut s = loaded();
    s.manifest.begin_fetch(URL);
    assert!(s.manifest.manifest.is_some());
    s.apply(
        &Event::ManifestError {
            url: URL.into(),
            message: "bad signature".into(),
        },
        URL,
    );
    assert!(s.manifest.manifest.is_some());
    assert_eq!(s.manifest.error.as_deref(), Some("bad signature"));
}

#[test]
fn install_status_compares_the_ledger_with_the_manifest() {
    let m = manifest(&["a", "b"]);
    let none = InstalledState::default();
    assert_eq!(
        install_status(false, false, Some(&m), &none),
        InstallStatus::NoFolder
    );
    assert_eq!(
        install_status(true, true, Some(&m), &none),
        InstallStatus::Adoptable
    );
    assert_eq!(
        install_status(true, false, Some(&m), &none),
        InstallStatus::NotInstalled
    );
    let partial = InstalledState {
        applied_patches: vec!["a".into()],
        seed_sha256: Some("seed-hash".into()),
        seed_adopted: false,
    };
    assert_eq!(
        install_status(true, false, Some(&m), &partial),
        InstallStatus::NeedsUpdate {
            seed: false,
            patches: 1
        }
    );
    assert_eq!(
        install_status(true, false, None, &partial),
        InstallStatus::InstalledUnchecked
    );
    let full = InstalledState {
        applied_patches: vec!["a".into(), "b".into()],
        ..partial
    };
    assert_eq!(
        install_status(true, false, Some(&m), &full),
        InstallStatus::UpToDate
    );
}

#[test]
fn up_to_date_without_sgw_exe_is_not_playable() {
    let s = loaded();
    let mut i = inputs(InstallStatus::UpToDate);
    i.sgw_present = false;
    assert!(matches!(primary_action(&s, &i), Primary::Unavailable(_)));
}

// Bug shape: editing the manifest URL left the old verified manifest in
// place, and an install paired it with the new URL's host.
#[test]
fn an_install_only_gets_the_manifest_from_the_url_in_use() {
    let s = loaded();
    assert!(s.manifest.for_url(URL).is_some());
    assert!(s
        .manifest
        .for_url("https://other.test/manifest.json")
        .is_none());
}
