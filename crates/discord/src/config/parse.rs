//! TOML schema (pre-interpolation), env-var substitution, validation, and
//! the [`ConfigError`] type.

use std::collections::HashMap;
use std::path::PathBuf;

use serde::Deserialize;
use thiserror::Error;

use crate::event::{ChannelKind, EventKind};

use super::model::{ChannelConfig, Config};
use super::EventToggles;

// ── TOML schema (intermediate, pre-interpolation) ───────────────────────

#[derive(Debug, Deserialize)]
pub(super) struct RawConfig {
    discord: RawDiscord,
}

#[derive(Debug, Deserialize)]
struct RawDiscord {
    #[serde(default)]
    enabled: bool,
    #[serde(default)]
    username: Option<String>,
    #[serde(default)]
    avatar_url: Option<String>,
    #[serde(default)]
    channels: HashMap<String, RawChannel>,
    #[serde(default)]
    events: HashMap<String, bool>,
}

#[derive(Debug, Deserialize, Clone)]
struct RawChannel {
    url: String,
    #[serde(default = "default_rate_limit")]
    rate_limit_per_min: u32,
}

const fn default_rate_limit() -> u32 {
    60
}

impl RawConfig {
    /// Substitute `${ENV_VAR}` patterns in all channel URLs. Missing vars
    /// produce an error (better than silently posting to "${VAR}").
    pub(super) fn interpolate_env(mut self) -> Result<Self, ConfigError> {
        for (name, ch) in self.discord.channels.iter_mut() {
            ch.url = interpolate_env_vars(&ch.url, name)?;
        }
        Ok(self)
    }

    pub(super) fn validate(self) -> Result<Config, ConfigError> {
        // Channels: parse string keys into `ChannelKind`.
        let mut channels = HashMap::new();
        for (key, ch) in self.discord.channels {
            let kind = parse_channel_kind(&key)?;
            if !ch.url.starts_with("https://discord.com/api/webhooks/")
                && !ch.url.starts_with("https://discordapp.com/api/webhooks/")
            {
                return Err(ConfigError::BadWebhookUrl {
                    channel: key,
                    url: ch.url,
                });
            }
            // Sanity-cap the rate limit. Discord allows ~150/min; over
            // that and we'll burn 429s.
            let rate_limit_per_min = ch.rate_limit_per_min.clamp(1, 150);
            channels.insert(
                kind,
                ChannelConfig {
                    url: ch.url,
                    rate_limit_per_min,
                },
            );
        }

        // Events: start from defaults, then overlay TOML keys. Unknown
        // keys are rejected (typo guard).
        let mut events = EventToggles::default();
        for (key, value) in self.discord.events {
            apply_event_toggle(&mut events, &key, value)?;
        }

        Ok(Config {
            enabled: self.discord.enabled,
            username: self.discord.username,
            avatar_url: self.discord.avatar_url,
            channels,
            events,
        })
    }
}

fn parse_channel_kind(s: &str) -> Result<ChannelKind, ConfigError> {
    for c in ChannelKind::ALL {
        if c.as_str() == s {
            return Ok(*c);
        }
    }
    Err(ConfigError::UnknownChannel(s.to_string()))
}

