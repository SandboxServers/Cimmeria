//! The stdio MCP server surface.
//!
//! Phase 1 (#684): three probe tools that proxy to the client bridge
//! (`client_lua_eval`, `client_module_info`, `client_mem_read`).
//! Phase 2 (#685): the supervisor tools that own the SGW.exe process
//! lifecycle — start/stop/restart/status, autologin, screenshot, and
//! crash reporting.
//!
//! All bridge traffic goes through the [`Supervisor`], which journals
//! probe commands (for crash quarantine) and re-points the bridge at
//! the per-launch token when it starts a client.

use std::sync::Arc;

use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::*,
    schemars, tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler,
};
use serde_json::{json, Value};

use crate::supervisor::Supervisor;
use crate::timeline::{Timeline, TimelineArgs};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct LuaEvalArgs {
    /// Lua source to run on the client's main thread. UTF-16LE
    /// encoding and character-length are handled inside the bridge.
    pub chunk: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct MemReadArgs {
    /// Start address as hex (`"0x416ec0"`) or decimal. Ghidra VAs
    /// should be adjusted by the ASLR slide from `client_module_info`.
    pub addr: String,
    /// Number of bytes to read. VirtualQuery-guarded by the bridge;
    /// capped at 64 KiB.
    pub len: u32,
}

/// Optional target-server selector for the lifecycle tools. Overrides
/// the `server` field in `lab-account.json` when present (local vs colo).
#[derive(Debug, Default, serde::Deserialize, schemars::JsonSchema)]
pub struct ServerArg {
    /// Shard name to select at login (e.g. `"local"`, `"colo"`). Falls
    /// back to `lab-account.json`'s `server` when omitted.
    #[serde(default)]
    pub server: Option<String>,
}

/// Arguments for `lab_timeline`.
#[derive(Debug, Default, serde::Deserialize, schemars::JsonSchema)]
pub struct TimelineToolArgs {
    /// Server session whose packet tap to merge in. Omit for a
    /// client-only (heartbeat) timeline.
    #[serde(default)]
    pub session_id: Option<String>,
    /// Lookback window in milliseconds, ending at the newest event.
    /// Defaults to 60000.
    #[serde(default)]
    pub window_ms: Option<i64>,
    /// Max packet-tap rows to request. Defaults to 1000.
    #[serde(default)]
    pub limit: Option<u32>,
    /// Explicit server-clock lower bound handed to the packet-tap read.
    #[serde(default)]
    pub since_ms: Option<i64>,
}

/// The MCP server. Holds the shared supervisor (which owns the bridge
/// client and the process lifecycle) and the timeline builder.
#[derive(Clone)]
pub struct LabServer {
    supervisor: Arc<Supervisor>,
    timeline: Arc<Timeline>,
    tool_router: ToolRouter<LabServer>,
}

#[tool_router(router = tool_router)]
impl LabServer {
    pub fn new(supervisor: Arc<Supervisor>) -> Self {
        Self {
            supervisor,
            timeline: Arc::new(Timeline::from_env()),
            tool_router: Self::tool_router(),
        }
    }

    // ---- Phase 1: probe tools (proxied + journaled) ------------------

    #[tool(
        description = "Evaluate a Lua chunk on the live SGW client's main thread via the client bridge. Returns the pcall status, any Lua error, results, and captured print output."
    )]
    async fn client_lua_eval(
        &self,
        Parameters(args): Parameters<LuaEvalArgs>,
    ) -> Result<CallToolResult, McpError> {
        self.proxy("lua_eval", json!({ "chunk": args.chunk })).await
    }

    #[tool(
        description = "Report the live client's image base, PE preferred base, and ASLR slide. Add the slide to a Ghidra VA to reach the runtime address."
    )]
    async fn client_module_info(&self) -> Result<CallToolResult, McpError> {
        self.proxy("module_info", json!({})).await
    }

    #[tool(
        description = "Read client process memory (VirtualQuery-guarded, never faults). Returns the bytes as a hex string. Max 64 KiB."
    )]
    async fn client_mem_read(
        &self,
        Parameters(args): Parameters<MemReadArgs>,
    ) -> Result<CallToolResult, McpError> {
        self.proxy("mem_read", json!({ "addr": args.addr, "len": args.len }))
            .await
    }

    // ---- Phase 2: supervisor lifecycle tools -------------------------

    #[tool(
        description = "Launch SGW.exe suspended, inject the lab-bridge telemetry DLL, and resume. Writes a fresh per-launch token into current-session.json and starts the heartbeat watchdog. Optional `server` selects local vs colo."
    )]
    async fn lab_client_start(
        &self,
        Parameters(args): Parameters<ServerArg>,
    ) -> Result<CallToolResult, McpError> {
        self.wrap(self.supervisor.start(args.server).await)
    }

    #[tool(description = "Terminate the supervised SGW.exe client.")]
    async fn lab_client_stop(&self) -> Result<CallToolResult, McpError> {
        self.wrap(self.supervisor.stop().await)
    }

    #[tool(
        description = "Stop the client (if running) and start a fresh one. Optional `server` selects local vs colo."
    )]
    async fn lab_client_restart(
        &self,
        Parameters(args): Parameters<ServerArg>,
    ) -> Result<CallToolResult, McpError> {
        self.wrap(self.supervisor.restart(args.server).await)
    }

    #[tool(
        description = "Report the client's PID, uptime, bridge heartbeat (tick count + age + alive/stale/unreachable), login state, and recent crash count."
    )]
    async fn lab_client_status(&self) -> Result<CallToolResult, McpError> {
        self.wrap(self.supervisor.status().await)
    }

    #[tool(
        description = "Drive Lua autologin (EULA/login/server-select/character-select) to enter the world on the lab character. NOTE: the screen reads need client_lua_eval return-value capture (a Phase-3 bridge TODO); the fire-and-forget actions work today."
    )]
    async fn lab_login(
        &self,
        Parameters(args): Parameters<ServerArg>,
    ) -> Result<CallToolResult, McpError> {
        self.wrap(self.supervisor.login(args.server).await)
    }

    #[tool(description = "Capture the client's main window and return it as a PNG image.")]
    async fn lab_screenshot(&self) -> Result<CallToolResult, McpError> {
        match self.supervisor.screenshot().await {
            Ok((b64, w, h)) => Ok(CallToolResult::success(vec![
                ContentBlock::text(format!("client window {w}x{h}")),
                ContentBlock::image(b64, "image/png"),
            ])),
            Err(e) => Err(McpError::internal_error(format!("screenshot: {e}"), None)),
        }
    }

    #[tool(
        description = "Report the last minidump path, the last N bridge commands, the command quarantined at crash time, and the DLL crash marker."
    )]
    async fn lab_crash_report(&self) -> Result<CallToolResult, McpError> {
        self.wrap(self.supervisor.crash_report().await)
    }

    #[tool(
        description = "Merge local client events (bridge heartbeat ring today; the full client-event ring is gated on #686) with server packet-tap rows (fetched from cimmeria-lab-mcp over HTTP) into one time-ordered window. Estimates the client↔server clock offset from the packet-tap round trip and projects client events onto the server clock. SigNoz stays the durable copy under the dev-session id. Client-only when no session_id or no server endpoint is configured."
    )]
    async fn lab_timeline(
        &self,
        Parameters(args): Parameters<TimelineToolArgs>,
    ) -> Result<CallToolResult, McpError> {
        let ta = TimelineArgs {
            session_id: args.session_id,
            window_ms: args.window_ms,
            limit: args.limit,
            since_ms: args.since_ms,
        };
        self.wrap(self.timeline.build(&self.supervisor, ta).await)
    }
}

