//! Minigame type implementations and factory dispatch.

mod kind;
pub use kind::{GameKind, Validation};

mod alignment;
mod converse_humanoid;
mod crystal_game;
mod goauld_crystals;
mod livewire;
mod placeholder;

use crate::minigame::game::MinigameInstance;
use crate::minigame::session::MinigameSession;

/// Create a minigame instance for the given session, dispatching by game name.
pub fn create(session: &MinigameSession) -> Option<Box<dyn MinigameInstance>> {
    let game: Box<dyn MinigameInstance> = match GameKind::from_name(&session.game_name) {
        Some(GameKind::Livewire) => Box::new(livewire::LivewireGame::new(session)),
        Some(GameKind::Alignment) => alignment::create(session),
        Some(GameKind::CrystalGame) => crystal_game::create(session),
        Some(GameKind::GoauldCrystals) => goauld_crystals::create(session),
        Some(GameKind::ConverseBasicHumanoid) => converse_humanoid::create(session),
        Some(
            GameKind::Hack
            | GameKind::Activate
            | GameKind::Analyze
            | GameKind::Bypass
            | GameKind::Converse,
        ) => Box::new(placeholder::PlaceholderGame::new(&session.game_name)),
        None => {
            tracing::warn!(
                entity_id = session.entity_id, // nt:id-only minigame server holds no entity name
                game = %session.game_name,
                "Unknown minigame type, using placeholder",
            );
            Box::new(placeholder::PlaceholderGame::new(&session.game_name))
        }
    };
    Some(game)
}

/// Who validates a victory for the named game: `server` when the server
/// checks the result itself (Livewire, and a port once its `PORTED` flag is
/// set), `client_declared` when the SWF's own `victory` command is taken as
/// the result (the placeholder games and the unported scaffolds). Names the
/// factory does not know run on the placeholder, so they report
/// `client_declared` too. TG-MG-07.
pub fn validation(game_name: &str) -> &'static str {
    GameKind::from_name(game_name).map_or("client_declared", |kind| kind.validation().as_str())
}

/// The abilityBitfield bits the named game implements (MG-22, D-MG18). 0 for
/// an unknown name, and 0 for every game until its port sets `PORTED`.
pub fn ability_bits(game_name: &str) -> u32 {
    GameKind::from_name(game_name).map_or(0, GameKind::ability_bits)
}
