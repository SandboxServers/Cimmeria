//! A beneficial area half's fan-out around its caster (AB-07 rule 3, audit
//! B-28): Morale Boost's "Short Radius AE 35% Focus Heal".
//!
//! The recipients are the caster and every other player in the caster's
//! space, alive and within the effect's radius tier (`tcm_param1`, through
//! `EffectDef::tcm_range_meters`) of the caster, that the support-shot rule
//! calls an ally (`support_shot::classify`: a player the caster may not
//! attack). No NPC is a recipient, friendly or not, as for support darts.
//! Area abilities check no line of sight (ADR decision 20), and neither does
//! this.
//!
//! **No double dip.** A recipient that this cast already lands the same
//! script on is skipped. Morale Boost carries "Rally User Focus heal" (939,
//! single target) beside its area heal (1215): the caster takes 939, and
//! 1215 heals the allies around them, so the caster gets 35%, not 70%.
//! Leadership's user and area halves have the same shape. An area half with
//! no single twin (Holy Warrior's 4220) still lands on the caster.

use cimmeria_cell_world::cell::duel::DuelResources;
use cimmeria_entity::abilities::{AbilityDef, EffectDef};

use super::super::super::combat;
use super::super::super::space_manager::SpaceManager;
use super::super::use_ability::{classify, SupportTarget};
use super::{Landing, LandingRoute, REASON_BENEFICIAL_AREA};

/// `event` of the fan-out row (target `abilities`).
pub(crate) const EVENT_ALLY_AREA: &str = "ally_area_fan_out";

/// The landings of `effect`, a beneficial area half of `caster_id`'s cast of
/// `def`, given the cast's `earlier` landings (module docs).
pub(super) fn ally_landings(
    space_mgr: &SpaceManager,
    caster_id: u32,
    def: &AbilityDef,
    effect: &EffectDef,
    earlier: &[Landing],
) -> Vec<Landing> {
    let Some(caster) = space_mgr.get_entity(caster_id) else {
        return Vec::new();
    };
    let radius = EffectDef::tcm_range_meters(&effect.tcm_param1);
    let duels = space_mgr.resources.duels();
    let mut candidates: Vec<u32> = space_mgr.all_player_entity_ids();
    if !candidates.contains(&caster_id) {
        candidates.push(caster_id);
    }
    candidates.sort_unstable();
    let mut in_radius = Vec::new();
    for id in candidates {
        let Some(e) = space_mgr.get_entity(id) else {
            continue;
        };
        if e.space_id != caster.space_id
            || combat::is_dead_state(e.state_field)
            || caster.position.distance_to(&e.position) > radius
            || classify(caster, e, duels) != SupportTarget::Ally
        {
            continue;
        }
        in_radius.push(id);
    }
    let (skipped, allies): (Vec<u32>, Vec<u32>) = in_radius
        .into_iter()
        .partition(|&id| already_landed(earlier, id, effect));
    let who = space_mgr.player_identity(caster_id);
    // Canonical subject ids beside the entity ids (instrumentation rule 5):
    // every ally is a player, so each has a `player_id`.
    let player_ids = |ids: &[u32]| -> Vec<Option<i32>> {
        ids.iter()
            .map(|&id| space_mgr.player_identity(id).player_id)
            .collect()
    };
    let ally_player_ids = player_ids(&allies);
    let skipped_player_ids = player_ids(&skipped);
    tracing::debug!(
        target: "abilities",
        event = EVENT_ALLY_AREA,
        account_id = who.account_id,
        player_id = who.player_id,
        entity_id = caster_id,
        ability_id = def.ability_id,
        effect_id = effect.effect_id,
        radius,
        ally_ids = ?allies,
        ally_player_ids = ?ally_player_ids,
        skipped_same_script = ?skipped,
        skipped_player_ids = ?skipped_player_ids,
        "beneficial area effect fanned out to the caster's allies in its radius"
    );
    allies
        .into_iter()
        .map(|id| Landing::new(effect, id, LandingRoute::AllyArea(REASON_BENEFICIAL_AREA)))
        .collect()
}

/// Whether an earlier landing of this cast already runs `effect`'s script on
/// `recipient`. An effect with no script lands nothing either way.
fn already_landed(earlier: &[Landing], recipient: u32, effect: &EffectDef) -> bool {
    effect.script_name.as_deref().is_some_and(|script| {
        earlier
            .iter()
            .any(|l| l.recipient == recipient && l.effect.script_name.as_deref() == Some(script))
    })
}
