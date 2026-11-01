//! Native input tools: DirectInput keys and buttons, window-message
//! cursor and text. Orchestration lives in `supervisor::input`.

use rmcp::{
    handler::server::wrapper::Parameters, model::*, schemars, tool, tool_router,
    ErrorData as McpError,
};
use serde_json::json;

use super::LabServer;
use crate::supervisor::input::{cursor_target, MouseInput};
use crate::supervisor::ui::click_expect::ClickExpect;
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
        description = "Press, release, or tap a key as the window messages a real key press produces (WM_KEYDOWN/WM_KEYUP; the game translates them itself). Works while the client is in the background. Keys: letters, digits, Enter, Escape, Tab, Space, Backspace, arrows, F1-F12, Shift, Ctrl, Alt, Home/End/PageUp/PageDown/Insert/Delete."
    )]
    async fn client_input_key(
        &self,
        Parameters(a): Parameters<InputKeyArgs>,
    ) -> Result<CallToolResult, McpError> {
        let action = a.action.as_deref().unwrap_or("tap");
        self.wrap(self.supervisor.input_key(&a.key, action, a.hold_ms).await)
    }

    #[tool(
        description = "Relative mouse motion / wheel through the game's DirectInput mouse (mouse-look while the viewport has the mouse captured), and/or a mouse-button press, release or click (window messages). Give x and y (UI pixels, both or neither) to put the UI cursor there first, with a real CEGUI MouseMove, and have the press land on it; without them the press lands wherever the UI cursor already is (CEGUI applies a button at its own cursor, never at a message's coordinates). x and y with no button just move the cursor."
    )]
    async fn client_input_mouse(
        &self,
        Parameters(a): Parameters<InputMouseArgs>,
    ) -> Result<CallToolResult, McpError> {
        let at = match cursor_target(a.x, a.y) {
            Ok(at) => at,
            Err(e) => return self.wrap(Err(e)),
        };
        self.wrap(
            self.supervisor
                .input_mouse(MouseInput {
                    dx: a.dx,
                    dy: a.dy,
                    wheel: a.wheel,
                    button: a.button,
                    action: a.action.as_deref(),
                    hold_ms: a.hold_ms,
                    at,
                })
                .await,
        )
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
        description = "Place the UI cursor at (x, y) in UI pixels and confirm it is there. Moves it through the client's own CEGUI injectMousePosition, so CEGUI sees a real MouseMove (hover, drag thresholds, minigames); cursor_via says native_inject, or lua_set_position (no MouseMove) with cursor_fallback_reason when the native call failed."
    )]
    async fn client_cursor_move(
        &self,
        Parameters(a): Parameters<CursorMoveArgs>,
    ) -> Result<CallToolResult, McpError> {
        let r = self.supervisor.place_cursor(a.x, a.y).await;
        self.wrap(r.map(|p| p.to_json()))
    }

    #[tool(
        description = "Type text into the focused edit box, one key press per character (Shift held for capitals). Letters, digits, space, - _ / and . (so slash commands and the GM console can be typed into chat). Click the edit box first with client_ui_click."
    )]
    async fn client_type_text(
        &self,
        Parameters(a): Parameters<TypeTextArgs>,
    ) -> Result<CallToolResult, McpError> {
        self.wrap(self.supervisor.type_text(&a.text).await)
    }

    #[tool(
        description = "Virtual focus on/off: while on, the game reports its window as the foreground window, so it keeps reading the lab's input while the desktop has focus elsewhere."
    )]
    async fn client_input_focus(
        &self,
        Parameters(a): Parameters<InputFocusArgs>,
    ) -> Result<CallToolResult, McpError> {
        self.wrap(self.supervisor.input_focus(a.on).await)
    }

    #[tool(
        description = "Input hook status: DirectInput devices seen, GetDeviceState/GetDeviceData call counts, held keys/buttons, virtual focus."
    )]
    async fn client_input_status(&self) -> Result<CallToolResult, McpError> {
        self.proxy("input_status", json!({})).await
    }

    #[tool(description = "Release every held key and mouse button.")]
    async fn client_input_release(&self) -> Result<CallToolResult, McpError> {
        self.wrap(self.supervisor.release_all_input("tool").await)
    }
}

#[cfg(test)]
#[path = "input_tools_tests.rs"]
mod tests;
