//! Native input tools: DirectInput keys and buttons, window-message
//! cursor and text. Orchestration lives in `supervisor::input`.

use std::time::Duration;

use rmcp::{
    handler::server::wrapper::Parameters, model::*, schemars, tool, tool_router,
    ErrorData as McpError,
};
use serde_json::{json, Value};

use super::LabServer;
use crate::proof::{EvVerdict, Evidence, Why};
use crate::supervisor::input::{cursor_target, MouseInput};
use crate::supervisor::ui::click_expect::ClickExpect;
use crate::supervisor::input_proof::{
    cursor_evidence, di_count, focus_evidence, key_evidence, mouse_evidence, release_evidence,
    MouseFacts,
};

/// How long `client_input_mouse` waits for the game to take the synthetic
/// records it queued: the game takes them on its next pump (AX-34).
const DI_SETTLE_MS: u64 = 500;
/// The interval of that settle poll.
const DI_POLL_MS: u64 = 50;

/// Args for `client_input_key`.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct InputKeyArgs {
    /// Key name (`"W"`, `"Enter"`, `"Escape"`, `"F1"`, `"Space"`, `"1"`, ...).
    pub key: String,
    /// `"tap"` (default: press, hold, release), `"down"` or `"up"`.
    #[serde(default)]
    pub action: Option<String>,
    /// How long a tap holds the key, in ms (default 80).
    #[serde(default)]
    pub hold_ms: Option<u64>,
}

/// Args for `client_input_mouse`.
#[derive(Debug, Default, serde::Deserialize, schemars::JsonSchema)]
pub struct InputMouseArgs {
    /// Relative motion (DirectInput counts).
    #[serde(default)]
    pub dx: i32,
    #[serde(default)]
    pub dy: i32,
    /// Wheel (120 per notch).
    #[serde(default)]
    pub wheel: i32,
    /// Button index: 0 left, 1 right, 2 middle.
    #[serde(default)]
    pub button: Option<usize>,
    /// `"click"` (default), `"down"` or `"up"`.
    #[serde(default)]
    pub action: Option<String>,
    #[serde(default)]
    pub hold_ms: Option<u64>,
    /// With `y`: put the UI cursor on this UI pixel first (a real CEGUI
    /// MouseMove), then press. Both or neither. With no button, just moves
    /// the cursor.
    #[serde(default)]
    pub x: Option<i32>,
    #[serde(default)]
    pub y: Option<i32>,
}

/// Args for `client_ui_click`.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct UiClickArgs {
    /// Named CEGUI window (a Lua global, e.g. `"Login_LoginButton"`).
    pub window: String,
    /// 0 left (default), 1 right.
    #[serde(default)]
    pub button: Option<usize>,
    /// Window that must be visible after the click; the result is judged on it.
    #[serde(default)]
    pub expect_visible: Option<String>,
    /// Window that must be hidden after the click.
    #[serde(default)]
    pub expect_hidden: Option<String>,
    /// How long to wait for the expected window, ms (default 2000, max 10000).
    #[serde(default)]
    pub expect_timeout_ms: Option<u64>,
}

/// Args for `client_cursor_move`.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CursorMoveArgs {
    /// UI pixel x (the client area CEGUI uses).
    pub x: i32,
    /// UI pixel y.
    pub y: i32,
}

/// Args for `client_type_text`.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct TypeTextArgs {
    /// Text to type into the focused edit box.
    pub text: String,
    /// Edit box to click first and read back; the result is judged only with it.
    pub into: Option<String>,
}

/// Args for `client_input_focus`.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct InputFocusArgs {
    /// true: the game treats its window as foreground (input works while
    /// it is in the background). false: normal focus.
    pub on: bool,
}

