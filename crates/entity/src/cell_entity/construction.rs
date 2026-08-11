//! [`CellEntity`] construction and its `Debug` projection.
//!
//! The constructor sets every field to its spawn-time default; template
//! and player-load paths overwrite the relevant subset afterward. The
//! `Debug` impl is a hand-rolled projection (the full struct is far too
//! large to derive) that surfaces the fields useful in logs and panics.

use std::collections::{HashMap, HashSet, VecDeque};

use cimmeria_common::{EntityId, SpaceId, Vector3};

use crate::abilities::AbilityManager;
use crate::missions::MissionManager;
use crate::stats::StatList;

use super::{AiState, CellEntity, SystemOptions};

impl CellEntity {
    /// Create a new cell entity at the given position in the given space.
    pub fn new(entity_id: EntityId, space_id: SpaceId, position: Vector3) -> Self {
        Self {
            entity_id,
            space_id,
            position,
            direction: Vector3::zero(),
            velocity: [0.0; 3],
            is_on_ground: true,
            properties: HashMap::new(),
            witnesses: HashSet::new(),
            aoi_radius: 100.0, // Default AoI radius (matches grid_vision_distance)
            is_player: false,
            use_cover: false,
            class_id: 0x02, // SGWPlayer by default
            stats: StatList::new(),
            abilities: AbilityManager::new(),
            interaction_type: None,
            npc_name: None,
            character_name: None,
            ignore_names: HashSet::new(),
            missions: MissionManager::new(),
            player_id: None,
            archetype_id: None,
            access_level: 0,
            level: 1,
            template_id: None,
            spawn_id: None,
            tag: None,
            name_id: None,
            speaker_id: None,
            event_set_id: None,
            interaction_type_flags: 0,
            entity_flags: 0,
            faction: 0,
            alignment: 0,
            static_interaction_sets: Vec::new(),
            has_dynamic_properties: false,
            available_interactions: HashMap::new(),
            static_mesh: None,
            body_set: None,
            components: Vec::new(),
            weapon_visual: None,
            weapon_holstered: true,
            state_field: 0,
            state_flag_counts: HashMap::new(),
            threatened_mobs: HashSet::new(),
            combat_exit_at: None,
            reload_complete_at: None,
            reload_slot_id: None,
            pending_reload_at: None,
            pending_attack_at: None,
            pending_attack_ability_id: None,
            pending_attack_target_id: None,
            pending_slot_swap_at: None,
            pending_slot_swap_target: None,
            last_aoe_deaths: Vec::new(),
            active_effects: Vec::new(),
            holster_animation_complete_at: None,
            ai_state: AiState::Idle,
            threat_list: HashMap::new(),
            spawn_position: None,
            spawn_direction: None,
            ai_cooldown_ticks: 0,
            ai_retry_at: None,
            nav_path: VecDeque::new(),
            move_speed: 0.6, // ~0.6 world units per 100ms tick = 6 units/sec
            is_stationary: false,
            aggression: 0,
            last_movement_type: None,
            respawn_secs: None,
            respawn_at: None,
            original_interaction_type_flags: 0,
            patrol_path: Vec::new(),
            patrol_next_index: 0,
            patrol_dwell_until: None,
            patrol_point_delay_secs: 2.0,
            wander_radius: 0.0,
            wander_min_dwell_secs: 3.0,
            wander_max_dwell_secs: 8.0,
            wander_next_at: None,
            poi: None,
            investigate_until: None,
            follow_target_id: None,
            follow_min_distance: 2.0,
            follow_max_distance: 5.0,
            saved_missions_loaded: false,
            loot_table_id: None,
            loot: Vec::new(),
            next_loot_index: 1,
            looting_entity: None,
            last_interaction_target: None,
            open_dialog_id: None,
            vendor_entity: None,
            trade_partner_entity_id: None,
            trade_proposal: None,
            current_target_id: None,
            active_bandolier_slot: 0,
            bandolier_items: HashMap::new(),
            bandolier_ammo_dirty: HashSet::new(),
            ring_source_id: None,
            destination_ring_id: None,
            counters: HashMap::new(),
            system_options: SystemOptions::default(),
        }
    }
}

impl std::fmt::Debug for CellEntity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CellEntity")
            .field("entity_id", &self.entity_id)
            .field("space_id", &self.space_id)
            .field("position", &self.position)
            .field("is_on_ground", &self.is_on_ground)
            .field("witness_count", &self.witnesses.len())
            .field("aoi_radius", &self.aoi_radius)
            .field("property_count", &self.properties.len())
            .field("stats", &self.stats)
            .field("known_abilities", &self.abilities.known_count())
            .field("template_id", &self.template_id)
            .field("tag", &self.tag)
            .finish()
    }
}
