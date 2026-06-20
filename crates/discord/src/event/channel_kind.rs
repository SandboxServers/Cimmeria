//! Logical destination channel for a category of events.

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

#[cfg(test)]
mod tests {
    use super::*;

    /// Pin that every `ChannelKind` variant has a unique `as_str`.
    #[test]
    fn every_channel_kind_has_unique_snake_case_name() {
        let mut seen = std::collections::HashSet::new();
        for c in ChannelKind::ALL {
            assert!(seen.insert(c.as_str()));
        }
        assert_eq!(seen.len(), ChannelKind::ALL.len());
    }
}
