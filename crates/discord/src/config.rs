//! Discord-notifications configuration.
//!
//! TOML schema, `${ENV_VAR}` substitution, live reload via `notify`. The
//! config is held inside an `ArcSwap` so the sender task and tracing layer
//! see swaps atomically without locking.
//!
//! # Schema
//!
//! See `config/discord.toml.example` for the annotated user-facing version.
//! In code, the parsed shape is:
//!
//! ```toml
//! [discord]
//! enabled = true
//! username = "Cimmeria"
//! avatar_url = ""
//!
//! [discord.channels.lifecycle]
//! url = "${DISCORD_LIFECYCLE_WEBHOOK}"
//! rate_limit_per_min = 30
//!
//! [discord.events]
//! server_startup = true
//! # ... 38 more flags
//! ```
//!
//! # Live reload semantics
//!
//! `ConfigWatcher` spawns a tokio task that owns a `notify::RecommendedWatcher`.
//! When the file changes, parse + validate are attempted. On success the
//! `ArcSwap` is updated; on failure the previous config stays in place and
//! the error is logged at `warn!` (the new config doesn't break the running
//! server — operator gets a chance to fix it without restart).
//!
//! `ConfigWatcher::handle()` returns a cheap `Config` snapshot any time.
//! Both sender + tracing layer hold this handle.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use arc_swap::ArcSwap;
use notify::Watcher;
use serde::Deserialize;
use thiserror::Error;

use crate::event::{ChannelKind, EventKind};

// ── Public API ──────────────────────────────────────────────────────────

/// Parsed, validated, env-interpolated Discord config.
///
/// `Config` is the type the sender and layer read on every event. Construct
/// via [`Config::from_toml_str`] (for tests/programmatic) or
/// [`ConfigWatcher::new`] (for files with live reload).
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

/// Per-event-kind toggle map. Stored as a dense struct rather than a
/// `HashMap<EventKind, bool>` so adding a variant to `EventKind` triggers
/// a compile error here, forcing the author to choose a default.
#[derive(Debug, Clone, PartialEq)]
pub struct EventToggles {
    pub server_startup: bool,
    pub server_shutdown: bool,
    pub server_panic: bool,
    pub player_login: bool,
    pub player_logout: bool,
    pub player_disconnect: bool,
    pub player_auth_failed: bool,
    pub player_world_entry: bool,
    pub player_world_exit: bool,
    pub chat_global: bool,
    pub chat_say: bool,
    pub chat_whisper: bool,
    pub chat_guild: bool,
    pub chat_team: bool,
    pub chat_command: bool,
    pub player_level_up: bool,
    pub player_death: bool,
    pub player_respawn: bool,
    pub mission_accepted: bool,
    pub mission_completed: bool,
    pub mission_failed: bool,
    pub mission_reward_granted: bool,
    pub loot_generated: bool,
    pub item_used: bool,
    pub gm_command: bool,
    pub gm_teleport: bool,
    pub gm_spawn: bool,
    pub gm_item_grant: bool,
    pub warning: bool,
    pub error: bool,
    pub wire_format_error: bool,
    pub db_error: bool,
    pub assertion_failure: bool,
    pub mercury_timeout: bool,
    pub high_latency: bool,
    pub packet_loss_spike: bool,
    pub memory_warning: bool,
    pub tick_stall: bool,
    pub aoi_burst_warning: bool,
    pub outbox_lag: bool,
}

