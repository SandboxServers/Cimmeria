//! The ammo campaign's telemetry catalog (AM-F, issue #1026): the `event`
//! names and refusal `reason` strings every packet emits on target `ammo`.
//!
//! The authoritative table is `docs/analysis/ammo/work-packets.md`
//! § Telemetry contract; this module is its Rust mirror, so a packet uses a
//! constant instead of retyping a string. A new event goes in both, through
//! the coordinator.
//!
//! Write the target as the literal `target: "ammo"` at every call site, never
//! through a constant: `cimmeria-server`'s `target_scan_tests` finds targets
//! by reading source text for `target: "…"`, and a constant hides the call
//! from it. The `ammo=debug` `OTEL_FILTER` row (added by AM-F) exports every
//! level from DEBUG up.
//!
//! Correlators on every event: `account_id`, `player_id`, `entity_id`,
//! `item_id` (the ammo item's design id, from `ammo_item_types`) and
//! `ammo_type`. A state change records its before and after values.

/// The tracing target, for documentation and tests only (see module docs).
pub const TARGET: &str = "ammo";

/// Event names, one per catalog row.
pub mod events {
    /// DEBUG, AM-02: `requested`, `drawn`, `clip_before`, `clip_after`,
    /// `stack_before`, `stack_after`.
    pub const RELOAD_DRAW: &str = "reload_draw";
    /// WARN, AM-02: `reason` ([`super::reasons::STACK_EMPTY`]).
    pub const RELOAD_REFUSED: &str = "reload_refused";
    /// DEBUG, AM-02: `returned`, `remainder`, `stack_before`, `stack_after`.
    pub const AMMO_SWITCH_RETURN: &str = "ammo_switch_return";
    /// WARN, AM-03: `reason` (one of the AM-03 reasons below).
    pub const AMMO_TYPE_CHANGE_REJECTED: &str = "ammo_type_change_rejected";
    /// DEBUG, AM-04+: `damage_mult`, `penetration_mult`, `damage_type`,
    /// `toggle_ability_id`, `on_hit_effect_id` (when present).
    pub const AMMO_DAMAGE_APPLIED: &str = "ammo_damage_applied";
    /// DEBUG, AM-05: `loot_table_id`, `quantity`.
    pub const AMMO_LOOT_DROPPED: &str = "ammo_loot_dropped";
    /// INFO (WARN on refusal), AM-06: `quantity`, `reason` on refusal.
    pub const GM_GIVE_AMMO: &str = "gm_give_ammo";
    /// INFO, AM-06: `on`.
    pub const GM_INFINITE_AMMO_TOGGLED: &str = "gm_infinite_ammo_toggled";
    /// INFO, AM-F: the resolved `ammo.finite_special` value at startup.
    pub const FEATURE_FLAG: &str = "feature_flag";
    /// WARN, AM-F: an unrecognised flag value, with the fallback used.
    pub const FEATURE_FLAG_INVALID: &str = "feature_flag_invalid";
}

/// Refusal `reason` strings.
pub mod reasons {
    /// AM-02: a special reload found no rounds in the bags.
    pub const STACK_EMPTY: &str = "stack_empty";
    /// AM-03: no `WeaponDef` for the item (#602's fail-closed case).
    pub const WEAPON_DEF_CACHE_MISS: &str = "weapon_def_cache_miss";
    /// AM-03: the weapon's `ammo_types` does not list the type.
    pub const NOT_IN_ALLOWED_TYPES: &str = "not_in_allowed_types";
    /// AM-03: the item sits in more than one bandolier slot.
    pub const AMBIGUOUS_SLOT: &str = "ambiguous_slot";
    /// AM-03: the item is not in the bandolier.
    pub const ITEM_NOT_IN_BANDOLIER: &str = "item_not_in_bandolier";
    /// AM-03: `ammo_type <= 0`.
    pub const NON_POSITIVE_AMMO_TYPE: &str = "non_positive_ammo_type";
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every name is distinct and snake_case, so a SigNoz filter on one
    /// never matches another.
    #[test]
    fn names_are_distinct_snake_case() {
        let all = [
            events::RELOAD_DRAW,
            events::RELOAD_REFUSED,
            events::AMMO_SWITCH_RETURN,
            events::AMMO_TYPE_CHANGE_REJECTED,
            events::AMMO_DAMAGE_APPLIED,
            events::AMMO_LOOT_DROPPED,
            events::GM_GIVE_AMMO,
            events::GM_INFINITE_AMMO_TOGGLED,
            events::FEATURE_FLAG,
            events::FEATURE_FLAG_INVALID,
            reasons::STACK_EMPTY,
            reasons::WEAPON_DEF_CACHE_MISS,
            reasons::NOT_IN_ALLOWED_TYPES,
            reasons::AMBIGUOUS_SLOT,
            reasons::ITEM_NOT_IN_BANDOLIER,
            reasons::NON_POSITIVE_AMMO_TYPE,
        ];
        let mut seen = std::collections::HashSet::new();
        for name in all {
            assert!(seen.insert(name), "duplicate {name}");
            assert!(
                name.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
                "{name} is not snake_case"
            );
        }
        assert_eq!(TARGET, "ammo");
    }
}
