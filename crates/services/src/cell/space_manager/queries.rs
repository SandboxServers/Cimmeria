//! Read-only accessors over the space/entity tables.
//!
//! Includes [`SpaceManager::find_online_player_by_name`], the CellApp-wide
//! exact-match name resolver that the GM travel commands (`.goto`,
//! `.summon`) resolve their target player through.

use cimmeria_common::EntityId;
use cimmeria_entity::cell_entity::CellEntity;

use super::{RegionData, SpaceManager};

/// Outcome of [`SpaceManager::find_online_player_by_name`].
///
/// The three non-success shapes are deliberately distinct variants rather
/// than a flat `Option`, because legacy produced two *different* GM error
/// strings for the two failure modes — `"Player is not available on this
/// CellApp"` (the name isn't in the roster at all) versus `"Player is not
/// on any reachable space"` (the name resolved, but the player isn't bound
/// to a space) — see `deprecated/python/cell/commands/Player.py:298-341`.
/// Collapsing them would force the command adapters to invent a single
/// message and lose that distinction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlayerNameLookup {
    /// Exactly one online player carries this name and is currently bound
    /// to a loaded space. `space_id` is that player's *actual* instance —
    /// callers performing a transfer must use this id rather than
    /// re-resolving the world name, or they'll join the wrong instance of
    /// an instanced world.
    Found { entity_id: u32, space_id: u32 },
    /// Exactly one entity carries this name, but it is not registered in
    /// its space's player set — the cell-side analogue of legacy's
    /// `destination.space is None`. See the reachability note on
    /// [`SpaceManager::find_online_player_by_name`].
    InTransition { entity_id: u32 },
    /// No entity on this CellApp carries this name (case-sensitive).
    NotFound,
    /// More than one entity carries this name. Character names are unique
    /// in this system, so this is an invariant violation, not a normal
    /// path — the lookup refuses to pick one rather than resolving
    /// nondeterministically (the scan walks `HashMap`s, so "the first
    /// match" is not stable across runs). Ids are sorted ascending.
    Ambiguous { entity_ids: Vec<u32> },
}

impl SpaceManager {
    /// Return all active spaces as (space_id, world_name) pairs.
    pub fn all_spaces(&self) -> Vec<(u32, String)> {
        self.spaces
            .values()
            .map(|s| (s.space_id, s.world_name.clone()))
            .collect()
    }

    /// Number of loaded world definitions.
    pub fn world_count(&self) -> usize {
        self.worlds.len()
    }

    /// Number of active space instances.
    pub fn space_count(&self) -> usize {
        self.spaces.len()
    }

    /// Look up the space_id for a world name.
    pub fn space_id_for_world(&self, world_name: &str) -> Option<u32> {
        self.world_spaces.get(world_name).copied()
    }

    /// Is `world_name` a world this CellApp knows about at all?
    ///
    /// Backed by `spaces.xml` (the static world table), so this answers
    /// "does this world exist" independently of whether any instance of it
    /// is currently loaded. Cross-world transfer validates against this
    /// BEFORE tearing an entity out of its origin space.
    pub fn world_is_known(&self, world_name: &str) -> bool {
        self.worlds.contains_key(world_name)
    }

    /// World name of a currently-loaded space instance, or `None` if no such
    /// instance is loaded.
    pub fn world_name_for_space(&self, space_id: u32) -> Option<&str> {
        self.spaces.get(&space_id).map(|s| s.world_name.as_str())
    }

    /// Resolve the "first/default loaded instance" of `world_name` (D15).
    ///
    /// - Non-instanced world: its single startup space.
    /// - Instanced world: the lowest-numbered currently-loaded instance.
    ///   Space ids are allocated monotonically (`allocate_space_id`), so
    ///   "lowest id" == "oldest live instance" and is deterministic —
    ///   `self.spaces` is a `HashMap`, so picking "any" entry would make the
    ///   destination vary run to run.
    /// - `None` when the world has no loaded instance at all (an instanced
    ///   world with nobody in it). Callers treat that as "let the create path
    ///   allocate a fresh instance", not as an error.
    pub fn default_space_for_world(&self, world_name: &str) -> Option<u32> {
        if let Some(&space_id) = self.world_spaces.get(world_name) {
            return Some(space_id);
        }
        self.spaces
            .values()
            .filter(|s| s.world_name == world_name)
            .map(|s| s.space_id)
            .min()
    }

    /// Get a mutable reference to a cell entity by its entity ID.
    ///
    /// Searches across all spaces using the entity→space index.
    pub fn get_entity_mut(&mut self, entity_id: u32) -> Option<&mut CellEntity> {
        let &space_id = self.entity_space.get(&entity_id)?;
        let space = self.spaces.get_mut(&space_id)?;
        space.entities.get_mut(&entity_id)
    }

