//! Unix regression guards: parent retention must survive rename plus symlink substitution.
use super::*;
use std::os::unix::fs::{symlink, MetadataExt};

fn exercise(mutate: impl FnOnce(&Parents, &Path) -> Result<(), Error>, verify: impl FnOnce(&Path)) {
    let temporary = tempfile::tempdir().unwrap();
    // macOS temp paths may use /var while reviewed plans pin /private/var.
    let temporary_root = temporary.path().canonicalize().unwrap();
    let root = temporary_root.join("selected");
    let displaced = temporary_root.join("retained");
    let outside = temporary_root.join("outside");
    for directory in [&root, &outside] {
        std::fs::create_dir_all(directory.join("nested/tree")).unwrap();
        for (name, bytes) in [
            ("from", b"original".as_slice()),
            ("to", b"existing"),
            ("tree/child", b"child"),
        ] {
            std::fs::write(directory.join("nested").join(name), bytes).unwrap();
        }
        let mut exe = vec![0; 0x200];
        exe[..2].copy_from_slice(b"MZ");
        exe[60..64].copy_from_slice(&0x128u32.to_le_bytes());
        exe[0x128..0x12c].copy_from_slice(b"PE\0\0");
        exe[0x13c..0x13e].copy_from_slice(&0xe0u16.to_le_bytes());
        exe[0x140..0x142].copy_from_slice(&0x10bu16.to_le_bytes());
        exe[0x186] = 0x40;
        std::fs::write(directory.join("nested/SGW.exe"), exe).unwrap();
    }
    let outside_before = inventory::scan(&outside, &CancellationToken::new()).unwrap();
    let outside_id = std::fs::metadata(&outside).unwrap().ino();
    let plan = super::tests::plan(&root);
    let paths: Vec<PathBuf> = ["from", "to", "new", "SGW.exe", "tree", "child/new"]
        .into_iter()
        .map(|name| root.join("nested").join(name))
        .collect();
    let refs: Vec<&Path> = paths.iter().map(PathBuf::as_path).collect();
    let guard = Parents::new(&plan, &refs, false).unwrap();
    // Retain every parent first, then replace an ancestor immediately before
    // the operation. The guard must not resolve that old name again.
    std::fs::rename(&root, &displaced).unwrap();
    symlink(&outside, &root).unwrap();
    mutate(&guard, &root.join("nested")).unwrap();
    assert_eq!(
        inventory::scan(&outside, &CancellationToken::new()).unwrap(),
        outside_before
    );
    assert_eq!(std::fs::metadata(&outside).unwrap().ino(), outside_id);
    verify(&displaced.join("nested"));
}
#[test]
fn retained_fds_confine_creation_and_atomic_setup_write_after_ancestor_substitution() {
    exercise(
        |parents, root| {
            parents.write(&root.join("new"), b"new")?;
            parents.write(&root.join("to"), b"setup")
        },
        |root| {
            assert_eq!(std::fs::read(root.join("new")).unwrap(), b"new");
            assert_eq!(std::fs::read(root.join("to")).unwrap(), b"setup");
        },
    );
}
#[test]
fn retained_fds_confine_mkdir_after_ancestor_substitution() {
    exercise(
        |parents, _| {
            parents.open_child_directory(
                Path::new("nested"),
                std::ffi::OsStr::new("made"),
                true,
            )?;
            Ok(())
        },
        |root| assert!(root.join("made").is_dir()),
    );
}
#[test]
fn retained_fds_confine_promotion_and_restore_after_ancestor_substitution() {
    exercise(
        |parents, root| {
            parents.rename(&root.join("from"), &root.join("new"), true)?;
            parents.copy_atomic(&root.join("new"), &root.join("to"))
        },
        |root| {
            assert!(!root.join("from").exists());
            assert_eq!(std::fs::read(root.join("new")).unwrap(), b"original");
            assert_eq!(std::fs::read(root.join("to")).unwrap(), b"original");
        },
    );
}
#[test]
fn retained_fds_confine_unlink_and_recursive_cleanup_after_ancestor_substitution() {
    exercise(
        |parents, root| {
            parents.remove(&root.join("from"), false)?;
            parents.remove_tree(&root.join("tree"))
        },
        |root| {
            assert!(!root.join("from").exists());
            assert!(!root.join("tree").exists());
        },
    );
}
#[test]
fn retained_file_handle_confines_aslr_edit_after_ancestor_substitution() {
    exercise(
        |parents, root| {
            crate::client_setup::aslr::disable_open(&mut parents.open(
                &root.join("SGW.exe"),
                true,
                false,
            )?)?;
            Ok(())
        },
        |root| assert_eq!(std::fs::read(root.join("SGW.exe")).unwrap()[0x186], 0),
    );
}
#[test]
fn exclusive_fd_promotion_preserves_a_concurrently_created_target() {
    exercise(
        |parents, root| {
            assert!(parents
                .rename(&root.join("from"), &root.join("to"), true)
                .is_err());
            Ok(())
        },
        |root| {
            assert_eq!(std::fs::read(root.join("to")).unwrap(), b"existing");
            assert_eq!(std::fs::read(root.join("from")).unwrap(), b"original");
        },
    );
}
