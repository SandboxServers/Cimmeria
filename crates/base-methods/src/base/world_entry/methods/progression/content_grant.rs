//! Base side of the `grant_ability` content action (Class Start v6, CS-01a).
//!
//! The cell sends `CellToBaseMsg::ContentGrantAbilities` for a player whose
//! chain fired. The base checks the session still plays that character,
//! writes `abilities`, `trained_abilities` (a conversion) and
//! `sgw_player_ability_grants` in one transaction ([`persist_content_grant`])
//! and answers with `BaseToCellMsg::ContentAbilitiesGranted`. The cell
//! mirrors it and tells the player. Nothing here is GM-gated: tutorials,
//! racial cores, class signatures and mission rewards all come this way;
//! the grant's own `archetypes` list is checked against the character's
//! real archetype.
//!
//! Telemetry (target `abilities`, `event = "content_grant_ability"`): one
//! row per requested id with `decision_outcome` `granted` or
//! `converted_from_trained` (INFO), `already_known` or `starter_kept`
//! (DEBUG), and one `refused` row per request that wrote nothing (WARN,
//! ERROR for a database error).

use cimmeria_entity::cell_entity::AbilityGrantKind;
use cimmeria_entity::known_names;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use sqlx::PgPool;

use super::super::super::super::ConnectedClientState;
use super::content_grant_write::{persist_content_grant, ContentGrantRefusal, ContentGrantWrite};
use super::grant_ability::active_player_of;
use crate::cell::messages::{BaseToCellMsg, ContentAbilitiesGranted, ContentGrantAbilities};

const EVENT: &str = "content_grant_ability";

/// `source_id`'s name: the mission's, when the grant is a mission reward
/// (Rule 6). Other kinds leave it out.
fn source_name(msg: &ContentGrantAbilities) -> Option<String> {
    (msg.source_kind == AbilityGrantKind::Mission)
        .then_some(msg.source_id)
        .flatten()
        .and_then(|id| cimmeria_names::book().mission(id).map(str::to_string))
}

/// One `refused` row for a request that wrote nothing.
fn log_refused(
    level: tracing::Level,
    reason: &'static str,
    msg: &ContentGrantAbilities,
    archetype: Option<i32>,
    error: Option<String>,
) {
    let player_label = known_names::player_name(msg.player_id);
    let source_name = source_name(msg);
    let book = cimmeria_names::book();
    let ability_names = msg
        .ability_ids
        .iter()
        .map(|&id| match book.ability(id) {
            Some(name) => format!("{id}:{name}"),
            None => id.to_string(),
        })
        .collect::<Vec<_>>()
        .join(", ");
    macro_rules! row {
        ($lvl:ident) => {
            tracing::$lvl!(
                target: "abilities",
                event = EVENT,
                decision_outcome = "refused",
                reason,
                persisted = false,
                entity_id = msg.entity_id,
                entity_name = player_label,
                account_id = msg.account_id,
                account_name = known_names::account_name(msg.account_id),
                player_id = msg.player_id,
                player_name = player_label,
                chain_id = msg.chain_id,
                chain_name = book.chain(msg.chain_id),
                source_kind = msg.source_kind.as_str(),
                source_id = msg.source_id,
                source_name = source_name.as_deref(),
                archetype,
                archetype_name = archetype.and_then(cimmeria_names::archetype_name),
                allowed_archetypes = ?msg.archetypes,
                ability_ids = ?msg.ability_ids,
                ability_names = %ability_names,
                error = error.as_deref(),
                "content ability grant refused; nothing written"
            )
        };
    }
    if level == tracing::Level::ERROR {
        row!(error);
    } else {
        row!(warn);
    }
}

