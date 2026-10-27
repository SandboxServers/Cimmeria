use super::*;
use tempfile::TempDir;

fn artifact() -> Artifact {
    Artifact {
        path: PathBuf::from("/unused"),
        sha256: [0; 32],
    }
}

fn resources(proton: Option<ProtonSelection>) -> Resources {
    Resources {
        helper: artifact(),
        client_patches: None,
        graphics: None,
        client_telemetry: None,
        proton,
    }
}

fn selection(root: &Path, version: &str) -> ProtonSelection {
    ProtonSelection {
        id: "proton-test".into(),
        root: root.into(),
        version: version.into(),
    }
}

/// A Proton install root whose `version` file reads `contents`.
fn proton_root(contents: &str) -> TempDir {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("version"), contents).unwrap();
    root
}

#[test]
fn linux_native_without_a_proton_selection_is_refused() {
    assert!(!resources(None).proton_admitted());
}

#[test]
fn matching_proton_version_is_admitted() {
    let root = proton_root("GE-Proton11-11\n");
    let picked = resources(Some(selection(root.path(), "GE-Proton11-11")));
    assert!(picked.proton_admitted());
}

#[test]
fn changed_proton_version_is_refused() {
    let root = proton_root("GE-Proton10-1\n");
    let picked = resources(Some(selection(root.path(), "GE-Proton11-11")));
    assert!(!picked.proton_admitted());
}

#[test]
fn missing_proton_version_file_is_refused() {
    let root = tempfile::tempdir().unwrap();
    let picked = resources(Some(selection(root.path(), "GE-Proton11-11")));
    assert!(!picked.proton_admitted());
}

#[test]
fn a_graphics_pass_is_refused_on_linux() {
    let root = proton_root("GE-Proton11-11");
    let mut picked = resources(Some(selection(root.path(), "GE-Proton11-11")));
    picked.graphics = Some(Graphics {
        d3d9: artifact(),
        rosetta_x87: None,
    });
    assert!(!picked.proton_admitted());
}

#[test]
fn plan_without_a_proton_selection_serializes_as_before() {
    let json = serde_json::to_value(resources(None)).unwrap();
    assert!(json.get("proton").is_none());
    let decoded: Resources = serde_json::from_value(json).unwrap();
    assert_eq!(decoded, resources(None));
}

#[test]
fn proton_selection_round_trips_and_refuses_unknown_fields() {
    let root = PathBuf::from("/opt/proton");
    let picked = resources(Some(selection(&root, "GE-Proton11-11")));
    let json = serde_json::to_value(&picked).unwrap();
    assert_eq!(json["proton"]["version"], "GE-Proton11-11");
    let decoded: Resources = serde_json::from_value(json.clone()).unwrap();
    assert_eq!(decoded, picked);
    let mut extra = json;
    extra["proton"]["extra"] = "x".into();
    assert!(serde_json::from_value::<Resources>(extra).is_err());
}

#[test]
fn a_plan_without_a_proton_selection_hashes_without_the_field() {
    // `Plan::digest` hashes exactly these bytes, so an earlier plan keeps its digest.
    let plan = Plan {
        id: Uuid::new_v4(),
        installation: crate::storage::runtime_setup::tests::fixture_with_runtime([7; 32]).2,
        runtime: None,
        resources: resources(None),
    };
    let bytes = serde_json::to_vec(&plan).unwrap();
    assert!(!String::from_utf8(bytes).unwrap().contains("proton"));
}
