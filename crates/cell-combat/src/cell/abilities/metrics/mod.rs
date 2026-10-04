//! The ability metrics (ability-mechanics AB-T6).
//!
//! | Metric | Kind | Labels |
//! |---|---|---|
//! | `abilities_cast_total` | counter | `outcome` ([`CastOutcome`]), `caster` ([`CasterKind`]), `world` |
//! | `abilities_refused_total` | counter | `reason` ([`RefusalReason`]), `caster`, `world` |
//! | `abilities_effect_applied_total` | counter | `path` ([`EffectPath`]), `world` |
//! | `abilities_ledger_removed_total` | counter | `reason` (`StatBuffRemoval`), `world` |
//! | `abilities_qr_total` | counter | `result` ([`QrOutcome`]), `world` |
//! | `abilities_wire_send_failed_total` | counter | `message` ([`WireMessage`]), `world` |
//! | `abilities_press_to_fire_ms` | histogram | `path` ([`FirePath`]), `caster`, `world` |
//! | `abilities_damage_dealt` | histogram | `pool` (`Pool`), `world` |
//! | `abilities_heal_done` | histogram | `pool`, `world` |
//!
//! **One cast, one outcome.** Every `useAbility` that reaches
//! `handle_use_ability` ends in exactly one `abilities_cast_total` sample:
//! `refused` (with its `abilities_refused_total` reason) at the launch,
//! `held` when the holstered-weapon queue defers it (the deferred re-press
//! counts again when it runs), `fired` when it fires (in the launch pass, or
//! from the warmup tick), `interrupted` when its warmup is broken, and
//! `abandoned` when its caster is torn down mid-warmup (a logout, a
//! cross-world teleport, a despawn; the `warmup_abandoned` row names which).
//! A fire-time re-check that fails (range, line of sight, target lost) is an
//! interrupt, not a refusal: the cooldown was charged at the launch.
//!
//! **Press to fire** is measured on the cell from `handle_use_ability`'s
//! entry (the receipt, carried on `PendingCast::received_at` through a
//! warmup) to the fire: near zero for an instant cast; any launch delay,
//! the warmup and the tick's lateness for a warmed one. The client-side
//! timings are AB-C6's.
//!
//! **Labels are enumerated** (Rule 4 of `instrumentation-discipline.md`):
//! each is a [`cimmeria_observability::metric_label`] enum, so a call site
//! cannot invent a value, and `tests.rs` pins each set against the code's
//! own reasons. `world` is bounded by the world table; correlators
//! (`cast_id`, `player_id`) stay on the log rows. Every sample is a no-op
//! when telemetry is off.
//!
//! The damage, heal and ledger metrics are recorded below the combat crate
//! (the effect scripts and the ledger live in `cimmeria-cell-world`); they
//! are re-exported here so one module documents and pins the whole set.

use std::time::Duration;

use cimmeria_entity::abilities::{
    RangeRefusal, RC_CRITICAL, RC_DOUBLE_CRITICAL, RC_GLANCING, RC_HIT, RC_MISS, RC_NONE,
};

pub(crate) use cimmeria_cell_world::cell::effects::ability_metrics::{
    damage_dealt, world_of, Pool, UNKNOWN_WORLD,
};

use cimmeria_entity::cell_entity::PlayerIdentity;

use super::super::space_manager::SpaceManager;

#[cfg(test)]
mod tests;

pub(crate) use cimmeria_cell_world::cell::effects::ability_metrics::CAST_TOTAL;
pub(crate) const REFUSED_TOTAL: &str = "abilities_refused_total";
pub(crate) const EFFECT_APPLIED_TOTAL: &str = "abilities_effect_applied_total";
pub(crate) const QR_TOTAL: &str = "abilities_qr_total";
pub(crate) const WIRE_SEND_FAILED_TOTAL: &str = "abilities_wire_send_failed_total";
pub(crate) const PRESS_TO_FIRE_MS: &str = "abilities_press_to_fire_ms";

