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

/// Args for `client_mem_write` — either `hex` (raw bytes) or
/// `value` + `type` (typed little-endian). See the bridge's `mem_write`.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct MemWriteArgs {
    /// Target address (hex or decimal; apply the ASLR slide to a Ghidra VA).
    pub addr: String,
    /// Raw bytes as hex, no separators (mutually exclusive with `value`).
    #[serde(default)]
    pub hex: Option<String>,
    /// Typed value encoded little-endian (needs `type`).
    #[serde(default)]
    pub value: Option<serde_json::Value>,
    /// Width for `value`: u8/i8/u16/…/f64.
    #[serde(default, rename = "type")]
    pub ty: Option<String>,
}

/// Args for `client_call_native`.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CallNativeArgs {
    /// Function address (hex or decimal).
    pub addr: String,
    /// Calling convention: cdecl (default), stdcall, thiscall, fastcall.
    #[serde(default)]
    pub conv: Option<String>,
    /// Positional args (numbers or hex strings). For thiscall the first is
    /// `this`; for fastcall the first two are ECX/EDX.
    #[serde(default)]
    pub args: Vec<serde_json::Value>,
    /// Return interpretation: u32 (default), i32, void, f32, f64.
    #[serde(default)]
    pub ret: Option<String>,
}

/// Args for `client_hook_install`.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct HookInstallArgs {
    /// Function-entry address to hook (hex or decimal). Phase-3 native
    /// hooks are cdecl-only.
    pub addr: String,
    /// Calling convention (default cdecl; only cdecl is patchable today).
    #[serde(default)]
    pub conv: Option<String>,
    /// Re-apply this hook automatically after a crash.
    #[serde(default)]
    pub persistent: bool,
    /// Capture spec object: `registers`, `stack_args`, `derefs`,
    /// `hit_limit`, `sample_rate`.
    #[serde(default)]
    pub capture: Option<serde_json::Value>,
}

/// Args for `client_hook_remove`.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct HookRemoveArgs {
    /// Hook id returned by `client_hook_install`.
    pub id: u32,
}

/// Args for `client_events_read`.
#[derive(Debug, Default, serde::Deserialize, schemars::JsonSchema)]
pub struct EventsReadArgs {
    /// Max events to drain this call (default 512, capped at the ring).
    #[serde(default)]
    pub max: Option<u32>,
}

/// Args for `client_console`.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ConsoleArgs {
    /// Command line to submit (e.g. `"/who"`).
    pub line: String,
    /// String-taking console-exec address (from Ghidra; required until a
    /// built-in one is confirmed — see the bridge's `console`).
    #[serde(default)]
    pub addr: Option<String>,
    /// `this` object pointer for a thiscall target.
    #[serde(default)]
    pub this: Option<String>,
    /// Whether the target expects a wide (UTF-16) string (default true).
    #[serde(default)]
    pub wide: Option<bool>,
    /// Calling convention: thiscall (default), cdecl, stdcall.
    #[serde(default)]
    pub conv: Option<String>,
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

/// The MCP server. Holds the shared supervisor (which owns the bridge
/// client and the process lifecycle).
#[derive(Clone)]
pub struct LabServer {
    supervisor: Arc<Supervisor>,
    tool_router: ToolRouter<LabServer>,
}

#[tool_router(router = tool_router)]
impl LabServer {
    pub fn new(supervisor: Arc<Supervisor>) -> Self {
        Self {
            supervisor,
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

    // ---- Phase 3: native probes (proxied + journaled) ----------------

    #[tool(
        description = "Write client process memory (VirtualProtect round-trip, exception-guarded, journaled). Provide `hex` for raw bytes, or `value` + `type` for a typed little-endian write. Never replayed after a crash."
    )]
    async fn client_mem_write(
        &self,
        Parameters(args): Parameters<MemWriteArgs>,
    ) -> Result<CallToolResult, McpError> {
        let mut params = json!({ "addr": args.addr });
        if let Some(hex) = args.hex {
            params["hex"] = json!(hex);
        }
        if let Some(value) = args.value {
            params["value"] = value;
        }
        if let Some(ty) = args.ty {
            params["type"] = json!(ty);
        }
        self.proxy("mem_write", params).await
    }

    #[tool(
        description = "Call a client function by address with a stated calling convention (cdecl/stdcall/thiscall/fastcall), on the main thread, exception-guarded and journaled. Never replayed after a crash."
    )]
    async fn client_call_native(
        &self,
        Parameters(args): Parameters<CallNativeArgs>,
    ) -> Result<CallToolResult, McpError> {
        let mut params = json!({ "addr": args.addr, "args": args.args });
        if let Some(conv) = args.conv {
            params["conv"] = json!(conv);
        }
        if let Some(ret) = args.ret {
            params["ret"] = json!(ret);
        }
        self.proxy("call_native", params).await
    }

    #[tool(
        description = "Install a non-freezing logging hook at a function entry with a capture spec (stack args, typed dereferences, hit limit, sample rate). `persistent: true` re-applies it after a crash. Returns the hook id. Phase-3 native hooks are cdecl, function-entry only."
    )]
    async fn client_hook_install(
        &self,
        Parameters(args): Parameters<HookInstallArgs>,
    ) -> Result<CallToolResult, McpError> {
        let mut params = json!({ "addr": args.addr, "persistent": args.persistent });
        if let Some(conv) = args.conv {
            params["conv"] = json!(conv);
        }
        if let Some(capture) = args.capture {
            params["capture"] = capture;
        }
        self.proxy("hook_install", params).await
    }

    #[tool(description = "Remove a dynamic hook by id (restores the patched bytes).")]
    async fn client_hook_remove(
        &self,
        Parameters(args): Parameters<HookRemoveArgs>,
    ) -> Result<CallToolResult, McpError> {
        self.proxy("hook_remove", json!({ "id": args.id })).await
    }

    #[tool(
        description = "List installed dynamic hooks: id, address, convention, persistent flag, hit count, and capture spec."
    )]
    async fn client_hook_list(&self) -> Result<CallToolResult, McpError> {
        self.proxy("hook_list", json!({})).await
    }

    #[tool(
        description = "Drain the client's local event ring: hook hits, Lua prints, and Mercury dispatch events (the same events also upload to SigNoz). Returns the events plus a dropped-since-last-read count."
    )]
    async fn client_events_read(
        &self,
        Parameters(args): Parameters<EventsReadArgs>,
    ) -> Result<CallToolResult, McpError> {
        let params = match args.max {
            Some(max) => json!({ "max": max }),
            None => json!({}),
        };
        self.proxy("events_read", params).await
    }

    #[tool(
        description = "Submit a native slash command through the client's console-exec path, exception-guarded. Supply `addr` (and `this` for a thiscall target) from Ghidra until a built-in console-exec address is confirmed."
    )]
    async fn client_console(
        &self,
        Parameters(args): Parameters<ConsoleArgs>,
    ) -> Result<CallToolResult, McpError> {
        let mut params = json!({ "line": args.line });
        if let Some(addr) = args.addr {
            params["addr"] = json!(addr);
        }
        if let Some(this) = args.this {
            params["this"] = json!(this);
        }
        if let Some(wide) = args.wide {
            params["wide"] = json!(wide);
        }
        if let Some(conv) = args.conv {
            params["conv"] = json!(conv);
        }
        self.proxy("console", params).await
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
                 process lifecycle and crash recovery."
                    .to_string(),
            )
    }
}
