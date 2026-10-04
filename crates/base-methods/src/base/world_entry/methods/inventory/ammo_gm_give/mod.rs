//! Base side of the GM `.giveammo` console command (ammo campaign AM-06,
//! issue #1026).
//!
//! The cell has already GM-gated the caller, resolved the ammo type to a
//! special `EAmmoType` with a reserve item, capped the round count and
//! resolved the recipient's character. The base grants through
//! [`ammo_reserve::return_rounds`](super::ammo_reserve::return_rounds), not
//! the generic `GrantItem`: that tops up existing stacks to the item's
//! `max_stack_size` before opening new ones in bag 1, then bag 15, and
//! reports what did not fit. (`GrantItem` writes its whole count into one
//! row, so a 1000-round grant would make one over-cap stack.)
//!
//! Every outcome answers the GM on the feedback channel, including a partial
//! grant ("N did not fit: bags full"), and logs `gm_give_ammo` on target
//! `ammo`: INFO when rounds were granted, WARN on a refusal, with the
//! catalog's correlators and before/after totals.

use cimmeria_entity::known_names;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_entity::ammo_telemetry::{events, reasons};
use cimmeria_entity::ammo_type;
use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;

use super::ammo_reserve::{self, AmmoReturn};
use super::core::send_full_inventory_update;
use crate::base::gm_feedback::send_gm_feedback_to_client;
use crate::base::ConnectedClientState;
use crate::cell::messages::GmGiveAmmo;

#[cfg(test)]
mod live_db_tests;

/// Grant `rounds` of `ammo_type` to `player_id`'s carried bags in one
/// transaction. What did not fit is `remainder`; nothing over-cap is written.
pub(crate) async fn grant_rounds(
    pool: &PgPool,
    player_id: i32,
    ammo_type: i32,
    rounds: i32,
) -> sqlx::Result<AmmoReturn> {
    let mut tx = pool.begin().await?;
    let out = ammo_reserve::return_rounds(&mut tx, player_id, ammo_type, rounds).await?;
    tx.commit().await?;
    Ok(out)
}

/// The GM's result line for a committed grant. Pure, so the wording of every
/// outcome (full, partial, nothing fit, no reserve item) is unit-tested.
pub(crate) fn result_line(msg: &GmGiveAmmo, out: &AmmoReturn) -> String {
    let name = ammo_type::label(msg.ammo_type).unwrap_or("?");
    let who = if msg.entity_id == msg.gm_entity_id {
        "you".to_string()
    } else {
        format!("entity {}", msg.entity_id)
    };
    if out.item_id.is_none() {
        return format!(".giveammo: refused, {name} has no reserve item; nothing granted");
    }
    if out.returned == 0 {
        return format!(
            ".giveammo: refused, bags full; none of the {} rounds of {name} fit, nothing granted",
            msg.rounds
        );
    }
    let mut line = format!(
        ".giveammo: gave {who} {} rounds of {name} (item {}); {} now carried",
        out.returned, msg.item_id, out.stack_after
    );
    if out.remainder > 0 {
        line.push_str(&format!("; {} did not fit: bags full", out.remainder));
    }
    line
}

/// The character `entity_id`'s session plays now, if any.
fn active_player_of(
    entity_id: u32,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) -> Option<i32> {
    let addr = match entity_to_addr.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    }
    .get(&entity_id)
    .copied()?;
    match connected.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    }
    .get(&addr)
    .and_then(|s| s.active_player_id)
}

