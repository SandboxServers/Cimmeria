//! Mission definition cache.
//!
//! Loads the first step + objectives per mission and per-step objectives
//! into in-memory maps so that `AcceptMission` / `AdvanceStep` content
//! actions don't need per-action DB queries.

use sqlx::PgPool;

/// Cached mission definition: first step + its objectives.
///
/// Loaded at startup from `resources.mission_steps` + `resources.mission_objectives`
/// so that `AcceptMission` content actions can look up step/objective data without
/// per-action DB queries.
#[derive(Debug, Clone)]
pub struct MissionDefEntry {
    pub step_id: i32,
    pub objectives: Vec<MissionObjectiveDef>,
    /// `resources.missions.is_hidden` — when true, the mission stays out of
    /// the player's mission log. Propagated to the per-player
    /// `MissionInstance.is_hidden` at accept time so the manager's
    /// `active_missions()` filter (which gates `onMissionUpdate` resends)
    /// honors it. Without this, hidden sub-missions like the
    /// Hallway0N Controllers (mission 682-686) leak into the player's UI.
    pub is_hidden: bool,
    /// `resources.missions.num_repeats` — re-acceptance cap. Python parity:
    /// `MissionManager.py canOffer()` refuses a COMPLETED mission when
    /// `repeats > numRepeats`. Read by the `accept_mission` offer guard.
    pub num_repeats: i32,
    /// `resources.missions.can_repeat_on_fail` — when false, a FAILED
    /// mission can never be re-offered (Python parity: `canOffer()`).
    pub can_repeat_on_fail: bool,
}

/// A single objective within a mission step.
#[derive(Debug, Clone)]
pub struct MissionObjectiveDef {
    pub objective_id: i32,
    pub is_hidden: bool,
    pub is_optional: bool,
}

/// Load mission definitions (first step + objectives) from the database.
///
/// Maps `mission_id → MissionDefEntry` for all missions that have at least one step.
/// Only loads the first step (lowest `index`) per mission, matching the behavior
/// of `AcceptMission` which starts at step 0.
pub async fn load_mission_defs(
    pool: &PgPool,
) -> Result<std::collections::HashMap<i32, MissionDefEntry>, sqlx::Error> {
    use sqlx::Row;

    // Get the first step per mission (lowest index) plus the mission-level
    // `is_hidden` flag, joined in one query.
    let step_rows = sqlx::query(
        "SELECT DISTINCT ON (s.mission_id) s.mission_id, s.step_id, m.is_hidden, \
                m.num_repeats, m.can_repeat_on_fail \
         FROM resources.mission_steps s \
         JOIN resources.missions m ON m.mission_id = s.mission_id \
         ORDER BY s.mission_id, s.index ASC",
    )
    .fetch_all(pool)
    .await?;

    let mut map = std::collections::HashMap::with_capacity(step_rows.len());
    for r in &step_rows {
        let mission_id: i32 = r.get("mission_id");
        let step_id: i32 = r.get("step_id");
        let is_hidden: bool = r.get("is_hidden");
        let num_repeats: i32 = r.get("num_repeats");
        let can_repeat_on_fail: bool = r.get("can_repeat_on_fail");
        map.insert(
            mission_id,
            MissionDefEntry {
                step_id,
                objectives: Vec::new(),
                is_hidden,
                num_repeats,
                can_repeat_on_fail,
            },
        );
    }

    // Load objectives for all steps we just loaded
    let step_ids: Vec<i32> = map.values().map(|e| e.step_id).collect();
    if !step_ids.is_empty() {
        let obj_rows = sqlx::query(
            "SELECT step_id, objective_id, is_hidden, is_optional \
             FROM resources.mission_objectives \
             WHERE step_id = ANY($1)",
        )
        .bind(&step_ids)
        .fetch_all(pool)
        .await?;

        // Build a step_id → objectives lookup
        let mut obj_by_step: std::collections::HashMap<i32, Vec<MissionObjectiveDef>> =
            std::collections::HashMap::new();
        for r in &obj_rows {
            let step_id: i32 = r.get("step_id");
            let obj = MissionObjectiveDef {
                objective_id: r.get("objective_id"),
                is_hidden: r.get("is_hidden"),
                is_optional: r.get("is_optional"),
            };
            obj_by_step.entry(step_id).or_default().push(obj);
        }

        // Attach objectives to their mission entries
        for entry in map.values_mut() {
            if let Some(objs) = obj_by_step.remove(&entry.step_id) {
                entry.objectives = objs;
            }
        }
    }

    tracing::info!(count = map.len(), "Loaded mission_defs cache");
    Ok(map)
}

/// Load step objectives for all steps from the database.
///
/// Maps `step_id → Vec<MissionObjectiveDef>` so that `AdvanceStep` can
/// look up the objectives for a new step without per-action DB queries.
pub async fn load_step_objectives(
    pool: &PgPool,
) -> Result<std::collections::HashMap<i32, Vec<MissionObjectiveDef>>, sqlx::Error> {
    use sqlx::Row;

    let rows = sqlx::query(
        "SELECT step_id, objective_id, is_hidden, is_optional \
         FROM resources.mission_objectives ORDER BY step_id, objective_id",
    )
    .fetch_all(pool)
    .await?;

    let mut map: std::collections::HashMap<i32, Vec<MissionObjectiveDef>> =
        std::collections::HashMap::new();
    for r in &rows {
        let step_id: i32 = r.get("step_id");
        let obj = MissionObjectiveDef {
            objective_id: r.get("objective_id"),
            is_hidden: r.get("is_hidden"),
            is_optional: r.get("is_optional"),
        };
        map.entry(step_id).or_default().push(obj);
    }

    tracing::info!(steps = map.len(), "Loaded step_objectives cache");
    Ok(map)
}
