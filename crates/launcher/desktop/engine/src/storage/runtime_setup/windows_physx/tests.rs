use super::*;
use std::cell::Cell;

struct Fixed {
    core: Option<PathBuf>,
}
impl Probe for Fixed {
    fn core_path(&self) -> Option<PathBuf> {
        self.core.clone()
    }
}
/// Detection results for `outcome`, which takes them directly.
const PRESENT: Detection = Detection {
    core_registered: true,
    core_present: true,
};
const ABSENT: Detection = Detection {
    core_registered: false,
    core_present: false,
};

/// Records whether msiexec would have been started.
struct Launcher {
    calls: Cell<u32>,
    answer: Result<u32, ElevateError>,
}
impl Launcher {
    fn answering(answer: Result<u32, ElevateError>) -> Self {
        Self {
            calls: Cell::new(0),
            answer,
        }
    }
}
impl Elevate for Launcher {
    fn run_msi(&self, _msi: &Path) -> Result<u32, ElevateError> {
        self.calls.set(self.calls.get() + 1);
        self.answer
    }
}

#[test]
fn detection_needs_the_2_6_3_core_under_the_registered_path() {
    let root = tempfile::tempdir().unwrap();
    let registered = Fixed {
        core: Some(root.path().to_path_buf()),
    };

    // The value names a real folder, but a newer PhysX left no v2.6.3 core.
    std::fs::create_dir_all(root.path().join("v2.8.1")).unwrap();
    std::fs::write(root.path().join("v2.8.1").join("PhysXCore.dll"), b"core").unwrap();
    let other_core = detect(&registered);
    assert_eq!(
        other_core,
        Detection {
            core_registered: true,
            core_present: false
        }
    );
    assert!(
        !other_core.installed(),
        "a registered folder without v2.6.3\\PhysXCore.dll is not an install"
    );

    let core = root.path().join("v2.6.3").join("PhysXCore.dll");
    std::fs::create_dir_all(core.parent().unwrap()).unwrap();
    std::fs::write(&core, b"core").unwrap();
    assert_eq!(core_file(root.path()), core);
    assert_eq!(detect(&registered), PRESENT);
    assert!(detect(&registered).installed());

    // The core file alone, with no registry value pointing at it, is not found.
    assert_eq!(detect(&Fixed { core: None }), ABSENT);
    assert!(!ABSENT.installed());
}

/// Read the vendor installer's size check on its own: `physx_msi` also checks
/// the size, so going through `install` would not show this check works.
#[test]
fn read_package_refuses_a_wrong_size_file_itself() {
    let root = tempfile::tempdir().unwrap();
    let package = root.path().join("PhysX_7.11.13_SystemSoftware.exe");
    std::fs::write(&package, b"not the vendor installer").unwrap();
    assert_eq!(read_package(&package), Err(PhysxError::PackageMismatch));
    std::fs::write(&package, vec![0u8; PHYSX_EXE_BYTES]).unwrap();
    assert_eq!(read_package(&package).unwrap().len(), PHYSX_EXE_BYTES);
}

/// The returned guard denies writers while msiexec reads the MSI; once it is
/// dropped a writer can open the file again.
#[cfg(windows)]
#[test]
fn the_extracted_msi_is_write_locked_while_its_guard_is_held() {
    const ERROR_SHARING_VIOLATION: i32 = 32;
    let root = tempfile::tempdir().unwrap();
    let msi = root.path().join("physx.msi");
    let guard = write_msi(&msi, b"msi bytes").unwrap();
    let writer = std::fs::OpenOptions::new().write(true).open(&msi);
    assert_eq!(
        writer.unwrap_err().raw_os_error(),
        Some(ERROR_SHARING_VIOLATION),
        "a writer opened the MSI while the guard was held"
    );
    assert!(
        std::fs::remove_file(&msi).is_err(),
        "the MSI was deleted under its guard"
    );
    drop(guard);
    std::fs::OpenOptions::new().write(true).open(&msi).unwrap();
}

