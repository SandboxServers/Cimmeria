//! `.dummy caster <abilityId> [intervalSecs]` — a lab dummy that casts one
//! ability at its owner (ability-mechanics AB-L2, for UAT rows AB-U20 and
//! AB-U22).
//!
//! The plain dummy ([`super::dummy`]) never casts, so nothing seeded could
//! stage "interrupt a warmup" or "cleanse a debuff an NPC put on you". A
//! caster dummy is a hostile plain dummy (same template, Health, cap,
//! ownership, ten-minute expiry, logout despawn and combat release) with a
//! second mark, [`LabCaster`]:
//!
//! - It still gets **no AI turn**: no target selection, threat, movement,
//!   chase or leash. The [`cast_due`] sweep, run from the 1 Hz
//!   `lab_dummy_tick`, is the only thing that makes it act.
//! - Every interval (default [`LAB_CASTER_DEFAULT_INTERVAL`]) it turns to its
//!   owner and launches the ability at them through `handle_use_ability`,
//!   the launch the NPC fight tick uses. Warmups, the AT-10 interrupts
//!   (a stun, Interrupting Shot), cooldowns, effects, the wire sequences and
//!   the `abilities` telemetry are the real ones.
//! - It **holds** while its owner is dead, gone, or in another space, and
//!   while its own previous cast is still warming up. The schedule advances
//!   by one interval per attempt either way, so it never bursts on resume.
//!
//! Placement refuses, with a feedback line and nothing spawned: an unknown or
//! passive ability, a beneficial one (it would land on the dummy, never the
//! owner), one whose range cannot reach the owner 3 m away, and an interval
//! shorter than the ability's cooldown or not longer than its warmup (every
//! other attempt would be refused as on cooldown or still warming).

use std::time::{Duration, Instant};

use cimmeria_entity::abilities::{ability_is_beneficial, ability_range_bounds};
use cimmeria_entity::cell_entity::MobAggression;
use cimmeria_entity::stats::HEALTH;
use tokio::sync::mpsc;

use super::dummy::{spawn_dummy, Placed, DEFAULT_DUMMY_TEMPLATE, PLACE_DISTANCE};
use crate::cell::combat::is_dead_state;
use crate::cell::console::send_gm_feedback;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::{
    LabCaster, LabDummy, SpaceManager, LAB_CASTER_DEFAULT_INTERVAL, LAB_DUMMY_LIFETIME,
};

const USAGE: &str = ".dummy caster: usage .dummy caster <abilityId> [intervalSecs]";

/// The longest interval. The first cast comes one interval after placement
/// and the next ones on that schedule, so 290 s fits two casts (at 290 s and
/// 580 s) before the ten-minute expiry, with 20 s to spare for the 1 Hz
/// sweep. A 600 s interval would never have cast at all.
pub(crate) const MAX_INTERVAL_SECS: u64 = (LAB_DUMMY_LIFETIME.as_secs() - 20) / 2;

