# LH expansion A: issue #1660 items 1-4 (split, tab switch, item use, vendor buy)

Worktree: `<wt>`. Crate `crates/lab`, package `cimmeria-lab`. Read-only research; nothing was run live.

Stock client Lua (not in the repo, read from disk): `C:\Users\Steve\source\projects\SGW\Client\game\Working\SGWGame\Content\UI\Core\{Inventory\Inventory.lua, Vault\Vault.lua, Bandolier\Bandolier.lua, Vendor\Vendor.lua}`. Line numbers below refer to those files.

## Packets at a glance

| ID | Title | Size | Depends on |
|---|---|---|---|
| LH-01 | Split (Ctrl) drag follows the stand-in drag window; extend the fake client for LH-02..04 | M | none |
| LH-02 | Verified tab/filter clicks: posted, then CEGUI injectors, then the stock handler | M | LH-01 (fake) |
| LH-03 | `client_item_action`: injected right-click first, effect check, Lua fallback, `expect_change` | M | LH-02 |
| LH-04 | `ui_click` resolves any CEGUI window name; new `client_vendor_buy` tool | L | LH-03 |

All four edit `cegui_fake.rs` or `ui/mod.rs`, so they run in series (LH-01 first). LH-01 owns the whole fake extension so the later packets only call it.

---

## Findings (root causes)

### Item 1: split never starts a drag. Cause found by reading the stock Lua. Confidence: high (about 90%).

`client_drag_drop` decides "a drag started" from the **source slot's own** `DragContainer` dragging byte (`Cegui::is_dragging(container)`, `drag_drop.rs:274`) and drops with `notify_dropped(target, container)`. The stock Lua defeats that on a Ctrl-drag. `Inventory.lua:615-669` `InventoryMod.onSlotItemDragStarted`, when `InventoryMod.dragType == 1 and quantity > 1` (line 626):

- line 640 `dragAborted( this )` aborts the slot's drag (CEGUI `onCaptureLost` clears the `+0x23e` byte, findings §8), then
- line 647 `dragStartedFrom(InventoryDragItemWin, mPos)` starts a **different** `DragContainer` (`InventoryDragItemWin`), and
- line 668 `dragItem(containerID, slotID, 1)` sets the global drag info (quantity 1).

So after the move that crosses the threshold the source container reports "not dragging" while `getDragInfo` shows a drag. That is exactly `drag_started=false` with (probably) `drag_info_seen=true`. The tool then reaches `drop_plan(false, ..)` = `Release` and drops nothing, so the N3 `notifyDragDropItemDropped` fallback (`drag_drop.rs:287`) never runs. `Vault.lua:492-521` does the same with `VaultDragItemWin`. `Bandolier.lua:322-344` (`BandolierDragWin`) hands off on **every** drag, split or not, so plain Bandolier drags have the same hole.

Not decided statically (cheap to confirm, probe P1): that Ctrl really reaches the handler as `buttonState == 9` (`Inventory.lua:737`), and that `InventoryDragItemWin`'s vtable is exactly the `DragContainer` vtable `0x01aaedc4` (the packet refuses before pressing anything if it is not).

N1 honesty: once the tool follows the stand-in, a split is N1 only if CEGUI resolves `d_dropTarget` by itself. In the live client `d_dropTarget` is null even for plain drags (findings §8; the `DragDropTarget` flag question is open), so a split will land through `notifyDragDropItemDropped` and be graded N3 (`native_call`) exactly like a plain drag. The guide must say so. Probe P5 reads the flag.

### Item 2: tab click does nothing; drag then dies with "still not on screen after 5 rounds". Cause: partly found.

Found (certain): `slots.rs:239-244` (`reveal_slot`, `Some("click")`) clicks `Inventory_GeneralTabInactive` through `ui_click` (posted `WM_LBUTTONDOWN/UP`), **never checks that the tab changed**, and loops the same click up to `MAX_REVEAL_ROUNDS` (5) times, then reports the misleading "slot N still not on screen". Any click that does not take effect ends there. `InventoryMod.onGeneralClicked` (`Inventory.lua:22-34`) is the `EventClicked` handler of that button (`Inventory.lua:939`), so a click that registers switches the tab at once.

Not decidable statically: *why* the posted click does not register for this button (the ledger shows posted clicks working on Loot All, trainer, Vendor_BuyButton). Candidates: another window covers the tab button's centre and takes the press; the posted button message path (`0x012357d0` -> `0x0056b130`, gated on `SGWUIManager+0x38 != 0`) drops it; a press/release timing issue. Probe P2 tells these apart. The fix does not depend on the answer: verify the effect, then try the CEGUI injectors (`injectMouseButtonDown/Up`, the path the drag already proves), then call the button's own stock handler. Whichever step works is recorded in the trail, so the tier is honest (injectors N1, handler N3).

The "intermittent at 08:08Z" symptom fits a stale active tab left by an earlier step (Crafting active), not randomness.

### Item 3: `client_item_action` use/doubleclick sends nothing but reports native_pass. Two causes found, a third undecided.

1. `doubleclick` can never work on an inventory slot: the stock inventory has **no** double-click handler. The only use path is `Inventory.lua:745-757` `onSlotItemMouseButtonUp` with `mouseButton == 1` (right button) and `InventoryMod.RightClickTarget == this` (set in `onSlotItemMouseButtonDown`, line 732-734) -> `contextSensitiveUseItem(containerID, slotID)`. `item_action.rs:246-249` sends a left double-click for `DoubleClick`.
2. The result is stamped `native_pass: true` whatever the inventory did: `item_action.rs:280-291` never sets `effect_ok`, unlike `drag_drop.rs:357`. That is the "reports native_pass but sends nothing" lie. The UAT runner already fails an action whose result carries `effect_ok: false` (`uat/runner/actions.rs:270`), so the honest fix is to set it.
3. Undecided (probe P3): for `use`/`rightclick` the click is `click_at` = native cursor move + **posted** `WM_RBUTTONDOWN/UP` (`ui/mod.rs:193-212`). The ledger says it sent nothing. Either the posted right button is lost for slot windows (same family as item 2) or the right button reaches CEGUI with a different button index. `Inventory.lua:729` documents `RightButton: mouseButton == 1`, so the CEGUI injector index for right is 1 (left 0 is proven; right is inferred from that comment, not from a live call). The packet makes the injected right-click the first attempt and keeps the posted one as the second.

### Item 4: `__auto_*` children unreachable by name; quantity starts at 0. Cause found.

There are two name resolvers. `client_window_click` uses `__lab_find` (`window_read.rs:44-53`: `_G[name]`, then `getWindow(name)`), and `lab-driver.md:48-49` already records `client_window_click {target: "TutorialWin__auto_closebutton__"}` working. `client_ui_click` / `client_ui_sequence {do: click}` use `widget_rect_chunk` (`input.rs:62-72`), which looks in `_G` only (plus the `Parent/Child` form), so `Vendor_Item1Quantity__auto_incbtn__` is "missing" there. That is the ledger line "client_ui_sequence cannot address CEGUI __auto_* children". Fix: give `widget_rect_chunk` the same `getWindow` fallback. `client_window_click` on the spinner's increment button should already work (probe P4 confirms); the Vendor spinner is a CEGUI `Spinner` whose `__auto_incbtn__` raises the value and fires `EventValueChanged` -> `VendorMod.onSpinnerValueChanged` (`Vendor.lua:30-41`) -> `setVendorSelectedQuantity(i + scrollPos - 1, value)`. So clicking the inc button N times **is** the native path to the same call: N1.

---

## Live probes (the coordinator runs these before the coder starts; a `lab-driver` can run them with `client_batch` Lua steps; ask the user before lab use)

P1 (split, confirm; optional, 3 calls). Setup: a Main slot holding a stack of at least 2, inventory open on General, an empty Main slot.
1. `client_drag_drop {from:{container:"Main",slot:S}, to:{container:"Main",slot:E}, split:true, allow_fallback:false, verbose:true}`. Expected today: `drag_started:false`, `drag_info_seen:true`, `moved:false`. `drag_info_seen:false` as well means Ctrl or the press never reached the stock handler (then the cause is not the hand-off).
2. `client_batch` Lua: `return tostring(InventoryMod.dragType)` right after a manual press is hard; instead repeat step 1 and read `InventoryMod.dragType`: `1` = Ctrl arrived as state 9.
3. `client_batch` Lua `return tostring(InventoryDragItemWin)` then `mem_read` of the address in the userdata, 4 bytes: must equal `0x01aaedc4 + slide` (`module_info.slide`). If it differs, tell the coordinator: LH-01 step 3 must relax `check_drag_container` for the stand-in.

P2 (tab click, item 2). Inventory open, Crafting tab active.
1. `client_window_click {target:"Inventory_GeneralTabInactive"}`; then `client_batch` Lua `return tostring(Inventory_GeneralTabActive:isVisible())`. Expect `false` (the bug).
2. `client_cursor_move` onto the same centre (`rect` from `client_window_read {root:"InventoryWin"}`), then `client_batch` Lua:
   `local r = getWindow('Root'); local p = CEGUI.MouseCursor:getSingleton():getPosition(); for _, f in ipairs({'getTargetChildAtPosition','getChildAtPosition'}) do local ok, w = pcall(function() return r[f](r, p) end); if ok and w then return f, w:getName() end end return 'no binding'`.
   A name other than `Inventory_GeneralTabInactive` = something covers the button (then also record its name). `no binding` = inconclusive.
3. `client_batch` Lua `InventoryMod.onGeneralClicked(Inventory_GeneralTabInactive)` then read step 1's Lua: `true` = the handler works, only the click is lost (the LH-02 ladder ends at N3 on this button).
Outcome table: step 1 `true` (tab switched) = the bug is not reproducible, record and skip; step 2 names another window = that window eats clicks (LH-02 injectors will not help; the handler step carries it); step 2 names the button = the press is being lost in the message path (LH-02 injector step should work, check in the live smoke).

P3 (right-click use, item 3). A consumable in Main slot S, inventory open on General.
1. `client_cursor_move` onto the slot centre, `client_input_mouse {button:1, action:"down"}`, `client_batch` Lua `return tostring(InventoryMod.RightClickTarget)` (`userdata` = the press arrived; `nil` = lost), then `client_input_mouse {button:1, action:"up"}`, then `client_inventory` (qty drops = posted right-click works; the bug is then only the false native_pass).
2. If the press arrived but nothing was used: `client_batch` Lua `return tostring(pcall(contextSensitiveUseItem, 1, S))` (container 1 = Main). Qty drops = the Lua path works (fallback tier N3, as the ledger says).
Outcome: press lost = injector-first ordering matters (LH-03 as written); press arrives, no use = the handler never saw the release (the up message): LH-03 as written still works because the injected pair replaces both.

