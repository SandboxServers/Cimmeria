//! Combatant state flags (dead/alive) packed into a `stateField` bitmask.
//!
//! From `python/Atrea/enums.py:176-184`. `BSF_*` enum values are bit *indices*;
//! `setStateFlag(flag)` does `stateField |= 1 << flag`. BSF_Dead = 0 → value 1.

/// Bitmask for dead state in combatantState (1 << BSF_Dead).
pub const PLAYER_STATE_DEAD: u32 = 1;

// The `BSF_*` bits are wire contract, shared with the base: they live in
// `cimmeria_wire::state_field` and are re-exported here at their old paths.
pub use cimmeria_wire::state_field::{
    BSF_AUTO_CYCLING, BSF_DEAD, BSF_DEAD_BIT, BSF_IN_COMBAT, BSF_MOVEMENT_LOCK,
};

/// Check if a state field indicates the entity is dead. Reads are fine
/// against the raw `state_field` bitmask — it's the writes that need to
/// route through the ref-counted helpers.
pub fn is_dead_state(state_field: u32) -> bool {
    state_field & BSF_DEAD != 0
}

/// Apply the NPC-side death-state mutations on a cell entity.
///
/// Single home for the contract every NPC-kill path must satisfy:
/// - Set `BSF_DEAD` and `BSF_MOVEMENT_LOCK` (counted, so `clear_all_state_flags`
///   on respawn drains the counters).
/// - Transition `ai_state` to `Dead` so the AI tick stops scheduling actions.
/// - Stamp `respawn_at = now + respawn_secs` when `respawn_secs.is_some()`;
///   leave `respawn_at = None` for one-shot mobs. The respawn tick promotes
///   the entity back to `Idle` once the deadline passes.
/// - Clear `last_movement_type` so the next AI state entry records its
///   movement type afresh (a server-side cache; nothing is sent, NA10).
/// - Clear `nav_path` + zero `velocity` so the corpse stops mid-pathfind.
///
/// **Does not** clear `threat_list` — the death-side wire helper
/// (`cell::abilities::death::apply_death_transition`) walks it to drain
/// each aggroed player's `threatened_mobs` set and broadcast the
/// `BSF_InCombat` clear. Wiping it here would leave every aggroed
/// player permanently in-combat.
///
/// **Does not** fire any wire packets. Caller is responsible for the
/// `apply_death_transition` burst (reticle clear, threat fanout, loot
/// drop, state-field broadcast) — splitting state mutation from wire
/// dispatch keeps the helper safe to call from non-async contexts and
/// keeps the wire-order invariants visible at the kill site.
///
/// Player entities are out of scope — the player-side death path has
/// its own concerns (weapon-action cleanup, defeat window, respawner
/// list). Callers must gate on `!entity.is_player` before invoking.
///
/// `world` labels the `npc_ai.transition` row; resolve it with
/// `npc_ai::world_label` before taking the `&mut` borrow.
pub fn mark_npc_dead(entity: &mut cimmeria_entity::cell_entity::CellEntity, world: &str) {
    mark_npc_dead_with_kill(entity, world, false);
}

