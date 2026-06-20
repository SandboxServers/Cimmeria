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

    // -- NET-NEW coverage: is_enabled mapping + default branches --

    /// `is_enabled` must round-trip every `EventKind` against the matching
    /// struct field. We mutate one field at a time from a known baseline
    /// and assert `is_enabled(kind)` tracks it -- this walks every arm of
    /// the big `match` (previously only a handful were touched indirectly).
    #[test]
    fn is_enabled_tracks_every_field() {
        // Start from a default and flip each field both ways, asserting
        // `is_enabled(kind)` follows the field. The on-then-off pair makes
        // the result independent of each field's default value and proves
        // every `match` arm is bound to the field it names.
        let mut t = EventToggles::default();
        macro_rules! check {
            ($field:ident, $kind:expr) => {{
                t.$field = true;
                assert!(
                    t.is_enabled($kind),
                    concat!("is_enabled must read ", stringify!($field), " when true")
                );
                t.$field = false;
                assert!(
                    !t.is_enabled($kind),
                    concat!("is_enabled must read ", stringify!($field), " when false")
                );
            }};
        }

        check!(server_startup, EventKind::ServerStartup);
        check!(server_shutdown, EventKind::ServerShutdown);
        check!(server_panic, EventKind::ServerPanic);
        check!(player_login, EventKind::PlayerLogin);
        check!(player_logout, EventKind::PlayerLogout);
        check!(player_disconnect, EventKind::PlayerDisconnect);
        check!(player_auth_failed, EventKind::PlayerAuthFailed);
        check!(player_world_entry, EventKind::PlayerWorldEntry);
        check!(player_world_exit, EventKind::PlayerWorldExit);
        check!(chat_global, EventKind::ChatGlobal);
        check!(chat_say, EventKind::ChatSay);
        check!(chat_whisper, EventKind::ChatWhisper);
        check!(chat_guild, EventKind::ChatGuild);
        check!(chat_team, EventKind::ChatTeam);
        check!(chat_command, EventKind::ChatCommand);
        check!(player_level_up, EventKind::PlayerLevelUp);
        check!(player_death, EventKind::PlayerDeath);
        check!(player_respawn, EventKind::PlayerRespawn);
        check!(mission_accepted, EventKind::MissionAccepted);
        check!(mission_completed, EventKind::MissionCompleted);
        check!(mission_failed, EventKind::MissionFailed);
        check!(mission_reward_granted, EventKind::MissionRewardGranted);
        check!(loot_generated, EventKind::LootGenerated);
        check!(item_used, EventKind::ItemUsed);
        check!(character_created, EventKind::CharacterCreated);
        check!(npc_death, EventKind::NpcDeath);
        check!(minigame_result, EventKind::MinigameResult);
        check!(dialog, EventKind::Dialog);
        check!(gm_command, EventKind::GmCommand);
        check!(gm_teleport, EventKind::GmTeleport);
        check!(gm_spawn, EventKind::GmSpawn);
        check!(gm_item_grant, EventKind::GmItemGrant);
        check!(warning, EventKind::Warning);
        check!(error, EventKind::Error);
        check!(wire_format_error, EventKind::WireFormatError);
        check!(db_error, EventKind::DbError);
        check!(assertion_failure, EventKind::AssertionFailure);
        check!(mercury_timeout, EventKind::MercuryTimeout);
        check!(high_latency, EventKind::HighLatency);
        check!(packet_loss_spike, EventKind::PacketLossSpike);
        check!(memory_warning, EventKind::MemoryWarning);
        check!(tick_stall, EventKind::TickStall);
        check!(aoi_burst_warning, EventKind::AoiBurstWarning);
        check!(outbox_lag, EventKind::OutboxLag);
    }

    /// Pin the documented default-OFF gameplay/chat events that the doc
    /// comment in `Default` calls out explicitly. These complement the
    /// existing `default_toggles_match_documented_defaults` test without
    /// duplicating its assertions.
    #[test]
    fn additional_default_off_events_pinned() {
        let t = EventToggles::default();
        assert!(!t.player_respawn, "respawn is noise -> off");
        assert!(!t.mission_failed, "mission_failed off by default");
        assert!(!t.mission_reward_granted, "reward_granted off by default");
        assert!(!t.item_used, "item_used off by default");
        assert!(!t.npc_death, "npc_death high-volume -> off");
        assert!(!t.dialog, "dialog high-volume -> off");
        assert!(!t.chat_guild);
        assert!(!t.chat_team);
        assert!(!t.chat_command);
    }

    /// Pin the documented default-ON events not already asserted by the
    /// existing default test: the new gameplay events, GM events, and the
    /// ops/error alerts that ship enabled.
    #[test]
    fn additional_default_on_events_pinned() {
        let t = EventToggles::default();
        // New gameplay events.
        assert!(t.character_created, "character_created on by default");
        assert!(t.minigame_result, "minigame_result on by default");
        // World.
        assert!(t.player_world_entry);
        assert!(t.player_world_exit);
        // GM (all on -- privileged visibility).
        assert!(t.gm_teleport);
        assert!(t.gm_spawn);
        assert!(t.gm_item_grant);
        // Error/diagnostic alerts.
        assert!(t.wire_format_error);
        assert!(t.db_error);
        assert!(t.assertion_failure);
        assert!(t.mercury_timeout);
        // Ops alerts.
        assert!(t.high_latency);
        assert!(t.packet_loss_spike);
        assert!(t.memory_warning);
        assert!(t.tick_stall);
        assert!(t.aoi_burst_warning);
        assert!(t.outbox_lag);
        // Lifecycle / auth.
        assert!(t.server_shutdown);
        assert!(t.server_panic);
        assert!(t.player_logout);
        assert!(t.player_disconnect);
        assert!(t.player_auth_failed);
        assert!(t.mission_accepted);
        assert!(t.mission_completed);
        assert!(t.player_level_up);
    }
}