/// One row per requested id, after the commit.
fn log_outcomes(msg: &ContentGrantAbilities, write: &ContentGrantWrite) {
    let player_label = known_names::player_name(msg.player_id);
    let source_name = source_name(msg);
    let book = cimmeria_names::book();
    for &ability_id in &msg.ability_ids {
        let provenance_written = write.rows_written.contains(&ability_id);
        let (outcome, why, info) = if write.converted.contains(&ability_id) {
            (
                "converted_from_trained",
                "bought from a trainer; converted to a grant and its cost refunded",
                true,
            )
        } else if write.learned.contains(&ability_id) {
            ("granted", "learned", true)
        } else if write.starters.contains(&ability_id) {
            (
                "starter_kept",
                "a character-creation starter; no provenance, no branch credit",
                false,
            )
        } else {
            (
                "already_known",
                "already known; provenance recorded if it had none",
                false,
            )
        };
        macro_rules! row {
            ($lvl:ident) => {
                tracing::$lvl!(
                    target: "abilities",
                    event = EVENT,
                    decision_outcome = outcome,
                    persisted = provenance_written || write.learned.contains(&ability_id),
                    entity_id = msg.entity_id,
                    entity_name = player_label,
                    account_id = msg.account_id,
                    account_name = known_names::account_name(msg.account_id),
                    player_id = msg.player_id,
                    player_name = player_label,
                    chain_id = msg.chain_id,
                    chain_name = book.chain(msg.chain_id),
                    source_kind = msg.source_kind.as_str(),
                    source_id = msg.source_id,
                    source_name = source_name.as_deref(),
                    ability_id,
                    ability_name = book.ability(ability_id),
                    provenance_written,
                    refunded = write.refunded,
                    training_points = write.training_points,
                    tree_points_spent = write.tree_points_spent,
                    "content ability grant: {why}"
                )
            };
        }
        if info {
            row!(info);
        } else {
            row!(debug);
        }
    }
}

/// Persist a content ability grant and tell the cell.
#[tracing::instrument(
    name = "progression.content_grant_ability",
    level = "info",
    skip_all,
    fields(
        entity_id = msg.entity_id,
        account_id = msg.account_id,
        player_id = msg.player_id,
        chain_id = msg.chain_id,
        source_kind = msg.source_kind.as_str(),
    )
)]
pub async fn handle_content_grant_abilities(
    msg: ContentGrantAbilities,
    db_pool: &Option<Arc<PgPool>>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    cell_tx: &Option<tokio::sync::mpsc::Sender<BaseToCellMsg>>,
) {
    let Some(pool) = db_pool else {
        log_refused(tracing::Level::WARN, "no_database", &msg, None, None);
        return;
    };
    // A recycled entity id would grant one character and mirror the
    // abilities onto another's cell entity.
    if active_player_of(msg.entity_id, connected, entity_to_addr) != Some(msg.player_id) {
        log_refused(tracing::Level::WARN, "session_mismatch", &msg, None, None);
        return;
    }
    let write = match persist_content_grant(
        pool,
        msg.player_id,
        &msg.ability_ids,
        msg.source_kind,
        msg.source_id,
        &msg.archetypes,
    )
    .await
    {
        Ok(Ok(write)) => write,
        Ok(Err(ContentGrantRefusal::PlayerRowMissing)) => {
            log_refused(tracing::Level::WARN, "player_row_missing", &msg, None, None);
            return;
        }
        Ok(Err(ContentGrantRefusal::GmKind)) => {
            log_refused(
                tracing::Level::WARN,
                "gm_kind_from_content",
                &msg,
                None,
                None,
            );
            return;
        }
        Ok(Err(ContentGrantRefusal::ArchetypeMismatch { archetype })) => {
            // The cell checks first; reaching here means its archetype and
            // the row's disagree.
            log_refused(
                tracing::Level::WARN,
                "archetype_mismatch",
                &msg,
                Some(archetype),
                None,
            );
            return;
        }
        Err(e) => {
            log_refused(
                tracing::Level::ERROR,
                "db_error",
                &msg,
                None,
                Some(e.to_string()),
            );
            return;
        }
    };
    log_outcomes(&msg, &write);

    let reply = BaseToCellMsg::ContentAbilitiesGranted(ContentAbilitiesGranted {
        entity_id: msg.entity_id,
        player_id: msg.player_id,
        chain_id: msg.chain_id,
        source_kind: msg.source_kind,
        learned: write.learned.clone(),
        credited: write.credited.clone(),
        converted: write.converted.clone(),
        training_points: write.training_points,
        tree_points_spent: write.tree_points_spent,
    });
    let sent = match cell_tx {
        Some(tx) => tx.send(reply).await.is_ok(),
        None => false,
    };
    if !sent {
        let player_label = known_names::player_name(msg.player_id);
        tracing::error!(
            target: "abilities",
            event = EVENT,
            decision_outcome = "mirror_send_failed",
            reason = "no_cell_channel",
            entity_id = msg.entity_id,
            entity_name = player_label,
            account_id = msg.account_id,
            account_name = known_names::account_name(msg.account_id),
            player_id = msg.player_id,
            player_name = player_label,
            chain_id = msg.chain_id,
            chain_name = cimmeria_names::book().chain(msg.chain_id),
            learned = ?write.learned,
            "content ability grant saved, but the cell was not told; it shows after relog"
        );
    }
}
