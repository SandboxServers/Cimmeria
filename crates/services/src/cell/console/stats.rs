//! Granular per-domain stat readouts (category F): `.stats`,
//! `.primarystats`, `.speedstats`, `.armorstats`, `.qrstats`,
//! `.absorbstats`, `.stealthstats` — each dumps a fixed list of stats of the
//! selected target (a `Being`) via the feedback channel, one `label: cur/max`
//! line per stat. These are the granular views the umbrella `/gmprintstats`
//! doesn't break out.
//!
//! Plus one setter (category K): `.speed` ([`set_speed`], P47) — the family's
//! only mutation, kept in this file rather than a new sibling since it's a
//! single command sharing `MOVEMENT_SPEED_MOD`/`ROTATION_SPEED_MOD` with the
//! `speedstats` readout above.
//!
//! Legacy reference: `deprecated/python/cell/commands/Entity.py`
//! (`entityStats`, `entityPrimaryStats` … `entityStealthStats`, `setSpeed`).

use cimmeria_entity::stats::Stat;
use cimmeria_entity::stats::{
    ABSORB_ENERGY, ABSORB_ENERGY_ENERGY, ABSORB_ENERGY_ITEM, ABSORB_HAZMAT, ABSORB_HAZMAT_ENERGY,
    ABSORB_HAZMAT_ITEM, ABSORB_PHYSICAL, ABSORB_PHYSICAL_ENERGY, ABSORB_PHYSICAL_ITEM,
    ABSORB_PSIONIC, ABSORB_PSIONIC_ENERGY, ABSORB_PSIONIC_ITEM, ABSORB_UNTYPED,
    ABSORB_UNTYPED_ENERGY, ABSORB_UNTYPED_ITEM, ACCURACY, AWARENESS, COORDINATION, COVER_ACCURACY,
    COVER_DEFENSE, COVER_QR_MODIFIER, CROUCHING_ACCURACY, CROUCHING_DEFENSE, DAMAGE, DEFENSE,
    DISGUISE_DETECTION, DISGUISE_RATING, ENERGY_AF, ENGAGEMENT, FOCUS, FOCUS_REGEN, FORTITUDE,
    HAZMAT_AF, HEALTH, HEALTH_REGEN, HEALTH_RES, INTELLIGENCE, INTERRUPT_RES, KINETIC_RES,
    MENTAL_RES, MITIGATION, MORALE, MOVEMENT_SPEED_MOD, NEGATION, PENETRATION, PERCEPTION,
    PHYSICAL_AF, PSIONIC_AF, QR_MOD, RECOVERY, RESPONSE, RESTORATION, REVEAL_RATING,
    ROTATION_SPEED_MOD, SPEED_ATTACK, SPEED_DEPLOY, SPEED_GRENADE, SPEED_RELOAD, STABILIZATION,
    STEALTH_MOVEMENT, STEALTH_RATING, SUBTLETY, TRACKING,
};
use tokio::sync::mpsc;

