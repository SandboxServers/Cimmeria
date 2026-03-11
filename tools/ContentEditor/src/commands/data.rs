use crate::state::AppState;
use serde::{Deserialize, Serialize};
use sqlx::Row;

// ---------------------------------------------------------------------------
// Entity Templates
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct EntityTemplateSummary {
    pub template_id: i32,
    pub name: Option<String>,
    pub template_name: String,
    pub class: String,
    pub level: Option<i32>,
}

#[derive(Serialize, Deserialize)]
pub struct EntityTemplate {
    pub template_id: Option<i32>,
    pub name: Option<String>,
    pub template_name: String,
    pub class: String,
    pub level: Option<i32>,
    pub alignment: Option<i32>,
    pub faction: Option<i32>,
    pub body_set: String,
    pub static_mesh: Option<String>,
    pub components: Vec<String>,
    pub flags: i64,
    pub interaction_type: i64,
    pub event_set_id: Option<i32>,
    pub buy_item_list: Option<i32>,
    pub sell_item_list: Option<i32>,
    pub repair_item_list: Option<i32>,
    pub recharge_item_list: Option<i32>,
    pub ability_set_id: Option<i32>,
    pub ammo_type: Option<String>,
    pub loot_table_id: Option<i32>,
    pub primary_color_id: i64,
    pub secondary_color_id: i64,
    pub skin_tint: i64,
    pub weapon_item_id: Option<i32>,
    pub static_interaction_sets: Vec<i32>,
    pub trainer_ability_list_id: Option<i32>,
    pub speaker_id: Option<i32>,
    pub has_dynamic_properties: bool,
    pub interaction_set_id: Option<i32>,
    pub name_id: Option<i32>,
    pub patrol_path_id: Option<i32>,
    pub patrol_point_delay: Option<f32>,
}

#[tauri::command]
pub async fn list_entity_templates(
    state: tauri::State<'_, AppState>,
    search: Option<String>,
) -> Result<Vec<EntityTemplateSummary>, String> {
    tracing::debug!("list_entity_templates called, search={:?}", search);
    let pool = state.pool()?;

    let rows = sqlx::query(
        r#"
        SELECT template_id, name, template_name, class, level
        FROM entity_templates
        WHERE ($1::text IS NULL
               OR name ILIKE '%' || $1 || '%'
               OR template_name ILIKE '%' || $1 || '%')
        ORDER BY template_id
        "#,
    )
    .bind(&search)
    .fetch_all(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to list entity templates: {e}");
        format!("Failed to list entity templates: {e}")
    })?;

    tracing::debug!("list_entity_templates returned {} rows", rows.len());
    Ok(rows
        .iter()
        .map(|r| EntityTemplateSummary {
            template_id: r.get("template_id"),
            name: r.get("name"),
            template_name: r.get("template_name"),
            class: r.get("class"),
            level: r.get("level"),
        })
        .collect())
}

#[tauri::command]
pub async fn get_entity_template(
    state: tauri::State<'_, AppState>,
    template_id: i32,
) -> Result<EntityTemplate, String> {
    tracing::debug!("get_entity_template called, template_id={template_id}");
    let pool = state.pool()?;

    let r = sqlx::query(
        r#"
        SELECT template_id, name, template_name, class, level, alignment, faction,
               body_set, static_mesh,
               COALESCE(components, '{}') AS components,
               flags, interaction_type,
               event_set_id, buy_item_list, sell_item_list, repair_item_list,
               recharge_item_list, ability_set_id, ammo_type::text, loot_table_id,
               primary_color_id, secondary_color_id, skin_tint,
               weapon_item_id,
               COALESCE(static_interaction_sets, '{}') AS static_interaction_sets,
               trainer_ability_list_id,
               speaker_id, has_dynamic_properties, interaction_set_id,
               name_id, patrol_path_id, patrol_point_delay
        FROM entity_templates
        WHERE template_id = $1
        "#,
    )
    .bind(template_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("Failed to get entity template: {e}"))?
    .ok_or_else(|| format!("Entity template {template_id} not found"))?;

    Ok(EntityTemplate {
        template_id: Some(r.get("template_id")),
        name: r.get("name"),
        template_name: r.get("template_name"),
        class: r.get("class"),
        level: r.get("level"),
        alignment: r.get("alignment"),
        faction: r.get("faction"),
        body_set: r.get("body_set"),
        static_mesh: r.get("static_mesh"),
        components: r.get::<Vec<String>, _>("components"),
        flags: r.get("flags"),
        interaction_type: r.get("interaction_type"),
        event_set_id: r.get("event_set_id"),
        buy_item_list: r.get("buy_item_list"),
        sell_item_list: r.get("sell_item_list"),
        repair_item_list: r.get("repair_item_list"),
        recharge_item_list: r.get("recharge_item_list"),
        ability_set_id: r.get("ability_set_id"),
        ammo_type: r.get("ammo_type"),
        loot_table_id: r.get("loot_table_id"),
        primary_color_id: r.get("primary_color_id"),
        secondary_color_id: r.get("secondary_color_id"),
        skin_tint: r.get("skin_tint"),
        weapon_item_id: r.get("weapon_item_id"),
        static_interaction_sets: r.get::<Vec<i32>, _>("static_interaction_sets"),
        trainer_ability_list_id: r.get("trainer_ability_list_id"),
        speaker_id: r.get("speaker_id"),
        has_dynamic_properties: r.get("has_dynamic_properties"),
        interaction_set_id: r.get("interaction_set_id"),
        name_id: r.get("name_id"),
        patrol_path_id: r.get("patrol_path_id"),
        patrol_point_delay: r.get("patrol_point_delay"),
    })
}