pub(super) async fn run(
    caller_id: u32,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let (ability_arg, interval_arg) = match args {
        [a] => (*a, None),
        [a, i] => (*a, Some(*i)),
        _ => return send_gm_feedback(caller_id, USAGE, tx).await,
    };
    let ability_id = match ability_arg.parse::<i32>() {
        Ok(id) if id > 0 => id,
        _ => {
            let line =
                format!(".dummy caster: abilityId must be a positive integer (got {ability_arg})");
            return send_gm_feedback(caller_id, &line, tx).await;
        }
    };
    let interval = match interval_arg {
        None => LAB_CASTER_DEFAULT_INTERVAL,
        Some(i) => match i.parse::<u64>() {
            Ok(secs @ 1..=MAX_INTERVAL_SECS) => Duration::from_secs(secs),
            _ => {
                let line = format!(
                    ".dummy caster: intervalSecs must be a whole number from 1 to {MAX_INTERVAL_SECS} (got {i})"
                );
                return send_gm_feedback(caller_id, &line, tx).await;
            }
        },
    };
    let (name, warmup) = match check_ability(space_mgr, ability_id, interval) {
        Ok(ok) => ok,
        Err(why) => {
            tracing::debug!(
                target: "abilities.gm",
                event = "lab_dummy_refused",
                reason = "caster_ability",
                entity_id = caller_id,
                ability_id,
                interval_secs = interval.as_secs(),
                detail = %why,
                "GM .dummy caster refused: the ability cannot be cast at its owner"
            );
            return send_gm_feedback(caller_id, &format!(".dummy caster: {why}"), tx).await;
        }
    };

    let Some(Placed {
        dummy_id,
        name: dummy_name,
        ..
    }) = spawn_dummy(
        caller_id,
        MobAggression::Hostile,
        DEFAULT_DUMMY_TEMPLATE,
        tx,
        space_mgr,
    )
    .await
    else {
        return;
    };
    let Some(dummy) = space_mgr.get_entity_mut(dummy_id) else {
        return;
    };
    // The launch refuses an ability the caster does not know.
    dummy.abilities.add_ability(ability_id);
    dummy.extensions.insert(LabCaster {
        ability_id,
        interval,
        next_cast_at: Instant::now() + interval,
    });
    let owner = space_mgr.player_identity(caller_id);
    tracing::info!(
        target: "abilities.gm",
        event = "lab_caster_placed",
        entity_id = caller_id,
        account_id = owner.account_id,
        player_id = owner.player_id,
        dummy_id,
        ability_id,
        interval_secs = interval.as_secs(),
        warmup_secs = warmup,
        "GM placed a caster lab dummy"
    );
    let line = format!(
        "dummy [{dummy_id}] placed: caster {dummy_name}; casts {name} ({ability_id}, warmup {warmup} s) at you every {} s while you are alive and in this space; gone in {} min, when you log out, or on .dummy clear",
        interval.as_secs(),
        LAB_DUMMY_LIFETIME.as_secs() / 60
    );
    send_gm_feedback(caller_id, &line, tx).await;
}

/// The ability's name and warmup, or why a caster dummy cannot use it.
fn check_ability(
    space_mgr: &SpaceManager,
    ability_id: i32,
    interval: Duration,
) -> Result<(String, f32), String> {
    let Some(def) = space_mgr.ability_defs.get(&ability_id) else {
        return Err(format!("no ability {ability_id}"));
    };
    let name = &def.name;
    if def.passive {
        return Err(format!(
            "{name} ({ability_id}) is passive; it is never cast"
        ));
    }
    if ability_is_beneficial(def, &space_mgr.effect_defs) {
        return Err(format!(
            "{name} ({ability_id}) is beneficial; it lands on its caster or an ally, never on you"
        ));
    }
    // The launch's own bounds for a weaponless NPC (metres, #919): a
    // `UseWeaponRange` ability falls back to its own range, a 0 to the
    // default reach. An NPC is never held to `min_range`.
    let bounds = ability_range_bounds(Some(def), None);
    if bounds.refusal(PLACE_DISTANCE, false).is_some() {
        return Err(format!(
            "{name} ({ability_id}) reaches {} m; the dummy stands {PLACE_DISTANCE} m from you",
            bounds.max
        ));
    }
    let secs = interval.as_secs_f32();
    if secs < def.cooldown || secs <= def.warmup {
        let least = def.cooldown.max(def.warmup.floor() + 1.0).ceil();
        return Err(format!(
            "{name} ({ability_id}) has a {} s cooldown and a {} s warmup; use an interval of at least {least} s",
            def.cooldown, def.warmup
        ));
    }
    Ok((name.clone(), def.warmup))
}

/// Why a due caster did not cast. The label is the `reason` log field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Hold {
    OwnerGone,
    OwnerDead,
    OwnerElsewhere,
    StillCasting,
}