use super::send_gm_feedback;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// (display label, stat id) lists, mirroring the legacy `statIds` arrays.
fn stat_set(cmd: &str) -> &'static [(&'static str, i32)] {
    match cmd {
        "stats" => &[
            ("health", HEALTH),
            ("focus", FOCUS),
            ("healthRegen", HEALTH_REGEN),
            ("focusRegen", FOCUS_REGEN),
        ],
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

/// Dump the stat set named by `cmd` for `target`. `dispatch::resolve_target`
/// already guarantees a resolved target for every command routed here (all
/// carry `Target::Being`, which has no fallback-to-self path), so this takes
/// `target: u32` directly rather than an `Option` with a dead "use caller"
/// branch.
pub(super) async fn show(
    cmd: &str,
    caller_id: u32,
    target: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(entity) = space_mgr.get_entity(target) else {
        send_gm_feedback(caller_id, &format!("{cmd}: no entity."), tx).await;
        return;
    };
    send_gm_feedback(caller_id, &format!("{cmd} [{target}]:",), tx).await;
    for (label, stat_id) in stat_set(cmd) {
        let line = format_stat_line(label, entity.stats.get(*stat_id));
        send_gm_feedback(caller_id, &line, tx).await;
    }
}

/// Render one `label: cur/max` feedback line, or `label: n/a` when the stat
/// is absent from the target's stat block. Split out from [`show`] so the
/// absent-stat branch is directly unit-testable: every stat id used by every
/// group in [`stat_set`] is unconditionally present in
/// `StatList::new()` (see `crates/entity/src/stats/stat_list.rs`), and
/// `StatList` exposes no public way to remove an entry — so a real `CellEntity`
/// fixture can never actually exercise the `None` arm end-to-end today. This
/// still tests the exact formatting code that would run if that ever changed.
fn format_stat_line(label: &str, stat: Option<&Stat>) -> String {
    match stat {
        Some(s) => format!("    {label}: {}/{}", s.cur, s.max),
        None => format!("    {label}: n/a"),
    }
}

/// The two stats `.speed` writes together, in legacy `setSpeed` order. Named
/// so the rejection path can say *which* stat a value is out of range for,
/// and kept separate from [`stat_set`]'s `"speedstats"` group because that
/// group also carries the four read-only action-speed stats (`speedReload`
/// …`speedAttack`) that `.speed` must not touch.
const SPEED_STATS: &[(&str, i32)] = &[
    ("movementSpeedMod", MOVEMENT_SPEED_MOD),
    ("rotationSpeedMod", ROTATION_SPEED_MOD),
];

/// `.speed <value>` — set the target's current `movementSpeedMod` **and**
/// `rotationSpeedMod` together, then publish the change so both the client
/// and the server-side NPC movement tick actually apply it.
///
/// Legacy `setSpeed` (`deprecated/python/cell/commands/Entity.py:537-548`)
/// calls `setCurrent(speed)` on both stats — *current* only, never `max` —
/// then `sendDirtyStats()`, then feeds the caller back. `entities/defs/
/// alias.xml` documents both aliases as "multiplies movement/rotation speed
/// by curr/100", so 100 is normal, 200 double, 0 frozen.
///
/// Two deliberate deviations from legacy, both recorded in the P47 handoff:
///
/// 1. **Reject instead of silently clamping.** Legacy leans on
///    `Stat.setCurrent`'s clamp into `[min, max]`, so `.speed 9000` quietly
///    became 500 and `.speed -5` quietly 0 while the feedback still echoed
///    the typed value. Here an out-of-range value is rejected and *neither*
///    stat is touched — matching the native `gmSetHealth` handler's
///    reject-don't-clamp precedent, and keeping the two-stat write atomic so
///    a GM can never end up with movement clamped one way and rotation
///    another should the two stats' ranges ever diverge.
/// 2. **Integer feedback.** Legacy's format string is `'Set speed of entity
///    %d to %f'`, which renders an int stat as `100.000000`. The prose is
///    preserved; the bogus float rendering is not (same call as P05/P26 made
///    on feedback wording — D02 pins spelling/arity/target semantics, not
///    verbatim legacy prose).
pub(super) async fn set_speed(
    caller_id: u32,
    target: u32,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(speed) = super::parse_i32(caller_id, args, 0, "speed", tx).await else {
        return;
    };

    // Validate against BOTH stats' live bounds *before* mutating either, so a
    // rejected value leaves the target completely unchanged (acceptance
    // criterion: no mutation on bad input).
    let Some(entity) = space_mgr.get_entity(target) else {
        send_gm_feedback(caller_id, "speed: no entity.", tx).await;
        return;
    };
    let is_player = entity.is_player;
    for (label, stat_id) in SPEED_STATS {
        let Some(stat) = entity.stats.get(*stat_id) else {
            send_gm_feedback(
                caller_id,
                &format!("speed: target has no {label} stat."),
                tx,
            )
            .await;
            return;
        };
        if speed < stat.min || speed > stat.max {
            send_gm_feedback(
                caller_id,
                &format!(
                    "speed: {label} must be between {} and {} (100 = normal)",
                    stat.min, stat.max
                ),
                tx,
            )
            .await;
            return;
        }
    }

    // Every check passed — apply both, then publish one `onStatUpdate`
    // carrying the whole dirty set (legacy's `sendDirtyStats()`). Both ids are
    // in `PUBLIC_STATS`, so the NPC branch's `serialize_dirty_public` carries
    // them to witnesses just as the player branch's `serialize_dirty` carries
    // them to the owning client.
    let payload = {
        let entity = space_mgr
            .get_entity_mut(target)
            .expect("target existence checked immediately above");
        for (_, stat_id) in SPEED_STATS {
            entity
                .stats
                .get_mut(*stat_id)
                .expect("stat presence checked immediately above")
                .set_current(speed);
        }
        let p = if is_player {
            entity.stats.serialize_dirty()
        } else {
            entity.stats.serialize_dirty_public()
        };
        entity.stats.clear_dirty();
        p
    };

    tracing::info!(
        caller_id,
        target,
        speed,
        is_player,
        "console .speed: movement/rotation speed mod applied"
    );
    if !payload.is_empty() {
        crate::cell::abilities::send_entity_method(
            target,
            crate::mercury::method_idx::ON_STAT_UPDATE,
            payload,
            tx,
            space_mgr,
        )
        .await;
    }
    // D03: feedback goes to the calling GM, which for `Target::Being` is
    // usually not the entity whose speed just changed.
    send_gm_feedback(
        caller_id,
        &format!("Set speed of entity {target} to {speed}"),
        tx,
    )
    .await;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression guard for the missing-stat branch: `format_stat_line` must
    /// report `n/a`, not a fabricated `0/0` or a panic, when the target's
    /// stat block has no entry for the requested id. See `format_stat_line`'s
    /// doc comment for why this is tested at the formatting-function level
    /// rather than via a full `CellEntity` fixture.
    #[test]
    fn legacy_p03_format_stat_line_reports_na_when_absent() {
        assert_eq!(format_stat_line("foo", None), "    foo: n/a");
    }

    #[test]
    fn legacy_p03_format_stat_line_reports_cur_over_max_when_present() {
        let stat = Stat::new(0, 42, 100, 0, 42, 100);
        assert_eq!(format_stat_line("foo", Some(&stat)), "    foo: 42/100");
    }
}
