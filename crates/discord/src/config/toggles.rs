//! Per-event-kind toggle map.

use crate::event::EventKind;

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
    pub character_created: bool,
    pub npc_death: bool,
    pub minigame_result: bool,
    pub dialog: bool,
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

            // New gameplay events: character creation + minigame results are
            // low-volume / high-signal → on. NPC death + dialog are very
            // high-volume during play → off by default, toggleable.
            character_created: true,
            npc_death: false,
            minigame_result: true,
            dialog: false,

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
            EventKind::CharacterCreated => self.character_created,
            EventKind::NpcDeath => self.npc_death,
            EventKind::MinigameResult => self.minigame_result,
            EventKind::Dialog => self.dialog,
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
