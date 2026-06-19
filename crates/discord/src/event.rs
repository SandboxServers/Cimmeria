//! Typed event taxonomy. Every server-side notification is one of these
//! variants — the type system makes it impossible to construct an event
//! with the wrong payload shape, and `EventKind` provides a discriminant
//! that lines up with the per-event toggles in [`crate::config`].
//!
//! Events route to one of 8 channels (`ChannelKind`). The mapping lives in
//! [`crate::router`] and is exhaustively tested.

use std::net::SocketAddr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ── ChannelKind: logical destination ────────────────────────────────────

/// Logical channel a category of events posts to. Each variant maps 1:1 to
/// a webhook URL in `Config::channels`. Eight channels keeps the signal-to-
/// noise tiers separable while staying inside Discord's per-webhook rate
/// limit (5 msg / 2 s) for each.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelKind {
    /// Server up/down/panic — low volume, high signal.
    Lifecycle,
    /// Login, logout, auth fail, unexpected disconnect.
    Auth,
    /// World entry / exit.
    World,
    /// Player chat (global/say/whisper/guild/team) — high volume.
    Chat,
    /// Level up, death, respawn, mission events, loot.
    Gameplay,
    /// GM-issued admin commands (teleport, spawn, item grant, ...).
    Gm,
    /// `warn!` / `error!` / wire-format / DB / assertion / mercury timeout.
    Errors,
    /// Latency, packet loss, memory, tick stall, AoI burst, outbox lag.
    Ops,
}

impl ChannelKind {
    /// All eight channels in a stable order. Used by config validation +
    /// stats reporting.
    pub const ALL: &'static [Self] = &[
        Self::Lifecycle,
        Self::Auth,
        Self::World,
        Self::Chat,
        Self::Gameplay,
        Self::Gm,
        Self::Errors,
        Self::Ops,
    ];

    /// Lowercase short name, matches the TOML key in `[discord.channels]`.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Lifecycle => "lifecycle",
            Self::Auth => "auth",
            Self::World => "world",
            Self::Chat => "chat",
            Self::Gameplay => "gameplay",
            Self::Gm => "gm",
            Self::Errors => "errors",
            Self::Ops => "ops",
        }
    }
}

// ── EventKind: 28-variant discriminant ──────────────────────────────────

/// Discriminant of [`Event`] — used as the key for the per-event toggle map
/// in [`crate::config::EventToggles`] and as the value type for the router.
///
/// **Adding a variant requires touching three places**: this enum, the
/// matching `Event` variant, and `EventToggles` field. The router test
/// `every_event_kind_has_a_channel` panics on a `match` `_` arm so a
/// missing route fails at compile time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    // Lifecycle
    ServerStartup,
    ServerShutdown,
    ServerPanic,

    // Auth
    PlayerLogin,
    PlayerLogout,
    PlayerDisconnect,
    PlayerAuthFailed,

    // World
    PlayerWorldEntry,
    PlayerWorldExit,

    // Chat
    ChatGlobal,
    ChatSay,
    ChatWhisper,
    ChatGuild,
    ChatTeam,
    ChatCommand,

    // Gameplay
    PlayerLevelUp,
    PlayerDeath,
    PlayerRespawn,
    MissionAccepted,
    MissionCompleted,
    MissionFailed,
    MissionRewardGranted,
    LootGenerated,
    ItemUsed,

    // GM
    GmCommand,
    GmTeleport,
    GmSpawn,
    GmItemGrant,

    // Errors
    Warning,
    Error,
    WireFormatError,
    DbError,
    AssertionFailure,
    MercuryTimeout,

    // Ops
    HighLatency,
    PacketLossSpike,
    MemoryWarning,
    TickStall,
    AoiBurstWarning,
    OutboxLag,
}

