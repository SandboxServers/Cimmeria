//! `playCharacter` -> entity teardown step.
//!
//! When the client picks a character and calls `playCharacter`, we tear down
//! its previous entity tree by sending RESET_ENTITIES. The next packet from
//! the client is ENABLE_ENTITIES, which drives the create-player step in
//! `enable_entities.rs`.
//!
//! The handler awaits the database and the cell between its entry latch and
//! its writes, and a held `playCharacter` (cooked-data resync) runs on its
//! own task, so the session at `addr` can be torn down or replaced by a
//! relaunch takeover in between. Every later write and send is checked
//! against the session the latch was taken on ([`still_owner`]), never
//! against the address alone.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;
use tokio::sync::mpsc;

use cimmeria_base_session::base::plugin::SessionStateHookPoint;
use cimmeria_entity::manager::EntityManager;

use crate::cell::messages::BaseToCellMsg;
use crate::mercury::build_reset_entities;

use super::super::ConnectedClientState;
use super::lab_fixture;
use super::methods::{query_player_load_data, query_world_entry};

#[cfg(feature = "lab-chaos")]
mod chaos;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Send RESET_ENTITIES when the client calls `playCharacter` to begin world entry.
#[tracing::instrument(
    name = "world_entry.play_character",
    level = "info",
    skip_all,
    fields(peer = %addr, account_id, player_id),
)]
pub async fn handle_play_character(
    transport: &Arc<dyn Transport>,
    addr: SocketAddr,
    key: [u8; 32],
    account_id: u32,
    player_id: i32,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    db_pool: &Option<Arc<PgPool>>,
    entity_manager: &Arc<Mutex<EntityManager>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
) -> Result<(), BoxError> {
    // Guard: only send once per connection. Capture access_level under the
    // same lock so the class-id decision in query_world_entry uses the
    // server-authoritative session value (sourced from account.accesslevel
    // at login), never a client-supplied byte. `cancelled` is the session's
    // own Arc, kept as the token of the session this entry is for.
    let arcs = {
        let mut clients = connected.lock().map_err(|_| "connected lock poisoned")?;
        if let Some(c) = clients.get_mut(&addr) {
            if !c.world_entry_sent {
                c.world_entry_sent = true;
                // A new character enters the world (#962 step 5): the base
                // plugins drop what the connection played before. No
                // crafting stations, tools or "craft anywhere" carry over,
                // and no crafting options go out until this entry's login
                // send. Before the cell entity is created, so every station
                // report that follows is this character's.
                let plugins = c.plugins.clone();
                plugins
                    .run_session_state_hook(SessionStateHookPoint::PlayCharacterAfterEntryLatch, c);
                Some((
                    Arc::clone(&c.pending_acks),
                    Arc::clone(&c.next_seq),
                    c.access_level,
                    c.enc_version,
                    c.account_name.clone(),
                    Arc::clone(&c.cancelled),
                    (c.key == key).then(|| c.wire_send_origin()).flatten(),
                ))
            } else {
                None
            }
        } else {
            tracing::warn!(%addr, "play_character: addr not in connected map");
            None
        }
    };

    let (pending_acks_arc, next_seq, access_level, enc_version, account_name, owner, origin) =
        match arcs {
            Some(a) => a,
            None => return Ok(()),
        };

    // A staged lab fixture is written before the first world-entry read
    // below, so the login and the cell's restore chains run on it.
    lab_fixture::apply_at_play(
        db_pool,
        account_id,
        account_name.as_deref(),
        player_id,
        access_level,
    )
    .await;

    // Query character data from DB and resolve space via CellService.
    // access_level selects the entity class id (SGWPlayer vs SGWGmPlayer).
    let entry_info = query_world_entry(
        db_pool,
        account_id,
        account_name.clone(),
        player_id,
        access_level,
        entity_manager,
        cell_tx,
    )
    .await;

    // query_world_entry returns NO_ENTITY_ID as a "world entry failed"
    // sentinel (DB error or character not found). Bail before dispatching any
    // packets that would target an entity the cell never registered, AND
    // clear the world_entry_sent latch so a subsequent playCharacter retry on
    // the same connection isn't silently dropped. Only this entry's session's
    // latch: a session that replaced it latched its own.
    if entry_info.player_entity_id == super::methods::world_entry_db::NO_ENTITY_ID {
        tracing::error!(
            %addr,
            player_id, // nt:id-only the character row failed to load, so nothing names it
            account_id,
            account_name = account_name.as_deref(),
            "World entry aborted: query_world_entry returned NO_ENTITY_ID sentinel"
        );
        if let Ok(mut clients) = connected.lock() {
            if still_owner(&clients, addr, &owner) {
                if let Some(c) = clients.get_mut(&addr) {
                    c.world_entry_sent = false;
                }
            }
        }
        return Ok(());
    }

    // Lab chaos armed for this account (D-CH10, lab-chaos builds only) is
    // set on the session now, tagged with the new entity: before
    // RESET_ENTITIES goes out, so before the client can send the
    // `onClientReady` whose flush it is meant to disturb. Only while the
    // session at `addr` is still this entry's.
    #[cfg(feature = "lab-chaos")]
    let promotion = chaos::promote_if_owner(
        connected,
        addr,
        &owner,
        account_id,
        entry_info.player_entity_id,
    );

    let entity_id = entry_info.player_entity_id;
    let entered: Result<bool, BoxError> = async {
        // Also query the full player data needed for mapLoaded
        let player_load_data = query_player_load_data(db_pool, account_id, player_id).await;

        // class_id selection: GMs (access_level > 0) come up as
        // SGWGmPlayer (0x03), regular players as SGWPlayer (0x02). Decided in
        // query_world_entry from the session access_level captured above.
        //
        // SGWGmPlayer declares <Parent>SGWPlayer</Parent> with empty
        // <Implements>, so its own methods APPEND at the end of the flattened
        // tables (cell 109+, client 157+); the inherited 0-108 / 0-156
        // indices do NOT renumber, and the wire idbase stays 61. So our
        // SGWPlayer method_idx constants (BeingAppearance=26, etc.) and
        // cell-dispatch ranges remain correct under 0x03 — the earlier
        // "shifts ALL indices" note was wrong about the mechanism. The real
        // reason 0x02 was hardcoded before was twofold and both are now
        // resolved: (a) no handlers existed for the gm* indices 109+, and
        // (b) the cell layer had no access_level to gate them. With the GM
        // gate (gm_gate.rs) covering 109+ and a verified gm* subset wired in
        // the cell router, flipping the class for GMs is safe.

        // Snapshot the world-channel emit fields before the store block below
        // moves `entry_info` / `player_load_data` into the connected state.
        let entry_character_name = player_load_data.player_name.clone();
        let entry_world_name = entry_info.world_name.clone();
        let entry_position = entry_info.pos;

        let player_name = Some(entry_character_name.as_str()).filter(|n| !n.is_empty());
        // The session at `addr` was torn down or replaced in one of the
        // awaits above: nothing of this entry goes to it, neither the
        // RESET_ENTITIES under the old key nor the old character's state.
        let session_gone = |stage: &'static str| {
            tracing::warn!(
                %addr,
                account_id,
                account_name = account_name.as_deref(),
                player_id,
                player_name,
                entity_id,
                entity_name = player_name,
                stage,
                reason = "session_gone_or_replaced",
                "World entry abandoned: the session at this address was torn down or replaced \
                 during playCharacter"
            );
        };
        if !still_owner(
            &*connected.lock().map_err(|_| "connected lock poisoned")?,
            addr,
            &owner,
        ) {
            session_gone("before_reset_entities");
            return Ok(false);
        }

        tracing::info!(
            %addr,
            account_id,
            account_name = account_name.as_deref(),
            player_id,
            player_name,
            entity_id,
            entity_name = player_name,
            space_id = entry_info.space_id,
            world = %entry_info.world_name,
            pos = ?entry_info.pos,
            class_id = entry_info.class_id,
            class_name = cimmeria_wire::names::class_name(entry_info.class_id),
            "World entry: sending RESET_ENTITIES (entity teardown)"
        );

        let acks: Vec<u32> = {
            let mut pending = pending_acks_arc.lock().unwrap();
            cimmeria_mercury::packet::take_piggyback_acks(&mut pending, enc_version)
        };

        // Entity teardown: Send ONLY RESET_ENTITIES.
        // The C++ server sends RESET_ENTITIES in its own flushed bundle. The
        // client tears down all entities, then sends ENABLE_ENTITIES, which
        // triggers the create-player step (CREATE_BASE_PLAYER + viewport +
        // cell + forced position).
        let seq =
            next_seq.fetch_add(1, Ordering::Relaxed) & cimmeria_mercury::packet::SEQUENCE_MASK;
        let pkt = build_reset_entities(&key, seq, &acks, enc_version);
        tracing::trace!(%addr, len = pkt.len(), seq, "UDP_OUT RESET_ENTITIES (entity teardown)");
        super::send_direct(transport, &pkt, addr, origin.as_ref()).await?;

        // Store the world entry info and player load data for the
        // create-player step, on this entry's session only.
        let mut entry_account_name = None;
        {
            let mut clients = connected.lock().map_err(|_| "connected lock poisoned")?;
            if !still_owner(&clients, addr, &owner) {
                drop(clients);
                session_gone("store_world_entry");
                return Ok(false);
            }
            if let Some(c) = clients.get_mut(&addr) {
                entry_account_name = c.account_name.clone();
                c.pending_player_entity_id = Some(entity_id);
                c.player_entity_id = Some(entity_id);
                c.player_name = Some(player_load_data.player_name.clone());
                c.player_level = Some(player_load_data.level);
                c.player_archetype = Some(player_load_data.archetype);
                // Reused by every later CREATE_BASE_PLAYER in this session
                // (respawn reanchor, gate travel) so a GM stays SGWGmPlayer.
                c.player_class_id = Some(entry_info.class_id);
                c.player_alignment = Some(player_load_data.alignment);
                c.world_name = Some(entry_info.world_name.clone());
                c.player_xp = Some(player_load_data.exp as u64);
                c.player_training_points = Some(player_load_data.training_points as u32);
                // Cache the selected player's DB id so gate-travel/respawn
                // can use it directly instead of falling back to a "lowest
                // player_id for this account" lookup that would target the
                // wrong character on multi-character accounts.
                c.active_player_id = Some(player_id);
                // Rule 6: helpers that log only a `player_id` name it from here.
                cimmeria_entity::known_names::remember_player(
                    player_id,
                    &player_load_data.player_name,
                );
                // Not listed in the online name index yet: the client has not
                // created this player entity. `handle_on_client_ready` lists it.
                c.pending_world_entry = Some(entry_info);
                c.pending_player_load_data = Some(player_load_data);
                c.pending_client_ready = None;
            }
        }
        // Kick-off RESET_ENTITIES is reliable state-change. Registered after
        // the owner check above, so it never lands in a replacing session's
        // channel.
        super::register_direct_reliable(connected, addr, seq, &pkt, &next_seq, origin.as_ref());

        tracing::info!(%addr, "Entity teardown sent -- waiting for ENABLE_ENTITIES from client");

        // Discord world-channel: the character is now known (unlike the
        // auth-time login emit), so this carries name + world + spawn
        // position.
        cimmeria_discord::emit_player_world_entry(
            cimmeria_discord::Named::new(account_id, entry_account_name),
            cimmeria_discord::Named::new(player_id, Some(entry_character_name)),
            cimmeria_base_session::base::discord_world(&entry_world_name),
            entry_position,
        );
        Ok(true)
    }
    .await;

    // An entry that failed or was abandoned after the promotion gives the
    // arm back, so the account's next real login still gets it.
    #[cfg(feature = "lab-chaos")]
    if !matches!(entered, Ok(true)) {
        if let Some(promotion) = promotion {
            chaos::rearm(promotion);
        }
    }
    // Every way out short of `Ok(true)` comes before the store, so no
    // session holds the entity `query_world_entry` created, and no teardown
    // would ever remove it.
    if !matches!(entered, Ok(true)) {
        remove_unentered_entity(
            entity_id,
            entity_manager,
            cell_tx,
            account_id,
            account_name.as_deref(),
            player_id,
        )
        .await;
    }
    entered.map(|_| ())
}

