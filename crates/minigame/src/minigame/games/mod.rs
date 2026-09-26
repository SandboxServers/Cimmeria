//! Minigame type implementations and factory dispatch.

mod livewire;
mod placeholder;

use crate::minigame::game::MinigameInstance;
use crate::minigame::session::MinigameSession;

/// Create a minigame instance for the given session, dispatching by game name.
pub fn create(session: &MinigameSession) -> Option<Box<dyn MinigameInstance>> {
    let game: Box<dyn MinigameInstance> = match session.game_name.as_str() {
        "Livewire" => Box::new(livewire::LivewireGame::new(session)),
        // TODO: Port Alignment and GoauldCrystals from Python
        // "Alignment" => Box::new(alignment::AlignmentGame::new(session)),
        // "GoauldCrystals" | "CrystalGame" => Box::new(goauld_crystals::GoauldCrystalsGame::new(session)),
        "Hack" | "Activate" | "Analyze" | "Bypass" | "Converse" | "ConverseBasicHumanoid" => {
            Box::new(placeholder::PlaceholderGame::new(&session.game_name))
        }
        _ => {
            tracing::warn!(game = %session.game_name, "Unknown minigame type, using placeholder");
            Box::new(placeholder::PlaceholderGame::new(&session.game_name))
        }
    };
    Some(game)
}
