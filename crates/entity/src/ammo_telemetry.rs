//! The ammo campaign's telemetry catalog (issue #1026): the `event` names and
//! refusal `reason` strings every packet emits on target `ammo`.
//!
//! The authoritative table is `docs/analysis/ammo/work-packets.md`
//! § Telemetry contract; this module is its Rust mirror, so a packet uses a
//! constant instead of retyping a string. A new event goes in both, and in
//! the `ammo` rows of `docs/architecture/observability.md`. AM-12 (the
//! close-out) added every event and reason the Wave-1 and Wave-2 worknotes
//! left for the coordinator, so this list now names everything the code
//! emits on target `ammo`.
//!
//! Write the target as the literal `target: "ammo"` at every call site, never
//! through a constant: `cimmeria-server`'s `target_scan_tests` finds targets
//! by reading source text for `target: "…"`, and a constant hides the call
//! from it. The `ammo=debug` `OTEL_FILTER` row (added by AM-F) exports every
//! level from DEBUG up.
//!
//! Correlators on every player event: `account_id`, `player_id`,
//! `entity_id`, `item_id` (the ammo item's design id, from
//! `ammo_item_types`) and `ammo_type`. A state change records its before and
//! after values. Startup events (`feature_flag*`, `catalog_*`) carry none.

/// The tracing target, for documentation and tests only (see module docs).
pub const TARGET: &str = "ammo";

/// Event names, one per catalog row.
pub mod events {
    // ── AM-F: startup ──────────────────────────────────────────────────
    /// INFO, AM-F: the resolved `ammo.finite_special` value at startup.
    pub const FEATURE_FLAG: &str = "feature_flag";
    /// WARN, AM-F: an unrecognised flag value; the flag is off.
    pub const FEATURE_FLAG_INVALID: &str = "feature_flag_invalid";
    /// INFO, AM-F: `modifiers`, `item_types` loaded at cell startup.
    pub const CATALOG_LOADED: &str = "catalog_loaded";
    /// ERROR, AM-F: the ammo catalog failed to load (`error`); every shot
    /// fires unmodified and `.giveammo` finds no item.
    pub const CATALOG_LOAD_FAILED: &str = "catalog_load_failed";

    // ── AM-02: the reserve ─────────────────────────────────────────────
    /// DEBUG, AM-02 (base): `requested`, `drawn`, `clip_before`,
    /// `clip_after`, `stack_before`, `stack_after`.
    pub const RELOAD_DRAW: &str = "reload_draw";
    /// WARN, AM-02 (base): `reason` ([`super::reasons::STACK_EMPTY`]).
    pub const RELOAD_REFUSED: &str = "reload_refused";
    /// DEBUG, AM-02 (base): `returned`, `remainder`, `stack_before`,
    /// `stack_after`.
    pub const AMMO_SWITCH_RETURN: &str = "ammo_switch_return";
    /// DEBUG, AM-02 (cell): a special reload asked the base for rounds;
    /// `slot_id`, `instance_id`, `clip_before`.
    pub const RELOAD_DRAW_REQUESTED: &str = "reload_draw_requested";
    /// DEBUG, AM-02 (cell): the drawn rounds are in the clip; `drawn`,
    /// `clip_after`, `stack_after`.
    pub const RELOAD_DRAWN_LOADED: &str = "reload_drawn_loaded";
    /// INFO, AM-02 (cell): a draw answer arrived after the player left
    /// (`reason=entity_gone`) or the weapon left its slot
    /// (`reason=slot_changed`); the rounds are in the weapon row.
    pub const RELOAD_DRAWN_STALE: &str = "reload_drawn_stale";
    /// INFO, AM-02 (cell): the player saw the refusal line; `reason` is a
    /// `ReserveRefusal` (`stack_empty`, `weapon_changed`, `db_error`).
    pub const RELOAD_REFUSED_FEEDBACK: &str = "reload_refused_feedback";
    /// WARN, AM-02 (base): the weapon row held a different clip than the
    /// cell reported; the base counted from the row.
    pub const RELOAD_DRAW_CLIP_MISMATCH: &str = "reload_draw_clip_mismatch";
    /// DEBUG, AM-02 (cell): a special clip is on its way back to the bags.
    pub const AMMO_SWITCH_RETURN_REQUESTED: &str = "ammo_switch_return_requested";
    /// INFO, AM-02 (cell): the switch did not happen; `reason`
    /// ([`super::reasons::BAGS_FULL`] with `remainder`, or a
    /// `ReserveRefusal`).
    pub const AMMO_SWITCH_REFUSED: &str = "ammo_switch_refused";
    /// DEBUG, AM-02 (cell): a default clip switched to a special type was
    /// emptied (default rounds are free, D-AM02).
    pub const AMMO_SWITCH_DEFAULT_EMPTIED: &str = "ammo_switch_default_emptied";
    /// DEBUG, AM-02 (cell): a second switch while a return is in flight;
    /// `reason=request_in_flight`.
    pub const AMMO_SWITCH_DROPPED: &str = "ammo_switch_dropped";
    /// INFO, AM-02 (cell): a switch answer for a player or slot that moved
    /// on (`reason` `entity_gone`, `slot_changed`).
    pub const SWITCH_RETURNED_STALE: &str = "switch_returned_stale";
    /// WARN, AM-02 (base): the row held a different clip than the cell's
    /// switch request.
    pub const SWITCH_RETURN_CLIP_MISMATCH: &str = "switch_return_clip_mismatch";
    /// WARN, AM-02 (cell): the reserve request could not be queued
    /// (`reason=base_channel_closed`); nothing moved.
    pub const RESERVE_REQUEST_SEND_FAILED: &str = "reserve_request_send_failed";
    /// WARN, AM-02 (base): the answer could not reach the cell
    /// (`reason=cell_channel_closed`); the commit stands.
    pub const RESERVE_ANSWER_SEND_FAILED: &str = "reserve_answer_send_failed";
    /// WARN, AM-02 (cell): a reserve feedback line could not be queued.
    pub const FEEDBACK_SEND_FAILED: &str = "feedback_send_failed";

