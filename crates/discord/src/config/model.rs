//! The parsed [`Config`] / [`ChannelConfig`] types and their lookups.

use std::collections::HashMap;

use crate::event::{ChannelKind, EventKind};

use super::parse::RawConfig;
use super::{ConfigError, EventToggles};

/// Parsed, validated, env-interpolated Discord config.
///
/// `Config` is the type the sender and layer read on every event. Construct
/// via [`Config::from_toml_str`] (for tests/programmatic) or
/// [`ConfigWatcher::new`](super::ConfigWatcher::new) (for files with live
/// reload).
///
/// **Debug redacts webhook URLs.** Webhook URLs embed a per-channel
/// auth token; logging the raw config via `tracing::debug!(?config)`
/// would leak credentials. The custom `Debug` impl prints `<redacted>`
/// in place of the URL but keeps every other field intact for
/// diagnostics. `PartialEq` and the field accessors are unaffected.
#[derive(Clone, PartialEq)]
pub struct Config {
    pub enabled: bool,
    pub username: Option<String>,
    pub avatar_url: Option<String>,
    /// Per-channel webhook URL + rate limit. Channels with no entry in
    /// the TOML get `None` and are silently dropped from routing.
    pub channels: HashMap<ChannelKind, ChannelConfig>,
    /// Per-event-kind toggles. Every `EventKind` is keyed; missing TOML
    /// entries fall back to [`EventToggles::default`].
    pub events: EventToggles,
}

impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config")
            .field("enabled", &self.enabled)
            .field("username", &self.username)
            .field("avatar_url", &self.avatar_url)
            .field("channels", &self.channels)
            .field("events", &self.events)
            .finish()
    }
}

/// Webhook URL + rate limit for a single Discord channel.
///
/// **Debug redacts the URL.** See [`Config`] — printing the raw URL
/// would leak the webhook auth token. The rate limit is kept visible
/// because it's diagnostic-useful and not a secret.
#[derive(Clone, PartialEq)]
pub struct ChannelConfig {
    pub url: String,
    /// Discord allows 5 / 2 s per webhook (≈ 150 / min). Cap at this or
    /// below to leave headroom for retries.
    pub rate_limit_per_min: u32,
}

impl std::fmt::Debug for ChannelConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChannelConfig")
            .field("url", &"<redacted>")
            .field("rate_limit_per_min", &self.rate_limit_per_min)
            .finish()
    }
}

