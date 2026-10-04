//! Client-entity resync — the per-entity caches the client keeps on its
//! player entity and rebuilds only from the server.
//!
//! Two callers share these helpers:
//!
//! - `service::base_messages::player_init::handle_init_player_state` on
//!   initial login / gate arrival, where the client entity has just been
//!   created for the first time.
//! - [`super::handle_respawn`] after the same-world reanchor. The
//!   reanchor burst re-issues `CREATE_BASE_PLAYER` to drop ragdoll, and the
//!   client answers by destroying and re-creating its player entity, which
//!   empties every cache the login burst had populated. The inventory
//!   snapshot was the first casualty found (empty bag until relog), the
//!   region-hint list the second (no region triggers for the rest of the
//!   session). Both are already replayed after the reanchor; this module
//!   covers the rest of the same wipe: the hotbar ability list, the active
//!   bandolier slot, the mission journal and the `state_field`.

use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// `origin` of the `abilities.wire` rows the respawn resync writes.
pub const ORIGIN_RESPAWN_RESYNC: &str = "respawn_resync";

/// The resync's methods that belong to the ability telemetry set and so
/// go through the wire ledger (`docs/analysis/ability-mechanics/telemetry-coverage.md`).
fn ledgered(method_index: u16) -> bool {
    use crate::cell::client_methods::{being, combatant, player};
    matches!(
        method_index,
        being::ON_STATE_FIELD_UPDATE
            | combatant::ON_STAT_UPDATE
            | combatant::ON_STAT_BASE_UPDATE
            | player::ON_ABILITY_TREE_INFO
    )
}

/// Re-send `onActiveSlotUpdate` for the bandolier.
///
/// On login this is a defensive resync against a client-side initialization
/// race: the login burst already carries the packet, but the client's NetIn
/// handler silently no-ops when the bag-list map is not yet initialized,
/// leaving the cached active slot at 0 and the `ActivateBandolierSlotN` Lua
/// gate misfiring ("F2 doesn't swap to the P90"). Sent after `onClientReady`
/// the bag list is guaranteed to exist. After a pawn recreate it is the only
/// copy the new entity ever gets. Ghidra detail in
/// docs/reverse-engineering/findings/client-wire-emit-suppression.md.
///
/// Wire: bag_id (i32 LE) + (slot_id + 1) (i32 LE, 1-indexed) = 8 bytes.
pub async fn send_active_slot_resend(
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    const CONTAINER_BANDOLIER: i32 = 3;
    let active_slot = space_mgr
        .get_entity(entity_id)
        .map(|e| e.active_bandolier_slot)
        .unwrap_or(0);
    let mut args = Vec::with_capacity(8);
    args.extend_from_slice(&CONTAINER_BANDOLIER.to_le_bytes());
    args.extend_from_slice(&(active_slot + 1).to_le_bytes());
    crate::cell::abilities::send_entity_method(
        entity_id,
        crate::cell::client_methods::inventory::ON_ACTIVE_SLOT_UPDATE,
        args,
        tx,
        space_mgr,
    )
    .await;
    tracing::info!(
        target: "bandolier.resend",
        entity_id,
        active_slot,
        "Re-sent onActiveSlotUpdate post-onClientReady (defensive resync \
         against client bag-list init race — see \
         docs/reverse-engineering/findings/client-wire-emit-suppression.md)"
    );
}

/// What [`resync_after_pawn_recreate`] replays, in send order. Logged as
/// the `replayed` field so SigNoz shows exactly what the new pawn got.
pub(crate) const RESYNC_REPLAYED: &str = "level,state_field,stats,base_stats,archetype,\
                                          ability_tree,known_abilities,active_slot,missions";

