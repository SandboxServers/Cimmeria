//! NPC aggro: threat-table mutation and Idle→Fighting transition.
//!
//! [`generate_threat`] is the entry point combat takes when an attacker
//! lands damage on an NPC: it accumulates threat so the AI knows who to
//! hit back, preempts the NPC's current behavior into `Fighting`, and (for
//! player attackers) mirrors the addition into the player's combat state
//! via [`super::player_combat::enter_player_combat`].

use crate::cell::service::npc_ai;

/// What made an NPC engage, passed to [`generate_threat`] and reported as
/// `cause` on the `npc_ai.aggro event=acquired` row. Enumerated; the label
/// is a metric label.
///
/// `assist` (a neighbour pulling the NPC in) is reserved for NA14 and is
/// deliberately not a variant until something produces it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggroCause {
    /// The Idle auto-aggro scan picked a witness.
    Proximity,
    /// Damage landed on the NPC.
    Damage,
    /// A content chain's `generate_threat` action.
    ContentThreat,
}

impl AggroCause {
    /// Stable snake_case label. Treat as API.
    pub fn label(self) -> &'static str {
        match self {
            Self::Proximity => "proximity",
            Self::Damage => "damage",
            Self::ContentThreat => "content_threat",
        }
    }
}

/// Default leash radius in world units, used when the NPC's template does not
/// set `entity_templates.leash_distance`. It is measured from the **NPC's own**
/// position to its spawn point, horizontally, not from the target (NA12,
/// D-NA03): the old target-to-spawn test fired on a player standing 49.9 u
/// from spawn and bounded how far any NPC would chase. The hysteresis band,
/// vertical cap and the other leash policy numbers live in
/// `cell::service::npc_ai::leash::policy`.
pub const LEASH_DISTANCE: f32 = 50.0;

/// Maximum attack range in world units for NPC ranged attacks.
/// NPCs won't fire until the target is within this distance.
///
/// Corroborated by the weapon table: `resources.items.max_ranged_range` is
/// `30` for 1,298 of the 1,299 items that bind a melee ability (the lone
/// exception is a ribbon device at 35). So the server-wide ranged default
/// already equals the reach the shipped weapons express.
pub const NPC_ATTACK_RANGE: f32 = 30.0;

/// Maximum attack range in world units for NPC **melee** attacks — an
/// ability whose def carries `is_ranged = false`.
///
/// # Why this is not [`NPC_ATTACK_RANGE`]
///
/// `resources.abilities.max_range` is the `0` sentinel ("use the server
/// default") on every auto-attack in the seed, and `is_ranged` is read only
/// by `calculate_qr` to pick the accuracy/defence branch — it has never
/// gated distance. Before this constant existed, a melee auto-attack
/// resolved to the full 30 m ranged default and the NPC played a swing
/// animation at a target it could not possibly reach. That is the class of
/// thing the 2026-09-18 Castle playtest testers reported as "broken AI": an
/// animation that contradicts what the NPC is doing.
///
/// # Where the number comes from
///
/// Melee reach is per-weapon data, not per-ability data.
/// `resources.items` carries `min_melee_range` / `max_melee_range`
/// alongside `min_ranged_range` / `max_ranged_range` (see
/// `docs/reverse-engineering/findings/combat-formulas-client-evidence.md`
/// E4, which recovers the same `MeleeRanges` / `RangeRanges` pair from the
/// client's cooked item schema). Across every item that binds an
/// `EVENT_ITEM_MELEE` (`event_id = 6`) ability, `max_melee_range` takes
/// exactly three values: `0` (5 items), `2` (1,116 items) and `3` (178
/// items). **3 is the largest melee reach any shipped weapon expresses**,
/// so a 3.0 gate can never truncate a real weapon, and it is also the value
/// 177 of the 204 Jaffa staff items carry — the family this gate was added
/// for.
///
/// # Why a constant rather than the item's own reach
///
/// An NPC has no resolvable weapon item: `entity_templates.weapon_item_id`
/// has zero consumers under `crates/`, and the Goa'uld templates carry no
/// `WP-*` component at all. Until an NPC can name its weapon, the ceiling
/// of the observed range is the honest approximation. Swapping this for a
/// per-weapon lookup is a strict refinement and needs no change at the call
/// sites, which all go through `ability_select::effective_max_range`.
pub const NPC_MELEE_RANGE: f32 = 3.0;