#[tauri::command]
pub async fn save_entity_template(
    state: tauri::State<'_, AppState>,
    template: EntityTemplate,
) -> Result<i32, String> {
    let pool = state.pool()?;

    // Upsert: ON CONFLICT on template_id
    let id: i32 = sqlx::query_scalar(
        r#"
        INSERT INTO entity_templates (
            template_id, name, template_name, class, level, alignment, faction,
            body_set, static_mesh, components, flags, interaction_type,
            event_set_id, buy_item_list, sell_item_list, repair_item_list,
            recharge_item_list, ability_set_id, ammo_type, loot_table_id,
            primary_color_id, secondary_color_id, skin_tint,
            weapon_item_id, static_interaction_sets, trainer_ability_list_id,
            speaker_id, has_dynamic_properties, interaction_set_id,
            name_id, patrol_path_id, patrol_point_delay
        ) VALUES (
            COALESCE($1, nextval('entity_templates_template_id_seq'::regclass)),
            $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15,
            $16, $17, $18, $19::text::"EAmmoType", $20, $21, $22, $23, $24, $25,
            $26, $27, $28, $29, $30, $31, $32
        )
        ON CONFLICT (template_id) DO UPDATE SET
            name = EXCLUDED.name,
            template_name = EXCLUDED.template_name,
            class = EXCLUDED.class,
            level = EXCLUDED.level,
            alignment = EXCLUDED.alignment,
            faction = EXCLUDED.faction,
            body_set = EXCLUDED.body_set,
            static_mesh = EXCLUDED.static_mesh,
            components = EXCLUDED.components,
            flags = EXCLUDED.flags,
            interaction_type = EXCLUDED.interaction_type,
            event_set_id = EXCLUDED.event_set_id,
            buy_item_list = EXCLUDED.buy_item_list,
            sell_item_list = EXCLUDED.sell_item_list,
            repair_item_list = EXCLUDED.repair_item_list,
            recharge_item_list = EXCLUDED.recharge_item_list,
            ability_set_id = EXCLUDED.ability_set_id,
            ammo_type = EXCLUDED.ammo_type,
            loot_table_id = EXCLUDED.loot_table_id,
            primary_color_id = EXCLUDED.primary_color_id,
            secondary_color_id = EXCLUDED.secondary_color_id,
            skin_tint = EXCLUDED.skin_tint,
            weapon_item_id = EXCLUDED.weapon_item_id,
            static_interaction_sets = EXCLUDED.static_interaction_sets,
            trainer_ability_list_id = EXCLUDED.trainer_ability_list_id,
            speaker_id = EXCLUDED.speaker_id,
            has_dynamic_properties = EXCLUDED.has_dynamic_properties,
            interaction_set_id = EXCLUDED.interaction_set_id,
            name_id = EXCLUDED.name_id,
            patrol_path_id = EXCLUDED.patrol_path_id,
            patrol_point_delay = EXCLUDED.patrol_point_delay
        RETURNING template_id
        "#,
    )
    .bind(template.template_id)
    .bind(&template.name)
    .bind(&template.template_name)
    .bind(&template.class)
    .bind(template.level)
    .bind(template.alignment)
    .bind(template.faction)
    .bind(&template.body_set)
    .bind(&template.static_mesh)
    .bind(&template.components)
    .bind(template.flags)
    .bind(template.interaction_type)
    .bind(template.event_set_id)
    .bind(template.buy_item_list)
    .bind(template.sell_item_list)
    .bind(template.repair_item_list)
    .bind(template.recharge_item_list)
    .bind(template.ability_set_id)
    .bind(&template.ammo_type)
    .bind(template.loot_table_id)
    .bind(template.primary_color_id)
    .bind(template.secondary_color_id)
    .bind(template.skin_tint)
    .bind(template.weapon_item_id)
    .bind(&template.static_interaction_sets)
    .bind(template.trainer_ability_list_id)
    .bind(template.speaker_id)
    .bind(template.has_dynamic_properties)
    .bind(template.interaction_set_id)
    .bind(template.name_id)
    .bind(template.patrol_path_id)
    .bind(template.patrol_point_delay)
    .fetch_one(pool)
    .await
    .map_err(|e| format!("Failed to save entity template: {e}"))?;

    tracing::info!("Saved entity template {id}");
    Ok(id)
}

#[tauri::command]
pub async fn delete_entity_template(
    state: tauri::State<'_, AppState>,
    template_id: i32,
) -> Result<(), String> {
    let pool = state.pool()?;

    sqlx::query("DELETE FROM entity_templates WHERE template_id = $1")
        .bind(template_id)
        .execute(pool)
        .await
        .map_err(|e| format!("Failed to delete entity template {template_id}: {e}"))?;

    tracing::info!("Deleted entity template {template_id}");
    Ok(())
}

// ---------------------------------------------------------------------------
// Items
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct ItemSummary {
    pub item_id: i32,
    pub name: String,
    pub quality_id: String,
    pub tier: i32,
}

#[derive(Serialize, Deserialize)]
pub struct Item {
    pub item_id: Option<i32>,
    pub name: String,
    pub description: String,
    pub quality_id: String,
    pub tier: i32,
    pub tech_comp: i32,
    pub icon_location: Option<String>,
    pub max_stack_size: i32,
    pub container_sets: Vec<i32>,
    pub moniker_ids: Vec<i64>,
    pub flags: i32,
    pub visual_component: Option<String>,
    pub applied_science_id: Option<i32>,
    pub max_ranged_range: Option<i32>,
    pub min_ranged_range: Option<i32>,
    pub max_melee_range: Option<i32>,
    pub min_melee_range: Option<i32>,
    pub discipline_ids: Vec<i32>,
    pub ammo_types: Vec<String>,
    pub default_ammo_type: Option<String>,
    pub clip_size: i32,
    pub charges: i32,
}

#[tauri::command]
pub async fn list_items(
    state: tauri::State<'_, AppState>,
    search: Option<String>,
) -> Result<Vec<ItemSummary>, String> {
    tracing::debug!("list_items called, search={:?}", search);
    let pool = state.pool()?;

    let rows = sqlx::query(
        r#"
        SELECT item_id, name, quality_id::text, tier
        FROM items
        WHERE ($1::text IS NULL OR name ILIKE '%' || $1 || '%')
        ORDER BY item_id
        "#,
    )
    .bind(&search)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to list items: {e}"))?;

    Ok(rows
        .iter()
        .map(|r| ItemSummary {
            item_id: r.get("item_id"),
            name: r.get("name"),
            quality_id: r.get("quality_id"),
            tier: r.get("tier"),
        })
        .collect())
}

#[tauri::command]
pub async fn get_item(
    state: tauri::State<'_, AppState>,
    item_id: i32,
) -> Result<Item, String> {
    tracing::debug!("get_item called, item_id={item_id:?}");
    let pool = state.pool()?;

    // Cast enum columns and enum arrays to text so sqlx can decode them as strings.
    // ammo_types is "EAmmoType"[] -- we convert it to text[] via an array cast expression.
    let r = sqlx::query(
        r#"
        SELECT item_id, name, description, quality_id::text, tier, tech_comp,
               icon_location, max_stack_size, container_sets, moniker_ids,
               flags, visual_component, applied_science_id,
               max_ranged_range, min_ranged_range, max_melee_range, min_melee_range,
               discipline_ids,
               (SELECT array_agg(x::text) FROM unnest(ammo_types) AS x) AS ammo_types,
               default_ammo_type::text,
               clip_size, charges
        FROM items
        WHERE item_id = $1
        "#,
    )
    .bind(item_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("Failed to get item: {e}"))?
    .ok_or_else(|| format!("Item {item_id} not found"))?;

    Ok(Item {
        item_id: Some(r.get("item_id")),
        name: r.get("name"),
        description: r.get("description"),
        quality_id: r.get("quality_id"),
        tier: r.get("tier"),
        tech_comp: r.get("tech_comp"),
        icon_location: r.get("icon_location"),
        max_stack_size: r.get("max_stack_size"),
        container_sets: r.get::<Vec<i32>, _>("container_sets"),
        moniker_ids: r.get::<Vec<i64>, _>("moniker_ids"),
        flags: r.get("flags"),
        visual_component: r.get("visual_component"),
        applied_science_id: r.get("applied_science_id"),
        max_ranged_range: r.get("max_ranged_range"),
        min_ranged_range: r.get("min_ranged_range"),
        max_melee_range: r.get("max_melee_range"),
        min_melee_range: r.get("min_melee_range"),
        discipline_ids: r.get::<Vec<i32>, _>("discipline_ids"),
        ammo_types: r.get::<Option<Vec<String>>, _>("ammo_types").unwrap_or_default(),
        default_ammo_type: r.get("default_ammo_type"),
        clip_size: r.get("clip_size"),
        charges: r.get("charges"),
    })
}

