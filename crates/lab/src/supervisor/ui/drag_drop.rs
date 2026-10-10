//! `client_drag_drop`: a real drag from one UI slot to another, verified
//! by an inventory diff rather than by what the UI shows.
//!
//! CEGUI starts a drag from a `DragContainer` when the cursor moves far
//! enough with the button held (`EventDragStarted` →
//! `InventoryMod.onSlotItemDragStarted`, which calls `dragItem`), and the
//! drop lands on the container's `d_dropTarget` at release
//! (`EventDragDropItemDropped` → `onSlotItemDragReceived` → `moveItem`).
//!
//! A posted `WM_MOUSEMOVE` never reaches CEGUI, so the drag is driven
//! through the client's own CEGUI injectors ([`crate::supervisor::cegui_native`]),
//! the sequence proven live on 2026-10-10: cursor onto the source, button
//! down, then the cursor walked to the target one step per frame (a bridge
//! round trip between steps), then button up. The move that crosses the
//! drag threshold only starts the drag; target selection runs on the moves
//! after it, so a drag takes at least [`MIN_STEPS`] steps.
//!
//! **The explicit drop.** In the live client `d_dropTarget` stays null even
//! over a slot, so button-up drops nothing. Why is not settled: the
//! findings' leading suspect is that no window from the hit slot up to the
//! sheet has `DragDropTarget` set. Until it is, when the target is null at
//! the end of a started drag the tool fires the target window's
//! `DragDropItemDropped` itself (`notifyDragDropItemDropped`) before
//! letting go. The stock handlers then run as for a real drop, but that
//! step is reported as `native_call` (N3), not as input.
//!
//! **Stack split.** The stock inventory splits on a **Ctrl**-drag: it pulls
//! one item off the stack (`dragType == 1`, CEGUI button state 9 = left +
//! control). Shift-drag, "choose how many", is a `TODO` in the stock Lua and
//! does nothing, so `split` here means Ctrl-drag, one item.

use std::time::{Duration, Instant};

use serde_json::{json, Value};

use super::inventory::diff;
use super::slots::ContainerRef;
use super::window_click::{click_point, locate_chunk, ClickTarget};
use super::{NativeLevel, NativeTrail, Supervisor};
use crate::supervisor::cegui_native::{drop_plan, DropPlan, LEFT_BUTTON};
use crate::supervisor::keys;

/// One end of a drag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DragEnd {
    Slot { container: ContainerRef, slot: u32 },
    Window(String),
}

/// Fewest cursor steps in a drag: one to cross the threshold (which only
/// starts the drag), then at least two for `doDragging` to pick a target
/// (findings §7.2).
pub const MIN_STEPS: u32 = 3;

/// Lua body: is a drag in progress (`getDragInfo`), and is the inventory's
/// drag stand-in visible.
pub const DRAG_STATE: &str = "local t, info = nil, nil \
     local ok = pcall(function() t, info = getDragInfo() end) \
     return __jenc({ drag_type = t, info = info, \
       drag_icon_visible = InventoryDragItemWin ~= nil and InventoryDragItemWin:isVisible() })";

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

