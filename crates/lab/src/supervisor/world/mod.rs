//! World interaction for automated UAT: find entities, click them in the
//! 3D view, walk, and aim the camera — with the client's own input.
//!
//! Tools (see `server::world` for the MCP surface):
//!
//! - `client_entity_find` ([`find`]): entities from the client's entity map
//!   with actor positions read from memory, names/level/hostility/mob id
//!   from the stock `unit*` Lua API on a private unit slot, and the screen
//!   point from the game's own `view:worldToPixel`.
//! - `client_world_click` / `client_target` ([`click`]): project, turn the
//!   camera if the entity is off screen, put the cursor on it, check the
//!   client's own mouse-over, click with real button messages, and report
//!   what changed (target, windows).
//! - `client_move_to` ([`movement`]): hold the forward key and steer with
//!   mouse-look, closed loop on the player's actor position.
//! - `client_camera` ([`camera`]): raw mouse-look/zoom, or face a point.
//!
//! **Native level.** Every result carries `native_level` so a UAT runner
//! can refuse shortcuts: `real_input` (N1: keys, mouse-look, clicks) >
//! `slash_command` (N2) > `ui_lua` (N3: a stock UI Lua call such as
//! `targetUnit`) > `server_shortcut` (X). Reads report `read`: they drive
//! nothing. A tool that fell back reports the lowest level it used.

pub mod camera;
pub mod click;
pub mod find;
pub mod geometry;
pub mod io;
pub mod lua;
pub mod memory;
pub mod movement;
pub mod steer;

#[cfg(test)]
mod sim;
#[cfg(test)]
mod tool_tests;

use rmcp::schemars;
use serde_json::{json, Value};

use geometry::{server_to_client, Vec3};

/// How natively a tool drove the game (see the module doc). The world
/// tools never type slash commands (N2) or use server shortcuts (X), so
/// those rungs of the ladder have no variant here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NativeLevel {
    /// Nothing was driven; the tool only read.
    Read,
    /// N1: real key/mouse input through the game's own input handling.
    RealInput,
    /// N3: a stock UI Lua call (the same code a button runs).
    UiLua,
}

impl NativeLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            NativeLevel::Read => "read",
            NativeLevel::RealInput => "real_input",
            NativeLevel::UiLua => "ui_lua",
        }
    }

    /// The UAT guide's tier label.
    pub fn tier(self) -> &'static str {
        match self {
            NativeLevel::Read => "-",
            NativeLevel::RealInput => "N1",
            NativeLevel::UiLua => "N3",
        }
    }

    /// Does a pass at this level count as a native pass?
    pub fn counts_as_native(self) -> bool {
        matches!(self, NativeLevel::RealInput)
    }
}

/// A world-tool failure: which tool, which step, which entity or point,
/// what the client showed, and the steps done so far with timings.
#[derive(Debug, Clone)]
pub struct WorldError(Box<WorldFailure>);

#[derive(Debug, Clone)]
pub struct WorldFailure {
    pub tool: &'static str,
    pub step: String,
    pub message: String,
    /// The entity id / name / point the step was about.
    pub subject: Option<Value>,
    /// Evidence at failure time (mouse-over, windows, positions).
    pub state: Option<Value>,
    pub native_level: NativeLevel,
    pub elapsed_ms: u64,
    pub steps: Vec<Value>,
}

impl std::ops::Deref for WorldError {
    type Target = WorldFailure;
    fn deref(&self) -> &WorldFailure {
        &self.0
    }
}

impl WorldError {
    pub fn summary(&self) -> String {
        let subject = self
            .subject
            .as_ref()
            .map(|s| format!(" [{s}]"))
            .unwrap_or_default();
        let state = self
            .state
            .as_ref()
            .map(|s| format!("; state: {s}"))
            .unwrap_or_default();
        format!(
            "{} failed at step '{}'{subject} after {} ms: {}{state}",
            self.tool, self.step, self.elapsed_ms, self.message
        )
    }

    pub fn to_json(&self) -> Value {
        json!({
            "tool": self.tool,
            "step": self.step,
            "error": self.message,
            "subject": self.subject,
            "state": self.state,
            "native_level": self.native_level.as_str(),
            "elapsed_ms": self.elapsed_ms,
            "steps": self.steps,
        })
    }
}

/// The step log of one tool call.
#[derive(Debug)]
pub struct Steps {
    tool: &'static str,
    steps: Vec<Value>,
    started_ms: u64,
    level: NativeLevel,
    subject: Option<Value>,
}

impl Steps {
    pub fn new(tool: &'static str, now_ms: u64) -> Self {
        Self {
            tool,
            steps: Vec::new(),
            started_ms: now_ms,
            level: NativeLevel::Read,
            subject: None,
        }
    }

    /// Note that input at `level` was used (the result reports the lowest).
    pub fn used(&mut self, level: NativeLevel) {
        self.level = self.level.max(level);
    }

    #[cfg(test)]
    pub fn level(&self) -> NativeLevel {
        self.level
    }

