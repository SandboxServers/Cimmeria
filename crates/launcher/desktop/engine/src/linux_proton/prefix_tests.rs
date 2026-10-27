use super::super::discovery::{ProtonInstall, ProtonKind};
use super::*;

fn install(version: &str) -> ProtonInstall {
    ProtonInstall {
        id: "ge:GE-Proton11-11-x86_64".into(),
        name: "GE-Proton11-11".into(),
        kind: ProtonKind::Ge,
        root: PathBuf::from("/nonexistent/proton"),
        version: version.into(),
        runtime: None,
    }
}

fn stamp(version: &str) -> Stamp {
    Stamp {
        schema_version: 1,
        proton_version: version.into(),
        physx: PhysxState::Installed,
    }
}

#[test]
fn second_claim_is_busy_while_the_first_is_held() {
    let root = tempfile::tempdir().unwrap();
    let first = claim(root.path()).unwrap();
    assert!(matches!(claim(root.path()), Err(ProtonError::Busy)));
    drop(first);
    assert!(claim(root.path()).is_ok());
}

#[cfg(unix)]
#[test]
fn claim_creates_a_private_compatdata_folder() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    let prefix = claim(root.path()).unwrap();
    assert_eq!(prefix.compat, root.path().join("proton").join("compatdata"));
    assert_eq!(prefix.pfx, prefix.compat.join("pfx"));
    let mode = std::fs::metadata(&prefix.compat)
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o700);
}

#[test]
fn stamp_round_trips_through_the_stamp_file() {
    let root = tempfile::tempdir().unwrap();
    let prefix = claim(root.path()).unwrap();
    assert_eq!(prefix.stamp(), None);
    prefix
        .write_stamp(&stamp("1792432585 GE-Proton11-11"))
        .unwrap();
    assert!(prefix.compat.join(STAMP_FILE).is_file());
    assert_eq!(prefix.stamp(), Some(stamp("1792432585 GE-Proton11-11")));
}

#[test]
fn an_unreadable_stamp_counts_as_missing() {
    let root = tempfile::tempdir().unwrap();
    let prefix = claim(root.path()).unwrap();
    std::fs::write(
        prefix.compat.join(STAMP_FILE),
        r#"{"schema_version":1,"proton_version":"v","extra":true}"#,
    )
    .unwrap();
    assert_eq!(prefix.stamp(), None);
}

#[test]
fn needs_setup_without_a_stamp_or_with_another_version() {
    let root = tempfile::tempdir().unwrap();
    let prefix = claim(root.path()).unwrap();
    let current = install("1792432585 GE-Proton11-11");
    assert!(prefix.needs_setup(&current));
    prefix
        .write_stamp(&stamp("1700000000 GE-Proton10-1"))
        .unwrap();
    assert!(prefix.needs_setup(&current));
    prefix.write_stamp(&stamp(&current.version)).unwrap();
    assert!(!prefix.needs_setup(&current));
}

#[test]
fn profile_root_is_the_steamuser_profile_in_the_prefix() {
    let root = tempfile::tempdir().unwrap();
    let prefix = claim(root.path()).unwrap();
    assert_eq!(
        prefix.profile_root(),
        prefix.pfx.join("drive_c/users/steamuser")
    );
}
