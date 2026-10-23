//! Selection-to-unlink race guards for adopted roots.
use super::*;

#[test]
fn direct_uninstall_retains_validated_parents_until_unlink() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let nested = root.path().join("Working/Binaries");
    let displaced = root.path().join("retained-binaries");
    std::fs::create_dir_all(&nested).unwrap();
    std::fs::write(nested.join("SGW.exe"), b"managed executable").unwrap();
    std::fs::write(nested.join("notes.txt"), b"player notes").unwrap();
    let sentinel = outside.path().join("SGW.exe");
    std::fs::write(&sentinel, b"outside executable").unwrap();
    let before = file_id(&sentinel);
    std::fs::write(root.path().join(".cimmeria-install.json"), b"owner").unwrap();
    let owner = File::open(root.path().join(".cimmeria-install.json")).unwrap();
    let paths = [PathBuf::from("Working/Binaries/SGW.exe")].into();
    let retained = remove_selected(root.path(), &paths, &owner, || {
        #[cfg(windows)]
        assert!(std::fs::rename(&nested, &displaced).is_err());
        #[cfg(unix)]
        {
            std::fs::rename(&nested, &displaced).unwrap();
            std::os::unix::fs::symlink(outside.path(), &nested).unwrap();
        }
        Ok(())
    })
    .unwrap();
    assert_eq!(std::fs::read(&sentinel).unwrap(), b"outside executable");
    assert_eq!(file_id(&sentinel), before);
    #[cfg(windows)]
    let original = nested;
    #[cfg(unix)]
    let original = displaced;
    assert!(!original.join("SGW.exe").exists());
    assert_eq!(
        std::fs::read(original.join("notes.txt")).unwrap(),
        b"player notes"
    );
    drop(retained);
}

fn file_id(path: &Path) -> (u64, u64) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let metadata = std::fs::metadata(path).unwrap();
        (metadata.dev(), metadata.ino())
    }
    #[cfg(windows)]
    {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::Storage::FileSystem::{
            GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION,
        };
        let file = File::open(path).unwrap();
        // SAFETY: initialized out parameter and an owned live file handle.
        let mut info: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
        assert_ne!(
            unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut info) },
            0
        );
        (
            u64::from(info.dwVolumeSerialNumber),
            (u64::from(info.nFileIndexHigh) << 32) | u64::from(info.nFileIndexLow),
        )
    }
}

#[test]
fn direct_uninstall_refuses_a_substituted_root_with_copied_owner_bytes() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("selected");
    let displaced = temporary.path().join("original-root");
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join(".cimmeria-install.json"), b"owner").unwrap();
    let owner = File::open(root.join(".cimmeria-install.json")).unwrap();
    std::fs::write(root.join("SGW.exe"), b"original executable").unwrap();
    #[cfg(windows)]
    {
        let _retained_owner = &owner;
        assert!(std::fs::rename(&root, &displaced).is_err());
    }
    #[cfg(unix)]
    {
        std::fs::rename(&root, &displaced).unwrap();
        std::fs::create_dir(&root).unwrap();
        std::fs::write(root.join(".cimmeria-install.json"), b"owner").unwrap();
        std::fs::write(root.join("SGW.exe"), b"outside executable").unwrap();
        let before = file_id(&root.join("SGW.exe"));
        let paths = [PathBuf::from("SGW.exe")].into();
        assert!(matches!(
            remove_selected(&root, &paths, &owner, || Ok(())),
            Err(StorageError::UnsafeFile)
        ));
        assert_eq!(
            std::fs::read(root.join("SGW.exe")).unwrap(),
            b"outside executable"
        );
        assert_eq!(file_id(&root.join("SGW.exe")), before);
        assert_eq!(
            std::fs::read(displaced.join("SGW.exe")).unwrap(),
            b"original executable"
        );
    }
}
