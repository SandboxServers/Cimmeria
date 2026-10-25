//! The Proton picker against a fake Steam tree (R3): discovery reads the
//! fixture roots, never the player's Steam.
use super::*;
use std::path::Path;

/// A Proton directory that discovery accepts: `proton`, `toolmanifest.vdf`, `version`.
fn proton_dir(parent: &Path, name: &str, version: &str) {
    let dir = parent.join(name);
    std::fs::create_dir_all(&dir).unwrap();
    for file in ["proton", "toolmanifest.vdf"] {
        std::fs::write(dir.join(file), b"").unwrap();
    }
    std::fs::write(dir.join("version"), version).unwrap();
}

/// A host whose Steam client holds a GE-Proton and a Valve Proton 9.0.
fn host_with_two_protons(root: &Path) -> NativeHost {
    let (home, system) = (root.join("home"), root.join("system"));
    let common = home.join(".local/share/Steam/steamapps/common");
    std::fs::create_dir_all(&common).unwrap();
    std::fs::create_dir_all(&system).unwrap();
    proton_dir(&common, "Proton 9.0", "1700000000 proton-9.0\n");
    proton_dir(
        &home.join(".local/share/Steam/compatibilitytools.d"),
        "GE-Proton11-11-x86_64",
        "1792432585 GE-Proton11-11\n",
    );
    let mut host = NativeHost::new(root.join("state"));
    host.proton_fixture = Some((home, system));
    host
}

const GE: &str = "ge:GE-Proton11-11-x86_64";
const VALVE: &str = "valve:Proton 9.0";

#[test]
fn inspect_lists_ids_in_rank_order_with_no_path() {
    let root = tempfile::tempdir().unwrap();
    let host = host_with_two_protons(root.path());
    let view = host.proton_command(ProtonCommand::Inspect {}).unwrap();
    let ids: Vec<&str> = view.available.iter().map(|c| c.id.as_str()).collect();
    assert_eq!(ids, [GE, VALVE]);
    assert_eq!(view.selected.as_deref(), Some(GE));
    assert_eq!(view.preferred, None);
    let json = serde_json::to_string(&view).unwrap();
    assert!(!json.contains('/'), "a path crossed IPC: {json}");
}

#[test]
fn select_of_an_unknown_id_is_refused_and_keeps_the_preference() {
    let root = tempfile::tempdir().unwrap();
    let host = host_with_two_protons(root.path());
    let select = |id: Option<&str>| ProtonCommand::Select {
        id: id.map(str::to_owned),
    };
    host.proton_command(select(Some(VALVE))).unwrap();
    assert_eq!(host.saved_proton().as_deref(), Some(VALVE));
    let refused = host.proton_command(select(Some("valve:Proton 8.0")));
    assert_eq!(refused.err().as_deref(), Some("unknown_proton"));
    assert_eq!(host.saved_proton().as_deref(), Some(VALVE));
}

#[test]
fn select_none_clears_the_preference_and_the_next_inspect_sees_it() {
    let root = tempfile::tempdir().unwrap();
    let host = host_with_two_protons(root.path());
    let view = host
        .proton_command(ProtonCommand::Select {
            id: Some(VALVE.to_owned()),
        })
        .unwrap();
    assert_eq!(view.selected.as_deref(), Some(VALVE));
    let inspected = host.proton_command(ProtonCommand::Inspect {}).unwrap();
    assert_eq!(inspected.selected.as_deref(), Some(VALVE));
    let cleared = host
        .proton_command(ProtonCommand::Select { id: None })
        .unwrap();
    assert_eq!(cleared.preferred, None);
    assert_eq!(cleared.selected.as_deref(), Some(GE));
}

#[test]
fn a_preferences_file_without_the_proton_field_loads_and_stays_unset() {
    // A document as the build before PP-10 wrote it: no `proton_preference` key.
    let old = r#"{"schema_version":2,"revision":4,"install_directory":null,"launcher_summary_consent":false,"login_servers":[],"client_patches_enabled":true}"#;
    let loaded: cimmeria_launcher_engine::Preferences = serde_json::from_str(old).unwrap();
    assert_eq!(loaded.proton_preference, None);
    assert_eq!(loaded.revision, 4);
    let written = serde_json::to_string(&loaded).unwrap();
    assert!(!written.contains("proton_preference"), "{written}");
}

#[test]
fn request_refuses_unknown_fields() {
    let parsed = serde_json::from_str::<ProtonCommand>(r#"{"op":"inspect","path":"/x"}"#);
    assert!(parsed.is_err());
}
