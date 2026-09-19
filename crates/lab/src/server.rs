//! The stdio MCP server surface: three tools that proxy to the
//! client bridge. Phase 1 of the Live Research Lab supervisor.
//!
//! Each tool is a thin translator — one MCP tool call becomes one
//! framed JSON-RPC request against the bridge — matching the ADR's
//! "the supervisor proxies tool calls to the bridge."

use std::sync::Arc;

use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::*,
    schemars, tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler,
};
use serde_json::{json, Value};

use crate::client::BridgeClient;

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

/// The MCP server. Holds a shared bridge client; cloned per request
/// by the rmcp router (cheap — the bridge lives behind an `Arc`).
#[derive(Clone)]
pub struct LabServer {
    bridge: Arc<BridgeClient>,
    tool_router: ToolRouter<LabServer>,
}

#[tool_router(router = tool_router)]
impl LabServer {
    pub fn new(bridge: Arc<BridgeClient>) -> Self {
        Self {
            bridge,
            tool_router: Self::tool_router(),
        }
    }

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
}

impl LabServer {
    /// Forward one call to the bridge and wrap the JSON result as MCP
    /// text content. A bridge/transport failure becomes an MCP
    /// internal error so the agent sees the message.
    async fn proxy(&self, method: &str, params: Value) -> Result<CallToolResult, McpError> {
        match self.bridge.call(method, params).await {
            Ok(result) => {
                let text =
                    serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string());
                Ok(CallToolResult::success(vec![ContentBlock::text(text)]))
            }
            Err(e) => Err(McpError::internal_error(format!("bridge: {e}"), None)),
        }
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for LabServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::from_build_env())
            .with_instructions(
                "Live Research Lab client bridge proxy. Tools: client_lua_eval, \
                 client_module_info, client_mem_read. All calls are forwarded to the \
                 injected cimmeria-client-telemetry DLL over a token-gated loopback \
                 TCP channel."
                    .to_string(),
            )
    }
}