/// Default NPC attack ability ID: "Pistol Shot" (ability 592, ranged DD).
/// Was incorrectly 597 ("Heal Focus") — a self-heal, not an attack.
pub const NPC_DEFAULT_ABILITY: i32 = 592;

/// How long after leaving combat the weapon stays drawn before
/// auto-holstering. Tuned to absorb the gap between killing one mob and
/// aggroing the next so chaining fights doesn't flicker the model.
///
/// Read by [`crate::cell::service::ticks::holster_timer_tick`]; not a
/// wire-format constraint, just a UX choice. Bump it if players still
/// see flicker when running between encounters.
pub const OOC_HOLSTER_DELAY: std::time::Duration = std::time::Duration::from_secs(10);

/// Generate threat on an NPC target from an attacker.
///
/// Transitions the NPC from Idle to Fighting on first hit, accumulates
/// threat so the NPC knows who to attack back, and (if the attacker is a
/// player) mirrors the addition into the player's `threatened_mobs` set
/// so multi-mob aggro tracking stays consistent.
///
/// Returns `Some(new_state_field)` when the attacker just entered combat
/// (i.e., their `threatened_mobs` was empty before this call) — the
/// caller is responsible for sending `onStateFieldUpdate` to the player
/// (via `send_entity_method`, which routes player methods to the player's
/// own client) so their in-combat HUD/cursor flips. Returns `None` when
/// no send is needed.
#[must_use]
#[tracing::instrument(
    name = "threat.generate",
    level = "trace",
    skip_all,
    fields(attacker_id, target_id, threat_amount)
)]
pub fn generate_threat(
    space_mgr: &mut crate::cell::space_manager::SpaceManager,
    attacker_id: u32,
    target_id: u32,
    threat_amount: f32,
    cause: AggroCause,
) -> Option<u32> {
    use cimmeria_entity::cell_entity::AiState;

    // Threat preemption: any non-Dead, non-already-fighting state
    // transitions to Fighting. Patrol/Wander/Investigating/Follow NPCs
    // that get attacked drop their current behavior and engage.
    // Per-state scratch (patrol index, wander deadline, POI, follow
    // target) persists on the entity so the post-Leashing return-to-Idle
    // path can resume the pre-fight behavior from where it left off.
    //
    // Leashing is deliberately absent, and more than that: an NPC walking
    // home **evades** (NA12, D-NA03). It takes no threat and does not put
    // the attacker into combat, so a player cannot pull it back out of the
    // reset, and cannot pin themselves in combat by shooting a mob that has
    // already given up. Before NA12 the threat accrued here and the leash
    // then discarded it, while the player kept `BSF_InCombat` (audit S12).
    let preemptable = match space_mgr.get_entity(target_id) {
        None => return None,
        Some(target) if target.is_player => return None,
        Some(target) if target.ai_state() == AiState::Leashing => {
            // NA02's `damage_ignored` row is the one trace of the evade.
            npc_ai::detectors::leash::on_damage_while_leashing(
                space_mgr,
                target_id,
                attacker_id,
                threat_amount,
            );
            return None;
        }
        Some(target) => matches!(
            target.ai_state(),
            AiState::Idle
                | AiState::Patrol
                | AiState::Wander
                | AiState::Investigating
                | AiState::Follow
        ),
    };
    // Resolved before the `&mut` borrow below; only needed on entry.
    let world = preemptable.then(|| npc_ai::world_label(space_mgr, target_id));

    let mut entered_from = None;
    if let Some(target) = space_mgr.get_entity_mut(target_id) {
        if let Some(world) = &world {
            entered_from = Some(npc_ai::set_ai_state_on(
                target,
                world,
                AiState::Fighting,
                cause.transition_reason(),
            ));
            // The transition above has already stopped the NPC: it clears
            // the in-flight patrol/wander route and zeroes velocity, so the
            // fight handler paths toward the target from a standstill
            // (NA10).
        }
        *target.threat_list.entry(attacker_id).or_insert(0.0) += threat_amount;
    }

    // Every entry into Fighting says why (`npc_ai.aggro event=acquired`).
    // This replaces the old unstructured "preempt -> Fighting" line.
    if let Some(from) = entered_from {
        npc_ai::log_aggro_acquired(space_mgr, target_id, attacker_id, from, cause);
    }

    super::player_combat::enter_player_combat(space_mgr, attacker_id, target_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::combat::state::BSF_IN_COMBAT;
    use crate::cell::combat::threat::player_combat::clear_dead_npc_from_all_player_threat;
    use crate::cell::space_manager::SpaceManager;

    fn make_test_space_mgr_with_npc() -> SpaceManager {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
        let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(cxml).unwrap();

        // Create a player entity (mark is_player so the new threat helpers
        // recognize it; create_entity defaults to is_player=false).
        mgr.create_entity(1, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
            .unwrap();
        mgr.get_entity_mut(1).unwrap().is_player = true;

        // Create an NPC entity
        mgr.spawn_npc(100, "Agnos", [15.0, 0.0, 15.0], [0.0; 3])
            .unwrap();

        mgr
    }

    fn add_player(mgr: &mut SpaceManager, id: u32, x: f32) {
        mgr.create_entity(id, "Agnos", [x, 0.0, 10.0], [0.0; 3])
            .unwrap();
        mgr.get_entity_mut(id).unwrap().is_player = true;
    }

    fn add_npc(mgr: &mut SpaceManager, id: u32, x: f32) {
        mgr.spawn_npc(id, "Agnos", [x, 0.0, 15.0], [0.0; 3])
            .unwrap();
    }

    #[test]
    fn npc_default_ability_is_pistol_shot() {
        // 592 = Pistol Shot (ranged DD). Was previously 597 (Heal Focus, self-heal).
        assert_eq!(NPC_DEFAULT_ABILITY, 592);
    }

    #[test]
    fn leash_distance_is_50() {
        assert_eq!(LEASH_DISTANCE, 50.0);
    }

    // ── generate_threat: NPC-side state preserved from pre-#92 behavior ────

    #[test]
    fn generate_threat_transitions_idle_to_fighting() {
        use cimmeria_entity::cell_entity::AiState;
        let mut mgr = make_test_space_mgr_with_npc();

        assert_eq!(mgr.get_entity(100).unwrap().ai_state(), AiState::Idle);

        let _ = generate_threat(&mut mgr, 1, 100, 50.0, AggroCause::Damage);

        let npc = mgr.get_entity(100).unwrap();
        assert_eq!(npc.ai_state(), AiState::Fighting);
        assert_eq!(npc.threat_list[&1], 50.0);
    }

    #[test]
    fn generate_threat_accumulates() {
        let mut mgr = make_test_space_mgr_with_npc();

        let _ = generate_threat(&mut mgr, 1, 100, 50.0, AggroCause::Damage);
        let _ = generate_threat(&mut mgr, 1, 100, 30.0, AggroCause::Damage);

        assert_eq!(mgr.get_entity(100).unwrap().threat_list[&1], 80.0);
    }

    #[test]
    fn generate_threat_multiple_attackers() {
        let mut mgr = make_test_space_mgr_with_npc();
        add_player(&mut mgr, 2, 20.0);

        let _ = generate_threat(&mut mgr, 1, 100, 50.0, AggroCause::Damage);
        let _ = generate_threat(&mut mgr, 2, 100, 100.0, AggroCause::Damage);

        let npc = mgr.get_entity(100).unwrap();
        assert_eq!(npc.threat_list.len(), 2);
        assert_eq!(npc.threat_list[&1], 50.0);
        assert_eq!(npc.threat_list[&2], 100.0);

        let top = npc
            .threat_list
            .iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(&id, _)| id);
        assert_eq!(top, Some(2));
    }

    #[test]
    fn generate_threat_ignores_player_targets() {
        use cimmeria_entity::cell_entity::AiState;
        let mut mgr = make_test_space_mgr_with_npc();

        // NPC entity 100 attacking player entity 1 — should be a no-op on
        // the player side and return None (no combat-enter broadcast).
        let result = generate_threat(&mut mgr, 100, 1, 50.0, AggroCause::Damage);
        assert_eq!(result, None);

        let player = mgr.get_entity(1).unwrap();
        assert_eq!(player.ai_state(), AiState::Idle);
        assert!(player.threat_list.is_empty());
    }

    #[test]
    fn generate_threat_stays_fighting_on_second_hit() {
        use cimmeria_entity::cell_entity::AiState;
        let mut mgr = make_test_space_mgr_with_npc();

        let _ = generate_threat(&mut mgr, 1, 100, 50.0, AggroCause::Damage);
        assert_eq!(mgr.get_entity(100).unwrap().ai_state(), AiState::Fighting);

        let _ = generate_threat(&mut mgr, 1, 100, 25.0, AggroCause::Damage);
        assert_eq!(mgr.get_entity(100).unwrap().ai_state(), AiState::Fighting);
    }

    // ── generate_threat: returns combat-enter state on first add ──────────

    #[test]
    fn generate_threat_returns_state_on_player_first_aggro() {
        let mut mgr = make_test_space_mgr_with_npc();

        let result = generate_threat(&mut mgr, 1, 100, 50.0, AggroCause::Damage);
        assert!(
            result.is_some(),
            "first aggro must return new state for broadcast"
        );

        let player = mgr.get_entity(1).unwrap();
        assert!(player.threatened_mobs.contains(&100));
        assert_ne!(player.state_field & BSF_IN_COMBAT, 0);
    }

    #[test]
    fn generate_threat_returns_none_on_subsequent_hits() {
        let mut mgr = make_test_space_mgr_with_npc();

        let _ = generate_threat(&mut mgr, 1, 100, 50.0, AggroCause::Damage);
        // Second hit on same mob — already in set, no transition.
        let result = generate_threat(&mut mgr, 1, 100, 30.0, AggroCause::Damage);
        assert_eq!(result, None);
    }

    #[test]
    fn generate_threat_returns_none_when_attacker_is_npc() {
        let mut mgr = make_test_space_mgr_with_npc();
        add_npc(&mut mgr, 101, 25.0);

        // NPC 101 attacking NPC 100 (e.g., pet) — NPC 100 enters Fighting,
        // but no player is involved so no combat-enter broadcast.
        let result = generate_threat(&mut mgr, 101, 100, 50.0, AggroCause::Damage);
        assert_eq!(result, None);

        // NPC 100's threat_list still got the NPC attacker so AI works.
        assert!(mgr.get_entity(100).unwrap().threat_list.contains_key(&101));
    }

    // ── End-to-end lifecycle ───────────────────────────────────────────────

    #[test]
    fn lifecycle_one_mob_aggro_kill_clears_combat() {
        let mut mgr = make_test_space_mgr_with_npc();

        // Aggro
        let entered = generate_threat(&mut mgr, 1, 100, 50.0, AggroCause::Damage);
        assert!(entered.is_some(), "first aggro should signal combat enter");
        assert_ne!(mgr.get_entity(1).unwrap().state_field & BSF_IN_COMBAT, 0);

        // Kill
        let to_broadcast = clear_dead_npc_from_all_player_threat(&mut mgr, 100);
        assert_eq!(to_broadcast.len(), 1);
        assert_eq!(mgr.get_entity(1).unwrap().state_field & BSF_IN_COMBAT, 0);
        assert!(mgr.get_entity(1).unwrap().threatened_mobs.is_empty());
    }

    #[test]
    fn lifecycle_two_mobs_kill_one_then_other() {
        let mut mgr = make_test_space_mgr_with_npc();
        add_npc(&mut mgr, 101, 25.0);

        // First aggro: combat enters.
        let entered = generate_threat(&mut mgr, 1, 100, 50.0, AggroCause::Damage);
        assert!(entered.is_some());

        // Second aggro on different mob: no transition, no broadcast.
        let second = generate_threat(&mut mgr, 1, 101, 30.0, AggroCause::Damage);
        assert_eq!(second, None);
        assert_eq!(mgr.get_entity(1).unwrap().threatened_mobs.len(), 2);

        // Kill mob 100: still in combat (mob 101 is alive).
        let after_first_kill = clear_dead_npc_from_all_player_threat(&mut mgr, 100);
        assert!(after_first_kill.is_empty());
        let player = mgr.get_entity(1).unwrap();
        assert_eq!(player.threatened_mobs.len(), 1);
        assert_ne!(player.state_field & BSF_IN_COMBAT, 0);

        // Kill mob 101: combat clears now.
        let after_second_kill = clear_dead_npc_from_all_player_threat(&mut mgr, 101);
        assert_eq!(after_second_kill.len(), 1);
        assert_eq!(mgr.get_entity(1).unwrap().state_field & BSF_IN_COMBAT, 0);
        assert!(mgr.get_entity(1).unwrap().threatened_mobs.is_empty());
    }
}