cimmeria_observability::metric_label! {
    /// How one cast ended (module docs).
    pub(crate) enum CastOutcome {
        Fired => "fired",
        Interrupted => "interrupted",
        Refused => "refused",
        /// The holstered-weapon queue deferred it until the draw finishes.
        Held => "held",
        /// Its caster was torn down mid-warmup (logout, cross-world
        /// teleport, despawn): `ability_metrics::abandon_pending_cast` in
        /// `cimmeria-cell-world` counts it.
        Abandoned => "abandoned",
    }
}

cimmeria_observability::metric_label! {
    /// Who cast it. A pet is an `npc`.
    pub(crate) enum CasterKind {
        Player => "player",
        Npc => "npc",
    }
}

cimmeria_observability::metric_label! {
    /// Why a launch was refused. The first eleven are `gate_rows`'
    /// `LaunchRefusal` reasons, spelled as their rows spell them.
    pub(crate) enum RefusalReason {
        CasterMissing => "caster_missing",
        CasterDead => "caster_dead",
        AlreadyWarming => "already_warming",
        NotKnown => "not_known",
        UnknownAbilityId => "unknown_ability_id",
        OnCooldown => "on_cooldown",
        TargetDead => "target_dead",
        NoBeneficialTarget => "no_beneficial_target",
        CasterMissingAtCommit => "caster_missing_at_commit",
        ReloadInFlight => "reload_in_flight",
        NoAmmo => "no_ammo",
        /// Stunned or knocked down (AB-09a).
        Incapacitated => "incapacitated",
        /// A known ability with no mechanic yet (AB-12).
        NoMechanics => "no_mechanics",
        /// A shield whose every pool is full (AB-10).
        ShieldFull => "shield_full",
        /// `RangeRefusal::TooFar`.
        TargetOutOfRange => "target_out_of_range",
        /// `RangeRefusal::TooClose` (#1016).
        TargetTooClose => "target_too_close",
        /// An ammo support shot aimed at a hostile (AM-11d).
        SupportHostileTarget => "support_hostile_target",
        /// A beneficial cast aimed at a non-ally after resolution (AB-01).
        BeneficialNonAllyTarget => "beneficial_non_ally_target",
        /// A player's attack at a target they may not attack (#444).
        NonHostileTarget => "non_hostile_target",
        /// A summon the pet rules refuse (pets PT-03).
        SummonRefused => "summon_refused",
        /// An owner ability with no pet to land on (pets PT-08).
        OwnerPetRefused => "owner_pet_refused",
        /// A deployable's ground point, timing or staging (Phase 0).
        DeployableRefused => "deployable_refused",
        /// The target is in another space (#906).
        TargetOtherSpace => "target_other_space",
        /// A wall between the eyes (NA31).
        NoLineOfSight => "no_line_of_sight",
        /// A weapon attack is already queued behind a draw.
        WeaponAttackQueued => "weapon_attack_queued",
        /// A bandolier slot swap is in progress.
        SlotSwapInProgress => "slot_swap_in_progress",
    }
}

impl RefusalReason {
    pub(crate) fn from_range(refusal: RangeRefusal) -> Self {
        match refusal {
            RangeRefusal::TooFar => Self::TargetOutOfRange,
            RangeRefusal::TooClose => Self::TargetTooClose,
        }
    }
}

cimmeria_observability::metric_label! {
    /// `effect_planned`'s `path` (AB-T3, `effect_plan`).
    pub(crate) enum EffectPath {
        Script => "script",
        Nvp => "nvp",
        RoutedToUser => "routed_to_user",
        AllyFanout => "ally_fanout",
        Ledger => "ledger",
        Pulse => "pulse",
        Skipped => "skipped",
    }
}

impl EffectPath {
    /// The path named `label`, `None` for a string that is not a path.
    pub(crate) fn from_label(label: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|p| p.label() == label)
    }
}

