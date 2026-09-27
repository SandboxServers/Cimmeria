//! Owner-only crafting state pushes: the one place the base tells a
//! player's own client what it knows about crafting.
//!
//! Every crafting packet that changes state ends in one of these, so the
//! client's copy never drifts from the database:
//!
//! - [`push_discipline`]: `onUpdateDiscipline` (136), one discipline and
//!   its expertise.
//! - [`push_paradigm`]: `onUpdateRacialParadigmLevel` (138).
//! - [`push_known_crafts`]: `onUpdateKnownCrafts` (139), the full blueprint
//!   list.
//! - [`push_asp`]: `onEntityProperty(GENERICPROPERTY_AppliedSciencePoints,
//!   total)`. Always the **total** (audit C-57); the client shows the value
//!   as it arrives (`DisciplineTrainer.lua:49-54`).
//! - [`push_login_bundle`]: all of the above for a whole
//!   [`CraftingState`], in one reliable bundle. [`push_crafting_on_login`]
//!   loads the state and sends it after `onClientReady` (CR-03).
//!
//! `onUpdateCraftingOptions` (140) is not here: the station and tool gate
//! owns it (CR-05). Until then the client keeps its default, every tab
//! disabled.
//!
//! The legacy server sent only the ASP property and 139 at login
//! (`python/cell/SGWPlayer.py:510`, `:524`), so a relog lost the client's
//! disciplines, expertise and paradigm levels (audit C-05).

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_entity::crafting::CraftingState;
use cimmeria_mercury::channel_bundle::{ChannelBundle, IDBASE_SGW_PLAYER};
use cimmeria_mercury::transport::Transport;
use cimmeria_wire::cell::client_methods::player::{
    ON_UPDATE_DISCIPLINE, ON_UPDATE_KNOWN_CRAFTS, ON_UPDATE_RACIAL_PARADIGM_LEVEL,
};
use cimmeria_wire::cell::client_methods::spawnable_entity::ON_ENTITY_PROPERTY;
use cimmeria_wire::crafting::{
    applied_science_points_property_args, known_crafts_args, racial_paradigm_level_args,
    update_discipline_args,
};
use sqlx::PgPool;

use super::persistence::load_crafting_state;
use crate::base::helpers::{send_bundle_to_witness_reliable, send_to_witness_reliable};
use crate::base::ConnectedClientState;
use crate::mercury::build_player_entity_method_packet;

/// The client-side half of a crafting push: who to send to and how.
#[derive(Clone, Copy)]
pub struct CraftClient<'a> {
    pub transport: &'a Arc<dyn Transport>,
    pub connected: &'a Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    pub entity_to_addr: &'a Arc<Mutex<HashMap<u32, SocketAddr>>>,
}

/// The messages that carry a whole [`CraftingState`], as
/// `(client method index, argument bytes)` in send order:
///
/// 1. `onUpdateDiscipline` (136) per known discipline, in `discipline_ids`
///    order. A known discipline with no expertise row reads 0.
/// 2. `onUpdateRacialParadigmLevel` (138) per paradigm, by id.
/// 3. `onUpdateKnownCrafts` (139).
/// 4. The ASP property.
///
/// Disciplines and paradigms go first because the discipline trainer
/// redraws its tree on each of them (`DisciplineTrainer.lua:246-255`); the
/// ASP count it shows is independent.
pub fn crafting_state_messages(state: &CraftingState) -> Vec<(u16, Vec<u8>)> {
    let mut messages = Vec::with_capacity(state.discipline_ids.len() + 7);
    for &discipline_id in &state.discipline_ids {
        let expertise = state.get_expertise(discipline_id).unwrap_or(0);
        messages.push((
            ON_UPDATE_DISCIPLINE,
            update_discipline_args(discipline_id, expertise),
        ));
    }
    let mut paradigms: Vec<(i32, i8)> = state
        .racial_paradigm_levels
        .iter()
        .map(|(&id, &level)| (id, level))
        .collect();
    paradigms.sort_unstable_by_key(|&(id, _)| id);
    for (paradigm_id, level) in paradigms {
        messages.push((
            ON_UPDATE_RACIAL_PARADIGM_LEVEL,
            racial_paradigm_level_args(paradigm_id, level),
        ));
    }
    messages.push((
        ON_UPDATE_KNOWN_CRAFTS,
        known_crafts_args(&state.blueprint_ids),
    ));
    messages.push((
        ON_ENTITY_PROPERTY,
        applied_science_points_property_args(state.applied_science_points),
    ));
    messages
}

