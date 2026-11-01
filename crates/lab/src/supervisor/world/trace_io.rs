//! The live `trace_line` call (C14): one bridge read, mapped onto [`TraceHit`].

use serde_json::json;

use super::geometry::Vec3;
use super::io::TraceHit;
use crate::supervisor::Supervisor;

/// Trace `start` to `end` in the client. An old DLL (error `-32601`) gives
/// `trace_line unsupported`; `hit: false` gives `None`.
pub(super) async fn live_trace(
    sup: &Supervisor,
    start: Vec3,
    end: Vec3,
    flags: u32,
) -> Result<Option<TraceHit>, String> {
    let params = json!({
        "start": [start.x, start.y, start.z],
        "end": [end.x, end.y, end.z],
        "flags": flags,
    });
    let v = sup.bridge.call("trace_line", params).await.map_err(|e| {
        if e.to_string().contains("-32601") {
            "trace_line unsupported".to_string()
        } else {
            format!("trace_line: {e}")
        }
    })?;
    if !v["hit"].as_bool().unwrap_or(false) {
        return Ok(None);
    }
    let loc = &v["location"];
    Ok(Some(TraceHit {
        time: v["time"].as_f64().unwrap_or(0.0),
        location: Vec3::new(
            loc[0].as_f64().unwrap_or(0.0),
            loc[1].as_f64().unwrap_or(0.0),
            loc[2].as_f64().unwrap_or(0.0),
        ),
        item: v["item"].as_i64().unwrap_or(-1) as i32,
        actor: v["actor"].as_u64().unwrap_or(0) as u32,
    }))
}
