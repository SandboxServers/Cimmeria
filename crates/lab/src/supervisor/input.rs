//! Driving the client with its own input, not Lua shortcuts.
//!
//! What the live client showed (2026-09-29):
//!
//! - **Keys and typing** are UE3 window messages: a posted `WM_KEYDOWN` /
//!   `WM_KEYUP` reaches the game and the UI (the game translates the key
//!   itself; a bare posted `WM_CHAR` is ignored). The DirectInput
//!   keyboard is created but never read.
//! - **Mouse buttons** are window messages too, and CEGUI applies them at
//!   *its* cursor position, not at the message's coordinates.
//! - **The UI cursor** is not moved by posted `WM_MOUSEMOVE` or by
//!   DirectInput motion; it is placed through CEGUI's own cursor
//!   (`MouseCursor:setPosition`), mirrored into the bridge's virtual
//!   `GetCursorPos` so the viewport agrees.
//! - **Mouse-look** is DirectInput relative motion (`input_mouse`), read
//!   while the viewport has the mouse captured.
//!
//! Lua only *reads* widget rectangles and places the cursor; every press
//! goes through the game's own input handling.
//!
//! Virtual focus (`input_focus`) makes the game treat its window as the
//! foreground window, so none of this takes focus from the desktop.

use std::time::Duration;

use serde_json::{json, Value};

use super::{keys, process, Supervisor};

/// How long a tapped key or clicked button is held: long enough for at
/// least a couple of frames at the game's frame rate.
pub const DEFAULT_HOLD_MS: u64 = 80;

/// A widget rectangle in UI pixels (CEGUI unclipped pixel rect).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub left: f64,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
}

impl Rect {
    pub fn centre(&self) -> (i32, i32) {
        (
            ((self.left + self.right) / 2.0).round() as i32,
            ((self.top + self.bottom) / 2.0).round() as i32,
        )
    }
}

/// Lua that returns `visible, left, top, right, bottom` for a window, or
/// `'missing'`. `window` is a Lua global (every named layout window is one),
/// or `Parent/Child` for a child with no global of its own, such as a frame
/// window's `Parent__auto_closebutton__`, found by `getChildRecursive`.
pub fn widget_rect_chunk(window: &str) -> String {
    let lookup = match window.split_once('/') {
        Some((parent, child)) => format!(
            "local p = _G[{parent:?}]              local w = p and p:getChildRecursive({child:?})"
        ),
        None => format!("local w = _G[{window:?}]"),
    };
    format!(
        "{lookup}          if not w then return 'missing' end          local r = w:getUnclippedPixelRect()          return tostring(w:isVisible()), r.left, r.top, r.right, r.bottom"
    )
}

/// Parse [`widget_rect_chunk`]'s results.
pub fn parse_widget_rect(window: &str, results: &[String]) -> Result<Rect, String> {
    match results {
        [missing] if missing == "missing" => Err(format!("no UI window named {window}")),
        [visible, l, t, r, b] => {
            if visible != "true" {
                return Err(format!("{window} is not visible"));
            }
            let n = |s: &String| s.parse::<f64>().map_err(|e| format!("{window} rect: {e}"));
            Ok(Rect {
                left: n(l)?,
                top: n(t)?,
                right: n(r)?,
                bottom: n(b)?,
            })
        }
        other => Err(format!("{window}: unexpected rect results {other:?}")),
    }
}

/// Lua that returns the UI cursor position.
pub const CURSOR_CHUNK: &str =
    "local p = CEGUI.MouseCursor:getSingleton():getPosition() return p.x, p.y";

/// Lua that places the UI cursor at `(x, y)` and returns where it is.
pub fn place_cursor_chunk(x: i32, y: i32) -> String {
    format!(
        "local c = CEGUI.MouseCursor:getSingleton() \
         c:setPosition(CEGUI.Vector2({x}, {y})) \
         local p = c:getPosition() return p.x, p.y"
    )
}

const WM_KEYDOWN: u32 = 0x0100;
const WM_KEYUP: u32 = 0x0101;
const MK_LBUTTON: usize = 0x0001;
const MK_RBUTTON: usize = 0x0002;
const MK_MBUTTON: usize = 0x0010;

/// `(down message, up message, MK_ flag)` for mouse button 0/1/2.
fn button_messages(button: usize) -> Result<(u32, u32, usize), String> {
    match button {
        0 => Ok((0x0201, 0x0202, MK_LBUTTON)),
        1 => Ok((0x0204, 0x0205, MK_RBUTTON)),
        2 => Ok((0x0207, 0x0208, MK_MBUTTON)),
        other => Err(format!("mouse button {other} (0 left, 1 right, 2 middle)")),
    }
}