P4 (vendor spinner, item 4). Vendor window open on Buy, row 1 shown.
1. `client_window_read {root:"Vendor_Item1Quantity", include_hidden:true}`: the child names (`..__auto_incbtn__`, `__auto_decbtn__`, `__auto_editbox__`).
2. `client_window_click {target:"Vendor_Item1Quantity__auto_incbtn__"}`; `client_batch` Lua `return tostring(VendorMod.QuantityWindows[1]:getCurrentValue())`. `1` = N1 spinner works through the existing tool (LH-04 then only adds the helper and the `ui_click` fix). `0` = the click is lost like item 2 (the ladder in `click_until` handles it).
3. `client_ui_sequence [{do:"click", window:"Vendor_Item1Quantity__auto_incbtn__"}]` expect the error "no UI window named" today (the `ui_click` bug).

P5 (can any drag be N1, informational for the guide). Inventory open: `client_batch` Lua `local w = InventoryWin; local out = {}; for _, n in ipairs({'Inventory_AllSlotFrame','InventoryWin'}) do local x = _G[n]; out[#out+1] = n .. '=' .. tostring(x and x:getProperty('DragDropTarget')) end return table.concat(out, ' ')`. `False` everywhere = CEGUI cannot resolve a drop target, so `notifyDragDropItemDropped` (N3) stays the only landing; the guide states that and the owner decides whether a lab-only layout tweak (setup at N3) is acceptable.

---

## Packet LH-01: split drag follows the stand-in window; extend the fake client

Size M. Depends on: none. Commit subject: `fix(lab): follow the stand-in drag window on a split and Bandolier drag (LH-01)`.

### Files
- `crates/lab/src/supervisor/ui/drag_drop.rs`
- `crates/lab/src/supervisor/ui/drag_drop_tests.rs`
- `crates/lab/src/supervisor/cegui_fake.rs` (whole fake extension for LH-01..04)
- `docs/guides/live-research-lab.md` (CRLF), `.claude/agents/lab-driver.md` is not touched here.

### Steps

1. `drag_drop.rs`, imports. Replace
```rust
use crate::supervisor::cegui_native::{drop_plan, DropPlan, LEFT_BUTTON};
```
with
```rust
use crate::supervisor::cegui_native::{drop_plan, Cegui, DropPlan, LEFT_BUTTON};
```

2. `drag_drop.rs`, after `pub const MIN_STEPS: u32 = 3;` add:
```rust
/// Lua body: the stock modules' own Ctrl-drag state (`buttonState == 9`
/// sets `dragType = 1` in `onSlotItemMouseButtonDown`), kept in the result
/// so a split that did nothing can be told from a Ctrl that never arrived.
pub const MOD_DRAG_TYPE: &str = "return __jenc({ inventory = InventoryMod and InventoryMod.dragType, \
     vault = VaultMod and VaultMod.dragType })";

/// The window the stock Lua hands a drag to once CEGUI has started it:
/// `dragAborted(slot)` then `dragStartedFrom(stand_in)`, so the slot's own
/// `DragContainer` stops dragging and the stand-in drags instead
/// (`Inventory.lua:640-647` and `Vault.lua:505-521` on a Ctrl-drag of a
/// stack, `Bandolier.lua:326-335` on every drag). `container` is the source
/// container's name as the inventory reader prints it.
pub fn stand_in_window(container: Option<&str>, split: bool) -> Option<&'static str> {
    match container?.to_ascii_lowercase().as_str() {
        "main" | "mission" | "crafting" if split => Some("InventoryDragItemWin"),
        "vault" if split => Some("VaultDragItemWin"),
        "bandolier" => Some("BandolierDragWin"),
        _ => None,
    }
}

/// How one native drag runs.
#[derive(Debug, Clone, Copy)]
struct DragRunPlan {
    split: bool,
    steps: u32,
    allow_notify: bool,
    wait: Duration,
    /// The game window to post Ctrl to (a split only).
    ctrl_hwnd: Option<isize>,
}

/// Which of our containers is being dragged now: the stand-in when the
/// stock Lua handed the drag over, else the source slot; 0 when neither.
/// Never someone else's drag.
async fn dragged_now(
    cegui: &Cegui<'_>,
    container: u32,
    stand_in: Option<u32>,
) -> Result<u32, String> {
    if let Some(s) = stand_in {
        if cegui.is_dragging(s).await? {
            return Ok(s);
        }
    }
    if cegui.is_dragging(container).await? {
        return Ok(container);
    }
    Ok(0)
}
```

3. `drag_drop.rs`, struct `DragRun`: replace its body with
```rust
#[derive(Debug, Default)]
struct DragRun {
    /// The first step at which one of our containers was dragging.
    started_at_step: Option<usize>,
    /// Whether `getDragInfo` (global: any item) ever showed a drag.
    drag_info_seen: bool,
    /// One of our containers was still dragging when the drop was decided.
    dragging_at_drop: bool,
    /// The stand-in window, not the source slot, held the drag.
    handed_off: bool,
    /// The container that was dragging at the drop (0: none).
    dragged: u32,
    drop_target: u32,
    notified: bool,
    mod_drag_type: Value,
}
```

4. `drag_drop.rs`, replace the whole `pub async fn drag_drop(...)` (from the doc comment `/// \`client_drag_drop\`. \`allow_notify\` permits ...` through its closing `}` before `}` of the impl) with the two functions below. Everything not shown in `drag_native` is the old body unchanged; the full new body is given so nothing needs inference.
```rust
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
        // Only Ctrl needs the window (a posted key); a plain drag is all
        // native calls.
        let ctrl_hwnd = if split {
            Some(self.game_hwnd().await?)
        } else {
            None
        };
        let plan = DragRunPlan {
            split,
            steps,
            allow_notify,
            wait,
            ctrl_hwnd,
        };
        self.drag_native(from, to, plan).await
    }

    async fn drag_native(
        &self,
        from: &DragEnd,
        to: &DragEnd,
        plan: DragRunPlan,
    ) -> Result<Value, String> {
        let t0 = Instant::now();
        let DragRunPlan {
            split,
            steps,
            allow_notify,
            wait,
            ctrl_hwnd: hwnd,
        } = plan;
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
        let stand_in_name = stand_in_window(src_key.as_ref().map(|k| k.0.as_str()), split);
        let stand_in = match stand_in_name {
            Some(n) => Some(cegui.drag_container(n).await?),
            None => None,
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
            if split {
                run.mod_drag_type = self.lua_json(MOD_DRAG_TYPE).await?;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
            for (i, p) in path(src, dst, steps).into_iter().enumerate() {
                self.inject_cursor(&cegui, p).await?;
                // One step per frame: this read is the round trip that
                // lets the client run a frame before the next move.
                let state = self.lua_json(DRAG_STATE).await?;
                run.drag_info_seen |= drag_active(&state);
                if run.started_at_step.is_none() {
                    let now = dragged_now(&cegui, container, stand_in).await?;
                    if now != 0 {
                        run.started_at_step = Some(i + 1);
                        run.handed_off = now != container;
                    }
                }
                tokio::time::sleep(Duration::from_millis(40)).await;
            }
            trail.push(
                format!("move to {dst_name} in {steps} steps"),
                NativeLevel::NativeCegui,
            );
            // Decide on what our containers say now, not on the global
            // drag info: a drag of any other item must never be dropped.
            run.dragged = dragged_now(&cegui, container, stand_in).await?;
            run.dragging_at_drop = run.dragged != 0;
            let dragged = if run.dragged != 0 { run.dragged } else { container };
            run.drop_target = cegui.drop_target(dragged).await?;
            if drop_plan(run.dragging_at_drop, run.drop_target, allow_notify) == DropPlan::Notify {
                cegui.notify_dropped(target, dragged).await?;
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
            "handed_off": run.handed_off,
            "stand_in": stand_in_name,
            "mod_drag_type": run.mod_drag_type,
            "drop_target_resolved": run.drop_target != 0,
            "drop_notified": run.notified,
            "moved": moved,
            "moved_check": moved_check,
            "inventory_changed": d["changed"],
            "snap_back": started && !moved,
            "native": {
                "container": format!("{container:#x}"),
                "stand_in": stand_in.map(|p| format!("{p:#x}")),
                "dragged": format!("{:#x}", run.dragged),
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
```
(`DragRun` derives `Default`: `Value` defaults to `Null`, fine.)

5. `drag_drop.rs` module doc: replace the `//! **Stack split.**` paragraph (the last doc paragraph, 4 lines) with
```rust
//! **Stack split.** The stock inventory splits on a **Ctrl**-drag: it pulls
//! one item off the stack (`dragType == 1`, CEGUI button state 9 = left +
//! control). Shift-drag, "choose how many", is a `TODO` in the stock Lua and
//! does nothing, so `split` here means Ctrl-drag, one item.
//!
//! **The stand-in window.** On that Ctrl-drag the stock Lua calls
//! `dragAborted` on the slot and `dragStartedFrom(InventoryDragItemWin)`
//! (`VaultDragItemWin` in the vault; `BandolierDragWin` on every Bandolier
//! drag): the slot's own container stops dragging and another
//! `DragContainer` carries the item. The tool resolves that window before
//! pressing ([`stand_in_window`]), reads "is it dragging" and its
//! `d_dropTarget` from whichever of the two holds the drag, and drops onto
//! that one. Reading only the slot (the old code) reported `drag_started:
//! false` and dropped nothing (#1660 item 1).
```

6. `cegui_fake.rs`: the whole fake extension (used by LH-01..04). Apply these hunks.

 a. After `pub const TARGET: u32 = 0x0700_0000;` add
```rust
/// The stand-in `DragContainer` (`InventoryDragItemWin`) the stock Lua hands
/// a split drag to.
pub const STAND_IN: u32 = 0x0800_0000;
const STAND_USERDATA: u32 = 0x0a00_0030;
```
 b. In `pub struct FakeCegui`, before `pub moved: bool,` add
```rust
    /// The stock UI hands the drag to `InventoryDragItemWin`: the source
    /// container stops dragging, the stand-in drags.
    pub stand_in: bool,
    /// A native right click (button 1) on `SrcSlot` uses the item.
    pub right_click_works: bool,
    /// A native left click on `TabBtn` shows the tab.
    pub tab_click_works: bool,
    pub tab_visible: bool,
    /// `contextSensitiveUseItem` calls and tab-handler calls made through Lua.
    pub lua_use_calls: usize,
    pub tab_handler_calls: usize,
    /// Vendor: the row-1 spinner value, whether its inc button works,
    /// whether Buy bought, and `setCurrentValue` calls through Lua.
    pub vendor_value: i64,
    pub inc_works: bool,
    pub bought: bool,
    pub lua_set_calls: usize,
