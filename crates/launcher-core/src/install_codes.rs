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
    fn local_seed_codes() {
        let error = LocalSeedError::SizeMismatch {
            expected: 10,
            actual: 4,
        };
        assert_eq!(error.code(), "size_mismatch");
    }
}
