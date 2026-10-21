//! The `telemetry_status` command (C13): what the launcher's game-diagnostics
//! pipeline last did, for the status line under "Send game diagnostics". It reads
//! the process-wide health (launcher-telemetry C7) and returns codes and counts
//! only. The age is computed here, from the last success, at the time of the call.
use cimmeria_launcher_engine::game_telemetry::health::TelemetryHealth;
use std::sync::PoisonError;
use std::time::{SystemTime, UNIX_EPOCH};

/// What the status line shows (C13). Outcomes and reasons are closed codes.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub(crate) struct TelemetryStatus {
    pub last_outcome: Option<&'static str>,
    pub last_reason: Option<&'static str>,
    pub last_success_age_s: Option<u64>,
    pub failures_since_success: u64,
}

/// The status at `now_ms`, in Unix milliseconds, the clock the health records with.
pub(crate) fn status(health: &TelemetryHealth, now_ms: u64) -> TelemetryStatus {
    let failed = health.since_success;
    TelemetryStatus {
        last_outcome: health.last_outcome,
        last_reason: health.last_reason,
        last_success_age_s: health
            .last_success_ms
            .map(|at| now_ms.saturating_sub(at) / 1000),
        failures_since_success: failed.mint_failed + failed.held + failed.dropped,
    }
}

/// Snapshots the pipeline health for the Settings status line (C13).
#[tauri::command]
pub(crate) fn telemetry_status() -> TelemetryStatus {
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| {
            u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX)
        });
    let health = crate::telemetry::health();
    let state = health.lock().unwrap_or_else(PoisonError::into_inner);
    status(&state, now_ms)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cimmeria_launcher_engine::game_telemetry::launcher_session::SendOutcome;

    #[test]
    fn a_seeded_run_of_failures_reports_the_last_outcome_reason_and_age() {
        let mut health = TelemetryHealth::default();
        health.record(&SendOutcome::Sent, 1_000);
        health.record(
            &SendOutcome::MintFailed {
                reason: "http_status",
            },
            2_000,
        );
        health.record(&SendOutcome::Held, 3_000);
        health.record(&SendOutcome::Dropped, 4_000);
        assert_eq!(
            status(&health, 126_000),
            TelemetryStatus {
                last_outcome: Some("dropped"),
                last_reason: Some("http_status"),
                last_success_age_s: Some(125),
                failures_since_success: 3,
            }
        );
    }

    #[test]
    fn a_success_clears_the_failures_and_shows_the_sent_outcome() {
        let mut health = TelemetryHealth::default();
        health.record(&SendOutcome::Held, 10);
        health.record(&SendOutcome::Sent, 2_000);
        assert_eq!(
            status(&health, 2_000),
            TelemetryStatus {
                last_outcome: Some("sent"),
                last_reason: None,
                last_success_age_s: Some(0),
                failures_since_success: 0,
            }
        );
    }

    #[test]
    fn a_fresh_pipeline_has_nothing_to_report() {
        assert_eq!(
            status(&TelemetryHealth::default(), 5_000),
            TelemetryStatus {
                last_outcome: None,
                last_reason: None,
                last_success_age_s: None,
                failures_since_success: 0,
            }
        );
    }
}
