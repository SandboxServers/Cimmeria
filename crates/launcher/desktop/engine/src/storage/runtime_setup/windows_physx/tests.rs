use super::*;
use std::cell::Cell;

struct Fixed {
    loader: bool,
    core: Option<PathBuf>,
}
impl Probe for Fixed {
    fn system_loader(&self) -> bool {
        self.loader
    }
    fn core_path(&self) -> Option<PathBuf> {
        self.core.clone()
    }
}
fn present() -> Fixed {
    Fixed {
        loader: true,
        core: Some(PathBuf::from(r"C:\Program Files (x86)\AGEIA Technologies\")),
    }
}

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
fn detection_needs_both_the_system_loader_and_the_registered_core() {
    let found = detect(&present());
    assert!(found.installed());
    assert_eq!(
        found,
        Detection {
            loader_present: true,
            core_registered: true
        }
    );

    let missing_key = detect(&Fixed {
        loader: true,
        core: None,
    });
    assert!(
        !missing_key.installed(),
        "a loader DLL alone is not an install"
    );
    assert!(missing_key.loader_present && !missing_key.core_registered);

    let missing_dll = detect(&Fixed {
        loader: false,
        ..present()
    });
    assert!(
        !missing_dll.installed(),
        "a stale AGEIA registry value alone is not an install"
    );
    assert!(!missing_dll.loader_present && missing_dll.core_registered);
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

    // Right size, wrong bytes: only the SHA-256 tells it apart.
    std::fs::write(&package, vec![0u8; PHYSX_EXE_BYTES]).unwrap();
    assert_eq!(
        install(&source, &launcher, &present()),
        Err(PhysxError::PackageMismatch)
    );
    // Wrong size is refused before it is read.
    std::fs::write(&package, b"not the vendor installer").unwrap();
    assert_eq!(
        install(&source, &launcher, &present()),
        Err(PhysxError::PackageMismatch)
    );
    std::fs::remove_file(&package).unwrap();
    assert_eq!(
        install(&source, &launcher, &present()),
        Err(PhysxError::InstallerUnavailable)
    );

    assert_eq!(launcher.calls.get(), 0, "no installer process was started");
    assert!(!source.scratch.exists(), "no MSI was extracted");
}

#[test]
fn a_declined_uac_prompt_is_a_cancel_not_a_failure() {
    assert_eq!(
        outcome(
            Err(ElevateError::Cancelled),
            detect(&Fixed {
                loader: false,
                core: None
            })
        ),
        InstallResult::CancelledAtUac {}
    );
    assert_eq!(
        outcome(Err(ElevateError::Win32(2)), detect(&present())),
        InstallResult::StartFailed { win32_error: 2 }
    );
    assert_eq!(
        outcome(Ok(1602), detect(&present())),
        InstallResult::CancelledInInstaller {}
    );
}

#[test]
fn installer_exit_codes_are_checked_against_detection_afterwards() {
    assert_eq!(
        outcome(Ok(0), detect(&present())),
        InstallResult::Installed {
            restart_required: false
        }
    );
    assert_eq!(
        outcome(Ok(3010), detect(&present())),
        InstallResult::Installed {
            restart_required: true
        }
    );
    // Success the system does not confirm is reported, not assumed.
    assert_eq!(
        outcome(
            Ok(0),
            detect(&Fixed {
                loader: true,
                core: None
            })
        ),
        InstallResult::NotDetected { exit_code: 0 }
    );
    assert_eq!(
        outcome(Ok(1603), detect(&present())),
        InstallResult::Failed { exit_code: 1603 }
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