    // ── AM-03: validation ──────────────────────────────────────────────
    /// WARN, AM-03: `reason` (one of the AM-03 reasons below).
    pub const AMMO_TYPE_CHANGE_REJECTED: &str = "ammo_type_change_rejected";
    /// WARN, AM-03: the refusal line could not be queued.
    pub const AMMO_FEEDBACK_SEND_FAILED: &str = "ammo_feedback_send_failed";

    // ── AM-04 and the family packets: the shot ─────────────────────────
    /// DEBUG, AM-04+: `damage_mult`, `penetration_mult`, `damage_type`,
    /// `toggle_ability_id`, `on_hit_effect_id` (when present).
    pub const AMMO_DAMAGE_APPLIED: &str = "ammo_damage_applied";
    /// WARN, AM-04: a row names an on-hit effect missing from `effect_defs`;
    /// the shot fires without it.
    pub const AMMO_ON_HIT_EFFECT_MISSING: &str = "ammo_on_hit_effect_missing";
    /// DEBUG, AM-09: one per EMP hit; `target_entity_id`, `mechanical`,
    /// `focus_before`, `focus_drained`, `health_before`, `health_damage`;
    /// `reason=target_missing` when the target is gone.
    pub const AMMO_EMP_DISRUPT: &str = "ammo_emp_disrupt";
    /// DEBUG, AM-10: one per Explosive splash; `radius`, `fraction`,
    /// `splash_count`, `los_blocked`, `targets`.
    pub const AMMO_SPLASH: &str = "ammo_splash";
    /// WARN, AM-10: an on-hit `TCM_AERadius` effect with a missing or
    /// out-of-range `SplashDamageFraction`; nothing splashes.
    pub const AMMO_SPLASH_BAD_FRACTION: &str = "ammo_splash_bad_fraction";
    /// DEBUG, AM-11d: a support dart healed or cleansed an ally or the
    /// shooter; `decision_outcome=applied`, before and after pools.
    pub const AMMO_SUPPORT_APPLIED: &str = "ammo_support_applied";
    /// DEBUG, AM-11d: a support dart was refused; `decision_outcome=refused`,
    /// `stage` (`launch`, `fire`), `reason`.
    pub const AMMO_SUPPORT_REFUSED: &str = "ammo_support_refused";
    /// WARN, AM-11d: the support refusal line could not be queued
    /// (`reason=cell_to_base_closed`).
    pub const AMMO_SUPPORT_FEEDBACK_SEND_FAILED: &str = "ammo_support_feedback_send_failed";

    // ── AM-05: loot ────────────────────────────────────────────────────
    /// DEBUG, AM-05 (emitted since AM-12): a looted item is special ammo;
    /// `quantity` (rounds), `corpse_id`, `corpse_template_id`,
    /// `loot_table_id`.
    pub const AMMO_LOOT_DROPPED: &str = "ammo_loot_dropped";

    // ── AM-06: GM tooling ──────────────────────────────────────────────
    /// INFO (WARN on refusal), AM-06: `quantity`, `reason` on refusal.
    pub const GM_GIVE_AMMO: &str = "gm_give_ammo";
    /// INFO, AM-06: `on`.
    pub const GM_INFINITE_AMMO_TOGGLED: &str = "gm_infinite_ammo_toggled";
}