/// [`crafting_state_messages`] as one reliable bundle on `entity_id`. Every
/// message targets the player's own, long-created entity, so they may share
/// a bundle (`docs/architecture/mercury-bundle.md`).
pub fn build_crafting_state_bundle(entity_id: u32, state: &CraftingState) -> ChannelBundle {
    let mut bundle = ChannelBundle::new(true);
    for (method_index, args) in crafting_state_messages(state) {
        bundle.append_entity_method(method_index, IDBASE_SGW_PLAYER, entity_id, &args);
    }
    bundle
}

/// Send the whole crafting state to the player's own client.
pub async fn push_login_bundle(entity_id: u32, state: &CraftingState, client: CraftClient<'_>) {
    let bundle = build_crafting_state_bundle(entity_id, state);
    send_bundle_to_witness_reliable(
        client.transport,
        client.connected,
        client.entity_to_addr,
        entity_id,
        bundle,
    )
    .await;
}

/// Load the player's crafting state and push it (CR-03). Called once per
/// world entry, after the `onClientReady` burst, so it lands on a live
/// entity whose UI has loaded.
///
/// Loading applies the D-CR03 starting paradigm levels to a character that
/// has none stored, so the tree draws the root disciplines as learnable. A
/// failed load sends nothing: the ASP count and blueprint list from the
/// `mapLoaded` bundle stay, and the WARN names the player.
pub async fn push_crafting_on_login(
    entity_id: u32,
    player_id: i32,
    db_pool: &Option<Arc<PgPool>>,
    client: CraftClient<'_>,
) {
    let Some(pool) = db_pool else {
        return;
    };
    match load_crafting_state(pool, player_id).await {
        Ok(state) => {
            tracing::debug!(
                target: "crafting",
                event = "login_sync",
                entity_id,
                player_id,
                disciplines = state.discipline_ids.len(),
                blueprints = state.blueprint_ids.len(),
                asp = state.applied_science_points,
                "crafting state pushed at login"
            );
            push_login_bundle(entity_id, &state, client).await;
        }
        Err(e) => {
            tracing::warn!(
                target: "crafting",
                event = "persist_failed",
                op = "login_load",
                entity_id,
                player_id,
                error = %e,
                "crafting login sync: load failed -- the client keeps no \
                 disciplines or paradigm levels until the next world entry"
            );
        }
    }
}

/// Send one crafting client method to the player's own client.
async fn push_one(entity_id: u32, method_index: u16, args: &[u8], client: CraftClient<'_>) {
    send_to_witness_reliable(
        client.transport,
        client.connected,
        client.entity_to_addr,
        entity_id,
        |key, version, seq, acks| {
            build_player_entity_method_packet(
                key,
                seq,
                acks,
                entity_id,
                method_index,
                args,
                version,
            )
        },
    )
    .await;
}

/// `onUpdateDiscipline(discipline_id, expertise)` (136).
pub async fn push_discipline(
    entity_id: u32,
    discipline_id: i32,
    expertise: i32,
    client: CraftClient<'_>,
) {
    let args = update_discipline_args(discipline_id, expertise);
    push_one(entity_id, ON_UPDATE_DISCIPLINE, &args, client).await;
}

/// `onUpdateRacialParadigmLevel(paradigm_id, level)` (138).
pub async fn push_paradigm(entity_id: u32, paradigm_id: i32, level: i8, client: CraftClient<'_>) {
    let args = racial_paradigm_level_args(paradigm_id, level);
    push_one(entity_id, ON_UPDATE_RACIAL_PARADIGM_LEVEL, &args, client).await;
}

/// `onUpdateKnownCrafts(blueprint_ids)` (139): the full list.
pub async fn push_known_crafts(entity_id: u32, blueprint_ids: &[i32], client: CraftClient<'_>) {
    let args = known_crafts_args(blueprint_ids);
    push_one(entity_id, ON_UPDATE_KNOWN_CRAFTS, &args, client).await;
}

/// The ASP property, carrying the player's unspent **total**.
pub async fn push_asp(entity_id: u32, total: i32, client: CraftClient<'_>) {
    let args = applied_science_points_property_args(total);
    push_one(entity_id, ON_ENTITY_PROPERTY, &args, client).await;
}

#[cfg(test)]
mod tests;