impl Config {
    /// Disabled config — used as the initial value when no TOML is
    /// available, and after a parse failure that we don't want to
    /// crash on.
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            username: None,
            avatar_url: None,
            channels: HashMap::new(),
            events: EventToggles::default(),
        }
    }

    /// Should `kind` post? Combines the per-event toggle, the global
    /// `enabled`, AND the presence of a webhook URL for the routed
    /// channel.
    pub fn should_post(&self, kind: EventKind) -> bool {
        if !self.enabled {
            return false;
        }
        if !self.events.is_enabled(kind) {
            return false;
        }
        let channel = crate::router::channel_for(kind);
        self.channels.contains_key(&channel)
    }

    /// Look up the webhook URL for an event's channel. `None` if the
    /// channel is unconfigured.
    pub fn webhook_url_for(&self, kind: EventKind) -> Option<&str> {
        let channel = crate::router::channel_for(kind);
        self.channels.get(&channel).map(|c| c.url.as_str())
    }

    /// Rate limit for the channel an event would post to.
    pub fn rate_limit_for(&self, kind: EventKind) -> Option<u32> {
        let channel = crate::router::channel_for(kind);
        self.channels.get(&channel).map(|c| c.rate_limit_per_min)
    }

    /// Parse TOML + interpolate env vars + validate. Public entry point
    /// for tests; production paths go through
    /// [`ConfigWatcher`](super::ConfigWatcher).
    pub fn from_toml_str(s: &str) -> Result<Self, ConfigError> {
        let raw: RawConfig = toml::from_str(s).map_err(ConfigError::Toml)?;
        let interpolated = raw.interpolate_env()?;
        interpolated.validate()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_config() {
        let cfg = Config::from_toml_str(&minimal_toml()).unwrap();
        assert!(cfg.enabled);
        assert_eq!(cfg.username.as_deref(), Some("Test"));
        assert_eq!(cfg.channels.len(), 1);
        let lc = cfg.channels.get(&ChannelKind::Lifecycle).unwrap();
        assert_eq!(lc.url, "https://discord.com/api/webhooks/1/abc");
        assert_eq!(lc.rate_limit_per_min, 30);
    }

    fn minimal_toml() -> String {
        // Pre-fill the env var so interpolation succeeds. Tests run in
        // process so this leaks across tests in the same process; we
        // use a distinctive name to avoid clash.
        std::env::set_var(
            "TEST_DISCORD_WEBHOOK",
            "https://discord.com/api/webhooks/1/abc",
        );
        r#"
[discord]
enabled = true
username = "Test"

[discord.channels.lifecycle]
url = "${TEST_DISCORD_WEBHOOK}"
rate_limit_per_min = 30
"#
        .to_string()
    }

    /// Regression guard: `Debug` formatting must never include the
    /// webhook URL or any substring of it. Webhook URLs embed a
    /// per-channel auth token; leaking one through `tracing::debug!`
    /// would compromise the channel until the operator rotates it.
    #[test]
    fn debug_format_redacts_webhook_url() {
        std::env::set_var(
            "TEST_DISCORD_REDACT_WEBHOOK",
            "https://discord.com/api/webhooks/1/SECRETTOKEN1234",
        );
        let toml_src = r#"
[discord]
enabled = true

[discord.channels.lifecycle]
url = "${TEST_DISCORD_REDACT_WEBHOOK}"
rate_limit_per_min = 60
"#;
        let cfg = Config::from_toml_str(toml_src).unwrap();
        let debug_repr = format!("{:?}", cfg);
        assert!(
            !debug_repr.contains("SECRETTOKEN1234"),
            "Debug must not leak the webhook token: {}",
            debug_repr
        );
        assert!(
            !debug_repr.contains("/api/webhooks/"),
            "Debug must not leak the webhook URL: {}",
            debug_repr
        );
        assert!(
            debug_repr.contains("<redacted>"),
            "Debug must replace the URL with a redaction marker: {}",
            debug_repr
        );
        // Non-secret fields must still be present for diagnostics.
        assert!(debug_repr.contains("enabled: true"));
        assert!(debug_repr.contains("rate_limit_per_min: 60"));
    }

    /// Bug shape: `should_post` returns true for an event whose channel
    /// isn't configured. Regression for "Discord rejects every post
    /// because we tried to send to a missing webhook" — the gate must
    /// also verify webhook presence, not just the toggle.
    #[test]
    fn should_post_false_when_channel_missing_even_if_toggle_on() {
        // Empty channels map → no webhook for `lifecycle`.
        let cfg = Config {
            enabled: true,
            username: None,
            avatar_url: None,
            channels: HashMap::new(),
            events: EventToggles::default(),
        };
        assert!(cfg.events.server_startup, "default toggle is on");
        assert!(
            !cfg.should_post(EventKind::ServerStartup),
            "no webhook configured → must NOT post even though toggle is on"
        );
    }

    #[test]
    fn should_post_false_when_disabled_globally() {
        std::env::set_var(
            "TEST_DISCORD_WEBHOOK",
            "https://discord.com/api/webhooks/1/abc",
        );
        let toml_src = r#"
[discord]
enabled = false

[discord.channels.lifecycle]
url = "${TEST_DISCORD_WEBHOOK}"
"#;
        let cfg = Config::from_toml_str(toml_src).unwrap();
        assert!(!cfg.should_post(EventKind::ServerStartup));
    }
}
