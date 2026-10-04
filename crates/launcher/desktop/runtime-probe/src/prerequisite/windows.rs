//! One-shot x86 installer/probe sequence. The host owns the entire Wine prefix:
//! this API cannot prove MSI service descendants exited after an interrupted call.
use super::{after_install, package, Failure, PrepareRequest, PrepareResult, ResultKind};
use std::{
    fs::OpenOptions,
    io::{Read, Write},
    os::windows::fs::{MetadataExt, OpenOptionsExt},
    path::Path,
};
use windows_sys::Win32::{
    Storage::FileSystem::{FILE_ATTRIBUTE_REPARSE_POINT, FILE_SHARE_READ},
    System::{ApplicationInstallationAndServicing::*, Diagnostics::Debug::*},
};

pub fn prepare(request: PrepareRequest) -> PrepareResult {
    let result = execute(&request).unwrap_or_else(|reason| ResultKind::Failed { reason });
    PrepareResult {
        schema_version: 1,
        operation_id: request.operation_id,
        prefix_generation: request.prefix_generation,
        result,
    }
}
fn plain(path: &Path, directory: bool) -> Result<(), Failure> {
    let meta = std::fs::symlink_metadata(path).map_err(|_| Failure::InvalidInput)?;
    if meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
        || if directory {
            !meta.is_dir()
        } else {
            !meta.is_file()
        }
    {
        return Err(Failure::InvalidInput);
    }
    Ok(())
}
fn execute(request: &PrepareRequest) -> Result<ResultKind, Failure> {
    // Public native entry points revalidate the wire contract too.
    let encoded = serde_json::to_vec(request).map_err(|_| Failure::InvalidInput)?;
    super::decode_request(&encoded).map_err(|_| Failure::InvalidInput)?;
    plain(&request.game_binaries, true)?;
    plain(&request.game_binaries.join("SGW.exe"), false)?;
    plain(&request.package, false)?;
    let mut source = OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ)
        .open(&request.package)
        .map_err(|_| Failure::Io)?;
    if source.metadata().map_err(|_| Failure::Io)?.len() != package::PHYSX_EXE_BYTES as u64 {
        return Err(Failure::PackageIdentity);
    }
    let mut bytes = Vec::new();
    (&mut source)
        .take(package::PHYSX_EXE_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| Failure::Io)?;
    let payload = package::physx_msi(&bytes).map_err(|_| Failure::PackageIdentity)?;
    // A private fresh directory preserves partial evidence and refuses replay.
    let parent = request.scratch.parent().ok_or(Failure::InvalidInput)?;
    plain(parent, true)?;
    std::fs::create_dir(&request.scratch).map_err(|_| Failure::ScratchUnavailable)?;
    let msi = request.scratch.join("physx-7.11.13.msi");
    let mut output = OpenOptions::new()
        .create_new(true)
        .read(true)
        .write(true)
        .share_mode(FILE_SHARE_READ)
        .open(&msi)
        .map_err(|_| Failure::Io)?;
    output.write_all(payload).map_err(|_| Failure::Io)?;
    output.sync_all().map_err(|_| Failure::Io)?;
    // Installer database readers may deny sharing with a writable handle. Reopen
    // read-only, then recheck the exact bytes under deny-write/delete sharing.
    drop(output);
    let mut msi_guard = OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ)
        .open(&msi)
        .map_err(|_| Failure::Io)?;
    if msi_guard.metadata().map_err(|_| Failure::Io)?.len() != payload.len() as u64 {
        return Err(Failure::PackageIdentity);
    }
    let mut buffer = [0u8; 65536];
    for expected in payload.chunks(buffer.len()) {
        let actual = &mut buffer[..expected.len()];
        msi_guard.read_exact(actual).map_err(|_| Failure::Io)?;
        if actual != expected {
            return Err(Failure::PackageIdentity);
        }
    }
    // Retain msi_guard until installation and the probe have both returned.
    let msi = msi.canonicalize().map_err(|_| Failure::Io)?;
    let name = crate::windows::wide(msi.as_os_str()).map_err(|_| Failure::InvalidInput)?;
    let properties = crate::windows::wide(std::ffi::OsStr::new("REBOOT=ReallySuppress"))
        .map_err(|_| Failure::InvalidInput)?;
    unsafe { SetErrorMode(SEM_FAILCRITICALERRORS | SEM_NOGPFAULTERRORBOX) };
    let previous = unsafe { MsiSetInternalUI(INSTALLUILEVEL_NONE, std::ptr::null_mut()) };
    if previous == INSTALLUILEVEL_NOCHANGE {
        return Err(Failure::Io);
    }
    struct Ui(INSTALLUILEVEL);
    impl Drop for Ui {
        fn drop(&mut self) {
            unsafe {
                MsiSetInternalUI(self.0, std::ptr::null_mut());
            }
        }
    }
    let _ui = Ui(previous);
    let code = unsafe { MsiInstallProductW(name.as_ptr(), properties.as_ptr()) };
    Ok(after_install(code, || {
        crate::windows::probe(crate::Request {
            schema_version: 1,
            game_binaries: request.game_binaries.clone(),
        })
        .map_err(|_| Failure::Probe)
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invalid_input_never_creates_scratch_or_invokes_msi() {
        let root = std::env::temp_dir().join(format!(
            "cimmeria-prereq-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&root).unwrap();
        std::fs::write(root.join("SGW.exe"), b"inert fixture").unwrap();
        std::fs::write(root.join("package.exe"), b"inert fixture").unwrap();
        let scratch = root.join("scratch");
        let result = prepare(PrepareRequest {
            schema_version: 1,
            operation_id: uuid::Uuid::from_u128(1),
            prefix_generation: uuid::Uuid::from_u128(2),
            game_binaries: root.clone(),
            package: root.join("package.exe"),
            scratch: scratch.clone(),
        });
        assert!(matches!(
            result.result,
            ResultKind::Failed {
                reason: Failure::PackageIdentity
            }
        ));
        assert!(!scratch.exists());
        std::fs::remove_dir_all(root).unwrap();
    }
}
