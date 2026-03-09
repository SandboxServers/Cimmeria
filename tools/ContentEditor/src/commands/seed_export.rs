use crate::state::AppState;
use serde_json::Value;
use sqlx::Row;
use std::fmt::Write;

#[tauri::command]
pub async fn export_to_seed_file(
    state: tauri::State<'_, AppState>,
    space_id: String,
    filename: Option<String>,
) -> Result<String, String> {
    let pool = state.pool()?;

    let chain_rows = sqlx::query(
        r#"
        SELECT chain_id, name, description, scope_type, scope_id, enabled, priority
        FROM content_chains
        WHERE editor_data ->> 'spaceId' = $1
        ORDER BY chain_id ASC
        "#,
    )
    .bind(&space_id)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to load chains for export: {e}"))?;

    if chain_rows.is_empty() {
        return Err(format!("No chains found for space '{space_id}'"));
    }

    let mut sql = String::new();
    writeln!(sql, "-- Content chains for space: {space_id}").unwrap();
    writeln!(sql, "-- Auto-exported by Cimmeria Content Editor").unwrap();
    writeln!(sql).unwrap();
    writeln!(sql, "SET search_path = resources, pg_catalog;").unwrap();
    writeln!(sql).unwrap();

    for row in &chain_rows {
        let chain_id: i64 = row.get("chain_id");
        let description: Option<String> = row.get("description");
        let scope_type: String = row.get("scope_type");
        let scope_id: Option<i32> = row.get("scope_id");
        let enabled: bool = row.get("enabled");
        let priority: i32 = row.get("priority");

        // Comment header
        writeln!(
            sql,
            "-- Chain {chain_id}: {}",
            description.as_deref().unwrap_or("(no description)")
        )
        .unwrap();

        // Chain INSERT
        writeln!(
            sql,
            "INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)"
        )
        .unwrap();
        writeln!(
            sql,
            "VALUES ({chain_id}, {}, '{}', {}, {}, {});",
            sql_str(description.as_deref()),
            scope_type,
            sql_opt_i32(scope_id),
            enabled,
            priority,
        )
        .unwrap();

        // Triggers
        let triggers = sqlx::query(
            "SELECT event_type, event_key, scope, once, sort_order \
             FROM content_triggers WHERE chain_id = $1 ORDER BY sort_order",
        )
        .bind(chain_id)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Failed to load triggers for chain {chain_id}: {e}"))?;

        for trigger in &triggers {
            let event_type: String = trigger.get("event_type");
            let event_key: String = trigger.get("event_key");
            let scope: String = trigger.get("scope");
            let once: bool = trigger.get("once");
            let sort_order: i32 = trigger.get("sort_order");

            writeln!(sql).unwrap();
            writeln!(
                sql,
                "INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)"
            )
            .unwrap();
            writeln!(
                sql,
                "VALUES ({chain_id}, '{event_type}', '{event_key}', '{scope}', {once}, {sort_order});"
            )
            .unwrap();
        }

        // Conditions
        let conditions = sqlx::query(
            "SELECT condition_type, target_id, target_key, operator, value, sort_order \
             FROM content_conditions WHERE chain_id = $1 ORDER BY sort_order",
        )
        .bind(chain_id)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Failed to load conditions for chain {chain_id}: {e}"))?;

        for condition in &conditions {
            let condition_type: String = condition.get("condition_type");
            let target_id: Option<i32> = condition.get("target_id");
            let target_key: Option<String> = condition.get("target_key");
            let operator: String = condition.get("operator");
            let value: String = condition.get("value");
            let sort_order: i32 = condition.get("sort_order");

            writeln!(sql).unwrap();
            writeln!(
                sql,
                "INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)"
            )
            .unwrap();
            writeln!(
                sql,
                "VALUES ({chain_id}, '{condition_type}', {}, {}, '{operator}', '{value}', {sort_order});",
                sql_opt_i32(target_id),
                sql_str(target_key.as_deref()),
            )
            .unwrap();
        }

        // Actions
        let actions = sqlx::query(
            "SELECT action_type, target_id, target_key, params, delay_ms, sort_order \
             FROM content_actions WHERE chain_id = $1 ORDER BY sort_order",
        )
        .bind(chain_id)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Failed to load actions for chain {chain_id}: {e}"))?;

        if !actions.is_empty() {
            writeln!(sql).unwrap();
            writeln!(
                sql,
                "INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)"
            )
            .unwrap();
            writeln!(sql, "VALUES").unwrap();

            for (i, action) in actions.iter().enumerate() {
                let action_type: String = action.get("action_type");
                let target_id: Option<i32> = action.get("target_id");
                let target_key: Option<String> = action.get("target_key");
                let params: Value = action.get("params");
                let delay_ms: i32 = action.get("delay_ms");
                let sort_order: i32 = action.get("sort_order");

                let params_str = serde_json::to_string(&params).unwrap_or_else(|_| "'{}'".to_string());
                let separator = if i < actions.len() - 1 { "," } else { ";" };

                writeln!(
                    sql,
                    "  ({chain_id}, '{action_type}', {}, {},\n   '{}', {delay_ms}, {sort_order}){separator}",
                    sql_opt_i32(target_id),
                    sql_str(target_key.as_deref()),
                    params_str.replace('\'', "''"),
                )
                .unwrap();
            }
        }

        writeln!(sql).unwrap();
    }

    // Determine output path
    let filename = filename.unwrap_or_else(|| {
        format!(
            "{}_chains.sql",
            space_id.to_lowercase().replace(' ', "_")
        )
    });
    let output_path = state.seed_dir().join(&filename);

    std::fs::write(&output_path, &sql)
        .map_err(|e| format!("Failed to write seed file {}: {e}", output_path.display()))?;

    let path_str = output_path.display().to_string();
    tracing::info!("Exported {} chains to {path_str}", chain_rows.len());

    Ok(path_str)
}

fn sql_str(value: Option<&str>) -> String {
    match value {
        Some(s) => format!("'{}'", s.replace('\'', "''")),
        None => "NULL".to_string(),
    }
}

fn sql_opt_i32(value: Option<i32>) -> String {
    match value {
        Some(v) => v.to_string(),
        None => "NULL".to_string(),
    }
}
