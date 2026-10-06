//! `.spawnset [list | on <setId> | off <setId> | clear]`: the console door to
//! the switchable spawn sets (Debug Area DA-10, the Visual NPC Lineup).
//!
//! `on` / `off` are the native `activateSpawnSet` (214) /
//! `deactivateSpawnSet` (215); `clear` switches off every set that is on in
//! the caller's world; `list` (the default) shows every set, its size and
//! whether it is on. All of them call `cimmeria-cell-content`'s
//! `spawn_sets`, the code the lineup attendants run.

use tokio::sync::mpsc;

use cimmeria_cell_content::cell::content::spawn_sets::{
    hide_spawn_set, log_switch, show_spawn_set, SpawnSetSwitch,
};

use super::super::send_gm_feedback;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

const USAGE: &str = ".spawnset [list | on <setId> | off <setId> | clear]";

pub(super) async fn spawn_set(
    caller_id: u32,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    match args.first().copied().unwrap_or("list") {
        "list" => list(caller_id, tx, space_mgr).await,
        sub @ ("on" | "off") => {
            let Some(set_id) = super::super::parse_i32(caller_id, args, 1, "setId", tx).await
            else {
                return;
            };
            let switch = if sub == "on" {
                show_spawn_set(set_id, tx, space_mgr).await
            } else {
                hide_spawn_set(set_id, tx, space_mgr).await
            };
            log_switch("console", caller_id, Some(set_id), &switch, space_mgr);
            send_gm_feedback(caller_id, &switch.line(), tx).await;
        }
        "clear" => clear(caller_id, tx, space_mgr).await,
        other => {
            send_gm_feedback(
                caller_id,
                &format!(".spawnset: unknown action '{other}'. Usage: {USAGE}"),
                tx,
            )
            .await;
        }
    }
}

async fn list(caller_id: u32, tx: &mpsc::Sender<CellToBaseMsg>, space_mgr: &SpaceManager) {
    if space_mgr.spawn_sets.is_empty() {
        send_gm_feedback(caller_id, ".spawnset: no spawn sets are loaded.", tx).await;
        return;
    }
    for set in space_mgr.spawn_sets.iter() {
        let state = if set.is_active() {
            format!("ON ({} live)", set.live.len())
        } else {
            "off".to_string()
        };
        let line = format!(
            "{} {} [{}, {}]: {} actors, {state}",
            set.set_id,
            set.name,
            set.kind,
            set.world_name.as_deref().unwrap_or("no world"),
            set.records.len()
        );
        send_gm_feedback(caller_id, &line, tx).await;
    }
}

/// Switch off every set that is on in the caller's world.
async fn clear(caller_id: u32, tx: &mpsc::Sender<CellToBaseMsg>, space_mgr: &mut SpaceManager) {
    let world = space_mgr
        .get_entity_space_id(caller_id)
        .and_then(|id| space_mgr.spaces.get(&id))
        .map(|s| s.world_name.clone());
    let on: Vec<i32> = space_mgr
        .spawn_sets
        .iter()
        .filter(|s| s.is_active() && s.world_name.is_some() && s.world_name == world)
        .map(|s| s.set_id)
        .collect();
    if on.is_empty() {
        let switch = SpawnSetSwitch::NothingShowing { name: None };
        log_switch("console", caller_id, None, &switch, space_mgr);
        send_gm_feedback(caller_id, &switch.line(), tx).await;
        return;
    }
    for set_id in on {
        let switch = hide_spawn_set(set_id, tx, space_mgr).await;
        log_switch("console", caller_id, Some(set_id), &switch, space_mgr);
        send_gm_feedback(caller_id, &switch.line(), tx).await;
    }
}