#[tool_router(router = input_router, vis = "pub(super)")]
impl LabServer {
    #[tool(
        description = "Press, release, or tap a key as the window messages a real key press produces (WM_KEYDOWN/WM_KEYUP; the game translates them itself). Works while the client is in the background. Keys: letters, digits, Enter, Escape, Tab, Space, Backspace, arrows, F1-F12, Shift, Ctrl, Alt, Home/End/PageUp/PageDown/Insert/Delete. The game's use of a posted key cannot be read back: the result is inconclusive (posted_only). Read the state you expected to change."
    )]
    async fn client_input_key(
        &self,
        Parameters(a): Parameters<InputKeyArgs>,
    ) -> Result<CallToolResult, McpError> {
        let action = a.action.as_deref().unwrap_or("tap");
        let r = self.supervisor.input_key(&a.key, action, a.hold_ms).await;
        self.wrap(r.map(|v| with_evidence(v, key_evidence())))
    }

    #[tool(
        description = "Relative mouse motion / wheel through the game's DirectInput mouse (mouse-look while the viewport has the mouse captured), and/or a mouse-button press, release or click (window messages). Give x and y (UI pixels, both or neither) to put the UI cursor there first, with a real CEGUI MouseMove, and have the press land on it; without them the press lands wherever the UI cursor already is (CEGUI applies a button at its own cursor, never at a message's coordinates). x and y with no button just move the cursor. Evidence: motion or wheel is delivered when the game took the synthetic records (hooks.events_delivered rose); a button is posted only (inconclusive, posted_only); a cursor-only move is read back."
    )]
    async fn client_input_mouse(
        &self,
        Parameters(a): Parameters<InputMouseArgs>,
    ) -> Result<CallToolResult, McpError> {
        let at = match cursor_target(a.x, a.y) {
            Ok(at) => at,
            Err(e) => return self.wrap(Err(e)),
        };
        let motion = a.button.is_none() && (a.dx != 0 || a.dy != 0 || a.wheel != 0);
        let di_before = if motion { self.di_now().await } else { None };
        let r = self
            .supervisor
            .input_mouse(MouseInput {
                dx: a.dx,
                dy: a.dy,
                wheel: a.wheel,
                button: a.button,
                action: a.action.as_deref(),
                hold_ms: a.hold_ms,
                at,
            })
            .await;
        let out = match r {
            Ok(v) => v,
            Err(e) => return self.wrap(Err(e)),
        };
        let di_after = self.settle_delivery(di_before).await;
        let facts = MouseFacts {
            dx: a.dx,
            dy: a.dy,
            wheel: a.wheel,
            button: a.button.is_some(),
            cursor_placed: at.map(|_| true),
            di_before,
            di_after,
        };
        self.wrap(Ok(with_evidence(out, mouse_evidence(facts))))
    }

    #[tool(
        description = "Click a named UI window like a player: read its on-screen rectangle, put the UI cursor on its centre (CEGUI cursor, mirrored into the virtual GetCursorPos), then click with real button messages. Refuses hidden or missing windows. Pass expect_visible or expect_hidden to have the click judged: without one the result is inconclusive (posted_only), because a click alone proves nothing."
    )]
    async fn client_ui_click(
        &self,
        Parameters(a): Parameters<UiClickArgs>,
    ) -> Result<CallToolResult, McpError> {
        let expect = ClickExpect::from_args(a.expect_visible, a.expect_hidden, a.expect_timeout_ms);
        self.wrap(
            self.supervisor
                .ui_click(&a.window, a.button.unwrap_or(0), &expect)
                .await,
        )
    }

    #[tool(
        description = "Place the UI cursor at (x, y) in UI pixels and confirm it is there. Moves it through the client's own CEGUI injectMousePosition, so CEGUI sees a real MouseMove (hover, drag thresholds, minigames); cursor_via says native_inject, or lua_set_position (no MouseMove) with cursor_fallback_reason when the native call failed. Evidence: cursor_at, read back from the client after the move."
    )]
    async fn client_cursor_move(
        &self,
        Parameters(a): Parameters<CursorMoveArgs>,
    ) -> Result<CallToolResult, McpError> {
        let r = self.supervisor.place_cursor(a.x, a.y).await;
        self.wrap(r.map(|p| {
            let ev = cursor_evidence(true, p.at);
            with_evidence(p.to_json(), ev)
        }))
    }

    #[tool(
        description = "Type text into the focused edit box, one key press per character (Shift held for capitals). Letters, digits, space, - _ / and . (so slash commands and the GM console can be typed into chat). Click the edit box first with client_ui_click."
    )]
    async fn client_type_text(
        &self,
        Parameters(a): Parameters<TypeTextArgs>,
    ) -> Result<CallToolResult, McpError> {
        self.wrap(self.supervisor.type_into(a.into.as_deref(), &a.text).await)
    }

    #[tool(
        description = "Virtual focus on/off: while on, the game reports its window as the foreground window, so it keeps reading the lab's input while the desktop has focus elsewhere. Evidence: virtual_focus, read back from the game's virtual_focus_hwnd after the change."
    )]
    async fn client_input_focus(
        &self,
        Parameters(a): Parameters<InputFocusArgs>,
    ) -> Result<CallToolResult, McpError> {
        let out = match self.supervisor.input_focus(a.on).await {
            Ok(v) => v,
            Err(e) => return self.wrap(Err(e)),
        };
        let ev = match self.input_status().await {
            Some(s) => {
                let hwnd = s.get("virtual_focus_hwnd").and_then(Value::as_u64);
                focus_evidence(a.on, hwnd)
            }
            None => unread(),
        };
        self.wrap(Ok(with_evidence(out, ev)))
    }

    #[tool(
        description = "Input hook status: DirectInput devices seen, GetDeviceState/GetDeviceData call counts, held keys/buttons, virtual focus."
    )]
    async fn client_input_status(&self) -> Result<CallToolResult, McpError> {
        self.proxy("input_status", json!({})).await
    }

    #[tool(
        description = "Release every held key and mouse button. Evidence: nothing_held, read back from input_status (held keys and buttons, and any C5 hold)."
    )]
    async fn client_input_release(&self) -> Result<CallToolResult, McpError> {
        let out = match self.supervisor.release_all_input("tool").await {
            Ok(v) => v,
            Err(e) => return self.wrap(Err(e)),
        };
        let ev = match self.input_status().await {
            Some(s) => release_evidence(&s["held"], s.get("holds")),
            None => unread(),
        };
        self.wrap(Ok(with_evidence(out, ev)))
    }
}

