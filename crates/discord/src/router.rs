//! Event → ChannelKind lookup. Pure function, but pinned by an exhaustive
//! `match` (no `_ => …` arm) so adding an `EventKind` variant without
//! also assigning it a channel is a compile-time error.

use crate::event::{ChannelKind, EventKind};

/// Determine which Discord channel an event belongs to.
///
/// **No `_ => …` fallback** — every variant must explicitly choose its
/// channel. Adding an `EventKind` variant without routing it is a compile
/// error. This is the single source of truth used by `Config::is_enabled`
/// to find the channel webhook for a given event.
pub const fn channel_for(kind: EventKind) -> ChannelKind {
    match kind {
        // Lifecycle channel
        EventKind::ServerStartup | EventKind::ServerShutdown | EventKind::ServerPanic => {
            ChannelKind::Lifecycle
        }

        // Auth channel
        EventKind::PlayerLogin
        | EventKind::PlayerLogout
        | EventKind::PlayerDisconnect
        | EventKind::PlayerAuthFailed => ChannelKind::Auth,

        // World channel
        EventKind::PlayerWorldEntry | EventKind::PlayerWorldExit => ChannelKind::World,

        // Chat channel
        EventKind::ChatGlobal
        | EventKind::ChatSay
        | EventKind::ChatWhisper
        | EventKind::ChatGuild
        | EventKind::ChatTeam
        | EventKind::ChatCommand => ChannelKind::Chat,

        // Gameplay channel
        EventKind::PlayerLevelUp
        | EventKind::PlayerDeath
        | EventKind::PlayerRespawn
        | EventKind::MissionAccepted
        | EventKind::MissionCompleted
        | EventKind::MissionFailed
        | EventKind::MissionRewardGranted
        | EventKind::LootGenerated
        | EventKind::ItemUsed => ChannelKind::Gameplay,

        // GM channel
        EventKind::GmCommand
        | EventKind::GmTeleport
        | EventKind::GmSpawn
        | EventKind::GmItemGrant => ChannelKind::Gm,

        // Errors channel
        EventKind::Warning
        | EventKind::Error
        | EventKind::WireFormatError
        | EventKind::DbError
        | EventKind::AssertionFailure
        | EventKind::MercuryTimeout => ChannelKind::Errors,

        // Ops channel
        EventKind::HighLatency
        | EventKind::PacketLossSpike
        | EventKind::MemoryWarning
        | EventKind::TickStall
        | EventKind::AoiBurstWarning
        | EventKind::OutboxLag => ChannelKind::Ops,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Spot-check pin — every channel's expected event-set. A regression
    /// that moves an event between channels (e.g., chat-whisper ends up
    /// in `auth` by accident) trips this immediately.
    #[test]
    fn channel_assignments_are_stable() {
        // Lifecycle
        assert_eq!(
            channel_for(EventKind::ServerStartup),
            ChannelKind::Lifecycle
        );
        assert_eq!(channel_for(EventKind::ServerPanic), ChannelKind::Lifecycle);

        // Auth
        assert_eq!(channel_for(EventKind::PlayerLogin), ChannelKind::Auth);
        assert_eq!(channel_for(EventKind::PlayerDisconnect), ChannelKind::Auth);

        // World
        assert_eq!(channel_for(EventKind::PlayerWorldEntry), ChannelKind::World);

        // Chat — every chat subtype lives here, including whisper
        assert_eq!(channel_for(EventKind::ChatGlobal), ChannelKind::Chat);
        assert_eq!(channel_for(EventKind::ChatWhisper), ChannelKind::Chat);
        assert_eq!(channel_for(EventKind::ChatCommand), ChannelKind::Chat);

        // Gameplay
        assert_eq!(channel_for(EventKind::PlayerLevelUp), ChannelKind::Gameplay);
        assert_eq!(
            channel_for(EventKind::MissionCompleted),
            ChannelKind::Gameplay
        );
        assert_eq!(channel_for(EventKind::LootGenerated), ChannelKind::Gameplay);

        // GM
        assert_eq!(channel_for(EventKind::GmCommand), ChannelKind::Gm);
        assert_eq!(channel_for(EventKind::GmItemGrant), ChannelKind::Gm);

        // Errors — `Warning` and `Error` (the tracing-harvested ones)
        // both land here.
        assert_eq!(channel_for(EventKind::Warning), ChannelKind::Errors);
        assert_eq!(channel_for(EventKind::Error), ChannelKind::Errors);
        assert_eq!(channel_for(EventKind::DbError), ChannelKind::Errors);

        // Ops
        assert_eq!(channel_for(EventKind::HighLatency), ChannelKind::Ops);
        assert_eq!(channel_for(EventKind::TickStall), ChannelKind::Ops);
    }

    /// Sanity: every channel is the target of at least one event. A
    /// channel with no events would be configured but never receive
    /// traffic — almost certainly a routing-table mistake.
    #[test]
    fn every_channel_has_at_least_one_event() {
        let mut seen = std::collections::HashSet::new();
        for k in EventKind::ALL {
            seen.insert(channel_for(*k));
        }
        for c in ChannelKind::ALL {
            assert!(
                seen.contains(c),
                "channel {:?} has no events routed to it",
                c
            );
        }
    }

    /// Sanity inverse: every `EventKind` resolves cleanly (no panic).
    /// Trips at compile time if a variant is added without routing —
    /// but this also pins the runtime invariant for someone reading the
    /// tests cold.
    #[test]
    fn every_event_kind_routes() {
        for k in EventKind::ALL {
            let _ = channel_for(*k);
        }
    }
}
