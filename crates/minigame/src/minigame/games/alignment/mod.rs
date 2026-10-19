//! Alignment: the port lands in MG-AL1..3 (docs/analysis/minigames/spec-alignment.md).
use crate::minigame::game::MinigameInstance;
use crate::minigame::session::MinigameSession;
/// Flipped to `true` by MG-AL3 only, once victory is server-decided.
pub(crate) const PORTED: bool = false;
pub(crate) fn create(session: &MinigameSession) -> Box<dyn MinigameInstance> {
    Box::new(super::placeholder::PlaceholderGame::new(&session.game_name))
}
