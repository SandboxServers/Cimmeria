//! `client_drag_drop`: a real drag from one UI slot to another, verified
//! by an inventory diff rather than by what the UI shows.
//!
//! CEGUI starts a drag from a `DragContainer` when the cursor moves far
//! enough with the button held (`EventDragStarted` →
//! `InventoryMod.onSlotItemDragStarted`, which calls `dragItem`), and the
//! drop lands on the window under the cursor at release
//! (`EventDragDropItemDropped` → `onSlotItemDragReceived` → `moveItem`).
//! The drag is: cursor onto the source, button down, the cursor walked to
//! the target in steps (each a CEGUI cursor placement plus a posted
//! `WM_MOUSEMOVE` with the button held), button up.
//!
//! **Stack split.** The stock inventory splits on a **Ctrl**-drag: it pulls
//! one item off the stack (`dragType == 1`, CEGUI button state 9 = left +
//! control). Shift-drag, "choose how many", is a `TODO` in the stock Lua and
//! does nothing, so `split` here means Ctrl-drag, one item.
//!
//! When the posted motion does not start a drag, the steps are replayed
//! through CEGUI's own input injection (`injectMousePosition`), which is a
//! Lua call and is reported at that level.

use std::time::{Duration, Instant};

use serde_json::{json, Value};

use super::inventory::diff;
use super::slots::ContainerRef;
use super::window_click::{click_point, locate_chunk, ClickTarget};
use super::{NativeLevel, NativeTrail, Supervisor};
use crate::supervisor::{keys, process};

/// One end of a drag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DragEnd {
    Slot { container: ContainerRef, slot: u32 },
    Window(String),
}

/// Lua body: is a drag in progress (`getDragInfo`), and is the inventory's
/// drag stand-in visible.
pub const DRAG_STATE: &str = "local t, info = nil, nil \
     local ok = pcall(function() t, info = getDragInfo() end) \
     return __jenc({ drag_type = t, info = info, \
       drag_icon_visible = InventoryDragItemWin ~= nil and InventoryDragItemWin:isVisible() })";

/// Lua body that moves CEGUI's pointer through its own input injection.
pub fn inject_motion_chunk(x: i32, y: i32) -> String {
    format!(
        "local ok = pcall(function() CEGUI.System:getSingleton():injectMousePosition({x}, {y}) end) \
         return __jenc({{ ok = ok }})"
    )
}

/// Evenly spaced points from `a` to `b`, excluding `a`, including `b`.
pub fn path(a: (i32, i32), b: (i32, i32), steps: u32) -> Vec<(i32, i32)> {
    let n = steps.max(1);
    (1..=n)
        .map(|i| {
            let f = f64::from(i) / f64::from(n);
            (
                (f64::from(a.0) + f64::from(b.0 - a.0) * f).round() as i32,
                (f64::from(a.1) + f64::from(b.1 - a.1) * f).round() as i32,
            )
        })
        .collect()
}

/// Whether a drag-state read shows a drag in progress.
pub fn drag_active(state: &Value) -> bool {
    let t = &state["drag_type"];
    let typed = !t.is_null() && t != &json!(0) && t != &json!(false);
    typed || state["drag_icon_visible"] == json!(true)
}

const WM_LBUTTONDOWN: u32 = 0x0201;
const WM_LBUTTONUP: u32 = 0x0202;
const MK_LBUTTON: usize = 0x0001;
const MK_CONTROL: usize = 0x0008;

impl Supervisor {
    /// Resolve one end to a screen point (revealing a slot as needed).
    async fn drag_point(
        &self,
        end: &DragEnd,
        trail: &mut NativeTrail,
    ) -> Result<((i32, i32), Value), String> {
        let name = match end {
            DragEnd::Slot { container, slot } => {
                let loc = self.reveal_slot(container, *slot, trail).await?;
                loc["window"].as_str().unwrap_or_default().to_string()
            }
            DragEnd::Window(w) => w.clone(),
        };
        let loc = self
            .lua_json(&locate_chunk(&ClickTarget::Named(name.clone())))
            .await?;
        if loc["found"] != json!(true) || loc["visible"] != json!(true) {
            return Err(format!("{name} is not on screen"));
        }
        let p = click_point(&loc).ok_or_else(|| format!("{name} has no screen rectangle"))?;
        Ok((p, json!({ "window": name, "point": [p.0, p.1] })))
    }

    async fn set_ctrl(&self, hwnd: isize, down: bool) -> Result<(), String> {
        let ctrl = keys::named("ctrl").expect("ctrl is a named key");
        if down {
            self.bridge_call("input_modifiers", json!({ "ctrl": true }))
                .await?;
            self.post_key(hwnd, ctrl, true).await
        } else {
            self.post_key(hwnd, ctrl, false).await?;
            self.bridge_call("input_modifiers", json!({}))
                .await
                .map(|_| ())
        }
    }