cimmeria_observability::metric_label! {
    /// A hit's QR result (`qr_rolled`'s `result`).
    pub(crate) enum QrOutcome {
        None => "none",
        Hit => "hit",
        Miss => "miss",
        Critical => "critical",
        DoubleCritical => "double_critical",
        Glancing => "glancing",
        Unknown => "unknown",
    }
}

impl QrOutcome {
    /// The client's `EResultCode`, as a label.
    pub(crate) fn from_code(result_code: u8) -> Self {
        match result_code {
            RC_NONE => Self::None,
            RC_HIT => Self::Hit,
            RC_MISS => Self::Miss,
            RC_CRITICAL => Self::Critical,
            RC_DOUBLE_CRITICAL => Self::DoubleCritical,
            RC_GLANCING => Self::Glancing,
            _ => Self::Unknown,
        }
    }
}

cimmeria_observability::metric_label! {
    /// The message a failed send carried: the `method` of its WARN row.
    /// The first fourteen are `wire_ledger::method_name`'s values.
    pub(crate) enum WireMessage {
        OnSequence => "onSequence",
        OnTimerUpdate => "onTimerUpdate",
        OnEffectResults => "onEffectResults",
        OnStateFieldUpdate => "onStateFieldUpdate",
        OnStatUpdate => "onStatUpdate",
        OnStatBaseUpdate => "onStatBaseUpdate",
        OnKnownAbilitiesUpdate => "onKnownAbilitiesUpdate",
        OnAbilityTreeInfo => "onAbilityTreeInfo",
        OnErrorCode => "onErrorCode",
        OnPlayerCommunication => "onPlayerCommunication",
        OnTargetUpdate => "onTargetUpdate",
        InteractionType => "InteractionType",
        OnBeginAidWait => "onBeginAidWait",
        Other => "other",
        /// Cell-to-base, not a client method: the weapon draw or holster.
        RefreshAppearance => "RefreshAppearance",
        /// Cell-to-base, not a client method: the contact list's "has died".
        ContactListPresenceEvent => "ContactListPresenceEvent",
    }
}

impl WireMessage {
    /// The client method at `method_index`, as `wire_ledger` names it.
    pub(crate) fn from_method(method_index: u16) -> Self {
        let name = super::wire_ledger::method_name(method_index);
        Self::ALL
            .iter()
            .copied()
            .find(|m| m.label() == name)
            .unwrap_or(Self::Other)
    }
}

cimmeria_observability::metric_label! {
    /// Whether a cast fired in its launch pass or after a warmup.
    pub(crate) enum FirePath {
        Instant => "instant",
        Warmup => "warmup",
    }
}

/// `entity_id`'s caster kind. A missing entity counts as an NPC.
pub(crate) fn caster_kind(space_mgr: &SpaceManager, entity_id: u32) -> CasterKind {
    if space_mgr.get_entity(entity_id).is_some_and(|e| e.is_player) {
        CasterKind::Player
    } else {
        CasterKind::Npc
    }
}

/// Count one cast's outcome.
pub(crate) fn cast(outcome: CastOutcome, caster: CasterKind, world: &'static str) {
    cimmeria_observability::counter!(
        CAST_TOTAL,
        "outcome" => outcome.label(),
        "caster" => caster.label(),
        "world" => world,
    );
}

/// [`cast`] for `entity_id`.
pub(crate) fn cast_in(space_mgr: &SpaceManager, entity_id: u32, outcome: CastOutcome) {
    cast(
        outcome,
        caster_kind(space_mgr, entity_id),
        world_of(space_mgr, entity_id),
    );
}

/// `event` of the one row every `abilities_refused_total` sample writes
/// (target `abilities`). Its `reason` is the metric's label, so the
/// **Abilities — Refusals by reason** view selects exactly the metric's
/// population. The refusing module's own row (`use_ability_on_cooldown`,
/// `cast_refused`, `deploy_refused`, …) keeps the detail.
pub(crate) const EVENT_ABILITY_REFUSED: &str = "ability_refused";

