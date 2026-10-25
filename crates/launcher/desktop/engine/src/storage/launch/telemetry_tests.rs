//! Game telemetry on a Proton Play (PP-11): the session's backend tag, and the
//! marker on a game whose binaries folder is lower case. Fixtures live in a
//! tempdir; nothing here reads the real `~/.steam`.
use super::*;
use crate::game_telemetry::Session;
use crate::storage::install_intent::ExtractionBackend;
use crate::storage::launch::tests::artifact;
use crate::storage::runtime_setup::tests::fixture_with_extraction;

/// A Proton Play's plan with a telemetry DLL. The game's binaries folder is
/// `Working/binaries`, as the cabinets store it, and `Working/Binaries` is absent.
fn proton_plan() -> (tempfile::TempDir, Plan) {
    let (root, _state, intent) =
        fixture_with_extraction(ExtractionBackend::Native, &"a".repeat(64), 1);
    let game = intent.content_root();
    std::fs::remove_dir_all(game.join("Working/Binaries")).unwrap();
    std::fs::create_dir_all(game.join("Working/binaries")).unwrap();
    let plan = Plan {
        id: Uuid::new_v4(),
        installation: intent,
        runtime: None,
        resources: Resources {
            proton: Some(ProtonSelection {
                id: "GE-Proton11-11".into(),
                root: root.path().join("proton"),
                version: "1792432585 GE-Proton11-11".into(),
            }),
            helper: artifact(root.path(), "helper.exe"),
            client_patches: None,
            client_telemetry: Some(artifact(root.path(), "telemetry.dll")),
            graphics: None,
        },
    };
    (root, plan)
}

#[test]
fn a_proton_play_tags_its_session_proton() {
    let (_root, plan) = proton_plan();
    assert_eq!(session_tags(&plan), ["desktop-launcher", "linux", "proton"]);
}

#[test]
fn the_marker_lands_in_the_lower_case_binaries_folder() {
    let (_root, plan) = proton_plan();
    let session = Session::fixture();
    assert!(injected(&plan, Some(&session)).is_some());
    let game = plan.installation.content_root();
    assert!(game
        .join("Working/binaries/sessions/current-session.json")
        .is_file());
    // A literal `Working/Binaries` join would have created this folder.
    assert!(!game.join("Working/Binaries").exists());
}