impl Hold {
    fn as_str(self) -> &'static str {
        match self {
            Self::OwnerGone => "owner_gone",
            Self::OwnerDead => "owner_dead",
            Self::OwnerElsewhere => "owner_in_another_space",
            Self::StillCasting => "still_casting",
        }
    }
}

/// Let every caster dummy whose time has come by `now` cast at its owner.
/// `now` is a parameter so tests can step past an interval without sleeping.
pub(crate) async fn cast_due(
    now: Instant,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    for dummy_id in space_mgr.lab_casters_due(now) {
        let Some(dummy) = space_mgr.get_entity_mut(dummy_id) else {
            continue;
        };
        let (Some(mark), Some(caster)) = (
            dummy.extensions.get::<LabDummy>().copied(),
            dummy.extensions.get_mut::<LabCaster>(),
        ) else {
            continue;
        };
        // One attempt per interval, cast or held, on the placement schedule
        // (the 1 Hz sweep's lateness does not accumulate). A schedule left
        // behind (a stalled loop) restarts from now instead of bursting.
        let next = caster.next_cast_at + caster.interval;
        caster.next_cast_at = if next > now {
            next
        } else {
            now + caster.interval
        };
        let ability_id = caster.ability_id;
        let owner_id = mark.owner_id;

        if let Some(hold) = hold_reason(space_mgr, dummy_id, owner_id) {
            tracing::debug!(
                target: "abilities.gm",
                event = "lab_caster_held",
                reason = hold.as_str(),
                entity_id = owner_id,
                account_id = mark.owner_identity.account_id,
                player_id = mark.owner_identity.player_id,
                dummy_id,
                ability_id,
                "caster lab dummy held its cast"
            );
            continue;
        }
        face_owner(space_mgr, dummy_id, owner_id);
        let launched = crate::cell::abilities::handle_use_ability(
            dummy_id,
            ability_id,
            owner_id as i32,
            tx,
            space_mgr,
        )
        .await;
        // The launch logs its own rows (`combat.use_ability`, the gate rows,
        // the warmup and fire); this one ties them to the lab dummy.
        tracing::info!(
            target: "abilities.gm",
            event = "lab_caster_cast",
            entity_id = owner_id,
            account_id = mark.owner_identity.account_id,
            player_id = mark.owner_identity.player_id,
            dummy_id,
            ability_id,
            launched,
            "caster lab dummy cast at its owner"
        );
    }
}

fn hold_reason(space_mgr: &SpaceManager, dummy_id: u32, owner_id: u32) -> Option<Hold> {
    let dummy = space_mgr.get_entity(dummy_id)?;
    let Some(owner) = space_mgr.get_entity(owner_id) else {
        return Some(Hold::OwnerGone);
    };
    let zeroed = owner.stats.get(HEALTH).is_some_and(|h| h.cur <= 0);
    if is_dead_state(owner.state_field) || zeroed {
        return Some(Hold::OwnerDead);
    }
    if owner.space_id != dummy.space_id {
        return Some(Hold::OwnerElsewhere);
    }
    if dummy.pending_cast.is_some() {
        return Some(Hold::StillCasting);
    }
    None
}

/// Turn the dummy to its owner, as the fight tick faces its target: yaw is
/// `atan2(dx, dz)`, and a coincident owner keeps the current yaw.
fn face_owner(space_mgr: &mut SpaceManager, dummy_id: u32, owner_id: u32) {
    let (Some(d), Some(o)) = (
        space_mgr.get_entity(dummy_id).map(|e| e.position),
        space_mgr.get_entity(owner_id).map(|e| e.position),
    ) else {
        return;
    };
    let (dx, dz) = (o.x - d.x, o.z - d.z);
    if dx * dx + dz * dz < f32::EPSILON {
        return;
    }
    if let Some(dummy) = space_mgr.get_entity_mut(dummy_id) {
        dummy.direction = cimmeria_common::Vector3::new(0.0, dx.atan2(dz), 0.0);
    }
}