impl EventKind {
    /// All variants in declaration order — used by config validation to
    /// ensure every variant has a toggle default, and by docs generation.
    pub const ALL: &'static [Self] = &[
        Self::ServerStartup,
        Self::ServerShutdown,
        Self::ServerPanic,
        Self::PlayerLogin,
        Self::PlayerLogout,
        Self::PlayerDisconnect,
        Self::PlayerAuthFailed,
        Self::PlayerWorldEntry,
        Self::PlayerWorldExit,
        Self::ChatGlobal,
        Self::ChatSay,
        Self::ChatWhisper,
        Self::ChatGuild,
        Self::ChatTeam,
        Self::ChatCommand,
        Self::PlayerLevelUp,
        Self::PlayerDeath,
        Self::PlayerRespawn,
        Self::MissionAccepted,
        Self::MissionCompleted,
        Self::MissionFailed,
        Self::MissionRewardGranted,
        Self::LootGenerated,
        Self::ItemUsed,
        Self::GmCommand,
        Self::GmTeleport,
        Self::GmSpawn,
        Self::GmItemGrant,
        Self::Warning,
        Self::Error,
        Self::WireFormatError,
        Self::DbError,
        Self::AssertionFailure,
        Self::MercuryTimeout,
        Self::HighLatency,
        Self::PacketLossSpike,
        Self::MemoryWarning,
        Self::TickStall,
        Self::AoiBurstWarning,
        Self::OutboxLag,
    ];

    /// Lowercase snake_case name. Matches the TOML key in `[discord.events]`.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ServerStartup => "server_startup",
            Self::ServerShutdown => "server_shutdown",
            Self::ServerPanic => "server_panic",
            Self::PlayerLogin => "player_login",
            Self::PlayerLogout => "player_logout",
            Self::PlayerDisconnect => "player_disconnect",
            Self::PlayerAuthFailed => "player_auth_failed",
            Self::PlayerWorldEntry => "player_world_entry",
            Self::PlayerWorldExit => "player_world_exit",
            Self::ChatGlobal => "chat_global",
            Self::ChatSay => "chat_say",
            Self::ChatWhisper => "chat_whisper",
            Self::ChatGuild => "chat_guild",
            Self::ChatTeam => "chat_team",
            Self::ChatCommand => "chat_command",
            Self::PlayerLevelUp => "player_level_up",
            Self::PlayerDeath => "player_death",
            Self::PlayerRespawn => "player_respawn",
            Self::MissionAccepted => "mission_accepted",
            Self::MissionCompleted => "mission_completed",
            Self::MissionFailed => "mission_failed",
            Self::MissionRewardGranted => "mission_reward_granted",
            Self::LootGenerated => "loot_generated",
            Self::ItemUsed => "item_used",
            Self::GmCommand => "gm_command",
            Self::GmTeleport => "gm_teleport",
            Self::GmSpawn => "gm_spawn",
            Self::GmItemGrant => "gm_item_grant",
            Self::Warning => "warning",
            Self::Error => "error",
            Self::WireFormatError => "wire_format_error",
            Self::DbError => "db_error",
            Self::AssertionFailure => "assertion_failure",
            Self::MercuryTimeout => "mercury_timeout",
            Self::HighLatency => "high_latency",
            Self::PacketLossSpike => "packet_loss_spike",
            Self::MemoryWarning => "memory_warning",
            Self::TickStall => "tick_stall",
            Self::AoiBurstWarning => "aoi_burst_warning",
            Self::OutboxLag => "outbox_lag",
        }
    }
}

// ── Severity: drives embed color ────────────────────────────────────────

/// Severity tier — drives embed color in [`crate::embed`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Normal operation (login, world entry, mission complete).
    Info,
    /// Positive event (level up, server startup).
    Good,
    /// Concerning but recoverable (warn, latency).
    Warn,
    /// Bad — likely needs human investigation (error, panic, disconnect).
    Error,
    /// Privileged action (GM commands).
    Privileged,
}

// ── Event: the rich payload ─────────────────────────────────────────────

