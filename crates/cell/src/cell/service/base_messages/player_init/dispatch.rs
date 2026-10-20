//! World-entry snapshot dispatch and player hook after initialization.

use super::super::init_grant_merge;
use super::super::player_init;
use crate::cell::messages::{BaseToCellMsg, CellToBaseMsg};
use crate::cell::space_manager::SpaceManager;
use cimmeria_cell_world::cell::plugin::PlayerHookPoint;
use cimmeria_content_engine::chain::ChainEngine;
use tokio::sync::mpsc;

/// Apply one already-matched player-init message.
pub(in crate::cell::service::base_messages) async fn handle(
    msg: BaseToCellMsg,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) {
    let BaseToCellMsg::InitPlayerState {
        entity_id,
        player_id,
        account_id,
        world_name,
        archetype_id,
        saved_missions,
        mut abilities,
        active_bandolier_slot,
        bandolier_items,
        system_options,
        access_level,
        known_stargates,
        mut tree_progress,
        level,
        character_name,
        body_set,
        looted_containers,
        shown_tutorials,
        entry_cause,
        session_kind,
    } = msg
    else {
        unreachable!("dispatch accepts only InitPlayerState")
    };
    init_grant_merge::merge_into_snapshot(
        entity_id,
        player_id,
        archetype_id,
        &mut tree_progress,
        &mut abilities,
        space_mgr,
    );
    // Cache the display name on the cell entity so cell-side seams (GM
    // `.`-console audit, mission/death/respawn Discord emits) can
    // attribute events to a name — the cell has no other source for it.
    // Set here rather than threaded through `handle_init_player_state`
    // to keep that function under the argument-count lint; the entity
    // already exists (created by the prior `ConnectEntity`).
    if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
        entity.character_name = character_name;
        // Re-assert the log name (Rule 6) from the same source; the
        // login name, which this message lacks, is kept.
        let name = entity.character_name.clone();
        entity.stamp_log_names(name.as_deref(), None);
        // Re-assert the account half of the stable log-correlation
        // identity. `CreateEntity` already stamped it; this is the
        // belt-and-braces path for any create route that didn't, so a
        // session can never reach in-world play with un-attributable
        // logs. Only the account half needs re-asserting here:
        // `handle_init_player_state` below already stamps
        // `player_id` unconditionally, and `account_id` is the one
        // field not threaded into its signature — which, like
        // `character_name`, is at the argument-count lint ceiling.
        entity.account_id = Some(account_id);
        // Address book for the dial gate (CAT-O-01). Stamped here
        // rather than threaded into `handle_init_player_state` for
        // the same reason as `character_name`: that signature is at
        // the `too_many_arguments` ceiling. Assigned unconditionally
        // so a gate arrival — which replays world entry against the
        // freshly-learned list — overwrites the pre-travel snapshot
        // instead of leaving the old one in place.
        entity.known_stargates = known_stargates;
        // The once-per-character loot flags (`open_loot`). Replaced,
        // not merged, for the same reason as the address book: a
        // gate arrival replays world entry from the DB row.
        entity.looted_containers = looted_containers.into_iter().collect();
        // The one-time tutorials (CS-03). Merged, not replaced: the
        // rows are append-only, and a `show_tutorial` whose record
        // is still in flight (the base read this snapshot first)
        // must not lose its mark, or the same tick could ask again.
        entity.shown_tutorials.extend(shown_tutorials);
        // Body set, for the line-of-sight eye height (NA31). Only NPC
        // AoI data reads `body_set` on the wire side, so setting it on
        // a player changes no packet.
        if body_set.is_some() {
            entity.body_set = body_set;
        }
        // Same reason as `known_stargates`: stamped here, not
        // threaded into the argument-capped handler below.
        entity.tree_progress = tree_progress;
        // The trainer's level gate reads this. Before AT-03 nothing
        // stamped a player's level, so every player trained as
        // level 1. `max(1)` keeps a corrupt row from reading as 0.
        entity.level = level.max(1) as u32;
        // Lab or player (TG-NET-17), for `session.start` below and
        // the `playtest.friction` rows. Stamped here, not threaded
        // into the argument-capped handler. Assigned as sent: the
        // account, and so the flag, is the same on a travel re-entry.
        entity.session_kind = session_kind;
        // Fill the friction watch's row context now: `player_tick`
        // first runs on movement, so a chain-fired friction row right
        // after entry would otherwise carry no identity, world or
        // session kind (TG-MIS-02). `player_id` is taken from the
        // message: `handle_init_player_state` stamps it only below.
        crate::cell::playtest_friction::note_session(
            entity_id,
            cimmeria_entity::cell_entity::PlayerIdentity {
                player_id: Some(player_id),
                ..entity.identity()
            },
            crate::cell::space_manager::EntityNames::of(entity).entity_name,
            Some(world_name.as_str()),
            entity.session_kind,
        );
    } else {
        // The entity should already exist (ConnectEntity precedes
        // InitPlayerState). If it doesn't, the name cache silently
        // fails and every cell-side emit for this player degrades to
        // `entity:<id>` — surface the ordering bug rather than hiding it.
        tracing::warn!(
            entity_id,
            entity_name = character_name.as_deref(),
            account_id, // nt:id-only InitPlayerState carries no login name to pair
            player_id,
            player_name = character_name.as_deref(),
            "InitPlayerState: entity absent when caching character_name -- \
                     ConnectEntity ordering bug; cell-side emits will fall back to entity id"
        );
    }
    player_init::handle_init_player_state(
        entity_id,
        player_id,
        world_name,
        archetype_id,
        saved_missions,
        abilities,
        active_bandolier_slot,
        bandolier_items,
        system_options,
        access_level,
        entry_cause,
        tx,
        space_mgr,
        engine,
    )
    .await;
    // Plugin hooks (#962): a gate arrival re-creates the player, and
    // the org plugin re-sends their squad here (ORG-03). After the
    // handler above, which stamps `player_id`.
    space_mgr
        .plugins()
        .clone()
        .run_player_hook(
            PlayerHookPoint::AfterInitPlayerState,
            entity_id,
            player_id,
            tx,
            space_mgr,
        )
        .await;
}
