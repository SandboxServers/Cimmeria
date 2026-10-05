//! In-combat player vitals sample (`target: "vitals"`, `combat_sample`).
//!
//! Every [`VITALS_SAMPLE_EVERY_TICKS`] AoI ticks (2 s) the cell logs the
//! health and focus of every living player whose threat set is non-empty.
//! The hit pipeline logs each hit (`damage_taken`), but a DoT, an effect
//! script's drain or a heal changes the pools without passing through it;
//! the sample is what makes those visible. Rows and budget:
//! `crate::cell::combat::vitals`.

use crate::cell::combat::state::BSF_DEAD;
use crate::cell::combat::vitals::log_combat_sample;
use crate::cell::space_manager::SpaceManager;

/// AoI ticks (100 ms) between samples: 2 s. Pinned by
/// `vitals_sample_budget_matches_the_documented_rate` against the figures in
/// `docs/architecture/observability.md` (`vitals` row).
pub(in crate::cell::service) const VITALS_SAMPLE_EVERY_TICKS: u32 = 20;

/// Log one `combat_sample` row per living, in-combat player.
pub(in crate::cell::service) fn vitals_sample_tick(space_mgr: &SpaceManager) {
    for eid in space_mgr.all_player_entity_ids() {
        if let Some(e) = space_mgr.get_entity(eid) {
            if !e.threatened_mobs.is_empty() && e.state_field & BSF_DEAD == 0 {
                log_combat_sample(space_mgr, e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::LogCapture;
    use tracing::Level;

    fn mgr() -> SpaceManager {
        let mut mgr = SpaceManager::new(1);
        mgr.parse_spaces_xml(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#,
        )
        .unwrap();
        mgr.create_startup_spaces(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
        )
        .unwrap();
        for (eid, pid) in [(1, 72), (2, 73)] {
            mgr.create_entity(eid, "Castle", [0.0; 3], [0.0; 3])
                .unwrap();
            let e = mgr.get_entity_mut(eid).unwrap();
            e.is_player = true;
            e.player_id = Some(pid);
            e.account_id = Some(6);
            mgr.connect_entity(eid);
        }
        mgr
    }

    /// Only the player with a non-empty threat set is sampled.
    #[test]
    fn samples_only_players_in_combat() {
        let mut mgr = mgr();
        mgr.get_entity_mut(1).unwrap().threatened_mobs.insert(900);
        let capture = LogCapture::install();
        vitals_sample_tick(&mgr);
        let rows: Vec<_> = capture
            .all()
            .into_iter()
            .filter(|c| c.level == Level::DEBUG && c.target == "vitals")
            .collect();
        assert_eq!(rows.len(), 1, "{rows:#?}");
        assert!(rows[0].has_field("event", "combat_sample"));
        assert!(rows[0].has_field("player_id", "72"));
        assert!(rows[0].has_field("account_id", "6"));
    }

    /// A dead player is not sampled, even with threat left on it.
    #[test]
    fn dead_player_is_not_sampled() {
        let mut mgr = mgr();
        let e = mgr.get_entity_mut(1).unwrap();
        e.threatened_mobs.insert(900);
        e.state_field |= BSF_DEAD;
        let capture = LogCapture::install();
        vitals_sample_tick(&mgr);
        assert!(capture.all().iter().all(|c| c.target != "vitals"));
    }

    /// 2 s at the 100 ms AoI tick: 1,800 rows an hour per fighting player,
    /// the figure `observability.md` quotes.
    #[test]
    fn vitals_sample_budget_matches_the_documented_rate() {
        assert_eq!(VITALS_SAMPLE_EVERY_TICKS, 20);
        assert_eq!(3_600_000 / (VITALS_SAMPLE_EVERY_TICKS * 100), 1_800);
    }
}