/// A single Discord-bound event. The variant uniquely determines an
/// [`EventKind`], a [`ChannelKind`], and a [`Severity`].
///
/// **Field types are deliberately small and well-defined.** No `dyn Any` /
/// `serde_json::Value` payloads — every field is typed so the embed builder
/// can format consistently without losing structure to stringification.
#[derive(Debug, Clone)]
pub enum Event {
    // ─── Lifecycle ──────────────────────────────────────────────────────
    ServerStartup {
        version: String,
        bind_addrs: Vec<String>,
        timestamp: DateTime<Utc>,
    },
    ServerShutdown {
        reason: String,
        uptime_secs: u64,
        timestamp: DateTime<Utc>,
    },
    ServerPanic {
        location: String,
        message: String,
        timestamp: DateTime<Utc>,
    },

    // ─── Auth ───────────────────────────────────────────────────────────
    PlayerLogin {
        account_id: u32,
        character_name: Option<String>,
        addr: SocketAddr,
        timestamp: DateTime<Utc>,
    },
    PlayerLogout {
        account_id: u32,
        character_name: Option<String>,
        session_secs: u64,
        timestamp: DateTime<Utc>,
    },
    PlayerDisconnect {
        account_id: Option<u32>,
        character_name: Option<String>,
        addr: SocketAddr,
        reason: DisconnectReason,
        session_secs: u64,
        timestamp: DateTime<Utc>,
    },
    PlayerAuthFailed {
        account_name: String,
        addr: SocketAddr,
        reason: String,
        timestamp: DateTime<Utc>,
    },

    // ─── World ──────────────────────────────────────────────────────────
    PlayerWorldEntry {
        account_id: u32,
        character_name: String,
        world_name: String,
        position: [f32; 3],
        timestamp: DateTime<Utc>,
    },
    PlayerWorldExit {
        account_id: u32,
        character_name: String,
        from_world: String,
        to_world: Option<String>,
        timestamp: DateTime<Utc>,
    },

    // ─── Chat ───────────────────────────────────────────────────────────
    /// `ChatGlobal` / `ChatSay` / `ChatGuild` / `ChatTeam` / `ChatCommand`
    /// share a payload shape. `ChatWhisper` is the privacy-sensitive one
    /// where `content` is replaced with `[hidden]` in the embed regardless
    /// of how the channel is toggled.
    Chat {
        kind: ChatKind,
        speaker: String,
        recipient: Option<String>,
        content: String,
        timestamp: DateTime<Utc>,
    },

    // ─── Gameplay ───────────────────────────────────────────────────────
    PlayerLevelUp {
        character_name: String,
        new_level: u32,
        timestamp: DateTime<Utc>,
    },
    PlayerDeath {
        character_name: String,
        killer: Option<String>,
        cause: String,
        timestamp: DateTime<Utc>,
    },
    PlayerRespawn {
        character_name: String,
        world_name: String,
        timestamp: DateTime<Utc>,
    },
    MissionAccepted {
        character_name: String,
        mission_id: i32,
        mission_name: Option<String>,
        timestamp: DateTime<Utc>,
    },
    MissionCompleted {
        character_name: String,
        mission_id: i32,
        mission_name: Option<String>,
        timestamp: DateTime<Utc>,
    },
    MissionFailed {
        character_name: String,
        mission_id: i32,
        mission_name: Option<String>,
        reason: String,
        timestamp: DateTime<Utc>,
    },
    MissionRewardGranted {
        character_name: String,
        mission_id: i32,
        xp: u64,
        cash: i64,
        items: Vec<i32>,
        timestamp: DateTime<Utc>,
    },
    LootGenerated {
        character_name: String,
        source: String,
        items: Vec<i32>,
        timestamp: DateTime<Utc>,
    },
    ItemUsed {
        character_name: String,
        item_type_id: i32,
        target: Option<String>,
        timestamp: DateTime<Utc>,
    },

