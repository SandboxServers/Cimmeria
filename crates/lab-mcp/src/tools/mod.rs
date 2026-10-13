//! The lab MCP tool set (v1) — the rmcp adapter.
//!
//! Each tool is a thin wrapper over the logic in the sibling modules; the
//! adapter's only jobs are argument deserialization, wrapping the result as
//! MCP content, and emitting the one [`crate::audit`] event per call. The tool
//! set is fixed: no dynamic registration, no eval, no filesystem access.

mod abilities;
// The chaos tools' `account` lookup (CH-14); in tests too, so the ordinary
// live-DB tier runs its query.
#[cfg(any(test, feature = "lab-chaos"))]
mod account_lookup;
// The lab chaos tools (CH-05) exist only in a `lab-chaos` build.
#[cfg(feature = "lab-chaos")]
mod chaos;
#[cfg(feature = "lab-chaos")]
mod chaos_args;
#[cfg(all(test, feature = "lab-chaos"))]
#[path = "chaos_args_tests.rs"]
mod chaos_args_tests;
#[cfg(all(test, feature = "lab-chaos"))]
#[path = "chaos_arm_tests.rs"]
mod chaos_arm_tests;
#[cfg(test)]
#[path = "chaos_tests.rs"]
mod chaos_tests;
mod console;
mod content;
mod db;
mod entities;
mod fixture;
mod logs;
mod names;
mod packet_tap;
mod sessions;
mod witnesses;