#[tauri::command]
pub async fn save_item(
    state: tauri::State<'_, AppState>,
    item: Item,
) -> Result<i32, String> {
    tracing::debug!("save_item called");
    let pool = state.pool()?;

    // ammo_types comes in as Vec<String> -- cast each element back to EAmmoType via SQL.
    // We pass the text array and let Postgres cast it.
    let id: i32 = sqlx::query_scalar(
        r#"
        INSERT INTO items (
            item_id, name, description, quality_id, tier, tech_comp,
            icon_location, max_stack_size, container_sets, moniker_ids,
            flags, visual_component, applied_science_id,
            max_ranged_range, min_ranged_range, max_melee_range, min_melee_range,
            discipline_ids, ammo_types, default_ammo_type,
            clip_size, charges
        ) VALUES (
            COALESCE($1, nextval('items_item_id_seq'::regclass)),
            $2, $3, $4::text::"EItemQuality", $5, $6, $7, $8, $9, $10,
            $11, $12, $13, $14, $15, $16, $17, $18,
            (SELECT array_agg(x::"EAmmoType") FROM unnest($19::text[]) AS x),
            $20::text::"EAmmoType",
            $21, $22
        )
        ON CONFLICT (item_id) DO UPDATE SET
            name = EXCLUDED.name,
            description = EXCLUDED.description,
            quality_id = EXCLUDED.quality_id,
            tier = EXCLUDED.tier,
            tech_comp = EXCLUDED.tech_comp,
            icon_location = EXCLUDED.icon_location,
            max_stack_size = EXCLUDED.max_stack_size,
            container_sets = EXCLUDED.container_sets,
            moniker_ids = EXCLUDED.moniker_ids,
            flags = EXCLUDED.flags,
            visual_component = EXCLUDED.visual_component,
            applied_science_id = EXCLUDED.applied_science_id,
            max_ranged_range = EXCLUDED.max_ranged_range,
            min_ranged_range = EXCLUDED.min_ranged_range,
            max_melee_range = EXCLUDED.max_melee_range,
            min_melee_range = EXCLUDED.min_melee_range,
            discipline_ids = EXCLUDED.discipline_ids,
            ammo_types = EXCLUDED.ammo_types,
            default_ammo_type = EXCLUDED.default_ammo_type,
            clip_size = EXCLUDED.clip_size,
            charges = EXCLUDED.charges
        RETURNING item_id
        "#,
    )
    .bind(item.item_id)
    .bind(&item.name)
    .bind(&item.description)
    .bind(&item.quality_id)
    .bind(item.tier)
    .bind(item.tech_comp)
    .bind(&item.icon_location)
    .bind(item.max_stack_size)
    .bind(&item.container_sets)
    .bind(&item.moniker_ids)
    .bind(item.flags)
    .bind(&item.visual_component)
    .bind(item.applied_science_id)
    .bind(item.max_ranged_range)
    .bind(item.min_ranged_range)
    .bind(item.max_melee_range)
    .bind(item.min_melee_range)
    .bind(&item.discipline_ids)
    .bind(&item.ammo_types)
    .bind(&item.default_ammo_type)
    .bind(item.clip_size)
    .bind(item.charges)
    .fetch_one(pool)
    .await
    .map_err(|e| format!("Failed to save item: {e}"))?;

    tracing::info!("Saved item {id}");
    Ok(id)
}

#[tauri::command]
pub async fn delete_item(
    state: tauri::State<'_, AppState>,
    item_id: i32,
) -> Result<(), String> {
    tracing::debug!("delete_item called, item_id={item_id:?}");
    let pool = state.pool()?;

    sqlx::query("DELETE FROM items WHERE item_id = $1")
        .bind(item_id)
        .execute(pool)
        .await
        .map_err(|e| format!("Failed to delete item {item_id}: {e}"))?;

    tracing::info!("Deleted item {item_id}");
    Ok(())
}

// ---------------------------------------------------------------------------
// Missions (full hierarchy)
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct MissionSummary {
    pub mission_id: i32,
    pub mission_label: String,
    pub level: i32,
    pub difficulty: i32,
    pub is_enabled: bool,
}

#[derive(Serialize, Deserialize)]
pub struct MissionReward {
    pub reward_id: Option<i32>,
    pub item_id: i32,
}

#[derive(Serialize, Deserialize)]
pub struct MissionRewardGroup {
    pub group_id: Option<i32>,
    pub choices: i32,
    pub rewards: Vec<MissionReward>,
}

#[derive(Serialize, Deserialize)]
pub struct MissionTask {
    pub task_id: i32,
    pub award_xp: bool,
    pub difficulty: i32,
    pub is_enabled: bool,
    pub task_type: i32,
}

#[derive(Serialize, Deserialize)]
pub struct MissionObjective {
    pub objective_id: i32,
    pub award_xp: bool,
    pub difficulty: i32,
    pub is_enabled: bool,
    pub is_hidden: bool,
    pub is_optional: bool,
    pub display_log_text: Option<String>,
    pub tasks: Vec<MissionTask>,
}

#[derive(Serialize, Deserialize)]
pub struct MissionStep {
    pub step_id: i32,
    pub award_xp: bool,
    pub difficulty: i32,
    pub step_enabled: bool,
    pub step_display_log_text: String,
    pub index: Option<i32>,
    pub objectives: Vec<MissionObjective>,
}

#[derive(Serialize, Deserialize)]
pub struct Mission {
    pub mission_id: Option<i32>,
    pub mission_label: String,
    pub mission_defn: String,
    pub history_text: String,
    pub level: i32,
    pub difficulty: i32,
    pub is_enabled: bool,
    pub is_hidden: bool,
    pub is_a_story: bool,
    pub is_shareable: bool,
    pub is_override_mission: bool,
    pub award_xp: bool,
    pub can_abandon: bool,
    pub can_fail: bool,
    pub can_repeat_on_fail: bool,
    pub num_repeats: i32,
    pub show_faction_change_icon: bool,
    pub show_instance_icon: bool,
    pub show_pvp_icon: bool,
    pub script_name: Option<String>,
    pub script_spaces: Option<String>,
    pub reward_naq: i32,
    pub reward_xp: i32,
    pub steps: Vec<MissionStep>,
    pub reward_groups: Vec<MissionRewardGroup>,
}

