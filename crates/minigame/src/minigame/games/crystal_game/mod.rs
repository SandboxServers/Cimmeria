//! CrystalGame: the port lands in MG-CG1..3 (docs/analysis/minigames/spec-crystalgame.md).
use crate::minigame::game::MinigameInstance;
use crate::minigame::session::MinigameSession;
/// Flipped to `true` by MG-CG3 only, once victory is server-decided.
pub(crate) const PORTED: bool = false;
pub(crate) fn create(session: &MinigameSession) -> Box<dyn MinigameInstance> {
    Box::new(super::placeholder::PlaceholderGame::new(&session.game_name))
}
