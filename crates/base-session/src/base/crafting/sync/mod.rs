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
//!   total)`. Always the **total**, never the change; the client shows the value
//!   as it arrives (`DisciplineTrainer.lua:49-54`).
//! - [`push_respec_prompt`]: `onCraftingRespecPrompt` (112), the respec
//!   confirmation dialog.
//! - [`push_discipline_respec`]: `onDisciplineRespec` (137), which zeroes the
//!   expertise of every discipline the client knows.
//! - [`push_login_bundle`]: all of the above for a whole
//!   [`CraftingState`], then `onUpdateCraftingOptions` (140), in one
//!   reliable bundle. [`push_crafting_on_login`] loads the state and the
//!   options and sends them after `onClientReady`: the one crafting push of
//!   a world entry.
//!
//! After login, 140 changes are sent by `super::options`, which owns the
//! stations, tools and "craft anywhere" behind it.
//!
//! The legacy server sent only the ASP property and 139 at login
//! (`python/cell/SGWPlayer.py:510`, `:524`), so a relog lost the client's
//! disciplines, expertise and paradigm levels.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_entity::crafting::CraftingState;
use cimmeria_mercury::channel_bundle::{ChannelBundle, IDBASE_SGW_PLAYER};
use cimmeria_mercury::transport::Transport;
use cimmeria_wire::cell::client_methods::player::{
    ON_CRAFTING_RESPEC_PROMPT, ON_DISCIPLINE_RESPEC, ON_UPDATE_CRAFTING_OPTIONS,
    ON_UPDATE_DISCIPLINE, ON_UPDATE_KNOWN_CRAFTS, ON_UPDATE_RACIAL_PARADIGM_LEVEL,
};
use cimmeria_wire::cell::client_methods::spawnable_entity::ON_ENTITY_PROPERTY;
use cimmeria_wire::crafting::{
    applied_science_points_property_args, crafting_options_args, crafting_respec_prompt_args,
    discipline_respec_args, known_crafts_args, racial_paradigm_level_args, update_discipline_args,
    CraftingOptions,
};
use sqlx::PgPool;

use super::options::{login_options, record_sent};
use super::persistence::load_crafting_state_reporting;
use super::telemetry::{account_id_of, bundle_send_failure, sql_error_class, witness_send_failure};
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

/// The messages of the login crafting bundle: [`crafting_state_messages`]
/// for `state`, then `onUpdateCraftingOptions` (140) for `options`. Either
/// half may be absent (the state failed to load, the entity has no
/// session); the options go last so the crafting window's machine and tool
/// arrive after the disciplines and blueprints it lists.
pub fn login_messages(
    state: Option<&CraftingState>,
    options: Option<&CraftingOptions>,
) -> Vec<(u16, Vec<u8>)> {
    let mut messages = state.map(crafting_state_messages).unwrap_or_default();
    if let Some(options) = options {
        messages.push((ON_UPDATE_CRAFTING_OPTIONS, crafting_options_args(options)));
    }
    messages
}

/// [`login_messages`] as one reliable bundle on `entity_id`.
pub fn build_login_bundle(
    entity_id: u32,
    state: Option<&CraftingState>,
    options: Option<&CraftingOptions>,
) -> ChannelBundle {
    let mut bundle = ChannelBundle::new(true);
    for (method_index, args) in login_messages(state, options) {
        bundle.append_entity_method(method_index, IDBASE_SGW_PLAYER, entity_id, &args);
    }
    bundle
}

/// Send the login crafting bundle to the player's own client. `Err`
/// carries why it did not go out (`entity_to_addr_miss`,
/// `client_disconnected`, `send_error`, `empty_bundle`); the caller logs
/// it.
pub async fn push_login_bundle(
    entity_id: u32,
    state: Option<&CraftingState>,
    options: Option<&CraftingOptions>,
    client: CraftClient<'_>,
) -> Result<(), &'static str> {
    let bundle = build_login_bundle(entity_id, state, options);
    let outcome = send_bundle_to_witness_reliable(
        client.transport,
        client.connected,
        client.entity_to_addr,
        entity_id,
        bundle,
    )
    .await;
    match bundle_send_failure(&outcome) {
        None => Ok(()),
        Some(reason) => Err(reason),
    }
}