/// Refusal `reason` strings.
pub mod reasons {
    /// AM-02: a special reload found no rounds in the bags.
    pub const STACK_EMPTY: &str = "stack_empty";
    /// AM-02: the weapon left its slot, or no longer holds the type being
    /// returned, before the base committed.
    pub const WEAPON_CHANGED: &str = "weapon_changed";
    /// AM-02: a second switch while a return is in flight.
    pub const REQUEST_IN_FLIGHT: &str = "request_in_flight";
    /// AM-02: an answer for a player that is no longer on this cell.
    pub const ENTITY_GONE: &str = "entity_gone";
    /// AM-02: an answer for a weapon that left its slot.
    pub const SLOT_CHANGED: &str = "slot_changed";
    /// AM-02 (cell): the cell-to-base channel is closed.
    pub const BASE_CHANNEL_CLOSED: &str = "base_channel_closed";
    /// AM-02 (base): the base-to-cell channel is closed.
    pub const CELL_CHANNEL_CLOSED: &str = "cell_channel_closed";
    /// AM-03: no `WeaponDef` for the item (#602's fail-closed case).
    pub const WEAPON_DEF_CACHE_MISS: &str = "weapon_def_cache_miss";
    /// AM-03: the weapon's `ammo_types` does not list the type.
    pub const NOT_IN_ALLOWED_TYPES: &str = "not_in_allowed_types";
    /// AM-03: the instance id sits in more than one bandolier slot
    /// (corrupt state).
    pub const AMBIGUOUS_SLOT: &str = "ambiguous_slot";
    /// AM-03: the item is not in the bandolier.
    pub const ITEM_NOT_IN_BANDOLIER: &str = "item_not_in_bandolier";
    /// AM-03: `ammo_type <= 0`.
    pub const NON_POSITIVE_AMMO_TYPE: &str = "non_positive_ammo_type";
    /// AM-06: `.giveammo`'s type names no `EAmmoType`, or names several.
    pub const UNKNOWN_AMMO_TYPE: &str = "unknown_ammo_type";
    /// AM-06: the type is default ammo (free, no reserve item to grant).
    pub const NOT_SPECIAL_AMMO: &str = "not_special_ammo";
    /// AM-06: a special type with no `ammo_item_types` row.
    pub const NO_RESERVE_ITEM: &str = "no_reserve_item";
    /// AM-06: the quantity is missing, not a number, or `<= 0`.
    pub const BAD_QUANTITY: &str = "bad_quantity";
    /// AM-06: an argument other than the quantity is malformed.
    pub const BAD_ARGS: &str = "bad_args";
    /// AM-06: the recipient has no character id.
    pub const NOT_A_PLAYER: &str = "not_a_player";
    /// AM-06: the recipient's session no longer plays that character.
    pub const SESSION_MISMATCH: &str = "session_mismatch";
    /// AM-02, AM-06: nothing (AM-06) or not everything (AM-02 switch) fit in
    /// the carried bags.
    pub const BAGS_FULL: &str = "bags_full";
    /// AM-02, AM-06: no database, or the write failed and rolled back.
    pub const DB_ERROR: &str = "db_error";
    /// AM-09: the EMP target left before the on-hit effect ran.
    pub const TARGET_MISSING: &str = "target_missing";
    /// AM-11d: a support dart aimed at a hostile NPC or a duel opponent.
    pub const HOSTILE_TARGET: &str = "hostile_target";
    /// AM-11d: the support dart's target left before the shot fired.
    pub const TARGET_GONE: &str = "target_gone";
    /// AM-11d: the target is neither the shooter nor a player (a vendor, a
    /// friendly NPC, a pet); reachable only through a warmup.
    pub const NOT_AN_ALLY: &str = "not_an_ally";
    /// AM-11d: the cell-to-base channel is closed.
    pub const CELL_TO_BASE_CLOSED: &str = "cell_to_base_closed";
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_EVENTS: [&str; 34] = [
        events::FEATURE_FLAG,
        events::FEATURE_FLAG_INVALID,
        events::CATALOG_LOADED,
        events::CATALOG_LOAD_FAILED,
        events::RELOAD_DRAW,
        events::RELOAD_REFUSED,
        events::AMMO_SWITCH_RETURN,
        events::RELOAD_DRAW_REQUESTED,
        events::RELOAD_DRAWN_LOADED,
        events::RELOAD_DRAWN_STALE,
        events::RELOAD_REFUSED_FEEDBACK,
        events::RELOAD_DRAW_CLIP_MISMATCH,
        events::AMMO_SWITCH_RETURN_REQUESTED,
        events::AMMO_SWITCH_REFUSED,
        events::AMMO_SWITCH_DEFAULT_EMPTIED,
        events::AMMO_SWITCH_DROPPED,
        events::SWITCH_RETURNED_STALE,
        events::SWITCH_RETURN_CLIP_MISMATCH,
        events::RESERVE_REQUEST_SEND_FAILED,
        events::RESERVE_ANSWER_SEND_FAILED,
        events::FEEDBACK_SEND_FAILED,
        events::AMMO_TYPE_CHANGE_REJECTED,
        events::AMMO_FEEDBACK_SEND_FAILED,
        events::AMMO_DAMAGE_APPLIED,
        events::AMMO_ON_HIT_EFFECT_MISSING,
        events::AMMO_EMP_DISRUPT,
        events::AMMO_SPLASH,
        events::AMMO_SPLASH_BAD_FRACTION,
        events::AMMO_SUPPORT_APPLIED,
        events::AMMO_SUPPORT_REFUSED,
        events::AMMO_SUPPORT_FEEDBACK_SEND_FAILED,
        events::AMMO_LOOT_DROPPED,
        events::GM_GIVE_AMMO,
        events::GM_INFINITE_AMMO_TOGGLED,
    ];

