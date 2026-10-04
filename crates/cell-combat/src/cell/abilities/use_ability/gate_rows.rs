//! The launch's refusal rows (ability-mechanics AB-T2).
//!
//! Every way `handle_use_ability` returns `false` without an answer of its
//! own (a refusal that sends `onErrorCode` or a feedback line logs in its
//! own module) lands one row here, under the `abilities` target with a
//! stable `event`, so a press that did nothing still has a row saying why.
//! The rows sit here rather than inline so `handle.rs` stays under the file
//! cap; each call site is one line naming the [`LaunchRefusal`].
//!
//! Level discipline (negative-logging convention): WARN where the server is
//! wrong (a known ability the caster cannot use, an entity gone mid-launch),
//! DEBUG where the client asked for something the rules refuse, because the
//! client controls the input and could flood a WARN index.

use cimmeria_entity::cell_entity::PlayerIdentity;

use super::super::metrics::{self, CasterKind, RefusalReason};
use crate::cell::space_manager::SpaceManager;

/// Why a launch stopped. Each variant is one `event`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LaunchRefusal {
    /// No caster entity at the first gate.
    CasterMissing,
    /// The caster is dead.
    CasterDead,
    /// Another cast of the caster's is in its warmup.
    AlreadyWarming { ability_id: i32, cast_id: i32 },
    /// A server-known ability the caster neither knows nor holds a weapon
    /// for.
    NotKnown,
    /// An ability id the server has no definition for.
    UnknownAbilityId,
    /// The ability is cooling down.
    OnCooldown,
    /// The named target is dead.
    TargetDead,
    /// A beneficial cast found neither an ally nor the caster to land on.
    NoBeneficialTarget,
    /// The caster passed every gate but was gone at the commit.
    CasterVanished,
    /// A reload is in flight.
    Reloading,
    /// The active slot holds fewer rounds than the ability needs.
    NoAmmo { current: i32, required: i32 },
}

impl From<LaunchRefusal> for RefusalReason {
    /// The metric reason: the row's `reason`, which `metrics::tests` pins.
    fn from(why: LaunchRefusal) -> Self {
        match why {
            LaunchRefusal::CasterMissing => Self::CasterMissing,
            LaunchRefusal::CasterDead => Self::CasterDead,
            LaunchRefusal::AlreadyWarming { .. } => Self::AlreadyWarming,
            LaunchRefusal::NotKnown => Self::NotKnown,
            LaunchRefusal::UnknownAbilityId => Self::UnknownAbilityId,
            LaunchRefusal::OnCooldown => Self::OnCooldown,
            LaunchRefusal::TargetDead => Self::TargetDead,
            LaunchRefusal::NoBeneficialTarget => Self::NoBeneficialTarget,
            LaunchRefusal::CasterVanished => Self::CasterMissingAtCommit,
            LaunchRefusal::Reloading => Self::ReloadInFlight,
            LaunchRefusal::NoAmmo { .. } => Self::NoAmmo,
        }
    }
}

struct Shape {
    event: &'static str,
    stage: &'static str,
    reason: &'static str,
    warn: bool,
    /// System, expected vs happened, and what the player sees.
    message: &'static str,
}

