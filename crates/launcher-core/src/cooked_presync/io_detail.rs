//! Names an I/O failure for the presync's report: a stable `io_kind` code and
//! the OS error number, when the OS gave one.

use std::io::ErrorKind;

use serde::Serialize;

/// The code and OS number of one I/O failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct IoDetail {
    /// The stable `io_kind` code, such as `sharing_violation`.
    pub kind: &'static str,
    /// The raw OS error number, when the failure has one.
    pub os_code: Option<i32>,
}

/// Names one I/O error. The Windows codes come first, because the OS gives
/// a more exact answer than the portable kind does.
pub fn io_detail_of(error: &std::io::Error) -> IoDetail {
    let os_code = error.raw_os_error();
    let kind = match os_code {
        Some(32) => "sharing_violation",
        Some(33) => "lock_violation",
        Some(112) => "storage_full",
        _ => match error.kind() {
            ErrorKind::StorageFull => "storage_full",
            ErrorKind::PermissionDenied => "permission_denied",
            ErrorKind::NotFound => "not_found",
            ErrorKind::ConnectionRefused => "connection_refused",
            ErrorKind::ConnectionReset => "connection_reset",
            ErrorKind::TimedOut => "timed_out",
            ErrorKind::HostUnreachable => "host_unreachable",
            ErrorKind::NetworkUnreachable => "network_unreachable",
            _ => "other",
        },
    };
    IoDetail { kind, os_code }
}

/// Walks an error's `source()` chain to the first `std::io::Error` and names
/// it. A transport error keeps its I/O cause a few levels down.
pub fn io_detail_in(error: &(dyn std::error::Error + 'static)) -> Option<IoDetail> {
    let mut current: Option<&(dyn std::error::Error + 'static)> = Some(error);
    while let Some(link) = current {
        if let Some(io) = link.downcast_ref::<std::io::Error>() {
            return Some(io_detail_of(io));
        }
        current = link.source();
    }
    None
}

#[cfg(test)]
mod tests {
    use std::io::{Error, ErrorKind};

    use super::*;

    #[test]
    fn io_detail_maps_the_windows_codes() {
        assert_eq!(
            io_detail_of(&Error::from_raw_os_error(32)),
            IoDetail {
                kind: "sharing_violation",
                os_code: Some(32)
            }
        );
        assert_eq!(
            io_detail_of(&Error::from_raw_os_error(33)).kind,
            "lock_violation"
        );
        assert_eq!(
            io_detail_of(&Error::from_raw_os_error(112)).kind,
            "storage_full"
        );
    }

    #[test]
    fn io_detail_maps_portable_kinds_without_an_os_code() {
        let refused = io_detail_of(&Error::new(ErrorKind::ConnectionRefused, "no"));
        assert_eq!(
            refused,
            IoDetail {
                kind: "connection_refused",
                os_code: None
            }
        );
        let other = io_detail_of(&Error::other("odd"));
        assert_eq!(other.kind, "other");
    }

    #[test]
    fn io_detail_in_finds_the_io_cause_under_a_wrapper() {
        #[derive(Debug)]
        struct Wrapper(Error);
        impl std::fmt::Display for Wrapper {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("wrapped")
            }
        }
        impl std::error::Error for Wrapper {
            fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
                Some(&self.0)
            }
        }
        let wrapped = Wrapper(Error::from_raw_os_error(33));
        assert_eq!(
            io_detail_in(&wrapped).map(|d| d.kind),
            Some("lock_violation")
        );
        assert_eq!(io_detail_in(&std::fmt::Error), None);
    }
}
