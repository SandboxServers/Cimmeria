//! `server_entity_get` + `server_entity_query` tool logic (issue #688).
//!
//! Both send a read-only [`LabQuery`] to the cell loop via
//! [`LabState::lab_query`] and shape the reply into JSON. Kept free of the rmcp
//! macro surface so the transport-independent logic can be reasoned about (the
//! cell-loop side is tested in `cimmeria-services`). Each reply gets its
//! NameBook names ([`super::names`]) before it is shaped (Rule 6, NT-41).

use serde_json::{json, Value};

use cimmeria_names::NameBook;
use cimmeria_services::cell::messages::{
    LabEntityFilter, LabQuery, LabQueryReply, LabRadius, LabRadiusCenter,
};

use super::names::name_snapshot;
use crate::state::LabState;

/// One entity snapshot by id. Returns `{ "entity": <snapshot|null> }`.
pub async fn entity_get(state: &LabState, entity_id: u32) -> Result<Value, String> {
    let reply = state.lab_query(LabQuery::EntityGet { entity_id }).await?;
    shape_entity_get(reply, &cimmeria_names::book())
}

/// The `server_entity_get` JSON for a cell reply, named from `book`.
fn shape_entity_get(reply: LabQueryReply, book: &NameBook) -> Result<Value, String> {
    match reply {
        LabQueryReply::Entity { mut entity } => {
            if let Some(e) = entity.as_mut() {
                name_snapshot(e, book);
            }
            Ok(json!({ "entity": entity }))
        }
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

    let reply = state.lab_query(LabQuery::EntityQuery { filter }).await?;
    shape_entity_query(reply, &cimmeria_names::book())
}

/// The `server_entity_query` JSON for a cell reply, named from `book`.
fn shape_entity_query(reply: LabQueryReply, book: &NameBook) -> Result<Value, String> {
    match reply {
        LabQueryReply::Entities {
            mut entities,
            total_matched,
            capped,
        } => {
            for e in &mut entities {
                name_snapshot(e, book);
            }
            Ok(json!({
                "entities": entities,
                "returned": entities.len(),
                "total_matched": total_matched,
                "capped": capped,
            }))
        }
        other => Err(unexpected_reply(&other)),
    }
}

/// The cell handler always returns the reply variant matching the query, so a
/// mismatch is a server-side bug — surface it as a tool error rather than
/// panicking.
pub(super) fn unexpected_reply(reply: &LabQueryReply) -> String {
    format!("unexpected LabQuery reply variant: {reply:?}")
}

#[cfg(test)]
#[path = "entities_tests.rs"]
mod tests;
