//! Linux host wiring for Proton discovery (PP-09): status and Play find a Proton on
//! every call. Discovery reads `proton_fixture`, never the player's Steam (R3).
use super::*;
use crate::host::ProtonCommand;
use cimmeria_launcher_engine::launch::{Artifact, Resources};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// A fake Steam client under `home` with one Valve Proton in its library.
fn steam_with_proton(home: &Path) -> PathBuf {
    let proton = home.join(".local/share/Steam/steamapps/common/Proton 9.0");
    std::fs::create_dir_all(&proton).unwrap();
    for file in ["proton", "toolmanifest.vdf"] {
        std::fs::write(proton.join(file), b"").unwrap();
    }
    std::fs::write(proton.join("version"), "1700000000 proton-9.0\n").unwrap();
    proton
}

fn helper(root: &Path) -> Artifact {
    let path = root.join("helper.exe");
    std::fs::write(&path, b"fixture").unwrap();
    let digest: String = Sha256::digest(b"fixture")
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    Artifact::open(path, &digest).unwrap()
}

/// A host whose Proton roots are empty tempdirs under `root`; returns (host, home).
fn host(root: &Path, with_resources: bool) -> (NativeHost, PathBuf) {
    let (home, system) = (root.join("home"), root.join("system"));
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&system).unwrap();
    let mut host = NativeHost::new(root.join("state"));
    host.proton_fixture = Some((home.clone(), system));
    host.prerequisites_fixture = Some(Vec::new());
    if with_resources {
        host.launch_resources = Some(Resources {
            proton: None,
            helper: helper(root),
            client_patches: None,
            client_telemetry: None,
            graphics: None,
        });
    }
    (host, home)
}

fn inspect(host: &NativeHost) -> LaunchStatus {
    host.launch_command(LaunchCommand::Inspect { schema_version: 1 })
        .unwrap()
}

fn play(host: &NativeHost) -> Result<LaunchStatus, JobError> {
    host.launch_command(LaunchCommand::Play {
        schema_version: 1,
        operation_id: Uuid::new_v4(),
        operation_revision: 0,
        installation_id: Uuid::new_v4(),
    })
}

#[test]
fn proton_found_under_home_is_chosen_and_clears_the_missing_prerequisite() {
    let home = tempfile::tempdir().unwrap();
    let system = tempfile::tempdir().unwrap();
    let (home, system) = (home.path(), system.path());
    assert_eq!(
        resources::missing_proton(resources::proton_under(home, system).as_ref()),
        vec!["proton"],
        "no Steam tree under HOME"
    );
    assert!(resources::proton_under(home, system).is_none());
    let proton = steam_with_proton(home);
    let found = resources::proton_under(home, system).expect("Proton under HOME");
    assert_eq!(found.version, "1700000000 proton-9.0");
    assert_eq!(found.root, proton.canonicalize().unwrap());
    assert!(resources::missing_proton(Some(&found)).is_empty());
}

#[test]
fn status_reports_proton_missing_until_one_is_installed_without_a_restart() {
    let root = tempfile::tempdir().unwrap();
    let (host, home) = host(root.path(), false);
    assert_eq!(inspect(&host).prerequisites_missing, vec!["proton"]);
    steam_with_proton(&home);
    // Same host: discovery runs per call, so no restart is needed.
    assert!(inspect(&host).prerequisites_missing.is_empty());
}

#[test]
fn play_without_proton_is_refused_as_a_missing_prerequisite_and_logged() {
    let root = tempfile::tempdir().unwrap();
    let (host, home) = host(root.path(), true);
    let (result, events) = crate::test_capture::capture(|| play(&host));
    assert_eq!(result.unwrap_err(), JobError::PrerequisitesMissing);
    let row = events
        .iter()
        .find(|event| event.field("event") == Some("prerequisites_missing"))
        .expect("prerequisites_missing row");
    assert_eq!(row.field("proton"), Some("true"));
    steam_with_proton(&home);
    // No content is installed, so another refusal follows; it is not this one.
    assert_ne!(play(&host).err(), Some(JobError::PrerequisitesMissing));
}

/// A fake Steam client under `home` with the Valve Proton 9.0 and a GE-Proton.
fn steam_with_two_protons(home: &Path) {
    steam_with_proton(home);
    let ge = home.join(".local/share/Steam/compatibilitytools.d/GE-Proton11-11-x86_64");
    std::fs::create_dir_all(&ge).unwrap();
    for file in ["proton", "toolmanifest.vdf"] {
        std::fs::write(ge.join(file), b"").unwrap();
    }
    std::fs::write(ge.join("version"), "1792432585 GE-Proton11-11\n").unwrap();
}

#[test]
fn a_saved_proton_reaches_play_and_inspect_and_a_missing_one_falls_back() {
    const GE: &str = "ge:GE-Proton11-11-x86_64";
    const VALVE: &str = "valve:Proton 9.0";
    let root = tempfile::tempdir().unwrap();
    let (host, home) = host(root.path(), true);
    steam_with_two_protons(&home);
    let system = root.path().join("system");
    let id_for = |preferred: Option<&str>| {
        resources::proton(Some((home.as_path(), system.as_path())), preferred).map(|p| p.id)
    };
    assert_eq!(id_for(Some(VALVE)).as_deref(), Some(VALVE));
    assert_eq!(id_for(Some("valve:Proton 8.0")).as_deref(), Some(GE));
    assert_eq!(
        host.proton(Some(VALVE)).map(|p| p.id).as_deref(),
        Some(VALVE)
    );
    host.proton_command(ProtonCommand::Select {
        id: Some(VALVE.to_owned()),
    })
    .unwrap();
    let saved = host.saved_proton();
    assert_eq!(
        host.proton(saved.as_deref()).map(|p| p.id).as_deref(),
        Some(VALVE)
    );
    assert!(inspect(&host).prerequisites_missing.is_empty());
}
