//! The DLL watch tool (`client_watch`): start, read or stop the per-frame
//! sample stream. Orchestration lives in `supervisor::watch_store`.

use rmcp::{
    handler::server::wrapper::Parameters, model::*, schemars, tool, tool_router,
    ErrorData as McpError,
};
use serde_json::{json, Map, Value};

use super::LabServer;
use crate::supervisor::detect;
use crate::supervisor::now_ms;

/// The window `lab_detectors_read` looks back over when `since_ms` is omitted.
const DEFAULT_SINCE_MS: u64 = 120_000;

/// Args for `lab_detectors_read`. `instance` and `lease_id` are routed by the
/// handler, so they need no field here.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct DetectArgs {
    /// How far back to look, in ms (default 120000).
    #[serde(default)]
    pub since_ms: Option<u64>,
}

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

    #[tool(
        description = "Run the lab detectors (hitch, teleport, long_fall, out_of_world, nan, stuck_input, actor_growth, log_error) over what the daemon already holds: client_watch samples, the heartbeat history, held keys and recent tool calls. No client call; watch is false when no samples were pulled, and then only hitch and log_error run."
    )]
    async fn lab_detectors_read(
        &self,
        Parameters(a): Parameters<DetectArgs>,
    ) -> Result<CallToolResult, McpError> {
        let window_ms = a.since_ms.unwrap_or(DEFAULT_SINCE_MS);
        let now = now_ms();
        let from = now.saturating_sub(i64::try_from(window_ms).unwrap_or(i64::MAX));
        let (_, samples) = self.supervisor.watch_samples_since(from).await;
        let heartbeats: Vec<_> = self
            .supervisor
            .heartbeat_samples()
            .await
            .into_iter()
            .filter(|b| b.observed_ms >= from)
            .collect();
        let holds: Vec<(String, i64)> = self
            .supervisor
            .hold_starts()
            .await
            .into_iter()
            .map(|(key, since)| (key, since as i64))
            .collect();
        let calls = detect::calls::rows_since(self.supervisor.label(), from);
        let fired = detect::run_all(&detect::Inputs {
            samples: &samples,
            heartbeats: &heartbeats,
            holds: &holds,
            calls: &calls,
            now_ms: now,
        });
        let body = json!({
            "watch": !samples.is_empty(),
            "window_ms": window_ms,
            "fired": fired,
        });
        Ok(CallToolResult::success(vec![ContentBlock::text(
            body.to_string(),
        )]))
    }
}