#[test]
fn a_package_that_is_not_the_pinned_installer_never_starts_msiexec() {
    let root = tempfile::tempdir().unwrap();
    let package = root.path().join("PhysX_7.11.13_SystemSoftware.exe");
    let source = InstallSource {
        package: package.clone(),
        scratch: root.path().join("scratch"),
    };
    let launcher = Launcher::answering(Ok(0));
    let probe = Fixed { core: None };

    // Right size, wrong bytes: only the SHA-256 tells it apart.
    std::fs::write(&package, vec![0u8; PHYSX_EXE_BYTES]).unwrap();
    assert_eq!(
        install(&source, &launcher, &probe),
        Err(PhysxError::PackageMismatch)
    );
    // Wrong size is refused too (by `read_package` and again by `physx_msi`;
    // `read_package_refuses_a_wrong_size_file_itself` guards the first check).
    std::fs::write(&package, b"not the vendor installer").unwrap();
    assert_eq!(
        install(&source, &launcher, &probe),
        Err(PhysxError::PackageMismatch)
    );
    std::fs::remove_file(&package).unwrap();
    assert_eq!(
        install(&source, &launcher, &probe),
        Err(PhysxError::InstallerUnavailable)
    );

    assert_eq!(launcher.calls.get(), 0, "no installer process was started");
    assert!(!source.scratch.exists(), "no MSI was extracted");
}

#[test]
fn a_declined_uac_prompt_is_a_cancel_not_a_failure() {
    assert_eq!(
        outcome(Err(ElevateError::Cancelled), ABSENT),
        InstallResult::CancelledAtUac {}
    );
    assert_eq!(
        outcome(Err(ElevateError::Win32(2)), PRESENT),
        InstallResult::StartFailed { win32_error: 2 }
    );
    assert_eq!(
        outcome(Ok(1602), PRESENT),
        InstallResult::CancelledInInstaller {}
    );
}

#[test]
fn installer_exit_codes_are_checked_against_detection_afterwards() {
    assert_eq!(
        outcome(Ok(0), PRESENT),
        InstallResult::Installed {
            restart_required: false
        }
    );
    assert_eq!(
        outcome(Ok(3010), PRESENT),
        InstallResult::Installed {
            restart_required: true
        }
    );
    // Success the system does not confirm is reported, not assumed.
    let registered_without_core = Detection {
        core_registered: true,
        core_present: false,
    };
    assert_eq!(
        outcome(Ok(0), registered_without_core),
        InstallResult::NotDetected { exit_code: 0 }
    );
    assert_eq!(
        outcome(Ok(1603), PRESENT),
        InstallResult::Failed { exit_code: 1603 }
    );
}

#[test]
fn another_installed_physx_version_is_judged_by_detection() {
    // 1638: Windows Installer refused because another PhysX version is there.
    assert_eq!(
        outcome(Ok(1638), PRESENT),
        InstallResult::Installed {
            restart_required: false
        },
        "the installed release already carries the 2.6.3 core"
    );
    assert_eq!(
        outcome(
            Ok(1638),
            Detection {
                core_registered: true,
                core_present: false
            }
        ),
        InstallResult::OtherVersionBlocks {}
    );
    assert_eq!(
        serde_json::to_value(InstallResult::OtherVersionBlocks {}).unwrap(),
        serde_json::json!({"state":"other_version_blocks"})
    );
}

#[test]
fn requests_and_results_keep_their_wire_names() {
    let request: PhysxCommand =
        serde_json::from_str(r#"{"command":"install","schema_version":1}"#).unwrap();
    assert_eq!(request, PhysxCommand::Install { schema_version: 1 });
    assert!(serde_json::from_str::<PhysxCommand>(
        r#"{"command":"install","schema_version":1,"msi":"C:\\evil.msi"}"#
    )
    .is_err());
    assert_eq!(
        serde_json::to_value(InstallResult::CancelledAtUac {}).unwrap(),
        serde_json::json!({"state":"cancelled_at_uac"})
    );
    assert_eq!(
        serde_json::to_value(PhysxError::PackageMismatch).unwrap(),
        serde_json::json!("package_mismatch")
    );
    assert_eq!(
        serde_json::to_value(PhysxError::Storage(StorageError::Busy)).unwrap(),
        serde_json::json!("busy")
    );
}
