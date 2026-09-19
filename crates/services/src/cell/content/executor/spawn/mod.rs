//! Entity-lifecycle action handlers: `Action::SpawnEntity`,
//! `Action::DespawnEntity`, and the shared despawn routine that
//! `Action::DestroyTaggedEntity` also uses (Harset H03, closing audit
//! defect H-B6 / overlap U5).
//!
//! Staged in its own module rather than in [`super::world`] because these
//! two verbs are the only ones that create or remove a world entity, and
//! because the spawn arm carries a policy layer (instanced-world refusal,
//! same-tag idempotence, respawn override) that has no analogue in the
//! flag-flipping world actions.

use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::{DespawnOutcome, SpaceManager};

// `pub(super)` so the executor's negative-logging guards can reuse the
// prototype-`SpawnRecord` fixture below instead of duplicating a 30-field
// struct literal that would break twice whenever `SpawnRecord` gains a field.
#[cfg(test)]
pub(super) mod tests;

/// `Action::SpawnEntity` — instantiate an `entity_templates` row into the
/// **acting player's current space**, tagged so `entity_dead_tag` /
/// `interact_tag` / `despawn_entity` chains can find it again.
///
/// The seed row never names a space. Mission content targets per-player
/// instanced worlds (Harset Market 69 / Storage 70, Castle Cellblock 12),
/// and a chain has no way to know which instance the firing player is in —
/// so the space is read off the triggering entity. Four guards follow from
/// that, in order:
///
/// 1. **Player-source.** Cover-node and NPC-death chains fire with an NPC
///    as the acting entity. An NPC source still resolves to a valid space,
///    so the spawn would *succeed* — just in whichever instance that NPC
///    happens to be in, which is not what "the acting player's space" means.
///    Refused.
/// 2. **Template present.** A cache miss means the `entity_templates` row
///    doesn't exist, or failed to decode at startup.
/// 3. **Shared-world.** Spawning into a non-instanced world drops a mission
///    NPC into the shared hub, where every other player sees it and — for a
///    hostile — is attacked by it. This is the guardrail behind the campaign
///    rule "mission-scoped hostile NPCs are spawned into the player's own
///    instance and never into world 57 or 68". `allow_shared: true` opts out.
/// 4. **Same-tag idempotence.** Relog-restore chains re-fire their step's
///    actions by design, so a second spawn with a tag already live in this
///    space is a no-op. The lookup deliberately matches dead entities too:
///    a corpse still holds its tag, and resurrecting a mission NPC the
///    player already killed would re-open completed content.
///
/// Note for wave content (H23 / H25): the idempotence guard is per-tag, so
/// N simultaneous spawns need N distinct tags (`ra_infiltrator_1`, `_2`, …),
/// one `spawn_entity` row each. A shared tag would both trip this guard and
/// make `despawn_entity` reach only one of them.
pub(super) async fn spawn_entity(
    template_id: i32,
    position: [f32; 3],
    heading: f32,
    tag: String,
    is_stationary: Option<bool>,
    aggression: Option<i32>,
    allow_shared: Option<bool>,
    entity_id: u32,
    chain_id: i64,
    space_mgr: &mut SpaceManager,
) {
    // ── 1. The acting entity must be a player ──
    let Some(actor) = space_mgr.get_entity(entity_id) else {
        tracing::warn!(
            entity_id, template_id, %tag, chain_id,
            reason = "actor_missing",
            "spawn_entity: the triggering entity is not in any space -- nothing spawned"
        );
        return;
    };
    if !actor.is_player {
        tracing::warn!(
            entity_id, template_id, %tag, chain_id,
            reason = "actor_not_player",
            "spawn_entity: the triggering entity is not a player, so \"the acting \
             player's space\" is undefined -- nothing spawned"
        );
        return;
    }

    let Some(space_id) = space_mgr.get_entity_space_id(entity_id) else {
        tracing::warn!(
            entity_id, template_id, %tag, chain_id,
            reason = "space_unresolved",
            "spawn_entity: could not resolve the acting player's space -- nothing spawned"
        );
        return;
    };
    let world_name = space_mgr
        .get_entity_world_name(entity_id)
        .unwrap_or_default();

    // ── 2. Template must be in the startup cache ──
    let Some(prototype) = space_mgr.spawn_templates.get(&template_id) else {
        tracing::warn!(
            entity_id, template_id, %tag, %world_name, chain_id,
            reason = "template_not_cached",
            "spawn_entity: template id is not in the entity_templates cache \
             (missing row, or the row failed to decode at startup) -- nothing spawned"
        );
        return;
    };
    let template_faction = prototype.faction.unwrap_or(0);

    // ── 3. Shared-world refusal ──
    if !space_mgr.is_world_instanced(&world_name) && allow_shared != Some(true) {
        tracing::warn!(
            entity_id, template_id, %tag, %world_name, space_id, chain_id,
            reason = "shared_world_refused",
            "spawn_entity: refusing to spawn into a non-instanced world -- every \
             player in the hub would see (and a hostile would attack) this \
             mission NPC. Set allow_shared=true on the action row if the spawn \
             really is meant to be world-visible."
        );
        return;
    }

    // ── 4. Same-tag idempotence ──
    if let Some(existing) = space_mgr.find_entity_by_tag(entity_id, &tag) {
        tracing::warn!(
            entity_id, template_id, %tag, %world_name, space_id, existing, chain_id,
            reason = "tag_already_live",
            "spawn_entity: an entity with this tag is already in the player's \
             space -- no-op. This is the expected path for a relog-restore \
             chain re-firing its step's actions."
        );
        return;
    }

    // No `respawn_secs` parameter by design: a content-scoped spawn is
    // always one-shot (see `SpaceManager::spawn_npc_from_template`), so
    // the value never survived to the entity. A seed row that supplies it
    // still loads and still spawns; the loader warns once at load time
    // rather than this executor warning on every fire (PR #662 review,
    // finding 5 — see `content_engine::loader::action_spawn`).

    let agg = aggression.unwrap_or(0);
    // Players are never assigned a faction (`CellEntity::new` sets 0 and no
    // world-entry path writes it), and `npc_ai_idle_auto_aggro` skips any
    // candidate whose faction equals the NPC's. A hostile template with
    // faction 0 therefore never aggros, silently. Surface it at spawn time
    // so the content author sees it next to the row that caused it.
    if agg > 0 && template_faction == 0 {
        tracing::warn!(
            entity_id, template_id, %tag, chain_id,
            reason = "aggressive_spawn_faction_zero",
            "spawn_entity: aggression > 0 on a template whose faction is 0 (or \
             NULL) -- auto-aggro compares NPC faction against the player's, and \
             players are always faction 0, so this NPC will never attack. Give \
             the entity_templates row a non-zero faction."
        );
    }

    match space_mgr.spawn_npc_from_template(
        template_id,
        space_id,
        &world_name,
        position,
        heading,
        &tag,
        is_stationary.unwrap_or(false),
        agg,
    ) {
        Ok(npc_entity_id) => {
            // No explicit AoI introduction: the 100ms AoI tick recomputes
            // each connected player's `current_aoi` from the spatial grid
            // every pass and diffs it against their stored witness set, so
            // a mid-session insert is picked up like any entity crossing
            // the radius. Same contract the GM `GmSpawnNpcReady` handler
            // relies on. `spawn_npc_from_record_into` does both required
            // inserts (spatial grid *and* `space.entities`).
            tracing::info!(
                entity_id, npc_entity_id, template_id, %tag, %world_name,
                space_id, ?position, heading, aggression = agg,
                is_stationary = is_stationary.unwrap_or(false), chain_id,
                "Content: spawned mission entity"
            );
        }
        Err(e) => {
            tracing::warn!(
                entity_id, template_id, %tag, %world_name, space_id, chain_id,
                reason = "spawn_failed",
                "spawn_entity: spawn into the player's space failed: {e}"
            );
        }
    }
}

