# lab-autopilot wave 4: UI, items, input, strict switch

> **Type:** Reference
> **Audience:** Packet-coders and packet-reviewers of wave 4
> **Last updated:** 2026-10-31
> **Companions:** [work packets](work-packets.md) (rules R1-R10, checks), [contract](contract.md), [ledger](README.md), [wave 3](packets-wave3.md)

Wave 4 makes the rest of the leased tools judge themselves:

- the UI tools (`client_window_click`, `client_ui_click`, `client_type_text`, `client_ui_sequence`) read back what a click was meant to do;
- the item tools (`client_item_action`, `client_drag_drop`, `client_vendor_buy`, `client_world_map_click`) turn the inventory diffs they already take into checks;
- the combat tools (`client_use_ability`, `client_die_and_respawn`, `client_minigame_play`) map their own outcome words to verdicts;
- the input primitives say honestly that a posted key proves nothing;
- the flows and `lab_ensure_in_world` record the end screen they waited for.

It adds `client_ui_clickables` (C16) for the `__auto_` children #1660 could not name. AX-44 then turns on the strict switch (C3 g): from there on an unjudged result counts as inconclusive. Everything is in `crates/lab`; nothing touches the DLL.

The #1660 items this wave guards were fixed by the LH campaign (#1663): split drags (LH-01), the main-bag tab (LH-02), `item_action` sending nothing (LH-03), `__auto_` names (LH-04) and `input_mouse` x/y (LH-05). This wave re-fixes none of them. It makes each one show up as `denied` or `inconclusive` if it ever comes back.

