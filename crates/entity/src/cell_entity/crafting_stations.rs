//! A player's crafting-station state on the cell (CR-05).
//!
//! The cell's 1 Hz station tick finds the nearest station per crafting verb
//! and reports the set to the base only when it differs from the last one it
//! reported. This is that last report. It lives on the entity, so a
//! destroyed entity takes it along and a re-created one (world change,
//! relog) starts from "never reported" and reports on its first tick.

/// The station set last reported to the base for one player.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CraftingStationState {
    /// The nearest station entity per verb, in `CraftingOptions` section
    /// order (crafting, research, reverseEngineering, alloying). `None`
    /// until the first report.
    pub last_reported: Option<[Option<u32>; 4]>,
}

impl CraftingStationState {
    /// Record `current` and return whether it differs from the last report
    /// (always true for the first one), i.e. whether the base must hear it.
    pub fn record(&mut self, current: [Option<u32>; 4]) -> bool {
        if self.last_reported == Some(current) {
            return false;
        }
        self.last_reported = Some(current);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_record_always_reports_even_when_empty() {
        let mut state = CraftingStationState::default();
        assert!(state.record([None; 4]));
        assert!(!state.record([None; 4]));
    }

    #[test]
    fn only_changes_report() {
        let mut state = CraftingStationState::default();
        assert!(state.record([Some(7), None, None, None]));
        assert!(!state.record([Some(7), None, None, None]));
        assert!(state.record([Some(8), None, None, None]));
        assert!(state.record([None; 4]));
    }
}
