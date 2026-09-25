//! `npc_ai.idle_parked`: an NPC was put into Idle away from its spawn, and
//! the AI tick will never visit it again (audit S6 + A1).
//!
//! A fight that ends by losing its target sets Idle wherever the NPC stands.
//! With no hostility (NA13), patrol or wander the dispatcher never admits an Idle
//! NPC, so it stays frozen there even when the player walks back up to it.
//! After NA12 this should read zero.

use cimmeria_entity::cell_entity::{AiState, CellEntity};

/// Further than this from spawn counts as "parked away from home".
pub(in crate::cell) const IDLE_PARKED_MIN_DIST: f32 = 2.0;

/// Whether the AI dispatcher admits an Idle NPC. Mirrors the admission
/// filter in `npc_ai::dispatch::npc_ai_tick`; the `idle_parked` guard test
/// pins the two together.
pub(in crate::cell) fn idle_is_ticked(npc: &CellEntity) -> bool {
    crate::cell::combat::is_hostile_to_players(npc)
        || !npc.patrol_path.is_empty()
        || npc.wander_radius > 0.0
}

/// Called by the transition helper after every real state change.
pub(in crate::cell) fn check(npc: &CellEntity, world: &str, from: AiState, reason: &'static str) {
    if npc.ai_state() != AiState::Idle || npc.is_player || idle_is_ticked(npc) {
        return;
    }
    let Some(spawn) = npc.spawn_position else {
        return;
    };
    let npc_to_spawn = spawn.distance_to(&npc.position);
    if npc_to_spawn <= IDLE_PARKED_MIN_DIST {
        return;
    }
    cimmeria_observability::counter!(
        "npc_idle_parked_total",
        "world" => world.to_string(),
        "reason" => reason,
    );
    tracing::info!(
        target: "npc_ai.idle_parked",
        event = "idle_parked",
        npc_id = npc.entity_id.0,
        tag = npc.tag.as_deref().unwrap_or(""),
        template_id = npc.template_id.unwrap_or(0),
        world,
        space_id = npc.space_id.0,
        from = from.label(),
        reason,
        npc_to_spawn,
        x = npc.position.x,
        y = npc.position.y,
        z = npc.position.z,
        nav_path_len = npc.nav_path.len(),
        "npc_ai.idle_parked: NPC went Idle away from spawn and will not be ticked \
         again -- it stays frozen here"
    );
}