    // ─── GM ─────────────────────────────────────────────────────────────
    GmCommand {
        gm_name: String,
        command: String,
        args: String,
        timestamp: DateTime<Utc>,
    },
    GmTeleport {
        gm_name: String,
        target: String,
        world_name: String,
        position: [f32; 3],
        timestamp: DateTime<Utc>,
    },
    GmSpawn {
        gm_name: String,
        template_id: i32,
        template_name: Option<String>,
        position: [f32; 3],
        timestamp: DateTime<Utc>,
    },
    GmItemGrant {
        gm_name: String,
        recipient: String,
        item_type_id: i32,
        quantity: i32,
        timestamp: DateTime<Utc>,
    },

    // ─── Errors ─────────────────────────────────────────────────────────
    /// `Warning` / `Error` — harvested from `tracing::warn!`/`error!`.
    /// `target` is the tracing target (module path), `fields` is the
    /// structured-field dump (`reason=`, `entity_id=`, etc.).
    TracingEvent {
        kind: TracingEventKind,
        target: String,
        message: String,
        fields: Vec<(String, String)>,
        timestamp: DateTime<Utc>,
    },
    WireFormatError {
        kind: String,
        addr: Option<SocketAddr>,
        details: String,
        timestamp: DateTime<Utc>,
    },
    DbError {
        operation: String,
        details: String,
        timestamp: DateTime<Utc>,
    },
    AssertionFailure {
        location: String,
        message: String,
        timestamp: DateTime<Utc>,
    },
    MercuryTimeout {
        addr: SocketAddr,
        account_id: Option<u32>,
        silence_secs: u64,
        timestamp: DateTime<Utc>,
    },

    // ─── Ops ────────────────────────────────────────────────────────────
    HighLatency {
        addr: SocketAddr,
        rtt_ms: u32,
        threshold_ms: u32,
        timestamp: DateTime<Utc>,
    },
    PacketLossSpike {
        loss_ratio: f32,
        window_secs: u32,
        timestamp: DateTime<Utc>,
    },
    MemoryWarning {
        rss_mb: u64,
        threshold_mb: u64,
        timestamp: DateTime<Utc>,
    },
    TickStall {
        tick_ms: u32,
        budget_ms: u32,
        subsystem: String,
        timestamp: DateTime<Utc>,
    },
    AoiBurstWarning {
        witness_id: u32,
        burst_size: u32,
        threshold: u32,
        timestamp: DateTime<Utc>,
    },
    OutboxLag {
        depth: u32,
        threshold: u32,
        timestamp: DateTime<Utc>,
    },
}

/// Why a player connection closed. `PlayerDisconnect` carries this.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DisconnectReason {
    /// Client sent `disconnectClient (0x0C)` — graceful.
    ///
    /// Note: in this case `PlayerLogout` fires too; `PlayerDisconnect`
    /// only fires on the lower-level connection teardown.
    Clean,
    /// Inactivity timeout — no traffic past `MERCURY_PEER_DEAD_MS`.
    Timeout,
    /// Client process likely crashed (received `WSAECONNRESET` / ICMP
    /// unreachable from the peer's OS).
    PeerReset,
    /// Server-side fault closed the connection.
    ServerInitiated,
}

impl DisconnectReason {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Clean => "clean",
            Self::Timeout => "timeout",
            Self::PeerReset => "peer_reset",
            Self::ServerInitiated => "server_initiated",
        }
    }

    /// Map the stable internal disconnect label that
    /// `base::helpers::destroy_client_entities` stamps on every teardown
    /// (`"client_disconnect"`, `"logoff"`, `"inactivity_timeout"`,
    /// `"send_error"`, `"duplicate_login"`) onto a typed reason for the
    /// embed. Unknown labels collapse to [`Self::ServerInitiated`] — the
    /// conservative "the server closed this" bucket — so a new label added
    /// upstream degrades to a sane default rather than failing to compile
    /// at a distance.
    pub fn from_label(label: &str) -> Self {
        match label {
            "client_disconnect" | "logoff" => Self::Clean,
            "inactivity_timeout" => Self::Timeout,
            "send_error" => Self::PeerReset,
            _ => Self::ServerInitiated,
        }
    }
}

/// Chat channel sub-kind. Mapped to its own `EventKind` for routing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatKind {
    Global,
    Say,
    Whisper,
    Guild,
    Team,
    Command,
}