/// Overlay a single `events.foo = bool` entry onto the toggle struct.
/// Unknown keys return `Err` (typo guard — silently ignoring would let
/// a typo'd `playr_login = false` silently NOT take effect).
fn apply_event_toggle(t: &mut EventToggles, key: &str, value: bool) -> Result<(), ConfigError> {
    // Find the EventKind whose `as_str()` matches the key.
    let kind = EventKind::ALL
        .iter()
        .copied()
        .find(|k| k.as_str() == key)
        .ok_or_else(|| ConfigError::UnknownEvent(key.to_string()))?;
    match kind {
        EventKind::ServerStartup => t.server_startup = value,
        EventKind::ServerShutdown => t.server_shutdown = value,
        EventKind::ServerPanic => t.server_panic = value,
        EventKind::PlayerLogin => t.player_login = value,
        EventKind::PlayerLogout => t.player_logout = value,
        EventKind::PlayerDisconnect => t.player_disconnect = value,
        EventKind::PlayerAuthFailed => t.player_auth_failed = value,
        EventKind::PlayerWorldEntry => t.player_world_entry = value,
        EventKind::PlayerWorldExit => t.player_world_exit = value,
        EventKind::ChatGlobal => t.chat_global = value,
        EventKind::ChatSay => t.chat_say = value,
        EventKind::ChatWhisper => t.chat_whisper = value,
        EventKind::ChatGuild => t.chat_guild = value,
        EventKind::ChatTeam => t.chat_team = value,
        EventKind::ChatCommand => t.chat_command = value,
        EventKind::PlayerLevelUp => t.player_level_up = value,
        EventKind::PlayerDeath => t.player_death = value,
        EventKind::PlayerRespawn => t.player_respawn = value,
        EventKind::MissionAccepted => t.mission_accepted = value,
        EventKind::MissionCompleted => t.mission_completed = value,
        EventKind::MissionFailed => t.mission_failed = value,
        EventKind::MissionRewardGranted => t.mission_reward_granted = value,
        EventKind::LootGenerated => t.loot_generated = value,
        EventKind::ItemUsed => t.item_used = value,
        EventKind::CharacterCreated => t.character_created = value,
        EventKind::NpcDeath => t.npc_death = value,
        EventKind::MinigameResult => t.minigame_result = value,
        EventKind::Dialog => t.dialog = value,
        EventKind::GmCommand => t.gm_command = value,
        EventKind::GmTeleport => t.gm_teleport = value,
        EventKind::GmSpawn => t.gm_spawn = value,
        EventKind::GmItemGrant => t.gm_item_grant = value,
        EventKind::Warning => t.warning = value,
        EventKind::Error => t.error = value,
        EventKind::WireFormatError => t.wire_format_error = value,
        EventKind::DbError => t.db_error = value,
        EventKind::AssertionFailure => t.assertion_failure = value,
        EventKind::MercuryTimeout => t.mercury_timeout = value,
        EventKind::HighLatency => t.high_latency = value,
        EventKind::PacketLossSpike => t.packet_loss_spike = value,
        EventKind::MemoryWarning => t.memory_warning = value,
        EventKind::TickStall => t.tick_stall = value,
        EventKind::AoiBurstWarning => t.aoi_burst_warning = value,
        EventKind::OutboxLag => t.outbox_lag = value,
    }
    Ok(())
}

// ── Env var interpolation ───────────────────────────────────────────────

/// Replace `${ENV_VAR}` placeholders with their values.
///
/// `channel` is included in the error message for context. Bare `$` chars
/// are passed through.
fn interpolate_env_vars(s: &str, channel: &str) -> Result<String, ConfigError> {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(start) = rest.find("${") {
        out.push_str(&rest[..start]);
        rest = &rest[start + 2..];
        let end = rest.find('}').ok_or_else(|| ConfigError::UnclosedEnvVar {
            channel: channel.to_string(),
        })?;
        let var = &rest[..end];
        let value = std::env::var(var).map_err(|_| ConfigError::MissingEnvVar {
            channel: channel.to_string(),
            var: var.to_string(),
        })?;
        out.push_str(&value);
        rest = &rest[end + 1..];
    }
    out.push_str(rest);
    Ok(out)
}