/// Removes the player entity of a world entry that did not complete, from
/// the cell and from the `EntityManager`.
///
/// `DestroyEntity` (the gate-travel teardown), not the logout
/// `DisconnectEntity`: the character never entered, so there is nothing to
/// persist, and the logout path's `player_id`-keyed work (the last-position
/// write, the squad leave) would hit the same character's entry on the
/// session that replaced this one. The id goes back to the free list only
/// after the destroy is queued: the cell handles its channel in order, so a
/// `CreateEntity` that recycles the id lands after it (#999). A failed send
/// withholds the id instead, as the session teardown does.
async fn remove_unentered_entity(
    entity_id: u32,
    entity_manager: &Arc<Mutex<EntityManager>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    account_id: u32,
    account_name: Option<&str>,
    player_id: i32,
) {
    let player_name = cimmeria_entity::known_names::player_name(player_id);
    if let Some(tx) = cell_tx {
        if let Err(e) = tx.send(BaseToCellMsg::DestroyEntity { entity_id }).await {
            tracing::warn!(
                entity_id,
                entity_name = player_name,
                account_id,
                account_name,
                player_id,
                player_name,
                error = %e,
                "World entry: DestroyEntity send failed for an entry that did not complete; \
                 the entity id is withheld from reuse"
            );
            return;
        }
    }
    if let Ok(mut manager) = entity_manager.lock() {
        manager.destroy_entity(cimmeria_common::EntityId(entity_id as i32));
    }
    tracing::info!(
        entity_id,
        entity_name = player_name,
        account_id,
        account_name,
        player_id,
        player_name,
        "World entry: removed the entity of an entry that did not complete"
    );
}

/// Whether the session at `addr` is still the one whose owner token is
/// `owner` (its `cancelled` Arc, captured at the entry latch). A teardown
/// removes the entry and a relaunch takeover inserts a new state with a new
/// token, so an address match alone says nothing.
fn still_owner(
    clients: &HashMap<SocketAddr, ConnectedClientState>,
    addr: SocketAddr,
    owner: &Arc<AtomicBool>,
) -> bool {
    clients
        .get(&addr)
        .is_some_and(|c| Arc::ptr_eq(&c.cancelled, owner))
}

#[cfg(test)]
mod test_fakes;
#[cfg(test)]
mod tests;

/// CH-14: chaos armed for an account reaches its next world entry's flush.
#[cfg(all(test, feature = "lab-chaos"))]
mod chaos_tests;
