//! Switching spawn sets on and off (Debug Area DA-10, the Visual NPC
//! Lineup). One implementation behind three doors: the native GM cell
//! methods `activateSpawnSet` (214) / `deactivateSpawnSet` (215), the
//! `.spawnset` console command, and the lineup attendants' `spawn_set`
//! content action.
//!
//! **Exclusive by kind.** Showing a set first switches off every other set of
//! its kind (`spawn_sets.type`) in its world, so only one lineup group is ever
//! loaded: the whole lineup at once ran the 32-bit client out of memory
//! (lab, 2026-10-05). The world is shared, so a switch changes what everyone
//! there sees; every line says so.
//!
//! **Clean despawn.** Switching a set off despawns each member through
//! `despawn_npc_releasing_combat`: players leave combat with it, and every
//! witness gets `LeftAoI` at once, so no ghost stays on a client.
//!
//! The caller checks authority (GM) and sends the line [`SpawnSetSwitch::line`]
//! returns; every switch logs one `spawn_set.switched` row.

use tokio::sync::mpsc;

use cimmeria_cell_combat::cell::combat::despawn_npc_releasing_combat;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::{DespawnOutcome, SpaceManager, SpawnSetError};

/// What a switch did, for the caller's chat line and the log row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpawnSetSwitch {
    /// The set is now on with `count` actors; `cleared` lists the sets of its
    /// kind that were switched off first, with their actor counts.
    Shown {
        name: String,
        world: String,
        count: usize,
        cleared: Vec<(String, usize)>,
    },
    /// The set was already on.
    AlreadyShowing { name: String, count: usize },
    /// The sets listed were switched off.
    Hidden {
        world: String,
        cleared: Vec<(String, usize)>,
    },
    /// Nothing was on to switch off (`name` is the set asked for, if any).
    NothingShowing { name: Option<String> },
    /// No set has this id.
    Unknown { set_id: i32 },
    /// The set's world has no running space, or the set has no member.
    NoSpace { name: String },
}

impl SpawnSetSwitch {
    /// The feedback line for the player who asked.
    pub fn line(&self) -> String {
        match self {
            Self::Shown {
                name,
                world,
                count,
                cleared,
            } => {
                let mut line = format!(
                    "{name}: showing {count} actors for everyone in {world} (one group at a time)."
                );
                if !cleared.is_empty() {
                    line.push_str(&format!(" Cleared first: {}.", list(cleared)));
                }
                line
            }
            Self::AlreadyShowing { name, count } => {
                format!("{name} is already showing ({count} actors).")
            }
            Self::Hidden { world, cleared } => {
                format!("Cleared {} for everyone in {world}.", list(cleared))
            }
            Self::NothingShowing { name: Some(name) } => format!("{name} is not showing."),
            Self::NothingShowing { name: None } => "Nothing to clear: no group is showing.".into(),
            Self::Unknown { set_id } => format!("There is no spawn set {set_id}."),
            Self::NoSpace { name } => {
                format!("{name} cannot be shown: its world is not running or the set is empty.")
            }
        }
    }

    /// The `decision_outcome` of the log row.
    pub fn outcome(&self) -> &'static str {
        match self {
            Self::Shown { .. } => "shown",
            Self::AlreadyShowing { .. } => "already_showing",
            Self::Hidden { .. } => "hidden",
            Self::NothingShowing { .. } => "nothing_showing",
            Self::Unknown { .. } => "unknown_set",
            Self::NoSpace { .. } => "no_space",
        }
    }
}

