//! Closed snake_case codes for core install errors in launcher events (C6).
use crate::install::InstallError;
use crate::seed_source::LocalSeedError;

impl InstallError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Http { .. } => "http",
            Self::Io { .. } => "io",
            Self::Unpack { .. } => "unpack",
            Self::State { .. } => "state",
            Self::HashMismatch { .. } => "hash_mismatch",
            Self::UnexpectedStatus { .. } => "unexpected_status",
            Self::InvalidSha256 { .. } => "invalid_sha256",
            Self::Cancelled => "cancelled",
            Self::SeedSwitched => "seed_switched",
            Self::LocalSeed { .. } => "local_seed",
            Self::SeedExtractionUncertain => "seed_extraction_uncertain",
            Self::PatchDest { .. } => "patch_dest",
            Self::PatchesFailed { .. } => "patches_failed",
        }
    }
    /// The HTTP status behind the error, when there is one. Never the URL.
    pub fn http_status(&self) -> Option<u16> {
        match self {
            Self::UnexpectedStatus { status, .. } => Some(*status),
            Self::Http(error) => error.status().map(|s| s.as_u16()),
            _ => None,
        }
    }
    /// The unpack error's closed code, only for an unpack failure (`unpack_kind`).
    pub fn unpack_code(&self) -> Option<&'static str> {
        match self {
            Self::Unpack(error) => Some(error.code()),
            _ => None,
        }
    }
}

impl crate::unpack::UnpackError {
    /// The closed contract code of an unpack error. Never the entry name or path.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Io(_) => "io",
            Self::Zip(_) => "zip",
            Self::Rar(_) => "rar",
            Self::Cab(_) => "cab",
            Self::UnknownFormat(_) => "unknown_format",
            Self::UnsafePath(_) => "unsafe_path",
            Self::EntryConflict(_) => "entry_conflict",
            Self::Patchset(_) => "patchset",
            Self::PatchsetHash { .. } => "patchset_hash",
            Self::Cancelled => "cancelled",
        }
    }
}

impl LocalSeedError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::NotAFile { .. } => "not_a_file",
            Self::SizeMismatch { .. } => "size_mismatch",
            Self::HashMismatch { .. } => "hash_mismatch",
            Self::Changed { .. } => "changed",
            Self::Cancelled { .. } => "cancelled",
            Self::Io { .. } => "io",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unexpected_status_reports_its_code_and_status() {
        let error = InstallError::UnexpectedStatus {
            status: 503,
            url: "https://example.invalid/blob".into(),
        };
        assert_eq!(error.code(), "unexpected_status");
        assert_eq!(error.http_status(), Some(503));
    }

    #[test]
    fn cancelled_has_no_status() {
        assert_eq!(InstallError::Cancelled.code(), "cancelled");
        assert_eq!(InstallError::Cancelled.http_status(), None);
    }

    #[test]
    fn io_is_io() {
        assert_eq!(InstallError::Io(std::io::Error::other("x")).code(), "io");
    }

    #[test]
    fn every_unpack_variant_has_its_contract_code() {
        use crate::unpack::UnpackError;
        let cases = [
            (UnpackError::Io(std::io::Error::other("x")), "io"),
            (
                UnpackError::Zip(::zip::result::ZipError::FileNotFound),
                "zip",
            ),
            (UnpackError::Rar("x".into()), "rar"),
            (UnpackError::Cab("x".into()), "cab"),
            (UnpackError::UnknownFormat("x".into()), "unknown_format"),
            (UnpackError::UnsafePath("x".into()), "unsafe_path"),
            (UnpackError::EntryConflict("x".into()), "entry_conflict"),
            (UnpackError::Patchset("x".into()), "patchset"),
            (
                UnpackError::PatchsetHash {
                    kind: "source_mismatch",
                    path: "x".into(),
                    expected: "a".into(),
                    actual: "b".into(),
                    message: "x".into(),
                },
                "patchset_hash",
            ),
            (UnpackError::Cancelled, "cancelled"),
        ];
        for (error, code) in cases {
            assert_eq!(error.code(), code);
        }
    }

    #[test]
    fn only_an_unpack_failure_has_an_unpack_code() {
        let error = InstallError::Unpack(crate::unpack::UnpackError::Cab("x".into()));
        assert_eq!(error.unpack_code(), Some("cab"));
        assert_eq!(InstallError::Cancelled.unpack_code(), None);
        assert_eq!(
            InstallError::Io(std::io::Error::other("x")).unpack_code(),
            None
        );
    }

    #[test]
    fn local_seed_codes() {
        let error = LocalSeedError::SizeMismatch {
            expected: 10,
            actual: 4,
        };
        assert_eq!(error.code(), "size_mismatch");
    }
}
