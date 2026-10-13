# Lab UAT reliability work packets

These are prescriptive Luna briefs. Each packet starts from current `origin/main`
in a new worktree named `lr-<number>-<slug>`, edits only its listed code and
documentation, commits locally, and reports the commit and lane results.
Workers share a repository: do not revert another worker's changes. Do not
drive the shared lab, restart its daemon, push, or merge. The coordinator
reviews, ships and performs live UAT. Read the linked issue and `TESTING.md`
before editing. Use PowerShell and the build lane for every compiling cargo
command. No packet changes protocol indices or the game client binary.

## Shared contract

- `native_pass` is true only when the claimed client effect occurred through
  the reported native input path. A mouse dispatch alone is not proof.
- A timed-out or failed bridge read returns a bounded error with the stage and
  last error; it does not fabricate a successful hover, UI read or click.
- Keep existing MCP tool names and argument schemas unless a packet explicitly
  extends one. Add only optional fields to responses.
- Use existing `crates/lab/src/supervisor` modules and tests. Do not add a
  script or sibling CLI for these fixes.
- Every packet updates the relevant tool contract in
  `docs/guides/live-research-lab.md` or `docs/guides/automated-uat.md`, and
  records a concise finding in `.claude/agent-memory/main-session/`.

## LR-01 — typed prompt reader (#1673)

**Own:** `crates/lab/src/supervisor/ui/typed.rs` and its tests; the typed
reader section of `docs/guides/live-research-lab.md`. Branch
`lab-uat-reliability/lr-01-prompt`; worktree `lr-01-prompt`; commit
`fix(lab): read modal prompts as typed windows`.

Add `kind: "prompt"` to `READERS`. Resolve visible `Prompt<N>_PromptWin` roots
by the same numbered-window pattern already used by `ui_state_chunk`; expose
instance, title, message, visible button labels and enabled state in `extra`.
Do not treat a hidden `Prompt<N>_Button1` as the real single-button OK: the
observed single button is `Prompt<N>_SingleButton`. Keep the normal typed
reader shape (`primary`, `roots`, `extra`) and return a closed/empty result
when no prompt is visible. Add a unit test that a mock numbered prompt can be
found and its text/button data survives shaping, plus a no-prompt test. The
guard must fail if the new reader is removed. Test type: UI logic unit.

## LR-02 — covered native click (#1676)

**Own:** `crates/lab/src/supervisor/ui/window_click.rs`, a new focused test
module only if needed, and the window-click section of
`docs/guides/live-research-lab.md`. Branch
`lab-uat-reliability/lr-02-covered-click`; worktree `lr-02-covered-click`;
commit `fix(lab): refuse covered window clicks`.

**Coordinator preflight:** prove that the stock Lua binding
`Root:getTargetChildAtPosition(CEGUI.MouseCursor:getSingleton():getPosition())`
is callable on the installed client, using the probe in
`docs/analysis/lab-harness/packets-ui.md`. The underlying CEGUI hit-test
walks visible children top first (`docs/reverse-engineering/findings/cegui-mouse-input-feed.md`
section 7). If the Lua binding is absent, stop and write a native-call design
before dispatching Luna; do not substitute rectangle overlap.

Before `click_at`, determine the CEGUI window under the click point from the
real hit-test/root z-order. If it is neither the target nor a descendant of
the target, return a covered-target error naming the covering window. For a
row, accept the owning list and its descendants as the hit surface. Never
stamp `native_pass` for a covered click. Avoid a blanket rectangle overlap
test: a transparent frame can overlap without consuming input. Add a pure
classification test for target, descendant and covering sibling, and a
client-Lua logic test when the CEGUI hit-test needs one. Test type: UI logic.

## LR-03 — bridge failures inside waits (#1677)

**Own:** `crates/lab/src/supervisor/flows/ui_state.rs` and its tests, plus
wait documentation. Branch `lab-uat-reliability/lr-03-bridge-wait`; worktree
`lr-03-bridge-wait`; commit `fix(lab): keep polling through transient bridge failures`.

Change `Supervisor::poll_until` so a transient `lua_results` transport error
during a world load records `last_error`, sleeps until the next poll and
retries while within the original deadline. Preserve a positive `met`, a
condition error, and the explicit `fail` outcome. At deadline, return
`met:false` with the last bridge error, elapsed time and poll count; do not
return `met:true` or spin without sleeping. Use an injectable/fake poll
sequence to test error → error → success and error → deadline; include a
condition `fail` case. Test type: unit plus bridge failure simulation.

## LR-04a — feedback channel metadata (#1679)

**Own:** `crates/lab/src/supervisor/ui/chat_log.rs` and its tests; chat reader
docs. Branch `lab-uat-reliability/lr-04a-chat-meta`; worktree
`lr-04a-chat-meta`; commit `fix(lab): expose feedback channel display metadata`.

The live feedback channel id 9 currently returns null channel/colour/tabs.
Inspect the stock `ChatMod.ChannelMap` and the observed `UIChannel` values;
read metadata for id 9 even when it is outside the current `UIChannel`
iteration. Resolve the display channel, colour and tab through the stock Lua
tables rather than hardcoding a hue. Keep `channel_id:9` and preserve the
caller’s numeric filter. Add a regression test with id 9 metadata and an
unknown-id null case. Test type: client-Lua logic plus Rust unit.

## LR-04b — capture welcome across relog (#1679)

