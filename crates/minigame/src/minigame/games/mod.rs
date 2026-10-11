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
/// checks the result itself (Livewire), `client_declared` when the SWF's own
/// `victory` command is taken as the result (the placeholder games). Names
/// the factory does not know run on the placeholder, so they report
/// `client_declared` too. TG-MG-07.
pub fn validation(game_name: &str) -> &'static str {
    match game_name {
        "Livewire" => "server",
        _ => "client_declared",
    }
}

#[cfg(test)]
mod tests {
    use super::validation;

    /// Livewire is the only game the server validates; every placeholder
    /// name, and any unknown name, is client-declared.
    #[test]
    fn validation_names_the_validator() {
        assert_eq!(validation("Livewire"), "server");
        for name in [
            "Hack",
            "Activate",
            "Analyze",
            "Bypass",
            "Converse",
            "ConverseBasicHumanoid",
        ] {
            assert_eq!(validation(name), "client_declared", "{name}");
        }
        assert_eq!(validation("NoSuchGame"), "client_declared");
    }
}
