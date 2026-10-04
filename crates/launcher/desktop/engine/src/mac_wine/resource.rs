//! Native build-pinned helper resource; never deserialized from webview input.
use super::*;
#[derive(Clone)]
pub struct HelperResource {
    path: PathBuf,
    sha256: [u8; 32],
}
impl HelperResource {
    /// Expected identity comes from the trusted build, independently of this file.
    pub fn open(path: PathBuf, expected_hex: &str) -> Result<Self, WineError> {
        let sha256 = decode(expected_hex)?;
        verify_file(&path, &sha256)?;
        Ok(Self { path, sha256 })
    }
    pub fn verify(&self) -> Result<(), WineError> {
        verify_file(&self.path, &self.sha256)
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn backend(&self) -> ExtractionBackend {
        ExtractionBackend::Wine {
            runtime_sha256: decode(mac_runtime::ARCHIVE_SHA256).expect("fixed runtime hash"),
            helper_sha256: self.sha256,
        }
    }
}
fn decode(value: &str) -> Result<[u8; 32], WineError> {
    if value.len() != 64 || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(WineError::Invalid);
    }
    let mut bytes = [0; 32];
    for (out, pair) in bytes.iter_mut().zip(value.as_bytes().as_chunks::<2>().0) {
        *out = u8::from_str_radix(
            std::str::from_utf8(pair).map_err(|_| WineError::Invalid)?,
            16,
        )
        .map_err(|_| WineError::Invalid)?;
    }
    Ok(bytes)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn independent_identity_is_required_and_replacement_is_rejected() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("helper.exe");
        std::fs::write(&path, b"fixture").unwrap();
        let hash = hex(&Sha256::digest(b"fixture"));
        for bad in ["", "é", &"00".repeat(32)] {
            assert!(HelperResource::open(path.clone(), bad).is_err());
        }
        let resource = HelperResource::open(path.clone(), &hash).unwrap();
        assert!(
            matches!(resource.backend(),ExtractionBackend::Wine{helper_sha256,..} if hex(&helper_sha256)==hash)
        );
        std::fs::write(path, b"replacement").unwrap();
        assert!(resource.verify().is_err());
    }
}
