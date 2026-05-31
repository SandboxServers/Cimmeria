//! Main CellService message-processing loop and tick scheduler.

use std::sync::Arc;

use sqlx::PgPool;
use tokio::sync::{mpsc, Notify};

use cimmeria_content_engine::chain::ChainEngine;

use super::super::content;
use super::super::messages::{BaseToCellMsg, CellToBaseMsg};
use super::super::space_manager::SpaceManager;
use super::super::spawner;

/// Main CellService message processing loop.
///
/// Exits when `shutdown` is notified, the BaseToCell channel closes, or the
/// task is dropped. `CellService::stop()` uses the `shutdown` arm to request
/// a clean exit and then awaits the join handle.
pub(super) async fn run_cell_loop(
    rx: &mut mpsc::Receiver<BaseToCellMsg>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    mut space_mgr: SpaceManager,
    mut engine: ChainEngine,
    db_pool: Option<Arc<PgPool>>,
    spawn_records: Vec<spawner::SpawnRecord>,
    shutdown: Arc<Notify>,
) {
    tracing::debug!("Cell service message loop started");

    let mut tick_interval = tokio::time::interval(std::time::Duration::from_millis(100));
    let mut aoi_tick_counter: u32 = 0;

    loop {
        tokio::select! {
            _ = shutdown.notified() => {
                tracing::info!("Cell service received shutdown signal — exiting loop");
                break;
            }
            msg = rx.recv() => {
                match msg {
                    Some(BaseToCellMsg::ReloadContentEngine) => {
                        tracing::info!("Hot-reloading content engine from database");
                        engine = content::build_engine(db_pool.as_deref()).await;
                        tracing::info!(chains = engine.chain_count(), "Content engine reloaded");
                    }
                    Some(msg) => super::base_messages::handle_base_message(msg, tx, &mut space_mgr, &engine, &spawn_records).await,
                    None => {
                        tracing::info!("Cell service channel closed — shutting down");
                        break;
                    }
                }
            }
            _ = tick_interval.tick() => {
                // `.instrument()` wrapping `async { … }.await` — the tick
                // body awaits, so a thread-local guard from `.entered()`
                // would silently fall off across runtime thread switches.
                use tracing::Instrument;
                let tick_span = tracing::debug_span!(
                    "cell.tick",
                    tick = aoi_tick_counter,
                );
                async {
                    super::ticks::run_aoi_tick(tx, &mut space_mgr).await;

                    aoi_tick_counter = aoi_tick_counter.wrapping_add(1);

                // Promote pending reloads whose warmup deadline has elapsed.
                // Runs before NPC movement so any onStatUpdate from the refill
                // is delivered in the same tick as other AoI-driven updates.
                super::ticks::reload_completion_tick(tx, &mut space_mgr).await;

                // Fire deferred holster for any player whose
                // out-of-combat grace window has elapsed (Phase 3 of
                // ). Cheap — the inner filter short-circuits to
                // the empty set unless someone just exited combat.
                super::ticks::holster_timer_tick(tx, &mut space_mgr).await;

                // Promote pending reload-while-holstered (Phase A → B):
                // any player whose draw animation has had time to play
                // gets the actual reload kicked off now. Cheap, same
                // filter shape as the holster tick.
                super::ticks::pending_reload_tick(tx, &mut space_mgr).await;

                // Promote queued attack-while-holstered: any player
                // whose draw window has elapsed gets the deferred
                // ability dispatched. Same filter shape — short-circuits
                // when the queue is empty.
                super::ticks::pending_attack_tick(tx, &mut space_mgr, &engine).await;

                // Drive the server-side auto-cycle loop: re-fire any
                // armed player's stashed ability against the LIVE
                // current_target_id whenever its cooldown has cleared.
                // Cooldown gate is the rate limiter; the eligibility
                // filter short-circuits when nobody is auto-cycling.
                // Engine is threaded for the kill-credit hook:
                // loop-driven kills against quest-tagged NPCs fire
                // `EntityDeath` so KillCount missions advance via the
                // same content-engine path as manual right-click kills.
                super::ticks::auto_cycle_tick(tx, &mut space_mgr, &engine).await;

                // Promote queued bandolier slot swap: any player whose
                // Item_Unequip animation window has elapsed gets the
                // deferred slot change finalized (active slot update +
                // Item_Equip for the new weapon).
                super::ticks::pending_slot_swap_tick(tx, &mut space_mgr).await;

                // Drive ring transporter timers (hide / warmup / cooldown).
                // Each transporter holds its own deadlines; this tick fires
                // the transitions and dispatches their effects.
                super::super::ring_transport::run_tick_with_engine(
                    tx, &mut space_mgr, &engine,
                ).await;

                // NPC movement runs every AoI tick (100ms) for smooth pathing
                super::ticks::npc_movement_tick(&mut space_mgr);

                // NPC AI runs every 20th AoI tick (2 seconds at 100ms intervals)
                if aoi_tick_counter.is_multiple_of(20) {
                    super::npc_ai::npc_ai_tick(tx, &mut space_mgr).await;
                }

                // Retry sweep for NPCs with a pending `ai_retry_at`
                // deadline — runs every AoI tick (100ms) so a
                // `handle_use_ability` launch failure can drive a 500ms
                // retry instead of waiting up to 2 seconds for the
                // natural cadence. Healthy NPCs (no `ai_retry_at`) are
                // skipped on the snapshot scan, so the per-tick cost
                // here is just an `all_npc_entity_ids()` walk + the
                // pending-retry set membership — negligible. See
                // `npc_ai::npc_ai_retry_sweep` for the rationale.
                super::npc_ai::npc_ai_retry_sweep(tx, &mut space_mgr).await;

                // Out-of-combat HP/focus regen — 1 Hz (every 10th 100ms tick).
                // Cadence is wired here so the per-call delta in `regen_tick`
                // can stay "points per second" without an internal time check.
                if aoi_tick_counter.is_multiple_of(10) {
                    super::ticks::regen_tick(tx, &mut space_mgr).await;
                    // NPC respawn promotion — also 1 Hz. Cadence here
                    // because respawn timers are second-granular; the
                    // scan filter on `ai_state == Dead` short-circuits
                    // when no NPC is currently waiting to come back.
                    // See `ticks::npc_respawn` for the full state +
                    // wire-burst sequence.
                    super::ticks::npc_respawn_tick(tx, &mut space_mgr).await;
                }

                // Channel-interrupt-on-movement sweep — BEFORE the
                // pulse tick so cancelled channels don't fire one
                // last pulse on this same tick. Short-circuits when
                // no entity has any active channels.
                super::super::effects::channel_interrupt_on_movement_tick(
                    tx,
                    &mut space_mgr,
                )
                .await;

                // Active-effect pulsing — DoT / HoT / timed debuff
                // pulses fire whenever their per-instance schedule
                // elapses. Runs every AoI tick (100ms) so pulse
                // intervals down to 0.1s resolve correctly. The
                // entity filter inside the tick short-circuits when
                // nobody has any active effects — cheap on idle
                // worlds.
                super::super::effects::effect_pulse_tick(tx, &mut space_mgr).await;
                }
                .instrument(tick_span)
                .await;
            }
        }
    }

    tracing::debug!("Cell service message loop exited");
}
