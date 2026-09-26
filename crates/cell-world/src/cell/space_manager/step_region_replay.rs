//! The re-entrancy guard of the H52 step-activation region replay.
//!
//! The replay itself (`content::event_dispatch::step_activation`) re-fires
//! `enter_region` when a mission step activates; a replayed chain can advance
//! another step, which replays again. The guard bounds that recursion. It is
//! a [`SpaceManager`](super::SpaceManager) field, so it lives here.

use std::collections::HashSet;

/// How deep a chain of step activations may replay before the guard stops it.
///
/// Four is a budget, not a modelled depth: the longest authored chain-of-steps
/// in the seed that could plausibly self-advance through regions is two, and a
/// run that reaches four is a content bug worth a WARN rather than a shape
/// worth serving.
pub const MAX_REPLAY_DEPTH: u32 = 4;

/// Re-entrancy bound for the step-activation region replay
/// (`content::event_dispatch::step_activation::fire_step_activation_regions`).
///
/// Lives on [`SpaceManager`](super::SpaceManager) because the recursion runs through
/// `executor::execute_actions`, which cannot thread a depth parameter back
/// here. A thread-local would be wrong: the cell task is `async` and tokio may
/// move it between worker threads at any `await`. The `&mut SpaceManager` the
/// whole call chain already holds *is* the exclusive token, so a plain field
/// on it is both correct and un-lockable.
#[derive(Debug, Default)]
pub struct StepRegionReplayGuard {
    depth: u32,
    /// `(entity_id, mission_id, step_id)` triples already replayed inside the
    /// current outermost activation. Cleared when `depth` returns to zero, so
    /// a later, genuine activation of the same step replays again.
    visited: HashSet<(u32, i32, i32)>,
}

impl StepRegionReplayGuard {
    /// Claim a replay slot. `false` means the caller must not replay and must
    /// not call [`Self::exit`].
    pub fn enter(&mut self, entity_id: u32, mission_id: i32, step_id: i32) -> Option<&'static str> {
        if self.depth >= MAX_REPLAY_DEPTH {
            return Some("replay_depth_exceeded");
        }
        if !self.visited.insert((entity_id, mission_id, step_id)) {
            return Some("step_already_replayed");
        }
        self.depth += 1;
        None
    }

    pub fn exit(&mut self) {
        self.depth = self.depth.saturating_sub(1);
        if self.depth == 0 {
            self.visited.clear();
        }
    }

    /// No replay in flight and nothing remembered — the state the guard must
    /// be back in after every balanced `enter`/`exit` pair.
    #[cfg(any(test, feature = "test-support"))]
    #[doc(hidden)]
    pub fn is_idle(&self) -> bool {
        self.depth == 0 && self.visited.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLAYER_EID: u32 = 1;
    const MISSION: i32 = 1343;
    const STEP_ONE: i32 = 4401;

    #[test]
    fn the_guard_caps_depth_and_clears_on_unwind() {
        let mut g = StepRegionReplayGuard::default();
        for step in 0..MAX_REPLAY_DEPTH as i32 {
            assert_eq!(g.enter(PLAYER_EID, MISSION, step), None, "step {step}");
        }
        assert_eq!(
            g.enter(PLAYER_EID, MISSION, 99),
            Some("replay_depth_exceeded"),
        );
        for _ in 0..MAX_REPLAY_DEPTH {
            g.exit();
        }
        assert!(g.is_idle());
    }

    #[test]
    fn the_guard_refuses_a_repeat_of_the_same_step_within_one_activation() {
        let mut g = StepRegionReplayGuard::default();
        assert_eq!(g.enter(PLAYER_EID, MISSION, STEP_ONE), None);
        assert_eq!(
            g.enter(PLAYER_EID, MISSION, STEP_ONE),
            Some("step_already_replayed"),
        );
        // A different player's identical step is its own slot.
        assert_eq!(g.enter(PLAYER_EID + 1, MISSION, STEP_ONE), None);
    }
}
