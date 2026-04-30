//! Helpers for populating the content engine [`ExecutionContext`] with
//! per-entity mission and step state.
//!
//! Every event-firing path needs to expose mission state to chain conditions
//! (e.g., `mission_622_status == "active"`). This module centralizes that
//! population so the dispatch sites stay focused on event-specific params.

use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_entity::cell_entity::CellEntity;
use cimmeria_entity::missions::{MISSION_ACTIVE, MISSION_COMPLETED, MISSION_NOT_ACTIVE};

/// Populate mission status and step status context params from entity state.
pub(super) fn populate_mission_context(entity: &CellEntity, ctx: &mut ExecutionContext) {
    for mission in entity.missions.all_missions() {
        let status_str = match mission.status {
            MISSION_NOT_ACTIVE => "not_active",
            MISSION_ACTIVE => "active",
            MISSION_COMPLETED => "completed",
            _ => "not_active",
        };
        ctx.set_param(
            format!("mission_{}_status", mission.mission_id),
            serde_json::json!(status_str),
        );

        // Also set step statuses for the current step
        if let Some(step_id) = mission.current_step_id {
            ctx.set_param(
                format!("mission_{}_step_{}_status", mission.mission_id, step_id),
                serde_json::json!("active"),
            );
        }
    }
}