impl LaunchRefusal {
    /// The row's `reason` (for `metrics::tests`).
    #[cfg(test)]
    pub(super) fn reason(self) -> &'static str {
        self.shape().reason
    }

    /// One of each variant (for `metrics::tests`).
    #[cfg(test)]
    pub(super) const ALL: [Self; 11] = [
        Self::CasterMissing,
        Self::CasterDead,
        Self::AlreadyWarming {
            ability_id: 0,
            cast_id: 0,
        },
        Self::NotKnown,
        Self::UnknownAbilityId,
        Self::OnCooldown,
        Self::TargetDead,
        Self::NoBeneficialTarget,
        Self::CasterVanished,
        Self::Reloading,
        Self::NoAmmo {
            current: 0,
            required: 0,
        },
    ];

    fn shape(self) -> Shape {
        let (event, stage, reason, warn, message) = match self {
            Self::CasterMissing => (
                "use_ability_caster_missing",
                "gate",
                "caster_missing",
                true,
                "useAbility gate: expected the caster in its space, found no entity; \
                 the press is dropped and the player sees nothing",
            ),
            Self::CasterDead => (
                "use_ability_caster_dead",
                "gate",
                "caster_dead",
                false,
                "useAbility gate: expected a living caster, the caster is dead; \
                 the press is dropped and the player sees nothing new",
            ),
            Self::AlreadyWarming { .. } => (
                "use_ability_already_warming",
                "gate",
                "already_warming",
                false,
                "useAbility gate: expected no cast in its warmup, one is warming; \
                 the press is dropped and the warming cast carries on",
            ),
            Self::NotKnown => (
                "use_ability_not_known",
                "gate",
                "not_known",
                true,
                "useAbility gate: expected a known or weapon-granted ability, the caster \
                 has neither; a player gets onErrorCode 167, an NPC's cast is dropped",
            ),
            Self::UnknownAbilityId => (
                "use_ability_unknown_id",
                "gate",
                "unknown_ability_id",
                false,
                "useAbility gate: the server has no definition for this ability id \
                 (forged or stale client); the press is dropped and the player sees nothing",
            ),
            Self::OnCooldown => (
                "use_ability_on_cooldown",
                "gate",
                "on_cooldown",
                false,
                "useAbility gate: expected the ability off cooldown, it is cooling down; \
                 the press is dropped, the client's cooldown timer shows why",
            ),
            Self::TargetDead => (
                "use_ability_target_dead",
                "gate",
                "target_dead",
                false,
                "useAbility gate: expected a living target, the target is dead; \
                 the press is dropped and the player sees nothing",
            ),
            Self::NoBeneficialTarget => (
                "use_ability_no_beneficial_target",
                "gate",
                "no_beneficial_target",
                false,
                "useAbility gate: expected an ally or the caster to land a beneficial \
                 cast on, found neither; the player gets the no-ally feedback line, no cooldown",
            ),
            Self::CasterVanished => (
                "use_ability_caster_vanished",
                "launch",
                "caster_missing_at_commit",
                true,
                "useAbility launch: the caster passed every gate but its entity was gone \
                 at the commit; no cooldown, no cast, the player sees nothing",
            ),
            Self::Reloading => (
                "use_ability_reloading",
                "launch",
                "reload_in_flight",
                false,
                "useAbility launch: expected no reload in flight, one is; the press is \
                 dropped, the reload bar shows why",
            ),
            Self::NoAmmo { .. } => (
                "use_ability_no_ammo",
                "launch",
                "no_ammo",
                false,
                "useAbility launch: expected the ability's rounds in the active slot, \
                 there are fewer; the press is dropped, no cooldown",
            ),
        };
        Shape {
            event,
            stage,
            reason,
            warn,
            message,
        }
    }
}

/// The ids every launch row carries (core fields, AB-T1/AB-T2).
#[derive(Debug, Clone, Copy)]
pub(super) struct LaunchRow<'a> {
    pub who: PlayerIdentity,
    pub entity_id: u32,
    /// The caster's name (Rule 6): a player's is a copy of the interned
    /// character name, an NPC's one NameBook read per press.
    pub entity_name: Option<&'static str>,
    pub ability_id: i32,
    /// `None` for an ability with no definition: the field is left out.
    pub ability_name: Option<&'a str>,
    /// The target the client sent.
    pub wire_target_id: i32,
    pub wire_target_name: Option<&'static str>,
    /// The target the launch resolved (the wire target until it has).
    /// Set it through [`LaunchRow::set_target`] so its name follows.
    pub target_id: i32,
    pub target_name: Option<&'static str>,
    /// The AB-T6 metrics' `caster` and `world` labels.
    pub caster: CasterKind,
    pub world: &'static str,
}

