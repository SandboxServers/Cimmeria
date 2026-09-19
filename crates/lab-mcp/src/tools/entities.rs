//! `server_entity_get` + `server_entity_query` tool logic (issue #688).
//!
//! Both send a read-only [`LabQuery`] to the cell loop via
//! [`LabState::lab_query`] and shape the reply into JSON. Kept free of the rmcp
//! macro surface so the transport-independent logic can be reasoned about (the
//! cell-loop side is tested in `cimmeria-services`).

use serde_json::{json, Value};

use cimmeria_services::cell::messages::{
    LabEntityFilter, LabQuery, LabQueryReply, LabRadius, LabRadiusCenter,
};

use crate::state::LabState;

/// One entity snapshot by id. Returns `{ "entity": <snapshot|null> }`.
pub async fn entity_get(state: &LabState, entity_id: u32) -> Result<Value, String> {
    match state.lab_query(LabQuery::EntityGet { entity_id }).await? {
        LabQueryReply::Entity { entity } => Ok(json!({ "entity": entity })),
        other => Err(unexpected_reply(&other)),
    }
}

/// Filtered entity query. `space_id` / `template_id` / `class_id` are
/// AND-combined; if `radius` is set it must be paired with a center
/// (`around_entity` or `around_point`).
#[allow(clippy::too_many_arguments)]
pub async fn entity_query(
    state: &LabState,
    space_id: Option<u32>,
    template_id: Option<i32>,
    class_id: Option<u8>,
    radius: Option<f32>,
    around_entity: Option<u32>,
    around_point: Option<[f32; 3]>,
) -> Result<Value, String> {
    let radius = match radius {
        None => None,
        Some(r) => {
            let center = match (around_entity, around_point) {
                (Some(eid), _) => LabRadiusCenter::Entity(eid),
                (None, Some(p)) => LabRadiusCenter::Point(p),
                (None, None) => {
                    return Err(
                        "radius requires a center: pass around_entity or around_point".to_string(),
                    )
                }
            };
            Some(LabRadius { center, radius: r })
        }
    };

    let filter = LabEntityFilter {
        space_id,
        template_id,
        class_id,
        radius,
    };

    match state.lab_query(LabQuery::EntityQuery { filter }).await? {
        LabQueryReply::Entities {
            entities,
            total_matched,
            capped,
        } => Ok(json!({
            "entities": entities,
            "returned": entities.len(),
            "total_matched": total_matched,
            "capped": capped,
        })),
        other => Err(unexpected_reply(&other)),
    }
}

/// The cell handler always returns the reply variant matching the query, so a
/// mismatch is a server-side bug — surface it as a tool error rather than
/// panicking.
fn unexpected_reply(reply: &LabQueryReply) -> String {
    format!("unexpected LabQuery reply variant: {reply:?}")
}
