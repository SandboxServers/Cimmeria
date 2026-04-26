use thiserror::Error;

const ORIGINAL_HOST: &[u8] = b"www.stargateworlds.com";
const MAX_HOST_LEN: usize = 22;

#[derive(Debug, Error)]
pub enum PatchError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Server address too long: {len} bytes (max {MAX_HOST_LEN})")]
    AddressTooLong { len: usize },
    #[error("Original hostname not found in binary — may already be patched")]
    PatternNotFound,
}

fn find_pattern(data: &[u8], pattern: &[u8]) -> Option<usize> {
    data.windows(pattern.len())
        .position(|window| window == pattern)
}

pub fn patch_hostname(data: &mut [u8], new_host: &str) -> Result<(), PatchError> {
    let host_bytes = new_host.as_bytes();
    if host_bytes.len() > MAX_HOST_LEN {
        return Err(PatchError::AddressTooLong { len: host_bytes.len() });
    }

    let offset = find_pattern(data, ORIGINAL_HOST)
        .ok_or(PatchError::PatternNotFound)?;

    data[offset..offset + host_bytes.len()].copy_from_slice(host_bytes);
    for i in host_bytes.len()..MAX_HOST_LEN {
        data[offset + i] = 0;
    }

    Ok(())
}

pub fn needs_patching(data: &[u8]) -> bool {
    find_pattern(data, ORIGINAL_HOST).is_some()
}

pub fn patch_exe(exe_path: &std::path::Path, new_host: &str) -> Result<(), PatchError> {
    let mut data = std::fs::read(exe_path)?;
    patch_hostname(&mut data, new_host)?;

    let temp_path = exe_path.with_extension(format!("tmp.{}", std::process::id()));
    std::fs::write(&temp_path, &data)?;

    if let Err(e) = std::fs::rename(&temp_path, exe_path) {
        let _ = std::fs::remove_file(&temp_path);
        return Err(PatchError::Io(e));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_fake_exe(host: &[u8]) -> Vec<u8> {
        let mut data = vec![0u8; 100];
        data[40..40 + host.len()].copy_from_slice(host);
        data
    }

    #[test]
    fn test_patch_hostname_success() {
        let mut data = make_fake_exe(b"www.stargateworlds.com");
        patch_hostname(&mut data, "localhost").unwrap();
        assert_eq!(&data[40..49], b"localhost");
        assert!(data[49..62].iter().all(|&b| b == 0));
    }

    #[test]
    fn test_patch_hostname_max_length() {
        let mut data = make_fake_exe(b"www.stargateworlds.com");
        let host = "abcdefghijklmnopqrstuv";
        assert_eq!(host.len(), 22);
        patch_hostname(&mut data, host).unwrap();
        assert_eq!(&data[40..62], host.as_bytes());
    }

    #[test]
    fn test_patch_hostname_too_long() {
        let mut data = make_fake_exe(b"www.stargateworlds.com");
        let host = "this-hostname-is-way-too-long.example.com";
        let result = patch_hostname(&mut data, host);
        assert!(matches!(result, Err(PatchError::AddressTooLong { .. })));
    }

    #[test]
    fn test_patch_hostname_not_found() {
        let mut data = vec![0u8; 100];
        let result = patch_hostname(&mut data, "localhost");
        assert!(matches!(result, Err(PatchError::PatternNotFound)));
    }

    #[test]
    fn test_needs_patching() {
        let data = make_fake_exe(b"www.stargateworlds.com");
        assert!(needs_patching(&data));

        let patched = make_fake_exe(b"localhost\0\0\0\0\0\0\0\0\0\0\0\0\0");
        assert!(!needs_patching(&patched));
    }

    #[test]
    fn test_patch_idempotent_check() {
        let mut data = make_fake_exe(b"www.stargateworlds.com");
        patch_hostname(&mut data, "play.cimmeria.gg").unwrap();
        assert!(!needs_patching(&data));
        assert!(matches!(
            patch_hostname(&mut data, "other.host"),
            Err(PatchError::PatternNotFound)
        ));
    }

    #[test]
    fn test_patch_exe_on_disk() {
        let dir = tempfile::tempdir().unwrap();
        let exe_path = dir.path().join("SGW.exe");
        let data = make_fake_exe(b"www.stargateworlds.com");
        std::fs::write(&exe_path, &data).unwrap();

        patch_exe(&exe_path, "localhost").unwrap();

        let patched = std::fs::read(&exe_path).unwrap();
        assert_eq!(&patched[40..49], b"localhost");
    }

    #[test]
    fn test_patch_exe_no_temp_left() {
        let dir = tempfile::tempdir().unwrap();
        let exe_path = dir.path().join("SGW.exe");
        let data = make_fake_exe(b"www.stargateworlds.com");
        std::fs::write(&exe_path, &data).unwrap();

        patch_exe(&exe_path, "localhost").unwrap();

        // Verify no temp files left behind
        let entries: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.contains("tmp."))
                    .unwrap_or(false)
            })
            .collect();
        assert_eq!(entries.len(), 0, "No temp files should remain after successful patch");
    }

    #[test]
    fn test_patch_exe_unchanged_on_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let exe_path = dir.path().join("SGW.exe");
        let data = make_fake_exe(b"different.host");
        let original = data.clone();
        std::fs::write(&exe_path, &data).unwrap();

        let result = patch_exe(&exe_path, "localhost");
        assert!(result.is_err());

        let on_disk = std::fs::read(&exe_path).unwrap();
        assert_eq!(on_disk, original, "File should be unchanged on patch failure");
    }
}
