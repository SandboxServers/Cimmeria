//! Ability warmup: the gap between `Ability_Begin` and the cast firing (AT-10).
//!
//! The 2009 flow (`deprecated/python/cell/AbilityManager.py`,
//! `AbilityInstance.launch` / `afterWarmup` / `interrupt`) split a cast in
//! two. `launch` charged the cooldown, played `Ability_Begin`, sent an
//! `AbilityWarmup` timer and armed a warmup timer; `afterWarmup` consumed the
//! ammo, played `Ability_End` and applied the effects when that timer
//! expired. Before AT-10 the Rust path sent `Ability_Begin`, then
//! `Ability_End`, and dealt the damage in the same pass, so a charged
//! ability hit the moment the charge started.
//!
//! Submodules:
//! - this file: the warmup length ([`effective_warmup`]) and the launch
//!   side ([`begin_warmup`]), which parks a [`PendingCast`] on the caster.
//! - `tick`: the per-100 ms sweep that interrupts a moving caster and
//!   fires, or interrupts, each cast whose warmup has expired.
//! - `interrupt`: the one cancel primitive and its reasons.
//!
//! Players and NPCs share all of it: the NPC fight tick launches through
//! the same `handle_use_ability` and its casts park here too.

mod interrupt;
mod pet_order;
mod tick;

use tokio::sync::mpsc;

use cimmeria_entity::abilities::{
    serialize_timer_update, AbilityDef, AF_SPEED_ATTACK, AF_SPEED_DEPLOY, AF_SPEED_GRENADE,
    AF_SPEED_PET, TIMER_ABILITY_WARMUP,
};
use cimmeria_entity::cell_entity::{CellEntity, PendingCast};
use cimmeria_entity::stats::{SPEED_ATTACK, SPEED_DEPLOY, SPEED_GRENADE, SPEED_PET};

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::mercury::game_clock;

use super::super::messaging::send_entity_method;
use super::sequence::{play_ability_sequence, AbilityPhase, PhaseSequence};

pub use interrupt::interrupt_unlearned_cast;
pub(crate) use interrupt::{interrupt_pending_cast, InterruptReason};
#[cfg(test)]
pub(crate) use tick::resolve_warmups;
pub use tick::warmup_tick;

/// The warmup of `ability` for `caster`, in seconds, after the speed stats.
///
/// Python `launch()` (`AbilityManager.py:589-598`): each of the
/// `SpeedGrenade`, `SpeedDeploy` and `SpeedAttack` ability flags scales the
/// warmup by `1 - stat / 100` of the matching `speedGrenade` / `speedDeploy`
/// / `speedAttack` stat. The stats default to 0, so a caster with no bonus
/// warms up for exactly the seeded time. Never negative.
///
/// `SpeedPet` / `speedPet` follows the same rule (pets D-PT10). Python had no
/// `SpeedPet` arm because it had no summon; the client's flag and stat names
/// pair it with the other three.
pub(crate) fn effective_warmup(ability: Option<&AbilityDef>, caster: &CellEntity) -> f32 {
    let Some(def) = ability else {
        return 0.0;
    };
    let mut warmup = def.warmup;
    if warmup <= 0.0 {
        return 0.0;
    }
    for (flag, stat) in [
        (AF_SPEED_GRENADE, SPEED_GRENADE),
        (AF_SPEED_DEPLOY, SPEED_DEPLOY),
        (AF_SPEED_ATTACK, SPEED_ATTACK),
        (AF_SPEED_PET, SPEED_PET),
    ] {
        if def.flags & flag != 0 {
            let cur = caster.stats.get(stat).map_or(0, |s| s.cur);
            warmup *= 1.0 - cur as f32 / 100.0;
        }
    }
    warmup.max(0.0)
}

/// `instance_id` of the weapon in `caster`'s active bandolier slot.
pub(super) fn active_weapon_instance(caster: &CellEntity) -> Option<i32> {
    caster
        .bandolier_items
        .get(&caster.active_bandolier_slot)
        .map(|item| item.instance_id)
}

