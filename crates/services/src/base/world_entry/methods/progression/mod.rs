use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::channel_bundle::{ChannelBundle, IDBASE_SGW_PLAYER};
use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;

use cimmeria_game::player::{MAX_LEVEL, TRAINING_POINTS_PER_LEVEL};

use super::super::super::gm_feedback::send_gm_feedback_to_client;
use super::super::super::helpers::{send_bundle_to_witness_reliable, send_to_witness_reliable};
use super::super::super::ConnectedClientState;
use crate::mercury::{build_player_entity_method_packet, method_idx};

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
#[tracing::instrument(
    name = "progression.grant_xp",
    level = "info",
    skip_all,
    fields(entity_id, xp_amount)
)]
pub async fn handle_grant_xp(
    entity_id: u32,
    xp_amount: u64,
    notify_gm: bool,
    db_pool: &Option<Arc<PgPool>>,
    transport: &Arc<dyn Transport>,
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

    // Bundle the post-grant notifications into a single Mercury frame.
    //
    // **Transaction-state audit**: every message in this bundle targets the
    // player's own `entity_id`, created in `handle_map_loaded`'s prior bundle
    // (the CREATE_BASE_PLAYER transaction released between bundles). No
    // CREATE_ENTITY / CELL_PLAYER fires here; the bundle is exclusively
    // post-transaction property/method updates — canonical "safe to combine"
    // per [docs/architecture/mercury-bundle.md].
    //
    // Pre-bundle burst-shape: 1 (always) + 2 × levels_gained.len() (per-level
    // pair) + 2 (if any level gained) = 1..2N+3 packets where N = number of
    // levels gained. Typical small grant: 1 packet. Worst case (max-level
    // catch-up): 2N+3 packets. Post-bundle: 1 packet (body fits one fragment
    // for any realistic N — each per-level pair is ~30 B and MAX_LEVEL=20
    // caps the per-grant level delta, so the body stays well under
    // FRAGMENT_BODY_SIZE = 1300 B). Pinned by
    // `grant_xp_max_level_burst_bundles_to_single_packet`.
    let bundle = build_grant_xp_bundle(
        entity_id,
        total_xp,
        new_level,
        training_points,
        &levels_gained,
    );
    send_bundle_to_witness_reliable(transport, connected, entity_to_addr, entity_id, bundle).await;

    // Definitive GM feedback (only for GM-sourced grants — mob-kill XP leaves
    // `notify_gm` false). Fired only here, on the true success path: every
    // failure branch above returns early without reaching this point.
    if notify_gm {
        send_gm_feedback_to_client(
            entity_id,
            &format!("gmGiveXp: now level {new_level} ({total_xp} xp total)"),
            transport,
            connected,
            entity_to_addr,
        )
        .await;
    }
}

/// Compose the post-grant_xp notification burst into a single Mercury bundle.
///
/// Order matches the pre-bundle dispatch sequence so a regression that
/// reorders two entries is caught by the order-sensitive
/// `grant_xp_*` burst-shape regression guards in
/// [`mod tests`]:
///   1. `onExpUpdate`                 (always)
///   2. per level gained, in order:
///      a. `GIVE_XP_FOR_LEVEL`
///      b. `onMaxExpUpdate`
///   3. `onLevelUpdate`               (only if any level gained)
///   4. `onEntityProperty(TRAINING_POINTS)` (only if any level gained)
///
/// Extracted as a pure builder so the burst-shape regression guard can pin
/// `num_messages` and `estimated_packet_count()` against the same composition
/// the handler emits (call-site duplication would let the test and the
/// handler drift).
fn build_grant_xp_bundle(
    entity_id: u32,
    total_xp: u64,
    new_level: u32,
    training_points: u32,
    levels_gained: &[u32],
) -> ChannelBundle {
    let mut bundle = ChannelBundle::new(true);
    bundle.append_entity_method(
        method_idx::ON_EXP_UPDATE,
        IDBASE_SGW_PLAYER,
        entity_id,
        // Wire format is i32; saturate so a u64 total exceeding 2^31-1
        // doesn't wrap negative on the client display.
        &(total_xp.min(i32::MAX as u64) as i32).to_le_bytes(),
    );

    for &lvl in levels_gained {
        bundle.append_entity_method(
            method_idx::GIVE_XP_FOR_LEVEL,
            IDBASE_SGW_PLAYER,
            entity_id,
            &(lvl as i32).to_le_bytes(),
        );
        let next_threshold = if lvl >= MAX_LEVEL {
            LEVEL_XP[MAX_LEVEL as usize] as i32
        } else {
            LEVEL_XP[lvl as usize] as i32
        };
        bundle.append_entity_method(
            method_idx::ON_MAX_EXP_UPDATE,
            IDBASE_SGW_PLAYER,
            entity_id,
            &next_threshold.to_le_bytes(),
        );
    }

    if !levels_gained.is_empty() {
        bundle.append_entity_method(
            method_idx::ON_LEVEL_UPDATE,
            IDBASE_SGW_PLAYER,
            entity_id,
            &(new_level as i32).to_le_bytes(),
        );

        let mut tp_args = Vec::with_capacity(8);
        tp_args.extend_from_slice(&GENERICPROPERTY_TRAINING_POINTS.to_le_bytes());
        tp_args.extend_from_slice(&(training_points as i32).to_le_bytes());
        bundle.append_entity_method(
            method_idx::ON_ENTITY_PROPERTY,
            IDBASE_SGW_PLAYER,
            entity_id,
            &tp_args,
        );
    }

    bundle
}