/// Handle `CellToBaseMsg::GmGiveAmmo`.
#[tracing::instrument(
    name = "ammo.gm_give",
    level = "info",
    skip_all,
    fields(
        entity_id = msg.gm_entity_id,
        account_id = msg.gm_account_id,
        player_id = msg.gm_player_id,
        subject_player_id = msg.player_id,
        ammo_type = msg.ammo_type,
        item_id = msg.item_id,
    )
)]
pub async fn handle_gm_give_ammo(
    msg: GmGiveAmmo,
    db_pool: &Option<Arc<PgPool>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let GmGiveAmmo {
        entity_id,
        player_id,
        gm_entity_id,
        gm_player_id,
        gm_account_id,
        ammo_type,
        item_id,
        rounds,
    } = msg;
    // Entity ids are recycled on relog: answer only while the GM's entity
    // still plays the GM's character, or a stranger reads the result line.
    let tell_gm = |text: String| async move {
        if active_player_of(gm_entity_id, connected, entity_to_addr) == Some(gm_player_id) {
            send_gm_feedback_to_client(gm_entity_id, &text, transport, connected, entity_to_addr)
                .await;
        }
    };
    let refuse = |reason: &'static str, text: String| async move {
        tracing::warn!(
            target: "ammo",
            event = events::GM_GIVE_AMMO,
            decision_outcome = "refused",
            reason,
            entity_id = gm_entity_id,
            entity_name = known_names::player_name(gm_player_id),
            account_id = gm_account_id,
            account_name = known_names::account_name(gm_account_id),
            player_id = gm_player_id,
            player_name = known_names::player_name(gm_player_id),
            subject_entity_id = entity_id,
            subject_entity_name = known_names::player_name(player_id),
            subject_player_id = player_id,
            subject_player_name = known_names::player_name(player_id),
            item_id,
            item_name = cimmeria_names::book().item(item_id),
            ammo_type,
            quantity = rounds,
            "gm_give_ammo refused; nothing granted"
        );
        tell_gm(text).await;
    };

    let Some(pool) = db_pool else {
        refuse(
            reasons::DB_ERROR,
            ".giveammo: refused, no database; nothing granted".to_string(),
        )
        .await;
        return;
    };
    // The recipient's session must still play the character the cell
    // resolved; a reused entity id would otherwise resync the wrong client.
    if active_player_of(entity_id, connected, entity_to_addr) != Some(player_id) {
        refuse(
            reasons::SESSION_MISMATCH,
            format!(".giveammo: refused, entity {entity_id} no longer plays that character"),
        )
        .await;
        return;
    }

    let out = match grant_rounds(pool, player_id, ammo_type, rounds).await {
        Ok(out) => out,
        Err(e) => {
            tracing::error!(
                target: "ammo",
                event = events::GM_GIVE_AMMO,
                decision_outcome = "refused",
                reason = reasons::DB_ERROR,
                entity_id = gm_entity_id,
                entity_name = known_names::player_name(gm_player_id),
                account_id = gm_account_id,
                account_name = known_names::account_name(gm_account_id),
                player_id = gm_player_id,
                player_name = known_names::player_name(gm_player_id),
                subject_player_id = player_id,
                subject_player_name = known_names::player_name(player_id),
                item_id,
                item_name = cimmeria_names::book().item(item_id),
                ammo_type,
                quantity = rounds,
                error = %e,
                "gm_give_ammo: the grant rolled back; nothing granted"
            );
            tell_gm(".giveammo: refused, the inventory write failed; nothing granted".into()).await;
            return;
        }
    };

    let line = result_line(&msg, &out);
    if out.returned == 0 {
        let reason = if out.item_id.is_none() {
            reasons::NO_RESERVE_ITEM
        } else {
            reasons::BAGS_FULL
        };
        refuse(reason, line).await;
        return;
    }
    tracing::info!(
        target: "ammo",
        event = events::GM_GIVE_AMMO,
        decision_outcome = "granted",
        entity_id = gm_entity_id,
        entity_name = known_names::player_name(gm_player_id),
        account_id = gm_account_id,
        account_name = known_names::account_name(gm_account_id),
        player_id = gm_player_id,
        player_name = known_names::player_name(gm_player_id),
        subject_entity_id = entity_id,
        subject_entity_name = known_names::player_name(player_id),
        subject_player_id = player_id,
        subject_player_name = known_names::player_name(player_id),
        item_id,
        item_name = cimmeria_names::book().item(item_id),
        ammo_type,
        quantity = rounds,
        returned = out.returned,
        remainder = out.remainder,
        stack_before = out.stack_before,
        stack_after = out.stack_after,
        stacks_touched = out.changes.len(),
        "gm_give_ammo: rounds granted"
    );
    send_full_inventory_update(
        entity_id,
        player_id,
        pool,
        transport,
        connected,
        entity_to_addr,
    )
    .await;
    tell_gm(line).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use cimmeria_entity::ammo_type::{BULLET_HOLLOW_POINT, DAGGER_DEFAULT};

    fn msg(entity_id: u32, rounds: i32) -> GmGiveAmmo {
        GmGiveAmmo {
            entity_id,
            player_id: 72,
            gm_entity_id: 1,
            gm_player_id: 71,
            gm_account_id: Some(601),
            ammo_type: BULLET_HOLLOW_POINT,
            item_id: 9001,
            rounds,
        }
    }

    fn out(returned: i32, remainder: i32, after: i32) -> AmmoReturn {
        AmmoReturn {
            item_id: Some(9001),
            returned,
            remainder,
            stack_before: after - returned,
            stack_after: after,
            changes: Vec::new(),
        }
    }

    #[test]
    fn am06_result_line_full_grant_names_type_item_and_total() {
        let line = result_line(&msg(1, 700), &out(700, 0, 750));
        assert_eq!(
            line,
            ".giveammo: gave you 700 rounds of Bullet_Hollow_Point (item 9001); 750 now carried"
        );
    }

    /// The coordinator's rule: leftover rounds come back to the GM visibly.
    #[test]
    fn am06_result_line_partial_grant_reports_what_did_not_fit() {
        let line = result_line(&msg(2, 700), &out(500, 200, 500));
        assert!(line.contains("gave entity 2 500 rounds"), "{line}");
        assert!(line.ends_with("; 200 did not fit: bags full"), "{line}");
    }

    #[test]
    fn am06_result_line_nothing_fit_or_no_reserve_item_is_a_refusal() {
        let line = result_line(&msg(1, 10), &out(0, 10, 0));
        assert!(line.starts_with(".giveammo: refused, bags full"), "{line}");
        let mut m = msg(1, 10);
        m.ammo_type = DAGGER_DEFAULT;
        let none = AmmoReturn {
            remainder: 10,
            ..AmmoReturn::default()
        };
        let line = result_line(&m, &none);
        assert!(
            line.contains("Dagger_Default has no reserve item"),
            "{line}"
        );
    }
}
