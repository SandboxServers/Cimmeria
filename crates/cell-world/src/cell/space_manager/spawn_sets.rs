//! Switchable spawn sets (Debug Area DA-10, the Visual NPC Lineup).
//!
//! A spawn set is a group of `spawnlist` rows (`spawnlist.set_name` ==
//! `spawn_sets.name`, same world). Members never spawn at startup:
//! [`partition_spawn_sets`] takes them out of the startup records and keeps
//! them here, and a GM switches a whole set on and off at run time
//! (`activateSpawnSet` / `deactivateSpawnSet`, `.spawnset`, the lineup
//! attendants). Sets of one kind in one world are exclusive: switching one on
//! switches the others of its kind off, so the 161-actor lineup is never
//! loaded all at once (its full load ran the 32-bit client out of memory).
//!
//! This module only keeps the state. Spawning goes through
//! [`SpaceManager::spawn_npc_from_record`]; despawning, which must release
//! combat and send every witness `LeftAoI`, is the caller's
//! (`cimmeria-cell-content`'s `spawn_sets`).

use std::collections::{BTreeMap, HashMap};

use super::super::spawner::{SpawnRecord, SpawnSetDef};
use super::SpaceManager;

/// One switchable spawn set: its definition, its members' records, and the
/// entities that stand for them while it is on.
#[derive(Debug, Clone)]
pub struct SpawnSet {
    /// `spawn_sets.set_id`.
    pub set_id: i32,
    /// `spawn_sets.name`: the label chat lines show.
    pub name: String,
    /// `spawn_sets.type`: sets of one kind in one world are exclusive.
    pub kind: String,
    /// `spawn_sets.world_id`.
    pub world_id: i32,
    /// The members' world, from their records (`None` for an empty set).
    pub world_name: Option<String>,
    /// The members' spawn records, in spawn-id order.
    pub records: Vec<SpawnRecord>,
    /// The members' entity ids while the set is on; empty while it is off.
    pub live: Vec<u32>,
}

impl SpawnSet {
    /// Whether the set is on.
    pub fn is_active(&self) -> bool {
        !self.live.is_empty()
    }
}

/// Every switchable spawn set, by `set_id`.
#[derive(Debug, Default, Clone)]
pub struct SpawnSetCatalog {
    sets: BTreeMap<i32, SpawnSet>,
}

impl SpawnSetCatalog {
    /// The set `set_id`.
    pub fn get(&self, set_id: i32) -> Option<&SpawnSet> {
        self.sets.get(&set_id)
    }

    /// Every set, by id.
    pub fn iter(&self) -> impl Iterator<Item = &SpawnSet> {
        self.sets.values()
    }

    /// The sets of `kind` in world `world_id`, by id.
    pub fn of_kind(&self, kind: &str, world_id: i32) -> Vec<&SpawnSet> {
        self.sets
            .values()
            .filter(|s| s.kind == kind && s.world_id == world_id)
            .collect()
    }

    /// The other sets of `set_id`'s kind and world that are on: switching
    /// `set_id` on switches these off.
    pub fn active_peers(&self, set_id: i32) -> Vec<i32> {
        let Some(set) = self.sets.get(&set_id) else {
            return vec![];
        };
        self.sets
            .values()
            .filter(|s| {
                s.set_id != set_id
                    && s.kind == set.kind
                    && s.world_id == set.world_id
                    && s.is_active()
            })
            .map(|s| s.set_id)
            .collect()
    }

    /// Add a set (tests and [`partition_spawn_sets`]).
    pub fn insert(&mut self, set: SpawnSet) {
        self.sets.insert(set.set_id, set);
    }

    /// How many sets there are.
    pub fn len(&self) -> usize {
        self.sets.len()
    }

    /// Whether there is no set.
    pub fn is_empty(&self) -> bool {
        self.sets.is_empty()
    }
}

/// Take every spawn-set member out of `records` (the startup spawn list) and
/// build the catalog from them. Members of a set whose rows were not loaded
/// (a missing template, say) are simply absent from it.
pub fn partition_spawn_sets(
    records: &mut Vec<SpawnRecord>,
    defs: Vec<SpawnSetDef>,
) -> SpawnSetCatalog {
    let mut owner: HashMap<i32, i32> = HashMap::new();
    for def in &defs {
        for &spawn_id in &def.spawn_ids {
            owner.insert(spawn_id, def.set_id);
        }
    }
    let mut by_set: HashMap<i32, Vec<SpawnRecord>> = HashMap::new();
    records.retain(|r| match owner.get(&r.spawn_id) {
        Some(&set_id) => {
            by_set.entry(set_id).or_default().push(r.clone());
            false
        }
        None => true,
    });
    let mut catalog = SpawnSetCatalog::default();
    for def in defs {
        let mut members = by_set.remove(&def.set_id).unwrap_or_default();
        members.sort_by_key(|r| r.spawn_id);
        catalog.insert(SpawnSet {
            set_id: def.set_id,
            world_name: members.first().map(|r| r.world_name.clone()),
            name: def.name,
            kind: def.kind,
            world_id: def.world_id,
            records: members,
            live: vec![],
        });
    }
    catalog
}

/// Why a set could not be switched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpawnSetError {
    /// No set has this id.
    Unknown,
    /// The set is already in the requested state.
    AlreadyActive,
    /// The set is already off.
    AlreadyInactive,
    /// The set's world has no running space (or the set has no member).
    NoSpace,
}

impl SpaceManager {
    /// Spawn every member of `set_id` into its world's shared space and mark
    /// the set on. Returns the new entity ids. Does not touch other sets:
    /// the caller switches the set's active peers off first.
    pub fn spawn_set_members(&mut self, set_id: i32) -> Result<Vec<u32>, SpawnSetError> {
        let set = self.spawn_sets.get(set_id).ok_or(SpawnSetError::Unknown)?;
        if set.is_active() {
            return Err(SpawnSetError::AlreadyActive);
        }
        let Some(world) = set.world_name.clone() else {
            return Err(SpawnSetError::NoSpace);
        };
        if !self.has_space_for_world(&world) {
            return Err(SpawnSetError::NoSpace);
        }
        let records = set.records.clone();
        let set_name = set.name.clone();
        let mut live = Vec::with_capacity(records.len());
        for record in &records {
            let npc_id = self.allocate_npc_id();
            match self.spawn_npc_from_record(npc_id, record) {
                Ok(_) => live.push(npc_id),
                Err(e) => tracing::warn!(
                    target: "spawner",
                    event = "spawn_set_member_failed",
                    set_id,
                    set_name = %set_name,
                    spawn_id = record.spawn_id, // nt:id-only spawnlist row id, the row has no name column
                    template_id = record.template_id,
                    template_name = %record.template_name,
                    world = %record.world_name,
                    "a spawn-set member failed to spawn: {e}"
                ),
            }
        }
        if let Some(set) = self.spawn_sets.sets.get_mut(&set_id) {
            set.live.clone_from(&live);
        }
        Ok(live)
    }

    /// Mark `set_id` off and hand back the entity ids that stood for its
    /// members, for the caller to despawn.
    pub fn take_spawn_set_members(&mut self, set_id: i32) -> Result<Vec<u32>, SpawnSetError> {
        let set = self
            .spawn_sets
            .sets
            .get_mut(&set_id)
            .ok_or(SpawnSetError::Unknown)?;
        if !set.is_active() {
            return Err(SpawnSetError::AlreadyInactive);
        }
        Ok(std::mem::take(&mut set.live))
    }
}