/// Handle cash grant from CellService -- update DB and send client notification.
#[tracing::instrument(
    name = "progression.grant_cash",
    level = "info",
    skip_all,
    fields(entity_id, player_id, amount)
)]
pub async fn handle_grant_cash(
    entity_id: u32,
    player_id: i32,
    amount: i32,
    notify_gm: bool,
    db_pool: &Option<Arc<PgPool>>,
    transport: &Arc<dyn Transport>,
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

        send_to_witness_reliable(
            transport,
            connected,
            entity_to_addr,
            entity_id,
            |key, seq, acks| {
                build_player_entity_method_packet(
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

        // Definitive GM feedback — only for GM-sourced grants (loot pickup
        // leaves `notify_gm` false). Inside the `Some(pool)` + `Ok(Some(total))`
        // success path: the row was found and updated.
        if notify_gm {
            send_gm_feedback_to_client(
                entity_id,
                &format!("gmGiveCash: +{amount} naquadah (total {total})"),
                transport,
                connected,
                entity_to_addr,
            )
            .await;
        }
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

/// Persist a trained ability + debit one training point.
///
/// Cell pre-validates archetype tree + prereqs (see Phase 5b);
/// base only validates training_points >= 1 and the DB UPDATE returning
/// `rows_affected == 1`. On success, sends
/// `BaseToCellMsg::AbilityGranted` so the cell can add to
/// `entity.abilities` and broadcast `onKnownAbilitiesUpdate`.
///
/// Persistence shape: `UPDATE sgw_player SET abilities = abilities || $1,
/// training_points = training_points - 1 WHERE player_id = $2 AND
/// training_points > 0`. The `training_points > 0` guard is the DB-side
/// authority — even if the in-memory training_points view is stale, the
/// row update only fires when actual rowstate allows.
#[tracing::instrument(
    name = "progression.train_ability",
    level = "info",
    skip_all,
    fields(entity_id, player_id, ability_id)
)]
pub async fn handle_train_ability(
    entity_id: u32,
    player_id: i32,
    ability_id: i32,
    db_pool: &Option<Arc<PgPool>>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    cell_tx: &Option<tokio::sync::mpsc::Sender<crate::cell::messages::BaseToCellMsg>>,
    _transport: &Arc<dyn Transport>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let pool = match db_pool {
        Some(p) => p,
        None => {
            tracing::warn!(entity_id, player_id, ability_id, "TrainAbility: no DB pool");
            return;
        }
    };

    let addr = match entity_to_addr.lock().unwrap().get(&entity_id).copied() {
        Some(a) => a,
        None => {
            tracing::warn!(entity_id, "TrainAbility: no address for entity");
            return;
        }
    };

    // Fast-path check: the UPDATE's `training_points > 0` guard is the
    // authoritative gate (atomic against the DB row), but a stale-cache
    // pre-check spares a DB round-trip on the common "out of TP" case.
    {
        let map = match connected.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let tp_in_memory = map
            .get(&addr)
            .and_then(|s| s.player_training_points)
            .unwrap_or(0);
        if tp_in_memory == 0 {
            tracing::info!(
                entity_id,
                player_id,
                ability_id,
                "TrainAbility: rejected — no training points available (in-memory)"
            );
            return;
        }
    }

    // Atomic: append ability_id + debit, but ONLY if training_points > 0
    // AND the ability isn't already present. The `NOT (abilities @> ARRAY[$1])`
    // clause prevents double-debit if two concurrent or replayed
    // TrainAbility messages for the same ability arrive: the second
    // returns 0 rows and the cell-side path treats that as a no-op.
    // Without this, a player who clicks Train twice fast could lose two
    // training points for one ability.
    let result = sqlx::query_scalar::<_, i32>(
        "UPDATE sgw_player \
            SET abilities = abilities || $1::integer, \
                training_points = training_points - 1 \
          WHERE player_id = $2 \
            AND training_points > 0 \
            AND NOT (abilities @> ARRAY[$1::integer]) \
        RETURNING training_points",
    )
    .bind(ability_id)
    .bind(player_id)
    .fetch_optional(pool.as_ref())
    .await;

    let training_points_remaining = match result {
        Ok(Some(tp)) => tp,
        Ok(None) => {
            tracing::info!(
                entity_id,
                player_id,
                ability_id,
                "TrainAbility: UPDATE matched 0 rows (player_id missing or no training_points)"
            );
            return;
        }
        Err(e) => {
            tracing::error!(
                entity_id,
                player_id,
                ability_id,
                "TrainAbility: UPDATE failed: {e}"
            );
            return;
        }
    };

    // Sync in-memory training_points so the next train attempt sees the
    // post-debit value without a DB read.
    {
        let mut map = match connected.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        if let Some(state) = map.get_mut(&addr) {
            state.player_training_points = Some(training_points_remaining as u32);
        }
    }

    tracing::info!(
        entity_id,
        player_id,
        ability_id,
        training_points_remaining,
        "TrainAbility: persisted + debited"
    );

    // Notify cell so it adds the ability + broadcasts onKnownAbilitiesUpdate.
    // If the channel is gone, the player's hotbar will be one ability behind
    // until next relog — log loudly so SigNoz surfaces the desync.
    if let Some(tx) = cell_tx {
        if let Err(e) = tx
            .send(crate::cell::messages::BaseToCellMsg::AbilityGranted {
                entity_id,
                ability_id,
                training_points_remaining,
            })
            .await
        {
            tracing::error!(
                entity_id, ability_id, error = %e,
                "TrainAbility: base→cell AbilityGranted send failed; hotbar will desync until relog"
            );
        }
    }
}

#[cfg(test)]
mod tests;