    pub fn subject(&mut self, s: Value) {
        self.subject = Some(s);
    }

    /// Log a finished step that started at `t0_ms`.
    pub fn record(&mut self, step: &str, t0_ms: u64, now_ms: u64, detail: Value) {
        let mut s = json!({ "step": step, "ms": now_ms.saturating_sub(t0_ms) });
        if !detail.is_null() {
            s["detail"] = detail;
        }
        self.steps.push(s);
    }

    pub fn fail(&self, now_ms: u64, step: &str, message: impl Into<String>) -> WorldError {
        WorldError(Box::new(WorldFailure {
            tool: self.tool,
            step: step.to_string(),
            message: message.into(),
            subject: self.subject.clone(),
            state: None,
            native_level: self.level,
            elapsed_ms: now_ms.saturating_sub(self.started_ms),
            steps: self.steps.clone(),
        }))
    }

    pub fn fail_with(
        &self,
        now_ms: u64,
        step: &str,
        message: impl Into<String>,
        state: Value,
    ) -> WorldError {
        let mut e = self.fail(now_ms, step, message);
        e.0.state = Some(state);
        e
    }

    /// Add the standard fields to a tool's result.
    pub fn finish(self, now_ms: u64, mut out: Value) -> Value {
        out["native_level"] = json!(self.level.as_str());
        out["native_tier"] = json!(self.level.tier());
        out["counts_as_native_pass"] = json!(self.level.counts_as_native());
        out["elapsed_ms"] = json!(now_ms.saturating_sub(self.started_ms));
        out["steps"] = json!(self.steps);
        out
    }
}

/// Coordinates as a tool argument: server metres (what `.location`,
/// `server_entity_get` and the GM console use) unless `space` says client.
#[derive(Debug, Clone, Copy, PartialEq, serde::Deserialize, schemars::JsonSchema)]
pub struct PointArg {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    /// `"server"` (default: BigWorld metres, Y up) or `"client"` (UE3
    /// units, Z up — what `client_entity_find` reports as `client`).
    #[serde(default)]
    pub space: Option<Space>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Space {
    Server,
    Client,
}

impl PointArg {
    /// The point in client space.
    pub fn to_client(self) -> Vec3 {
        let v = Vec3::new(self.x, self.y, self.z);
        match self.space.unwrap_or(Space::Server) {
            Space::Server => server_to_client(v),
            Space::Client => v,
        }
    }
}

/// An entity or a point to act on.
#[derive(Debug, Clone, Default, serde::Deserialize, schemars::JsonSchema)]
pub struct TargetArg {
    /// Entity id (from `client_entity_find` or the server tools).
    #[serde(default)]
    pub entity_id: Option<u32>,
    /// Name (case-insensitive substring; the nearest match wins).
    #[serde(default)]
    pub name: Option<String>,
    /// A world point instead of an entity.
    #[serde(default)]
    pub point: Option<PointArg>,
}

impl TargetArg {
    pub fn describe(&self) -> Value {
        json!({
            "entity_id": self.entity_id,
            "name": self.name,
            "point": self.point.map(|p| json!({ "x": p.x, "y": p.y, "z": p.z, "space": match p.space.unwrap_or(Space::Server) { Space::Server => "server", Space::Client => "client" } })),
        })
    }

    pub fn is_empty(&self) -> bool {
        self.entity_id.is_none() && self.name.is_none() && self.point.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_levels_order_from_most_to_least_native() {
        let mut s = Steps::new("t", 0);
        assert_eq!(s.level(), NativeLevel::Read);
        s.used(NativeLevel::RealInput);
        s.used(NativeLevel::UiLua);
        s.used(NativeLevel::RealInput);
        assert_eq!(s.level(), NativeLevel::UiLua);
        let out = s.finish(10, json!({}));
        assert_eq!(out["native_level"], "ui_lua");
        assert_eq!(out["native_tier"], "N3");
        assert_eq!(out["counts_as_native_pass"], false);
    }

    #[test]
    fn errors_name_the_tool_step_and_subject() {
        let mut s = Steps::new("client_target", 100);
        s.subject(json!({ "entity_id": 42 }));
        s.record("project", 100, 130, json!(null));
        let e = s.fail(250, "hover", "occluded by entity 7");
        let text = e.summary();
        assert!(text.contains("client_target failed at step 'hover'"));
        assert!(text.contains("\"entity_id\":42"));
        assert!(text.contains("after 150 ms"));
        assert_eq!(e.to_json()["steps"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn point_args_default_to_server_metres() {
        let p = PointArg {
            x: 1.0,
            y: 2.0,
            z: 3.0,
            space: None,
        };
        assert_eq!(p.to_client(), Vec3::new(300.0, 100.0, 200.0));
        let c = PointArg {
            space: Some(Space::Client),
            ..p
        };
        assert_eq!(c.to_client(), Vec3::new(1.0, 2.0, 3.0));
    }
}
