//! Shape the cell loop's read-only navmesh corridor for lab movement.

use serde_json::{json, Value};

use cimmeria_services::cell::messages::{LabQuery, LabQueryReply};

use crate::state::LabState;

pub(super) async fn nav_path(
    state: &LabState,
    space_id: u32,
    from: [f32; 3],
    to: [f32; 3],
) -> Result<Value, String> {
    if !from.into_iter().chain(to).all(f32::is_finite) {
        return Err("nav path coordinates must be finite".into());
    }
    let reply = state
        .lab_query(LabQuery::NavPath { space_id, from, to })
        .await?;
    shape(reply, space_id)
}

fn shape(reply: LabQueryReply, space_id: u32) -> Result<Value, String> {
    match reply {
        LabQueryReply::NavPath {
            status,
            waypoints,
            start_snap,
            end_snap,
        } => {
            let waypoints: Vec<Value> = waypoints
                .into_iter()
                .map(|p| json!({"x": p[0], "y": p[1], "z": p[2]}))
                .collect();
            Ok(json!({
                "space_id": space_id,
                "status": status,
                "waypoints": waypoints,
                "start_snap": start_snap,
                "end_snap": end_snap,
                "native_level": "read"
            }))
        }
        other => Err(format!("unexpected LabQuery reply variant: {other:?}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complete_path_is_machine_readable_but_only_a_read() {
        let out = shape(
            LabQueryReply::NavPath {
                status: "ok".into(),
                waypoints: vec![[-196.0, 55.0, -150.0], [-180.0, 55.0, -130.0]],
                start_snap: Some([-196.0, 55.0, -150.0]),
                end_snap: Some([-180.0, 55.0, -130.0]),
            },
            100,
        )
        .unwrap();
        assert_eq!(out["status"], "ok");
        assert_eq!(out["waypoints"][1]["x"], -180.0);
        assert_eq!(out["native_level"], "read");
    }
}