/// Replay the per-entity client caches after a `CREATE_BASE_PLAYER`
/// re-issue (same-world respawn reanchor).
///
/// The client destroys its player entity and builds a new one, so this
/// replays the login burst's player-state half
/// (`wire::mercury::world_data::map_loaded`) from the live cell entity:
///
/// 1. `onLevelUpdate`.
/// 2. `onStateFieldUpdate(state_field)`, always. The client's handler
///    stores the value it is given and fires side effects for the bits
///    that differ from its cached copy (0 on a new entity), so the full
///    field is correct whatever the client held before.
/// 3. `onStatUpdate` with every stat (HEALTH and FOCUS at max after the
///    respawn reset), then `onStatBaseUpdate`.
/// 4. `onArchetypeUpdate`.
/// 5. `onAbilityTreeInfo`, from the same catalog the trainer reads.
/// 6. `onKnownAbilitiesUpdate` (the hotbar), `onActiveSlotUpdate` (the
///    bandolier slot) and the mission journal.
///
/// Without 1-5 the new pawn had no stats, no archetype and no tree, and
/// the client sent no hotbar `useAbility` for the rest of the session
/// (2026-09-28 colo playtest, 16:54-18:27). Region hints and the inventory
/// snapshot are not repeated here: the respawn handler queues both right
/// behind the reanchor.
///
/// Every message targets the player's own entity and is queued on the
/// same reliable channel *after* the reanchor, so it lands once the
/// client's creation transaction has settled — the same reason the
/// reanchor's appearance replay travels as a separate bundle.
pub(crate) async fn resync_after_pawn_recreate(
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    use crate::cell::client_methods::{being, combatant, player};

    let Some(entity) = space_mgr.get_entity(entity_id) else {
        tracing::warn!(
            entity_id,
            reason = "entity_missing",
            "resync after pawn recreate skipped — entity not found"
        );
        return;
    };
    let state_field = entity.state_field;
    let level = entity.level;
    let archetype_id = entity.archetype_id.unwrap_or(0);
    let stats = entity.stats.serialize_all();
    let base_stats = entity.stats.serialize_all_base();
    let tree = crate::ability_tree::tree_info(
        &space_mgr.ability_tree_catalog,
        archetype_id,
        entity.player_id.unwrap_or(0),
    )
    .serialize();

    let player_state: [(u16, Vec<u8>); 6] = [
        (
            being::ON_LEVEL_UPDATE,
            (level as i32).to_le_bytes().to_vec(),
        ),
        (
            being::ON_STATE_FIELD_UPDATE,
            state_field.to_le_bytes().to_vec(),
        ),
        (combatant::ON_STAT_UPDATE, stats),
        (combatant::ON_STAT_BASE_UPDATE, base_stats),
        (
            combatant::ON_ARCHETYPE_UPDATE,
            archetype_id.to_le_bytes().to_vec(),
        ),
        (player::ON_ABILITY_TREE_INFO, tree),
    ];
    for (method_index, args) in player_state {
        // The ability methods write their `abilities.wire` row (AB-C7), so
        // a respawn's stat and tree replay is accounted for like a cast's.
        if ledgered(method_index) {
            crate::cell::abilities::send_entity_method_ledgered(
                entity_id,
                method_index,
                args,
                crate::cell::abilities::WireRoute::EntityDefault,
                crate::cell::abilities::WireCtx::new(ORIGIN_RESPAWN_RESYNC),
                tx,
                space_mgr,
            )
            .await;
        } else {
            crate::cell::abilities::send_entity_method(
                entity_id,
                method_index,
                args,
                tx,
                space_mgr,
            )
            .await;
        }
    }

    send_known_abilities_update(entity_id, ORIGIN_RESPAWN_RESYNC, tx, space_mgr).await;
    send_active_slot_resend(entity_id, tx, space_mgr).await;
    crate::cell::missions::resend_missions(entity_id, tx, space_mgr).await;

    let id = space_mgr.player_identity(entity_id);
    tracing::info!(
        entity_id,
        account_id = id.account_id,
        player_id = id.player_id,
        level,
        archetype_id,
        state_field,
        state_field_names = %cimmeria_wire::state_field::STATE_FLAGS.render(state_field),
        replayed = RESYNC_REPLAYED,
        "Resynced client entity state after pawn recreate"
    );
}

/// Send `onKnownAbilitiesUpdate(ARRAY<INT32> abilities)` to the player so the
/// hotbar UI populates at world entry. Without this, characters with starter
/// abilities (every char_def_id after Phase 2) still showed an empty
/// hotbar because the client only gets ability state via this RPC and the
/// previous code never sent it on login.
///
/// Mirrors Python `SGWPlayer.addAbility` which calls `onKnownAbilitiesUpdate`
/// on every add (`deprecated/python/cell/SGWPlayer.py:861`). We send the full
/// list once at world entry so the in-game add path can stay event-driven.
///
/// Wire format: `ARRAY<INT32> AbilityData` → `u32 count` + N × `i32 ability_id`.
/// Method index 101 (`ON_KNOWN_ABILITIES_UPDATE`).
///
/// `origin` names the trigger on the `abilities.wire` row (AB-C7):
/// `world_entry`, `respawn_resync`, `ability_granted`, `gm_ability_granted`,
/// `gm_abilities_changed`, `respec`.
pub async fn send_known_abilities_update(
    entity_id: u32,
    origin: &'static str,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let ability_ids: Vec<i32> = match space_mgr.get_entity(entity_id) {
        Some(e) => e.abilities.known_ability_ids(),
        None => return,
    };

    let mut args = Vec::with_capacity(4 + ability_ids.len() * 4);
    args.extend_from_slice(&(ability_ids.len() as u32).to_le_bytes());
    for id in &ability_ids {
        args.extend_from_slice(&id.to_le_bytes());
    }

    let delivery = crate::cell::abilities::send_entity_method_ledgered(
        entity_id,
        crate::cell::client_methods::player::ON_KNOWN_ABILITIES_UPDATE,
        args,
        // The player's own client, as before: a player-only method.
        crate::cell::abilities::WireRoute::SelfOnly,
        crate::cell::abilities::WireCtx::new(origin),
        tx,
        space_mgr,
    )
    .await;
    match delivery.failed {
        0 => {
            tracing::info!(
                entity_id,
                count = ability_ids.len(),
                "Sent onKnownAbilitiesUpdate (hotbar seed)"
            );
        }
        _ => {
            // Channel closed — the player's hotbar will be empty until
            // they reconnect. Log at error so the symptom ("no abilities
            // on the bar") has a corresponding server-side log entry.
            tracing::error!(
                entity_id,
                count = ability_ids.len(),
                origin,
                "Failed to send onKnownAbilitiesUpdate (hotbar seed) — cell→base channel closed"
            );
        }
    }
}
