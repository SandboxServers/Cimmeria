/** TypeScript types for the Data Editor (Phase 4) - game data CRUD. */

// Entity Templates
export interface EntityTemplateSummary {
  template_id: number;
  name: string;
  template_name: string | null;
  class: string | null;
  level: number | null;
}

export interface EntityTemplate {
  template_id: number;
  name: string | null;
  template_name: string | null;
  class: string | null;
  level: number | null;
  alignment: number | null;
  faction: number | null;
  body_set: string | null;
  static_mesh: string | null;
  components: string[];
  flags: number | null;
  interaction_type: number | null;
  event_set_id: number | null;
  buy_item_list: number | null;
  sell_item_list: number | null;
  repair_item_list: number | null;
  recharge_item_list: number | null;
  ability_set_id: number | null;
  ammo_type: number | null;
  loot_table_id: number | null;
  primary_color_id: number | null;
  secondary_color_id: number | null;
  skin_tint: number | null;
  weapon_item_id: number | null;
  trainer_ability_list_id: number | null;
  speaker_id: number | null;
  has_dynamic_properties: boolean;
  interaction_set_id: number | null;
  name_id: number | null;
  patrol_path_id: number | null;
  patrol_point_delay: number | null;
  static_interaction_sets: number[];
}

// Items
export interface ItemSummary {
  item_id: number;
  name: string;
  quality_id: number | null;
  tier: number | null;
}

export interface Item {
  item_id: number;
  name: string | null;
  description: string | null;
  quality_id: number | null;
  tier: number | null;
  icon_location: string | null;
  max_stack_size: number | null;
  container_sets: number[];
  flags: number | null;
  applied_science_id: number | null;
  tech_comp: number | null;
  moniker_ids: number[];
  max_ranged_range: number | null;
  min_ranged_range: number | null;
  max_melee_range: number | null;
  min_melee_range: number | null;
  visual_component: string | null;
  discipline_ids: number[];
  ammo_types: number[];
  default_ammo_type: number | null;
  clip_size: number | null;
  charges: number | null;
}

// Missions (hierarchical)
export interface MissionSummary {
  mission_id: number;
  mission_label: string | null;
  level: number | null;
  difficulty: number | null;
  is_enabled: boolean;
}

export interface Mission {
  mission_id: number;
  mission_label: string | null;
  mission_defn: string | null;
  history_text: string | null;
  level: number | null;
  difficulty: number | null;
  is_enabled: boolean;
  is_a_story: boolean;
  is_hidden: boolean;
  is_shareable: boolean;
  can_abandon: boolean;
  can_fail: boolean;
  can_repeat_on_fail: boolean;
  is_override_mission: boolean;
  num_repeats: number | null;
  award_xp: number | null;
  reward_xp: number | null;
  reward_naq: number | null;
  script_name: string | null;
  script_spaces: string | null;
  show_faction_change_icon: boolean;
  show_instance_icon: boolean;
  show_pvp_icon: boolean;
  steps: MissionStep[];
  reward_groups: MissionRewardGroup[];
}

export interface MissionStep {
  step_id: number;
  mission_id: number;
  index: number;
  award_xp: number | null;
  difficulty: number | null;
  step_enabled: boolean;
  step_display_log_text: string | null;
  objectives: MissionObjective[];
}

export interface MissionObjective {
  objective_id: number;
  step_id: number;
  award_xp: number | null;
  difficulty: number | null;
  is_enabled: boolean;
  is_hidden: boolean;
  is_optional: boolean;
  display_log_text: string | null;
  tasks: MissionTask[];
}

export interface MissionTask {
  task_id: number;
  objective_id: number;
  award_xp: number | null;
  difficulty: number | null;
  is_enabled: boolean;
  task_type: string | null;
}

export interface MissionRewardGroup {
  group_id: number;
  mission_id: number;
  choices: number;
  rewards: MissionReward[];
}

export interface MissionReward {
  reward_id: number;
  group_id: number;
  item_id: number;
}

// Loot Tables
export interface LootTableSummary {
  loot_table_id: number;
  description: string | null;
  entry_count: number;
}

export interface LootTable {
  loot_table_id: number;
  description: string | null;
  entries: LootEntry[];
}

export interface LootEntry {
  loot_id: number;
  loot_table_id: number;
  design_id: number;
  min_quantity: number;
  max_quantity: number;
  probability: number;
}

// Abilities
export interface AbilitySummary {
  ability_id: number;
  name: string | null;
  type_id: number | null;
  cooldown: number | null;
}

export interface Ability {
  ability_id: number;
  name: string | null;
  description: string | null;
  type_id: number | null;
  cooldown: number | null;
  flags: number | null;
  icon: string | null;
  is_ranged: boolean;
  min_range: number | null;
  max_range: number | null;
  passive_yn: boolean;
  target_type_id: number | null;
  target_collection_method: number | null;
  warmup: number | null;
  velocity: number | null;
  threat_level_id: number | null;
  training_cost: number | null;
  taunt_adjustment: number | null;
  effect_ids: number[];
  moniker_ids: number[];
  event_set_id: number | null;
  required_ammo: number | null;
  effects: Effect[];
}

export interface Effect {
  effect_id: number;
  ability_id: number | null;
  name: string | null;
  effect_desc: string | null;
  delay: number | null;
  effect_sequence: number | null;
  flags: number | null;
  icon_location: string | null;
  pulse_count: number | null;
  pulse_duration: number | null;
  is_channeled: boolean;
  script_name: string | null;
  target_collection_method: number | null;
  target_collection_id: number | null;
  event_set_id: number | null;
  nvps: EffectNvp[];
}

export interface EffectNvp {
  nvp_id: number;
  effect_id: number;
  name: string;
  value: string;
}

// Data category type
export type DataCategory = 'entities' | 'items' | 'missions' | 'loot' | 'abilities';
