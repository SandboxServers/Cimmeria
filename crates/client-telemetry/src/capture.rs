//! Capture switches for the engine-layer log sinks.
//!
//! Two switches, both off by default:
//!
//! - **`unfilter`** lifts the client's own log thresholds so the sinks see
//!   (and the client's own outputs get) messages it would normally drop:
//!   the BigWorld message filter, the log4cxx `isXEnabled` checks and the
//!   UE3 per-category suppress flag. Off, the sinks still report every
//!   message that reaches them; on, more messages reach them. The
//!   thresholds live in the game's memory, so this changes what the client
//!   itself writes to `SGWDebugLog.log` and `OutputDebugString` too. It is a
//!   lab/debug switch, not something a player build should carry.
//! - **`firehose`** raises the per-message rate limit of every sink from the
//!   default (burst 8, 4 per second per distinct message) to (burst 64, 64
//!   per second). The limit still exists: an injected DLL must not turn a
//!   chatty subsystem into a stall.
//!
//! They are read once at boot, from the optional top-level `capture` block
//! of `current-session.json` and from the `CIMMERIA_CLIENT_CAPTURE`
//! environment variable (a comma-separated list of switch names; either
//! source can turn a switch on, neither can turn the other's off). The
//! session file is written by the launcher or the lab supervisor; the
//! environment variable is for running the DLL by hand.

use std::sync::atomic::{AtomicBool, Ordering};

use serde::Deserialize;

/// Environment variable that turns switches on: `unfilter`, `firehose`.
pub const ENV_VAR: &str = "CIMMERIA_CLIENT_CAPTURE";

/// The `capture` block of `current-session.json`.
#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq)]
pub struct CaptureConfig {
    /// Lift the client's own log thresholds.
    #[serde(default)]
    pub unfilter: bool,
    /// Raise the sinks' rate limits.
    #[serde(default)]
    pub firehose: bool,
}

impl CaptureConfig {
    /// Parse a comma-separated switch list. Unknown names are ignored, so a
    /// newer supervisor can pass a switch an older DLL does not know.
    pub fn parse_list(list: &str) -> Self {
        let mut cfg = Self::default();
        for name in list.split(',').map(|s| s.trim().to_ascii_lowercase()) {
            match name.as_str() {
                "unfilter" => cfg.unfilter = true,
                "firehose" => cfg.firehose = true,
                _ => {}
            }
        }
        cfg
    }

    /// The switches the environment asks for.
    pub fn from_env() -> Self {
        std::env::var(ENV_VAR)
            .map(|v| Self::parse_list(&v))
            .unwrap_or_default()
    }

    /// Either source turns a switch on.
    pub fn merged(self, other: Self) -> Self {
        Self {
            unfilter: self.unfilter || other.unfilter,
            firehose: self.firehose || other.firehose,
        }
    }
}

static UNFILTER: AtomicBool = AtomicBool::new(false);
static FIREHOSE: AtomicBool = AtomicBool::new(false);

/// Set the process-wide switches. Called once from the bootstrap, before
/// any sink is installed.
pub fn init(cfg: CaptureConfig) {
    UNFILTER.store(cfg.unfilter, Ordering::Release);
    FIREHOSE.store(cfg.firehose, Ordering::Release);
}

/// The switches in force.
pub fn current() -> CaptureConfig {
    CaptureConfig {
        unfilter: UNFILTER.load(Ordering::Acquire),
        firehose: FIREHOSE.load(Ordering::Acquire),
    }
}

/// Whether the client's log thresholds are lifted.
pub fn unfilter() -> bool {
    UNFILTER.load(Ordering::Acquire)
}

/// Per-message limits for the sinks: `(burst, per_second)`.
pub fn sink_limits() -> (u32, u32) {
    if FIREHOSE.load(Ordering::Acquire) {
        FIREHOSE_LIMITS
    } else {
        DEFAULT_LIMITS
    }
}

/// The default limits (same as the event-registry hook's).
pub const DEFAULT_LIMITS: (u32, u32) = (8, 4);

/// The `firehose` limits.
pub const FIREHOSE_LIMITS: (u32, u32) = (64, 64);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_switch_list_turns_on_what_it_names() {
        assert_eq!(CaptureConfig::parse_list(""), CaptureConfig::default());
        assert_eq!(
            CaptureConfig::parse_list("unfilter"),
            CaptureConfig {
                unfilter: true,
                firehose: false
            }
        );
        assert_eq!(
            CaptureConfig::parse_list(" Unfilter , FIREHOSE "),
            CaptureConfig {
                unfilter: true,
                firehose: true
            }
        );
    }

    /// A supervisor newer than the DLL may pass switches it has not heard of.
    #[test]
    fn unknown_switches_are_ignored() {
        assert_eq!(
            CaptureConfig::parse_list("nonsense,firehose,,x=y"),
            CaptureConfig {
                unfilter: false,
                firehose: true
            }
        );
    }

    #[test]
    fn either_source_can_turn_a_switch_on() {
        let session = CaptureConfig {
            unfilter: true,
            firehose: false,
        };
        let env = CaptureConfig {
            unfilter: false,
            firehose: true,
        };
        assert_eq!(
            session.merged(env),
            CaptureConfig {
                unfilter: true,
                firehose: true
            }
        );
    }

    /// The session block may carry either key, both or neither.
    #[test]
    fn the_session_block_parses_partial_objects() {
        let cfg: CaptureConfig = serde_json::from_str(r#"{"unfilter": true}"#).unwrap();
        assert!(cfg.unfilter && !cfg.firehose);
        let cfg: CaptureConfig = serde_json::from_str("{}").unwrap();
        assert_eq!(cfg, CaptureConfig::default());
    }

    #[test]
    fn firehose_raises_the_limits() {
        assert!(FIREHOSE_LIMITS.0 > DEFAULT_LIMITS.0);
        assert!(FIREHOSE_LIMITS.1 > DEFAULT_LIMITS.1);
    }
}