/// Who a refusal is about, for its `ability_refused` row.
#[derive(Debug, Clone, Copy)]
pub(crate) struct RefusedCast {
    pub entity_id: u32,
    pub entity_name: Option<&'static str>,
    pub ability_id: i32,
    pub who: PlayerIdentity,
    pub caster: CasterKind,
    pub world: &'static str,
}

impl RefusedCast {
    /// `entity_id`'s refusal of `ability_id`.
    pub(crate) fn of(space_mgr: &SpaceManager, entity_id: u32, ability_id: i32) -> Self {
        Self {
            entity_id,
            entity_name: space_mgr.entity_names(entity_id).entity_name,
            ability_id,
            who: space_mgr.player_identity(entity_id),
            caster: caster_kind(space_mgr, entity_id),
            world: world_of(space_mgr, entity_id),
        }
    }
}

/// Count one launch refusal and its `refused` cast outcome, and write its
/// `ability_refused` row. The only place `abilities_refused_total` is
/// counted, so the row and the metric cannot drift apart.
pub(crate) fn refused(reason: RefusalReason, cast_ids: RefusedCast) {
    let RefusedCast {
        entity_id,
        entity_name,
        ability_id,
        who,
        caster,
        world,
    } = cast_ids;
    tracing::debug!(
        target: "abilities",
        event = EVENT_ABILITY_REFUSED,
        stage = "gate",
        reason = reason.label(),
        caster = caster.label(),
        world,
        account_id = who.account_id,
        account_name = who.account_name,
        player_id = who.player_id,
        player_name = who.player_name,
        entity_id,
        entity_name,
        ability_id,
        ability_name = cimmeria_names::book().ability(ability_id),
        "ability launch refused (one row per abilities_refused_total sample)"
    );
    cimmeria_observability::counter!(
        REFUSED_TOTAL,
        "reason" => reason.label(),
        "caster" => caster.label(),
        "world" => world,
    );
    cast(CastOutcome::Refused, caster, world);
}

/// [`refused`] for `entity_id`'s press of `ability_id`.
pub(crate) fn refused_in(
    space_mgr: &SpaceManager,
    entity_id: u32,
    ability_id: i32,
    reason: RefusalReason,
) {
    refused(reason, RefusedCast::of(space_mgr, entity_id, ability_id));
}

/// Count one `effect_planned` row's path.
pub(crate) fn effect_applied(path: EffectPath, world: &'static str) {
    cimmeria_observability::counter!(
        EFFECT_APPLIED_TOTAL,
        "path" => path.label(),
        "world" => world,
    );
}

/// Count one hit's QR result.
pub(crate) fn qr(result_code: u8, world: &'static str) {
    cimmeria_observability::counter!(
        QR_TOTAL,
        "result" => QrOutcome::from_code(result_code).label(),
        "world" => world,
    );
}

/// Count one failed send (its WARN row is the caller's).
pub(crate) fn wire_send_failed(message: WireMessage, world: &'static str) {
    cimmeria_observability::counter!(
        WIRE_SEND_FAILED_TOTAL,
        "message" => message.label(),
        "world" => world,
    );
}

/// [`wire_send_failed`] about `entity_id`.
pub(crate) fn wire_send_failed_in(space_mgr: &SpaceManager, entity_id: u32, message: WireMessage) {
    wire_send_failed(message, world_of(space_mgr, entity_id));
}

/// Record a fire `elapsed` after its press reached the cell, and count the
/// `fired` outcome.
pub(crate) fn fired(path: FirePath, elapsed: Duration, caster: CasterKind, world: &'static str) {
    cimmeria_observability::histogram!(
        PRESS_TO_FIRE_MS,
        elapsed.as_secs_f64() * 1000.0,
        "path" => path.label(),
        "caster" => caster.label(),
        "world" => world,
    );
    cast(CastOutcome::Fired, caster, world);
}
