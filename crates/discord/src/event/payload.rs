//! The rich [`Event`] payload, its supporting enums, and the
//! discriminant/severity derivations.

use std::net::SocketAddr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::EventKind;

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
/// [`EventKind`], a [`ChannelKind`](super::ChannelKind), and a
/// [`Severity`].
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

    /// `DisconnectReason::as_str` is the stable snake_case label each
    /// variant serialises to. Pin every variant so a rename (which would
    /// silently change a downstream log/metric label) trips here. Net-new
    /// coverage added with the event-module split — the previous suite
    /// only exercised `from_label`, leaving `as_str` unguarded.
    #[test]
    fn disconnect_reason_as_str_is_stable() {
        assert_eq!(DisconnectReason::Clean.as_str(), "clean");
        assert_eq!(DisconnectReason::Timeout.as_str(), "timeout");
        assert_eq!(DisconnectReason::PeerReset.as_str(), "peer_reset");
        assert_eq!(
            DisconnectReason::ServerInitiated.as_str(),
            "server_initiated"
        );
    }
}
