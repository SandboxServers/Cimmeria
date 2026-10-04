//! A hit's notes for the in-game combat debug (AB-N1,
//! `cimmeria_cell_world::cell::combat_debug`).
//!
//! The hit takes its pools before the damage ([`HitDebug::start`]), hands
//! `nvp_damage` a vector for its `nvp_damage_resolved` values
//! ([`HitDebug::nvp`]), and once the damage, a god-mode restore and a duel
//! clamp are settled writes the hit, its `effect_planned` rows and its NVP
//! entries ([`HitDebug::finish`]). With no one debugging, `start` takes
//! nothing and the rest are no-ops.

use cimmeria_cell_world::cell::combat_debug::{pools_of, HitNote, Note, NvpNote, PlanNote, Pools};
use cimmeria_entity::abilities::AbilityDef;

use super::super::effect_plan::PlannedEffect;
use super::qr_gate;
use super::HitIds;
use crate::cell::combat::QrResult;
use crate::cell::space_manager::SpaceManager;

/// One hit's debug notes in progress; empty while no one is debugging.
#[derive(Debug, Default)]
pub(super) struct HitDebug {
    before: Option<Pools>,
    nvp: Vec<NvpNote>,
}

impl HitDebug {
    /// The target's pools before the hit, when someone is debugging.
    pub(super) fn start(space_mgr: &SpaceManager, target_eid: u32) -> Self {
        Self {
            before: space_mgr
                .combat_debug
                .is_active()
                .then(|| pools_of(space_mgr, target_eid)),
            nvp: Vec::new(),
        }
    }

    /// Where `apply_nvp_damage` pushes its values, when debugging.
    pub(super) fn nvp(&mut self) -> Option<&mut Vec<NvpNote>> {
        self.before.is_some().then_some(&mut self.nvp)
    }

    /// Note the hit (roll and pools), its plan rows and its NVP entries.
    pub(super) fn finish(
        self,
        space_mgr: &mut SpaceManager,
        ids: HitIds,
        qr: f64,
        result: &QrResult,
        ability_def: Option<&AbilityDef>,
        planned: &[PlannedEffect],
    ) {
        let Some(before) = self.before else {
            return;
        };
        let dont_use_qr = qr_gate::hit_skips_qr(ability_def, space_mgr);
        let after = pools_of(space_mgr, ids.target_eid);
        let (caster, cast, ability) = (ids.entity_id, ids.cast_id, ids.ability_id);
        let debug = &mut space_mgr.combat_debug;
        debug.note(
            caster,
            cast,
            ability,
            Note::Hit(HitNote {
                target_id: ids.target_eid,
                qr,
                roll: result.qr_rand,
                result_code: result.result_code,
                result: qr_gate::result_label(result.result_code),
                dont_use_qr,
                before,
                after,
            }),
        );
        for p in planned {
            let plan = PlanNote {
                target_id: ids.target_eid,
                effect_id: p.effect_id,
                path: p.path,
                reason: p.reason,
            };
            debug.note(caster, cast, ability, Note::Plan(plan));
        }
        for v in self.nvp {
            debug.note(caster, cast, ability, Note::Nvp(v));
        }
    }
}