/// The name of the entity a wire target id names; `None` for target 0 or
/// a negative id.
pub(super) fn target_label(space_mgr: &SpaceManager, target_id: i32) -> Option<&'static str> {
    u32::try_from(target_id)
        .ok()
        .and_then(|t| space_mgr.entity_names(t).entity_name)
}

impl LaunchRow<'_> {
    /// The launch resolved its target: record it with its name.
    pub(super) fn set_target(&mut self, space_mgr: &SpaceManager, target_id: i32) {
        self.target_id = target_id;
        self.target_name = target_label(space_mgr, target_id);
    }

    /// `player` or `npc`: the caster's kind, from its identity (an NPC has
    /// no `player_id`).
    fn caster_kind(&self) -> &'static str {
        if self.who.player_id.is_some() {
            "player"
        } else {
            "npc"
        }
    }

    /// Count a refusal whose row and answer are another module's
    /// (`abilities_refused_total`, AB-T6).
    pub(super) fn count(&self, reason: RefusalReason) {
        metrics::refused(
            reason,
            metrics::RefusedCast {
                entity_id: self.entity_id,
                entity_name: self.entity_name,
                ability_id: self.ability_id,
                who: self.who,
                caster: self.caster,
                world: self.world,
            },
        );
    }

    /// Log why the launch stopped, and count it.
    pub(super) fn refused(&self, why: LaunchRefusal) {
        self.count(why.into());
        let Shape {
            event,
            stage,
            reason,
            warn,
            message,
        } = why.shape();
        let (warming_ability_id, warming_cast_id) = match why {
            LaunchRefusal::AlreadyWarming {
                ability_id,
                cast_id,
            } => (Some(ability_id), Some(cast_id)),
            _ => (None, None),
        };
        let (ammo_current, ammo_required) = match why {
            LaunchRefusal::NoAmmo { current, required } => (Some(current), Some(required)),
            _ => (None, None),
        };
        // One field list for both levels: `tracing` fixes the level per
        // callsite, so the two arms are two callsites.
        macro_rules! row {
            ($level:ident) => {
                tracing::$level!(
                    target: "abilities",
                    event,
                    stage,
                    reason,
                    account_id = self.who.account_id,
                    account_name = self.who.account_name,
                    player_id = self.who.player_id,
                    player_name = self.who.player_name,
                    entity_id = self.entity_id,
                    entity_name = self.entity_name,
                    caster_id = self.entity_id,
                    caster_name = self.entity_name,
                    caster_kind = self.caster_kind(),
                    ability_id = self.ability_id,
                    ability_name = self.ability_name,
                    wire_target_id = self.wire_target_id,
                    wire_target_name = self.wire_target_name,
                    target_id = self.target_id,
                    target_name = self.target_name,
                    warming_ability_id,
                    warming_ability_name = cimmeria_cell_world::cell::effects::content_names::ability_name(warming_ability_id),
                    warming_cast_id, // nt:id-only per-cast sequence number, no name exists
                    ammo_current,
                    ammo_required,
                    "{message}"
                )
            };
        }
        if warn {
            row!(warn);
        } else {
            row!(debug);
        }
    }

    /// A manual press of a different ability cleared the armed auto-cycle
    /// loop (python `AbilityManager.useAbility`: `self.autoCycle = False`).
    pub(super) fn auto_cycle_overridden(&self) {
        tracing::info!(
            target: "abilities",
            event = "auto_cycle_cleared_by_override",
            stage = "gate",
            account_id = self.who.account_id,
            account_name = self.who.account_name,
            player_id = self.who.player_id,
            player_name = self.who.player_name,
            entity_id = self.entity_id,
            entity_name = self.entity_name,
            ability_id = self.ability_id,
            ability_name = self.ability_name,
            "auto-cycle: cleared by manual override (different ability fired)"
        );
    }
}