fn lparam_xy(x: i32, y: i32) -> isize {
    (((y as u32 & 0xFFFF) << 16) | (x as u32 & 0xFFFF)) as isize
}

fn results_of(v: &Value) -> Vec<String> {
    v.get("results")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn parse_point(results: &[String]) -> Option<(i32, i32)> {
    match results {
        [x, y] => Some((
            x.parse::<f64>().ok()?.round() as i32,
            y.parse::<f64>().ok()?.round() as i32,
        )),
        _ => None,
    }
}

impl Supervisor {
    async fn game_hwnd(&self) -> Result<isize, String> {
        let pid = {
            let st = self.state.lock().await;
            st.pid.ok_or("no client running")?
        };
        tokio::task::spawn_blocking(move || process::find_main_window(pid))
            .await
            .map_err(|e| format!("window lookup: {e}"))?
            .ok_or_else(|| "the client has no main window yet".to_string())
    }

    /// `client_input_focus` — turn virtual focus on (the game believes it
    /// is the foreground window) or off.
    pub async fn input_focus(&self, on: bool) -> Result<Value, String> {
        let hwnd = if on {
            Some(self.game_hwnd().await?)
        } else {
            None
        };
        self.bridge_call("input_focus", json!({ "hwnd": hwnd }))
            .await
    }

    async fn post_key(&self, hwnd: isize, key: keys::Key, down: bool) -> Result<(), String> {
        let (msg, lparam) = if down {
            (WM_KEYDOWN, key.lparam_down())
        } else {
            (WM_KEYUP, key.lparam_up())
        };
        process::post_message(hwnd, msg, key.vk as usize, lparam)
    }

    /// `client_input_key` — press, release, or tap (press, hold, release)
    /// a key, as the window messages a real key produces.
    pub async fn input_key(
        &self,
        key: &str,
        action: &str,
        hold_ms: Option<u64>,
    ) -> Result<Value, String> {
        let k = keys::named(key).ok_or_else(|| format!("unknown key {key:?}"))?;
        let hwnd = self.game_hwnd().await?;
        match action {
            "down" => self.post_key(hwnd, k, true).await?,
            "up" => self.post_key(hwnd, k, false).await?,
            "tap" => {
                self.post_key(hwnd, k, true).await?;
                tokio::time::sleep(Duration::from_millis(hold_ms.unwrap_or(DEFAULT_HOLD_MS))).await;
                self.post_key(hwnd, k, false).await?;
            }
            other => return Err(format!("action must be down, up or tap, not {other:?}")),
        }
        Ok(json!({ "key": key, "action": action }))
    }

    /// `client_input_mouse` — relative motion / wheel (DirectInput, for
    /// mouse-look), and a button press, release or click at the current
    /// UI cursor (window messages).
    pub async fn input_mouse(
        &self,
        dx: i32,
        dy: i32,
        wheel: i32,
        button: Option<usize>,
        action: Option<&str>,
        hold_ms: Option<u64>,
    ) -> Result<Value, String> {
        if dx != 0 || dy != 0 || wheel != 0 {
            self.bridge_call("input_mouse", json!({ "dx": dx, "dy": dy, "wheel": wheel }))
                .await?;
        }
        let Some(button) = button else {
            return self.bridge_call("input_status", json!({})).await;
        };
        let at = self.cursor().await?;
        self.post_button(button, action.unwrap_or("click"), at, hold_ms)
            .await?;
        Ok(json!({ "button": button, "at": [at.0, at.1] }))
    }

    async fn post_button(
        &self,
        button: usize,
        action: &str,
        (x, y): (i32, i32),
        hold_ms: Option<u64>,
    ) -> Result<(), String> {
        let (down_msg, up_msg, mk) = button_messages(button)?;
        let hwnd = self.game_hwnd().await?;
        let lp = lparam_xy(x, y);
        match action {
            "down" => process::post_message(hwnd, down_msg, mk, lp),
            "up" => process::post_message(hwnd, up_msg, 0, lp),
            "click" => {
                process::post_message(hwnd, down_msg, mk, lp)?;
                tokio::time::sleep(Duration::from_millis(hold_ms.unwrap_or(DEFAULT_HOLD_MS))).await;
                process::post_message(hwnd, up_msg, 0, lp)
            }
            other => Err(format!("action must be down, up or click, not {other:?}")),
        }
    }

    /// The UI cursor position.
    pub async fn cursor(&self) -> Result<(i32, i32), String> {
        let v = self
            .bridge_call("lua_eval", json!({ "chunk": CURSOR_CHUNK }))
            .await?;
        parse_point(&results_of(&v)).ok_or_else(|| "could not read the UI cursor".to_string())
    }

    /// Place the UI cursor at `(x, y)` (UI pixels) and confirm it is there.
    pub async fn move_cursor(&self, x: i32, y: i32) -> Result<(i32, i32), String> {
        self.bridge_call("input_cursor", json!({ "x": x, "y": y }))
            .await?;
        let v = self
            .bridge_call("lua_eval", json!({ "chunk": place_cursor_chunk(x, y) }))
            .await?;
        match parse_point(&results_of(&v)) {
            Some(p) if (p.0 - x).abs() <= 1 && (p.1 - y).abs() <= 1 => Ok(p),
            other => Err(format!(
                "cursor did not reach ({x}, {y}); it is at {other:?}"
            )),
        }
    }

    /// `client_ui_click` — find a named UI window, put the cursor on its
    /// centre, and click it with real button messages.
    pub async fn ui_click(&self, window: &str, button: usize) -> Result<Value, String> {
        let v = self
            .bridge_call("lua_eval", json!({ "chunk": widget_rect_chunk(window) }))
            .await?;
        let rect = parse_widget_rect(window, &results_of(&v))?;
        let (x, y) = rect.centre();
        let at = self.move_cursor(x, y).await?;
        // Give the UI a frame to register the hover before the press.
        tokio::time::sleep(Duration::from_millis(40)).await;
        self.post_button(button, "click", at, None).await?;
        Ok(json!({ "window": window, "clicked_at": [at.0, at.1] }))
    }

    /// `client_type_text` — type into the focused edit box, one key press
    /// per character (the game translates keys itself), with a virtual
    /// Shift for capitals and `_`.
    pub async fn type_text(&self, text: &str) -> Result<Value, String> {
        let hwnd = self.game_hwnd().await?;
        let plan = keys::plan_text(text)?;
        for step in plan {
            if step.shift {
                // The game reads Shift from its keyboard state, not from
                // messages: hold it there, and post it for UE3's own tracking.
                self.bridge_call("input_modifiers", json!({ "shift": true }))
                    .await?;
                self.post_key(hwnd, keys::SHIFT, true).await?;
            }
            self.post_key(hwnd, step.key, true).await?;
            tokio::time::sleep(Duration::from_millis(20)).await;
            self.post_key(hwnd, step.key, false).await?;
            if step.shift {
                self.post_key(hwnd, keys::SHIFT, false).await?;
                self.bridge_call("input_modifiers", json!({})).await?;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        Ok(json!({ "typed_chars": text.chars().count() }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_centre_rounds_to_the_middle_pixel() {
        let r = Rect {
            left: 412.0,
            top: 309.0,
            right: 612.0,
            bottom: 344.0,
        };
        assert_eq!(r.centre(), (512, 327));
    }

    #[test]
    fn widget_rect_parses_a_visible_window() {
        let res: Vec<String> = ["true", "412", "204", "612", "234"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let r = parse_widget_rect("Login_AccountEdit", &res).unwrap();
        assert_eq!(r.centre(), (512, 219));
    }

    /// Clicking a hidden widget would click whatever is under it: refuse.
    #[test]
    fn widget_rect_refuses_hidden_and_missing_windows() {
        let hidden: Vec<String> = ["false", "0", "0", "1", "1"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert!(parse_widget_rect("X", &hidden)
            .unwrap_err()
            .contains("not visible"));
        assert!(parse_widget_rect("X", &["missing".to_string()])
            .unwrap_err()
            .contains("no UI window"));
    }

    #[test]
    fn widget_rect_chunk_resolves_parent_child_paths() {
        let c = widget_rect_chunk("DialogWin/DialogWin__auto_closebutton__");
        assert!(c.contains("_G[\"DialogWin\"]"));
        assert!(c.contains("getChildRecursive(\"DialogWin__auto_closebutton__\")"));
    }

    #[test]
    fn widget_rect_chunk_quotes_the_name() {
        assert!(
            widget_rect_chunk("CharSelect_PlayButton").contains("_G[\"CharSelect_PlayButton\"]")
        );
    }
}
