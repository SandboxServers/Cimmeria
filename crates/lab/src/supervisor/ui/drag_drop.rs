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
//! the end of a drag of *our* source container (its own dragging byte, not
//! the global `getDragInfo`, which would also report someone else's drag)
//! the tool fires the target window's `DragDropItemDropped` itself
//! (`notifyDragDropItemDropped`) before letting go. The stock handlers then
//! run as for a real drop, but that step is reported as `native_call` (N3),
//! not as input.
//!
//! **What counts as moved.** With slot ends, `moved` means the diff shows
//! the source slot's item leaving it and an item of the same name in the
//! target slot (a move, swap, merge or split); any other inventory change
//! is reported in `diff` but is not this drag's move. With a named-window
//! end the slot is unknown, so any inventory change counts, and
//! `moved_check` says which test ran. `effect_ok` is false when nothing
//! moved, and the UAT runner fails the action on it: a drag that ran but
//! moved nothing is never a native pass.
//!
//! **Stack split.** The stock inventory splits on a **Ctrl**-drag: it pulls
//! one item off the stack (`dragType == 1`, CEGUI button state 9 = left +
//! control). Shift-drag, "choose how many", is a `TODO` in the stock Lua and
//! does nothing, so `split` here means Ctrl-drag, one item.

use std::time::{Duration, Instant};

use serde_json::{json, Value};

use super::inventory::diff;
use super::lua_json::list;
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

/// A slot end as the inventory reader names it: `(container, slot)`.
pub type SlotKey = (String, i64);

/// Whether diff `d` shows this drag's move: the item in `from` changed
/// (left, or lost some of its stack), and `to` now holds an item of the
/// same name (moved in, swapped in, merged or split off).
pub fn landed(d: &Value, from: &SlotKey, to: &SlotKey) -> bool {
    let changes = list(&d["slot_changes"]);
    let change = |k: &SlotKey| {
        changes
            .iter()
            .find(|c| c["container"] == json!(k.0) && c["slot"] == json!(k.1))
            .cloned()
    };
    let Some(src) = change(from) else {
        return false;
    };
    let item = &src["before"];
    if item.is_null() {
        return false;
    }
    change(to).is_some_and(|dst| !dst["after"].is_null() && dst["after"]["name"] == item["name"])
}

