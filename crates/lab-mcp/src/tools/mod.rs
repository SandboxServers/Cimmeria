//! The lab MCP tool set (v1) — the rmcp adapter.
//!
//! Each tool is a thin wrapper over the logic in the sibling modules; the
//! adapter's only jobs are argument deserialization, wrapping the result as
//! MCP content, and emitting the one [`crate::audit`] event per call. The tool
//! set is fixed: no dynamic registration, no eval, no filesystem access.

mod console;
mod content;
mod db;
mod logs;
mod sessions;

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock, ServerCapabilities, ServerConfig};
use rmcp::schemars::{self, JsonSchema};
use rmcp::{tool, tool_handler, tool_router, ErrorData, ServerHandler};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::audit::{self, OUTCOME_ERROR, OUTCOME_OK};
use crate::state::LabState;

/// The MCP server handler holding the shared [`LabState`] and the generated
/// tool router.
#[derive(Clone)]
pub struct LabTools {
    state: LabState,
    tool_router: ToolRouter<LabTools>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ConsoleExecArgs {
    /// Acting entity id — the agent's own logged-in lab character. The GM
    /// access-level gate is applied to this entity cell-side.
    entity_id: u32,
    /// The `.`-console line to run (the leading `.` is optional).
    line: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct LogTailArgs {
    /// Max number of recent lines to return (default 100).
    #[serde(default)]
    limit: Option<usize>,
    /// Optional level filter, case-insensitive (e.g. "warn").
    #[serde(default)]
    level: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct DbQueryArgs {
    /// A single read-only SQL statement. Writes are rejected; results are
    /// capped at 500 rows.
    sql: String,
}

#[tool_router]
impl LabTools {
    pub fn new(state: LabState) -> Self {
        Self {
            state,
            tool_router: Self::tool_router(),
        }
    }

    /// Enumerate every `.`-console command with its arg/target contract.
    #[tool(description = "List all server dot-console commands (name, arg counts, target, help).")]
    async fn server_console_list(&self) -> Result<CallToolResult, ErrorData> {
        let v = console::list_commands();
        audit::emit("server_console_list", &json!({}), OUTCOME_OK);
        ok(v)
    }

    /// Run a dot-console line as a given entity and return the captured output.
    #[tool(
        description = "Run a server dot-console command line as the given entity_id (must be a GameMaster) and return its captured output."
    )]
    async fn server_console_exec(
        &self,
        Parameters(args): Parameters<ConsoleExecArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let audit_args = json!({ "entity_id": args.entity_id, "line": args.line });
        match console::exec_console(&self.state, args.entity_id, args.line.clone()).await {
            Ok(v) => {
                audit::emit("server_console_exec", &audit_args, OUTCOME_OK);
                ok(v)
            }
            Err(e) => {
                audit::emit("server_console_exec", &audit_args, OUTCOME_ERROR);
                err(e)
            }
        }
    }

    /// List connected accounts/characters, their entity ids, and spaces.
    #[tool(
        description = "List connected players: entity id, name, archetype, level, zone, status."
    )]
    async fn server_sessions(&self) -> Result<CallToolResult, ErrorData> {
        let v = sessions::list_sessions(&self.state).await;
        audit::emit("server_sessions", &json!({}), OUTCOME_OK);
        ok(v)
    }

    /// Read the most recent buffered server log lines.
    #[tool(description = "Return the most recent server log lines, optionally filtered by level.")]
    async fn server_log_tail(
        &self,
        Parameters(args): Parameters<LogTailArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let limit = args.limit.unwrap_or(logs::DEFAULT_LIMIT);
        let audit_args = json!({ "limit": limit, "level": args.level });
        let v = logs::log_tail(&self.state, limit, args.level.as_deref());
        audit::emit("server_log_tail", &audit_args, OUTCOME_OK);
        ok(v)
    }

    /// Hot-reload the content engine from the database.
    #[tool(description = "Trigger a content-engine hot reload from the database.")]
    async fn server_content_reload(&self) -> Result<CallToolResult, ErrorData> {
        match content::reload_content(&self.state).await {
            Ok(v) => {
                audit::emit("server_content_reload", &json!({}), OUTCOME_OK);
                ok(v)
            }
            Err(e) => {
                audit::emit("server_content_reload", &json!({}), OUTCOME_ERROR);
                err(e)
            }
        }
    }

    /// Run one read-only SQL statement (row-capped) and return the rows.
    #[tool(
        description = "Run ONE read-only SQL statement in a read-only transaction (writes rejected, max 500 rows) and return the rows as JSON."
    )]
    async fn server_db_query(
        &self,
        Parameters(args): Parameters<DbQueryArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let audit_args = json!({ "sql": args.sql });
        match db::db_query(&self.state, args.sql.clone()).await {
            Ok(v) => {
                audit::emit("server_db_query", &audit_args, OUTCOME_OK);
                ok(v)
            }
            Err(e) => {
                audit::emit("server_db_query", &audit_args, OUTCOME_ERROR);
                err(e)
            }
        }
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for LabTools {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build()).with_instructions(
            "Cimmeria live-research-lab control endpoint. Fixed tool set: \
             inspect and drive a running SGW server (console, sessions, logs, \
             content reload, read-only SQL).",
        )
    }
}

/// Wrap a JSON value as a successful tool result.
fn ok(v: Value) -> Result<CallToolResult, ErrorData> {
    Ok(CallToolResult::success(vec![ContentBlock::json(v)?]))
}

/// Wrap an error message as a tool-level error result (visible to the model,
/// not a protocol error).
fn err(msg: String) -> Result<CallToolResult, ErrorData> {
    Ok(CallToolResult::error(vec![ContentBlock::text(msg)]))
}