impl LabServer {
    /// Forward one probe call through the supervisor (journaled) and wrap
    /// the JSON result as MCP text content.
    async fn proxy(&self, method: &str, params: Value) -> Result<CallToolResult, McpError> {
        self.wrap(self.supervisor.bridge_call(method, params).await)
    }

    /// Wrap a supervisor `Result<Value, String>` as an MCP result.
    fn wrap(&self, r: Result<Value, String>) -> Result<CallToolResult, McpError> {
        match r {
            Ok(result) => {
                let text =
                    serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string());
                Ok(CallToolResult::success(vec![ContentBlock::text(text)]))
            }
            Err(e) => Err(McpError::internal_error(e, None)),
        }
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for LabServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::from_build_env())
            .with_instructions(
                "Live Research Lab supervisor. Probe tools (client_lua_eval, \
                 client_module_info, client_mem_read) forward to the injected \
                 cimmeria-client-telemetry DLL over a token-gated loopback TCP \
                 channel. Supervisor tools (lab_client_start/stop/restart/status, \
                 lab_login, lab_screenshot, lab_crash_report) own the SGW.exe \
                 process lifecycle and crash recovery. lab_timeline merges \
                 local client events with server packet-tap rows (from \
                 cimmeria-lab-mcp over HTTP) into one clock-aligned window."
                    .to_string(),
            )
    }
}