fn list(sets: &[(String, usize)]) -> String {
    sets.iter()
        .map(|(name, n)| format!("{name} ({n})"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Show `set_id`: switch off the other sets of its kind in its world, then
/// spawn its members. Idempotent: a set already on stays as it is.
pub async fn show_spawn_set(
    set_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> SpawnSetSwitch {
    let Some(set) = space_mgr.spawn_sets.get(set_id) else {
        return SpawnSetSwitch::Unknown { set_id };
    };
    let name = set.name.clone();
    if set.is_active() {
        return SpawnSetSwitch::AlreadyShowing {
            name,
            count: set.live.len(),
        };
    }
    let mut cleared = Vec::new();
    for peer in space_mgr.spawn_sets.active_peers(set_id) {
        if let Some(entry) = despawn_set(peer, tx, space_mgr).await {
            cleared.push(entry);
        }
    }
    match space_mgr.spawn_set_members(set_id) {
        Ok(live) => SpawnSetSwitch::Shown {
            world: world_of(space_mgr, set_id),
            name,
            count: live.len(),
            cleared,
        },
        Err(SpawnSetError::Unknown) => SpawnSetSwitch::Unknown { set_id },
        Err(SpawnSetError::AlreadyActive) => SpawnSetSwitch::AlreadyShowing {
            count: space_mgr.spawn_sets.get(set_id).map_or(0, |s| s.live.len()),
            name,
        },
        Err(SpawnSetError::NoSpace | SpawnSetError::AlreadyInactive) => {
            SpawnSetSwitch::NoSpace { name }
        }
    }
}

/// Hide `set_id`: despawn its members. Idempotent: a set already off stays
/// off.
pub async fn hide_spawn_set(
    set_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> SpawnSetSwitch {
    let Some(set) = space_mgr.spawn_sets.get(set_id) else {
        return SpawnSetSwitch::Unknown { set_id };
    };
    if !set.is_active() {
        return SpawnSetSwitch::NothingShowing {
            name: Some(set.name.clone()),
        };
    }
    let world = world_of(space_mgr, set_id);
    match despawn_set(set_id, tx, space_mgr).await {
        Some(entry) => SpawnSetSwitch::Hidden {
            world,
            cleared: vec![entry],
        },
        None => SpawnSetSwitch::NothingShowing {
            name: space_mgr.spawn_sets.get(set_id).map(|s| s.name.clone()),
        },
    }
}

/// Hide every set of `kind` in world `world_id` that is on.
pub async fn clear_spawn_set_kind(
    kind: &str,
    world_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> SpawnSetSwitch {
    let on: Vec<i32> = space_mgr
        .spawn_sets
        .of_kind(kind, world_id)
        .into_iter()
        .filter(|s| s.is_active())
        .map(|s| s.set_id)
        .collect();
    let Some(&first) = on.first() else {
        return SpawnSetSwitch::NothingShowing { name: None };
    };
    let world = world_of(space_mgr, first);
    let mut cleared = Vec::new();
    for set_id in on {
        if let Some(entry) = despawn_set(set_id, tx, space_mgr).await {
            cleared.push(entry);
        }
    }
    SpawnSetSwitch::Hidden { world, cleared }
}

/// Mark `set_id` off and despawn its members; `(name, count)` if it was on.
async fn despawn_set(
    set_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> Option<(String, usize)> {
    let name = space_mgr.spawn_sets.get(set_id)?.name.clone();
    let live = space_mgr.take_spawn_set_members(set_id).ok()?;
    let (mut despawned, mut witnesses, mut gone) = (0usize, 0usize, 0usize);
    for &npc_id in &live {
        match despawn_npc_releasing_combat(npc_id, "spawn_set_off", tx, space_mgr).await {
            DespawnOutcome::Despawned { witnesses_notified } => {
                despawned += 1;
                witnesses += witnesses_notified;
            }
            // Already gone (a GM `.despawn` of one member, say): nothing to
            // leave anyone's AoI.
            DespawnOutcome::NotFound | DespawnOutcome::RefusedPlayer => gone += 1,
        }
    }
    tracing::info!(
        target: "content",
        event = "spawn_set.despawned",
        set_id,
        set_name = %name,
        despawned,
        already_gone = gone,
        left_aoi_sent = witnesses,
        "spawn set members despawned"
    );
    Some((name, despawned))
}

fn world_of(space_mgr: &SpaceManager, set_id: i32) -> String {
    space_mgr
        .spawn_sets
        .get(set_id)
        .and_then(|s| s.world_name.clone())
        .unwrap_or_else(|| "its world".into())
}

/// Log one switch: who asked, through which door, and what happened.
pub fn log_switch(
    door: &'static str,
    entity_id: u32,
    set_id: Option<i32>,
    switch: &SpawnSetSwitch,
    space_mgr: &SpaceManager,
) {
    let who = space_mgr.player_identity(entity_id);
    let set_name = set_id.and_then(|id| space_mgr.spawn_sets.get(id).map(|s| s.name.clone()));
    tracing::info!(
        target: "content",
        event = "spawn_set.switched",
        door,
        decision_outcome = switch.outcome(),
        entity_id,
        entity_name = who.player_name,
        account_id = who.account_id,
        account_name = who.account_name,
        player_id = who.player_id,
        player_name = who.player_name,
        set_id,
        set_name = set_name.as_deref(),
        line = %switch.line(),
        "spawn set switched"
    );
}