impl Default for EventToggles {
    /// Defaults: every high-signal event ON; potentially-noisy events
    /// OFF; whisper OFF (even the channel itself — needs explicit opt-in).
    fn default() -> Self {
        Self {
            // Lifecycle (on)
            server_startup: true,
            server_shutdown: true,
            server_panic: true,

            // Auth (on)
            player_login: true,
            player_logout: true,
            player_disconnect: true,
            player_auth_failed: true,

            // World (on)
            player_world_entry: true,
            player_world_exit: true,

            // Chat: only global on by default. Say/guild/team/cmd off
            // because they're either high-volume or repetitive. Whisper
            // off because privacy (even with content hidden, the
            // _existence_ of a whisper is a signal we shouldn't post
            // unless deliberately enabled).
            chat_global: true,
            chat_say: false,
            chat_whisper: false,
            chat_guild: false,
            chat_team: false,
            chat_command: false,

            // Gameplay: signal events on, noise off.
            player_level_up: true,
            player_death: false,
            player_respawn: false,
            mission_accepted: true,
            mission_completed: true,
            mission_failed: false,
            mission_reward_granted: false,
            loot_generated: false,
            item_used: false,

            // GM: all on (privileged actions need visibility).
            gm_command: true,
            gm_teleport: true,
            gm_spawn: true,
            gm_item_grant: true,

            // Errors: error on, warn off (warn is noisier; explicit
            // opt-in keeps the channel useful). Wire/DB/assert/timeout
            // are all on because they're rare + actionable.
            warning: false,
            error: true,
            wire_format_error: true,
            db_error: true,
            assertion_failure: true,
            mercury_timeout: true,

            // Ops: all on (alerts).
            high_latency: true,
            packet_loss_spike: true,
            memory_warning: true,
            tick_stall: true,
            aoi_burst_warning: true,
            outbox_lag: true,
        }
    }
}

impl EventToggles {
    /// Is this event-kind enabled?
    pub const fn is_enabled(&self, kind: EventKind) -> bool {
        match kind {
            EventKind::ServerStartup => self.server_startup,
            EventKind::ServerShutdown => self.server_shutdown,
            EventKind::ServerPanic => self.server_panic,
            EventKind::PlayerLogin => self.player_login,
            EventKind::PlayerLogout => self.player_logout,
            EventKind::PlayerDisconnect => self.player_disconnect,
            EventKind::PlayerAuthFailed => self.player_auth_failed,
            EventKind::PlayerWorldEntry => self.player_world_entry,
            EventKind::PlayerWorldExit => self.player_world_exit,
            EventKind::ChatGlobal => self.chat_global,
            EventKind::ChatSay => self.chat_say,
            EventKind::ChatWhisper => self.chat_whisper,
            EventKind::ChatGuild => self.chat_guild,
            EventKind::ChatTeam => self.chat_team,
            EventKind::ChatCommand => self.chat_command,
            EventKind::PlayerLevelUp => self.player_level_up,
            EventKind::PlayerDeath => self.player_death,
            EventKind::PlayerRespawn => self.player_respawn,
            EventKind::MissionAccepted => self.mission_accepted,
            EventKind::MissionCompleted => self.mission_completed,
            EventKind::MissionFailed => self.mission_failed,
            EventKind::MissionRewardGranted => self.mission_reward_granted,
            EventKind::LootGenerated => self.loot_generated,
            EventKind::ItemUsed => self.item_used,
            EventKind::GmCommand => self.gm_command,
            EventKind::GmTeleport => self.gm_teleport,
            EventKind::GmSpawn => self.gm_spawn,
            EventKind::GmItemGrant => self.gm_item_grant,
            EventKind::Warning => self.warning,
            EventKind::Error => self.error,
            EventKind::WireFormatError => self.wire_format_error,
            EventKind::DbError => self.db_error,
            EventKind::AssertionFailure => self.assertion_failure,
            EventKind::MercuryTimeout => self.mercury_timeout,
            EventKind::HighLatency => self.high_latency,
            EventKind::PacketLossSpike => self.packet_loss_spike,
            EventKind::MemoryWarning => self.memory_warning,
            EventKind::TickStall => self.tick_stall,
            EventKind::AoiBurstWarning => self.aoi_burst_warning,
            EventKind::OutboxLag => self.outbox_lag,
        }
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
    /// for tests; production paths go through [`ConfigWatcher`].
    pub fn from_toml_str(s: &str) -> Result<Self, ConfigError> {
        let raw: RawConfig = toml::from_str(s).map_err(ConfigError::Toml)?;
        let interpolated = raw.interpolate_env()?;
        interpolated.validate()
    }
}

// ── TOML schema (intermediate, pre-interpolation) ───────────────────────

#[derive(Debug, Deserialize)]
struct RawConfig {
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
    fn interpolate_env(mut self) -> Result<Self, ConfigError> {
        for (name, ch) in self.discord.channels.iter_mut() {
            ch.url = interpolate_env_vars(&ch.url, name)?;
        }
        Ok(self)
    }

