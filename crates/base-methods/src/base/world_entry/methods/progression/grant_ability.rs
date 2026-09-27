//! Base side of the GM `.giveability` console command (pets campaign PT-07).
//!
//! The cell has already GM-gated the caller (the `.`-console channel gate),
//! checked that the ability exists and resolved the subject's `player_id`.
//! The base appends the ability in one guarded `UPDATE`
//! ([`persist_ability_grant`]), answers the cell with
//! `BaseToCellMsg::GmAbilityGranted`, and sends the GM the outcome.
//!
//! A GM grant is not a trainer purchase: it touches `abilities` only, never
//! `trained_abilities`, `training_points` or `tree_points_spent`. The respec
//! `UPDATE` (`respec.rs`) removes only `trained_abilities` from `abilities`,
//! so a granted ability survives a respec and refunds nothing.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;

use super::super::super::super::gm_feedback::send_gm_feedback_to_client;
use super::super::super::super::ConnectedClientState;
use crate::cell::messages::BaseToCellMsg;

/// One `CellToBaseMsg::GmGrantAbility`, as the base handles it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AbilityGrant {
    /// The subject's entity id.
    pub entity_id: u32,
    /// The subject's character, resolved by the cell.
    pub player_id: i32,
    pub ability_id: i32,
    /// The GM who typed the command, and the character that entity played
    /// when the cell sent it.
    pub gm_entity_id: u32,
    pub gm_player_id: i32,
}

/// Append `ability_id` to `player_id`'s known abilities.
///
/// `Ok(true)` when the row changed. `Ok(false)` means the guard held it back
/// and nothing changed: no such player, or the ability is already known. The
/// `NOT (abilities @> ...)` guard is what makes a replayed or double-typed
/// grant a no-op under row locking; the cell's "already known" check is only
/// for a friendlier message.
pub(super) async fn persist_ability_grant(
    pool: &PgPool,
    player_id: i32,
    ability_id: i32,
) -> sqlx::Result<bool> {
    let r = sqlx::query(
        "UPDATE sgw_player \
            SET abilities = abilities || $1::integer \
          WHERE player_id = $2 \
            AND NOT (abilities @> ARRAY[$1::integer])",
    )
    .bind(ability_id)
    .bind(player_id)
    .execute(pool)
    .await?;
    Ok(r.rows_affected() == 1)
}

/// The character `entity_id`'s session plays now, if any.
fn active_player_of(
    entity_id: u32,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) -> Option<i32> {
    let addr = entity_to_addr.lock().unwrap().get(&entity_id).copied()?;
    match connected.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    }
    .get(&addr)
    .and_then(|s| s.active_player_id)
}

/// Persist a GM ability grant, tell the cell, and report to the GM.
#[tracing::instrument(
    name = "progression.gm_grant_ability",
    level = "info",
    skip_all,
    fields(
        entity_id = grant.entity_id,
        player_id = grant.player_id,
        ability_id = grant.ability_id,
        gm_entity_id = grant.gm_entity_id,
    )
)]
pub async fn handle_gm_grant_ability(
    grant: AbilityGrant,
    db_pool: &Option<Arc<PgPool>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    cell_tx: &Option<tokio::sync::mpsc::Sender<BaseToCellMsg>>,
) {
    let AbilityGrant {
        entity_id,
        player_id,
        ability_id,
        gm_entity_id,
        gm_player_id,
    } = grant;
    // The GM's entity id is recycled on relog, like the subject's: answer only
    // while that entity still plays the GM's character, or a stranger reads
    // "granted ability X".
    let tell_gm = |text: String| async move {
        if active_player_of(gm_entity_id, connected, entity_to_addr) != Some(gm_player_id) {
            tracing::debug!(
                gm_entity_id,
                gm_player_id,
                "GmGrantAbility: GM session gone or reused; feedback dropped"
            );
            return;
        }
        send_gm_feedback_to_client(gm_entity_id, &text, transport, connected, entity_to_addr).await;
    };

    let Some(pool) = db_pool else {
        tracing::warn!(
            entity_id,
            player_id,
            ability_id,
            "GmGrantAbility: no DB pool, dropping grant"
        );
        tell_gm(format!(
            ".giveability: refused, no database; ability {ability_id} not granted"
        ))
        .await;
        return;
    };

    // The subject's session must still play the character the cell resolved.
    // A reused entity id would otherwise grant one character and mirror the
    // ability onto another's cell entity.
    let active = active_player_of(entity_id, connected, entity_to_addr);
    if active != Some(player_id) {
        tracing::warn!(
            entity_id,
            player_id,
            ability_id,
            active_player_id = ?active,
            "GmGrantAbility: session is not playing the resolved character — rejecting"
        );
        tell_gm(format!(
            ".giveability: refused, entity {entity_id} no longer plays that character"
        ))
        .await;
        return;
    }

    match persist_ability_grant(pool, player_id, ability_id).await {
        Ok(true) => {}
        Ok(false) => {
            tracing::info!(
                entity_id,
                player_id,
                ability_id,
                "GmGrantAbility: UPDATE matched 0 rows (already known, or no such player)"
            );
            tell_gm(format!(
                ".giveability: entity {entity_id} already knows ability {ability_id}; nothing changed"
            ))
            .await;
            return;
        }
        Err(e) => {
            tracing::error!(
                entity_id,
                player_id,
                ability_id,
                "GmGrantAbility: UPDATE failed: {e}"
            );
            tell_gm(format!(
                ".giveability: database error; ability {ability_id} not granted"
            ))
            .await;
            return;
        }
    }

    tracing::info!(
        entity_id,
        player_id,
        ability_id,
        gm_entity_id,
        gm_player_id,
        "GmGrantAbility: persisted"
    );

    match cell_tx {
        Some(tx) => {
            if let Err(e) = tx
                .send(BaseToCellMsg::GmAbilityGranted {
                    entity_id,
                    player_id,
                    ability_id,
                })
                .await
            {
                tracing::error!(
                    entity_id,
                    ability_id,
                    error = %e,
                    "GmGrantAbility: base→cell send failed; the ability shows after relog"
                );
            }
        }
        None => tracing::warn!(
            entity_id,
            ability_id,
            "GmGrantAbility: no cell channel; the ability shows after relog"
        ),
    }

    tell_gm(format!(
        ".giveability: granted ability {ability_id} to entity {entity_id} \
         (saved to the character; it survives relog and respec)"
    ))
    .await;
}