/// Shared implementation of `Action::DespawnEntity` and
/// `Action::DestroyTaggedEntity` — remove the tagged entity and tell every
/// current witness it left.
///
/// Routes through [`SpaceManager::despawn_npc`] rather than the bare
/// `destroy_entity` the content executor used before (audit defect H-B6,
/// overlap U5). `destroy_entity` alone drops the entity from
/// `space.entities` and the spatial grid but leaves its id sitting in every
/// observer's `witnesses` set: the next AoI tick *would* eventually emit
/// `LeftAoI`, but only for players it happens to visit, only after up to a
/// full tick of the client rendering a ghost, and only while the observer is
/// still in the space. That is the #582 invisible-corpse shape, and GM
/// `.despawn` has always done it correctly.
pub(super) async fn despawn_by_tag(
    entity_tag: String,
    entity_id: u32,
    chain_id: i64,
    verb: &'static str,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(target_id) = space_mgr.find_entity_by_tag(entity_id, &entity_tag) else {
        tracing::debug!(
            entity_id, %entity_tag, chain_id, verb,
            "Content: entity tag not found for despawn"
        );
        return;
    };
    match space_mgr.despawn_npc(target_id, tx).await {
        DespawnOutcome::Despawned { witnesses_notified } => {
            tracing::info!(
                entity_id, %entity_tag, target_id, witnesses_notified, chain_id, verb,
                "Content: despawned tagged entity"
            );
        }
        DespawnOutcome::NotFound => {
            // The tag resolved a moment ago, so this means the entity left
            // the space between the lookup and the despawn.
            tracing::warn!(
                entity_id, %entity_tag, target_id, chain_id, verb,
                reason = "despawn_target_vanished",
                "Content: despawn target disappeared between tag lookup and despawn"
            );
        }
        DespawnOutcome::RefusedPlayer => {
            // Structurally unreachable today — player entities carry no
            // `tag` (tags come only from `spawnlist.tag` at NPC spawn), so
            // `find_entity_by_tag` cannot resolve one. Logged rather than
            // discarded because if it ever does fire, a content chain came
            // one step from destroying a logged-in player's cell entity.
            tracing::warn!(
                entity_id, %entity_tag, target_id, chain_id, verb,
                reason = "despawn_refused_player",
                "Content: despawn refused -- the tag resolved to a PLAYER entity"
            );
        }
    }
}
