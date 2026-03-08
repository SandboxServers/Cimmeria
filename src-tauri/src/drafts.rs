use crate::state::AppState;

const CREATE_CHAIN_EDITOR_DRAFTS_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS chain_editor_drafts (
    id BIGSERIAL PRIMARY KEY,
    space_id TEXT NOT NULL,
    mission_id TEXT NOT NULL DEFAULT '',
    payload JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chain_editor_drafts_unique_scope UNIQUE (space_id, mission_id)
)
"#;

#[tauri::command]
pub async fn load_chain_editor_draft(
    state: tauri::State<'_, AppState>,
    space_id: String,
    mission_id: Option<String>,
) -> Result<Option<serde_json::Value>, String> {
    let pool = state.db_pool().await?;
    ensure_chain_editor_drafts_table(pool).await?;
    let normalized_mission_id = normalize_mission_id(mission_id);

    sqlx::query_scalar::<_, serde_json::Value>(
        r#"
        SELECT payload
        FROM chain_editor_drafts
        WHERE space_id = $1 AND mission_id = $2
        "#,
    )
    .bind(space_id)
    .bind(normalized_mission_id)
    .fetch_optional(pool)
    .await
    .map_err(|error| format!("failed to load chain editor draft: {error}"))
}

#[tauri::command]
pub async fn save_chain_editor_draft(
    state: tauri::State<'_, AppState>,
    space_id: String,
    mission_id: Option<String>,
    payload: serde_json::Value,
) -> Result<(), String> {
    let pool = state.db_pool().await?;
    ensure_chain_editor_drafts_table(pool).await?;
    let normalized_mission_id = normalize_mission_id(mission_id);

    sqlx::query(
        r#"
        INSERT INTO chain_editor_drafts (space_id, mission_id, payload)
        VALUES ($1, $2, $3)
        ON CONFLICT (space_id, mission_id)
        DO UPDATE SET
            payload = EXCLUDED.payload,
            updated_at = NOW()
        "#,
    )
    .bind(space_id)
    .bind(normalized_mission_id)
    .bind(payload)
    .execute(pool)
    .await
    .map_err(|error| format!("failed to save chain editor draft: {error}"))?;

    Ok(())
}

async fn ensure_chain_editor_drafts_table(pool: &sqlx::PgPool) -> Result<(), String> {
    sqlx::query(CREATE_CHAIN_EDITOR_DRAFTS_TABLE)
        .execute(pool)
        .await
        .map_err(|error| format!("failed to prepare chain editor drafts table: {error}"))?;

    Ok(())
}

fn normalize_mission_id(mission_id: Option<String>) -> String {
    mission_id.unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::normalize_mission_id;

    #[test]
    fn normalize_mission_id_defaults_to_empty_scope() {
        assert_eq!(normalize_mission_id(None), "");
    }

    #[test]
    fn normalize_mission_id_preserves_specific_scope() {
        assert_eq!(normalize_mission_id(Some("638".to_string())), "638");
    }
}
