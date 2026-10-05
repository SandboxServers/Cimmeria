//! GM god mode (`gmSetGodMode`, SGWGmPlayer index 142): the damage gate.
//!
//! Damage reaches a target's Health and Focus through many writers: the NVP
//! pipeline, every damage and after-hit effect script, and each DoT pulse
//! (script or NVP). They meet at two seams, the per-target hit
//! (`abilities::damage_apply::apply_hit`, which every single-target, AoE,
//! cone, splash and deployable hit goes through) and the pulse
//! (`effects::pulsing::tick::fire_pulse`). Each seam arms one
//! [`GodModeGuard`] before its damage and calls [`GodModeGuard::restore`]
//! after it, before the death check and the stat flush.
//!
//! Restoring the pools, rather than refusing the hit, is deliberate: a
//! god-mode GM still receives heals, buffs, debuffs and crowd control, so an
//! ability under test lands everything but the loss. Absorb shields still
//! spend themselves on the hit; only Health and Focus are put back.

use cimmeria_entity::abilities::{ClientEffectResult, SRC_ABSORB};
use cimmeria_entity::stats::{FOCUS, HEALTH};

use crate::cell::space_manager::SpaceManager;

/// Health and Focus of a god-mode entity before a damage seam.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GodModeGuard {
    target_id: u32,
    health: Option<i32>,
    focus: Option<i32>,
}

/// Where the absorbed damage came from, for the `god_mode_absorbed` row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DamageSource {
    /// The attacker, or the pulse's invoker.
    pub source_id: u32,
    pub ability_id: Option<i32>,
    pub effect_id: Option<i32>,
    /// `ability_hit`, `ability_script` or `effect_pulse`.
    pub seam: &'static str,
}

impl DamageSource {
    /// An ability hit's source: the attacker and the ability.
    pub fn hit(attacker_id: u32, ability_id: i32, seam: &'static str) -> Self {
        Self {
            source_id: attacker_id,
            ability_id: Some(ability_id),
            effect_id: None,
            seam,
        }
    }
}

/// What [`GodModeGuard::restore`] put back.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Absorbed {
    pub health: i32,
    pub focus: i32,
}

impl Absorbed {
    /// Whether anything was put back.
    pub fn any(self) -> bool {
        self.health > 0 || self.focus > 0
    }
}

impl GodModeGuard {
    /// Snapshot `target_id`'s pools if it is in god mode; `None` otherwise,
    /// so the seam pays one lookup for an ordinary target.
    pub fn arm(space_mgr: &SpaceManager, target_id: u32) -> Option<Self> {
        let target = space_mgr.get_entity(target_id).filter(|e| e.god_mode)?;
        Some(Self {
            target_id,
            health: target.stats.get(HEALTH).map(|s| s.cur),
            focus: target.stats.get(FOCUS).map(|s| s.cur),
        })
    }

    /// Put back any Health or Focus the target lost since [`Self::arm`],
    /// and log one `god_mode_absorbed` row when something was. A gain (a
    /// heal) is kept. Marks the restored stats dirty, so the seam's own
    /// flush sends the client the unchanged bar.
    pub fn restore(&self, space_mgr: &mut SpaceManager, source: DamageSource) -> Absorbed {
        let Some(target) = space_mgr.get_entity_mut(self.target_id) else {
            return Absorbed::default();
        };
        let mut absorbed = Absorbed::default();
        for (stat_id, before, lost) in [
            (HEALTH, self.health, &mut absorbed.health),
            (FOCUS, self.focus, &mut absorbed.focus),
        ] {
            let (Some(before), Some(stat)) = (before, target.stats.get_mut(stat_id)) else {
                continue;
            };
            if stat.cur < before {
                *lost = before - stat.cur;
                stat.set_current(before);
            }
        }
        if absorbed.any() {
            let target_identity = space_mgr.player_identity(self.target_id);
            let source_identity = space_mgr.player_identity(source.source_id);
            tracing::debug!(
                target: "abilities",
                event = "god_mode_absorbed",
                // Rule 5: the canonical pair names the actor (the damage
                // source, empty for an NPC); the protected GM is the
                // target, as in every other damage row.
                account_id = source_identity.account_id,
                account_name = source_identity.account_name,
                player_id = source_identity.player_id,
                player_name = source_identity.player_name,
                entity_id = source.source_id,
                entity_name = space_mgr.entity_label(source.source_id),
                target_account_id = target_identity.account_id,
                target_account_name = target_identity.account_name,
                target_player_id = target_identity.player_id,
                target_player_name = target_identity.player_name,
                target_id = self.target_id,
                target_name = space_mgr.entity_label(self.target_id),
                ability_id = source.ability_id,
                ability_name = cimmeria_cell_world::cell::effects::content_names::ability_name(source.ability_id),
                effect_id = source.effect_id,
                effect_name = cimmeria_cell_world::cell::effects::content_names::effect_name(source.effect_id),
                seam = source.seam,
                health_absorbed = absorbed.health,
                focus_absorbed = absorbed.focus,
                "god mode: damage to a GM put back"
            );
        }
        absorbed
    }

