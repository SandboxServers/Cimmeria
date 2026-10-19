//! ConverseBasicHumanoid: the port lands in MG-CH1..3 (docs/analysis/minigames/spec-conversebasichumanoid.md).
use crate::minigame::game::MinigameInstance;
use crate::minigame::session::MinigameSession;
/// Flipped to `true` by MG-CH3 only, once victory is server-decided.
pub(crate) const PORTED: bool = false;
pub(crate) fn create(session: &MinigameSession) -> Box<dyn MinigameInstance> {
    Box::new(super::placeholder::PlaceholderGame::new(&session.game_name))
}