```
 c. In `impl Default`, before `moved: false,` add `stand_in: false, right_click_works: false, tab_click_works: false, tab_visible: false, lua_use_calls: 0, tab_handler_calls: 0, vendor_value: 0, inc_works: false, bought: false, lua_set_calls: 0,` (one field per line).
 d. Replace `fn drag_flags` with
```rust
    /// The source container's flag word as the live client lays it out.
    fn drag_flags(&self) -> u32 {
        let mut w = u32::from(self.button_down);
        if self.dragging && !self.stand_in {
            w |= 0x0001_0100;
        }
        w
    }

    /// The stand-in's flag word: dragging only after the hand-off.
    fn stand_in_flags(&self) -> u32 {
        if self.dragging && self.stand_in {
            0x0001_0101
        } else {
            0
        }
    }
```
 e. In `fn memory`, add four entries to the `HashMap::from([...])`:
```rust
            (STAND_USERDATA, STAND_IN),
            (STAND_IN, rebase(self.container_vtable, self.slide)),
            (STAND_IN + DROP_TARGET_OFFSET, self.drop_target),
            (STAND_IN + DRAG_FLAGS_OFFSET, self.stand_in_flags()),
```
 f. In `call_native`, replace the `INJECT_MOUSE_BUTTON_UP_VA` branch with
```rust
        } else if addr == at(INJECT_MOUSE_BUTTON_UP_VA) {
            if self.dragging && self.drop_target != 0 {
                self.moved = true;
            }
            // Clicks (no drag): which window the cursor is over decides.
            let (x, y) = self.cursor;
            if args[1] == 1 && self.right_click_works && x < 20.0 && y < 20.0 {
                self.moved = true;
            }
            if args[1] == 0 && self.tab_click_works && (300.0..320.0).contains(&x) {
                self.tab_visible = true;
            }
            if args[1] == 0 && self.inc_works && (400.0..420.0).contains(&x) {
                self.vendor_value += 1;
            }
            if args[1] == 0 && (500.0..540.0).contains(&x) && self.vendor_value >= 1 {
                self.bought = true;
            }
            self.button_down = false;
            self.dragging = false;
            self.calls.push(format!("up {}", args[1]));
```
 g. In `fn lua`, replace the `let window = |c: &str| {...};` closure with
```rust
        let window = |c: &str| {
            if c.contains("\"SrcSlot\"") {
                Some(("SrcSlot", [0, 0, 20, 20], SRC_USERDATA))
            } else if c.contains("\"DstSlot\"") {
                Some(("DstSlot", [100, 0, 120, 20], DST_USERDATA))
            } else if c.contains("\"InventoryDragItemWin\"") {
                Some(("InventoryDragItemWin", [200, 0, 220, 20], STAND_USERDATA))
            } else if c.contains("\"TabBtn\"") {
                Some(("TabBtn", [300, 0, 320, 20], 0))
            } else if c.contains("\"Vendor_Item1Quantity__auto_incbtn__\"") {
                Some(("Vendor_Item1Quantity__auto_incbtn__", [400, 0, 420, 20], 0))
            } else if c.contains("\"Vendor_BuyButton\"") {
                Some(("Vendor_BuyButton", [500, 0, 540, 20], 0))
            } else {
                None
            }
        };
```
 h. In `fn lua`, change the cash in the inventory doc: replace `json!({ "cash": 0, "containers": [{ "name": "Main",` with `json!({ "cash": if self.bought { -50 } else { 0 }, "containers": [{ "name": "Main",`.
 i. In `fn lua`, insert these branches immediately before `} else if chunk.contains("getDragInfo") {`:
```rust
        } else if chunk.contains("InventoryMod") && chunk.contains("dragType") {
            json!({ "inventory": 1 })
        } else if chunk.contains("contextSensitiveUseItem") {
            self.lua_use_calls += 1;
            self.moved = true;
            json!({ "ok": true })
        } else if chunk.contains("pcall(f, w)") {
            self.tab_handler_calls += 1;
            self.tab_visible = true;
            json!({ "ok": true })
        } else if chunk.contains("done = w ~= nil") {
            json!({ "done": self.tab_visible })
        } else if chunk.contains("vendor_open") {
            json!({ "vendor_open": true, "buy_visible": true, "row_visible": true,
                    "current": self.vendor_value, "max": 10, "scroll": 0 })
        } else if chunk.contains("done = cur ==") {
            let want: i64 = chunk
                .split("done = cur == ")
                .nth(1)
                .map(|s| s.chars().take_while(char::is_ascii_digit).collect::<String>())
                .and_then(|s| s.parse().ok())
                .unwrap_or(-1);
            json!({ "done": self.vendor_value == want })
        } else if chunk.contains("setCurrentValue(") {
            let n: i64 = chunk
                .split("setCurrentValue(")
                .nth(1)
                .map(|s| s.chars().take_while(char::is_ascii_digit).collect::<String>())
                .and_then(|s| s.parse().ok())
                .unwrap_or(-1);
            self.lua_set_calls += 1;
            self.vendor_value = n;
            json!({ "ok": true, "via": "spinner", "current": n })
```
 Note the `InventoryMod`+`dragType` branch must come before `getDragInfo` (the drag-state chunk contains `getDragInfo` but not `InventoryMod ... dragType`, so order is safe either way).

7. `drag_drop_tests.rs`: add at the end (needs `use crate::supervisor::cegui_fake::STAND_IN;`; extend the existing `use crate::supervisor::cegui_fake::{self, FakeCegui, CONTAINER, TARGET};` to `{self, FakeCegui, CONTAINER, STAND_IN, TARGET}`):
```rust
fn run_plan(split: bool, allow_notify: bool) -> DragRunPlan {
    DragRunPlan {
        split,
        steps: 4,
        allow_notify,
        wait: Duration::ZERO,
        ctrl_hwnd: None,
    }
}

#[test]
fn stand_in_windows_follow_the_stock_lua() {
    assert_eq!(
        stand_in_window(Some("Main"), true),
        Some("InventoryDragItemWin")
    );
    assert_eq!(stand_in_window(Some("Main"), false), None);
    assert_eq!(
        stand_in_window(Some("vault"), true),
        Some("VaultDragItemWin")
    );
    assert_eq!(
        stand_in_window(Some("Bandolier"), false),
        Some("BandolierDragWin")
    );
    assert_eq!(stand_in_window(None, true), None);
}

/// Regression guard (#1660 item 1): on a Ctrl-drag the stock Lua aborts the
/// slot's drag and a stand-in window carries it. The tool must follow the
/// stand-in: it started, it was dragging at the drop, and the explicit drop
/// is fired with the stand-in as the dragged item. Reading only the slot
/// (the old code) gives drag_started false and no drop at all.
#[tokio::test]
async fn a_split_drag_follows_the_stand_in_the_stock_ui_hands_it_to() {
    let s = Arc::new(Mutex::new(FakeCegui {
        stand_in: true,
        ..FakeCegui::default()
    }));
    let sup = fake_bridge::supervisor(cegui_fake::responder(s.clone())).await;
    let out = sup
        .drag_native(&main_slot(1), &main_slot(2), run_plan(true, true))
        .await
        .unwrap();
    let notify = format!("notify {TARGET:#x} {STAND_IN:#x}");
    assert_eq!(
        s.lock().unwrap().edges(),
        vec!["down 0", notify.as_str(), "up 0"]
    );
    assert_eq!(out["stand_in"], "InventoryDragItemWin");
    assert_eq!(out["drag_started"], true);
    assert_eq!(out["handed_off"], true);
    assert_eq!(out["dragging_at_drop"], true);
    assert_eq!(out["drop_notified"], true);
    assert_eq!(out["moved"], true);
    assert_eq!(out["effect_ok"], true);
    assert_eq!(out["mod_drag_type"]["inventory"], 1);
    assert_eq!(out["native_tier"], "N3");
}

/// When CEGUI resolves the target, the stand-in drops by itself: N1.
#[tokio::test]
async fn a_split_with_a_resolved_target_is_a_native_pass() {
    let s = Arc::new(Mutex::new(FakeCegui {
        stand_in: true,
        drop_target: TARGET,
        ..FakeCegui::default()
    }));
    let sup = fake_bridge::supervisor(cegui_fake::responder(s.clone())).await;
    let out = sup
        .drag_native(&main_slot(1), &main_slot(2), run_plan(true, true))
        .await
        .unwrap();
    assert_eq!(s.lock().unwrap().edges(), vec!["down 0", "up 0"]);
    assert_eq!(out["moved"], true);
    assert_eq!(out["native_pass"], true);
}
```

### Test: why it fails when reverted
`a_split_drag_follows_the_stand_in_the_stock_ui_hands_it_to`: if `dragged_now` only looks at the source container, the fake (`stand_in: true`) reports the source as not dragging, so `drag_started` is false, `drop_plan(false, ..)` is `Release`, no `notify` is issued and `moved` stays false; every assertion above fails.

### Lane checks (from the worktree)
```text
pwsh -NoProfile -File <wt>\tools\build-lane\lane.ps1 cargo fmt -p cimmeria-lab
pwsh -NoProfile -File <wt>\tools\build-lane\lane.ps1 cargo clippy -p cimmeria-lab --all-targets -- -D warnings
pwsh -NoProfile -File <wt>\tools\build-lane\lane.ps1 cargo nextest run -p cimmeria-lab drag
```
(`Set-Location` to the worktree in each call, per the lane brief.)

### Docs (`docs/guides/live-research-lab.md`, CRLF)
In the `client_drag_drop` row of the tool table (the row that ends `...the drag snaps back.` at about line 918), append to the first cell, after `...the UAT runner fails the action.`:
`On a Ctrl-drag of a stack (and on every Bandolier drag) the stock Lua hands the drag to a stand-in window (InventoryDragItemWin, VaultDragItemWin, BandolierDragWin); the tool follows it (result fields stand_in, handed_off, mod_drag_type) and a split lands as native_cegui only when CEGUI resolves the drop target itself, otherwise through the explicit drop as native_call (N3), like a plain drag.`
In the first-live-check item 6 (`6. \`client_drag_drop\` between two \`Main\` slots ...`) append: `with split: true on a stack of 2 or more, handed_off is true and mod_drag_type.inventory is 1.`

### Live smoke (close-out lab UAT, ask the user first)
`client_drag_drop {from:{container:"Main",slot:S}, to:{container:"Main",slot:E}, split:true}` with a stack of at least 2 in S: expect `drag_started:true`, `handed_off:true`, `moved:true`, `diff` showing qty -1 in S and a 1-stack in E; server evidence: ClickHouse `move_accepted kind=split` for the player at that time. `native_tier` is N3 unless `drop_target_resolved:true`.

---

## Packet LH-02: verified tab/filter clicks

Size M. Depends on: LH-01 (the fake). Commit subject: `fix(lab): verify inventory tab clicks and escalate to injected clicks and the stock handler (LH-02)`.

### Files
- `crates/lab/src/supervisor/cegui_native.rs`
- `crates/lab/src/supervisor/ui/click_checked.rs` (new)
- `crates/lab/src/supervisor/ui/mod.rs`
- `crates/lab/src/supervisor/ui/slots.rs`
- `docs/guides/live-research-lab.md`, `docs/guides/automated-uat.md`

### Steps

1. `cegui_native.rs`, after `pub const LEFT_BUTTON: u32 = 0;` add
```rust
/// CEGUI `RightButton`. `Inventory.lua:729` documents `RightButton:
/// mouseButton == 1`; left is 0 (proven live), so right is 1 and middle 2.
pub const RIGHT_BUTTON: u32 = 1;
/// CEGUI `MiddleButton`.
pub const MIDDLE_BUTTON: u32 = 2;

/// The CEGUI button index for the lab's button number (0 left, 1 right, 2
/// middle).
pub fn cegui_button(button: usize) -> Result<u32, String> {
    match button {
        0 => Ok(LEFT_BUTTON),
        1 => Ok(RIGHT_BUTTON),
        2 => Ok(MIDDLE_BUTTON),
        other => Err(format!("mouse button {other} (0 left, 1 right, 2 middle)")),
    }
}
```
and in `impl Cegui<'_>`, after `button_up`, add
```rust
    /// A whole click at the CEGUI cursor: injected button down, `hold`, button
    /// up. True when CEGUI took the press.
    pub async fn click(&self, button: u32, hold: std::time::Duration) -> Result<bool, String> {
        let taken = self.button_down(button).await?;
        tokio::time::sleep(hold).await;
        self.button_up(button).await?;
        Ok(taken)
    }
```

2. `ui/mod.rs`: add `pub mod click_checked;` after `pub mod chat_log;`. In `impl Supervisor`, after `click_at` add
```rust
    /// [`Self::click_at`] through CEGUI's own button injectors instead of
    /// posted window messages: cursor onto `(x, y)` (native `MouseMove`),
    /// then `injectMouseButtonDown/Up` at the CEGUI cursor. N1
    /// (`NativeLevel::NativeCegui`). Errors when CEGUI did not take the press.
    pub async fn click_at_native(
        &self,
        x: i32,
        y: i32,
        button: usize,
        double: bool,
    ) -> Result<CursorPlaced, String> {
        let b = crate::supervisor::cegui_native::cegui_button(button)?;
        let placed = self.place_cursor(x, y).await?;
        let cegui = self.cegui().await?;
        // A frame for the hover to register before the press.
        tokio::time::sleep(Duration::from_millis(40)).await;
        let hold = Duration::from_millis(super::input::DEFAULT_HOLD_MS);
        for i in 0..(if double { 2 } else { 1 }) {
            if i > 0 {
                tokio::time::sleep(Duration::from_millis(60)).await;
            }
            if !cegui.click(b, hold).await? {
                return Err(format!("CEGUI did not take the press at ({x}, {y})"));
            }
        }
        Ok(placed)
    }
```

3. New file `crates/lab/src/supervisor/ui/click_checked.rs` (module doc says what it is; the test fake lives in `cegui_fake`):
```rust
//! Clicking a window until its effect shows, escalating through the lab's
//! input paths and saying which one worked.
//!
//! A posted button message can be lost on one window and work on the next
//! (#1660: a click on `Inventory_GeneralTabInactive` registered as native
//! input and changed nothing). So a click whose effect can be read back is
//! tried in order, each step followed by the readback:
//!
//! 1. `ui_click`: native cursor move, posted button messages (N1);
//! 2. the same point through CEGUI's own button injectors (N1);
//! 3. the button's own stock `EventClicked` handler called from Lua (N3),
//!    only when the caller names one.
//!
//! The trail records the step that worked, so a tab opened by step 3 makes
//! the surrounding tool N3 and says why.

use std::time::Duration;

use serde_json::json;

use super::slots::lua_path;
use super::window_click::{click_point, locate_chunk, ClickTarget};
use super::{NativeLevel, NativeTrail, Supervisor};
use crate::supervisor::flows::widgets::lua_quote;

/// A frame or two for the client to apply a click.
const SETTLE: Duration = Duration::from_millis(200);

/// Lua body for the readback "is window `name` visible": `{ done }`.
pub fn visible_chunk(name: &str) -> String {
    format!(
        "local w = _G[{n}] if w == nil then w = __jcall(getWindow, {n}) end \
         return __jenc({{ done = w ~= nil and __jcall(function() return w:isVisible() end) == true }})",
        n = lua_quote(name)
    )
}

/// Lua body that calls a stock click handler (`InventoryMod.onGeneralClicked`)
/// with the button, as the button's `EventClicked` would: `{ ok }`.
pub fn handler_chunk(handler: &str, window: &str) -> String {
    format!(
        "local f = {lookup} local w = _G[{w}] \
         local ok = type(f) == 'function' and w ~= nil and pcall(f, w) \
         return __jenc({{ ok = ok == true }})",
        lookup = lua_path(handler),
        w = lua_quote(window)
    )
}

impl Supervisor {
    async fn click_took_effect(&self, done: &str) -> Result<bool, String> {
        tokio::time::sleep(SETTLE).await;
        Ok(self.lua_json(done).await?["done"] == json!(true))
    }

    /// Click `window` until the readback `done` (a Lua body returning
    /// `{ done = <bool> }`) says it worked. `handler` is the stock Lua
    /// handler to call as the last resort. Errors name every attempt.
    pub async fn click_until(
        &self,
        window: &str,
        done: &str,
        handler: Option<&str>,
        trail: &mut NativeTrail,
    ) -> Result<(), String> {
        let mut tried: Vec<String> = Vec::new();
        match self.ui_click(window, 0).await {
            Ok(_) => {
                if self.click_took_effect(done).await? {
                    trail.push(format!("click {window}"), NativeLevel::RealInput);
                    return Ok(());
                }
                tried.push("posted click: no effect".to_string());
            }
            Err(e) => tried.push(format!("posted click: {e}")),
        }
        let loc = self
            .lua_json(&locate_chunk(&ClickTarget::Named(window.to_string())))
            .await?;
        match click_point(&loc) {
            Some((x, y)) => match self.click_at_native(x, y, 0, false).await {
                Ok(placed) => {
                    if self.click_took_effect(done).await? {
                        placed.note_fallback(trail);
                        trail.push(
                            format!("click {window} (CEGUI injectors)"),
                            NativeLevel::NativeCegui,
                        );
                        return Ok(());
                    }
                    tried.push("injected click: no effect".to_string());
                }
                Err(e) => tried.push(format!("injected click: {e}")),
            },
            None => tried.push(format!("{window} has no screen rectangle")),
        }
        if let Some(h) = handler {
            let r = self.lua_json(&handler_chunk(h, window)).await?;
            if r["ok"] == json!(true) && self.click_took_effect(done).await? {
                trail.push(format!("call {h}"), NativeLevel::ClientUiLua);
                return Ok(());
            }
            tried.push(format!("{h}: no effect"));
        }
        Err(format!(
            "{window} was clicked but the expected change did not show ({})",
            tried.join("; ")
        ))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::supervisor::cegui_fake::{self, FakeCegui};
    use crate::supervisor::events::fake_bridge;

    async fn run(
        state: FakeCegui,
        handler: Option<&str>,
    ) -> (Result<(), String>, NativeTrail, Arc<Mutex<FakeCegui>>) {
        let s = Arc::new(Mutex::new(state));
        let sup = fake_bridge::supervisor(cegui_fake::responder(s.clone())).await;
        let mut trail = NativeTrail::default();
        let r = sup
            .click_until("TabBtn", &visible_chunk("TabActive"), handler, &mut trail)
            .await;
        (r, trail, s)
    }

    /// The posted click is unavailable in the fake (no game window), the
    /// injected one works: N1, the handler is never called.
    #[tokio::test]
    async fn an_injected_click_is_the_second_step_and_stays_n1() {
        let (r, trail, s) = run(
            FakeCegui {
                tab_click_works: true,
                ..FakeCegui::default()
            },
            Some("InventoryMod.onGeneralClicked"),
        )
        .await;
        r.unwrap();
        assert_eq!(trail.overall(), Some(NativeLevel::NativeCegui));
        let s = s.lock().unwrap();
        assert_eq!(s.edges(), vec!["down 0", "up 0"]);
        assert_eq!(s.tab_handler_calls, 0);
    }

    /// Neither click registers: the stock handler finishes the job and the
    /// trail says N3, so a drag that needed it is not a native pass.
    #[tokio::test]
    async fn the_stock_handler_is_the_last_resort_and_is_n3() {
        let (r, trail, s) = run(
            FakeCegui::default(),
            Some("InventoryMod.onGeneralClicked"),
        )
        .await;
        r.unwrap();
        assert_eq!(trail.overall(), Some(NativeLevel::ClientUiLua));
        assert_eq!(s.lock().unwrap().tab_handler_calls, 1);
    }

    /// No step works and there is no handler: an error that names the
    /// attempts, not five silent retries.
    #[tokio::test]
    async fn a_click_that_never_takes_effect_is_an_error_naming_the_attempts() {
        let (r, _, _) = run(FakeCegui::default(), None).await;
        let e = r.unwrap_err();
        assert!(e.contains("did not show"), "{e}");
        assert!(e.contains("injected click: no effect"), "{e}");
    }
}
```

4. `slots.rs`:
 a. Replace `toggle_chunk` and add `lua_path` (keep the existing tests green; `toggle_chunk` output is unchanged):
```rust
/// Lua expression for a dotted global path (`InventoryMod.onToggleInventory`)
/// that is nil instead of an error when a part is missing.
pub fn lua_path(handler: &str) -> String {
    let path: Vec<&str> = handler.split('.').collect();
    path.iter()
        .skip(1)
        .fold(format!("_G[{}]", lua_quote(path[0])), |acc, p| {
            format!("({acc} or {{}})[{}]", lua_quote(p))
        })
}

/// Lua body that calls a host window's toggle handler (`InventoryMod.onToggleInventory`).
pub fn toggle_chunk(handler: &str) -> String {
    let lookup = lua_path(handler);
    format!(
        "local f = {lookup} local ok = type(f) == 'function' and pcall(f, nil) \
         return __jenc({{ ok = ok == true }})"
    )
}
```
 b. In `locate_chunk`'s Lua, replace the `local inv = ...` table and the two `need = 'click'` lines with (each entry gets handler names as items 5 and 6; the `click_expect` is the Active window that must become visible):
```lua
local inv = {{ [C.Main or -1] = {{ 'Inventory_GeneralTabActive', 'Inventory_GeneralTabInactive', 'Inventory_FilterAllActive', 'Inventory_FilterAllInactive', 'InventoryMod.onGeneralClicked', 'InventoryMod.onFilterGeneralAll' }},
              [C.Mission or -2] = {{ 'Inventory_MissionTabActive', 'Inventory_MissionTabInactive', 'Inventory_FilterMissionAllActive', 'Inventory_FilterMissionAllInactive', 'InventoryMod.onMissionClicked', 'InventoryMod.onFilterMissionAll' }},
              [C.Crafting or -3] = {{ 'Inventory_CraftingTabActive', 'Inventory_CraftingTabInactive', 'Inventory_FilterCraftingAllActive', 'Inventory_FilterCraftingAllInactive', 'InventoryMod.onCraftingClicked', 'InventoryMod.onFilterCraftingAll' }} }}
```
and
```lua
  if not vis(_G[t[1]]) then res.need = 'click' res.click = t[2] res.click_expect = t[1] res.click_handler = t[5] return __jenc(res) end
  if not vis(_G[t[3]]) then res.need = 'click' res.click = t[4] res.click_expect = t[3] res.click_handler = t[6] return __jenc(res) end
```
(this lives inside the `format!` raw string, so keep the doubled braces `{{ }}` as in the surrounding lines.) The equipment line becomes
```lua
  if not vis(Character_EquipmentTabActive) then res.need = 'click' res.click = 'Character_EquipmentTabInactive' res.click_expect = 'Character_EquipmentTabActive' return __jenc(res) end
```
 c. Update the doc comment above `locate_chunk` (`... scroll_bar, scroll_to, window, ...`) to add `click_expect, click_handler` to the list of returned fields.
 d. In `reveal_slot`, replace the whole `Some("click") => { ... }` arm with
```rust
                Some("click") => {
                    let w = loc["click"].as_str().unwrap_or_default().to_string();
                    let expect = loc["click_expect"].as_str().unwrap_or_default().to_string();
                    let handler = loc["click_handler"].as_str().map(str::to_string);
                    self.click_until(&w, &super::click_checked::visible_chunk(&expect), handler.as_deref(), trail)
                        .await?;
                }
```
 e. Add test in `slots.rs` `mod tests`:
```rust
    #[test]
    fn tab_clicks_carry_their_readback_and_stock_handler() {
        let c = locate_chunk(&ContainerRef::Name("Main".into()), 12);
        assert!(c.contains("res.click_expect = t[1] res.click_handler = t[5]"));
        assert!(c.contains("'InventoryMod.onGeneralClicked'"));
        assert!(c.contains("res.click_expect = 'Character_EquipmentTabActive'"));
        assert_eq!(
            lua_path("InventoryMod.onGeneralClicked"),
            r#"(_G["InventoryMod"] or {})["onGeneralClicked"]"#
        );
    }
```

### Test: why it fails when reverted
The three `click_checked` tests exercise the ladder; with the old `reveal_slot` (one posted click, no readback) there is no code that clicks through the injectors or calls the handler, so a tab whose posted click is lost is never switched. The `slots.rs` test pins the Lua that tells `reveal_slot` what to wait for.

### Lane checks
`cargo fmt -p cimmeria-lab`, `cargo clippy -p cimmeria-lab --all-targets -- -D warnings`, `cargo nextest run -p cimmeria-lab click_checked slots cegui` (same `lane.ps1` prefix as LH-01).

### Docs
`docs/guides/live-research-lab.md`, the `client_item_action` first-cell text (about line 917, `...and the right tab and the All filter are clicked first.`): replace `are clicked first` with `are clicked first; each tab or filter click is read back and, if the posted click had no effect, repeated through CEGUI's button injectors (N1) and finally through the button's stock handler (client_ui_lua, N3), so a lost click is never five silent retries`.
`docs/guides/automated-uat.md`, after the `A spec may declare a known tool *less* native than its floor ...` paragraph (about line 139) add:
`Tools that put a window on screen first (item and drag tools) escalate a click whose effect they can read back: posted click, then the CEGUI button injectors (both N1), then the button's own stock handler (N3). The step that worked is in native_steps; a tool that needed the handler reports N3.`

### Live smoke
Crafting tab active, then `client_drag_drop` of two Main slots: expect success with `native_steps` containing `click Inventory_GeneralTabInactive` (RealInput or `(CEGUI injectors)`, N1) and no `call InventoryMod.onGeneralClicked`. If the handler step shows, record that in the ledger: the click path for that button is broken at N1 and the guide must say so (probe P2 explains why).

---

## Packet LH-03: `client_item_action` honest use

Size M. Depends on: LH-02 (`click_at_native`, `cegui_button`). Commit subject: `fix(lab): item use tries the injected right-click first and reports effect_ok (LH-03)`.

### Files
- `crates/lab/src/supervisor/ui/item_action.rs`
- `crates/lab/src/server/ui/args.rs`
- `crates/lab/src/server/ui/mod.rs`
- `docs/guides/live-research-lab.md`, `docs/guides/automated-uat.md`

### Steps

1. `item_action.rs` imports. Replace `use super::inventory::{diff, find_items};` block with
```rust
use super::inventory::{diff, find_items};
use super::slots::ContainerRef;
use super::window_click::{click_point, locate_chunk, ClickTarget};
use super::{NativeLevel, NativeTrail, Supervisor};
```
(delete the now duplicate `use super::slots::ContainerRef;` line; the later `super::window_click::...` full paths in the loot arm may stay).

2. Add above `impl Supervisor` (after `LOOT_STATE`):
```rust
/// Lua body: the stock right-click handler, `contextSensitiveUseItem`, called
/// the way `InventoryMod.onSlotItemMouseButtonUp` calls it
/// (`Inventory.lua:753`): `{ ok }`.
pub fn use_item_chunk(container: &ContainerRef, slot: u32) -> String {
    format!(
        "local cid = {cid} if cid == nil then return __jenc({{ ok = false }}) end \
         local ok = type(contextSensitiveUseItem) == 'function' and pcall(contextSensitiveUseItem, cid, {slot}) \
         return __jenc({{ ok = ok == true }})",
        cid = container.lua_id()
    )
}
```

3. Change `await_inventory_change` from `async fn` to `pub(super) async fn` (LH-04 reuses it).

4. Signature: replace
```rust
        loot_index: Option<u32>,
        allow_fallback: bool,
        wait: Duration,
    ) -> Result<Value, String> {
        let t0 = Instant::now();
        let mut trail = NativeTrail::default();
```
with
```rust
        loot_index: Option<u32>,
        allow_fallback: bool,
        expect_change: bool,
        wait: Duration,
    ) -> Result<Value, String> {
        let t0 = Instant::now();
        let mut trail = NativeTrail::default();
        // A step that already saw the inventory change (so the tail need
        // not wait again).
        let mut settled: Option<(Value, Value)> = None;
```
(`clippy::too_many_arguments` limit is 7 including self; this is self + 6 = 7, fine.)

5. Replace the whole `_ => { ... }` arm (from `_ => {` after the loot arm to its closing brace before `}` of the `match verb`) with:
```rust
            _ => {
                let (container, slot, item_id, name) = select_item(&before, sel)?;
                out["item"] = json!({ "container": container, "slot": slot, "item_id": item_id, "name": name });
                // The stock inventory has no double-click handler on a slot
                // (`Inventory.lua:745-757` acts on the right button only), so
                // a double-click would do nothing: do the right-click and say so.
                let verb = if verb == ItemVerb::DoubleClick {
                    out["requested"] = json!("doubleclick");
                    out["performed"] = json!("rightclick");
                    ItemVerb::RightClick
                } else {
                    verb
                };
                let revealed = self
                    .reveal_slot(&ContainerRef::Name(container.clone()), slot, &mut trail)
                    .await;
                match revealed {
                    Ok(loc) => {
                        let window = loc["window"].as_str().unwrap_or_default().to_string();
                        let r = self
                            .lua_json(&locate_chunk(&ClickTarget::Named(window.clone())))
                            .await?;
                        let (x, y) = click_point(&r)
                            .ok_or_else(|| format!("{window} has no screen rectangle"))?;
                        let step_wait = wait / 2;
                        let mut attempts: Vec<String> = Vec::new();
                        // 1. CEGUI's own button injectors (N1).
                        match self.click_at_native(x, y, 1, false).await {
                            Ok(placed) => {
                                let (a, d) = self.await_inventory_change(&before, step_wait).await?;
                                if d["changed"] == json!(true) {
                                    placed.note_fallback(&mut trail);
                                    trail.push(
                                        format!("right-click {window} (CEGUI injectors)"),
                                        NativeLevel::NativeCegui,
                                    );
                                    settled = Some((a, d));
                                } else {
                                    attempts.push("injected right-click: no inventory change".into());
                                }
                            }
                            Err(e) => attempts.push(format!("injected right-click: {e}")),
                        }
                        // 2. Posted button messages (N1).
                        if settled.is_none() {
                            match self.click_at(x, y, 1, false).await {
                                Ok(placed) => {
                                    let (a, d) =
                                        self.await_inventory_change(&before, step_wait).await?;
                                    if d["changed"] == json!(true) {
                                        placed.note_fallback(&mut trail);
                                        trail.push(
                                            format!("right-click {window}"),
                                            NativeLevel::RealInput,
                                        );
                                        settled = Some((a, d));
                                    } else {
                                        attempts.push("posted right-click: no inventory change".into());
                                    }
                                }
                                Err(e) => attempts.push(format!("posted right-click: {e}")),
                            }
                        }
                        // 3. The stock handler the right-click would reach (N3).
                        if settled.is_none() && allow_fallback {
                            let r = self
                                .lua_json(&use_item_chunk(&ContainerRef::Name(container.clone()), slot))
                                .await?;
                            if r["ok"] == json!(true) {
                                let (a, d) = self.await_inventory_change(&before, wait).await?;
                                trail.push(
                                    "contextSensitiveUseItem via Lua",
                                    NativeLevel::ClientUiLua,
                                );
                                settled = Some((a, d));
                            } else {
                                attempts.push("contextSensitiveUseItem: not callable".into());
                            }
                        }
                        out["slot_window"] = json!(window);
                        out["attempts"] = json!(attempts);
                    }
                    Err(e) => {
                        let line = match verb.slash(item_id, 0) {
                            Some(l) if allow_fallback => l,
                            _ => return Err(format!("could not put the item on screen: {e}")),
                        };
                        self.chat_command(&line).await?;
                        trail.push(line, NativeLevel::SlashCommand);
                        out["fallback_reason"] = json!(e);
                    }
                }
            }
```

6. Tail: replace `let (after, d) = self.await_inventory_change(&before, wait).await?;` (just after the match) with
```rust
        let (after, d) = match settled.take() {
            Some(done) => done,
            None => self.await_inventory_change(&before, wait).await?,
        };
```
and after `trail.stamp(&mut out);` and before `out["elapsed_ms"]` insert
```rust
        // An item action that changed nothing is no pass at any level
        // (#1660 item 3). Loot actions have their own counters; a caller who
        // expects no change (a negative check) passes expect_change false.
        if expect_change && !matches!(verb, ItemVerb::LootAll | ItemVerb::LootSlot) {
            let moved = out["inventory_changed"] == json!(true);
            out["effect_ok"] = json!(moved);
            if !moved {
                out["native_pass"] = json!(false);
            }
        }
```
(`verb` was shadowed inside the arm only; the outer `verb` is still the requested one, which is fine for this `matches!`.)

7. `args.rs` `ItemActionArgs`: replace the `allow_fallback` doc+field with
```rust
    /// Fall back to the stock `contextSensitiveUseItem` through Lua (N3) when
    /// the clicks change nothing, and to the client's slash command when the
    /// item cannot be put on screen (default true).
    #[serde(default)]
    pub allow_fallback: Option<bool>,
    /// Whether the action is expected to change the inventory (default
    /// true). When true and nothing changes the result carries effect_ok
    /// false and native_pass false, and the UAT runner fails the action;
    /// pass false for a negative check (an item that must not be usable).
    #[serde(default)]
    pub expect_change: Option<bool>,
```
8. `server/ui/mod.rs` `client_item_action`: pass `a.allow_fallback.unwrap_or(true),` then `a.expect_change.unwrap_or(true),` then the wait. Replace the tool description's first sentences with: `Act on an item like a player. use / equip / unequip / rightclick: a right-click on the item's slot (the stock UI's contextSensitiveUseItem), tried through CEGUI's button injectors, then posted button messages, then (allow_fallback) the stock handler through Lua (client_ui_lua, N3); the inventory or character window is opened with its bound key and the right tab clicked first. doubleclick on a slot is performed as a right-click (the stock inventory has no double-click handler; result fields requested/performed). loot_all: ... ` (keep the rest of the old description). Add at its end: `effect_ok and native_pass are false when the inventory did not change (expect_change false disables the check).`

9. Tests in `item_action.rs` `mod tests` (add imports at the top of the module: `use std::sync::{Arc, Mutex};`, `use crate::supervisor::cegui_fake::{self, FakeCegui};`, `use crate::supervisor::events::fake_bridge;`):
```rust
    fn main_slot_1() -> ItemSelector {
        ItemSelector {
            container: Some("Main".into()),
            slot: Some(1),
            ..Default::default()
        }
    }

    async fn use_item(
        state: FakeCegui,
        allow_fallback: bool,
        expect_change: bool,
    ) -> (Value, Arc<Mutex<FakeCegui>>) {
        let s = Arc::new(Mutex::new(state));
        let sup = fake_bridge::supervisor(cegui_fake::responder(s.clone())).await;
        let out = sup
            .item_action(
                ItemVerb::Use,
                &main_slot_1(),
                None,
                allow_fallback,
                expect_change,
                Duration::ZERO,
            )
            .await
            .unwrap();
        (out, s)
    }

    /// The injected right-click uses the item: N1, one injected click, the
    /// Lua handler never called.
    #[tokio::test]
    async fn an_injected_right_click_that_works_is_a_native_pass() {
        let (out, s) = use_item(
            FakeCegui {
                right_click_works: true,
                ..FakeCegui::default()
            },
            true,
            true,
        )
        .await;
        assert_eq!(out["effect_ok"], true);
        assert_eq!(out["native_level"], "native_cegui");
        assert_eq!(out["native_pass"], true);
        let s = s.lock().unwrap();
        assert_eq!(s.edges(), vec!["down 1", "up 1"]);
        assert_eq!(s.lua_use_calls, 0);
    }

    /// Both clicks are lost: the stock handler uses the item, and the result
    /// says N3 and no native pass.
    #[tokio::test]
    async fn the_lua_handler_is_the_last_resort_and_is_n3() {
        let (out, s) = use_item(FakeCegui::default(), true, true).await;
        assert_eq!(out["effect_ok"], true);
        assert_eq!(out["native_level"], "client_ui_lua");
        assert_eq!(out["native_pass"], false);
        assert_eq!(s.lock().unwrap().lua_use_calls, 1);
    }

    /// Regression guard (#1660 item 3): nothing changed, so the result is no
    /// pass. The old code stamped native_pass true here.
    #[tokio::test]
    async fn a_use_that_changed_nothing_is_no_pass() {
        let (out, _) = use_item(FakeCegui::default(), false, true).await;
        assert_eq!(out["effect_ok"], false);
        assert_eq!(out["native_pass"], false);
        assert_eq!(out["inventory_changed"], false);
        assert!(out["attempts"].as_array().unwrap().len() >= 2);
    }

    /// A negative check opts out of the effect test.
    #[tokio::test]
    async fn expect_change_false_leaves_effect_ok_unset() {
        let (out, _) = use_item(FakeCegui::default(), false, false).await;
        assert!(out.get("effect_ok").is_none());
    }

    #[tokio::test]
    async fn a_double_click_on_a_slot_is_performed_as_a_right_click() {
        let s = Arc::new(Mutex::new(FakeCegui {
            right_click_works: true,
            ..FakeCegui::default()
        }));
        let sup = fake_bridge::supervisor(cegui_fake::responder(s.clone())).await;
        let out = sup
            .item_action(
                ItemVerb::DoubleClick,
                &main_slot_1(),
                None,
                true,
                true,
                Duration::ZERO,
            )
            .await
            .unwrap();
        assert_eq!(out["requested"], "doubleclick");
        assert_eq!(out["performed"], "rightclick");
        assert_eq!(s.lock().unwrap().edges(), vec!["down 1", "up 1"]);
    }
```

### Test: why it fails when reverted
`a_use_that_changed_nothing_is_no_pass` is the item-3 guard: the old function never set `effect_ok` and left `native_pass` true for a RealInput trail whatever the inventory did. `an_injected_right_click_that_works_is_a_native_pass` fails against the old posted-only code because in the fake the posted click cannot be sent and only the injected click uses the item.

### Lane checks
`fmt`, `clippy --all-targets -D warnings`, `cargo nextest run -p cimmeria-lab item_action ui::` (same prefix as before).

### Docs
`docs/guides/live-research-lab.md`:
- Table row (about line 917), `client_item_action` first cell: replace `` `use` / `equip` / `unequip` / `rightclick`: a right-click on the item's slot, which the stock UI turns into `contextSensitiveUseItem`. `` with `` `use` / `equip` / `unequip` / `rightclick`: a right-click on the item's slot, which the stock UI turns into `contextSensitiveUseItem`; it is sent through CEGUI's own button injectors first, then as posted button messages (both N1), each followed by an inventory check. `doubleclick` on a slot is performed as the same right-click (the stock inventory has no double-click handler; `requested`/`performed` say so). ``; append to the cell: `` The result carries `effect_ok` and `native_pass: false` when the inventory did not change (`expect_change: false` for a negative check). ``; in the fallback cell append: `` The clicks changed nothing and `allow_fallback` is on: the stock handler `contextSensitiveUseItem` through Lua (`client_ui_lua`, N3). ``
- First-live-check item 4 (`4. \`client_item_action {action: "use"}\` on a consumable ...`): replace `` `native_level: "real_input"` `` with `` `native_level: "native_cegui"` or `"real_input"`, `effect_ok: true` ``.
`docs/guides/automated-uat.md`: in the paragraph added by LH-02 append: `An item or vendor action whose inventory did not change reports effect_ok: false and the runner fails the action; a negative check passes expect_change: false.`

### Live smoke
Consumable in Main (Health Slappack at reduced HP). `client_item_action {action:"use", name:"Health Slappack", wait_ms:4000}` -> `effect_ok:true`, `native_level` `native_cegui` (or `real_input`), diff `delta -1`. Server evidence: ClickHouse `consumable_used` row for the player, `health_after` greater than `health_before`. If `native_level` is `client_ui_lua`, record it: the click paths are both lost on slot windows (probe P3 says which) and the guide must call item use N3 on this client.

---

## Packet LH-04: `ui_click` name resolution and `client_vendor_buy`

Size L. Depends on: LH-03 (`await_inventory_change` made `pub(super)`, `click_until`). Commit subject: `feat(lab): client_vendor_buy sets the row quantity with the spinner buttons and buys (LH-04)`.

### Files
- `crates/lab/src/supervisor/input.rs`
- `crates/lab/src/supervisor/ui/vendor_buy.rs` (new)
- `crates/lab/src/supervisor/ui/mod.rs`
- `crates/lab/src/server/ui/args.rs`, `crates/lab/src/server/ui/mod.rs`
- `crates/lab/src/lease/policy.rs`, `crates/lab/src/server/flows.rs`, `crates/lab/src/uat/tools.rs`
- `docs/guides/live-research-lab.md`, `docs/guides/automated-uat.md`, `.claude/agents/lab-driver.md`

### Steps

1. `input.rs`, `widget_rect_chunk`: the bare-name branch looked in `_G` only. Replace
```rust
        None => format!("local w = _G[{window:?}]"),
```
with
```rust
        // `_G` first, then CEGUI's own window name (`getWindow`): an
        // auto-created child such as `Vendor_Item1Quantity__auto_incbtn__`
        // is no Lua global but is a named window (#1660 item 4).
        None => format!(
            "local w = _G[{window:?}] \
             if w == nil then local ok, g = pcall(getWindow, {window:?}) if ok then w = g end end"
        ),
```
and add to `mod tests`:
```rust
    /// Regression guard (#1660 item 4): a CEGUI auto child is found by its
    /// window name, not only as a Lua global.
    #[test]
    fn widget_rect_chunk_falls_back_to_the_cegui_window_name() {
        let c = widget_rect_chunk("Vendor_Item1Quantity__auto_incbtn__");
        assert!(c.contains(r#"_G["Vendor_Item1Quantity__auto_incbtn__"]"#));
        assert!(c.contains(r#"pcall(getWindow, "Vendor_Item1Quantity__auto_incbtn__")"#));
    }
```
2. New file `crates/lab/src/supervisor/ui/vendor_buy.rs`:
```rust
//! `client_vendor_buy`: set a vendor row's quantity and press Buy.
//!
//! A vendor row's quantity spinner (`Vendor_Item<N>Quantity`, a CEGUI
//! `Spinner`) starts at 0, and its value is what `vendorBuySelected` buys:
//! `VendorMod.onSpinnerValueChanged` (`Vendor.lua:30-41`) calls
//! `setVendorSelectedQuantity(row + scroll - 1, value)` on every change. So
//! the native way to set a quantity is to click the spinner's own
//! `__auto_incbtn__` / `__auto_decbtn__` until the value is right
//! ([`Supervisor::click_until`] with the value as the readback), then click
//! `Vendor_BuyButton`. Only when the buttons cannot do it (and
//! `allow_fallback`) the value is set through the spinner's Lua
//! (`setCurrentValue`, which fires the same handler) or the
//! `setVendorSelectedQuantity` binding, and the result is N3.

use std::time::{Duration, Instant};

use serde_json::{json, Value};

use super::click_checked::handler_chunk;
use super::inventory::diff;
use super::window_click::{click_point, locate_chunk, ClickTarget};
use super::{NativeLevel, NativeTrail, Supervisor};
use crate::supervisor::flows::widgets::lua_quote;

/// Rows on screen in the stock vendor window.
pub const MAX_ROWS: u32 = 7;
/// Most spinner clicks per call; a larger quantity needs the Lua fallback.
pub const MAX_CLICKS: i64 = 30;
const BUY_BUTTON: &str = "Vendor_BuyButton";

pub fn spinner_name(row: u32) -> String {
    format!("Vendor_Item{row}Quantity")
}

pub fn inc_button(row: u32) -> String {
    format!("{}__auto_incbtn__", spinner_name(row))
}

pub fn dec_button(row: u32) -> String {
    format!("{}__auto_decbtn__", spinner_name(row))
}

/// Lua body: `{ vendor_open, buy_visible, row_visible, current, max, scroll }`.
pub fn state_chunk(row: u32) -> String {
    format!(
        "local sp = _G[{sp}] \
         local scroll = __jcall(function() return math.floor(Vendor_Scrollbar:getProperty('ScrollPosition')) end) or 0 \
         local buy = _G['Vendor_BuyButton'] \
         return __jenc({{ vendor_open = VendorWin ~= nil and __jcall(function() return VendorWin:isVisible() end) == true, \
           buy_visible = buy ~= nil and __jcall(function() return buy:isVisible() end) == true, \
           row_visible = sp ~= nil and __jcall(function() return sp:isVisible() end) == true, \
           current = sp and __jcall(function() return tonumber(sp:getCurrentValue()) end), \
           max = sp and __jcall(function() return tonumber(sp:getMaximumValue()) end), \
           scroll = scroll }})",
        sp = lua_quote(&spinner_name(row))
    )
}

/// Lua body: `{ done }`, true when the row's spinner reads `want`.
pub fn value_is_chunk(row: u32, want: i64) -> String {
    format!(
        "local sp = _G[{sp}] \
         local cur = sp and __jcall(function() return tonumber(sp:getCurrentValue()) end) \
         return __jenc({{ done = cur == {want} }})",
        sp = lua_quote(&spinner_name(row))
    )
}

/// Lua body: set the quantity without the buttons. The spinner's
/// `setCurrentValue` fires the stock handler; if the value still differs,
/// the `setVendorSelectedQuantity(abs0, n)` binding is called directly.
/// `{ ok, via, current }`.
pub fn set_quantity_chunk(row: u32, abs0: i64, n: i64) -> String {
    format!(
        "local sp = _G[{sp}] \
         if sp == nil then return __jenc({{ ok = false }}) end \
         pcall(function() sp:setCurrentValue({n}) end) \
         local cur = __jcall(function() return tonumber(sp:getCurrentValue()) end) \
         if cur == {n} then return __jenc({{ ok = true, via = 'spinner', current = cur }}) end \
         local ok = pcall(setVendorSelectedQuantity, {abs0}, {n}) \
         cur = __jcall(function() return tonumber(sp:getCurrentValue()) end) \
         return __jenc({{ ok = ok, via = 'setVendorSelectedQuantity', current = cur }})",
        sp = lua_quote(&spinner_name(row))
    )
}

impl Supervisor {
    /// Click Buy and wait for the inventory or cash to change: posted
    /// click, then the injectors, then (allow_fallback) the stock handler.
    async fn press_buy(
        &self,
        before: &Value,
        allow_fallback: bool,
        wait: Duration,
        trail: &mut NativeTrail,
    ) -> Result<(Value, Value), String> {
        if self.ui_click(BUY_BUTTON, 0).await.is_ok() {
            let (a, d) = self.await_inventory_change(before, wait).await?;
            if d["changed"] == json!(true) {
                trail.push(format!("click {BUY_BUTTON}"), NativeLevel::RealInput);
                return Ok((a, d));
            }
        }
        let loc = self
            .lua_json(&locate_chunk(&ClickTarget::Named(BUY_BUTTON.to_string())))
            .await?;
        if let Some((x, y)) = click_point(&loc) {
            if self.click_at_native(x, y, 0, false).await.is_ok() {
                let (a, d) = self.await_inventory_change(before, wait).await?;
                if d["changed"] == json!(true) {
                    trail.push(
                        format!("click {BUY_BUTTON} (CEGUI injectors)"),
                        NativeLevel::NativeCegui,
                    );
                    return Ok((a, d));
                }
            }
        }
        if allow_fallback {
            let r = self
                .lua_json(&handler_chunk("VendorMod.onBuyClicked", BUY_BUTTON))
                .await?;
            if r["ok"] == json!(true) {
                trail.push(
                    "VendorMod.onBuyClicked via Lua",
                    NativeLevel::ClientUiLua,
                );
            }
        }
        self.await_inventory_change(before, wait).await
    }

    /// `client_vendor_buy`: set row `row` (1-based, as shown) to `quantity`
    /// and buy it.
    pub async fn vendor_buy(
        &self,
        row: u32,
        quantity: u32,
        allow_fallback: bool,
        expect_change: bool,
        wait: Duration,
    ) -> Result<Value, String> {
        let t0 = Instant::now();
        if !(1..=MAX_ROWS).contains(&row) {
            return Err(format!("row must be 1..={MAX_ROWS} (the rows on screen), not {row}"));
        }
        if quantity == 0 {
            return Err("quantity must be at least 1".to_string());
        }
        let want = i64::from(quantity);
        let mut trail = NativeTrail::default();
        let st = self.lua_json(&state_chunk(row)).await?;
        if st["vendor_open"] != json!(true) {
            return Err("the vendor window is not open (open it at a vendor first)".to_string());
        }
        if st["buy_visible"] != json!(true) {
            return Err("the Buy tab is not active (click Vendor_BuyTabInactive)".to_string());
        }
        if st["row_visible"] != json!(true) {
            return Err(format!("row {row} shows no item (its quantity spinner is hidden)"));
        }
        if let Some(max) = st["max"].as_i64() {
            if want > max {
                return Err(format!("row {row} allows at most {max}, not {want}"));
            }
        }
        let before = self.inventory_read(&[], false).await?;
        let start = st["current"].as_i64().unwrap_or(0);
        let mut cur = start;
        let mut clicks = 0i64;
        let mut click_error: Option<String> = None;
        while cur != want && clicks < MAX_CLICKS {
            let (button, next) = if cur < want {
                (inc_button(row), cur + 1)
            } else {
                (dec_button(row), cur - 1)
            };
            match self
                .click_until(&button, &value_is_chunk(row, next), None, &mut trail)
                .await
            {
                Ok(()) => {
                    cur = next;
                    clicks += 1;
                }
                Err(e) => {
                    click_error = Some(e);
                    break;
                }
            }
        }
        let mut quantity_via = "spinner_buttons";
        if cur != want {
            if !allow_fallback {
                return Err(format!(
                    "the quantity spinner stopped at {cur}, not {want}: {}",
                    click_error.unwrap_or_else(|| format!("more than {MAX_CLICKS} clicks needed"))
                ));
            }
            let abs0 = st["scroll"].as_i64().unwrap_or(0) + i64::from(row) - 1;
            let r = self
                .lua_json(&set_quantity_chunk(row, abs0, want))
                .await?;
            trail.push(
                format!("set quantity {want} via {}", r["via"].as_str().unwrap_or("Lua")),
                NativeLevel::ClientUiLua,
            );
            cur = r["current"].as_i64().unwrap_or(-1);
            if r["ok"] != json!(true) || cur != want {
                return Err(format!("the quantity could not be set to {want} (reads {cur})"));
            }
            quantity_via = "lua";
        }
        let (after, d) = self.press_buy(&before, allow_fallback, wait, &mut trail).await?;
        let bought = d["changed"] == json!(true);
        let mut out = json!({
            "row": row,
            "quantity": want,
            "spinner_start": start,
            "spinner_clicks": clicks,
            "spinner_final": cur,
            "quantity_via": quantity_via,
            "bought": bought,
            "inventory_changed": d["changed"],
            "diff": d,
            "cash_after": after["cash"],
        });
        trail.stamp(&mut out);
        if expect_change {
            out["effect_ok"] = json!(bought);
            if !bought {
                out["native_pass"] = json!(false);
            }
        }
        out["elapsed_ms"] = json!(t0.elapsed().as_millis() as u64);
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::supervisor::cegui_fake::{self, FakeCegui};
    use crate::supervisor::events::fake_bridge;

    async fn buy(
        state: FakeCegui,
        quantity: u32,
        allow_fallback: bool,
    ) -> (Result<Value, String>, Arc<Mutex<FakeCegui>>) {
        let s = Arc::new(Mutex::new(state));
        let sup = fake_bridge::supervisor(cegui_fake::responder(s.clone())).await;
        let out = sup
            .vendor_buy(1, quantity, allow_fallback, true, Duration::ZERO)
            .await;
        (out, s)
    }

    #[test]
    fn names_follow_the_stock_layout() {
        assert_eq!(spinner_name(3), "Vendor_Item3Quantity");
        assert_eq!(inc_button(3), "Vendor_Item3Quantity__auto_incbtn__");
        assert_eq!(dec_button(3), "Vendor_Item3Quantity__auto_decbtn__");
        let c = set_quantity_chunk(2, 5, 4);
        assert!(c.contains("sp:setCurrentValue(4)"));
        assert!(c.contains("pcall(setVendorSelectedQuantity, 5, 4)"));
    }

    /// The spinner's increment button is clicked until the value is right,
    /// then Buy: every step is an injected click (the posted path is not
    /// available in the fake), so N1 and a native pass.
    #[tokio::test]
    async fn the_spinner_buttons_set_the_quantity_at_n1() {
        let (out, s) = buy(
            FakeCegui {
                inc_works: true,
                ..FakeCegui::default()
            },
            2,
            true,
        )
        .await;
        let out = out.unwrap();
        assert_eq!(out["spinner_clicks"], 2);
        assert_eq!(out["spinner_final"], 2);
        assert_eq!(out["quantity_via"], "spinner_buttons");
        assert_eq!(out["bought"], true);
        assert_eq!(out["effect_ok"], true);
        assert_eq!(out["native_level"], "native_cegui");
        assert_eq!(out["native_pass"], true);
        let s = s.lock().unwrap();
        assert_eq!(s.lua_set_calls, 0);
        assert_eq!(s.edges(), vec!["down 0", "up 0", "down 0", "up 0", "down 0", "up 0"]);
    }

    /// The buttons do nothing: the Lua setter finishes the job and the
    /// result is N3, never a native pass.
    #[tokio::test]
    async fn the_lua_setter_is_the_fallback_and_is_n3() {
        let (out, s) = buy(FakeCegui::default(), 2, true).await;
        let out = out.unwrap();
        assert_eq!(out["quantity_via"], "lua");
        assert_eq!(out["bought"], true);
        assert_eq!(out["native_level"], "client_ui_lua");
        assert_eq!(out["native_pass"], false);
        assert_eq!(s.lock().unwrap().lua_set_calls, 1);
    }

    /// Without the fallback a spinner that will not move is an error that
    /// names where it stopped; nothing is bought.
    #[tokio::test]
    async fn without_fallback_a_stuck_spinner_is_an_error() {
        let (out, s) = buy(FakeCegui::default(), 2, false).await;
        let e = out.unwrap_err();
        assert!(e.contains("stopped at 0, not 2"), "{e}");
        assert!(!s.lock().unwrap().bought);
    }

    #[tokio::test]
    async fn a_zero_quantity_is_refused() {
        let (out, _) = buy(FakeCegui::default(), 0, true).await;
        assert!(out.unwrap_err().contains("at least 1"));
    }
}
```
3. `ui/mod.rs`: add `pub mod vendor_buy;` after `pub mod typed;`.
4. `server/ui/args.rs`: add after `DragDropArgs`:
```rust
/// Args for `client_vendor_buy`.
#[derive(Debug, Default, serde::Deserialize, schemars::JsonSchema)]
pub struct VendorBuyArgs {
    /// The row on screen, 1-based (1..=7), as the vendor window lists them.
    pub row: u32,
    /// How many to buy (at least 1).
    pub quantity: u32,
    /// Set the quantity through Lua (N3) when the spinner buttons cannot,
    /// and press Buy through its stock handler (default true).
    #[serde(default)]
    pub allow_fallback: Option<bool>,
    /// Whether the purchase is expected to change the inventory or cash
    /// (default true); false for a negative check (too little cash).
    #[serde(default)]
    pub expect_change: Option<bool>,
    /// How long to wait for the inventory or cash to change, ms (default 3000).
    #[serde(default)]
    pub wait_ms: Option<u64>,
}
```
5. `server/ui/mod.rs`: add after `client_drag_drop` (inside the same `#[tool_router]` impl):
```rust
    #[tool(
        description = "Buy from the open vendor window like a player. Sets row `row`'s quantity spinner to `quantity` by clicking its increment or decrement button (real or CEGUI-injected clicks, N1), then clicks Vendor_BuyButton. Refuses when the vendor is closed, the Buy tab is not active or the row is empty. When the spinner buttons cannot reach the quantity and allow_fallback is on, sets it through the spinner's Lua / setVendorSelectedQuantity (client_ui_lua, N3). Verified by an inventory and cash diff: bought, effect_ok and native_pass false when nothing changed (expect_change false disables the check)."
    )]
    async fn client_vendor_buy(
        &self,
        Parameters(a): Parameters<VendorBuyArgs>,
    ) -> Result<CallToolResult, McpError> {
        let t0 = Instant::now();
        let r = self
            .supervisor
            .vendor_buy(
                a.row,
                a.quantity,
                a.allow_fallback.unwrap_or(true),
                a.expect_change.unwrap_or(true),
                Duration::from_millis(a.wait_ms.unwrap_or(DEFAULT_WAIT_MS).min(60_000)),
            )
            .await;
        ui_result("client_vendor_buy", t0, r)
    }
```
6. `lease/policy.rs`: in the lease-gated list after `"client_drag_drop",` add `"client_vendor_buy",`. `server/flows.rs` test list after `"client_drag_drop",` add `"client_vendor_buy",`. `uat/tools.rs` after the `drag_drop` line add `drive("vendor_buy", "client_vendor_buy", Tier::N1),`.

### Test: why it fails when reverted
`the_spinner_buttons_set_the_quantity_at_n1` needs the `click_until` ladder with the value readback; the old workflow had no tool and no way to address the auto child. `widget_rect_chunk_falls_back_to_the_cegui_window_name` is the item-4 name guard (the old chunk had no `getWindow` fallback). `without_fallback_a_stuck_spinner_is_an_error` and the N3 test pin the honest tiers.

### Lane checks
`fmt`, `clippy --all-targets -D warnings`, `cargo nextest run -p cimmeria-lab vendor widget_rect flows lease policy tools` (same `lane.ps1` prefix).

### Docs
`docs/guides/live-research-lab.md`:
- Add a tool-table row after the `client_drag_drop` row (about line 918): `| \`client_vendor_buy\` | Buy from the open vendor window: row \`row\` (1-based as shown) to \`quantity\` by clicking the spinner's \`__auto_incbtn__\` / \`__auto_decbtn__\` (read back after each click), then \`Vendor_BuyButton\`; verified by an inventory and cash diff (\`bought\`, \`effect_ok\`). | The spinner buttons cannot reach the quantity: \`setCurrentValue\` / \`setVendorSelectedQuantity\` through Lua (\`client_ui_lua\`, N3), only with \`allow_fallback\`. |` (same column layout as its neighbours).
- Where the guide lists named-window clicking, add: `Auto-created children (\`Vendor_Item1Quantity__auto_incbtn__\`, \`TutorialWin__auto_closebutton__\`) are found by their CEGUI window name by \`client_window_click\`, \`client_ui_click\` and \`client_ui_sequence\` alike (\`ui_click\` falls back to \`getWindow\` after the Lua global).`
- First-live-check list: add `9. \`client_vendor_buy {row: 1, quantity: 3}\` at a vendor: \`spinner_clicks\` 3, \`quantity_via: "spinner_buttons"\`, \`bought: true\`, \`native_tier: "N1"\`.`
`docs/guides/automated-uat.md`: in the native-levels paragraph from LH-02 add: `Split, item use and vendor quantity run at N1 when the injected clicks take. Two things keep them N3: a split or drag whose drop target CEGUI does not resolve (the stock slot windows may carry no DragDropTarget flag) lands through notifyDragDropItemDropped, reported native_call (N3); a click path that is lost on a window ends at the stock handler (client_ui_lua, N3). native_steps names the step.`
`.claude/agents/lab-driver.md`:
- Frontmatter tool list (line 10): append `mcp__cimmeria-lab__client_vendor_buy` (the same line is edited by the item-16 packet; apply after it, or add both tools in whichever lands second).
- Line 48 (`- \`client_ui_sequence\` \`click\` only takes named windows (Lua globals); ...`): replace with `- \`client_ui_sequence\` \`click\` and \`client_window_click\` both take CEGUI window names, auto children included (\`TutorialWin__auto_closebutton__\`, \`Vendor_Item1Quantity__auto_incbtn__\`).`
- Add under "Windows:": `- Buy from a vendor with \`client_vendor_buy {row, quantity}\` (not \`setVendorSelectedQuantity\`): it clicks the spinner buttons and Buy at N1 and verifies the purchase.`

### Live smoke
At the Common Materials Components vendor (teleport `.gotolocation Castle_CellBlock -337.0 73.47 -227.5`, 4.8 m): `client_vendor_buy {row:1, quantity:3}` -> `spinner_final:3`, `bought:true`, `native_tier:"N1"`, `cash_after` lower. Server evidence: ClickHouse purchase row for the player (3 units of the row-1 item); a second call with `quantity` above `max` returns the "allows at most" error.

---

## Owner decisions (recommendations)

1. **`effect_ok` default for `client_item_action` and `client_vendor_buy`.** Recommend default true (the runner then fails a no-op), with `expect_change: false` for negative checks. The cost: any existing spec row that deliberately runs an item action expecting no change must add the argument (none in the repo today; the Ambernol negative check in `docs/analysis/cellblock-autoplay/scenario-map.md` is planned, not built).
2. **`doubleclick` on an inventory slot is performed as a right-click** (the stock slot has no double-click handler), with `requested`/`performed` in the result. Recommend yes; the alternative, an error, breaks the round-1 briefs.
3. **Split and plain drags will stay N3 until the drop-target question is closed** (probe P5). Recommend the guide state it (done in LH-04 docs) and not mutating the layout's `DragDropTarget` from the lab, which would make a pass meaningless. If the owner wants N1 drags, the client-side cause must be found first (`findings §7.2` suspect 2); that is a research packet, not a lab change.
4. **Item use may double-fire in a slow server round trip** (injected click first, posted click second, each waiting `wait/2`). Recommend accepting it for now; after probe P3 shows which path works, drop the loser from `item_action` in a follow-up.