/// True when `entity_id` has a cast in its warmup.
pub(crate) fn is_casting(space_mgr: &SpaceManager, entity_id: u32) -> bool {
    space_mgr
        .get_entity(entity_id)
        .is_some_and(|e| e.pending_cast.is_some())
}

/// Record the ground point of a `useAbilityOnGroundTarget` cast whose
/// primary just committed into its warmup, so the AoE secondaries are
/// collected when it fires. Returns false (and changes nothing) when the
/// caster has no cast warming up, i.e. the primary fired at once.
///
/// Called straight after a committed launch, and a launch is refused while
/// another cast warms up, so any parked cast is this one. It is not matched
/// by ability id: the launch may have redirected the id the client sent
/// (592 to the active weapon's ranged ability), and a mismatch here used to
/// apply the splash damage at launch while the primary was still warming up.
pub(crate) fn attach_ground_point(
    space_mgr: &mut SpaceManager,
    entity_id: u32,
    ground: [f32; 3],
) -> bool {
    match space_mgr
        .get_entity_mut(entity_id)
        .and_then(|e| e.pending_cast.as_mut())
    {
        Some(pc) => {
            pc.ground = Some(ground);
            true
        }
        None => false,
    }
}

/// What the launch hands to [`begin_warmup`].
#[derive(Debug, Clone, Copy)]
pub(super) struct WarmupStart {
    pub ability_id: i32,
    pub target_id: i32,
    pub effect_seq: i32,
    pub warmup_secs: f32,
    pub event_set_id: Option<i32>,
}

/// Start a committed cast's warmup: park it on the caster, play
/// `Ability_Begin`, and send the caster its `AbilityWarmup` timer.
///
/// The order is python's `launch()` order: the cooldown timer (already sent
/// by the caller), then `Ability_Begin`, then the warmup timer. Like python
/// it sends the warmup timer only to a player (`if ent.client is not None`);
/// `Ability_Begin` goes to the caster and its witnesses like every
/// `onSequence`.
pub(super) async fn begin_warmup(
    entity_id: u32,
    start: WarmupStart,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let WarmupStart {
        ability_id,
        target_id,
        effect_seq,
        warmup_secs,
        event_set_id,
    } = start;

    let Some(caster) = space_mgr.get_entity_mut(entity_id) else {
        return;
    };
    let is_player = caster.is_player;
    let weapon_instance = active_weapon_instance(caster);
    caster.pending_cast = Some(PendingCast {
        ability_id,
        target_id,
        ground: None,
        effect_seq,
        fire_at: std::time::Instant::now() + std::time::Duration::from_secs_f32(warmup_secs),
        warmup_secs,
        anchor: caster.position,
        space_id: caster.space_id,
        weapon_instance,
    });
    space_mgr.pending_casts.insert(entity_id);

    tracing::debug!(
        target: "abilities",
        event = "warmup_started",
        entity_id,
        ability_id,
        target_id,
        warmup_secs,
        "ability warmup started; the cast fires when it expires"
    );

    // Send Ability_Begin (event_id 1000), the warmup animation, to the
    // caster and its witnesses.
    play_ability_sequence(
        PhaseSequence {
            phase: AbilityPhase::Begin,
            entity_id,
            ability_id,
            target_id,
            instance_id: effect_seq,
            event_set_id,
        },
        tx,
        space_mgr,
    )
    .await;

    if is_player {
        let timer_args = serialize_timer_update(
            ability_id,
            TIMER_ABILITY_WARMUP,
            entity_id as i32,
            0,
            warmup_secs,
            // Absolute, on the client's game clock, like the cooldown timer.
            game_clock::game_time_secs() + warmup_secs,
        );
        send_entity_method(entity_id, 12, timer_args, tx, space_mgr).await; // 12 = onTimerUpdate
    }
}