// ── Errors ──────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to parse Discord config TOML: {0}")]
    Toml(toml::de::Error),

    #[error("unknown channel name in [discord.channels]: `{0}` (expected one of: lifecycle, auth, world, chat, gameplay, gm, errors, ops)")]
    UnknownChannel(String),

    #[error(
        "unknown event in [discord.events]: `{0}` (typo? expected one of the EventKind names)"
    )]
    UnknownEvent(String),

    #[error("channel `{channel}` URL is not a Discord webhook URL: `{url}`")]
    BadWebhookUrl { channel: String, url: String },

    #[error("channel `{channel}` URL contains unclosed `${{...`")]
    UnclosedEnvVar { channel: String },

    #[error("channel `{channel}` URL references env var `{var}` which is not set")]
    MissingEnvVar { channel: String, var: String },

    #[error("failed to read Discord config file `{path}`: {error}")]
    Io {
        path: PathBuf,
        #[source]
        error: std::io::Error,
    },

    #[error("file watcher setup failed: {0}")]
    Watcher(notify::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression guard: the TOML shape produced by the colo overlay
    /// (rendered from `docker/compose.discord.yml` by the release
    /// workflow) must round-trip cleanly through `Config::from_toml_str`.
    ///
    /// If a future schema change to the crate breaks this shape, the
    /// colo deployment goes down on the next watchtower swap — and the
    /// failure is silent unless an operator is watching startup logs.
    /// Pinning the rendered shape here means schema changes that drop
    /// this contract trip CI instead of production.
    ///
    /// The TOML below is byte-identical to the rendered output of
    /// `awk render | colo-deploy upload`, with two webhook URLs
    /// substituted. Update both places in lockstep when you change
    /// either side.
    #[test]
    fn rendered_colo_overlay_toml_parses() {
        let toml_src = r#"
[discord]
enabled = true
username = "Cimmeria (colo)"

[discord.channels.lifecycle]
url = "https://discord.com/api/webhooks/1/lifeABC"
rate_limit_per_min = 30

[discord.channels.errors]
url = "https://discord.com/api/webhooks/2/errXYZ"
rate_limit_per_min = 60

[discord.events]
warning = true
"#;
        let cfg = Config::from_toml_str(toml_src).expect("colo overlay TOML must parse");
        assert!(cfg.enabled);
        assert_eq!(cfg.username.as_deref(), Some("Cimmeria (colo)"));

        // Two channels configured, both webhook URLs preserved.
        assert_eq!(cfg.channels.len(), 2);
        let lc = cfg
            .channels
            .get(&ChannelKind::Lifecycle)
            .expect("lifecycle present");
        assert_eq!(lc.url, "https://discord.com/api/webhooks/1/lifeABC");
        assert_eq!(lc.rate_limit_per_min, 30);
        let err = cfg
            .channels
            .get(&ChannelKind::Errors)
            .expect("errors present");
        assert_eq!(err.url, "https://discord.com/api/webhooks/2/errXYZ");
        assert_eq!(err.rate_limit_per_min, 60);

        // The overlay's `warning = true` override took effect; other
        // toggles inherited defaults.
        assert!(cfg.events.warning, "colo overlay opts into warning");
        assert!(cfg.events.error, "default-on retained");
        assert!(cfg.events.server_startup, "default-on retained");
        assert!(!cfg.events.chat_whisper, "default-off retained");

        // `should_post` returns true for lifecycle/errors events and
        // false for channels we deliberately didn't render (auth,
        // world, etc. — no channel block, so they drop).
        assert!(cfg.should_post(EventKind::ServerStartup));
        assert!(cfg.should_post(EventKind::Error));
        assert!(
            !cfg.should_post(EventKind::PlayerLogin),
            "PlayerLogin routes to 'auth' channel, which isn't configured in the overlay"
        );
    }

    /// Single-channel variant: only `DISCORD_LIFECYCLE_WEBHOOK` set on
    /// the GH Actions secrets. The render step drops the entire
    /// `[discord.channels.errors]` block. Pin that the resulting TOML
    /// still parses cleanly with just one channel.
    #[test]
    fn rendered_colo_overlay_single_channel_parses() {
        let toml_src = r#"
[discord]
enabled = true
username = "Cimmeria (colo)"

[discord.channels.lifecycle]
url = "https://discord.com/api/webhooks/1/lifeABC"
rate_limit_per_min = 30

[discord.events]
warning = true
"#;
        let cfg = Config::from_toml_str(toml_src).expect("single-channel overlay must parse");
        assert_eq!(cfg.channels.len(), 1);
        assert!(cfg.channels.contains_key(&ChannelKind::Lifecycle));
        assert!(!cfg.channels.contains_key(&ChannelKind::Errors));
        // Errors-routed events become should_post=false because the
        // channel isn't configured. The toggle is still `true` (it's
        // the default) but routing-level gating drops it.
        assert!(!cfg.should_post(EventKind::Error));
        assert!(cfg.should_post(EventKind::ServerStartup));
    }

    #[test]
    fn toml_event_overlay_takes_effect() {
        std::env::set_var(
            "TEST_DISCORD_WEBHOOK",
            "https://discord.com/api/webhooks/1/abc",
        );
        let toml_src = r#"
[discord]
enabled = true

[discord.channels.errors]
url = "${TEST_DISCORD_WEBHOOK}"

[discord.events]
warning = true
chat_whisper = true
"#;
        let cfg = Config::from_toml_str(toml_src).unwrap();
        assert!(cfg.events.warning);
        assert!(cfg.events.chat_whisper);
        // Untouched defaults preserved
        assert!(cfg.events.server_startup);
        assert!(!cfg.events.chat_say);
    }

    #[test]
    fn unknown_event_key_rejected() {
        std::env::set_var(
            "TEST_DISCORD_WEBHOOK",
            "https://discord.com/api/webhooks/1/abc",
        );
        let toml_src = r#"
[discord]
enabled = true

[discord.channels.errors]
url = "${TEST_DISCORD_WEBHOOK}"

[discord.events]
playr_login = true
"#;
        let err = Config::from_toml_str(toml_src).unwrap_err();
        assert!(matches!(err, ConfigError::UnknownEvent(ref s) if s == "playr_login"));
    }

    #[test]
    fn unknown_channel_key_rejected() {
        std::env::set_var(
            "TEST_DISCORD_WEBHOOK",
            "https://discord.com/api/webhooks/1/abc",
        );
        let toml_src = r#"
[discord]
enabled = true

[discord.channels.audit]
url = "${TEST_DISCORD_WEBHOOK}"
"#;
        let err = Config::from_toml_str(toml_src).unwrap_err();
        assert!(matches!(err, ConfigError::UnknownChannel(ref s) if s == "audit"));
    }

    #[test]
    fn non_discord_webhook_url_rejected() {
        let toml_src = r#"
[discord]
enabled = true

[discord.channels.errors]
url = "https://example.com/hook"
"#;
        let err = Config::from_toml_str(toml_src).unwrap_err();
        assert!(matches!(err, ConfigError::BadWebhookUrl { .. }));
    }

    #[test]
    fn missing_env_var_rejected() {
        // Ensure the env var is not present.
        std::env::remove_var("TEST_MISSING_DISCORD_WEBHOOK");
        let toml_src = r#"
[discord]
enabled = true

[discord.channels.errors]
url = "${TEST_MISSING_DISCORD_WEBHOOK}"
"#;
        let err = Config::from_toml_str(toml_src).unwrap_err();
        assert!(
            matches!(err, ConfigError::MissingEnvVar { ref var, .. } if var == "TEST_MISSING_DISCORD_WEBHOOK")
        );
    }

    #[test]
    fn rate_limit_clamped_to_safe_range() {
        std::env::set_var(
            "TEST_DISCORD_WEBHOOK",
            "https://discord.com/api/webhooks/1/abc",
        );
        let toml_src = r#"
[discord]
enabled = true

[discord.channels.errors]
url = "${TEST_DISCORD_WEBHOOK}"
rate_limit_per_min = 9999
"#;
        let cfg = Config::from_toml_str(toml_src).unwrap();
        let ch = cfg.channels.get(&ChannelKind::Errors).unwrap();
        assert!(
            ch.rate_limit_per_min <= 150,
            "must clamp to Discord's burst budget"
        );
    }

    #[test]
    fn env_var_interpolation_handles_multiple() {
        std::env::set_var("TEST_A", "alpha");
        std::env::set_var("TEST_B", "bravo");
        let out = interpolate_env_vars("x/${TEST_A}/y/${TEST_B}/z", "test").unwrap();
        assert_eq!(out, "x/alpha/y/bravo/z");
    }

    #[test]
    fn env_var_unclosed_returns_error() {
        let err = interpolate_env_vars("${OOPS", "test").unwrap_err();
        assert!(matches!(err, ConfigError::UnclosedEnvVar { .. }));
    }
}
