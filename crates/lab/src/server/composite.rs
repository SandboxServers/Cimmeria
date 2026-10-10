//! The composite tools (`supervisor::composite`): `lab_ensure_in_world`,
//! `client_batch`, `client_ui_sequence`. Each is one lease-guarded call for
//! a sequence an agent used to spend a turn per step on.

use rmcp::{
    handler::server::wrapper::Parameters, model::*, schemars, tool, tool_router,
    ErrorData as McpError,
};
use serde_json::Value;

use super::LabServer;
use crate::supervisor::composite::batch::parse_steps;
use crate::supervisor::composite::ensure::{EnsureRequest, StopAt};
use crate::supervisor::composite::sequence::parse_actions;
use crate::supervisor::flows::characters::CreateRequest;

/// Args for `lab_ensure_in_world`.
#[derive(Debug, Default, serde::Deserialize, schemars::JsonSchema)]
pub struct EnsureArgs {
    /// Start override when a client must be launched (`local`, `colo`).
    #[serde(default)]
    pub server: Option<String>,
    /// Server row to pick at login (default: `server`, then lab-account.json).
    #[serde(default)]
    pub shard: Option<String>,
    /// Character name as the list shows it (default: lab-account.json).
    #[serde(default)]
    pub character: Option<String>,
    /// Turn on virtual focus at the end (default true).
    #[serde(default)]
    pub focus: Option<bool>,
    /// Stop at `running` (window and bridge up), `character_select`, or
    /// `world` (default).
    #[serde(default)]
    pub stop_at: Option<String>,
    /// Create the character when missing: {alignment, archetype, gender,
    /// first}; its last name is `character`.
    #[serde(default)]
    pub create: Option<CreateArg>,
}

#[derive(Debug, Clone, serde::Deserialize, schemars::JsonSchema)]
pub struct CreateArg {
    pub alignment: String,
    pub archetype: String,
    pub gender: String,
    /// First name (default `Lab`).
    #[serde(default)]
    pub first: Option<String>,
}

/// Args for `client_batch`.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct BatchArgs {
    /// Ordered steps, each {id?, op, ...}: lua {chunk}; mem_read {addr, len?,
    /// as: hex|u8|u16|u32|i32|f32|f64, default u32 for 4 bytes}; call_native {addr, conv?, args?,
    /// ret?: u32|i32|f32|f64|void|hex}; wait {frames|ms}; player_state
    /// {fields?}; window_text {window, children?}. "$id", "$id.key" or
    /// "$id+0x270" in an argument is an earlier step's value; "${id}" inside
    /// a string interpolates it. JSON floats in call_native args are passed
    /// as f32 bits ({"f64": x} for a double).
    pub steps: Vec<Value>,
    /// Stop at the first failed step (default true).
    #[serde(default)]
    pub stop_on_error: Option<bool>,
}

/// Args for `client_ui_sequence`.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SequenceArgs {
    /// Ordered actions, each {do, ...}: click {window, button?}; key {key,
    /// action?: tap|down|up}; type {text, into?: edit box to click first};
    /// drag {from, to: {container, slot} or {window}, split?}; wait_window
    /// {window, gone?, timeout_ms?}; wait {ms}.
    pub actions: Vec<Value>,
    /// Stop at the first failed action (default true).
    #[serde(default)]
    pub stop_on_error: Option<bool>,
}

fn json_result(v: Value) -> CallToolResult {
    CallToolResult::success(vec![ContentBlock::text(v.to_string())])
}

#[tool_router(router = composite_router, vis = "pub(super)")]
impl LabServer {
    #[tool(
        description = "Get in the world as a character in one call: start the client if none is running, wait for its window and bridge, log in (lab-account.json credentials), pick the server row, play the character (with `create` and the character missing: the arguments are checked first, then the oldest unprotected character may be deleted to free a slot, reported as deleted_to_free_a_slot, and it is created), finish an intro dialog, turn on virtual focus. `stop_at` running or character_select stops earlier. Already there: returns at once. Returns {in_world, character, world_id, pos, steps_ms} (or {at, characters?}); a failure names the step. Refuses to start while a client the lab did not launch is running."
    )]
    async fn lab_ensure_in_world(
        &self,
        Parameters(a): Parameters<EnsureArgs>,
    ) -> Result<CallToolResult, McpError> {
        let create = a.create.map(|c| CreateRequest {
            first: c.first.unwrap_or_else(|| "Lab".into()),
            last: a.character.clone().unwrap_or_default(),
            alignment: c.alignment,
            archetype: c.archetype,
            gender: c.gender,
        });
        let req = EnsureRequest {
            server: a.server,
            shard: a.shard,
            character: a.character,
            focus: a.focus.unwrap_or(true),
            create,
            stop: match a.stop_at.as_deref() {
                Some(s) => StopAt::parse(s).map_err(|e| McpError::invalid_params(e, None))?,
                None => StopAt::World,
            },
        };
        self.supervisor
            .ensure_in_world(req)
            .await
            .map(json_result)
            .map_err(|e| McpError::internal_error(format!("lab_ensure_in_world: {e}"), None))
    }

    #[tool(
        description = "Run ordered read/probe steps in one call (lua, mem_read, call_native, wait, player_state, window_text) with $id references to earlier results. call_native is journaled and exception-guarded like client_call_native and never replayed after a crash. Returns {steps: {id: value | {error}}, ms, stopped_at?}."
    )]
    async fn client_batch(
        &self,
        Parameters(a): Parameters<BatchArgs>,
    ) -> Result<CallToolResult, McpError> {
        let steps = parse_steps(&a.steps).map_err(|e| McpError::invalid_params(e, None))?;
        let out = self
            .supervisor
            .batch(&steps, a.stop_on_error.unwrap_or(true))
            .await;
        Ok(json_result(out.to_json()))
    }

    #[tool(
        description = "Run a scripted UI step in one call: clicks on named windows, key taps, typing (optionally into an edit box clicked first), slot drags, and waits for a window to show or close. Same input paths as client_ui_click / client_input_key / client_type_text / client_drag_drop; refuses hidden or missing windows. Returns per-action results and the least native level used (native_level, native_tier, native_pass)."
    )]
    async fn client_ui_sequence(
        &self,
        Parameters(a): Parameters<SequenceArgs>,
    ) -> Result<CallToolResult, McpError> {
        let actions = parse_actions(&a.actions).map_err(|e| McpError::invalid_params(e, None))?;
        let out = self
            .supervisor
            .ui_sequence(&actions, a.stop_on_error.unwrap_or(true))
            .await;
        Ok(json_result(out))
    }
}
