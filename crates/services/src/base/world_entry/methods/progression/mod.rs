use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use sqlx::PgPool;
use tokio::net::UdpSocket;

use cimmeria_game::player::{MAX_LEVEL, TRAINING_POINTS_PER_LEVEL};

use super::super::super::helpers::send_to_witness;
use super::super::super::ConnectedClientState;
use crate::mercury::{build_entity_method_packet, method_idx};

const LEVEL_XP: [u64; 21] = [
    0, 100, 200, 300, 600, 1_000, 1_600, 2_500, 4_000, 6_000, 9_000, 14_000, 18_000, 25_000,
    40_000, 60_000, 90_000, 120_000, 180_000, 250_000, 400_000,
];

// Compile-time guard: LEVEL_XP must cover every level from 1 through MAX_LEVEL,
// indexed by current-level (1-based), so its length must equal MAX_LEVEL + 1.
const _: () = assert!(
    LEVEL_XP.len() == MAX_LEVEL as usize + 1,
    "LEVEL_XP table length must equal MAX_LEVEL + 1; update LEVEL_XP when MAX_LEVEL changes"
);

const GENERICPROPERTY_TRAINING_POINTS: i32 = 1;

/// Handle XP grant from CellService -- compute level-ups, persist, and send
/// client notifications.
///
/// Matches the Python `giveExperience()` flow: add XP, send updates, fire
/// level-up events. Persists exp/level/training_points to `sgw_player` before
/// emitting wire packets so a relog after a grant doesn't roll the player back.
pub async fn handle_grant_xp(
    entity_id: u32,
    xp_amount: u64,
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
                tracing::warn!(entity_id, "GrantXP: no address for entity");
                return;
            }
        }
    };

    // Read current state and compute the new values WITHOUT mutating state
    // yet. We defer the in-memory update until after DB persistence succeeds
    // — Copilot caught that the previous order (mutate-then-persist) leaked
    // a failed grant onto the next successful one: the next GrantXP would
    // saturate-add on top of the unpersisted value and then write the
    // combined sum, effectively persisting the grant we tried to drop.
    let (player_id, total_xp, new_level, training_points, levels_gained) = {
        // Tolerate poison instead of panicking — another thread crashing the
        // mutex shouldn't stop XP grants from continuing on the recovered state.
        let map = match connected.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let state = match map.get(&addr) {
            Some(s) => s,
            None => {
                tracing::warn!(entity_id, "GrantXP: no connected state for entity");
                return;
            }
        };

        let player_id = state.active_player_id;
        let prev_xp = state.player_xp.unwrap_or(0);
        let mut level = state.player_level.unwrap_or(1) as u32;
        let mut tp = state.player_training_points.unwrap_or(0);

        // Saturate to prevent a pathological grant (e.g., from a corrupted
        // GrantXP message) wrapping the accumulator and producing a negative
        // wire value or a phantom delevel.
        let xp = prev_xp.saturating_add(xp_amount);

        let mut gained = Vec::new();
        while level < MAX_LEVEL && xp > LEVEL_XP[level as usize] {
            level += 1;
            tp += TRAINING_POINTS_PER_LEVEL;
            gained.push(level);
        }

        (player_id, xp, level, tp, gained)
    };

    // Persist first. On DB failure we return WITHOUT mutating in-memory
    // state and WITHOUT emitting wire packets — the next GrantXP will then
    // recompute from the truly persisted values rather than compounding the
    // failure.
    match (db_pool, player_id) {
        (Some(pool), Some(player_id)) => {
            // `sgw_player.exp` is `integer`; saturate to i32::MAX so a u64
            // total exceeding 2^31-1 doesn't wrap negative on either the
            // column or the wire payload (also i32).
            let exp_i32 = total_xp.min(i32::MAX as u64) as i32;
            match sqlx::query(
                "UPDATE sgw_player \
                    SET exp = $1, level = $2, training_points = $3 \
                  WHERE player_id = $4",
            )
            .bind(exp_i32)
            .bind(new_level as i32)
            .bind(training_points as i32)
            .bind(player_id)
            .execute(pool.as_ref())
            .await
            {
                Ok(r) if r.rows_affected() == 0 => {
                    tracing::warn!(
                        entity_id, player_id, total_xp, new_level,
                        "GrantXP: 0 rows updated (player_id missing from sgw_player); dropping wire emit"
                    );
                    return;
                }
                Ok(_) => {}
                Err(e) => {
                    tracing::error!(
                        entity_id,
                        player_id,
                        "GrantXP: persistence UPDATE failed: {e}"
                    );
                    return;
                }
            }
        }
        (Some(_), None) => {
            // No active character (play-character flow incomplete). Skip
            // persistence; the in-memory grant will be lost on reconnect, but
            // we shouldn't be granting XP before character selection anyway.
            tracing::warn!(
                entity_id, total_xp, new_level,
                "GrantXP: no active_player_id — skipping persist (likely a pre-character-select grant)"
            );
        }
        (None, _) => {
            // Mirrors the no-DB branch in handle_grant_cash: the in-memory
            // state is updated but un-authoritative. Log loudly.
            tracing::warn!(
                entity_id,
                total_xp,
                new_level,
                "GrantXP: no DB pool — XP/level not persisted, will be lost on reconnect"
            );
        }
    }

    // DB write succeeded (or we're in a no-persist best-effort branch).
    // Apply the in-memory mutation now so subsequent reads see the new state.
    {
        let mut map = match connected.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        if let Some(state) = map.get_mut(&addr) {
            state.player_xp = Some(total_xp);
            state.player_level = Some(new_level as i32);
            state.player_training_points = Some(training_points);
        }
    }

    tracing::info!(
        entity_id,
        xp_amount,
        total_xp,
        new_level,
        levels_up = levels_gained.len(),
        "GrantXP processed"
    );

    let _ = send_to_witness(
        socket,
        connected,
        entity_to_addr,
        entity_id,
        entity_id,
        "METHOD",
        |key, seq, acks| {
            build_entity_method_packet(
                key,
                seq,
                acks,
                entity_id,
                method_idx::ON_EXP_UPDATE,
                // Wire format is i32; saturate so a u64 total exceeding 2^31-1
                // doesn't wrap negative on the client display.
                &(total_xp.min(i32::MAX as u64) as i32).to_le_bytes(),
            )
        },
    )
    .await;

    for &lvl in &levels_gained {
        let _ = send_to_witness(
            socket,
            connected,
            entity_to_addr,
            entity_id,
            entity_id,
            "METHOD",
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
        let _ = send_to_witness(
            socket,
            connected,
            entity_to_addr,
            entity_id,
            entity_id,
            "METHOD",
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
        let _ = send_to_witness(
            socket,
            connected,
            entity_to_addr,
            entity_id,
            entity_id,
            "METHOD",
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
        let _ = send_to_witness(
            socket,
            connected,
            entity_to_addr,
            entity_id,
            entity_id,
            "METHOD",
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
    player_id: i32,
    amount: i32,
    db_pool: &Option<Arc<PgPool>>,
    socket: &Arc<UdpSocket>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let _addr = {
        let map = entity_to_addr.lock().unwrap();
        match map.get(&entity_id) {
            Some(a) => *a,
            None => {
                tracing::warn!(entity_id, player_id, "GrantCash: no address for entity");
                return;
            }
        }
    };

    if let Some(pool) = db_pool {
        let new_total: i32 = match sqlx::query_scalar::<_, i32>(
            "UPDATE sgw_player SET naquadah = naquadah + $1 \
             WHERE player_id = $2 \
             RETURNING naquadah",
        )
        .bind(amount)
        .bind(player_id)
        .fetch_optional(pool.as_ref())
        .await
        {
            Ok(Some(total)) => total,
            Ok(None) => {
                tracing::warn!(
                    entity_id,
                    player_id,
                    amount,
                    "GrantCash: player row not found, dropping grant"
                );
                return;
            }
            Err(e) => {
                tracing::error!(
                    entity_id,
                    player_id,
                    amount,
                    "GrantCash: UPDATE failed: {e}"
                );
                return;
            }
        };

        let total = new_total;
        tracing::info!(entity_id, amount, total, "GrantCash: updated naquadah");

        let _ = send_to_witness(
            socket,
            connected,
            entity_to_addr,
            entity_id,
            entity_id,
            "METHOD",
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
        // No-DB-pool mode: we have no authoritative balance to send. Drop the
        // grant entirely rather than emitting onCashChanged with the *delta*
        // as the absolute total — the client treats the payload as a new total
        // and would desync from what the server (eventually) persists.
        tracing::warn!(
            entity_id,
            player_id,
            amount,
            "GrantCash: no DB pool, dropping grant (cannot send authoritative onCashChanged)"
        );
    }
}

#[cfg(test)]
mod tests;