use std::time::Instant;

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
    /// Optional exact tracing target (e.g. "aoi.cinematic_hold").
    #[serde(default)]
    target: Option<String>,
    /// Optional case-sensitive substring of the message.
    #[serde(default)]
    contains: Option<String>,
    /// Optional: only entries with `timestamp_ms` strictly greater than this.
    #[serde(default)]
    since_ms: Option<u64>,
    /// Optional: also return the client telemetry the server replays into
    /// its log (targets client.native, launcher.client_log, ...). Default
    /// false.
    #[serde(default)]
    include_client: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct DbQueryArgs {
    /// A single read-only SQL statement. Writes are rejected; results are
    /// capped at 500 rows.
    sql: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct EntityGetArgs {
    /// Runtime entity id to snapshot.
    entity_id: u32,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct EntityQueryArgs {
    /// Restrict to a single space instance id.
    #[serde(default)]
    space_id: Option<u32>,
    /// Restrict to entities of this `entity_templates.template_id`.
    #[serde(default)]
    template_id: Option<i32>,
    /// Restrict to a wire class id (2 = SGWPlayer, 4 = SGWMob).
    #[serde(default)]
    class_id: Option<u8>,
    /// Restrict to entities with this spawn tag (`spawnlist.tag`), exact.
    #[serde(default)]
    tag: Option<String>,
    /// Radius in world units. Requires a center: `around_entity` or `around_point`.
    #[serde(default)]
    radius: Option<f32>,
    /// Center the radius on this entity's current position.
    #[serde(default)]
    around_entity: Option<u32>,
    /// Center the radius on this explicit `[x, y, z]` world point.
    #[serde(default)]
    around_point: Option<[f32; 3]>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct AbilityStateArgs {
    /// Runtime entity id (player or NPC) whose ability state to snapshot.
    entity_id: u32,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct WitnessesArgs {
    /// Entity whose bidirectional witness relationship to report.
    entity_id: u32,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct PacketTapStartArgs {
    /// The session to tap, named by its in-world player entity id.
    entity_id: u32,
    /// Ring capacity (messages). Clamped to [1, 10000]; default 500.
    #[serde(default)]
    capacity: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct PacketTapArgs {
    /// The tapped session's player entity id.
    entity_id: u32,
}

#[tool_router]
impl LabTools {
    pub fn new(state: LabState) -> Self {
        let tool_router = Self::tool_router();
        #[cfg(feature = "lab-chaos")]
        let tool_router = tool_router + Self::chaos_router();
        Self { state, tool_router }
    }

    /// Enumerate every `.`-console command with its arg/target contract.
    #[tool(description = "List all server dot-console commands (name, arg counts, target, help).")]
    async fn server_console_list(&self) -> Result<CallToolResult, ErrorData> {
        let started = Instant::now();
        let v = console::list_commands();
        audit::emit("server_console_list", &json!({}), OUTCOME_OK, None, started);
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
        let started = Instant::now();
        let audit_args = json!({ "entity_id": args.entity_id, "line": args.line });
        match console::exec_console(&self.state, args.entity_id, args.line.clone()).await {
            Ok(v) => {
                audit::emit(
                    "server_console_exec",
                    &audit_args,
                    OUTCOME_OK,
                    None,
                    started,
                );
                ok(v)
            }
            Err(e) => {
                audit::emit(
                    "server_console_exec",
                    &audit_args,
                    OUTCOME_ERROR,
                    Some(e.as_str()),
                    started,
                );
                err(e)
            }
        }
    }

    /// List connected accounts/characters, their entity ids, and spaces.
    #[tool(
        description = "List connected players: entity id, name, archetype, level, zone, status."
    )]
    async fn server_sessions(&self) -> Result<CallToolResult, ErrorData> {
        let started = Instant::now();
        let v = sessions::list_sessions(&self.state).await;
        audit::emit("server_sessions", &json!({}), OUTCOME_OK, None, started);
        ok(v)
    }

    /// Read the most recent buffered server log lines.
    #[tool(
        description = "Return the most recent server log lines, optionally filtered by level, target, message text and since_ms; newest_ms is the buffer's newest timestamp. Forwarded client telemetry (client.native, launcher.client_log, ...) is left out unless include_client is true."
    )]
    async fn server_log_tail(
        &self,
        Parameters(args): Parameters<LogTailArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let started = Instant::now();
        let limit = args.limit.unwrap_or(logs::DEFAULT_LIMIT);
        let audit_args = json!({
            "limit": limit,
            "level": args.level,
            "target": args.target,
            "contains": args.contains,
            "since_ms": args.since_ms,
            "include_client": args.include_client,
        });
        let filter = logs::LogFilter {
            level: args.level,
            target: args.target,
            contains: args.contains,
            since_ms: args.since_ms,
            include_client: args.include_client.unwrap_or(false),
        };
        let v = logs::log_tail(&self.state, &filter, limit);
        audit::emit("server_log_tail", &audit_args, OUTCOME_OK, None, started);
        ok(v)
    }

    /// Hot-reload the content engine from the database.
    #[tool(description = "Trigger a content-engine hot reload from the database.")]
    async fn server_content_reload(&self) -> Result<CallToolResult, ErrorData> {
        let started = Instant::now();
        match content::reload_content(&self.state).await {
            Ok(v) => {
                audit::emit(
                    "server_content_reload",
                    &json!({}),
                    OUTCOME_OK,
                    None,
                    started,
                );
                ok(v)
            }
            Err(e) => {
                audit::emit(
                    "server_content_reload",
                    &json!({}),
                    OUTCOME_ERROR,
                    Some(e.as_str()),
                    started,
                );
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
        let started = Instant::now();
        let audit_args = json!({ "sql": args.sql });
        match db::db_query(&self.state, args.sql.clone()).await {
            Ok(v) => {
                audit::emit("server_db_query", &audit_args, OUTCOME_OK, None, started);
                ok(v)
            }
            Err(e) => {
                audit::emit(
                    "server_db_query",
                    &audit_args,
                    OUTCOME_ERROR,
                    Some(e.as_str()),
                    started,
                );
                err(e)
            }
        }
    }

    /// Stage a lab row fixture for an offline lab fixture character
    /// (lab-fixtures FX-04).
    #[tool(
        description = "Lab only: stage a row fixture (world, position, missions, exact item counts, tutorials to mark unseen) for a lab fixture character (last name Fx followed by letters) on a GM account that is offline or mid-login (a character whose world entry has not been sent yet counts as offline; a fixture staged then applies at that account's next Play within 10 minutes), named by its full name or the name's last word. Nothing is written now: the server applies it at that character's next Play, takes it once, and drops it unclaimed after 10 minutes. Staging again replaces it. Every refusal is an invalid_params error starting 'stage refused: '; a server database failure is an internal error."
    )]
    async fn server_stage_fixture(
        &self,
        Parameters(call): Parameters<fixture::StageFixtureCall>,
    ) -> Result<CallToolResult, ErrorData> {
        let started = Instant::now();
        let audit_args = call.0.clone();
        match fixture::stage_fixture(&self.state, call).await {
            Ok(v) => {
                audit::emit(
                    "server_stage_fixture",
                    &audit_args,
                    OUTCOME_OK,
                    None,
                    started,
                );
                ok(v)
            }
            Err(e) => {
                audit::emit(
                    "server_stage_fixture",
                    &audit_args,
                    OUTCOME_ERROR,
                    Some(e.message.as_str()),
                    started,
                );
                // The FX contract: a refusal is a protocol-level
                // invalid_params error, unlike the other tools' error results.
                // A database failure is the server's, not the caller's.
                if e.internal {
                    Err(ErrorData::internal_error(e.message, None))
                } else {
                    Err(ErrorData::invalid_params(e.message, None))
                }
            }
        }
    }

    /// Snapshot one live cell entity by id.
    #[tool(
        description = "Snapshot one live cell entity by id: position, class/faction, health, focus, every stat (cur/min/max), AI state, appearance, and witness count."
    )]
    async fn server_entity_get(
        &self,
        Parameters(args): Parameters<EntityGetArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let started = Instant::now();
        let audit_args = json!({ "entity_id": args.entity_id });
        emit_result(
            "server_entity_get",
            &audit_args,
            started,
            entities::entity_get(&self.state, args.entity_id).await,
        )
    }

    /// Query live cell entities, filtered by space / template / class / tag / radius.
    #[tool(
        description = "Query live cell entities filtered by space_id, template_id, class_id (2=player, 4=NPC), tag (spawn tag, exact), and/or a radius around an entity or point. Capped at 256 snapshots; reports total_matched and capped."
    )]
    async fn server_entity_query(
        &self,
        Parameters(args): Parameters<EntityQueryArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let started = Instant::now();
        let audit_args = json!({
            "space_id": args.space_id,
            "template_id": args.template_id,
            "class_id": args.class_id,
            "tag": args.tag,
            "radius": args.radius,
            "around_entity": args.around_entity,
            "around_point": args.around_point,
        });
        emit_result(
            "server_entity_query",
            &audit_args,
            started,
            entities::entity_query(
                &self.state,
                args.space_id,
                args.template_id,
                args.class_id,
                args.tag,
                args.radius,
                args.around_entity,
                args.around_point,
            )
            .await,
        )
    }

    /// One entity's ability state (AB-L1 over AB-T5's snapshot).
    #[tool(
        description = "Snapshot one live entity's ability state: the cast in its warmup (ability, cast_id, time left), running ability and moniker cooldowns with time left, pulsing effects (effect, invoker, cast_id, pulses left), timed and held effect-ledger entries (stat deltas, expiry or held, monikers, flags, absorb pools), owed effect-icon clears, state_field refcounts per bit, and every stat's cur/min/max. Times are seconds from now. Read-only; state is null for an unknown entity."
    )]
    async fn server_ability_state(
        &self,
        Parameters(args): Parameters<AbilityStateArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let started = Instant::now();
        let audit_args = json!({ "entity_id": args.entity_id });
        emit_result(
            "server_ability_state",
            &audit_args,
            started,
            abilities::ability_state(&self.state, args.entity_id).await,
        )
    }

    /// Read the generated Livewire board for one active player session.
    #[tool(
        description = "Read-only active Livewire board snapshot by player entity id: goal counts, wire names, depths, positions and cut state. No ticket or control capability is returned; state is null before generation or after teardown."
    )]
    async fn server_minigame_state(
        &self,
        Parameters(args): Parameters<EntityGetArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let started = Instant::now();
        let audit_args = json!({ "entity_id": args.entity_id });
        let board = self.state.minigame_board(args.entity_id).await;
        emit_result(
            "server_minigame_state",
            &audit_args,
            started,
            Ok(json!({ "entity_id": args.entity_id, "state": board })),
        )
    }

    /// Report who witnesses an entity and whom it witnesses (both directions).
    #[tool(
        description = "Report the bidirectional witness relationship for an entity: who has it in their AoI, and (for a player) whom it sees. Targets the invisible-corpse class of AoI bug."
    )]
    async fn server_witnesses(
        &self,
        Parameters(args): Parameters<WitnessesArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let started = Instant::now();
        let audit_args = json!({ "entity_id": args.entity_id });
        emit_result(
            "server_witnesses",
            &audit_args,
            started,
            witnesses::witnesses(&self.state, args.entity_id).await,
        )
    }

    /// Start a per-session decoded packet tap.
    #[tool(
        description = "Start capturing decoded Mercury messages (both directions) for ONE session, named by its in-world player entity id, into a bounded ring. Restarting clears the ring."
    )]
    async fn server_packet_tap_start(
        &self,
        Parameters(args): Parameters<PacketTapStartArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let started = Instant::now();
        let audit_args = json!({ "entity_id": args.entity_id, "capacity": args.capacity });
        emit_result(
            "server_packet_tap_start",
            &audit_args,
            started,
            packet_tap::tap_start(&self.state, args.entity_id, args.capacity).await,
        )
    }

    /// Drain a packet tap's ring (messages + dropped count).
    #[tool(
        description = "Drain the packet tap for a session: return captured decoded messages (oldest first) and the count dropped since the last read, then clear the ring."
    )]
    async fn server_packet_tap_read(
        &self,
        Parameters(args): Parameters<PacketTapArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let started = Instant::now();
        let audit_args = json!({ "entity_id": args.entity_id });
        emit_result(
            "server_packet_tap_read",
            &audit_args,
            started,
            packet_tap::tap_read(args.entity_id),
        )
    }

    /// Stop a packet tap and discard its ring.
    #[tool(description = "Stop the packet tap for a session and discard its ring buffer.")]
    async fn server_packet_tap_stop(
        &self,
        Parameters(args): Parameters<PacketTapArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let started = Instant::now();
        let audit_args = json!({ "entity_id": args.entity_id });
        emit_result(
            "server_packet_tap_stop",
            &audit_args,
            started,
            packet_tap::tap_stop(args.entity_id),
        )
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for LabTools {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build()).with_instructions(
            "Cimmeria live-research-lab control endpoint. Fixed tool set: \
             inspect and drive a running SGW server (console, sessions, logs, \
             content reload, read-only SQL), stage a lab fixture for an \
             offline lab fixture character (server_stage_fixture), snapshot live \
             entities and witness relationships (entity_get/entity_query/witnesses), \
             read one entity's ability state (ability_state), and capture \
             decoded per-session Mercury traffic (packet_tap_start/read/stop).",
        )
    }
}