/// Load the player's crafting state and crafting options and push them in
/// one bundle. Called once per world entry, after the `onClientReady`
/// burst, so it lands on a live entity whose UI has loaded.
///
/// Loading fills in the starting level of every paradigm with none stored,
/// so the bundle carries all five 138s (none left at its pre-relog value)
/// and the tree draws the root disciplines as learnable. A sent
/// bundle is a `login_sync` event recording what was sent. A failed load
/// is a `login_sync_failed` WARN with `reason = load` and the
/// `error_class`; the bundle then carries only the options (the ASP count
/// and blueprint list from the `mapLoaded` bundle stay). Without a
/// database there is no state to load and only the options go. A bundle
/// that could not be sent is `login_sync_failed` with `reason = send`.
pub async fn push_crafting_on_login(
    entity_id: u32,
    player_id: i32,
    db_pool: &Option<Arc<PgPool>>,
    client: CraftClient<'_>,
) {
    let account_id = account_id_of(entity_id, client.connected, client.entity_to_addr);
    let options = login_options(
        entity_id,
        player_id,
        db_pool,
        client.connected,
        client.entity_to_addr,
    )
    .await;
    let loaded = match db_pool {
        None => None,
        Some(pool) => match load_crafting_state_reporting(pool, player_id).await {
            Ok(loaded) => Some(loaded),
            Err(e) => {
                tracing::warn!(
                    target: "crafting",
                    event = "login_sync_failed",
                    reason = "load",
                    error_class = sql_error_class(&e),
                    error = %e,
                    account_id,
                    player_id,
                    entity_id,
                    "crafting login sync: load failed -- the client keeps no \
                     disciplines or paradigm levels until the next world entry"
                );
                None
            }
        },
    };
    if loaded.is_none() && options.is_none() {
        return;
    }
    let state = loaded.as_ref().map(|(state, _)| state);
    match push_login_bundle(entity_id, state, options.as_ref(), client).await {
        Ok(()) => {
            tracing::info!(
                target: "crafting",
                event = "login_sync",
                account_id,
                player_id,
                entity_id,
                disciplines = state.map(|s| s.discipline_ids.len()),
                paradigms = state.map(|s| s.racial_paradigm_levels.len()),
                blueprints = state.map(|s| s.blueprint_ids.len()),
                asp = state.map(|s| s.applied_science_points),
                defaults_applied = loaded.as_ref().map(|&(_, applied)| applied),
                crafting_options = options.is_some(),
                "crafting state pushed at login"
            );
            if let Some(options) = options {
                record_sent(entity_id, options, client.connected, client.entity_to_addr);
            }
        }
        Err(error_class) => tracing::warn!(
            target: "crafting",
            event = "login_sync_failed",
            reason = "send",
            error_class,
            account_id,
            player_id,
            entity_id,
            "crafting login sync: bundle not sent -- the client keeps no \
             disciplines or paradigm levels until the next world entry"
        ),
    }
}

fn warn_push_failed(
    entity_id: u32,
    player_id: i32,
    what: &'static str,
    reason: &'static str,
    client: CraftClient<'_>,
) {
    tracing::warn!(
        target: "crafting",
        event = "push_failed",
        what,
        reason,
        account_id = account_id_of(entity_id, client.connected, client.entity_to_addr),
        player_id,
        entity_id,
        "crafting state push not sent -- the client shows stale crafting state"
    );
}

/// Send one crafting client method to the player's own client; a send that
/// does not go out is a WARN.
async fn push_one(
    entity_id: u32,
    player_id: i32,
    what: &'static str,
    method_index: u16,
    args: &[u8],
    client: CraftClient<'_>,
) {
    let outcome = send_to_witness_reliable(
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
    if let Some(reason) = witness_send_failure(&outcome) {
        warn_push_failed(entity_id, player_id, what, reason, client);
    }
}

/// `onUpdateDiscipline(discipline_id, expertise)` (136).
pub async fn push_discipline(
    entity_id: u32,
    player_id: i32,
    discipline_id: i32,
    expertise: i32,
    client: CraftClient<'_>,
) {
    let args = update_discipline_args(discipline_id, expertise);
    push_one(
        entity_id,
        player_id,
        "discipline",
        ON_UPDATE_DISCIPLINE,
        &args,
        client,
    )
    .await;
}

/// `onUpdateRacialParadigmLevel(paradigm_id, level)` (138).
pub async fn push_paradigm(
    entity_id: u32,
    player_id: i32,
    paradigm_id: i32,
    level: i8,
    client: CraftClient<'_>,
) {
    let args = racial_paradigm_level_args(paradigm_id, level);
    push_one(
        entity_id,
        player_id,
        "paradigm",
        ON_UPDATE_RACIAL_PARADIGM_LEVEL,
        &args,
        client,
    )
    .await;
}

/// `onUpdateKnownCrafts(blueprint_ids)` (139): the full list.
pub async fn push_known_crafts(
    entity_id: u32,
    player_id: i32,
    blueprint_ids: &[i32],
    client: CraftClient<'_>,
) {
    let args = known_crafts_args(blueprint_ids);
    push_one(
        entity_id,
        player_id,
        "known_crafts",
        ON_UPDATE_KNOWN_CRAFTS,
        &args,
        client,
    )
    .await;
}

/// The ASP property, carrying the player's unspent **total**.
pub async fn push_asp(entity_id: u32, player_id: i32, total: i32, client: CraftClient<'_>) {
    let args = applied_science_points_property_args(total);
    push_one(
        entity_id,
        player_id,
        "asp",
        ON_ENTITY_PROPERTY,
        &args,
        client,
    )
    .await;
}

/// `onCraftingRespecPrompt(cost)` (112): the client stores the cost and
/// shows the "unlearn all your crafting knowledge" dialog, whose Yes sends
/// `respecCrafting` (100).
pub async fn push_respec_prompt(
    entity_id: u32,
    player_id: i32,
    cost: i32,
    client: CraftClient<'_>,
) {
    let args = crafting_respec_prompt_args(cost);
    push_one(
        entity_id,
        player_id,
        "respec_prompt",
        ON_CRAFTING_RESPEC_PROMPT,
        &args,
        client,
    )
    .await;
}

/// `onDisciplineRespec()` (137): the client zeroes the expertise of every
/// discipline it knows. Blueprints and paradigm levels are untouched.
pub async fn push_discipline_respec(entity_id: u32, player_id: i32, client: CraftClient<'_>) {
    push_one(
        entity_id,
        player_id,
        "discipline_respec",
        ON_DISCIPLINE_RESPEC,
        &discipline_respec_args(),
        client,
    )
    .await;
}

#[cfg(test)]
mod tests;