    const ALL_REASONS: [&str; 26] = [
        reasons::STACK_EMPTY,
        reasons::WEAPON_CHANGED,
        reasons::REQUEST_IN_FLIGHT,
        reasons::ENTITY_GONE,
        reasons::SLOT_CHANGED,
        reasons::BASE_CHANNEL_CLOSED,
        reasons::CELL_CHANNEL_CLOSED,
        reasons::WEAPON_DEF_CACHE_MISS,
        reasons::NOT_IN_ALLOWED_TYPES,
        reasons::AMBIGUOUS_SLOT,
        reasons::ITEM_NOT_IN_BANDOLIER,
        reasons::NON_POSITIVE_AMMO_TYPE,
        reasons::UNKNOWN_AMMO_TYPE,
        reasons::NOT_SPECIAL_AMMO,
        reasons::NO_RESERVE_ITEM,
        reasons::BAD_QUANTITY,
        reasons::BAD_ARGS,
        reasons::NOT_A_PLAYER,
        reasons::SESSION_MISMATCH,
        reasons::BAGS_FULL,
        reasons::DB_ERROR,
        reasons::TARGET_MISSING,
        reasons::HOSTILE_TARGET,
        reasons::TARGET_GONE,
        reasons::NOT_AN_ALLY,
        reasons::CELL_TO_BASE_CLOSED,
    ];

    /// Every name is distinct and snake_case, so a SigNoz filter on one
    /// never matches another. Events and reasons are checked apart: a
    /// reason may share a spelling with nothing, but two events may not.
    #[test]
    fn names_are_distinct_snake_case() {
        for set in [&ALL_EVENTS[..], &ALL_REASONS[..]] {
            let mut seen = std::collections::HashSet::new();
            for name in set {
                assert!(seen.insert(name), "duplicate {name}");
                assert!(
                    name.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
                    "{name} is not snake_case"
                );
            }
        }
        assert_eq!(TARGET, "ammo");
    }

    /// The catalog names every event the code emits on target `ammo`
    /// (AM-12's sweep). Reads the workspace sources, so a new
    /// `event = "…"` on target `ammo` without a catalog constant fails
    /// here. Skips when the sources are not beside the crate (a packaged
    /// build).
    #[test]
    fn catalog_covers_every_ammo_event_in_the_workspace() {
        let crates = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        if !crates.join("cell-combat").is_dir() {
            return;
        }
        let mut missing = Vec::new();
        let mut stack = vec![crates];
        while let Some(dir) = stack.pop() {
            let Ok(entries) = std::fs::read_dir(&dir) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if path.file_name().is_some_and(|n| n != "target") {
                        stack.push(path);
                    }
                    continue;
                }
                if path.extension().is_none_or(|e| e != "rs") {
                    continue;
                }
                let Ok(text) = std::fs::read_to_string(&path) else {
                    continue;
                };
                for (i, _) in text.match_indices("target: \"ammo\"") {
                    // The event field follows the target within a few lines.
                    let window: String = text[i..].lines().take(6).collect::<Vec<_>>().join(" ");
                    let Some(rest) = window.split("event = \"").nth(1) else {
                        continue;
                    };
                    let name = rest.split('"').next().unwrap_or("");
                    if !ALL_EVENTS.contains(&name) {
                        missing.push(format!("{name} ({})", path.display()));
                    }
                }
            }
        }
        assert!(
            missing.is_empty(),
            "ammo events missing from ammo_telemetry::events: {missing:#?}"
        );
    }
}
