//! Typed convenience constructors: one `emit_*` helper per event type with
//! a permanent emit site.
//!
//! Each helper builds the [`Event`] variant + current timestamp + calls
//! [`emit`]. The point is to keep the call site terse:
//!
//! ```ignore
//! cimmeria_discord::emit_level_up(Named::new(player_id, Some(name)), new_level);
//! ```
//!
//! rather than:
//!
//! ```ignore
//! cimmeria_discord::emit(cimmeria_discord::Event::PlayerLevelUp {
//!     character: Named::new(player_id, Some(name)),
//!     new_level,
//!     timestamp: chrono::Utc::now(),
//! });
//! ```
//!
//! Every object an event names is a [`Named`]: its ID and its name, so
//! the embed renders it `Name (#id)` (Rule 6, "Discord"). Pass what the
//! call site has; the renderer degrades to `#id`, the bare name, or `?`.
//!
//! Add a helper here for any event type that has a permanent emit site.

use std::net::SocketAddr;

use crate::{emit, ChatKind, DisconnectReason, Event, Named};

pub fn emit_server_startup(version: impl Into<String>, bind_addrs: Vec<String>) {
    emit(Event::ServerStartup {
        version: version.into(),
        bind_addrs,
        timestamp: chrono::Utc::now(),
    });
}

pub fn emit_server_shutdown(reason: impl Into<String>, uptime_secs: u64) {
    emit(Event::ServerShutdown {
        reason: reason.into(),
        uptime_secs,
        timestamp: chrono::Utc::now(),
    });
}

pub fn emit_player_login(account: Named, character: Option<Named>, addr: SocketAddr) {
    emit(Event::PlayerLogin {
        account,
        character,
        addr,
        timestamp: chrono::Utc::now(),
    });
}

pub fn emit_player_logout(account: Named, character: Option<Named>, session_secs: u64) {
    emit(Event::PlayerLogout {
        account,
        character,
        session_secs,
        timestamp: chrono::Utc::now(),
    });
}

pub fn emit_player_disconnect(
    account: Named,
    character: Option<Named>,
    addr: SocketAddr,
    reason: DisconnectReason,
    session_secs: u64,
) {
    emit(Event::PlayerDisconnect {
        account,
        character,
        addr,
        reason,
        session_secs,
        timestamp: chrono::Utc::now(),
    });
}

pub fn emit_player_auth_failed(
    account_name: impl Into<String>,
    addr: SocketAddr,
    reason: impl Into<String>,
) {
    emit(Event::PlayerAuthFailed {
        account_name: account_name.into(),
        addr,
        reason: reason.into(),
        timestamp: chrono::Utc::now(),
    });
}

pub fn emit_player_world_entry(account: Named, character: Named, world: Named, position: [f32; 3]) {
    emit(Event::PlayerWorldEntry {
        account,
        character,
        world,
        position,
        timestamp: chrono::Utc::now(),
    });
}

pub fn emit_player_world_exit(
    account: Named,
    character: Named,
    from_world: Named,
    to_world: Option<Named>,
) {
    emit(Event::PlayerWorldExit {
        account,
        character,
        from_world,
        to_world,
        timestamp: chrono::Utc::now(),
    });
}

pub fn emit_chat(
    kind: ChatKind,
    speaker: Named,
    recipient: Option<Named>,
    content: impl Into<String>,
) {
    emit(Event::Chat {
        kind,
        speaker,
        recipient,
        content: content.into(),
        timestamp: chrono::Utc::now(),
    });
}

pub fn emit_level_up(character: Named, new_level: u32) {
    emit(Event::PlayerLevelUp {
        character,
        new_level,
        timestamp: chrono::Utc::now(),
    });
}

pub fn emit_mission_accepted(character: Named, mission: Named) {
    emit(Event::MissionAccepted {
        character,
        mission,
        timestamp: chrono::Utc::now(),
    });
}

pub fn emit_mission_completed(character: Named, mission: Named) {
    emit(Event::MissionCompleted {
        character,
        mission,
        timestamp: chrono::Utc::now(),
    });
}

pub fn emit_mission_failed(character: Named, mission: Named, reason: impl Into<String>) {
    emit(Event::MissionFailed {
        character,
        mission,
        reason: reason.into(),
        timestamp: chrono::Utc::now(),
    });
}

pub fn emit_gm_command(
    gm: Named,
    command: impl Into<String>,
    args: impl Into<String>,
    target: Option<Named>,
) {
    emit(Event::GmCommand {
        gm,
        command: command.into(),
        args: args.into(),
        target,
        timestamp: chrono::Utc::now(),
    });
}

pub fn emit_item_used(character: Named, item: Named, target: Option<Named>) {
    emit(Event::ItemUsed {
        character,
        item,
        target,
        timestamp: chrono::Utc::now(),
    });
}

pub fn emit_character_created(account: Named, character: Named, archetype: Named, world: Named) {
    emit(Event::CharacterCreated {
        account,
        character,
        archetype,
        world,
        timestamp: chrono::Utc::now(),
    });
}

pub fn emit_npc_death(
    npc: Named,
    template: Option<Named>,
    killer: Option<Named>,
    cause: impl Into<String>,
    world: Option<Named>,
) {
    emit(Event::NpcDeath {
        npc,
        template,
        killer,
        cause: cause.into(),
        world,
        timestamp: chrono::Utc::now(),
    });
}

pub fn emit_minigame_result(
    game: impl Into<String>,
    character: Named,
    success: bool,
    victory_chains: Vec<Named>,
) {
    emit(minigame_result_event(
        game,
        character,
        success,
        victory_chains,
    ));
}

/// The [`Event::MinigameResult`] [`emit_minigame_result`] posts, stamped
/// now. Public so the minigame server can test what it would post without
/// the global runtime.
pub fn minigame_result_event(
    game: impl Into<String>,
    character: Named,
    success: bool,
    victory_chains: Vec<Named>,
) -> Event {
    Event::MinigameResult {
        game: game.into(),
        character,
        success,
        victory_chains,
        timestamp: chrono::Utc::now(),
    }
}

pub fn emit_dialog(character: Named, dialog: Named, choice: Option<Named>) {
    emit(Event::Dialog {
        character,
        dialog,
        choice,
        timestamp: chrono::Utc::now(),
    });
}

pub fn emit_wire_format_error(
    kind: impl Into<String>,
    addr: Option<SocketAddr>,
    details: impl Into<String>,
) {
    emit(Event::WireFormatError {
        kind: kind.into(),
        addr,
        details: details.into(),
        timestamp: chrono::Utc::now(),
    });
}

pub fn emit_db_error(operation: impl Into<String>, details: impl Into<String>) {
    emit(Event::DbError {
        operation: operation.into(),
        details: details.into(),
        timestamp: chrono::Utc::now(),
    });
}

pub fn emit_mercury_timeout(
    addr: SocketAddr,
    account: Named,
    character: Option<Named>,
    silence_secs: u64,
) {
    emit(Event::MercuryTimeout {
        addr,
        account,
        character,
        silence_secs,
        timestamp: chrono::Utc::now(),
    });
}

pub fn emit_player_death(
    character: Named,
    killer: Option<Named>,
    cause: impl Into<String>,
    world: Option<Named>,
) {
    emit(Event::PlayerDeath {
        character,
        killer,
        cause: cause.into(),
        world,
        timestamp: chrono::Utc::now(),
    });
}

pub fn emit_player_respawn(character: Named, world: Named) {
    emit(Event::PlayerRespawn {
        character,
        world,
        timestamp: chrono::Utc::now(),
    });
}
