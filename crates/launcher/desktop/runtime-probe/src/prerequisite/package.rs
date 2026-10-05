//! Extract the exact original vendor MSI from its authenticated Wise wrapper.
use sha2::{Digest, Sha256};

pub const PHYSX_EXE_BYTES: usize = 39_242_016;
pub const PHYSX_EXE_SHA256: &str =
    "920d5e09e6ba0a92342271c18c67472461813424d70b5c0b981b6f13b129fbf6";
const MSI_OFFSET: usize = 35_463;
const MSI_BYTES: usize = 38_811_648;
const MSI_SHA256: &str = "3f122f4be03c6ae42652d28cc5ed48669e0348c8d3650f389954014992a06c8b";
const COMPOUND_HEADER: &[u8] = &[0xd0, 0xcf, 0x11, 0xe0, 0xa1, 0xb1, 0x1a, 0xe1];

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum PackageError {
    #[error("prerequisite package identity does not match")]
    Identity,
    #[error("prerequisite payload does not match the extraction recipe")]
    Payload,
}

/// Return only the MSI bytes from the original PhysX 7.11.13 installer.
///
/// The entire EXE is authenticated before its fixed container boundary is used;
/// no caller-selected offset, heuristic scan, network URL or external extractor
/// is accepted. The independent payload hash detects recipe drift as well.
/// The caller owns source-file safety, bounded reading and destination creation.
/// This function neither accepts license terms nor grants execution permission.
pub fn physx_msi(package: &[u8]) -> Result<&[u8], PackageError> {
    if package.len() != PHYSX_EXE_BYTES || digest(package) != PHYSX_EXE_SHA256 {
        return Err(PackageError::Identity);
    }
    let payload = package
        .get(MSI_OFFSET..MSI_OFFSET + MSI_BYTES)
        .ok_or(PackageError::Payload)?;
    if !payload.starts_with(COMPOUND_HEADER) || digest(payload) != MSI_SHA256 {
        return Err(PackageError::Payload);
    }
    Ok(payload)
}

fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_truncation_and_appended_container_data() {
        for size in [
            0,
            MSI_OFFSET + MSI_BYTES,
            PHYSX_EXE_BYTES - 1,
            PHYSX_EXE_BYTES + 1,
        ] {
            assert_eq!(physx_msi(&vec![0; size]), Err(PackageError::Identity));
        }
    }

    #[test]
    fn compound_signature_at_expected_offset_cannot_bypass_exe_identity() {
        let mut forged = vec![0; PHYSX_EXE_BYTES];
        forged[MSI_OFFSET..MSI_OFFSET + COMPOUND_HEADER.len()].copy_from_slice(COMPOUND_HEADER);
        assert_eq!(physx_msi(&forged), Err(PackageError::Identity));
    }
}