/// [`mark_npc_dead`] for a kill whose killer is known. `player_side` is
/// whether a player or a player's pet took part in the kill (the attacker
/// is the caller's to say); the respawn row is INFO when it is, or when the
/// dead entity is itself a pet, and DEBUG for an NPC-only kill (TG-CMB-09).
pub fn mark_npc_dead_with_kill(
    entity: &mut cimmeria_entity::cell_entity::CellEntity,
    world: &str,
    player_side: bool,
) {
    use crate::cell::service::npc_ai::{set_ai_state_on, AiTransitionReason};
    use cimmeria_entity::cell_entity::{AiState, PetState};

    entity.set_state_flag(BSF_DEAD);
    entity.set_state_flag(BSF_MOVEMENT_LOCK);
    set_ai_state_on(entity, world, AiState::Dead, AiTransitionReason::Died);
    if let Some(secs) = entity.respawn_secs {
        entity.respawn_at =
            Some(std::time::Instant::now() + std::time::Duration::from_secs(secs as u64));
        let entity_name =
            cimmeria_cell_world::cell::space_manager::EntityNames::of(entity).entity_name;
        let template_name =
            cimmeria_cell_world::cell::effects::content_names::template_name(entity.template_id);
        let space_id = entity.space_id.0;
        // `tracing` takes a constant level, so the two levels are two calls.
        if player_side || entity.extensions.contains::<PetState>() {
            tracing::info!(
                event = "respawn_scheduled",
                entity_id = entity.entity_id.0,
                entity_name,
                template_id = entity.template_id,
                template_name,
                respawn_secs = secs,
                world,
                space_id,
                "NPC death: respawn scheduled",
            );
        } else {
            tracing::debug!(
                event = "respawn_scheduled",
                entity_id = entity.entity_id.0,
                entity_name,
                template_id = entity.template_id,
                template_name,
                respawn_secs = secs,
                world,
                space_id,
                "NPC death: respawn scheduled",
            );
        }
    } else {
        // TG-NPC-04: a corpse that will not come back says why. `content_spawn`
        // is a content-placed NPC with no spawn row (`spawn_id` is None).
        let reason = if entity.spawn_id.is_none() {
            "content_spawn"
        } else {
            "no_respawn_secs"
        };
        let entity_name =
            cimmeria_cell_world::cell::space_manager::EntityNames::of(entity).entity_name;
        let template_name =
            cimmeria_cell_world::cell::effects::content_names::template_name(entity.template_id);
        tracing::info!(
            target: "npc_ai.respawn",
            event = "respawn_not_scheduled",
            reason,
            entity_id = entity.entity_id.0,
            entity_name,
            template_id = entity.template_id,
            template_name,
            world,
            space_id = entity.space_id.0,
            "NPC death: respawn not scheduled",
        );
    }
    entity.last_movement_type = None;
    // The Dead transition above already stopped the corpse; this covers a
    // re-kill of an entity that was already Dead (no state change, no stop).
    crate::cell::service::npc_ai::stop_movement_on(entity);
}

#[cfg(test)]
mod tests {
    use super::*;
    use cimmeria_common::{EntityId, SpaceId, Vector3};
    use cimmeria_entity::cell_entity::CellEntity;

    fn entity() -> CellEntity {
        CellEntity::new(EntityId(1), SpaceId(0), Vector3::new(0.0, 0.0, 0.0))
    }

    /// Stand-in test flag used to exercise the generic ref-counted helpers
    /// against a flag that's neither `BSF_DEAD` nor `BSF_MOVEMENT_LOCK`.
    /// Bit 12 sits in the wire-dead range (bits 8-31; client only
    /// dispatches on bits 0-7 per `docs/architecture/state-field-bits.md`),
    /// so these tests pin the counter map's bounded-growth invariants
    /// without affecting any client-visible behavior.
    const BSF_TEST_UNUSED_BIT_12: u32 = 1 << 12;

    #[test]
    fn dead_state_via_entity_helpers() {
        let mut e = entity();
        assert!(!is_dead_state(e.state_field));

        e.set_state_flag(BSF_DEAD);
        assert!(is_dead_state(e.state_field));
        assert_eq!(e.state_field, BSF_DEAD);

        e.unset_state_flag(BSF_DEAD);
        assert!(!is_dead_state(e.state_field));
        assert_eq!(e.state_field, 0);
    }

    #[test]
    fn refcount_keeps_flag_set_after_partial_unset() {
        // Multi-source semantics: two stuns both setting BSF_MovementLock
        // bump the counter to 2; clearing one MUST keep the bit set until
        // the second clear drains the counter.
        let mut e = entity();
        assert!(
            e.set_state_flag(BSF_MOVEMENT_LOCK),
            "first set should transition"
        );
        assert!(
            !e.set_state_flag(BSF_MOVEMENT_LOCK),
            "second set must not re-fire"
        );
        assert!(e.has_state_flag(BSF_MOVEMENT_LOCK));

        assert!(
            !e.unset_state_flag(BSF_MOVEMENT_LOCK),
            "first unset should not transition"
        );
        assert!(
            e.has_state_flag(BSF_MOVEMENT_LOCK),
            "flag must remain set with one source still active"
        );

        assert!(
            e.unset_state_flag(BSF_MOVEMENT_LOCK),
            "second unset should clear"
        );
        assert!(!e.has_state_flag(BSF_MOVEMENT_LOCK));
    }

    #[test]
    fn unset_at_zero_is_noop() {
        // Defensive: a stray clear without a prior set must not underflow
        // (would silently flip the bit on the next set if it did).
        let mut e = entity();
        assert!(
            !e.unset_state_flag(BSF_DEAD),
            "unset on empty counter is a no-op"
        );
        assert_eq!(e.state_field, 0);
    }

