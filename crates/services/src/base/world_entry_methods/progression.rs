use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use sqlx::PgPool;
use tokio::net::UdpSocket;

use cimmeria_game::player::{MAX_LEVEL, TRAINING_POINTS_PER_LEVEL};

use crate::mercury::{build_entity_method_packet, method_idx};
use super::super::helpers::send_to_witness;
use super::super::ConnectedClientState;

const LEVEL_XP: [u64; 21] = [
    0, 100, 200, 300, 600, 1_000, 1_600, 2_500, 4_000, 6_000, 9_000, 14_000, 18_000, 25_000,
    40_000, 60_000, 90_000, 120_000, 180_000, 250_000, 400_000,
];

const GENERICPROPERTY_TRAINING_POINTS: i32 = 1;

/// Handle XP grant from CellService -- compute level-ups and send client notifications.
///
/// Matches the Python `giveExperience()` flow: add XP, send updates, fire level-up events.
pub async fn handle_grant_xp(
    entity_id: u32,
    xp_amount: u64,
    socket: &Arc<UdpSocket>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let addr = {
        let map = entity_to_addr.lock().unwrap();
        match map.get(&entity_id) {
            Some(a) => *a,
            None => {
                tracing::warn!(entity_id, "GrantXP: no address for entity");
                return;
            }
        }
    };

    let (total_xp, new_level, training_points, levels_gained) = {
        let mut map = connected.lock().unwrap();
        let state = match map.get_mut(&addr) {
            Some(s) => s,
            None => {
                tracing::warn!(entity_id, "GrantXP: no connected state for entity");
                return;
            }
        };

        let mut xp = state.player_xp.unwrap_or(0);
        let mut level = state.player_level.unwrap_or(1) as u32;
        let mut tp = state.player_training_points.unwrap_or(0);

        xp += xp_amount;

        let mut gained = Vec::new();
        while level < MAX_LEVEL && xp > LEVEL_XP[level as usize] {
            level += 1;
            tp += TRAINING_POINTS_PER_LEVEL;
            gained.push(level);
        }

        state.player_xp = Some(xp);
        state.player_level = Some(level as i32);
        state.player_training_points = Some(tp);

        (xp, level, tp, gained)
    };

    tracing::info!(
        entity_id,
        xp_amount,
        total_xp,
        new_level,
        levels_up = levels_gained.len(),
        "GrantXP processed"
    );

    send_to_witness(
        socket,
        connected,
        entity_to_addr,
        entity_id,
        |key, seq, acks| {
            build_entity_method_packet(
                key,
                seq,
                acks,
                entity_id,
                method_idx::ON_EXP_UPDATE,
                &(total_xp as i32).to_le_bytes(),
            )
        },
    )
    .await;

    for &lvl in &levels_gained {
        send_to_witness(
            socket,
            connected,
            entity_to_addr,
            entity_id,
            |key, seq, acks| {
                build_entity_method_packet(
                    key,
                    seq,
                    acks,
                    entity_id,
                    method_idx::GIVE_XP_FOR_LEVEL,
                    &(lvl as i32).to_le_bytes(),
                )
            },
        )
        .await;

        let next_threshold = if lvl >= MAX_LEVEL {
            LEVEL_XP[MAX_LEVEL as usize] as i32
        } else {
            LEVEL_XP[lvl as usize] as i32
        };
        send_to_witness(
            socket,
            connected,
            entity_to_addr,
            entity_id,
            |key, seq, acks| {
                build_entity_method_packet(
                    key,
                    seq,
                    acks,
                    entity_id,
                    method_idx::ON_MAX_EXP_UPDATE,
                    &next_threshold.to_le_bytes(),
                )
            },
        )
        .await;
    }

    if !levels_gained.is_empty() {
        send_to_witness(
            socket,
            connected,
            entity_to_addr,
            entity_id,
            |key, seq, acks| {
                build_entity_method_packet(
                    key,
                    seq,
                    acks,
                    entity_id,
                    method_idx::ON_LEVEL_UPDATE,
                    &(new_level as i32).to_le_bytes(),
                )
            },
        )
        .await;

        let mut tp_args = Vec::with_capacity(8);
        tp_args.extend_from_slice(&GENERICPROPERTY_TRAINING_POINTS.to_le_bytes());
        tp_args.extend_from_slice(&(training_points as i32).to_le_bytes());
        send_to_witness(
            socket,
            connected,
            entity_to_addr,
            entity_id,
            |key, seq, acks| {
                build_entity_method_packet(
                    key,
                    seq,
                    acks,
                    entity_id,
                    method_idx::ON_ENTITY_PROPERTY,
                    &tp_args,
                )
            },
        )
        .await;
    }
}

/// Handle cash grant from CellService -- update DB and send client notification.
pub async fn handle_grant_cash(
    entity_id: u32,
    amount: i32,
    db_pool: &Option<Arc<PgPool>>,
    socket: &Arc<UdpSocket>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let addr = {
        let map = entity_to_addr.lock().unwrap();
        match map.get(&entity_id) {
            Some(a) => *a,
            None => {
                tracing::warn!(entity_id, "GrantCash: no address for entity");
                return;
            }
        }
    };
    let account_id = {
        let clients = connected.lock().unwrap();
        match clients.get(&addr) {
            Some(c) => c.account_id,
            None => {
                tracing::warn!(entity_id, "GrantCash: no connected state");
                return;
            }
        }
    };

    if let Some(pool) = db_pool {
        let new_total: Option<i32> = sqlx::query_scalar(
            "UPDATE sgw_player SET naquadah = naquadah + $1 \
             WHERE account_id = $2 \
             RETURNING naquadah",
        )
        .bind(amount)
        .bind(account_id as i32)
        .fetch_optional(pool.as_ref())
        .await
        .unwrap_or(None);

        let total = new_total.unwrap_or(amount);
        tracing::info!(entity_id, amount, total, "GrantCash: updated naquadah");

        send_to_witness(
            socket,
            connected,
            entity_to_addr,
            entity_id,
            |key, seq, acks| {
                build_entity_method_packet(
                    key,
                    seq,
                    acks,
                    entity_id,
                    method_idx::ON_CASH_CHANGED,
                    &total.to_le_bytes(),
                )
            },
        )
        .await;
    } else {
        tracing::debug!(
            entity_id,
            amount,
            "GrantCash: no DB pool, sending untracked"
        );
        send_to_witness(
            socket,
            connected,
            entity_to_addr,
            entity_id,
            |key, seq, acks| {
                build_entity_method_packet(
                    key,
                    seq,
                    acks,
                    entity_id,
                    method_idx::ON_CASH_CHANGED,
                    &amount.to_le_bytes(),
                )
            },
        )
        .await;
    }
}