//! Wine runtime and Rosetta events (PD-14, F12). A runtime preparation or a
//! Rosetta check the player sees fail is one warn row with closed codes. No
//! URL, path or error text is written (R5).
use super::RuntimeError;
use crate::failure::FailureCode;
use crate::preflight::RosettaProbe;

impl RuntimeError {
    /// The failure taxonomy code (C-PD1) for this error; `None` for
    /// [`RuntimeError::Cancelled`], which is not a failure.
    pub fn failure(&self) -> Option<FailureCode> {
        match self {
            Self::Network => Some(FailureCode::WineRuntimeDownloadFailed),
            Self::Verification | Self::Extraction => Some(FailureCode::WineRuntimeInvalid),
            Self::Busy => Some(FailureCode::WineRuntimeBusy),
            Self::Io => Some(FailureCode::LocalIo),
            Self::Cancelled => None,
        }
    }

    /// The closed snake_case name of the variant, for event fields.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Io => "io",
            Self::Busy => "busy",
            Self::Network => "network",
            Self::Verification => "verification",
            Self::Extraction => "extraction",
            Self::Cancelled => "cancelled",
        }
    }
}

/// Runtime preparation failed. Nothing is written for a cancellation.
/// `http_status` is set only when the download got a non-success status.
pub(crate) fn runtime_prepare_failed(error: &RuntimeError, http_status: Option<u16>) {
    let Some(failure) = error.failure() else {
        return;
    };
    let error = error.code();
    let failure_code = failure.code();
    match http_status {
        Some(http_status) => tracing::warn!(
            target: "launcher.install",
            event = "runtime_prepare_failed",
            error,
            failure_code,
            http_status,
            "launcher install: the compatibility runtime could not be prepared; expected a verified runtime; the operation stops"
        ),
        None => tracing::warn!(
            target: "launcher.install",
            event = "runtime_prepare_failed",
            error,
            failure_code,
            "launcher install: the compatibility runtime could not be prepared; expected a verified runtime; the operation stops"
        ),
    }
}

/// The closed name of a Rosetta probe result.
fn rosetta_result(probe: RosettaProbe) -> &'static str {
    match probe {
        RosettaProbe::Present => "present",
        RosettaProbe::Missing => "missing",
        RosettaProbe::TimedOut => "timed_out",
        RosettaProbe::Error => "spawn_error",
    }
}

/// The Rosetta check before Wine preparation ran. INFO when Rosetta is
/// present, WARN otherwise.
pub(crate) fn rosetta_probe(probe: RosettaProbe) {
    let result = rosetta_result(probe);
    if probe == RosettaProbe::Present {
        tracing::info!(
            target: "launcher.install",
            event = "rosetta_probe",
            result,
            "launcher install: the Rosetta check passed; expected Rosetta 2; Wine preparation continues"
        );
    } else {
        tracing::warn!(
            target: "launcher.install",
            event = "rosetta_probe",
            result,
            "launcher install: the Rosetta check did not pass; expected Rosetta 2; the operation stops"
        );
    }
}