/// What happened between button down and button up.
#[derive(Debug, Default)]
struct DragRun {
    started_at_step: Option<usize>,
    drop_target: u32,
    notified: bool,
}

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

    /// `client_drag_drop`. `allow_notify` permits the explicit drop when
    /// CEGUI resolved no drop target (see the module docs).
    pub async fn drag_drop(
        &self,
        from: &DragEnd,
        to: &DragEnd,
        split: bool,
        steps: u32,
        allow_notify: bool,
        wait: Duration,
    ) -> Result<Value, String> {
        let t0 = Instant::now();
        let steps = steps.max(MIN_STEPS);
        let mut trail = NativeTrail::default();
        let before = self.inventory_read(&[], false).await?;
        let (_, src_info) = self.drag_point(from, &mut trail).await?;
        let (dst, dst_info) = self.drag_point(to, &mut trail).await?;
        // Revealing the target can hide the source (another tab of the
        // same window): re-read the source without revealing anything.
        let src_name = src_info["window"].as_str().unwrap_or_default().to_string();
        let dst_name = dst_info["window"].as_str().unwrap_or_default().to_string();
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
        // Resolve every native pointer before anything is pressed: a
        // refusal here leaves the client untouched.
        let cegui = self.cegui().await?;
        let container = cegui.drag_container(&src_name).await?;
        let target = cegui.window_ptr(&dst_name).await?;
        // Only Ctrl needs the window (a posted key); a plain drag is all
        // native calls.
        let hwnd = if split {
            Some(self.game_hwnd().await?)
        } else {
            None
        };

        self.inject_cursor(&cegui, src).await?;
        trail.push(format!("cursor onto {src_name}"), NativeLevel::NativeCegui);
        // A frame for the hover to register before the press.
        tokio::time::sleep(Duration::from_millis(60)).await;
        if let Some(h) = hwnd {
            self.set_ctrl(h, true).await?;
            tokio::time::sleep(Duration::from_millis(60)).await;
        }
        let mut run = DragRun::default();
        let result = async {
            cegui.button_down(LEFT_BUTTON).await?;
            trail.push(format!("press on {src_name}"), NativeLevel::NativeCegui);
            tokio::time::sleep(Duration::from_millis(100)).await;
            for (i, p) in path(src, dst, steps).into_iter().enumerate() {
                self.inject_cursor(&cegui, p).await?;
                // One step per frame: this read is the round trip that
                // lets the client run a frame before the next move.
                let state = self.lua_json(DRAG_STATE).await?;
                if run.started_at_step.is_none() && drag_active(&state) {
                    run.started_at_step = Some(i + 1);
                }
                tokio::time::sleep(Duration::from_millis(40)).await;
            }
            trail.push(
                format!("move to {dst_name} in {steps} steps"),
                NativeLevel::NativeCegui,
            );
            run.drop_target = cegui.drop_target(container).await?;
            if drop_plan(run.started_at_step.is_some(), run.drop_target, allow_notify)
                == DropPlan::Notify
            {
                cegui.notify_dropped(target, container).await?;
                run.notified = true;
                trail.push(
                    format!("notifyDragDropItemDropped on {dst_name} (no drop target)"),
                    NativeLevel::NativeCall,
                );
            }
            tokio::time::sleep(Duration::from_millis(60)).await;
            Ok::<_, String>(())
        }
        .await;
        // Always let go, even after a failure mid-drag.
        let released = cegui.button_up(LEFT_BUTTON).await;
        if let Some(h) = hwnd {
            self.set_ctrl(h, false).await?;
        }
        result?;
        released?;
        trail.push(format!("release on {dst_name}"), NativeLevel::NativeCegui);
        let wait_t0 = Instant::now();
        let (after, d) = loop {
            tokio::time::sleep(Duration::from_millis(250)).await;
            let after = self.inventory_read(&[], false).await?;
            let d = diff(&before, &after);
            if d["changed"] == json!(true) || wait_t0.elapsed() >= wait {
                break (after, d);
            }
        };
        let started = run.started_at_step.is_some();
        let moved = d["changed"] == json!(true);
        let mut out = json!({
            "from": src_info,
            "to": dst_info,
            "split": split,
            "steps": steps,
            "drag_started": started,
            "drag_started_at_step": run.started_at_step,
            "drop_target_resolved": run.drop_target != 0,
            "drop_notified": run.notified,
            "moved": moved,
            "snap_back": started && !moved,
            "native": {
                "container": format!("{container:#x}"),
                "target": format!("{target:#x}"),
                "drop_target": format!("{:#x}", run.drop_target),
            },
            "diff": d,
            "cash_after": after["cash"],
        });
        trail.stamp(&mut out);
        out["elapsed_ms"] = json!(t0.elapsed().as_millis() as u64);
        Ok(out)
    }
}

#[cfg(test)]
#[path = "drag_drop_tests.rs"]
mod tests;
