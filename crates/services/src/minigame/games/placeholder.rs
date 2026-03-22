//! Placeholder minigame — auto-victory handler for stub game types.
//!
//! Handles: Hack, Activate, Analyze, Bypass, Converse, ConverseBasicHumanoid.
//! These game types have no Flash SWF implementation beyond a basic shell.
//! The only supported client message is `victory` → instant win.
//!
//! Reference: `python/base/minigame/Placeholder.py`

use std::collections::HashMap;

use crate::minigame::game::{GameOutput, MinigameInstance};
use crate::minigame::protocol::SfsValue;
use crate::sfs_vars;

pub struct PlaceholderGame {
    game_name: String,
}

impl PlaceholderGame {
    pub fn new(game_name: &str) -> Self {
        Self {
            game_name: game_name.to_string(),
        }
    }
}

impl MinigameInstance for PlaceholderGame {
    fn started(&mut self) -> Vec<GameOutput> {
        tracing::info!(game = %self.game_name, "Placeholder minigame started");
        // Send a minimal fullgamestate so the SWF doesn't crash
        vec![GameOutput::Send(sfs_vars! {
            "_cmd" => SfsValue::String("fullgamestate".to_string()),
            "gameStarted" => SfsValue::String("false".to_string()),
        })]
    }

    fn message(&mut self, cmd: &str, _params: &HashMap<String, SfsValue>) -> Vec<GameOutput> {
        match cmd {
            "victory" => {
                tracing::info!(game = %self.game_name, "Placeholder minigame: victory");
                vec![
                    GameOutput::Send(sfs_vars! {
                        "_cmd" => SfsValue::String("victory".to_string()),
                    }),
                    GameOutput::Victory,
                ]
            }
            _ => {
                tracing::debug!(game = %self.game_name, cmd, "Placeholder: ignoring unknown command");
                vec![]
            }
        }
    }

    fn tick(&mut self) -> Vec<GameOutput> {
        vec![]
    }

    fn aborted(&mut self) -> Vec<GameOutput> {
        vec![GameOutput::Send(sfs_vars! {
            "_cmd" => SfsValue::String("failure".to_string()),
        })]
    }

    fn needs_tick(&self) -> bool {
        false
    }
}