impl ChatKind {
    pub const fn event_kind(self) -> EventKind {
        match self {
            Self::Global => EventKind::ChatGlobal,
            Self::Say => EventKind::ChatSay,
            Self::Whisper => EventKind::ChatWhisper,
            Self::Guild => EventKind::ChatGuild,
            Self::Team => EventKind::ChatTeam,
            Self::Command => EventKind::ChatCommand,
        }
    }
}

/// Tracing-level signal kind. `Warning` / `Error` come from `warn!`/`error!`
/// emit; the layer maps the tracing `Level` to this discriminant before
/// constructing the `Event::TracingEvent`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TracingEventKind {
    Warn,
    Error,
}

impl TracingEventKind {
    pub const fn event_kind(self) -> EventKind {
        match self {
            Self::Warn => EventKind::Warning,
            Self::Error => EventKind::Error,
        }
    }
}

impl Event {
    /// Discriminant for routing + toggle lookup.
    pub fn kind(&self) -> EventKind {
        match self {
            Self::ServerStartup { .. } => EventKind::ServerStartup,
            Self::ServerShutdown { .. } => EventKind::ServerShutdown,
            Self::ServerPanic { .. } => EventKind::ServerPanic,
            Self::PlayerLogin { .. } => EventKind::PlayerLogin,
            Self::PlayerLogout { .. } => EventKind::PlayerLogout,
            Self::PlayerDisconnect { .. } => EventKind::PlayerDisconnect,
            Self::PlayerAuthFailed { .. } => EventKind::PlayerAuthFailed,
            Self::PlayerWorldEntry { .. } => EventKind::PlayerWorldEntry,
            Self::PlayerWorldExit { .. } => EventKind::PlayerWorldExit,
            Self::Chat { kind, .. } => kind.event_kind(),
            Self::PlayerLevelUp { .. } => EventKind::PlayerLevelUp,
            Self::PlayerDeath { .. } => EventKind::PlayerDeath,
            Self::PlayerRespawn { .. } => EventKind::PlayerRespawn,
            Self::MissionAccepted { .. } => EventKind::MissionAccepted,
            Self::MissionCompleted { .. } => EventKind::MissionCompleted,
            Self::MissionFailed { .. } => EventKind::MissionFailed,
            Self::MissionRewardGranted { .. } => EventKind::MissionRewardGranted,
            Self::LootGenerated { .. } => EventKind::LootGenerated,
            Self::ItemUsed { .. } => EventKind::ItemUsed,
            Self::GmCommand { .. } => EventKind::GmCommand,
            Self::GmTeleport { .. } => EventKind::GmTeleport,
            Self::GmSpawn { .. } => EventKind::GmSpawn,
            Self::GmItemGrant { .. } => EventKind::GmItemGrant,
            Self::TracingEvent { kind, .. } => kind.event_kind(),
            Self::WireFormatError { .. } => EventKind::WireFormatError,
            Self::DbError { .. } => EventKind::DbError,
            Self::AssertionFailure { .. } => EventKind::AssertionFailure,
            Self::MercuryTimeout { .. } => EventKind::MercuryTimeout,
            Self::HighLatency { .. } => EventKind::HighLatency,
            Self::PacketLossSpike { .. } => EventKind::PacketLossSpike,
            Self::MemoryWarning { .. } => EventKind::MemoryWarning,
            Self::TickStall { .. } => EventKind::TickStall,
            Self::AoiBurstWarning { .. } => EventKind::AoiBurstWarning,
            Self::OutboxLag { .. } => EventKind::OutboxLag,
        }
    }