/// Several cleanup results as one: every error, in order.
fn all_ok(results: Vec<Result<(), String>>) -> Result<(), String> {
    let errors: Vec<String> = results.into_iter().filter_map(Result::err).collect();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

/// What happened between button down and button up.
#[derive(Debug, Default)]
struct DragRun {
    /// The first step at which our source container was dragging.
    started_at_step: Option<usize>,
    /// Whether `getDragInfo` (global: any item) ever showed a drag.
    drag_info_seen: bool,
    /// Our container was still dragging when the drop was decided.
    dragging_at_drop: bool,
    drop_target: u32,
    notified: bool,
}

impl Supervisor {
    /// Resolve one end to a screen point (revealing a slot as needed).
    async fn drag_point(
        &self,
        end: &DragEnd,
        trail: &mut NativeTrail,
    ) -> Result<((i32, i32), Value, Option<SlotKey>), String> {
        let (name, key) = match end {
            DragEnd::Slot { container, slot } => {
                let loc = self.reveal_slot(container, *slot, trail).await?;
                let key = loc["container"]
                    .as_str()
                    .map(|c| (c.to_string(), i64::from(*slot)));
                (loc["window"].as_str().unwrap_or_default().to_string(), key)
            }
            DragEnd::Window(w) => (w.clone(), None),
        };
        let loc = self
            .lua_json(&locate_chunk(&ClickTarget::Named(name.clone())))
            .await?;
        if loc["found"] != json!(true) || loc["visible"] != json!(true) {
            return Err(format!("{name} is not on screen"));
        }
        let p = click_point(&loc).ok_or_else(|| format!("{name} has no screen rectangle"))?;
        let mut info = json!({ "window": name, "point": [p.0, p.1] });
        if let Some((c, s)) = &key {
            info["container"] = json!(c);
            info["slot"] = json!(s);
        }
        Ok((p, info, key))
    }

    /// Hold Ctrl: the bridge's virtual modifier state, then the key. If the
    /// key cannot be posted, the modifier is cleared again before the error
    /// returns, so a failed split never leaves Ctrl held.
    pub(super) async fn hold_ctrl(&self, hwnd: isize) -> Result<(), String> {
        let ctrl = keys::named("ctrl").expect("ctrl is a named key");
        self.bridge_call("input_modifiers", json!({ "ctrl": true }))
            .await?;
        if let Err(e) = self.post_key(hwnd, ctrl, true).await {
            let cleared = self.release_ctrl(hwnd).await;
            return all_ok(vec![Err(format!("ctrl down: {e}")), cleared]);
        }
        Ok(())
    }

    /// Let go of Ctrl. Both halves always run (the modifier is cleared even
    /// when the key-up cannot be posted), and both errors are reported.
    pub(super) async fn release_ctrl(&self, hwnd: isize) -> Result<(), String> {
        let ctrl = keys::named("ctrl").expect("ctrl is a named key");
        let key_up = self
            .post_key(hwnd, ctrl, false)
            .await
            .map_err(|e| format!("ctrl up: {e}"));
        let cleared = self
            .bridge_release_call("input_modifiers", json!({}))
            .await
            .map(|_| ())
            .map_err(|e| format!("clearing input modifiers: {e}"));
        all_ok(vec![key_up, cleared])
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
        let (_, src_info, src_key) = self.drag_point(from, &mut trail).await?;
        let (dst, dst_info, dst_key) = self.drag_point(to, &mut trail).await?;
        // Revealing the target can hide the source (another tab of the
        // same window): re-read the source without revealing anything.
        let src_name = src_info["window"].as_str().unwrap_or_default().to_string();
        let dst_name = dst_info["window"].as_str().unwrap_or_default().to_string();
        let (src, _, _) = self
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
            self.hold_ctrl(h).await?;
            tokio::time::sleep(Duration::from_millis(60)).await;
        }
        let mut run = DragRun::default();
        let result = async {
            // A press CEGUI did not take landed on no window (or a
            // disabled one): nothing of ours can be dragging, so stop
            // before any drop can be fired.
            if !cegui.button_down(LEFT_BUTTON).await? {
                return Err(format!(
                    "CEGUI did not take the press on {src_name}; nothing was dragged"
                ));
            }
            trail.push(format!("press on {src_name}"), NativeLevel::NativeCegui);
            tokio::time::sleep(Duration::from_millis(100)).await;
            for (i, p) in path(src, dst, steps).into_iter().enumerate() {
                self.inject_cursor(&cegui, p).await?;
                // One step per frame: this read is the round trip that
                // lets the client run a frame before the next move.
                let state = self.lua_json(DRAG_STATE).await?;
                run.drag_info_seen |= drag_active(&state);
                if run.started_at_step.is_none() && cegui.is_dragging(container).await? {
                    run.started_at_step = Some(i + 1);
                }
                tokio::time::sleep(Duration::from_millis(40)).await;
            }
            trail.push(
                format!("move to {dst_name} in {steps} steps"),
                NativeLevel::NativeCegui,
            );
            // Decide on what our container says now, not on the global
            // drag info: a drag of any other item must never be dropped.
            run.dragging_at_drop = cegui.is_dragging(container).await?;
            run.drop_target = cegui.drop_target(container).await?;
            if drop_plan(run.dragging_at_drop, run.drop_target, allow_notify) == DropPlan::Notify {
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
        // Always let go of the button and Ctrl, even after a failure
        // mid-drag, and report every error rather than the first.
        let released = cegui
            .button_up(LEFT_BUTTON)
            .await
            .map(|_| ())
            .map_err(|e| format!("button up: {e}"));
        let ctrl_released = match hwnd {
            Some(h) => self.release_ctrl(h).await,
            None => Ok(()),
        };
        all_ok(vec![result, released, ctrl_released])?;
        trail.push(format!("release on {dst_name}"), NativeLevel::NativeCegui);
        let check = |d: &Value| match (&src_key, &dst_key) {
            (Some(f), Some(t)) => landed(d, f, t),
            _ => d["changed"] == json!(true),
        };
        let wait_t0 = Instant::now();
        let (after, d) = loop {
            tokio::time::sleep(Duration::from_millis(250)).await;
            let after = self.inventory_read(&[], false).await?;
            let d = diff(&before, &after);
            if check(&d) || wait_t0.elapsed() >= wait {
                break (after, d);
            }
        };
        let started = run.started_at_step.is_some();
        let moved = check(&d);
        let moved_check = if src_key.is_some() && dst_key.is_some() {
            "slots"
        } else {
            "any_change"
        };
        let mut out = json!({
            "from": src_info,
            "to": dst_info,
            "split": split,
            "steps": steps,
            "drag_started": started,
            "drag_started_at_step": run.started_at_step,
            "drag_info_seen": run.drag_info_seen,
            "dragging_at_drop": run.dragging_at_drop,
            "drop_target_resolved": run.drop_target != 0,
            "drop_notified": run.notified,
            "moved": moved,
            "moved_check": moved_check,
            "inventory_changed": d["changed"],
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
        // A drag that moved nothing is no pass at any level.
        out["effect_ok"] = json!(moved);
        if !moved {
            out["native_pass"] = json!(false);
        }
        out["elapsed_ms"] = json!(t0.elapsed().as_millis() as u64);
        Ok(out)
    }
}

#[cfg(test)]
#[path = "drag_drop_tests.rs"]
mod tests;