    #[test]
    fn unset_on_unowned_flag_does_not_grow_counter_map() {
        // Hot paths that defensively unset flags they may not own (e.g.
        // npc_ai cleanup loops) shouldn't leak map entries — best-effort
        // clears must stay zero-cost.
        let mut e = entity();
        for _ in 0..100 {
            e.unset_state_flag(BSF_DEAD);
            e.unset_state_flag(BSF_MOVEMENT_LOCK);
            e.unset_state_flag(BSF_TEST_UNUSED_BIT_12);
        }
        assert!(
            e.state_flag_counts.is_empty(),
            "no map entries from stray unsets"
        );
    }

    #[test]
    fn drained_counter_drops_map_entry() {
        // After a balanced set/unset pair the counter map should hold no
        // residual zero-count entry. Otherwise the map grows without bound
        // for any flag ever touched, even ones currently unheld.
        let mut e = entity();
        e.set_state_flag(BSF_DEAD);
        e.unset_state_flag(BSF_DEAD);
        assert!(
            e.state_flag_counts.is_empty(),
            "drained counter should be removed"
        );
    }

    #[test]
    fn clear_all_resets_counters_too() {
        // Respawn path: hard reset must drop counters, otherwise the next
        // `unset_state_flag` for an unrelated source would still see leftover
        // refs and skip the bit transition.
        let mut e = entity();
        e.set_state_flag(BSF_DEAD);
        e.set_state_flag(BSF_MOVEMENT_LOCK);
        e.set_state_flag(BSF_MOVEMENT_LOCK); // counter = 2
        e.clear_all_state_flags();
        assert_eq!(e.state_field, 0);
        // Subsequent set should transition cleanly (counter starts at 0).
        assert!(e.set_state_flag(BSF_MOVEMENT_LOCK));
    }

    #[test]
    fn independent_flags_dont_share_counters() {
        // Pin: counter map is keyed by mask, so each flag has its own
        // counter. Clearing one must not affect the other.
        let mut e = entity();
        e.set_state_flag(BSF_DEAD);
        e.set_state_flag(BSF_TEST_UNUSED_BIT_12);
        e.unset_state_flag(BSF_DEAD);
        assert!(!e.has_state_flag(BSF_DEAD));
        assert!(e.has_state_flag(BSF_TEST_UNUSED_BIT_12));
    }

    #[test]
    fn death_without_respawn_secs_logs_reason() {
        use crate::test_support::LogCapture;
        // A content-placed NPC (no spawn row) and a spawned NPC with no timer
        // each give a not-scheduled INFO row with its own `reason`.
        let capture = LogCapture::install();
        let mut content = entity();
        mark_npc_dead(&mut content, "NA26");
        let row = capture
            .find_event(
                tracing::Level::INFO,
                "respawn not scheduled",
                "content_spawn",
            )
            .expect("content-placed death without respawn_secs must log INFO");
        assert_eq!(row.target, "npc_ai.respawn");
        assert!(row.has_field("event", "respawn_not_scheduled"));
        assert!(row.has_field("world", "NA26"));
        assert!(row.has_field("space_id", "0"));
        assert!(row.has_field("entity_id", "1"));

        let mut spawned = entity();
        spawned.spawn_id = Some(7);
        mark_npc_dead(&mut spawned, "NA26");
        let row = capture
            .find_event(
                tracing::Level::INFO,
                "respawn not scheduled",
                "no_respawn_secs",
            )
            .expect("spawned death without respawn_secs must log INFO");
        assert!(row.has_field("event", "respawn_not_scheduled"));
    }

    #[test]
    fn respawn_scheduled_row_carries_world() {
        use crate::test_support::LogCapture;
        let capture = LogCapture::install();
        let mut e = entity();
        e.respawn_secs = Some(30);
        mark_npc_dead_with_kill(&mut e, "NA26", true);
        let row = capture
            .find_message(tracing::Level::INFO, "respawn scheduled")
            .expect("player-side kill with respawn_secs must log INFO");
        assert!(row.has_field("event", "respawn_scheduled"));
        assert!(row.has_field("world", "NA26"));
        assert!(row.has_field("space_id", "0"));
        assert!(row.has_field("respawn_secs", "30"));
    }
}