    /// `client_drag_drop`.
    pub async fn drag_drop(
        &self,
        from: &DragEnd,
        to: &DragEnd,
        split: bool,
        steps: u32,
        allow_fallback: bool,
        wait: Duration,
    ) -> Result<Value, String> {
        let t0 = Instant::now();
        let mut trail = NativeTrail::default();
        let before = self.inventory_read(&[], false).await?;
        let (_, src_info) = self.drag_point(from, &mut trail).await?;
        let (dst, dst_info) = self.drag_point(to, &mut trail).await?;
        // Revealing the target can hide the source (another tab of the
        // same window): re-read the source without revealing anything.
        let src_name = src_info["window"].as_str().unwrap_or_default().to_string();
        let (src, _) = self
            .drag_point(&DragEnd::Window(src_name.clone()), &mut trail)
            .await
            .map_err(|e| format!("the source and the target cannot be on screen together: {e}"))?;
        if src == dst {
            return Err(format!(
                "the source and the target are the same point ({}, {})",
                src.0, src.1
            ));
        }
        let hwnd = self.game_hwnd().await?;
        let lp = |p: (i32, i32)| super::lparam_xy(p.0, p.1);
        self.drag_motion(src.0, src.1, false).await?;
        tokio::time::sleep(Duration::from_millis(60)).await;
        if split {
            self.set_ctrl(hwnd, true).await?;
        }
        let mk = MK_LBUTTON | if split { MK_CONTROL } else { 0 };
        let pressed = process::post_message(hwnd, WM_LBUTTONDOWN, mk, lp(src));
        let result = async {
            pressed?;
            trail.push(format!("press on {src_name}"), NativeLevel::RealInput);
            tokio::time::sleep(Duration::from_millis(100)).await;
            let mut started = false;
            let mut started_at_step = None;
            for (i, p) in path(src, dst, steps).into_iter().enumerate() {
                self.drag_motion(p.0, p.1, true).await?;
                tokio::time::sleep(Duration::from_millis(40)).await;
                if !started {
                    started = drag_active(&self.lua_json(DRAG_STATE).await?);
                    if started {
                        started_at_step = Some(i + 1);
                    }
                }
            }
            trail.push(
                format!("move to {} in {steps} steps", dst_info["window"]),
                NativeLevel::RealInput,
            );
            let mut injected = false;
            if !started && allow_fallback {
                for p in std::iter::once(src).chain(path(src, dst, steps)) {
                    self.lua_json(&inject_motion_chunk(p.0, p.1)).await?;
                    tokio::time::sleep(Duration::from_millis(30)).await;
                }
                started = drag_active(&self.lua_json(DRAG_STATE).await?);
                injected = true;
                trail.push(
                    "replay motion via CEGUI injectMousePosition",
                    NativeLevel::ClientUiLua,
                );
            }
            tokio::time::sleep(Duration::from_millis(60)).await;
            Ok::<_, String>((started, started_at_step, injected))
        }
        .await;
        // Always let go, even after a failure mid-drag.
        let released = process::post_message(hwnd, WM_LBUTTONUP, 0, lp(dst));
        if split {
            self.set_ctrl(hwnd, false).await?;
        }
        let (started, started_at_step, injected) = result?;
        released?;
        trail.push(
            format!("release on {}", dst_info["window"]),
            NativeLevel::RealInput,
        );
        let wait_t0 = Instant::now();
        let (after, d) = loop {
            tokio::time::sleep(Duration::from_millis(250)).await;
            let after = self.inventory_read(&[], false).await?;
            let d = diff(&before, &after);
            if d["changed"] == json!(true) || wait_t0.elapsed() >= wait {
                break (after, d);
            }
        };
        let moved = d["changed"] == json!(true);
        let mut out = json!({
            "from": src_info,
            "to": dst_info,
            "split": split,
            "drag_started": started,
            "drag_started_at_step": started_at_step,
            "motion_injected": injected,
            "moved": moved,
            "snap_back": started && !moved,
            "diff": d,
            "cash_after": after["cash"],
        });
        trail.stamp(&mut out);
        out["elapsed_ms"] = json!(t0.elapsed().as_millis() as u64);
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_ends_on_the_target_in_even_steps() {
        let p = path((0, 0), (100, 50), 4);
        assert_eq!(p, vec![(25, 13), (50, 25), (75, 38), (100, 50)]);
        assert_eq!(path((5, 5), (9, 9), 0), vec![(9, 9)]);
    }

    #[test]
    fn drag_state_reads_type_or_the_drag_icon() {
        assert!(!drag_active(
            &json!({ "drag_type": null, "drag_icon_visible": false })
        ));
        assert!(!drag_active(&json!({ "drag_type": 0 })));
        assert!(drag_active(
            &json!({ "drag_type": 1, "drag_icon_visible": false })
        ));
        assert!(drag_active(
            &json!({ "drag_type": null, "drag_icon_visible": true })
        ));
    }

    #[test]
    fn inject_chunk_uses_cegui_system_injection() {
        assert!(inject_motion_chunk(10, 20).contains("injectMousePosition(10, 20)"));
    }
}