/// Emit the one audit event for a tool call and wrap its `Result<Value, String>`
/// into an MCP result — `OUTCOME_OK` + JSON on success, `OUTCOME_ERROR` + a
/// model-visible error on failure. Collapses the repeated match arm the tools
/// share.
fn emit_result(
    tool: &str,
    audit_args: &Value,
    started: Instant,
    result: Result<Value, String>,
) -> Result<CallToolResult, ErrorData> {
    match result {
        Ok(v) => {
            audit::emit(tool, audit_args, OUTCOME_OK, None, started);
            ok(v)
        }
        Err(e) => {
            audit::emit(tool, audit_args, OUTCOME_ERROR, Some(e.as_str()), started);
            err(e)
        }
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use cimmeria_admin_api::ws::broadcast_layer::LogBuffer;
    use cimmeria_services::orchestrator::Orchestrator;
    use tracing::Level;

    use super::*;
    use crate::test_support::LogCapture;

    #[tokio::test]
    async fn minigame_state_tool_tracks_generated_board_update_and_teardown() {
        let orchestrator = Arc::new(Orchestrator::new(Default::default()));
        let tools = LabTools::new(LabState::new(orchestrator.clone(), LogBuffer::new()));
        assert!(tools.tool_router.has_route("server_minigame_state"));
        async fn read(tools: &LabTools) -> Value {
            let result = tools
                .server_minigame_state(Parameters(EntityGetArgs { entity_id: 42 }))
                .await
                .unwrap();
            serde_json::from_str(&result.content[0].as_text().unwrap().text).unwrap()
        }
        assert!(read(&tools).await["state"].is_null());
        let registry = orchestrator
            .state()
            .read()
            .await
            .base
            .minigame_registry
            .clone();
        let ticket = registry
            .register(
                42,
                1,
                "Livewire".into(),
                1,
                30,
                17,
                0,
                0,
                1,
                vec![],
                None,
                None,
            )
            .await
            .unwrap();
        let session = registry
            .authenticate_and_claim(42, &ticket, "Livewire")
            .await
            .unwrap();
        let mut game = cimmeria_services::minigame::game::create_game(&session).unwrap();
        game.started();
        registry
            .publish_board(42, &ticket, game.board_snapshot().unwrap())
            .await;
        let ready = read(&tools).await;
        assert_eq!(ready["state"]["state"], "ready");
        assert_eq!(ready["state"]["goal_total"], 2);
        assert!(ready["state"]["wires"].as_array().unwrap().len() > 2);
        assert!(ready["state"]["wires"][0]["lib"].is_string());
        assert!(!ready.to_string().contains(&ticket));
        game.message("opendoor", &Default::default());
        registry
            .publish_board(42, &ticket, game.board_snapshot().unwrap())
            .await;
        assert_eq!(read(&tools).await["state"]["state"], "running");
        game.aborted();
        registry
            .publish_board(42, &ticket, game.board_snapshot().unwrap())
            .await;
        registry
            .record_event(42, &ticket, "result_canceled", None)
            .await;
        registry.remove_if_ticket(42, &ticket).await;
        let final_state = read(&tools).await;
        assert_eq!(final_state["state"]["outcome"], "canceled");
        assert!(final_state["state"]["events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|event| event["event"] == "ended"));
    }

    /// A bad or failed SQL call leaves one WARN `lab.tool_call` row carrying
    /// `error` and `duration_ms`. No database is connected here, so the call
    /// fails on the pool check, which takes the same error path as bad SQL.
    #[tokio::test]
    async fn bad_sql_audit_row_has_error() {
        let capture = LogCapture::install();
        let orchestrator = Arc::new(Orchestrator::new(Default::default()));
        let tools = LabTools::new(LabState::new(orchestrator, LogBuffer::new()));
        let result = tools
            .server_db_query(Parameters(DbQueryArgs {
                sql: "SELEC nonsense".to_string(),
            }))
            .await;
        assert!(
            result.is_ok(),
            "a failed query is a tool-level error result"
        );

        let event = capture
            .find_message(Level::WARN, "lab MCP tool call")
            .expect("an error outcome logs at WARN");
        assert!(event
            .fields
            .get("tool")
            .is_some_and(|t| t.contains("server_db_query")));
        assert!(event
            .fields
            .get("outcome")
            .is_some_and(|o| o.contains("error")));
        assert!(event.fields.contains_key("duration_ms"));
        assert!(event
            .fields
            .get("error")
            .is_some_and(|e| e.contains("database not connected")));
    }
}