#[tauri::command]
pub async fn list_missions(
    state: tauri::State<'_, AppState>,
    search: Option<String>,
) -> Result<Vec<MissionSummary>, String> {
    tracing::debug!("list_missions called, search={:?}", search);
    let pool = state.pool()?;

    let rows = sqlx::query(
        r#"
        SELECT mission_id, mission_label, level, difficulty, is_enabled
        FROM missions
        WHERE ($1::text IS NULL OR mission_label ILIKE '%' || $1 || '%')
        ORDER BY mission_id
        "#,
    )
    .bind(&search)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to list missions: {e}"))?;

    Ok(rows
        .iter()
        .map(|r| MissionSummary {
            mission_id: r.get("mission_id"),
            mission_label: r.get("mission_label"),
            level: r.get("level"),
            difficulty: r.get("difficulty"),
            is_enabled: r.get("is_enabled"),
        })
        .collect())
}

#[tauri::command]
pub async fn get_mission(
    state: tauri::State<'_, AppState>,
    mission_id: i32,
) -> Result<Mission, String> {
    tracing::debug!("get_mission called, mission_id={mission_id:?}");
    let pool = state.pool()?;

    // Fetch the mission header
    let mr = sqlx::query(
        r#"
        SELECT mission_id, mission_label, mission_defn, history_text,
               level, difficulty, is_enabled, is_hidden, is_a_story,
               is_shareable, is_override_mission, award_xp, can_abandon,
               can_fail, can_repeat_on_fail, num_repeats,
               show_faction_change_icon, show_instance_icon, show_pvp_icon,
               script_name, script_spaces, reward_naq, reward_xp
        FROM missions
        WHERE mission_id = $1
        "#,
    )
    .bind(mission_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("Failed to get mission: {e}"))?
    .ok_or_else(|| format!("Mission {mission_id} not found"))?;

    // Fetch steps
    let step_rows = sqlx::query(
        "SELECT step_id, award_xp, difficulty, step_enabled, step_display_log_text, index \
         FROM mission_steps WHERE mission_id = $1 ORDER BY index, step_id",
    )
    .bind(mission_id)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to load mission steps: {e}"))?;

    let mut steps = Vec::new();
    for sr in &step_rows {
        let step_id: i32 = sr.get("step_id");

        // Fetch objectives for this step
        let obj_rows = sqlx::query(
            "SELECT objective_id, award_xp, difficulty, is_enabled, is_hidden, \
                    is_optional, display_log_text \
             FROM mission_objectives WHERE step_id = $1 ORDER BY objective_id",
        )
        .bind(step_id)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Failed to load mission objectives: {e}"))?;

        let mut objectives = Vec::new();
        for or_ in &obj_rows {
            let objective_id: i32 = or_.get("objective_id");

            // Fetch tasks for this objective
            let task_rows = sqlx::query(
                "SELECT task_id, award_xp, difficulty, is_enabled, task_type \
                 FROM mission_tasks WHERE objective_id = $1 ORDER BY task_id",
            )
            .bind(objective_id)
            .fetch_all(pool)
            .await
            .map_err(|e| format!("Failed to load mission tasks: {e}"))?;

            let tasks = task_rows
                .iter()
                .map(|tr| MissionTask {
                    task_id: tr.get("task_id"),
                    award_xp: tr.get("award_xp"),
                    difficulty: tr.get("difficulty"),
                    is_enabled: tr.get("is_enabled"),
                    task_type: tr.get("task_type"),
                })
                .collect();

            objectives.push(MissionObjective {
                objective_id,
                award_xp: or_.get("award_xp"),
                difficulty: or_.get("difficulty"),
                is_enabled: or_.get("is_enabled"),
                is_hidden: or_.get("is_hidden"),
                is_optional: or_.get("is_optional"),
                display_log_text: or_.get("display_log_text"),
                tasks,
            });
        }

        steps.push(MissionStep {
            step_id,
            award_xp: sr.get("award_xp"),
            difficulty: sr.get("difficulty"),
            step_enabled: sr.get("step_enabled"),
            step_display_log_text: sr.get("step_display_log_text"),
            index: sr.get("index"),
            objectives,
        });
    }

    // Fetch reward groups and their rewards
    let rg_rows = sqlx::query(
        "SELECT group_id, choices \
         FROM mission_reward_groups WHERE mission_id = $1 ORDER BY group_id",
    )
    .bind(mission_id)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to load mission reward groups: {e}"))?;

    let mut reward_groups = Vec::new();
    for rg in &rg_rows {
        let group_id: i32 = rg.get("group_id");

        let reward_rows = sqlx::query(
            "SELECT reward_id, item_id \
             FROM mission_rewards WHERE group_id = $1 ORDER BY reward_id",
        )
        .bind(group_id)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Failed to load mission rewards: {e}"))?;

        let rewards = reward_rows
            .iter()
            .map(|rr| MissionReward {
                reward_id: Some(rr.get("reward_id")),
                item_id: rr.get("item_id"),
            })
            .collect();

        reward_groups.push(MissionRewardGroup {
            group_id: Some(group_id),
            choices: rg.get("choices"),
            rewards,
        });
    }

    Ok(Mission {
        mission_id: Some(mr.get("mission_id")),
        mission_label: mr.get("mission_label"),
        mission_defn: mr.get("mission_defn"),
        history_text: mr.get("history_text"),
        level: mr.get("level"),
        difficulty: mr.get("difficulty"),
        is_enabled: mr.get("is_enabled"),
        is_hidden: mr.get("is_hidden"),
        is_a_story: mr.get("is_a_story"),
        is_shareable: mr.get("is_shareable"),
        is_override_mission: mr.get("is_override_mission"),
        award_xp: mr.get("award_xp"),
        can_abandon: mr.get("can_abandon"),
        can_fail: mr.get("can_fail"),
        can_repeat_on_fail: mr.get("can_repeat_on_fail"),
        num_repeats: mr.get("num_repeats"),
        show_faction_change_icon: mr.get("show_faction_change_icon"),
        show_instance_icon: mr.get("show_instance_icon"),
        show_pvp_icon: mr.get("show_pvp_icon"),
        script_name: mr.get("script_name"),
        script_spaces: mr.get("script_spaces"),
        reward_naq: mr.get("reward_naq"),
        reward_xp: mr.get("reward_xp"),
        steps,
        reward_groups,
    })
}

