//! Updater events (launcher-telemetry LT-12; C5, C6). Target `launcher.update`.
use super::Error;
use std::time::Instant;

impl Error {
    /// The closed code for this error (C6 `reason`). No wildcard arm.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Disabled => "disabled",
            Self::Busy => "busy",
            Self::StaleRevision => "stale_revision",
            Self::StaleOffer => "stale_offer",
            Self::Policy => "policy",
            Self::Feed => "feed",
            Self::Platform => "platform",
            Self::NotNewer => "not_newer",
            Self::Signature => "signature",
            Self::SignedVersion => "signed_version",
            Self::Size => "size",
            Self::Transport => "transport",
            Self::Timeout => "timeout",
            Self::Interrupted => "interrupted",
            Self::Package => "package",
            Self::Target => "target",
            Self::Replace => "replace",
            Self::Spawn => "spawn",
            Self::Reconciliation => "reconciliation",
            Self::Storage(_) => "storage",
        }
    }
}

fn elapsed_ms(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}

/// A release check that ended. `outcome` is `up_to_date` or `available`.
pub(super) fn check_finished(outcome: &'static str, started: Instant) {
    tracing::info!(
        target: "launcher.update",
        event = "update_check_finished",
        outcome,
        duration_ms = elapsed_ms(started),
        "launcher updater: release check finished; expected the feed to answer; an available release is offered, otherwise the launcher stays on its version"
    );
}

/// A feed check (`check`) or an artifact download that did not complete. The
/// error is a closed code; `status` is the HTTP status when the server answered.
pub(super) fn fetch_failed(check: bool, error: Error, status: Option<u16>) {
    let event = if check {
        "update_check_failed"
    } else {
        "update_download_failed"
    };
    tracing::warn!(
        target: "launcher.update",
        event,
        reason = error.code(),
        http_status = status,
        "launcher updater: fetch from the update server failed; expected the feed or package over https; the launcher keeps its version and retries later"
    );
}

/// The release package was downloaded; it is verified next.
pub(super) fn downloaded(bytes: u64, started: Instant) {
    tracing::info!(
        target: "launcher.update",
        event = "update_downloaded",
        bytes,
        duration_ms = elapsed_ms(started),
        "launcher updater: release package downloaded; expected a signed package to verify; it is verified before any install"
    );
}

/// A signature, version or feed check refused an offer or package. `reason` is
/// `signature`, `signed_version` or `feed`.
pub(super) fn verify_failed(reason: &'static str) {
    tracing::warn!(
        target: "launcher.update",
        event = "update_verify_failed",
        reason,
        "launcher updater: release refused; expected a package signed for the offered version; the update is not installed and the launcher keeps its version"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_capture::capture;

    #[test]
    fn a_failed_download_names_its_closed_code_and_status() {
        let ((), events) = capture(|| fetch_failed(false, Error::Transport, Some(503)));
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].target, "launcher.update");
        assert_eq!(events[0].level, tracing::Level::WARN);
        assert_eq!(events[0].field("event"), Some("update_download_failed"));
        assert_eq!(events[0].field("reason"), Some("transport"));
        assert_eq!(events[0].field("http_status"), Some("503"));
    }

    #[test]
    fn a_failed_check_without_a_status_carries_no_http_status() {
        let ((), events) = capture(|| fetch_failed(true, Error::Timeout, None));
        assert_eq!(events[0].field("event"), Some("update_check_failed"));
        assert_eq!(events[0].field("reason"), Some("timeout"));
        assert_eq!(events[0].field("http_status"), None);
    }

    #[test]
    fn a_finished_check_names_its_outcome_and_duration() {
        let ((), events) = capture(|| check_finished("up_to_date", Instant::now()));
        assert_eq!(events[0].level, tracing::Level::INFO);
        assert_eq!(events[0].field("event"), Some("update_check_finished"));
        assert_eq!(events[0].field("outcome"), Some("up_to_date"));
        assert!(events[0].field("duration_ms").is_some());
    }

    #[test]
    fn a_downloaded_package_names_its_size() {
        let ((), events) = capture(|| downloaded(4096, Instant::now()));
        assert_eq!(events[0].field("event"), Some("update_downloaded"));
        assert_eq!(events[0].field("bytes"), Some("4096"));
    }

    #[test]
    fn a_refused_signature_is_a_warn_with_its_reason() {
        let ((), events) = capture(|| verify_failed("signature"));
        assert_eq!(events[0].level, tracing::Level::WARN);
        assert_eq!(events[0].field("event"), Some("update_verify_failed"));
        assert_eq!(events[0].field("reason"), Some("signature"));
    }

    #[test]
    fn every_error_has_a_snake_case_code() {
        assert_eq!(Error::SignedVersion.code(), "signed_version");
        assert_eq!(Error::Storage(crate::StorageError::Io).code(), "storage");
    }
}
