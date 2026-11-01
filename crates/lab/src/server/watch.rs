//! The DLL watch tool (`client_watch`): start, read or stop the per-frame
//! sample stream. Orchestration lives in `supervisor::watch_store`.

use rmcp::{
    handler::server::wrapper::Parameters, model::*, schemars, tool, tool_router,
    ErrorData as McpError,
};
use serde_json::{json, Map, Value};

use super::LabServer;

/// Args for `client_watch`.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct WatchArgs {
    /// `start`, `read` or `stop`.
    pub action: String,
    /// `start`: sample every N frames (the DLL default when omitted).
    #[serde(default)]
    pub period_frames: Option<u32>,
    /// `start`: most samples the DLL keeps before the oldest go.
    #[serde(default)]
    pub cap: Option<u32>,
    /// `read`: return the samples with index at or after this one.
    #[serde(default)]
    pub since: Option<u64>,
    /// `read`: most samples to return in one call.
    #[serde(default)]
    pub max: Option<u32>,
}

#[tool_router(router = watch_router, vis = "pub(super)")]
impl LabServer {
    #[tool(
        description = "Start, read or stop the lab DLL's per-frame watch (pose, rotation, hover, controller mode, buttons, game time). read returns the samples since an index and keeps them for lab_detectors_read. Returns the DLL result plus pulled; no evidence (a read)."
    )]
    async fn client_watch(
        &self,
        Parameters(a): Parameters<WatchArgs>,
    ) -> Result<CallToolResult, McpError> {
        let mut params = Map::new();
        if let Some(v) = a.period_frames {
            params.insert("period_frames".into(), json!(v));
        }
        if let Some(v) = a.cap {
            params.insert("cap".into(), json!(v));
        }
        if let Some(v) = a.since {
            params.insert("since".into(), json!(v));
        }
        if let Some(v) = a.max {
            params.insert("max".into(), json!(v));
        }
        let out = self
            .supervisor
            .watch(&a.action, Value::Object(params))
            .await;
        Ok(match out {
            Ok(v) => CallToolResult::success(vec![ContentBlock::text(v.to_string())]),
            Err(e) => {
                CallToolResult::error(vec![ContentBlock::text(json!({ "error": e }).to_string())])
            }
        })
    }
}