    /// [`Self::restore`] for a hit's direct damage, which also rewrites the
    /// hit's `onEffectResults` entries: a loss becomes `0` with
    /// `SRC_ABSORB`, so the client shows "Absorbed" rather than a number or
    /// a kill. Returns whether anything was put back (the hit then dealt no
    /// damage, for threat).
    pub fn restore_hit(
        &self,
        space_mgr: &mut SpaceManager,
        source: DamageSource,
        results: &mut [ClientEffectResult],
    ) -> bool {
        if !self.restore(space_mgr, source).any() {
            return false;
        }
        for r in results.iter_mut().filter(|r| r.delta < 0) {
            r.delta = 0;
            r.stat_result_code = SRC_ABSORB;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::LogCapture;

    fn mgr_with(god_mode: bool) -> SpaceManager {
        let mut mgr = SpaceManager::new(1);
        mgr.parse_spaces_xml(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#,
        )
        .unwrap();
        mgr.create_startup_spaces(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
        )
        .unwrap();
        mgr.create_entity(1, "Castle", [0.0; 3], [0.0; 3]).unwrap();
        // The attacker (2) is a player too, so actor and target identities
        // are both present and distinguishable.
        mgr.create_entity(2, "Castle", [0.0; 3], [0.0; 3]).unwrap();
        let attacker = mgr.get_entity_mut(2).unwrap();
        attacker.account_id = Some(702);
        attacker.player_id = Some(72);
        let e = mgr.get_entity_mut(1).unwrap();
        e.account_id = Some(701);
        e.player_id = Some(71);
        e.god_mode = god_mode;
        e.stats.get_mut(HEALTH).unwrap().update(0, 100, 100);
        e.stats.get_mut(FOCUS).unwrap().update(0, 50, 50);
        mgr
    }

    fn hit(mgr: &mut SpaceManager, health: i32, focus: i32) {
        let e = mgr.get_entity_mut(1).unwrap();
        e.stats.get_mut(HEALTH).unwrap().change(-health);
        e.stats.get_mut(FOCUS).unwrap().change(-focus);
    }

    fn absorbed_row(
        capture: &crate::test_support::LogCaptureGuard,
    ) -> Option<crate::test_support::Captured> {
        capture
            .all()
            .into_iter()
            .find(|c| c.has_field("event", "god_mode_absorbed"))
    }

    const SOURCE: DamageSource = DamageSource {
        source_id: 2,
        ability_id: Some(592),
        effect_id: None,
        seam: "ability_hit",
    };

    #[test]
    fn an_ordinary_target_arms_nothing() {
        assert!(GodModeGuard::arm(&mgr_with(false), 1).is_none());
    }

    #[test]
    fn a_loss_is_put_back_and_logged() {
        let mut mgr = mgr_with(true);
        let capture = LogCapture::install();
        let guard = GodModeGuard::arm(&mgr, 1).expect("god mode arms");
        hit(&mut mgr, 30, 10);
        let absorbed = guard.restore(&mut mgr, SOURCE);
        assert_eq!(
            absorbed,
            Absorbed {
                health: 30,
                focus: 10
            }
        );
        let e = mgr.get_entity(1).unwrap();
        assert_eq!(e.stats.get(HEALTH).unwrap().cur, 100);
        assert_eq!(e.stats.get(FOCUS).unwrap().cur, 50);
        let row = absorbed_row(&capture).expect("one god_mode_absorbed row");
        assert!(row.has_field("health_absorbed", "30"));
        assert!(row.has_field("seam", "ability_hit"));
        // Rule 5 (instrumentation-discipline.md, "an actor acts on someone
        // else"): the canonical pair is the attacker's, the GM is target_*.
        for (k, v) in [
            ("account_id", "702"),
            ("player_id", "72"),
            ("entity_id", "2"),
            ("target_account_id", "701"),
            ("target_player_id", "71"),
            ("target_id", "1"),
        ] {
            assert!(row.has_field(k, v), "{k}={v}: {row:#?}");
        }
    }

    #[test]
    fn a_heal_is_kept_and_nothing_is_logged() {
        let mut mgr = mgr_with(true);
        mgr.get_entity_mut(1)
            .unwrap()
            .stats
            .get_mut(HEALTH)
            .unwrap()
            .update(0, 60, 100);
        let capture = LogCapture::install();
        let guard = GodModeGuard::arm(&mgr, 1).unwrap();
        hit(&mut mgr, -25, 0);
        assert!(!guard.restore(&mut mgr, SOURCE).any());
        assert_eq!(
            mgr.get_entity(1).unwrap().stats.get(HEALTH).unwrap().cur,
            85
        );
        assert!(absorbed_row(&capture).is_none());
    }
}