**Dependency:** LR-04a merged. **Own:**
`crates/lab/src/supervisor/events/lua_rings.rs`,
`crates/lab/src/supervisor/ui/chat_log.rs` and focused tests; chat reader
docs. Branch `lab-uat-reliability/lr-04b-relog-chat`; worktree
`lr-04b-relog-chat`; commit `fix(lab): preserve chat capture across relog`.

Trace capture installation and re-subscription when the chat window is
recreated during logout/play. Install before `onClientReady` or report a
precise capture gap; do not invent missing lines from the HUD tail. The
relog welcome should appear once in the `client_chat_log` cursor with its
real channel metadata. Add a lifecycle test for capture installed → window
destroyed → window recreated → first feedback line, and prove no duplicate
hook or duplicate line. Test type: UI lifecycle simulation.

## LR-05a — dead-player observation (#1680)

**Own:** `crates/lab/src/supervisor/composite/ensure.rs` and its tests; ensure
flow docs. Branch `lab-uat-reliability/lr-05a-dead-observe`; worktree
`lr-05a-dead-observe`; commit `fix(lab): recognize a dead player in world`.

`observe_chunk` currently treats an empty `unitName(Unit.Player)` as an
unknown player even when World/HUD is healthy. Read a second stable identity
source (already selected character or authenticated player state) and carry
an explicit `player_dead`/identity uncertainty observation so `plan` does
not settle for 15 s and then declare the world absent. It must not accept
the wrong character. Test the dead-player observation, a genuinely loading
world, and a wrong-character result. Test type: flow state-machine unit.

## LR-05b — safe logout after death (#1680)

**Dependency:** LR-05a merged. **Own:**
`crates/lab/src/supervisor/flows/login.rs` and flow tests; logout docs.
Branch `lab-uat-reliability/lr-05b-dead-logout`; worktree
`lr-05b-dead-logout`; commit `fix(lab): leave defeat state safely before logout`.

Detect Defeat/respawn state before typing `/logout`. Follow the client's
normal respawn or logout UI path; never send chat keystrokes through an
active Defeat window. If recovery cannot be proven, fail with a bounded
diagnostic and leave the client running. Test healthy world, defeat window
and character-select no-op. Test type: flow state-machine unit and live UAT.

## LR-06 — bounded target hover (#1684)

**Own:** `crates/lab/src/supervisor/world/click_hover.rs`,
`crates/lab/src/supervisor/world/hover.rs`, and related hover tests; targeting
docs. Branch `lab-uat-reliability/lr-06-target-timeout`; worktree
`lr-06-target-timeout`; commit `fix(lab): bound mouse-over reads in combat`.

The `read_mouse_over` bridge operation blocked for 57 s in #1684. Bound
each hover read and the total target attempt; distinguish a transport
timeout from `Hover::Nothing` and include attempted pixel and remaining
deadline in the error. Preserve the client's current target if verification
is unavailable. Do not mark an unverified target as N1. Simulate an
unresponsive `WorldIo::slots()` future and assert a timely error; preserve
the existing verified-hover success case. Test type: async timeout unit.

## LR-07 — native ring-map input (#1686)

**Own:** `crates/lab/src/supervisor/ui/world_map.rs`, relevant input
adapter, and their tests; map-click docs. Branch
`lab-uat-reliability/lr-07-ring-native`; worktree `lr-07-ring-native`;
commit `fix(lab): deliver native ring destination clicks`.

First reproduce on a clean client with a lab lease: compare the posted
mouse, `client_ui_click`, and a physical click at one destination while a
packet tap watches `setRingTransporterDestination`. Record which CEGUI
window receives each event. Then make the smallest lab input correction
that causes the stock `WorldMapMod.onClick` subscription to run through
input; retain #1687's explicit N3 fallback. A native pass requires method
91 and map closure. Add a regression guard for the selected injection path
and an N3 fallback guard. If the physical-path probe contradicts the issue,
update the evidence and stop before a speculative patch. Test type: input
simulation plus live packet tap. This packet needs coordinator-led live
reconnaissance before Luna edits code.

## Existing Cellblock-autoplay packets (#1685 and #1689)

The detailed design in [cellblock-autoplay](../cellblock-autoplay/work-packets.md)
is authoritative. Recheck it against current code before dispatch.

- **#1685:** CA-MG-L0 (live input spike), CA-MG-L1 (board snapshot and
  traffic tap), CA-MG-L4 (hit-map generator), CA-MG-L2 (server state and
  solver), CA-MG-L3 (N1 player), CA-MG-L5 (UAT rows). Dispatch the pure
  snapshot and solver steps to Luna after the coordinator validates the
  design; one live lab driver runs the spike and final proof.
- **#1689:** CA-AP-11 owns `server_path`, a route recorder and Cellblock
  waypoints. Give Luna the bounded tool/route substeps only after the
  coordinator confirms current point sets and path source. A straight-line
  `client_move_to` success is not route proof; drive the route in the real
  client and verify zone/mission events.

## Check and handoff for every Luna packet

From its own worktree, in order:

```powershell
cargo fmt -p cimmeria-lab --check
pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-lab --all-targets -- -D warnings
pwsh tools/build-lane/lane.ps1 cargo test -p cimmeria-lab
git diff --check
```

The coordinator runs the focused test against the pre-fix code or applies
an equivalent mutation to confirm it detects the defect, then requests a
read-only adversarial review, ships through `ship.py`, and performs live
validation under a lease. A Luna worker reports the exact tests and any
unproven live behavior; it does not call a row PASS from compilation alone.
