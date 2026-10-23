use super::*;

fn fixture(root: &Path) -> (InstallIntent, File) {
    let value: serde_json::Value = serde_json::from_str(include_str!(
        "../../adoption/fixtures/plan-v1-in-place.json"
    ))
    .unwrap();
    let mut owner: InstallIntent = serde_json::from_value(value["installation"].clone()).unwrap();
    owner.schema_version = 2;
    owner.destination = root.to_owned();
    atomic::write(root, ".cimmeria-install.json", &owner).unwrap();
    let file = File::open(root.join(".cimmeria-install.json")).unwrap();
    (owner, file)
}

#[test]
fn receipt_bytes_match_existing_formats_and_reject_another_owner() {
    let temp = tempfile::tempdir().unwrap();
    let other = tempfile::tempdir().unwrap();
    let (owner, file) = fixture(temp.path());
    let (_, wrong) = fixture(other.path());
    assert!(write_ready_owned(temp.path(), &owner, owner.release_identity(), &wrong).is_err());
    assert!(!temp.path().join("content-ready.json").exists());
    write_ready_owned(temp.path(), &owner, owner.release_identity(), &file).unwrap();
    assert_eq!(
        std::fs::read(temp.path().join("content-ready.json")).unwrap(),
        serde_json::to_vec(&owner).unwrap()
    );
    let mut updated = owner.release_identity();
    updated.evidence_id = uuid::Uuid::new_v4();
    write_ready_owned(temp.path(), &owner, updated, &file).unwrap();
    assert_eq!(read_ready(temp.path(), &owner).unwrap(), Some(updated));
}

#[test]
fn receipt_retained_root_and_ancestor_substitution_preserve_outside_bytes() {
    for ancestor in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        let container = temp.path().join("container");
        let root = container.join("selected");
        std::fs::create_dir_all(&root).unwrap();
        let (owner, file) = fixture(&root);
        let parents = Parents::for_owner(&root, &[], false, &file).unwrap();
        let original = if ancestor { &container } else { &root };
        let displaced = temp.path().join("retained");
        #[cfg(windows)]
        {
            assert!(std::fs::rename(original, &displaced).is_err());
            publish(&parents, &root, &owner, owner.release_identity()).unwrap();
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            std::fs::rename(original, &displaced).unwrap();
            std::fs::create_dir_all(&root).unwrap();
            std::fs::write(
                root.join(".cimmeria-install.json"),
                serde_json::to_vec(&owner).unwrap(),
            )
            .unwrap();
            let outside = root.join("content-ready.json");
            std::fs::write(&outside, b"outside receipt").unwrap();
            let before = std::fs::metadata(&outside).unwrap();
            assert!(write_ready_owned(&root, &owner, owner.release_identity(), &file).is_err());
            publish(&parents, &root, &owner, owner.release_identity()).unwrap();
            assert_eq!(std::fs::read(&outside).unwrap(), b"outside receipt");
            assert_eq!(std::fs::metadata(&outside).unwrap().ino(), before.ino());
            let retained = if ancestor {
                displaced.join("selected")
            } else {
                displaced
            };
            assert_eq!(
                read_ready(&retained, &owner).unwrap(),
                Some(owner.release_identity())
            );
        }
    }
}