Every packet carries R1-R10 of [work-packets.md § Rules](work-packets.md#rules-every-packet-carries). Paths are under `crates/lab/src/`. Line numbers are from `main` `2019922fe`; waves 1-3 move them, so re-grep every anchor.

**Dispatch:**

1. AX-45 first, from `lax-w4` (created by `ops` from main once the wave 3 PR merged).
2. When AX-45 has merged into `lax-w4`, dispatch AX-40, AX-41, AX-42 and AX-43 at once, each at that head.
3. AX-46 waits until AX-40 has merged (it uses `ClickExpect`).
4. AX-44 goes last, after AX-40..AX-43 and AX-46 merged. Its test fails while any leased tool is still unjudged.

**Merge order:** AX-45, AX-40, AX-46, AX-41, AX-42, AX-43, AX-44. One PR from `lab-autopilot/wave4`.

**The pattern every packet in this wave follows:**

1. Build the `Check` list in a **pure** function in a sibling `*_proof.rs` file. It takes the plain values the tool already computed: no `Supervisor`, no bridge, no `await`. The tests cover every branch without a fake bridge.
2. The tool puts `"evidence": Evidence::judge(checks).to_value()` on its JSON result before it returns. A tool whose output is an `Err` stays an error; the gate passes errors unchanged (C3 a).
3. Register the tool in `proof/claims.rs` `JUDGED` in the same packet (R9). The AX-11 test `claim_lists_name_routed_tools` catches a typo.
4. `effect_ok`, `native_pass` and `counts_as_native_pass` stay as the tool sets them today. The gate (C3 e) downgrades them from the verdict, so no tool edits those flags itself.

Contract excerpt every packet here needs (C1, C2):

- `Check::new(name, Source, ok: Option<bool>, required: bool)` with `.with_before(v)`, `.with_after(v)`, `.with_frame(f)`, `.with_why(Why)`.
- `Evidence::judge(checks)`, in order:
  1. The first required `Some(false)` gives `denied`, with its `why` (else `no_effect`).
  2. The first required `None` gives `inconclusive`, with its `why` (else `no_post_read`).
  3. No required check gives `inconclusive` with `no_evidence`.
  4. Otherwise `confirmed`.
- `Evidence::from_verdict(verdict, why, checks)` sets the verdict directly. Use it only where this file says so.
- `Source` is `Client`, `Server`, `Pixel` or `Bridge`.
- Why codes used here: `confirmed`, `not_delivered`, `ignored`, `no_effect`, `no_post_read`, `server_unreachable`, `delivered`, `posted_only`, `timeout` and `already_true`.

### AX-45 client_ui_clickables

- **Host:** windows · **Agent:** packet-coder · **Branch:** `lab-autopilot/AX-45-clickables` · **Worktree:** `lax-45` (at `lax-w4`)
- **Commit:** `feat(lab): list clickable CEGUI windows with their full names (AX-45)`
- **Rules:** R1-R4, R6, R8, R9. Contract: [C16](contract.md#c16-client_ui_clickables), quoted below. The tool lists the `__auto_` children by the full names `getWindow` accepts. LH-04 made those names clickable; nothing yet lets an agent find them.

C16, the whole contract for this packet:

- Tool: `client_ui_clickables { root?: str (default "Root"), include_hidden?: bool (default false), limit?: 1..=500 (default 200), cursor?: u32 (default 0), instance? }`.
- Policy: **OPEN**. Because it is OPEN, `claim()` already gives `NoClaim`. Never also list it in `NO_CLAIM`; the AX-11 test forbids it.
- One Lua walk per call from `getWindow(root)` through `getChildCount`/`getChildAtIdx`, depth-first in child-index order. Never touch `CEGUI.*`: that global is not exposed to the bridge (live L8); the global `getWindow` is.
- Result: `{ "windows": [ { "name", "type"?, "text", "rect"?: [x, y, w, h], "visible", "enabled", "auto", "parent" } ], "truncated": bool, "visited": u32, "next": u32 | null }`.
- `name` is the full CEGUI name. `auto` is `name.contains("__auto_")`.
- `type` comes from `getType()`. The walk checks `type(w.getType) == "function"` first and omits the field when the binding lacks it.
- `rect` comes from `getUnclippedPixelRect`, in client pixels; omit it when absent.
- Hidden windows and their children are skipped unless `include_hidden`.
- Paging (the live client has 21 416 windows under `Root`, L8): `cursor` is a walk position, the count of windows the walk order puts before this page (pruned hidden subtrees are not in the order unless `include_hidden`). The walk steps past the first `cursor` positions reading only `getChildCount`/`getChildAtIdx`, then inspects windows. It stops at `limit` returned windows or `VISIT_CAP` = 2000 inspected windows, whichever comes first, and sets `truncated: true` and `next` = the position it stopped at; a finished walk gives `next: null`. `visited` counts the windows this call inspected. A page is a one-frame snapshot; `next` is a hint, not a stable key.

Files and changes:

1. New `supervisor/ui/clickables.rs`:
   - `pub const VISIT_CAP: u32 = 2000;` and `pub fn clickables_chunk(root: &str, include_hidden: bool, limit: u32, cursor: u32) -> String`, the Lua body. The walk keeps a position counter; windows before `cursor` are stepped past without reading `getText`/`getType`/the rect; it returns `truncated`, `visited` and `next` (nil when the walk finished).
     - Copy the walk-and-encode pattern of `supervisor/ui/window_read.rs`: its `WALK_FN` constant and `__jenc`. Quote names with `crate::supervisor::flows::widgets::lua_quote`.
     - Read the rect the way `supervisor/input.rs` (~l.104) reads `getUnclippedPixelRect`.
     - Wrap each getter in `__jcall` so a missing method on one window does not abort the walk.
     - Clamp `limit` to 1..=500 in Rust before formatting.
   - `impl Supervisor { pub async fn ui_clickables(&self, root: Option<&str>, include_hidden: bool, limit: Option<u32>, cursor: Option<u32>) -> Result<Value, String> }`. It calls `self.lua_json(&clickables_chunk(..))`, then post-processes in Rust: set `auto` from `name`, drop `type` and `rect` when empty, and add `"truncated"`, `"visited"` and `"next"` (JSON `null` when absent).
   - Declare `pub mod clickables;` in `supervisor/ui/mod.rs`.
2. `server/ui/args.rs`: `ClickablesArgs { root: Option<String>, include_hidden: Option<bool>, limit: Option<u32>, cursor: Option<u32> }`, with `#[serde(default)]` and doc lines as `WindowClickArgs` has. `instance` is routed by the handler; check how the other args structs leave it out.
3. `server/ui/mod.rs`: add the `#[tool]` `client_ui_clickables` (at most 25 lines). Return through `ui_result`. Description: `"List the clickable CEGUI windows under root (default Root) with their full names, type, text, pixel rect, visible and enabled. Names containing __auto_ are the stock UI's unnamed children; pass the full name to client_ui_click or a client_ui_sequence click step. Large trees page: pass next back as cursor while truncated is true. Read-only."`
4. `lease/policy.rs`: add `client_ui_clickables` to `OPEN` only.
5. Tests: a `#[cfg(test)] mod tests` in `clickables.rs` for the chunk, and a new `supervisor/ui/clickables_tests.rs` for the call, which answers through `supervisor/events/fake_bridge.rs` (`fake_bridge::supervisor(responder)` replies to `lua_eval` with a canned walk).

Tests:

| Test | Asserts | Fails on revert because |
|---|---|---|
| `chunk_walks_from_the_root_and_checks_get_type` | the chunk contains `getChildCount`, `getChildAtIdx`, `type(w.getType) == "function"` and the quoted root; `limit` 9999 formats as 500 | C16 walk and clamp |
| `hidden_windows_are_skipped_unless_asked` | the chunk with `include_hidden: false` contains an `isVisible` guard before descending; with `true`, it does not | C16 |
| `auto_flag_and_empty_fields` (clickables_tests) | a canned walk with `VendorWin/__auto_closebutton__` (no type, no rect) and `VendorWin/BuyButton` gives `auto` true then false; the first has no `type` and no `rect` keys | C16 result shape |
| `truncated_is_reported` | a canned walk answering `truncated = true, visited = 2000, next = 2000` gives `"truncated": true`, `"visited": 2000`, `"next": 2000`; one with no `next` gives `"next": null` | C16 paging |
| `chunk_carries_cursor_and_visit_cap` | `clickables_chunk("Root", false, 50, 400)` contains `400` as the start position and `2000` as the visit cap; `getText` is read only after the position check | C16 paging: never inspect skipped windows |
| `clickables_is_open_and_no_claim` | `policy::OPEN` contains it, `claims::NO_CLAIM` does not, `claims::claim("client_ui_clickables") == Claim::NoClaim` | C16 policy |

Checks: L1, L2. Report the line counts of `clickables.rs` and `server/ui/mod.rs`.

### AX-40 Clicks that say what they expect

- **Host:** windows · **Agent:** packet-coder · **Branch:** `lab-autopilot/AX-40-click-expect` · **Worktree:** `lax-40` (at `lax-w4` after AX-45)
- **Commit:** `feat(lab): window and ui clicks read back the window they expect (AX-40)`
- **Rules:** R1-R4, R6, R8, R9. Contract: C1, C2 (`already_true`, `posted_only`, `no_effect`). Decision D-AX15 (architect, [ledger](README.md#decisions)): a click on a window already in the expected state is `inconclusive` with `already_true`, never `confirmed`.

A click today reports that the cursor was over the target (`received_by`), not what the click did. This packet adds optional `expect_visible` / `expect_hidden` arguments. With them, the tool reads the window before the click, waits for the change after it, and judges. Without them, the result is `inconclusive` with `posted_only`, which is the truth.

Files and changes:

1. New `supervisor/ui/click_expect.rs`:

```rust
#[derive(Debug, Clone, Default)]
pub struct ClickExpect {
    pub visible: Option<String>,   // this window must be visible after the click
    pub hidden: Option<String>,    // this window must not be visible after the click
    pub timeout: Duration,         // default 2 s, clamped to 200 ms..=10 s
}
impl ClickExpect {
    pub fn from_args(visible: Option<String>, hidden: Option<String>, timeout_ms: Option<u64>) -> Self;
    pub fn is_empty(&self) -> bool;
}
/// What the expected windows looked like before the click.
pub struct ExpectBefore { pub visible: Option<bool>, pub hidden: Option<bool> } // each: was the window visible?
impl Supervisor {
    /// Read each expected window's visibility before the click (`click_checked::visible_chunk`).
    pub async fn expect_before(&self, e: &ClickExpect) -> ExpectBefore;
    /// After the click: `wait_window(name, gone, e.timeout)` for each expected window.
    pub async fn expect_after(&self, e: &ClickExpect, before: &ExpectBefore) -> Vec<Check>;
}
/// Pure: the evidence of one click.
pub fn click_evidence(received_by: Option<&str>, target: &str, row_selected: Option<bool>, expect: Vec<Check>) -> Evidence;
```

   - `expect_after`, for `visible: Some(w)`:
     - `before.visible == Some(true)` gives `Check::new("window_visible", Client, None, true)` with why `already_true`, `before: true`. Do not wait.
     - Otherwise `wait_window(w, false, timeout)` (`supervisor/composite/mod.rs` ~l.97): `Ok(ms)` gives `ok: Some(true)` with `after: {"visible": true, "ms": ms}`. `Err(e)` gives `ok: Some(false)`, why `no_effect`, `after: {"visible": false, "error": e}`.
     - A failed before-read (`None`) is not `already_true`: wait as above and record `before: null`.
   - `hidden: Some(w)` is the mirror: the check is `"window_hidden"`, `already_true` when the window was already not visible, and the wait is `wait_window(w, true, timeout)`.
   - `click_evidence` builds these checks:
     - `cursor_on_target`, optional, Client: `ok: Some(received_by.is_some())`, `before: target`, `after: received_by`.
     - `row_selected` when `Some(sel)`: required, Client, `ok: Some(sel)`, why `no_effect` when false.
     - Then the expect checks.
     - With no required check at all, return `Evidence::from_verdict(Verdict::Inconclusive, Why::PostedOnly, checks)`. Otherwise return `Evidence::judge(checks)`.
   - Declare `pub mod click_expect;` in `supervisor/ui/mod.rs`.
2. `supervisor/ui/window_click.rs` (526 lines; at most 20 added): `window_click` gains a last parameter `expect: &ClickExpect`. It reads `expect_before` just before the click and `expect_after` after it. It adds `"evidence": click_evidence(received_by, &target_name, selected, checks).to_value()` to the result. `selected` is `Some` only for a row click. A row click that errs because the row was not selected stays an error, as today. Update the one caller (`server/ui/mod.rs` ~l.127) and the test call (~l.421).
3. `supervisor/input.rs` `ui_click` (~l.439; the file is 596 lines, so at most 12 added): same change, with `row_selected: None`. Callers that pass no expectation pass `&ClickExpect::default()`. Grep `\.ui_click(` across the crate and update each call site. Only the two tools below build a non-empty one.
4. `server/ui/args.rs` `WindowClickArgs` and the `UiClickArgs` (moved to `server/input_tools.rs` by AX-15) each gain:

```rust
/// Window that must be visible after the click; the result is judged on it.
#[serde(default)] pub expect_visible: Option<String>,
/// Window that must be hidden after the click.
#[serde(default)] pub expect_hidden: Option<String>,
/// How long to wait for the expected window, ms (default 2000, max 10000).
#[serde(default)] pub expect_timeout_ms: Option<u64>,
```

5. The two `#[tool]` descriptions gain this sentence: `"Pass expect_visible or expect_hidden to have the click judged: without one the result is inconclusive (posted_only), because a click alone proves nothing."`
6. `proof/claims.rs`: add `client_window_click` and `client_ui_click` to `JUDGED`.
7. New `supervisor/ui/click_expect_tests.rs` (wired from `click_expect.rs`). The pure `click_evidence` cases need no bridge. The `expect_after` cases use `fake_bridge::supervisor`, answering the visibility `lua_eval` from a scripted list.

Tests:

| Test | Asserts | Fails on revert because |
|---|---|---|
| `no_expectation_is_posted_only` | `click_evidence(Some("Btn"), "Btn", None, vec![])`: verdict `inconclusive`, why `posted_only`, one optional `cursor_on_target` check | the default stops claiming |
| `expected_window_appearing_confirms` | `expect_after` with `visible: "VendorWin"`, before `false`, the fake answering not visible then visible: one check `ok: true`; `click_evidence` gives `confirmed` | step 1 |
| `expected_window_never_appearing_denies` | the fake never answers visible, timeout 300 ms: check `ok: false`, why `no_effect`; verdict `denied` | step 1 |
| `already_visible_is_already_true` | before `true`: check `ok: None`, why `already_true`, no wait (the fake saw exactly one `lua_eval`); verdict `inconclusive` | D-AX15 |
| `hidden_mirror` | `hidden: "DialogWin"`, before visible, then gone: `confirmed`; before already hidden: `already_true` | step 1 |
| `unselected_row_denies` | `click_evidence(.., Some(false), vec![])`: `denied`, why `no_effect` | row clicks stay judged |
| `timeout_is_clamped` | `from_args(.., Some(60000))` gives 10 s; `Some(10)` gives 200 ms; `None` gives 2 s | step 1 |

Checks: L1, L2. Report the line counts of `window_click.rs`, `input.rs` and `click_expect.rs`, and every `ui_click` call site you updated.

### AX-46 Typing and sequences that read back

- **Host:** windows · **Agent:** packet-coder · **Branch:** `lab-autopilot/AX-46-sequence-proof` · **Worktree:** `lax-46` (at `lax-w4` after AX-40)
- **Commit:** `feat(lab): type_text and ui_sequence judge what they did (AX-46)`
- **Rules:** R1-R4, R6, R8, R9. Contract: C1, C2. Uses AX-40's `ClickExpect`.

Files and changes:

1. `client_type_text` (`server/input_tools.rs`, `TypeTextArgs`) gains `into: Option<String>`, documented as `"Edit box to click first and read back; the result is judged only with it."`
   - With `into`, the tool calls `ui_click(into, ..)` first with an empty `ClickExpect`, then `type_text`, then reads the box: `getText()` through `lua_json`, the same way `supervisor/ui/window_read.rs` reads text.
   - Add the pure `pub fn typed_evidence(into: Option<&str>, typed: &str, read_back: Option<&str>) -> Evidence` to `click_expect.rs`:
     - `into` present: a required Client check `text_entered`, `ok = read_back.map(|t| t.contains(typed))`, `after: read_back`. A failed read gives `None`, why `no_post_read`.
     - `into` absent: `from_verdict(Inconclusive, PostedOnly, vec![])`.
   - Put the supervisor half in a new `supervisor/ui/type_into.rs`, `impl Supervisor { pub async fn type_into(&self, into: Option<&str>, text: &str) -> Result<Value, String> }`. It returns `{typed_chars, into?, read_back?, evidence}`. Declare it in `supervisor/ui/mod.rs`.
2. `supervisor/composite/sequence.rs` (`ui_action` ~l.161-217, `ui_sequence` ~l.220-253):
   - `ui_action` also returns `Vec<Check>`, built by a new pure `pub(super) fn action_checks(index: usize, action: &Action, result: &Result<Value, String>, moved: Option<bool>) -> Vec<Check>` in a new `supervisor/composite/sequence_proof.rs`. Each check carries `"action": index` inside `before`. Rules, per action:
     - Any action that returned `Err(e)`: a required Client check `action_<i>`, `ok: Some(false)`, why `no_effect`, `after: e`.
     - `WaitWindow` `Ok`: a required check `window_<i>`, `ok: Some(true)`, `after: {"window", "gone", "ms"}`.
     - `Drag` with `moved == Some(true)` (from `drag_moved`, ~l.142): a required check `drag_<i>`, `ok: Some(true)`. A drag that did not move already errs.
     - `Type` with `into`: the `text_entered` check from step 1, renamed `typed_<i>`. `Type` without `into`, `Click`, `Key` and `Wait` add nothing.
   - `ui_sequence` collects every action's checks. With no required check at all, it uses `from_verdict(Inconclusive, PostedOnly, checks)`; otherwise `judge`. It puts `evidence` on its result. The early-stop path (`stopped_at`) keeps the failing action's check, so the verdict is `denied`.
   - Stay inside `sequence.rs`'s cap: the new logic lives in `sequence_proof.rs`. Declare it in `supervisor/composite/mod.rs`.
3. The `client_ui_sequence` description gains: `"End a click with a wait_window step (or type with into) to make the result confirmed; a sequence of bare clicks and keys is inconclusive (posted_only)."`
4. `proof/claims.rs`: add `client_type_text` and `client_ui_sequence` to `JUDGED`.
5. Tests: `supervisor/composite/sequence_proof_tests.rs` (pure), and `supervisor/ui/type_into_tests.rs` (`fake_bridge`).

Tests:

| Test | Asserts | Fails on revert because |
|---|---|---|
| `bare_clicks_are_posted_only` | `[Click, Key]` both `Ok`: no required check; the sequence evidence is `inconclusive` / `posted_only` | step 2 |
| `click_then_wait_window_confirms` | `[Click, WaitWindow ok]`: verdict `confirmed`, check `window_1` with `before.action == 1` | step 2 |
| `a_failed_step_denies` | `[Click, WaitWindow Err("VendorWin not visible after 2000 ms")]`: `denied`, why `no_effect`, the error in `after` | step 2 |
| `type_into_reads_back` (type_into_tests) | the fake answers `getText` with `"hello"` for typed `"hello"`: `confirmed`; with `""`: `denied` | step 1 |
| `type_without_into_is_posted_only` | `typed_evidence(None, "x", None)`: `inconclusive` / `posted_only` | step 1 |
| `read_back_failure_is_no_post_read` | `typed_evidence(Some("ChatEdit"), "x", None)`: `inconclusive`, why `no_post_read` | honesty |

Checks: L1, L2. Report the line counts of `sequence.rs`, `sequence_proof.rs` and `type_into.rs`.

### AX-41 Item, drag, vendor and map evidence

- **Host:** windows · **Agent:** packet-coder · **Branch:** `lab-autopilot/AX-41-item-proof` · **Worktree:** `lax-41` (at `lax-w4` after AX-45)
- **Commit:** `feat(lab): item, drag, vendor and world-map tools judge their effect (AX-41)`
- **Rules:** R1-R4, R6, R8, R9. Contract: C1, C2. These four tools already measure their effect; this packet turns the measurements into checks. `ui/item_action.rs` is 629 lines: **at most 8 lines added there**. Its logic goes in the sibling file.

Files and changes:

1. New `supervisor/ui/item_proof.rs`, pure functions over the values each tool already computes. Read each tool's tail to find them (anchors below). Each function returns `Vec<Check>`:
   - `item_checks(expect_change: bool, inventory_changed: bool, diff: &Value, loot: Option<LootFacts>, double_use_risk: bool)`. `LootFacts { count_before: u32, count_after: u32, window_open_after: bool }`. Rules:
     - `loot: Some(l)`: a required Client check `loot_taken`, `ok = Some(l.count_after < l.count_before || !l.window_open_after || inventory_changed)`.
     - Else, when `expect_change`: a required `inventory_changed`, `ok = Some(inventory_changed)`, `after: diff`.
     - Else (a negative check): a required `inventory_unchanged`, `ok = Some(!inventory_changed)`, `after: diff`.
     - `double_use_risk`: an optional check `double_use_risk`, `ok: Some(false)`, why `no_effect`, with the note in `after`. Optional, so it never decides the verdict.
     - Anchor: `item_action.rs` ~l.167-405. The result keys are `inventory_changed`, `diff`, `double_use_risk`, `loot_count_before`/`loot_count_after` and `loot_window_open`. `effect_ok` is set at ~l.396-402.
   - `drag_checks(drag_started: bool, moved: bool, moved_check: &str, snap_back: bool)`. Rules:
     - `drag_started`: required, Client. When false, why `ignored`.
     - `moved`: required, why `no_effect` when false, `after: {"moved_check": moved_check, "snap_back": snap_back}`.
     - Anchor: `drag_drop.rs` output ~l.470-505.
   - `vendor_checks(expect_change: bool, bought: bool, items_gained: u32, want: u32)`. Rules:
     - `expect_change`: a required `items_arrived` (`ok = Some(bought && items_gained > 0)`), and a required `quantity_matched` (`ok = Some(items_gained == want)`, `before: want`, `after: items_gained`).
     - Else: a required `nothing_bought`, `ok = Some(items_gained == 0)`.
     - Anchor: `vendor_buy.rs` ~l.245-267.
   - `map_checks(map_closed: bool)`. Rules:
     - A required Client check `map_closed`.
     - An optional Server check `destination_sent`, `ok: None`, why `server_unreachable`, `after: "method 91 on the packet tap; a UAT clause proves it"`. The tool cannot drain the tap (the runner owns it), so it says so instead of guessing.
     - Anchor: `world_map.rs` ~l.65-131.
2. Each of the four tools adds `out["evidence"] = Evidence::judge(<fn>(..)).to_value();` just before it returns its result: 2-4 lines per file. Declare `pub mod item_proof;` in `supervisor/ui/mod.rs`.
3. `proof/claims.rs`: add `client_item_action`, `client_drag_drop`, `client_vendor_buy` and `client_world_map_click` to `JUDGED`.
4. New `supervisor/ui/item_proof_tests.rs`.

Tests:

| Test | Asserts | Fails on revert because |
|---|---|---|
| `item_use_that_changed_nothing_denies` | `expect_change` true, unchanged: `denied`, why `no_effect` (the #1660 "native_pass but sends nothing" shape, fixed by LH-03) | step 1 |
| `negative_item_check_confirms_when_unchanged` | `expect_change` false, unchanged: `confirmed`; changed: `denied` | `expect_change: false` is a negative check (`ItemActionArgs` doc) |
| `loot_counts_down_confirms` | loot 3 → 2, window open: `confirmed`; 3 → 3, open, unchanged: `denied` | step 1 |
| `double_use_risk_never_decides` | changed + `double_use_risk`: still `confirmed`, and the optional check is present | optional checks are notes |
| `drag_that_never_started_is_ignored` | `drag_started` false: `denied`, why `ignored` (the #1660 split-drag shape, fixed by LH-01) | step 1 |
| `drag_snap_back_denies` | started, not moved, `snap_back` true: `denied`, why `no_effect`, `after.snap_back == true` | step 1 |
| `vendor_short_quantity_denies` | want 5, gained 3: `denied` on `quantity_matched` | step 1 |
| `vendor_negative_check` | `expect_change` false, gained 0: `confirmed` | negative purchase rows |
| `map_closed_confirms_with_a_pending_server_note` | closed: `confirmed`, and the `destination_sent` check is present with `ok` absent and why `server_unreachable` | the optional server half never denies |

Checks: L1, L2. Report the line counts of the four tool files and `item_proof.rs`.

### AX-42 Combat evidence: ability, defeat, minigame

- **Host:** windows · **Agent:** packet-coder · **Branch:** `lab-autopilot/AX-42-combat-proof` · **Worktree:** `lax-42` (at `lax-w4` after AX-45)
- **Commit:** `feat(lab): ability, defeat and minigame tools judge their outcome (AX-42)`
- **Rules:** R1-R4, R6, R8, R9. Contract: C1, C2. Decisions D-AX11 (minigame server corroboration) and D-AX14 (`use_ability` judges whether the press got an answer, not whether the ability succeeded).

Why D-AX14 matters: `docs/guides/uat-specs/debug-area.toml` row DA-U22 presses Pistol Shot at a neutral NPC and expects a **refusal**. If a refusal were `denied`, the runner would fail that row's action (AX-12) before its clause could pass. So `use_ability` confirms that the press was answered, and records the answer; the row's clauses decide whether that answer was the right one.

Files and changes:

1. New `supervisor/combat/use_ability/proof.rs`: `pub fn ability_evidence(verdict: &str, sent: bool) -> Evidence`. It builds one required Client check `press_answered`, `after: {"verdict": verdict, "sent": sent}`, and maps the `outcome.rs` `verdict()` words (~l.260-277):

   | `verdict` | Check `ok` | why |
   |---|---|---|
   | `effect_applied`, `refused`, `refused_client_side`, `refused_with_feedback` | `Some(true)` | `confirmed` |
   | `cast_started`, `sent_no_reply` | `None` | `no_post_read` |
   | `nothing_observed` | `Some(false)` | `ignored` |
   | anything else | `None` | `no_evidence` |

   Hook it where `use_ability/mod.rs` (~l.353) puts the outcome at `out["result"]`: add `out["evidence"] = ability_evidence(out["result"]["verdict"].as_str().unwrap_or(""), ..).to_value();`. The nested `result.verdict` string stays as it is. The gate's top-level `verdict` mirror (C3 f) is a different key.
2. New `supervisor/combat/defeat_proof.rs`: `pub fn defeat_checks(window_closed: bool, alive: Option<bool>) -> Vec<Check>`. Rules:
   - `defeat_window_closed`: required, Client.
   - `alive`: required, Client. `None` gives `ok: None`, why `no_post_read`.
   - Hook: `combat/defeat.rs` ~l.329-348, which builds `checks{defeat_window_closed, alive, ..}`. Add `evidence` next to `verified`; leave `verified` as it is.
   - `alive` stays what `defeat.rs` reads today. Do **not** read the `GameBeing + 0x158` dead bit: the live pass saw it only on an NPC (L6), and AX-02's optional step decides whether the player sets it. If it does, a later packet may add it as an **optional** Client check, never a required one.
3. New `supervisor/minigame/proof.rs` (put it next to the file that holds `action_effect`, ~l.170, and the result tail, ~l.508-522; grep `fn action_effect`). `pub fn minigame_checks(game: &str, placeholder: bool, server_outcome: Option<&str>, client_window_hidden: bool) -> Vec<Check>`. Rules:
   - The placeholder games are Hack, Activate, Analyze, Bypass and Converse. Use the list the file already has; grep `placeholder`.
   - Not placeholder: a required Server check `server_outcome`, `ok = server_outcome.map(|o| o == "victory")`. `None` gives why `server_unreachable`; `Some(other)` gives why `no_effect`, `after: other`.
   - Placeholder: a required Client check `client_window_hidden`, plus the same `server_outcome` check as **optional**.
4. Wire each with `out["evidence"] = Evidence::judge(..).to_value();` at the point the tool builds its final JSON. Declare the new modules.
5. `proof/claims.rs`: add `client_use_ability`, `client_die_and_respawn` and `client_minigame_play` to `JUDGED`.
6. Tests: `use_ability/proof_tests.rs`, `combat/defeat_proof_tests.rs` and `minigame/proof_tests.rs`.

Tests:

| Test | Asserts | Fails on revert because |
|---|---|---|
| `a_refusal_is_an_answer` | `refused` and `refused_client_side` give `confirmed`, with `after.verdict` naming the refusal | D-AX14 (DA-U22) |
| `cast_without_result_is_inconclusive` | `cast_started` gives `inconclusive` / `no_post_read` | step 1 |
| `nothing_observed_is_ignored` | `denied`, why `ignored` | step 1 |
| `respawn_needs_both_reads` | window closed, alive `None`: `inconclusive`; closed + alive: `confirmed`; still open: `denied` | step 2 |
| `real_minigame_needs_the_server` | not placeholder, server `None`: `inconclusive` / `server_unreachable`; `Some("defeat")`: `denied`; `Some("victory")`: `confirmed` | D-AX11 |
| `placeholder_minigame_is_client_judged` | placeholder, window hidden, server `None`: `confirmed`, with the optional server check present | D-AX11 |

Checks: L1, L2. Report the files you hooked, with line numbers.

### AX-43 Input primitives say what they know

- **Host:** windows · **Agent:** packet-coder · **Branch:** `lab-autopilot/AX-43-input-proof` · **Worktree:** `lax-43` (at `lax-w4` after AX-45)
- **Commit:** `feat(lab): input primitives report delivered, posted or confirmed (AX-43)`
- **Rules:** R1-R4, R6, R8, R9. Contract: C1, C2 (`posted_only`, `delivered`, `not_delivered`), and C5's `input_status` keys. No x/y change: LH-05 (#1663) already fixed `client_input_mouse` x/y.

A posted key cannot be read back; the honest verdict is `inconclusive` with `posted_only`. A cursor move, a focus change and a release **can** be read back, so those are `confirmed` or `denied`.

Files and changes:

1. New `supervisor/input_proof.rs`, pure:
   - `pub fn key_evidence() -> Evidence`: `from_verdict(Inconclusive, PostedOnly, vec![])`.
   - `pub fn mouse_evidence(m: MouseFacts) -> Evidence`, with `MouseFacts { dx: i32, dy: i32, wheel: i32, button: bool, cursor_placed: Option<bool>, di_before: Option<u64>, di_after: Option<u64> }`. Rules:
     - `button`: `from_verdict(Inconclusive, PostedOnly, ..)`.
     - Motion or wheel with `di_before` and `di_after` both `Some`: a required Bridge check `di_read`, `ok = Some(after > before)`, why `not_delivered` when false. `di_*` is the DirectInput read counter in `input_status` `hooks`; grep `input_status` and `hooks` in `supervisor/input.rs` and `bridge/input/`, and use whichever counter the DLL increments per `GetDeviceState`/`GetDeviceData` read. A confirmed `di_read` is reported with why `delivered` through `from_verdict(Inconclusive, Delivered, checks)`. The game read the input; whether it did anything is not judged.
     - Motion with no counter (an old DLL): `from_verdict(Inconclusive, PostedOnly, ..)`.
     - A cursor-only move (`cursor_placed: Some(p)`, from `place_cursor`'s `CursorPlaced`, which reads back within ±1 px): a required Client check `cursor_at`, `ok = Some(p)`.
   - `pub fn cursor_evidence(placed: bool, at: (i32, i32)) -> Evidence`: a required `cursor_at` with `after: at`.
   - `pub fn focus_evidence(want_on: bool, hwnd_after: Option<u64>) -> Evidence`: a required Bridge check `virtual_focus`, `ok = Some(hwnd_after.is_some() == want_on)`.
   - `pub fn release_evidence(held_after: &Value, holds_after: Option<&Value>) -> Evidence`: a required Bridge check `nothing_held`, `ok = Some(held empty && holds empty or absent)`.
2. Wire each tool in `server/input_tools.rs` (or wherever the supervisor half builds the JSON, `supervisor/input.rs`; that file is near its cap, so at most 15 lines added there in total):
   - `client_input_key`: `key_evidence`.
   - `client_input_mouse`: read `input_status` before and after **only when** `dx`/`dy`/`wheel` is non-zero (the AX-34 pattern). Then `mouse_evidence`.
   - `client_cursor_move`: `cursor_evidence`.
   - `client_input_focus`: after the call, read `input_status` `virtual_focus_hwnd`; then `focus_evidence`.
   - `client_input_release`: after the call, read `input_status` `held` and C5 `holds`; then `release_evidence`.
   - `client_input_status` is a read; it stays as it is. Check whether it is in `OPEN` or `NO_CLAIM`. If it is in neither, add it to `NO_CLAIM` and say so in the report.
3. Descriptions: `client_input_key` gains `"The game's use of a posted key cannot be read back: the result is inconclusive (posted_only). Read the state you expected to change."`. The other four name what they read back.
4. `proof/claims.rs`: add `client_input_key`, `client_input_mouse`, `client_cursor_move`, `client_input_focus` and `client_input_release` to `JUDGED`.
5. New `supervisor/input_proof_tests.rs`.

Tests:

| Test | Asserts | Fails on revert because |
|---|---|---|
| `a_key_is_posted_only` | `inconclusive` / `posted_only` | honesty |
| `di_motion_read_is_delivered` | before 10, after 12: `inconclusive` / `delivered` with a `di_read` check `ok: true` | step 1 |
| `di_motion_unread_is_not_delivered` | before 10, after 10: `denied`, why `not_delivered` | step 1 |
| `cursor_move_confirms_on_read_back` | placed: `confirmed`; not placed: `denied` | step 1 |
| `focus_mismatch_denies` | want on, hwnd `None`: `denied` | step 1 |
| `release_with_a_hold_left_denies` | `holds_after` `[{"vk":87}]`: `denied`; empty: `confirmed` | step 1 |

Checks: L1, L2. Report the line counts of `input.rs`, `input_tools.rs` and `input_proof.rs`.

### AX-44 Flow evidence and the strict switch

- **Host:** windows · **Agent:** rust-gameserver-dev · **Branch:** `lab-autopilot/AX-44-strict` · **Worktree:** `lax-44` (at `lax-w4` after AX-40..AX-43 and AX-46)
- **Commit:** `feat(lab): flows judge their end screen, and unjudged results stop counting (AX-44)`
- **Rules:** R1-R4, R6, R8, R9. Contract: C3 (g), quoted below. Decision D-AX13: unjudged is legacy only until this packet.

C3 (g), the strict switch:

- `pub const STRICT: bool` lives in `proof/gate.rs`, `false` until this packet sets it `true`.
- With it on, rule (e) treats `Unjudged` like `Inconclusive`: it removes `effect_ok` and records `claimed.effect_ok`.
- The runner's `verdict::of_outcome` maps `unjudged` to `Inconclusive("{tool}: unjudged")`.
- Rule (d) still inserts `Evidence::unjudged()`, so a tool someone adds later without evidence stays visible.

Files and changes:

1. `server/flows.rs` `flow_result` (~l.211): it becomes `flow_result(tool: &str, r: Result<Value, FlowError>)`, and every caller passes its tool name. On `Ok(v)` it adds `v["evidence"] = flow_evidence(tool, &v).to_value()`. `Err` is unchanged.
2. New `server/flow_proof.rs`: `pub(super) fn flow_evidence(tool: &str, v: &Value) -> Evidence`. Each flow returns `Ok` only after it has read its end screen (`supervisor/flows/`).
   - Build one required Client check `end_state`, `ok: Some(true)`, `after: {"screen": <the screen the flow waited for>}`.
   - Read each flow's last wait to name the screen. Fill this table in the report:

   | Tool | Screen (fill in from the flow's last wait) |
   |---|---|
   | `lab_login` | |
   | `lab_create_character` | |
   | `lab_delete_character` | |
   | `lab_ensure_character_slot` | |
   | `lab_play_character` | |
   | `lab_finish_dialog` | |
   | `lab_logout` | |
   | `lab_disconnect` | |

   - A flow that returns `Ok` **without** a final read gives `ok: None`, why `no_post_read`. List it in the report; do not add a read.
   - `lab_characters` is OPEN and stays out.
3. `server/composite.rs` `lab_ensure_in_world` (~l.90): add `evidence`. It holds one required Client check `in_world`, `ok = Some(v["in_world"] == true)`, `after: {"world_id", "pos"}`. Put the pure builder in `flow_proof.rs`.
4. `proof/gate.rs`:
   - Set `pub const STRICT: bool = true;`.
   - Rule (e)'s `Unjudged` branch, when `STRICT`, does what the `Inconclusive` branch does.
   - Remove the `#![cfg_attr(not(test), allow(dead_code))]` line and its comment from `proof/mod.rs`. Fix any dead-code warning that appears by deleting the dead item or wiring it; never re-add the allow.
5. `uat/runner/verdict.rs` `of_outcome`: when `STRICT` and the evidence is `unjudged`, return `Inconclusive(format!("{tool}: unjudged"))`.
6. `proof/claims.rs`: add the eight flow tools and `lab_ensure_in_world` to `JUDGED`.
7. Update the tests that encode the old rule. These are the only expectation changes allowed:
   - `proof/gate_tests.rs` `unjudged_gets_unjudged_and_keeps_effect_ok`: rename it to `unjudged_drops_effect_ok_when_strict`, and assert no `effect_ok` and `claimed.effect_ok == true`.
   - `uat/runner/verdict_tests.rs` `an_unjudged_step_changes_nothing`: rename it to `an_unjudged_step_is_inconclusive_when_strict`, and assert row UNVERIFIED with the flag `inconclusive: client_world_click: unjudged`.
8. New test in `proof/claims_tests.rs`: `every_leased_tool_is_judged_or_no_claim`. For every routed tool in `policy::LEASED` (and `OWN_LEASE`, if any routed tool sits there that is not a lease tool), assert `claim(tool) != Claim::Unjudged`. When you write it, list the tools that fail:
   - Names starting `lab_record_`, `lab_golden_`, `lab_uat_` or `lab_lease_`, and `lab_client_*` lifecycle tools, are bookkeeping. Add them to `NO_CLAIM` and list them in the report.
   - Any other unjudged leased tool: stop and report `blocked - contract gap: unjudged leased tools <list>`. The architect decides each one.
9. New `server/flow_proof_tests.rs`.

Tests:

| Test | Asserts | Fails on revert because |
|---|---|---|
| `flow_ok_confirms_its_end_screen` | `flow_evidence("lab_login", &json!({..}))`: `confirmed`, `after.screen` as in your table | step 2 |
| `ensure_in_world_false_denies` | `{"in_world": false}`: `denied` | step 3 |
| `unjudged_drops_effect_ok_when_strict` | as step 7 | C3 (g) |
| `an_unjudged_step_is_inconclusive_when_strict` | as step 7 | C3 (g), runner |
| `every_leased_tool_is_judged_or_no_claim` | step 8 | the strict switch would silently make an unlisted tool's rows UNVERIFIED |

Checks: L1, L2. Report the table from step 2, the `NO_CLAIM` additions from step 8, and the `JUDGED` list's final length.