impl LabServer {
    /// `input_status` from the DLL, or `None` when the read failed.
    async fn input_status(&self) -> Option<Value> {
        self.supervisor
            .bridge_call("input_status", json!({}))
            .await
            .map_err(|e| tracing::warn!(error = %e, "input_status read-back failed (AX-43)"))
            .ok()
    }

    /// `hooks.events_delivered` now, or `None` when it cannot be read.
    async fn di_now(&self) -> Option<u64> {
        di_count(&self.input_status().await?)
    }

    /// Poll the delivery counter until it rises above `before`, for at most
    /// `DI_SETTLE_MS`. Returns the last reading taken. The game takes queued
    /// records on its next pump, so a read right after the call is too early.
    async fn settle_delivery(&self, before: Option<u64>) -> Option<u64> {
        let before = before?;
        let mut last = None;
        for _ in 0..DI_SETTLE_MS / DI_POLL_MS {
            tokio::time::sleep(Duration::from_millis(DI_POLL_MS)).await;
            if let Some(n) = self.di_now().await {
                last = Some(n);
                if n > before {
                    break;
                }
            }
        }
        last
    }
}

/// `v` with the judged `evidence` under its key. A non-object result is
/// wrapped first so the key always lands.
fn with_evidence(v: Value, ev: Evidence) -> Value {
    let mut out = if v.is_object() {
        v
    } else {
        json!({ "result": v })
    };
    out["evidence"] = ev.to_value();
    out
}

/// The read-back after a state change failed: nothing was measured.
fn unread() -> Evidence {
    Evidence::from_verdict(EvVerdict::Inconclusive, Why::NoPostRead, Vec::new())
}

#[cfg(test)]
#[path = "input_tools_tests.rs"]
mod tests;
