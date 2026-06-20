//! Appearance compositing and holster-state helpers.
//!
//! The cell entity's wire `ComponentList` depends on the current holster
//! state: when the player's weapon is holstered, the active weapon visual
//! is filtered out so the client renders the unarmed stance. This module
//! owns the holster-filter free function ([`filter_holstered_weapon`]) and
//! the [`CellEntity`] methods that read/write holster state and build the
//! `BeingAppearance` component list.

use super::CellEntity;

/// Filter the active bandolier weapon visual out of a component list
/// when the player is holstered. Returns `components.to_vec()` unchanged
/// when `holstered = false` or when no weapon visual is set.
///
/// **Invariant:** when `weapon_visual` is `Some(v)`, `v` should also
/// appear in `components`. The two are populated by the same DB loader
/// to stay in sync — divergence indicates a data bug upstream (case
/// drift, whitespace, a stale row). We can't fix the upstream data
/// from here, but the filter surfaces the violation: when the
/// post-filter length equals the input length even though we asked
/// for a filter, a `warn`-level log fires so the operator knows the
/// holster filter did not take effect, and a `debug_assert!` panics
/// in debug builds so test runs catch it loudly. Production builds
/// degrade gracefully: the wire `ComponentList` carries the weapon
/// entry through, the client renders armed-pose instead of holstered,
/// and the visual is wrong but nothing crashes.
///
/// Used by both [`CellEntity::appearance_components`] and (via the
/// `cimmeria_services` crate) `PlayerLoadData::appearance_components`
/// so the wire-format holster filter has one implementation, not two.
pub fn filter_holstered_weapon(
    components: &[String],
    weapon_visual: Option<&str>,
    holstered: bool,
) -> Vec<String> {
    let Some(weapon) = weapon_visual.filter(|_| holstered) else {
        return components.to_vec();
    };
    let filtered: Vec<String> = components
        .iter()
        .filter(|c| c.as_str() != weapon)
        .cloned()
        .collect();
    if filtered.len() == components.len() {
        tracing::warn!(
            weapon = %weapon,
            ?components,
            "filter_holstered_weapon: weapon_visual not found in components \
             — invariant violated, holster filter is a no-op for this emit \
             (check DB-side string normalization between weapon_visual and \
             the components query)",
        );
        debug_assert!(
            false,
            "weapon_visual {weapon:?} not found in components {components:?} \
             — invariant violated"
        );
    }
    filtered
}

impl CellEntity {
    /// Build the `ComponentList` that should go out in `BeingAppearance`,
    /// applying the current holster state.
    ///
    /// Returns `components` unchanged when not holstered, or `components`
    /// with the `weapon_visual` entry filtered out when holstered. The
    /// SGW BigWorld client's appearance compositor picks weapon-stance
    /// vs. holstered-stance from whichever weapon-shaped entry it finds
    /// in this list (`ghidra://SGW.exe@0x00ec0840`), so omitting the
    /// weapon visual is the wire-format-correct way to render the
    /// holstered pose — it falls back to `WEAP_Melee = 4` and plays the
    /// unarmed-stance animation blend.
    ///
    /// Callers should use this in place of `&entity.components` at every
    /// `BeingAppearance`-emit site. Reading `components` directly is fine
    /// for non-wire purposes (debug logs, AoI propagation copies, NPC
    /// templates that don't have a holster concept).
    pub fn appearance_components(&self) -> Vec<String> {
        filter_holstered_weapon(
            &self.components,
            self.weapon_visual.as_deref(),
            self.weapon_holstered,
        )
    }

    /// Toggle the holster state. Returns `true` if the state actually
    /// changed (the caller should rebroadcast `BeingAppearance`), `false`
    /// if it was already in the requested state.
    ///
    /// The return value gates on state-change ONLY. The `weapon_visual`
    /// check that used to live here was wrong: in production the cell
    /// entity's `weapon_visual` is always `None` (only the base side
    /// populates it from `PlayerLoadData`), so the gate silently
    /// suppressed every holster broadcast. The base-side
    /// `RefreshAppearance` handler has its own change-detection
    /// (`weapon_holstered != cached` short-circuit), so redundant calls
    /// are already a free no-op there — gating here just hid bugs.
    pub fn set_weapon_holstered(&mut self, holstered: bool) -> bool {
        if self.weapon_holstered == holstered {
            return false;
        }
        self.weapon_holstered = holstered;
        true
    }

    /// Lockstep the holster state with `BSF_InCombat`: in-combat = drawn,
    /// out-of-combat = holstered. Returns `true` when the caller should
    /// rebroadcast `BeingAppearance` (state actually flipped AND there's
    /// a `weapon_visual` whose presence in the wire list will change).
    ///
    /// This is the canonical entry point from `enter_player_combat` /
    /// `exit_player_combat`. Keeping the policy here (rather than
    /// duplicating `!in_combat` at every caller) means future tweaks —
    /// e.g. "stay drawn for 5s after leaving combat" — happen in one
    /// place.
    pub fn sync_holster_to_combat(&mut self, in_combat: bool) -> bool {
        self.set_weapon_holstered(!in_combat)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn comps(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    /// Holstered with a weapon visual present in the list → the weapon
    /// entry is dropped and every other component survives.
    #[test]
    fn holstered_filters_matching_weapon_entry() {
        let components = comps(&["body", "head", "WEAP_P90"]);
        let out = filter_holstered_weapon(&components, Some("WEAP_P90"), true);
        assert_eq!(out, comps(&["body", "head"]));
    }

    /// Not holstered → list returned unchanged even when a weapon_visual
    /// is set (drawn pose keeps the weapon entry). Pins the early-return
    /// `else` arm of the `.filter(|_| holstered)` guard.
    #[test]
    fn not_holstered_returns_components_unchanged() {
        let components = comps(&["body", "WEAP_P90"]);
        let out = filter_holstered_weapon(&components, Some("WEAP_P90"), false);
        assert_eq!(out, components);
    }

    /// Holstered but no weapon_visual (e.g. unarmed / NPC) → unchanged.
    /// Pins the `None` arm of the `let Some(weapon) = ... else` guard.
    #[test]
    fn holstered_without_weapon_visual_returns_unchanged() {
        let components = comps(&["body", "head"]);
        let out = filter_holstered_weapon(&components, None, true);
        assert_eq!(out, components);
    }

    /// Invariant-violation edge: holstered with a weapon_visual that is
    /// NOT present in the list. The filter is a no-op (post-filter length
    /// equals input length), which fires the `warn` and — in debug/test
    /// builds — the `debug_assert!(false, ...)`. This `should_panic`
    /// guard covers the previously-uncovered warn + debug_assert branch.
    /// Reverting the `debug_assert!` would make this test fail (no panic).
    #[test]
    #[should_panic(expected = "invariant violated")]
    fn holstered_with_missing_weapon_visual_trips_invariant() {
        let components = comps(&["body", "head"]);
        // "WEAP_P90" is not in `components` — drift between weapon_visual
        // and the components query. Filter can't remove what isn't there.
        let _ = filter_holstered_weapon(&components, Some("WEAP_P90"), true);
    }
}