    /// Get an immutable reference to a cell entity by its entity ID.
    pub fn get_entity(&self, entity_id: u32) -> Option<&CellEntity> {
        let &space_id = self.entity_space.get(&entity_id)?;
        let space = self.spaces.get(&space_id)?;
        space.entities.get(&entity_id)
    }

    /// Get the world name for an entity's current space.
    pub fn get_entity_world_name(&self, entity_id: u32) -> Option<String> {
        let &space_id = self.entity_space.get(&entity_id)?;
        let space = self.spaces.get(&space_id)?;
        Some(space.world_name.clone())
    }

    /// Get the `space_id` for an entity's current space.
    ///
    /// Use this when constructing a `CellToBaseMsg` (or other space-keyed
    /// payload) for an entity outside the normal AoI tick path.
    pub fn get_entity_space_id(&self, entity_id: u32) -> Option<u32> {
        self.entity_space.get(&entity_id).copied()
    }

    /// Get the objectives for a given step from the step_objectives cache.
    ///
    /// Returns an empty vec if the step has no objectives in the cache.
    pub fn get_step_objectives(
        &self,
        step_id: i32,
    ) -> Vec<super::super::spawner::MissionObjectiveDef> {
        self.step_objectives
            .get(&step_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Look up a registered region by its runtime ID.
    pub fn get_region(&self, runtime_id: u32) -> Option<&RegionData> {
        self.regions.get(&runtime_id)
    }

    /// Return all registered regions for a given world name.
    pub fn regions_for_world(&self, world_name: &str) -> Vec<&RegionData> {
        self.regions
            .values()
            .filter(|r| r.world_name == world_name)
            .collect()
    }

    /// Collect all NPC entity IDs (class_id=0x04, not players) across all spaces.
    pub fn all_npc_entity_ids(&self) -> Vec<u32> {
        let mut ids = Vec::new();
        for space in self.spaces.values() {
            for entity in space.entities.values() {
                if !entity.is_player && entity.class_id == 0x04 {
                    ids.push(entity.entity_id.0 as u32);
                }
            }
        }
        ids
    }

    /// Collect all player entity IDs (entries in each space's `players` set)
    /// across all spaces. Returned as a `Vec` so callers can iterate without
    /// holding a borrow on `SpaceManager`.
    pub fn all_player_entity_ids(&self) -> Vec<u32> {
        let mut ids = Vec::new();
        for space in self.spaces.values() {
            ids.extend(space.players.iter().copied());
        }
        ids
    }

    /// Resolve an **exact, case-sensitive** online player name to its entity
    /// id, across every space loaded on this CellApp.
    ///
    /// Port of legacy's `PlayersByName` dict lookup
    /// (`deprecated/python/cell/Global.py:5`, populated in
    /// `cell/SGWPlayer.py:264` and torn down in `:268`), which `.goto` /
    /// `.summon` probe with a raw Python `name in PlayersByName` — a plain
    /// dict-key hit, so case-sensitive with no fuzzy, partial, or
    /// ambiguity handling. This matches D05's "online players across loaded
    /// spaces on this service" scope: no offline lookup, no cluster routing.
    ///
    /// Matching is keyed on [`CellEntity::character_name`], which is
    /// player-exclusive (NPCs use `npc_name`) and is cached from the base's
    /// `ConnectedClientState.player_name` by `BaseToCellMsg::InitPlayerState`.
    /// We deliberately don't *also* gate on `is_player`: presence in the
    /// space's `players` set is the authoritative "is this player reachable"
    /// signal, and it's what separates [`PlayerNameLookup::Found`] from
    /// [`PlayerNameLookup::InTransition`].
    ///
    /// **Reachability of `InTransition` today.** The data model produces the
    /// state — `disconnect_entity` removes the id from `space.players`
    /// before `destroy_entity` drops the named entity — but the cell loop is
    /// a single task that holds `&mut SpaceManager` across that teardown, so
    /// no other handler currently observes the window. The other real-world
    /// case, a player mid-gate-travel, does *not* land here: the cell
    /// destroys the old entity and `create_entity` builds a fresh one with
    /// `character_name: None`, so the name is absent until `InitPlayerState`
    /// re-caches it, and this lookup reports `NotFound` where legacy (whose
    /// dict key survives until `disconnected()`) would have reported the
    /// not-on-a-reachable-space error. Closing that gap needs a name roster
    /// that outlives the entity, which is out of scope here — the variant is
    /// the splice point for it.
    pub fn find_online_player_by_name(&self, name: &str) -> PlayerNameLookup {
        // Collect every match before deciding, so a duplicated name is
        // reported rather than silently resolved to whichever space the
        // HashMap iterator happened to visit first.
        let mut matches: Vec<(u32, u32, bool)> = Vec::new();
        for space in self.spaces.values() {
            for (&entity_id, entity) in &space.entities {
                if entity.character_name.as_deref() == Some(name) {
                    matches.push((
                        entity_id,
                        space.space_id,
                        space.players.contains(&entity_id),
                    ));
                }
            }
        }

        match matches.len() {
            0 => PlayerNameLookup::NotFound,
            1 => {
                let (entity_id, space_id, in_space) = matches[0];
                if in_space {
                    PlayerNameLookup::Found {
                        entity_id,
                        space_id,
                    }
                } else {
                    PlayerNameLookup::InTransition { entity_id }
                }
            }
            _ => {
                let mut entity_ids: Vec<u32> = matches.iter().map(|&(eid, _, _)| eid).collect();
                entity_ids.sort_unstable();
                tracing::error!(
                    target: "player.name_lookup",
                    player_name = name,
                    ?entity_ids,
                    match_count = entity_ids.len(),
                    "player.name_lookup_ambiguous: character-name uniqueness invariant \
                     violated -- refusing to resolve; the caller will report a failure \
                     instead of picking an arbitrary entity"
                );
                PlayerNameLookup::Ambiguous { entity_ids }
            }
        }
    }

    /// Collect every entity id across all spaces, regardless of
    /// class_id or is_player. Used by passes that must visit every
    /// entity (e.g. active-effect pulsing — DoTs apply to NPCs,
    /// players, and any future entity type like turrets or destructibles).
    pub fn all_entity_ids(&self) -> Vec<u32> {
        let mut ids = Vec::new();
        for space in self.spaces.values() {
            for entity in space.entities.values() {
                ids.push(entity.entity_id.0 as u32);
            }
        }
        ids
    }

    /// Find an NPC entity by its spawn tag within the same space as `source_entity_id`.
    ///
    /// Used by content chain actions (SetInteractionType, DestroyTaggedEntity, etc.)
    /// to locate entities by their `spawnlist.tag` value. Restricting the search
    /// to the source's space prevents instanced worlds from leaking entity
    /// resolution across instance boundaries.
    pub fn find_entity_by_tag(&self, source_entity_id: u32, tag: &str) -> Option<u32> {
        let &space_id = self.entity_space.get(&source_entity_id)?;
        let space = self.spaces.get(&space_id)?;
        space
            .entities
            .iter()
            .find_map(|(&eid, entity)| (entity.tag.as_deref() == Some(tag)).then_some(eid))
    }

    /// Find all entities with a given `template_id` in the same space as
    /// `source_entity_id`.
    ///
    /// Used by `add_dialog_set` to locate NPC entities that match the slot
    /// (template_id) so per-player InteractionType updates can be sent.
    /// Restricted to a single space so instanced worlds don't cross-pollinate.
    pub fn find_entities_by_template(&self, source_entity_id: u32, template_id: i32) -> Vec<u32> {
        let Some(&space_id) = self.entity_space.get(&source_entity_id) else {
            return Vec::new();
        };
        let Some(space) = self.spaces.get(&space_id) else {
            return Vec::new();
        };
        space
            .entities
            .iter()
            .filter(|(_, e)| e.template_id == Some(template_id))
            .map(|(&eid, _)| eid)
            .collect()
    }

    /// Return all player entity IDs that currently have `target_entity_id` in their AoI.
    ///
    /// Used for broadcasting property updates (InteractionType, SetVisible, etc.)
    /// to players who can see the entity.
    ///
    /// In this codebase `entity.witnesses` is populated only for players (see
    /// [`super::aoi::SpaceManager::compute_aoi_changes`]), and stores the set
    /// of entities the player currently sees. The reverse mapping (observers
    /// of a target) isn't materialized, so we have to scan player witness
    /// sets. Restricted to the target's own space, so the scan is bounded by
    /// players in that space rather than the whole world.
    pub fn get_witnesses_of(&self, target_entity_id: u32) -> Vec<u32> {
        let Some(&space_id) = self.entity_space.get(&target_entity_id) else {
            return vec![];
        };
        let Some(space) = self.spaces.get(&space_id) else {
            return vec![];
        };
        let target_eid = EntityId(target_entity_id as i32);
        space
            .players
            .iter()
            .filter(|&&pid| {
                space
                    .entities
                    .get(&pid)
                    .is_some_and(|p| p.witnesses.contains(&target_eid))
            })
            .copied()
            .collect()
    }
}