    /// Severity drives embed color. Derived from variant — never
    /// configurable, since this is what makes the embed legible at a
    /// glance.
    pub fn severity(&self) -> Severity {
        match self {
            Self::ServerStartup { .. } | Self::PlayerLevelUp { .. } => Severity::Good,

            Self::ServerShutdown { .. }
            | Self::PlayerLogin { .. }
            | Self::PlayerLogout { .. }
            | Self::PlayerWorldEntry { .. }
            | Self::PlayerWorldExit { .. }
            | Self::Chat { .. }
            | Self::PlayerRespawn { .. }
            | Self::MissionAccepted { .. }
            | Self::MissionCompleted { .. }
            | Self::MissionRewardGranted { .. }
            | Self::ItemUsed { .. }
            | Self::LootGenerated { .. } => Severity::Info,

            Self::PlayerDisconnect { .. }
            | Self::PlayerDeath { .. }
            | Self::MissionFailed { .. }
            | Self::HighLatency { .. }
            | Self::PacketLossSpike { .. }
            | Self::MemoryWarning { .. }
            | Self::TickStall { .. }
            | Self::AoiBurstWarning { .. }
            | Self::OutboxLag { .. }
            | Self::MercuryTimeout { .. } => Severity::Warn,

            Self::ServerPanic { .. }
            | Self::PlayerAuthFailed { .. }
            | Self::WireFormatError { .. }
            | Self::DbError { .. }
            | Self::AssertionFailure { .. } => Severity::Error,

            Self::TracingEvent { kind, .. } => match kind {
                TracingEventKind::Warn => Severity::Warn,
                TracingEventKind::Error => Severity::Error,
            },

            Self::GmCommand { .. }
            | Self::GmTeleport { .. }
            | Self::GmSpawn { .. }
            | Self::GmItemGrant { .. } => Severity::Privileged,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pin that every `EventKind` variant has a unique `as_str`. A
    /// duplicate would let two variants compete for the same TOML toggle
    /// key.
    #[test]
    fn every_event_kind_has_unique_snake_case_name() {
        let mut seen = std::collections::HashSet::new();
        for k in EventKind::ALL {
            assert!(
                seen.insert(k.as_str()),
                "duplicate snake-case name: {}",
                k.as_str()
            );
        }
        assert_eq!(seen.len(), EventKind::ALL.len(), "ALL has duplicates");
    }

    /// Pin that every `ChannelKind` variant has a unique `as_str`.
    #[test]
    fn every_channel_kind_has_unique_snake_case_name() {
        let mut seen = std::collections::HashSet::new();
        for c in ChannelKind::ALL {
            assert!(seen.insert(c.as_str()));
        }
        assert_eq!(seen.len(), ChannelKind::ALL.len());
    }

    /// `DisconnectReason::from_label` is the single mapping the auth-channel
    /// `player_disconnect` seam relies on. Pin every label the
    /// `destroy_client_entities` choke point stamps, plus the unknown-label
    /// fallback. Reverting the mapping (or renaming a label upstream without
    /// updating it) trips this.
    #[test]
    fn disconnect_reason_from_label_maps_every_teardown_label() {
        assert_eq!(
            DisconnectReason::from_label("client_disconnect"),
            DisconnectReason::Clean
        );
        assert_eq!(
            DisconnectReason::from_label("logoff"),
            DisconnectReason::Clean
        );
        assert_eq!(
            DisconnectReason::from_label("inactivity_timeout"),
            DisconnectReason::Timeout
        );
        assert_eq!(
            DisconnectReason::from_label("send_error"),
            DisconnectReason::PeerReset
        );
        assert_eq!(
            DisconnectReason::from_label("duplicate_login"),
            DisconnectReason::ServerInitiated
        );
        // Unknown label degrades to the conservative server-initiated bucket.
        assert_eq!(
            DisconnectReason::from_label("something_new"),
            DisconnectReason::ServerInitiated
        );
    }

    /// `EventKind::ALL` length is the canonical count of toggleable event
    /// types. Adding a variant without updating `ALL` would silently break
    /// config-default generation.
    #[test]
    fn event_kind_all_matches_variant_count() {
        // Update this when adding/removing an EventKind variant. The
        // doc-comment on EventKind enumerates the three places to touch
        // (event.rs, EventToggles, router).
        assert_eq!(EventKind::ALL.len(), 40);
    }
}
