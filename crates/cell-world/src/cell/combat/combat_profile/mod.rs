//! Combat-profile resolution for spawned NPCs: which abilities an NPC gets,
//! and whether that came from an explicit ability set or from a placeholder.
//!
//! `NPC_DEFAULT_ABILITY` (592 Pistol Shot) is a Giza/Python placeholder, not
//! Cimmeria behaviour. Spawning applies it only under `PLACEHOLDER_POLICY`:
//! `Legacy` gives every set-less template 592; `RequireWeaponEvidence`
//! (production, NC-30) gives it to templates with a human firearm component.
//! It is a human firearm, so it is wrong for creatures and for melee humans.
//! The resolver makes that choice in one place and reports it.
//!
//! This module is pure. [`PLACEHOLDER_POLICY`] is `RequireWeaponEvidence`
//! (decision D-NC1, NC-30; narrowed to firearms by D-NC13, NC-50b): a
//! template with no ability set gets 592 only when it carries a component
//! from [`HUMAN_FIREARM_PREFIXES`].

use super::aggression::NPC_DEFAULT_ABILITY; // 592 Pistol Shot

use std::collections::HashSet;
use std::sync::{LazyLock, Mutex};

/// What an NPC with no explicit ability set receives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaceholderPolicy {
    /// Keep the 592 Pistol Shot placeholder for every NPC with no set.
    Legacy,
    /// Give 592 only to a human with a human firearm weapon component.
    RequireWeaponEvidence,
}

/// The policy production runs. D-NC1 (decided 2026-11-27) flipped it from
/// `Legacy` to `RequireWeaponEvidence` under NC-30. D-NC13 (2026-11-28)
/// narrowed the firearm test to [`HUMAN_FIREARM_PREFIXES`] under NC-50b.
pub const PLACEHOLDER_POLICY: PlaceholderPolicy = PlaceholderPolicy::RequireWeaponEvidence;

/// A component whose package starts with this is a weapon mesh.
pub const WEAPON_COMPONENT_PREFIX: &str = "WP-";
/// 592 Pistol Shot is a human firearm: under RequireWeaponEvidence only a
/// component starting with one of these may earn it. The list is a
/// whitelist of firearm classes, so a melee human (`WP-Human.WP_HandToHand_1A`,
/// templates 28, 92, 1415, 1465) and any other `WP-Human.*` part earn nothing.
pub const HUMAN_FIREARM_PREFIXES: &[&str] = &[
    "WP-Human.WP_Pistol_",
    "WP-Human.WP_SMG_",
    "WP-Human.WP_AR_",
    "WP-Human.WP_LMG_",
    "WP-Human.WP_HMG_",
    "WP-Human.WP_Rifle_",
    "WP-Human.WP_Shotgun_",
];

/// Where an NPC's abilities came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombatProfileSource {
    /// The template's own ability set.
    ExplicitSet,
    /// The 592 placeholder was applied.
    LegacyPlaceholder,
    /// The NPC has no abilities and no placeholder applies.
    Unresolved,
}

/// The weapon component an NPC carries, if any.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WeaponEvidence {
    /// No `WP-*` component.
    None,
    /// The first `WP-*` component.
    Component(String),
}

/// The resolved combat profile of one NPC template.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NpcCombatProfile {
    /// Ability ids in the order the set gives them.
    pub abilities: Vec<i32>,
    /// Where `abilities` came from.
    pub source: CombatProfileSource,
    /// The weapon component, reported under either policy.
    pub weapon_evidence: WeaponEvidence,
}

/// First component starting with WEAPON_COMPONENT_PREFIX, cloned.
pub fn weapon_evidence(components: Option<&[String]>) -> WeaponEvidence {
    components
        .and_then(|list| list.iter().find(|c| c.starts_with(WEAPON_COMPONENT_PREFIX)))
        .map_or(WeaponEvidence::None, |c| {
            WeaponEvidence::Component(c.clone())
        })
}

/// Resolve the abilities for one NPC template. An explicit ability set always
/// wins; an empty set falls through the placeholder rules.
pub fn resolve_npc_combat_profile(
    ability_ids: &[i32],
    components: Option<&[String]>,
    policy: PlaceholderPolicy,
) -> NpcCombatProfile {
    let weapon = weapon_evidence(components);
    if !ability_ids.is_empty() {
        return NpcCombatProfile {
            abilities: ability_ids.to_vec(),
            source: CombatProfileSource::ExplicitSet,
            weapon_evidence: weapon,
        };
    }
    let placeholder = match policy {
        PlaceholderPolicy::Legacy => true,
        PlaceholderPolicy::RequireWeaponEvidence => components.unwrap_or(&[]).iter().any(|c| {
            HUMAN_FIREARM_PREFIXES
                .iter()
                .any(|prefix| c.starts_with(prefix))
        }),
    };
    let (abilities, source) = if placeholder {
        (
            vec![NPC_DEFAULT_ABILITY],
            CombatProfileSource::LegacyPlaceholder,
        )
    } else {
        (Vec::new(), CombatProfileSource::Unresolved)
    };
    NpcCombatProfile {
        abilities,
        source,
        weapon_evidence: weapon,
    }
}

/// The AI-time fallback for an NPC that knows no ability:
/// Legacy → Some(NPC_DEFAULT_ABILITY); RequireWeaponEvidence → None.
pub fn placeholder_ability(policy: PlaceholderPolicy) -> Option<i32> {
    match policy {
        PlaceholderPolicy::Legacy => Some(NPC_DEFAULT_ABILITY),
        PlaceholderPolicy::RequireWeaponEvidence => None,
    }
}

/// Template ids already reported this process, so each template logs once.
static REPORTED: LazyLock<Mutex<HashSet<i32>>> = LazyLock::new(Mutex::default);

/// Process-wide once-per-template throttle: true the first time a
/// template id is seen, false after. Static LazyLock<Mutex<HashSet<i32>>>,
/// the same pattern as INERT_LOG in
/// crates/cell-combat/src/cell/abilities/effect_plan.rs:57.
pub fn first_report(template_id: i32) -> bool {
    REPORTED
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .insert(template_id)
}

mod report;
pub use report::report_profile;

#[cfg(test)]
mod tests;