    fn validate(self) -> Result<Config, ConfigError> {
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

// ── ConfigWatcher: live reload ──────────────────────────────────────────

/// Holds the live config and reloads it when the file changes.
///
/// Drop the watcher to stop the reload task. The atomic snapshot returned
/// by [`ConfigWatcher::load`] is safe to clone freely.
pub struct ConfigWatcher {
    config: Arc<ArcSwap<Config>>,
    _watcher: notify::RecommendedWatcher,
    _reload_task: tokio::task::JoinHandle<()>,
    /// Send-side of the manual-reload channel. Calling [`reload`] pushes
    /// a unit message that the task drains to re-read the file. Intended
    /// for an admin-api endpoint that forces a reload even when the
    /// file mtime didn't change — endpoint wiring lives outside this
    /// crate.
    ///
    /// [`reload`]: ConfigWatcher::reload
    reload_tx: tokio::sync::mpsc::Sender<()>,
    path: PathBuf,
}

impl ConfigWatcher {
    /// Construct a watcher around a TOML file. Reads the file once at
    /// startup; if that read or parse fails, returns the error (caller
    /// can decide to fall back to `Config::disabled()`). Once running,
    /// subsequent reload failures keep the previous config in place and
    /// log at `warn!`.
    pub fn new(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref().to_path_buf();

        // Initial load — fail loudly if the operator's TOML is broken
        // on startup. This is intentional: silently disabling Discord
        // because the TOML had a typo would defeat the purpose.
        let initial = read_and_parse(&path)?;
        let config = Arc::new(ArcSwap::new(Arc::new(initial)));

        // notify::Watcher needs a callback that runs on its own thread.
        // We funnel events through a tokio mpsc so the reload task can
        // debounce + retry on the tokio runtime.
        let (fs_tx, mut fs_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut watcher = notify::RecommendedWatcher::new(
            move |res: notify::Result<notify::Event>| {
                // Best-effort send — if the channel is closed (watcher
                // dropped) we silently discard.
                let _ = fs_tx.send(res);
            },
            notify::Config::default(),
        )
        .map_err(ConfigError::Watcher)?;
        notify::Watcher::watch(&mut watcher, &path, notify::RecursiveMode::NonRecursive)
            .map_err(ConfigError::Watcher)?;

        // Manual reload channel — `ConfigWatcher::reload()` pushes here
        // to force a re-read even when the file mtime hasn't changed
        // (e.g. an admin-api endpoint calls it on operator demand).
        let (reload_tx, mut reload_rx) = tokio::sync::mpsc::channel::<()>(4);

        let config_for_task = config.clone();
        let path_for_task = path.clone();
        let reload_task = tokio::spawn(async move {
            // Debounce file events. Editors often write in two stages
            // (truncate, then write content) — without debouncing we'd
            // parse an empty file in between.
            const DEBOUNCE: Duration = Duration::from_millis(150);

            loop {
                tokio::select! {
                    Some(_) = fs_rx.recv() => {
                        // Drain any further events that arrived during
                        // the debounce window so we don't reload twice
                        // back to back.
                        tokio::time::sleep(DEBOUNCE).await;
                        while fs_rx.try_recv().is_ok() {}
                        reload_from_disk(&path_for_task, &config_for_task);
                    }
                    Some(_) = reload_rx.recv() => {
                        reload_from_disk(&path_for_task, &config_for_task);
                    }
                    else => break,
                }
            }
        });

        Ok(Self {
            config,
            _watcher: watcher,
            _reload_task: reload_task,
            reload_tx,
            path,
        })
    }

    /// Construct from an in-memory config — used by tests and by the
    /// server bin when no config file is provided (in which case Discord
    /// is just disabled).
    pub fn from_static(config: Config) -> Self {
        let (reload_tx, _) = tokio::sync::mpsc::channel(1);
        Self {
            config: Arc::new(ArcSwap::new(Arc::new(config))),
            // We can't construct a no-op `RecommendedWatcher`; use a
            // throw-away tempfile that we don't actually watch. Easiest
            // path: leak a watcher pointed at the current dir but
            // ignore everything. The cost is one `notify::Watcher` per
            // server instance; the gain is a uniform `ConfigWatcher`
            // type for both paths.
            //
            // Concretely: re-using the file-watching constructor would
            // require a real file. For static configs we accept the
            // dummy here.
            _watcher: notify::RecommendedWatcher::new(|_| {}, notify::Config::default())
                .expect("constructing a no-op notify watcher cannot fail"),
            _reload_task: tokio::spawn(async {}),
            reload_tx,
            path: PathBuf::new(),
        }
    }

    /// Snapshot of the current config. Calling this in a hot path is
    /// fine — `ArcSwap::load` is lock-free.
    pub fn load(&self) -> Arc<Config> {
        self.config.load_full()
    }

    /// Get the underlying `ArcSwap` so callers (sender, layer) can hold
    /// a long-lived `Arc<ArcSwap<Config>>` and `load_full()` per event.
    pub fn handle(&self) -> Arc<ArcSwap<Config>> {
        self.config.clone()
    }

    /// Force an immediate reload from disk. Returns immediately; the
    /// actual file read happens on the reload task. If the parse fails,
    /// the old config stays in place and the error is logged at `warn!`.
    pub fn reload(&self) {
        // Best-effort: if the channel is full (4 pending reloads), drop
        // the request. Excess reload pressure has no value.
        let _ = self.reload_tx.try_send(());
    }

    /// Path being watched (for diagnostics / stats endpoint).
    pub fn watched_path(&self) -> &Path {
        &self.path
    }
}

fn read_and_parse(path: &Path) -> Result<Config, ConfigError> {
    let bytes = std::fs::read_to_string(path).map_err(|error| ConfigError::Io {
        path: path.to_path_buf(),
        error,
    })?;
    Config::from_toml_str(&bytes)
}

/// Re-read `path`, parse, and atomically swap. Failures log at warn
/// level and leave the previous config in place.
fn reload_from_disk(path: &Path, target: &Arc<ArcSwap<Config>>) {
    match read_and_parse(path) {
        Ok(new) => {
            // Equality check skips the swap when the file changed but
            // the meaningful content didn't — common when editors touch
            // timestamps without modifying bytes.
            let current = target.load();
            if **current == new {
                tracing::debug!(target: "cimmeria_discord", path = %path.display(), "Discord config reload: no semantic change");
                return;
            }
            tracing::info!(target: "cimmeria_discord", path = %path.display(), "Discord config reloaded");
            target.store(Arc::new(new));
        }
        Err(e) => {
            tracing::warn!(
                target: "cimmeria_discord",
                path = %path.display(),
                error = %e,
                "Discord config reload failed — keeping previous config"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn default_toggles_match_documented_defaults() {
        let t = EventToggles::default();
        assert!(t.server_startup);
        assert!(t.player_login);
        assert!(t.gm_command);
        assert!(t.error);
        // Off-by-default
        assert!(!t.warning);
        assert!(!t.chat_whisper);
        assert!(!t.chat_say);
        assert!(!t.player_death);
        assert!(!t.loot_generated);
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
