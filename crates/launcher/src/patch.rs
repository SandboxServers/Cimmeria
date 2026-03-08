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

pub fn patch_hostname(data: &mut [u8], new_host: &str) -> Result<usize, PatchError> {
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

    Ok(offset)
}

pub fn needs_patching(data: &[u8]) -> bool {
    find_pattern(data, ORIGINAL_HOST).is_some()
}

pub fn patch_exe(exe_path: &std::path::Path, new_host: &str) -> Result<usize, PatchError> {
    let mut data = std::fs::read(exe_path)?;
    let offset = patch_hostname(&mut data, new_host)?;
    std::fs::write(exe_path, &data)?;
    Ok(offset)
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
        let offset = patch_hostname(&mut data, "localhost").unwrap();
        assert_eq!(offset, 40);
        assert_eq!(&data[40..49], b"localhost");
        assert!(data[49..62].iter().all(|&b| b == 0));
    }

    #[test]
    fn test_patch_hostname_max_length() {
        let mut data = make_fake_exe(b"www.stargateworlds.com");
        let host = "abcdefghijklmnopqrstuv";
        assert_eq!(host.len(), 22);
        let offset = patch_hostname(&mut data, host).unwrap();
        assert_eq!(offset, 40);
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

        let offset = patch_exe(&exe_path, "localhost").unwrap();
        assert_eq!(offset, 40);

        let patched = std::fs::read(&exe_path).unwrap();
        assert_eq!(&patched[40..49], b"localhost");
    }
}
