//! Per-NPC leash and reset bookkeeping (NA12, D-NA03).
//!
//! Grouped in one struct so the leash policy's scratch state lives in one
//! place on [`super::CellEntity`] rather than as four loose fields. Every
//! field is server-side runtime state: never persisted, never on the wire.

use std::time::Instant;

/// Leash radius, walk-home timer, lost-target timer and the post-reset
/// re-aggro window for one NPC.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LeashState {
    /// Per-template leash radius in world units, from
    /// `entity_templates.leash_distance`. `None` means the template does not
    /// set one, and the server default applies
    /// (`cell::combat::LEASH_DISTANCE`, 50 u).
    pub distance_override: Option<f32>,
    /// When the NPC started walking home. Set on entering `Leashing` and
    /// cleared on arrival. The leash tick snaps the NPC home when the walk
    /// takes longer than its timeout.
    pub walk_started_at: Option<Instant>,
    /// When the NPC first found its top-threat target beyond its AoI radius.
    /// Cleared as soon as the target is back in range. Once this has been set
    /// for the lost-target grace period, the NPC drops the target.
    pub target_lost_since: Option<Instant>,
    /// Until this instant, the Idle auto-aggro scan ignores players. Set on
    /// leash arrival so a reset NPC does not re-aggro on the same player in
    /// the same breath (the aggro/leash loop, audit S5).
    pub reaggro_suppressed_until: Option<Instant>,
}

impl LeashState {
    /// Whether the post-reset re-aggro window is still open at `now`.
    pub fn reaggro_suppressed(&self, now: Instant) -> bool {
        self.reaggro_suppressed_until.is_some_and(|t| now < t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn reaggro_window_is_open_only_before_its_deadline() {
        let now = Instant::now();
        let mut s = LeashState::default();
        assert!(!s.reaggro_suppressed(now), "no window by default");
        s.reaggro_suppressed_until = Some(now + Duration::from_secs(5));
        assert!(s.reaggro_suppressed(now));
        assert!(!s.reaggro_suppressed(now + Duration::from_secs(5)));
    }
}
