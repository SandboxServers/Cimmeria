//! The capture switches the governor reads.
//!
//! - **`raw`** turns the governor off for the upload path: every event is
//!   forwarded as raised, nothing is collapsed or summarized. Health events
//!   are still emitted, so the drop counters stay visible. For a lab
//!   session that needs full volume on purpose.
//! - **`firehose`** keeps the governor on but widens it: the per-target
//!   budget is 16 times larger and per-entity streams keep 4 times as many
//!   events per entity. Hot streams are still summarized.
//!
//! Both come from the same places as the engine-sink switches of PR #1084:
//! the `CIMMERIA_CLIENT_CAPTURE` environment variable (a comma-separated
//! list) and the optional `capture` block of `current-session.json`. Either
//! source can turn a switch on. Unknown names are ignored on both sides, so
//! the lists can be shared.

use std::path::Path;

use serde::Deserialize;

/// Environment variable carrying a comma-separated switch list.
pub const ENV_VAR: &str = "CIMMERIA_CLIENT_CAPTURE";

/// The governor's switches.
#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq)]
pub struct Switches {
    /// Forward everything, summarize nothing.
    #[serde(default)]
    pub raw: bool,
    /// Widen the budgets.
    #[serde(default)]
    pub firehose: bool,
}

#[derive(Deserialize)]
struct SessionCapture {
    #[serde(default)]
    capture: Option<Switches>,
}

impl Switches {
    /// Parse a comma-separated switch list; unknown names are ignored.
    pub fn parse_list(list: &str) -> Self {
        let mut s = Self::default();
        for name in list.split(',').map(|n| n.trim().to_ascii_lowercase()) {
            match name.as_str() {
                "raw" => s.raw = true,
                "firehose" => s.firehose = true,
                _ => {}
            }
        }
        s
    }

    /// The switches in the session file's `capture` block. A missing file,
    /// bad JSON or no block all mean "off".
    pub fn from_session_json(text: &str) -> Self {
        serde_json::from_str::<SessionCapture>(text)
            .ok()
            .and_then(|s| s.capture)
            .unwrap_or_default()
    }

    /// The switches the environment asks for.
    pub fn from_env() -> Self {
        std::env::var(ENV_VAR)
            .map(|v| Self::parse_list(&v))
            .unwrap_or_default()
    }

    /// Session file next to `host_exe`, merged with the environment.
    pub fn for_host(host_exe: &Path) -> Self {
        let from_file = crate::session::session_path_for_host(host_exe)
            .ok()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .map(|t| Self::from_session_json(&t))
            .unwrap_or_default();
        from_file.merged(Self::from_env())
    }

    /// Either source turns a switch on.
    pub fn merged(self, other: Self) -> Self {
        Self {
            raw: self.raw || other.raw,
            firehose: self.firehose || other.firehose,
        }
    }
}