#[tauri::command]
pub async fn save_mission(
    state: tauri::State<'_, AppState>,
    mission: Mission,
) -> Result<i32, String> {
    tracing::debug!("save_mission called");
    let pool = state.pool()?;
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| format!("Failed to begin transaction: {e}"))?;

    // Upsert the mission header
    let mission_id: i32 = sqlx::query_scalar(
        r#"
        INSERT INTO missions (
            mission_id, mission_label, mission_defn, history_text,
            level, difficulty, is_enabled, is_hidden, is_a_story,
            is_shareable, is_override_mission, award_xp, can_abandon,
            can_fail, can_repeat_on_fail, num_repeats,
            show_faction_change_icon, show_instance_icon, show_pvp_icon,
            script_name, script_spaces, reward_naq, reward_xp
        ) VALUES (
            COALESCE($1, nextval('missions_mission_id_seq'::regclass)),
            $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14,
            $15, $16, $17, $18, $19, $20, $21, $22, $23
        )
        ON CONFLICT (mission_id) DO UPDATE SET
            mission_label = EXCLUDED.mission_label,
            mission_defn = EXCLUDED.mission_defn,
            history_text = EXCLUDED.history_text,
            level = EXCLUDED.level,
            difficulty = EXCLUDED.difficulty,
            is_enabled = EXCLUDED.is_enabled,
            is_hidden = EXCLUDED.is_hidden,
            is_a_story = EXCLUDED.is_a_story,
            is_shareable = EXCLUDED.is_shareable,
            is_override_mission = EXCLUDED.is_override_mission,
            award_xp = EXCLUDED.award_xp,
            can_abandon = EXCLUDED.can_abandon,
            can_fail = EXCLUDED.can_fail,
            can_repeat_on_fail = EXCLUDED.can_repeat_on_fail,
            num_repeats = EXCLUDED.num_repeats,
            show_faction_change_icon = EXCLUDED.show_faction_change_icon,
            show_instance_icon = EXCLUDED.show_instance_icon,
            show_pvp_icon = EXCLUDED.show_pvp_icon,
            script_name = EXCLUDED.script_name,
            script_spaces = EXCLUDED.script_spaces,
            reward_naq = EXCLUDED.reward_naq,
            reward_xp = EXCLUDED.reward_xp
        RETURNING mission_id
        "#,
    )
    .bind(mission.mission_id)
    .bind(&mission.mission_label)
    .bind(&mission.mission_defn)
    .bind(&mission.history_text)
    .bind(mission.level)
    .bind(mission.difficulty)
    .bind(mission.is_enabled)
    .bind(mission.is_hidden)
    .bind(mission.is_a_story)
    .bind(mission.is_shareable)
    .bind(mission.is_override_mission)
    .bind(mission.award_xp)
    .bind(mission.can_abandon)
    .bind(mission.can_fail)
    .bind(mission.can_repeat_on_fail)
    .bind(mission.num_repeats)
    .bind(mission.show_faction_change_icon)
    .bind(mission.show_instance_icon)
    .bind(mission.show_pvp_icon)
    .bind(&mission.script_name)
    .bind(&mission.script_spaces)
    .bind(mission.reward_naq)
    .bind(mission.reward_xp)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| format!("Failed to upsert mission: {e}"))?;

    // Delete old children -- cascade from steps down through objectives/tasks,
    // and separately reward_groups/rewards.
    sqlx::query(
        "DELETE FROM mission_tasks WHERE objective_id IN \
         (SELECT objective_id FROM mission_objectives WHERE step_id IN \
          (SELECT step_id FROM mission_steps WHERE mission_id = $1))",
    )
    .bind(mission_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Failed to delete old tasks: {e}"))?;

    sqlx::query(
        "DELETE FROM mission_objectives WHERE step_id IN \
         (SELECT step_id FROM mission_steps WHERE mission_id = $1)",
    )
    .bind(mission_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Failed to delete old objectives: {e}"))?;

    sqlx::query("DELETE FROM mission_steps WHERE mission_id = $1")
        .bind(mission_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to delete old steps: {e}"))?;

    sqlx::query(
        "DELETE FROM mission_rewards WHERE group_id IN \
         (SELECT group_id FROM mission_reward_groups WHERE mission_id = $1)",
    )
    .bind(mission_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Failed to delete old rewards: {e}"))?;

    sqlx::query("DELETE FROM mission_reward_groups WHERE mission_id = $1")
        .bind(mission_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to delete old reward groups: {e}"))?;

    // Re-insert steps -> objectives -> tasks
    for step in &mission.steps {
        sqlx::query(
            "INSERT INTO mission_steps (step_id, mission_id, award_xp, difficulty, \
             step_enabled, step_display_log_text, index) \
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(step.step_id)
        .bind(mission_id)
        .bind(step.award_xp)
        .bind(step.difficulty)
        .bind(step.step_enabled)
        .bind(&step.step_display_log_text)
        .bind(step.index)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to insert step: {e}"))?;

        for obj in &step.objectives {
            sqlx::query(
                "INSERT INTO mission_objectives (objective_id, step_id, award_xp, difficulty, \
                 is_enabled, is_hidden, is_optional, display_log_text) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            )
            .bind(obj.objective_id)
            .bind(step.step_id)
            .bind(obj.award_xp)
            .bind(obj.difficulty)
            .bind(obj.is_enabled)
            .bind(obj.is_hidden)
            .bind(obj.is_optional)
            .bind(&obj.display_log_text)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Failed to insert objective: {e}"))?;

            for task in &obj.tasks {
                sqlx::query(
                    "INSERT INTO mission_tasks (task_id, objective_id, award_xp, difficulty, \
                     is_enabled, task_type) \
                     VALUES ($1, $2, $3, $4, $5, $6)",
                )
                .bind(task.task_id)
                .bind(obj.objective_id)
                .bind(task.award_xp)
                .bind(task.difficulty)
                .bind(task.is_enabled)
                .bind(task.task_type)
                .execute(&mut *tx)
                .await
                .map_err(|e| format!("Failed to insert task: {e}"))?;
            }
        }
    }

    // Re-insert reward groups -> rewards
    for rg in &mission.reward_groups {
        let group_id: i32 = sqlx::query_scalar(
            "INSERT INTO mission_reward_groups (mission_id, choices) \
             VALUES ($1, $2) RETURNING group_id",
        )
        .bind(mission_id)
        .bind(rg.choices)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| format!("Failed to insert reward group: {e}"))?;

        for reward in &rg.rewards {
            sqlx::query(
                "INSERT INTO mission_rewards (group_id, item_id) VALUES ($1, $2)",
            )
            .bind(group_id)
            .bind(reward.item_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Failed to insert reward: {e}"))?;
        }
    }

    tx.commit()
        .await
        .map_err(|e| format!("Failed to commit mission save: {e}"))?;

    tracing::info!("Saved mission {mission_id} with {} steps, {} reward groups",
        mission.steps.len(), mission.reward_groups.len());
    Ok(mission_id)
}

#[tauri::command]
pub async fn delete_mission(
    state: tauri::State<'_, AppState>,
    mission_id: i32,
) -> Result<(), String> {
    tracing::debug!("delete_mission called, mission_id={mission_id:?}");
    let pool = state.pool()?;
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| format!("Failed to begin transaction: {e}"))?;

    // Manual cascade: tasks -> objectives -> steps, rewards -> reward_groups, then mission
    sqlx::query(
        "DELETE FROM mission_tasks WHERE objective_id IN \
         (SELECT objective_id FROM mission_objectives WHERE step_id IN \
          (SELECT step_id FROM mission_steps WHERE mission_id = $1))",
    )
    .bind(mission_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Failed to delete tasks: {e}"))?;

    sqlx::query(
        "DELETE FROM mission_objectives WHERE step_id IN \
         (SELECT step_id FROM mission_steps WHERE mission_id = $1)",
    )
    .bind(mission_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Failed to delete objectives: {e}"))?;

    sqlx::query("DELETE FROM mission_steps WHERE mission_id = $1")
        .bind(mission_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to delete steps: {e}"))?;

    sqlx::query(
        "DELETE FROM mission_rewards WHERE group_id IN \
         (SELECT group_id FROM mission_reward_groups WHERE mission_id = $1)",
    )
    .bind(mission_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Failed to delete rewards: {e}"))?;

    sqlx::query("DELETE FROM mission_reward_groups WHERE mission_id = $1")
        .bind(mission_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to delete reward groups: {e}"))?;

    sqlx::query("DELETE FROM missions WHERE mission_id = $1")
        .bind(mission_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to delete mission: {e}"))?;

    tx.commit()
        .await
        .map_err(|e| format!("Failed to commit mission delete: {e}"))?;

    tracing::info!("Deleted mission {mission_id}");
    Ok(())
}

// ---------------------------------------------------------------------------
// Loot Tables
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct LootTableSummary {
    pub loot_table_id: i32,
    pub description: String,
    pub entry_count: i64,
}

#[derive(Serialize, Deserialize)]
pub struct LootEntry {
    pub loot_id: Option<i32>,
    pub design_id: Option<i32>,
    pub min_quantity: i32,
    pub max_quantity: i32,
    pub probability: f32,
}

#[derive(Serialize, Deserialize)]
pub struct LootTable {
    pub loot_table_id: Option<i32>,
    pub description: String,
    pub entries: Vec<LootEntry>,
}

#[tauri::command]
pub async fn list_loot_tables(
    state: tauri::State<'_, AppState>,
    search: Option<String>,
) -> Result<Vec<LootTableSummary>, String> {
    tracing::debug!("list_loot_tables called, search={:?}", search);
    let pool = state.pool()?;

    let rows = sqlx::query(
        r#"
        SELECT lt.loot_table_id, lt.description,
               COUNT(l.loot_id) AS entry_count
        FROM loot_tables lt
        LEFT JOIN loot l ON l.loot_table_id = lt.loot_table_id
        WHERE ($1::text IS NULL OR lt.description ILIKE '%' || $1 || '%')
        GROUP BY lt.loot_table_id, lt.description
        ORDER BY lt.loot_table_id
        "#,
    )
    .bind(&search)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to list loot tables: {e}"))?;

    Ok(rows
        .iter()
        .map(|r| LootTableSummary {
            loot_table_id: r.get("loot_table_id"),
            description: r.get("description"),
            entry_count: r.get("entry_count"),
        })
        .collect())
}

#[tauri::command]
pub async fn get_loot_table(
    state: tauri::State<'_, AppState>,
    loot_table_id: i32,
) -> Result<LootTable, String> {
    tracing::debug!("get_loot_table called, loot_table_id={loot_table_id:?}");
    let pool = state.pool()?;

    let header = sqlx::query(
        "SELECT loot_table_id, description FROM loot_tables WHERE loot_table_id = $1",
    )
    .bind(loot_table_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("Failed to get loot table: {e}"))?
    .ok_or_else(|| format!("Loot table {loot_table_id} not found"))?;

    let entry_rows = sqlx::query(
        "SELECT loot_id, design_id, min_quantity, max_quantity, probability \
         FROM loot WHERE loot_table_id = $1 ORDER BY loot_id",
    )
    .bind(loot_table_id)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to load loot entries: {e}"))?;

    let entries = entry_rows
        .iter()
        .map(|r| LootEntry {
            loot_id: Some(r.get("loot_id")),
            design_id: r.get("design_id"),
            min_quantity: r.get("min_quantity"),
            max_quantity: r.get("max_quantity"),
            probability: r.get("probability"),
        })
        .collect();

    Ok(LootTable {
        loot_table_id: Some(header.get("loot_table_id")),
        description: header.get("description"),
        entries,
    })
}

#[tauri::command]
pub async fn save_loot_table(
    state: tauri::State<'_, AppState>,
    loot_table: LootTable,
) -> Result<i32, String> {
    tracing::debug!("save_loot_table called");
    let pool = state.pool()?;
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| format!("Failed to begin transaction: {e}"))?;

    // Upsert the loot table header
    let loot_table_id: i32 = sqlx::query_scalar(
        r#"
        INSERT INTO loot_tables (loot_table_id, description)
        VALUES (
            COALESCE($1, nextval('loot_tables_loot_table_id_seq'::regclass)),
            $2
        )
        ON CONFLICT (loot_table_id) DO UPDATE SET
            description = EXCLUDED.description
        RETURNING loot_table_id
        "#,
    )
    .bind(loot_table.loot_table_id)
    .bind(&loot_table.description)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| format!("Failed to upsert loot table: {e}"))?;

    // Delete old entries and re-insert
    sqlx::query("DELETE FROM loot WHERE loot_table_id = $1")
        .bind(loot_table_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to delete old loot entries: {e}"))?;

    for entry in &loot_table.entries {
        sqlx::query(
            "INSERT INTO loot (loot_table_id, design_id, min_quantity, max_quantity, probability) \
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(loot_table_id)
        .bind(entry.design_id)
        .bind(entry.min_quantity)
        .bind(entry.max_quantity)
        .bind(entry.probability)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to insert loot entry: {e}"))?;
    }

    tx.commit()
        .await
        .map_err(|e| format!("Failed to commit loot table save: {e}"))?;

    tracing::info!("Saved loot table {loot_table_id} with {} entries",
        loot_table.entries.len());
    Ok(loot_table_id)
}

#[tauri::command]
pub async fn delete_loot_table(
    state: tauri::State<'_, AppState>,
    loot_table_id: i32,
) -> Result<(), String> {
    tracing::debug!("delete_loot_table called, loot_table_id={loot_table_id:?}");
    let pool = state.pool()?;
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| format!("Failed to begin transaction: {e}"))?;

    sqlx::query("DELETE FROM loot WHERE loot_table_id = $1")
        .bind(loot_table_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to delete loot entries: {e}"))?;

    sqlx::query("DELETE FROM loot_tables WHERE loot_table_id = $1")
        .bind(loot_table_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to delete loot table: {e}"))?;

    tx.commit()
        .await
        .map_err(|e| format!("Failed to commit loot table delete: {e}"))?;

    tracing::info!("Deleted loot table {loot_table_id}");
    Ok(())
}

// ---------------------------------------------------------------------------
// Abilities (with nested effects and effect_nvps)
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct AbilitySummary {
    pub ability_id: i32,
    pub name: String,
    pub type_id: String,
    pub cooldown: f32,
}

#[derive(Serialize, Deserialize)]
pub struct EffectNvp {
    pub nvp_id: Option<i32>,
    pub name: String,
    pub value: String,
}

#[derive(Serialize, Deserialize)]
pub struct Effect {
    pub effect_id: i32,
    pub delay: i32,
    pub effect_desc: String,
    pub effect_sequence: i32,
    pub flags: i32,
    pub icon_location: Option<String>,
    pub pulse_count: i32,
    pub pulse_duration: f32,
    pub tcm_param1: Option<String>,
    pub tcm_param2: Option<String>,
    pub target_collection_method: String,
    pub use_ability_velocity: bool,
    pub is_channeled: bool,
    pub name: String,
    pub target_collection_id: i32,
    pub event_set_id: Option<i32>,
    pub script_name: Option<String>,
    pub nvps: Vec<EffectNvp>,
}

#[derive(Serialize, Deserialize)]
pub struct Ability {
    pub ability_id: Option<i32>,
    pub name: String,
    pub description: String,
    pub type_id: String,
    pub cooldown: f32,
    pub flags: i32,
    pub icon: Option<String>,
    pub is_ranged: bool,
    pub min_range: i32,
    pub max_range: i32,
    pub passive_yn: bool,
    pub param1: Option<String>,
    pub param2: Option<String>,
    pub target_type_id: i32,
    pub target_collection_method: String,
    pub taunt_adjustment: i32,
    pub threat_level_id: String,
    pub training_cost: i32,
    pub velocity: f32,
    pub warmup: f32,
    pub effect_ids: Vec<i32>,
    pub moniker_ids: Vec<i64>,
    pub event_set_id: Option<i32>,
    pub required_ammo: i32,
    pub positions: Vec<String>,
    pub item_monikers: Vec<i64>,
    pub effects: Vec<Effect>,
}

#[tauri::command]
pub async fn list_abilities(
    state: tauri::State<'_, AppState>,
    search: Option<String>,
) -> Result<Vec<AbilitySummary>, String> {
    tracing::debug!("list_abilities called, search={:?}", search);
    let pool = state.pool()?;

    let rows = sqlx::query(
        r#"
        SELECT ability_id, name, type_id::text, cooldown
        FROM abilities
        WHERE ($1::text IS NULL OR name ILIKE '%' || $1 || '%')
        ORDER BY ability_id
        "#,
    )
    .bind(&search)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to list abilities: {e}"))?;

    Ok(rows
        .iter()
        .map(|r| AbilitySummary {
            ability_id: r.get("ability_id"),
            name: r.get("name"),
            type_id: r.get("type_id"),
            cooldown: r.get("cooldown"),
        })
        .collect())
}

#[tauri::command]
pub async fn get_ability(
    state: tauri::State<'_, AppState>,
    ability_id: i32,
) -> Result<Ability, String> {
    tracing::debug!("get_ability called, ability_id={ability_id:?}");
    let pool = state.pool()?;

    let ar = sqlx::query(
        r#"
        SELECT ability_id, name, description, type_id::text, cooldown, flags,
               icon, is_ranged, min_range, max_range, passive_yn,
               param1, param2, target_type_id,
               target_collection_method::text, taunt_adjustment,
               threat_level_id::text, training_cost, velocity, warmup,
               effect_ids, moniker_ids, event_set_id, required_ammo,
               (SELECT array_agg(x::text) FROM unnest(positions) AS x) AS positions,
               item_monikers
        FROM abilities
        WHERE ability_id = $1
        "#,
    )
    .bind(ability_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("Failed to get ability: {e}"))?
    .ok_or_else(|| format!("Ability {ability_id} not found"))?;

    // Fetch effects linked to this ability
    let effect_rows = sqlx::query(
        r#"
        SELECT effect_id, delay, effect_desc, effect_sequence, flags,
               icon_location, pulse_count, pulse_duration,
               tcm_param1, tcm_param2, target_collection_method::text,
               use_ability_velocity, is_channeled, name,
               target_collection_id, event_set_id, script_name
        FROM effects
        WHERE ability_id = $1
        ORDER BY effect_sequence
        "#,
    )
    .bind(ability_id)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to load effects: {e}"))?;

    let mut effects = Vec::new();
    for er in &effect_rows {
        let effect_id: i32 = er.get("effect_id");

        let nvp_rows = sqlx::query(
            "SELECT nvp_id, name, value FROM effect_nvps \
             WHERE effect_id = $1 ORDER BY nvp_id",
        )
        .bind(effect_id)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Failed to load effect NVPs: {e}"))?;

        let nvps = nvp_rows
            .iter()
            .map(|nr| EffectNvp {
                nvp_id: Some(nr.get("nvp_id")),
                name: nr.get("name"),
                value: nr.get("value"),
            })
            .collect();

        effects.push(Effect {
            effect_id,
            delay: er.get("delay"),
            effect_desc: er.get("effect_desc"),
            effect_sequence: er.get("effect_sequence"),
            flags: er.get("flags"),
            icon_location: er.get("icon_location"),
            pulse_count: er.get("pulse_count"),
            pulse_duration: er.get("pulse_duration"),
            tcm_param1: er.get("tcm_param1"),
            tcm_param2: er.get("tcm_param2"),
            target_collection_method: er.get("target_collection_method"),
            use_ability_velocity: er.get("use_ability_velocity"),
            is_channeled: er.get("is_channeled"),
            name: er.get("name"),
            target_collection_id: er.get("target_collection_id"),
            event_set_id: er.get("event_set_id"),
            script_name: er.get("script_name"),
            nvps,
        });
    }

    Ok(Ability {
        ability_id: Some(ar.get("ability_id")),
        name: ar.get("name"),
        description: ar.get("description"),
        type_id: ar.get("type_id"),
        cooldown: ar.get("cooldown"),
        flags: ar.get("flags"),
        icon: ar.get("icon"),
        is_ranged: ar.get("is_ranged"),
        min_range: ar.get("min_range"),
        max_range: ar.get("max_range"),
        passive_yn: ar.get("passive_yn"),
        param1: ar.get("param1"),
        param2: ar.get("param2"),
        target_type_id: ar.get("target_type_id"),
        target_collection_method: ar.get("target_collection_method"),
        taunt_adjustment: ar.get("taunt_adjustment"),
        threat_level_id: ar.get("threat_level_id"),
        training_cost: ar.get("training_cost"),
        velocity: ar.get("velocity"),
        warmup: ar.get("warmup"),
        effect_ids: ar.get::<Vec<i32>, _>("effect_ids"),
        moniker_ids: ar.get::<Vec<i64>, _>("moniker_ids"),
        event_set_id: ar.get("event_set_id"),
        required_ammo: ar.get("required_ammo"),
        positions: ar.get::<Option<Vec<String>>, _>("positions").unwrap_or_default(),
        item_monikers: ar.get::<Vec<i64>, _>("item_monikers"),
        effects,
    })
}

#[tauri::command]
pub async fn save_ability(
    state: tauri::State<'_, AppState>,
    ability: Ability,
) -> Result<i32, String> {
    tracing::debug!("save_ability called");
    let pool = state.pool()?;
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| format!("Failed to begin transaction: {e}"))?;

    // Upsert ability header
    let ability_id: i32 = sqlx::query_scalar(
        r#"
        INSERT INTO abilities (
            ability_id, name, description, type_id, cooldown, flags,
            icon, is_ranged, min_range, max_range, passive_yn,
            param1, param2, target_type_id, target_collection_method,
            taunt_adjustment, threat_level_id, training_cost,
            velocity, warmup, effect_ids, moniker_ids, event_set_id,
            required_ammo, positions, item_monikers
        ) VALUES (
            COALESCE($1, nextval('abilities_ability_id_seq'::regclass)),
            $2, $3, $4::text::"EAbilityTypes", $5, $6, $7, $8, $9, $10,
            $11, $12, $13, $14,
            $15::text::"ETargetCollectionMethod",
            $16, $17::text::"EThreatLevel", $18, $19, $20, $21, $22, $23, $24,
            (SELECT array_agg(x::"ECASPosition") FROM unnest($25::text[]) AS x),
            $26
        )
        ON CONFLICT (ability_id) DO UPDATE SET
            name = EXCLUDED.name,
            description = EXCLUDED.description,
            type_id = EXCLUDED.type_id,
            cooldown = EXCLUDED.cooldown,
            flags = EXCLUDED.flags,
            icon = EXCLUDED.icon,
            is_ranged = EXCLUDED.is_ranged,
            min_range = EXCLUDED.min_range,
            max_range = EXCLUDED.max_range,
            passive_yn = EXCLUDED.passive_yn,
            param1 = EXCLUDED.param1,
            param2 = EXCLUDED.param2,
            target_type_id = EXCLUDED.target_type_id,
            target_collection_method = EXCLUDED.target_collection_method,
            taunt_adjustment = EXCLUDED.taunt_adjustment,
            threat_level_id = EXCLUDED.threat_level_id,
            training_cost = EXCLUDED.training_cost,
            velocity = EXCLUDED.velocity,
            warmup = EXCLUDED.warmup,
            effect_ids = EXCLUDED.effect_ids,
            moniker_ids = EXCLUDED.moniker_ids,
            event_set_id = EXCLUDED.event_set_id,
            required_ammo = EXCLUDED.required_ammo,
            positions = EXCLUDED.positions,
            item_monikers = EXCLUDED.item_monikers
        RETURNING ability_id
        "#,
    )
    .bind(ability.ability_id)
    .bind(&ability.name)
    .bind(&ability.description)
    .bind(&ability.type_id)
    .bind(ability.cooldown)
    .bind(ability.flags)
    .bind(&ability.icon)
    .bind(ability.is_ranged)
    .bind(ability.min_range)
    .bind(ability.max_range)
    .bind(ability.passive_yn)
    .bind(&ability.param1)
    .bind(&ability.param2)
    .bind(ability.target_type_id)
    .bind(&ability.target_collection_method)
    .bind(ability.taunt_adjustment)
    .bind(&ability.threat_level_id)
    .bind(ability.training_cost)
    .bind(ability.velocity)
    .bind(ability.warmup)
    .bind(&ability.effect_ids)
    .bind(&ability.moniker_ids)
    .bind(ability.event_set_id)
    .bind(ability.required_ammo)
    .bind(&ability.positions)
    .bind(&ability.item_monikers)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| format!("Failed to upsert ability: {e}"))?;

    // Delete old effects and NVPs for this ability, then re-insert
    sqlx::query(
        "DELETE FROM effect_nvps WHERE effect_id IN \
         (SELECT effect_id FROM effects WHERE ability_id = $1)",
    )
    .bind(ability_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Failed to delete old effect NVPs: {e}"))?;

    sqlx::query("DELETE FROM effects WHERE ability_id = $1")
        .bind(ability_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to delete old effects: {e}"))?;

    for effect in &ability.effects {
        sqlx::query(
            r#"
            INSERT INTO effects (
                effect_id, ability_id, delay, effect_desc, effect_sequence, flags,
                icon_location, pulse_count, pulse_duration,
                tcm_param1, tcm_param2, target_collection_method,
                use_ability_velocity, is_channeled, name,
                target_collection_id, event_set_id, script_name
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11,
                $12::text::"ETargetCollectionMethod",
                $13, $14, $15, $16, $17, $18
            )
            "#,
        )
        .bind(effect.effect_id)
        .bind(ability_id)
        .bind(effect.delay)
        .bind(&effect.effect_desc)
        .bind(effect.effect_sequence)
        .bind(effect.flags)
        .bind(&effect.icon_location)
        .bind(effect.pulse_count)
        .bind(effect.pulse_duration)
        .bind(&effect.tcm_param1)
        .bind(&effect.tcm_param2)
        .bind(&effect.target_collection_method)
        .bind(effect.use_ability_velocity)
        .bind(effect.is_channeled)
        .bind(&effect.name)
        .bind(effect.target_collection_id)
        .bind(effect.event_set_id)
        .bind(&effect.script_name)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to insert effect: {e}"))?;

        for nvp in &effect.nvps {
            sqlx::query(
                "INSERT INTO effect_nvps (effect_id, name, value) VALUES ($1, $2, $3)",
            )
            .bind(effect.effect_id)
            .bind(&nvp.name)
            .bind(&nvp.value)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Failed to insert effect NVP: {e}"))?;
        }
    }

    tx.commit()
        .await
        .map_err(|e| format!("Failed to commit ability save: {e}"))?;

    tracing::info!("Saved ability {ability_id} with {} effects", ability.effects.len());
    Ok(ability_id)
}

#[tauri::command]
pub async fn delete_ability(
    state: tauri::State<'_, AppState>,
    ability_id: i32,
) -> Result<(), String> {
    tracing::debug!("delete_ability called, ability_id={ability_id:?}");
    let pool = state.pool()?;
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| format!("Failed to begin transaction: {e}"))?;

    sqlx::query(
        "DELETE FROM effect_nvps WHERE effect_id IN \
         (SELECT effect_id FROM effects WHERE ability_id = $1)",
    )
    .bind(ability_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Failed to delete effect NVPs: {e}"))?;

    sqlx::query("DELETE FROM effects WHERE ability_id = $1")
        .bind(ability_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to delete effects: {e}"))?;

    sqlx::query("DELETE FROM abilities WHERE ability_id = $1")
        .bind(ability_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to delete ability: {e}"))?;

    tx.commit()
        .await
        .map_err(|e| format!("Failed to commit ability delete: {e}"))?;

    tracing::info!("Deleted ability {ability_id}");
    Ok(())
}
