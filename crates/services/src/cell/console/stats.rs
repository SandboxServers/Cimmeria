//! Granular per-domain stat readouts (category F): `.primarystats`,
//! `.speedstats`, `.armorstats`, `.qrstats`, `.absorbstats`, `.stealthstats`.
//!
//! Read-only — each dumps a fixed list of stats of the selected target (a
//! `Being`) via the feedback channel, one `label: cur/max` line per stat. These
//! are the granular views the umbrella `/gmprintstats` doesn't break out.
//!
//! Legacy reference: `deprecated/python/cell/commands/Entity.py`
//! (`entityPrimaryStats` … `entityStealthStats`).

use cimmeria_entity::stats::{
    ABSORB_ENERGY, ABSORB_ENERGY_ENERGY, ABSORB_ENERGY_ITEM, ABSORB_HAZMAT, ABSORB_HAZMAT_ENERGY,
    ABSORB_HAZMAT_ITEM, ABSORB_PHYSICAL, ABSORB_PHYSICAL_ENERGY, ABSORB_PHYSICAL_ITEM,
    ABSORB_PSIONIC, ABSORB_PSIONIC_ENERGY, ABSORB_PSIONIC_ITEM, ABSORB_UNTYPED,
    ABSORB_UNTYPED_ENERGY, ABSORB_UNTYPED_ITEM, ACCURACY, AWARENESS, COORDINATION, COVER_ACCURACY,
    COVER_DEFENSE, COVER_QR_MODIFIER, CROUCHING_ACCURACY, CROUCHING_DEFENSE, DAMAGE, DEFENSE,
    DISGUISE_DETECTION, DISGUISE_RATING, ENERGY_AF, ENGAGEMENT, FORTITUDE, HAZMAT_AF, HEALTH_RES,
    INTELLIGENCE, INTERRUPT_RES, KINETIC_RES, MENTAL_RES, MITIGATION, MORALE, MOVEMENT_SPEED_MOD,
    NEGATION, PENETRATION, PERCEPTION, PHYSICAL_AF, PSIONIC_AF, QR_MOD, RECOVERY, RESPONSE,
    RESTORATION, REVEAL_RATING, ROTATION_SPEED_MOD, SPEED_ATTACK, SPEED_DEPLOY, SPEED_GRENADE,
    SPEED_RELOAD, STABILIZATION, STEALTH_MOVEMENT, STEALTH_RATING, SUBTLETY, TRACKING,
};
use tokio::sync::mpsc;

use super::send_gm_feedback;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// (display label, stat id) lists, mirroring the legacy `statIds` arrays.
fn stat_set(cmd: &str) -> &'static [(&'static str, i32)] {
    match cmd {
        "primarystats" => &[
            ("coordination", COORDINATION),
            ("engagement", ENGAGEMENT),
            ("fortitude", FORTITUDE),
            ("morale", MORALE),
            ("perception", PERCEPTION),
            ("intelligence", INTELLIGENCE),
        ],
        "speedstats" => &[
            ("movementSpeedMod", MOVEMENT_SPEED_MOD),
            ("rotationSpeedMod", ROTATION_SPEED_MOD),
            ("speedReload", SPEED_RELOAD),
            ("speedGrenade", SPEED_GRENADE),
            ("speedDeploy", SPEED_DEPLOY),
            ("speedAttack", SPEED_ATTACK),
        ],
        "armorstats" => &[
            ("physicalAF", PHYSICAL_AF),
            ("energyAF", ENERGY_AF),
            ("hazmatAF", HAZMAT_AF),
            ("psionicAF", PSIONIC_AF),
            ("kineticRes", KINETIC_RES),
            ("mentalRes", MENTAL_RES),
            ("healthRes", HEALTH_RES),
            ("interruptRes", INTERRUPT_RES),
        ],
        "qrstats" => &[
            ("accuracy", ACCURACY),
            ("defense", DEFENSE),
            ("qrMod", QR_MOD),
            ("coverQRModifier", COVER_QR_MODIFIER),
            ("response", RESPONSE),
            ("damage", DAMAGE),
            ("penetration", PENETRATION),
            ("tracking", TRACKING),
            ("stabilization", STABILIZATION),
            ("awareness", AWARENESS),
            ("coverAccuracy", COVER_ACCURACY),
            ("coverDefense", COVER_DEFENSE),
            ("crouchingAccuracy", CROUCHING_ACCURACY),
            ("crouchingDefense", CROUCHING_DEFENSE),
            ("negation", NEGATION),
            ("mitigation", MITIGATION),
            ("recovery", RECOVERY),
            ("restoration", RESTORATION),
            ("subtlety", SUBTLETY),
        ],
        "absorbstats" => &[
            ("absorbPhysical", ABSORB_PHYSICAL),
            ("absorbEnergy", ABSORB_ENERGY),
            ("absorbHazmat", ABSORB_HAZMAT),
            ("absorbPsionic", ABSORB_PSIONIC),
            ("absorbUntyped", ABSORB_UNTYPED),
            ("absorbPhysicalItem", ABSORB_PHYSICAL_ITEM),
            ("absorbEnergyItem", ABSORB_ENERGY_ITEM),
            ("absorbHazmatItem", ABSORB_HAZMAT_ITEM),
            ("absorbPsionicItem", ABSORB_PSIONIC_ITEM),
            ("absorbUntypedItem", ABSORB_UNTYPED_ITEM),
            ("absorbPhysicalEnergy", ABSORB_PHYSICAL_ENERGY),
            ("absorbEnergyEnergy", ABSORB_ENERGY_ENERGY),
            ("absorbHazmatEnergy", ABSORB_HAZMAT_ENERGY),
            ("absorbPsionicEnergy", ABSORB_PSIONIC_ENERGY),
            ("absorbUntypedEnergy", ABSORB_UNTYPED_ENERGY),
        ],
        "stealthstats" => &[
            ("stealthRating", STEALTH_RATING),
            ("stealthMovement", STEALTH_MOVEMENT),
            ("revealRating", REVEAL_RATING),
            ("disguiseRating", DISGUISE_RATING),
            ("disguiseDetection", DISGUISE_DETECTION),
        ],
        _ => &[],
    }
}

/// Dump the stat set named by `cmd` for the target (or caller if the optional
/// target is absent).
pub(super) async fn show(
    cmd: &str,
    caller_id: u32,
    target_id: Option<u32>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let subject = target_id.unwrap_or(caller_id);
    let Some(entity) = space_mgr.get_entity(subject) else {
        send_gm_feedback(caller_id, &format!("{cmd}: no entity."), tx).await;
        return;
    };
    send_gm_feedback(caller_id, &format!("{cmd} [{subject}]:",), tx).await;
    for (label, stat_id) in stat_set(cmd) {
        let line = match entity.stats.get(*stat_id) {
            Some(s) => format!("    {label}: {}/{}", s.cur, s.max),
            None => format!("    {label}: n/a"),
        };
        send_gm_feedback(caller_id, &line, tx).await;
    }
}
