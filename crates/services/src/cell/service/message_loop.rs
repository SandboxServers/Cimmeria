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
                super::ticks::run_aoi_tick(tx, &mut space_mgr).await;

                aoi_tick_counter = aoi_tick_counter.wrapping_add(1);

                // Promote pending reloads whose warmup deadline has elapsed.
                // Runs before NPC movement so any onStatUpdate from the refill
                // is delivered in the same tick as other AoI-driven updates.
                super::ticks::reload_completion_tick(tx, &mut space_mgr).await;

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

                // Out-of-combat health regen — 1 Hz (every 10th 100ms tick).
                // Cadence is wired here so the per-call delta in `regen_tick`
                // can stay "HP per second" without an internal time check.
                if aoi_tick_counter.is_multiple_of(10) {
                    super::ticks::regen_tick(tx, &mut space_mgr).await;
                }
            }
        }
    }

    tracing::debug!("Cell service message loop exited");
}
