//! Discriminant of [`Event`](super::Event) — the toggle/router key.

use serde::{Deserialize, Serialize};

// ── EventKind: 44-variant discriminant ──────────────────────────────────

/// Discriminant of [`Event`](super::Event) — used as the key for the
/// per-event toggle map in [`crate::config::EventToggles`] and as the value
/// type for the router.
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
    CharacterCreated,
    NpcDeath,
    MinigameResult,
    Dialog,

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
        Self::CharacterCreated,
        Self::NpcDeath,
        Self::MinigameResult,
        Self::Dialog,
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
            Self::CharacterCreated => "character_created",
            Self::NpcDeath => "npc_death",
            Self::MinigameResult => "minigame_result",
            Self::Dialog => "dialog",
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

    /// `EventKind::ALL` length is the canonical count of toggleable event
    /// types. Adding a variant without updating `ALL` would silently break
    /// config-default generation.
    #[test]
    fn event_kind_all_matches_variant_count() {
        // Update this when adding/removing an EventKind variant. The
        // doc-comment on EventKind enumerates the three places to touch
        // (event.rs, EventToggles, router).
        assert_eq!(EventKind::ALL.len(), 44);
    }
}
