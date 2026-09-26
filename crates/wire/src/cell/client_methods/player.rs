//! SGWPlayer own ClientMethods (indices 98–156).

/// Begin aid/respawn wait with timer and respawn point list.
pub const ON_BEGIN_AID_WAIT: u16 = 98;
/// End aid/respawn wait.
pub const ON_END_AID_WAIT: u16 = 99;
/// DHD (Dial Home Device) reply message.
pub const ON_DHD_REPLY: u16 = 100;
/// Known abilities list update.
pub const ON_KNOWN_ABILITIES_UPDATE: u16 = 101;
/// Time of day update (time, wind, weather).
pub const ON_TIME_OF_DAY: u16 = 102;
/// Override performance stats reporting rate.
pub const ON_OVERRIDE_PERF_STATS_RATE: u16 = 103;
/// Initial interaction choices for an entity.
pub const ON_INITIAL_INTERACTION: u16 = 104;
/// Display a dialog window.
pub const ON_DIALOG_DISPLAY: u16 = 105;
/// Open personal vault.
pub const ON_VAULT_OPEN: u16 = 106;
/// Open team vault.
pub const ON_TEAM_VAULT_OPEN: u16 = 107;
/// Open command vault.
pub const ON_COMMAND_VAULT_OPEN: u16 = 108;
/// Open vendor store UI.
pub const ON_STORE_OPEN: u16 = 109;
/// Update vendor store contents.
pub const ON_STORE_UPDATE: u16 = 110;
/// Close vendor store UI.
pub const ON_STORE_CLOSE: u16 = 111;
/// Crafting respec cost prompt.
pub const ON_CRAFTING_RESPEC_PROMPT: u16 = 112;
/// Open trainer ability UI.
pub const ON_TRAINER_OPEN: u16 = 113;
/// Display loot window.
pub const ON_LOOT_DISPLAY: u16 = 114;
/// Player data fully loaded (post world-entry).
pub const ON_PLAYER_DATA_LOADED: u16 = 115;
/// Teleport player to position.
pub const ON_PLAYER_TELEPORT: u16 = 116;
/// Trigger client map load (world entry).
pub const ON_CLIENT_MAP_LOAD: u16 = 117;
/// Grant an ability to the player.
pub const GIVE_ABILITY: u16 = 118;
/// Grant XP for reaching a level.
pub const GIVE_XP_FOR_LEVEL: u16 = 119;
/// Display the DHD (stargate dialing) UI.
pub const ON_DISPLAY_DHD: u16 = 120;
/// Display an error code to the player.
pub const ON_ERROR_CODE: u16 = 121;
/// Set world parameters (gravity, speeds, weather, time).
pub const SETUP_WORLD_PARAMETERS: u16 = 122;
/// Map marker info (quest markers, POIs).
pub const ON_MAP_INFO: u16 = 123;
/// Clear all client-hinted generic regions.
pub const CLEAR_CLIENT_HINTED_GENERIC_REGIONS: u16 = 124;
/// Register a client-hinted generic region for hit testing.
pub const ADD_CLIENT_HINTED_GENERIC_REGION: u16 = 125;
/// Reset map info.
pub const ON_RESET_MAP_INFO: u16 = 126;
/// Display mission rewards selection.
pub const ON_MISSION_REWARDS_DISPLAY: u16 = 127;
/// Display mission offer.
pub const ON_MISSION_OFFER_DISPLAY: u16 = 128;
/// Stargate trigger failed notification.
pub const STARGATE_TRIGGER_FAILED: u16 = 129;
/// Extra display name update.
pub const ON_EXTRA_NAME_UPDATE: u16 = 130;
/// Current XP amount changed.
pub const ON_EXP_UPDATE: u16 = 131;
/// Max XP for current level changed.
pub const ON_MAX_EXP_UPDATE: u16 = 132;
/// Ring transporter destination list.
pub const ON_RING_TRANSPORTER_LIST: u16 = 133;
/// Organization creation result.
pub const ON_ORGANIZATION_CREATION_RESULT: u16 = 134;
/// Launch organization creation UI.
pub const LAUNCH_ORGANIZATION_CREATION: u16 = 135;
/// Discipline/expertise update.
pub const ON_UPDATE_DISCIPLINE: u16 = 136;
/// Discipline respec notification.
pub const ON_DISCIPLINE_RESPEC: u16 = 137;
/// Racial paradigm level changed.
pub const ON_UPDATE_RACIAL_PARADIGM_LEVEL: u16 = 138;
/// Known crafts list update.
pub const ON_UPDATE_KNOWN_CRAFTS: u16 = 139;
/// Crafting options update.
pub const ON_UPDATE_CRAFTING_OPTIONS: u16 = 140;
/// Ability tree info (skill tree data).
pub const ON_ABILITY_TREE_INFO: u16 = 141;
/// Client challenge request (anti-cheat).
pub const ON_CLIENT_CHALLENGE: u16 = 142;
/// Duel challenge from another player.
pub const ON_DUEL_CHALLENGE: u16 = 143;
/// Trade state update (items, cash).
pub const ON_TRADE_STATE: u16 = 144;
/// Trade completion results.
pub const ON_TRADE_RESULTS: u16 = 145;
/// Queued for instanced space entry.
pub const ON_SPACE_QUEUED: u16 = 146;
/// Instanced space queue is ready.
pub const ON_SPACE_QUEUE_READY: u16 = 147;
/// Remote entity created (cross-space tracking).
pub const ON_REMOTE_ENTITY_CREATE: u16 = 148;
/// Remote entity moved.
pub const ON_REMOTE_ENTITY_MOVE: u16 = 149;
/// Remote entity removed.
pub const ON_REMOTE_ENTITY_REMOVE: u16 = 150;
/// Duel participant entities set.
pub const ON_DUEL_ENTITIES_SET: u16 = 151;
/// Duel participant entity removed.
pub const ON_DUEL_ENTITIES_REMOVE: u16 = 152;
/// Clear all duel participant entities.
pub const ON_DUEL_ENTITIES_CLEAR: u16 = 153;
/// Threatened mobs list update.
pub const ON_THREATENED_MOBS_UPDATE: u16 = 154;
/// Play a cinematic movie.
pub const ON_PLAY_MOVIE: u16 = 155;
/// Cancel a playing movie.
pub const ON_CANCEL_MOVIE: u16 = 156;
