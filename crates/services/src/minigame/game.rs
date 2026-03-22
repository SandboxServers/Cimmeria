//! Minigame instance trait and game factory.

use std::collections::HashMap;

use super::protocol::SfsValue;
use super::session::MinigameSession;

/// Output from a minigame instance.
#[derive(Debug)]
pub enum GameOutput {
    /// Send an extension message to the client(s).
    Send(HashMap<String, SfsValue>),
    /// Game completed with victory.
    Victory,
    /// Game completed with failure/defeat.
    Failure,
}

/// Trait implemented by each minigame type.
///
/// The server creates one instance per player session. The instance manages
/// its own game state and produces `GameOutput` messages in response to
/// client actions and timer ticks.
pub trait MinigameInstance: Send {
    /// Called after successful login + room join. Initialize game state.
    /// Returns initial messages to send (typically `fullgamestate`).
    fn started(&mut self) -> Vec<GameOutput>;

    /// Handle an extension message from the client.
    fn message(&mut self, cmd: &str, params: &HashMap<String, SfsValue>) -> Vec<GameOutput>;

    /// Called on each tick (250ms). Returns messages to send.
    fn tick(&mut self) -> Vec<GameOutput>;

    /// Called when the game is aborted (disconnect/cancel).
    fn aborted(&mut self) -> Vec<GameOutput>;

    /// Whether this game needs periodic ticking.
    fn needs_tick(&self) -> bool;
}

/// Create a minigame instance for the given session.
pub fn create_game(session: &MinigameSession) -> Option<Box<dyn MinigameInstance>> {
    super::games::create(session)
}
