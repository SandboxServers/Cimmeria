# Lab record: work packets (2 of 2)

> Type: work packets. Same audience, rules and lane commands as
> [work-packets.md](work-packets.md), whose [contract](work-packets.md#contract)
> these packets build on. Ledger: [README.md](README.md).

## Contents

- [LR-06 Recorder sampler and read-only probes](#lr-06-recorder-sampler-and-read-only-probes)
- [LR-07 MCP tools](#lr-07-mcp-tools)
- [LR-08 lab record command](#lr-08-lab-record-command)
- [LR-09 Optional row fixture](#lr-09-optional-row-fixture)
- [LR-10 Lab Lua ring for CEGUI window clicks](#lr-10-lab-lua-ring-for-cegui-window-clicks)
- [LR-11 Window clicks become window_click](#lr-11-window-clicks-become-window_click)
- [LR-12 Docs](#lr-12-docs)
- [LR-13 Live acceptance: record FS-P3](#lr-13-live-acceptance-record-fs-p3)
- [LR-14 Rolling recorder mode](#lr-14-rolling-recorder-mode)
- [LR-15 .bug spec trigger and capped draft](#lr-15-bug-spec-trigger-and-capped-draft)
- [LR-16 Live check: one .bug spec](#lr-16-live-check-one-bug-spec)
- [LR-17 Close-out](#lr-17-close-out)

---

## LR-06 Recorder sampler and read-only probes

**Implementer:** rust-gameserver-dev. **Size:** L. **Wave:** 2. **Depends on:** LR-01.
**Branch:** `lab-record/lr06-sampler`. **Worktree:** `lr06`.
**Subject:** `feat(lab): LR-06 sample a human-driven lab client into a recording`

Why: D-LR2, D-LR3, F4, F6, F8.

Files:

1. `crates/lab/src/supervisor/world/camera_read.rs` (and `mod camera_read;`
   in `supervisor/world/mod.rs`): `impl Supervisor { pub(crate) async fn camera_read(&self) -> Result<Value, String> }`
   that builds the same world IO `client_camera` uses and calls
   `camera_readout` (`io.rs:365-373`), and nothing else: no `ensure_focus`,
   no `look`. A comment says why (D-LR2: the person owns the mouse).
2. `crates/lab/src/uat/record/sampler.rs`:

   ```rust
   pub type BoxedJson<'a> = Pin<Box<dyn Future<Output = Result<Value, String>> + Send + 'a>>;
   /// The client reads a recording may make (fakes in tests).
   pub trait ClientProbe: Send + Sync {
       fn camera(&self) -> BoxedJson<'_>;   // Supervisor::camera_read
       fn ui(&self) -> BoxedJson<'_>;       // Supervisor::ui_state(Some(0))
   }
   pub struct SamplerConfig { pub tap_capacity: u32 /* 10000 */, pub max_ticks: u32 /* 30 min */, pub max_bytes: u64 /* 64 MiB */ }
   pub struct Sampler<'a> { /* server: &'a dyn ServerInvoker, client: &'a dyn ClientProbe, cfg, seen: HashSet<u32>, tail: u32, seq: u32, bytes: u64 */ }
   impl<'a> Sampler<'a> {
       /// Resolve the player (server_sessions by character), its player_id
       /// (server_db_query on sgw_player.player_name; the name must match
       /// `^[A-Za-z][A-Za-z ]{0,63}$`), start the tap, take the first
       /// snapshot. Returns the start line and tick 0.
       pub async fn begin(server: &'a dyn ServerInvoker, client: &'a dyn ClientProbe, cfg: SamplerConfig,
           section: &str, instance: &str, character: &str, now_ms: i64, date: &str) -> Result<(Self, RecordStart, Tick), String>;
       /// One tick, in this order: drain the tap; camera; the player's
       /// `server_entity_get`; the UI; `server_entity_get` for each id an
       /// inbound call names that is not yet seen; the known-server-event
       /// probe; a snapshot when this or the previous tick carried a
       /// non-noise message, or within 10 ticks after one.
       pub async fn tick(&mut self, now_ms: i64) -> Tick;
       /// Final snapshot, tap stop. Never fails: errors go in the tick.
       pub async fn finish(&mut self, now_ms: i64, reason: &str) -> (Tick, RecordStop);
       /// True when max_ticks or max_bytes is reached (the caller stops with that reason).
       pub fn full(&self) -> Option<&'static str>;
   }
   ```

   - A failed read never stops the recording: it is one `errors` entry (cut
     to 120 characters, at most 4 per tick) and the field is left out.
   - The UI read: `windows` from the result's visible windows, sorted;
     `dialog` from its open dialog (title, visible button labels).
   - The camera: from the readout, `pitch_deg = pitch_offset_deg`, `zoom`,
     `yaw_gain = gain` (or SV-05's `yaw_gain` when merged), `yaw_deg = pose.yaw_deg`,
     `pos = pose.server`.
   - The tap: if the player's entity id changes (a relog), restart the tap on
     the new id and add the error `relog: tap restarted on <id>`.
   - Known server events: when `server_log_tail` accepts `target` (SV-02),
     poll it with `{target, since_ms}` every second tick and record each
     `KNOWN_SERVER_EVENTS` key whose `event` field and message match; without
     SV-02, skip it and log one `lab.record` warning at begin.
   - Snapshot SQL: the contract's three statements.
   - Every call goes to the server through `ServerInvoker::call`; the sampler
     never calls a client tool by name.
3. `crates/lab/src/supervisor/record_slot.rs` (and the `Supervisor` field):
   `pub struct RecordSlot(Mutex<Option<RecordHandle>>)`;
   `pub struct RecordHandle { pub recording_id: String, pub dir: PathBuf, pub section: String, pub started_ms: i64, pub ticks: Arc<AtomicU32>, pub anchors: Arc<AtomicU32>, pub stop: tokio::sync::watch::Sender<Option<String>>, pub done: tokio::task::JoinHandle<Result<PathBuf, String>> }`.
   One recording per instance. lab-watch's LW-01 also adds a field to
   `Supervisor`; the second to merge rebases.

Tests (unit, fakes; no lab):

- `begin_resolves_the_player_and_starts_a_10000_tap`: a scripted
  `ServerInvoker` fake (the pattern of `runner/packet_tests.rs:65`) answers
  `server_sessions`, `server_db_query`, `server_packet_tap_start`; assert the
  calls and `start.player_id`.
- `a_tick_drains_samples_and_snapshots_after_activity`: tap returns an
  `interact` row; the tick has `tap`, `camera`, `player`, `windows`, one
  `entities` entry and a `snapshot`; the next quiet tick still snapshots
  (tail), the eleventh quiet one does not.
- `the_sampler_never_calls_a_client_tool`: the fake `ClientProbe` counts
  calls and the server fake fails on any name starting `client_`. Fails if a
  later edit routes a read through the tool router.
- `a_failed_read_is_an_error_entry_not_a_stop` and
  `a_relog_restarts_the_tap`.
- `a_bad_character_name_is_refused_before_any_sql`: `Px'; DROP`.

Review: also `server-authority-enforcer` (the recorder must stay read-only).

---

## LR-07 MCP tools

**Implementer:** packet-coder. **Size:** M. **Wave:** 4. **Depends on:** LR-05, LR-06.
**Branch:** `lab-record/lr07-tools`. **Worktree:** `lr07`.
**Subject:** `feat(lab): LR-07 lab_record_start, lab_record_status and lab_record_stop`

Why: D-LR1, D-LR2, D-LR16, F12, F13.

Files:

1. `crates/lab/src/server/record.rs`: `#[tool_router(router = record_router, vis = "pub(super)")]`
   with three tools.
   - `lab_record_start { section: String, character: Option<String>, specs_dir: Option<String>, max_minutes: Option<u32> }`:
     refuse when the instance's `RecordSlot` is busy. Take an own lease
     exactly as `lab_uat_run` does (`server/uat.rs:345-358`), owner
     `lab_record`, purpose `recording a human: <section>`; keep it alive
     (`RunLease::keep_alive`). The character defaults to the instance's lab
     character (`lab_account`, `server/uat.rs:198`). Create
     `<default_root()>\records\<recording id>\`, run `Sampler::begin`, write the
     start line and tick 0 to `record.jsonl`, then spawn the loop: every
     `TICK_MS`, `tick`, append, bump the counters, and stop on the watch
     signal, `full()`, or a revoked lease (`lease_lost`). `finish` writes the
     last tick and the stop line. Returns
     `{ ok, recording_id, instance, section, dir }`.
   - `lab_record_status {}`: `{ recording: bool, instance, section, age_s, ticks, anchors }`.
   - `lab_record_stop { recording_id: String, character_expr: Option<String>, profile: Option<String>, no_draft: bool }`:
     signal, await the task, read the log, run `transform` with
     `base` = the named section's meta when `load_sections(specs_dir)` finds it,
     write `<specs_dir>/drafts/<file_name>` (refuse to overwrite), release
     the lease. Returns `{ ok, draft, rows, steps, todo, dropped, raw }`
     (D-LR16; `raw` is the recording folder).
   `anchors` is counted by the loop with `decode_call` on each new inbound
   row (an anchor kind other than `Region`).
2. `crates/lab/src/server/mod.rs`: `+ Self::record_router()` in `new`
   (`:270-278`).
3. `crates/lab/src/lease/policy.rs`: `lab_record_start` in `OWN_LEASE`
   (`:42`); `lab_record_status` and `lab_record_stop` in the open list (the
   stop is gated by the recording id instead). Extend the policy tests.
4. `crates/lab/src/uat/record/mod.rs`: remove the dead-code allow.

Tests:

- `server/record.rs` unit tests with a fake sampler path (inject
  `ServerInvoker` and `ClientProbe` the way `server/uat.rs:598` builds a test
  supervisor): `start_refuses_a_second_recording_on_the_instance`;
  `stop_with_a_wrong_id_is_refused_and_keeps_recording`;
  `stop_writes_the_draft_and_releases_the_lease` (draft file exists, lease
  book empty); `stop_never_overwrites_a_draft`.
- `lease/policy.rs`: `record_start_takes_its_own_lease_and_stop_is_open`.
- Output budget: `the_stop_result_is_under_400_characters` for a typical
  result.

Docs: worknote with the tool rows for LR-12 (`docs/guides/live-research-lab.md`
tool table).

---

## LR-08 lab record command

**Implementer:** packet-coder. **Size:** S. **Wave:** 5. **Depends on:** LR-07.
**Branch:** `lab-record/lr08-cli`. **Worktree:** `lr08`.
**Subject:** `feat(lab): LR-08 lab record start, status, stop and transform`

Why: D-LR16, F12, F15. The handoff's `lab record start <section> -Instance p2` / `lab record stop`.

Files:

1. `tools/lab/cli/record.ps1` (discovered as `lab record`, F15):

   ```text
   lab record start <section> [-Instance p2] [-Character <name>] [-MaxMinutes 30] [-Json]
   lab record status [-Instance p2] [-Json]
   lab record stop [-Instance p2] [-CharacterExpr 'Px${run_id}'] [-Profile <name>] [-NoDraft] [-Json]
   lab record transform <recording folder> [-Section <id>] [-CharacterExpr ...] [-Profile ...] [-Json]
   ```

   `start` calls `lab_record_start` with `instance`, and saves
   `{recording_id, instance}` to `%LOCALAPPDATA%\cimmeria-lab\record-<instance>.json`
   so `stop` needs no id. `stop` reads that file, calls `lab_record_stop`,
   deletes the file. `transform` calls `lab_record_transform` (below) and
   needs the daemon running but no lease and no client; without the daemon
   it exits 3 with "the transform runs in the lab daemon; start it with
   `lab start`" (a standalone binary is out of scope). Use `uat-lib.ps1`'s MCP helpers
   (`New-McpSession`, `Invoke-McpTool`, `Close-McpSession`; `:112-152`) and
   `Hide-LeaseIds` on every printed string.
   Exit codes: 0 ok; 1 the tool failed; 2 usage; 3 no daemon; 4 the instance
   is busy (leased or already recording).
2. `tools/lab/cli/record-lib.ps1`: pure helpers (argument checks, the state
   file path, `ConvertTo-RecordCompactJson`).
3. `tools/lab/cli/test-record.ps1`: Pester-free script tests in the style of
   `test-uat.ps1`: argument validation, state-file round trip, and the
   compact JSON (no nulls, under 400 characters, failure shape
   `{"ok":false,"exit":N,"error":"..."}`).

For `transform`, LR-07 adds no tool; this packet adds
`lab_record_transform { dir, section, character_expr, profile }` (open, reads
a recording folder under `<default_root()>\records\`, writes the draft) in
`server/record.rs`, with a unit test that a `dir` outside the records root is
refused.

Lane: the Rust part through the lane as above; the PowerShell tests with
`pwsh -NoProfile -File tools/lab/cli/test-record.ps1`.

Docs: worknote for LR-12 (the `lab record` rows in the commands table).

---

## LR-09 Optional row fixture

**Implementer:** packet-coder. **Size:** S. **Wave:** 4. **Depends on:** LR-05, FX-01.
**Branch:** `lab-record/lr09-fixture`. **Worktree:** `lr09`.
**Subject:** `feat(lab): LR-09 emit a row fixture from the recorded starting state`

Why: D-LR11. A recorded row knows the exact state it started from.

Files:

1. `record/fixture.rs`:

   ```rust
   /// The fixture that puts a character back where this row started: the
   /// missions the row's diffs touch, at their start state, and the item
   /// design ids whose count changed, at their start count. No position
   /// (the row's @stand places the player) and no world.
   pub fn row_fixture(profile: &str, before: &DbSnapshot, diffs: &[Diff]) -> FixtureSpec;
   pub fn fixture_toml(f: &FixtureSpec) -> Vec<String>; // the `[row.fixture]` lines
   ```

   Mission start state (lab-fixtures contract): status 1 → `{ id, step }`;
   status 2 → `state = "completed"`; no row → `state = "not_active"`. Items:
   `{ id, count, container }` with `container = "main"` for bag 1 and
   `"mission"` for bag 2; any other bag is left out with a `# TODO item <id>
   started in bag <n>` line (fixtures set carried counts only). Counts above
   10 (FX's cap) are a TODO line too.
2. `record/emit.rs`: after a row's clauses, when `opts.profile` is set,
   write `[row.fixture]` with `character = "<profile>"` and the lines from
   `fixture_toml`; when it is not, write the same lines prefixed `# ` under
   `# fixture (pass -Profile to enable):`.

Reconcile before coding: FX-01's contract maps `mission` to container 0,
but `crates/entity/src/inventory.rs:14` has `INV_MISSION = 2`. Ask the
coordinator which FX-01 shipped; write the container name, never a number.

Tests (unit):

- `fs_p3_fixture_puts_622_on_2113_and_1360_not_active`: on the fixture's row.
  Fails if the fixture is built from the end snapshot instead of the start.
- `without_a_profile_the_fixture_is_commented_and_the_draft_parses`.
- Update `fs_p3_draft.toml` (it gains the commented block) and say so in the
  commit body.

---

## LR-10 Lab Lua ring for CEGUI window clicks

**Implementer:** rust-gameserver-dev (an RE spike first). **Size:** M. **Wave:** 1. **Depends on:** none.
**Branch:** `lab-record/lr10-click-ring`. **Worktree:** `lr10`.
**Subject:** `feat(lab): LR-10 record CEGUI window clicks in a lab Lua ring`

Why: D-LR9 G1. A title-bar X or a tab click sends no server call, so a
recording sees only that a window vanished.

Step 1, the spike (no code yet; result in `worknotes/LR-10.md`): use the
`re-lookup` skill, docs first. Find whether the client's Lua can subscribe
to a CEGUI global event (`CEGUI::GlobalEventSet`, event
`Window/MouseClick` or `PushButton/Clicked`) through the binding the stock
UI Lua uses, and what the handler receives (the window name). Read the stock
UI Lua under the client's UI folder for any `subscribeEvent` on a global set
(the client UI is CEGUI and Lua source; `reference_client_ui_lua_overlay_testing`
in the main-session memory). The spike is static: no lab, no running client.
If it is not reachable, write that in the worknote, mark LR-10 and LR-11
Done (not feasible) and stop: G1 stays a TODO line.

Step 2, if reachable:

1. `crates/lab/src/supervisor/events/lua_rings.rs`: a third ring next to
   the combat-text and chat wrappers (`:3-6`, `:90-105`): kind `ui.click`,
   fields `{ window, button }` (the clicked window's name and its parent
   frame's name), capped like the others. Install and reinstall it on the
   same path as the existing two. Lab-only: this is the lab's injected Lua,
   nothing ships to players and nothing in the client is patched. Its
   events reach the supervisor's event store like the other two rings
   (`supervisor/events/store.rs`), as kind `ui.click`.

Tests (`lua_rings.rs`): the generated chunk contains the subscription and
the ring push, and the install is idempotent (the pattern of the existing
ring tests).

---

## LR-11 Window clicks become window_click

**Implementer:** packet-coder. **Size:** S. **Wave:** 4. **Depends on:** LR-05, LR-06, LR-10.
**Branch:** `lab-record/lr11-clicks`. **Worktree:** `lr11`.
**Subject:** `feat(lab): LR-11 turn recorded window clicks into window_click steps`

Why: D-LR9 G1.

Files: `record/sampler.rs` (each tick, read `ui.click` events from the event
store through a named cursor `record` and fill `Tick.clicks`; add a
`fn clicks(&self) -> BoxedJson<'_>` to `ClientProbe`), `record/segment.rs`
(carry `Tick.clicks` into the row as `(tick, UiClick)`) and `record/emit.rs`
(the click arm). Rule: a click whose
`button` is a frame's close button (`<Frame>__auto_closebutton__`) or any
named button becomes `{ tool = "@window_click", args = { target = "<button>" } }`
at its place in the step list; a click on a window that also produced a
server call in the same tick is dropped (the call already maps to a step).
The TODO for "window closed with no call" is not written when a click in the
row closed it. Merges after LR-09 (both edit `emit.rs`).

Tests (unit):

- `the_sampler_fills_clicks_once_per_event`: a fake probe returns two
  `ui.click` events, then none; the first tick has both, the next has none.
- `a_tutorial_close_click_becomes_a_window_click`: add a tick with
  `clicks = [{window: "TutorialWin", button: "TutorialWin__auto_closebutton__"}]`
  and `TutorialWin` leaving `windows`; the step appears and no TODO does.
  This is FS-P4's close (`first-session.toml:395`).
- `a_click_with_a_server_call_in_the_same_tick_is_dropped`.

---

## LR-12 Docs

**Implementer:** documentation-writer. **Size:** S. **Wave:** 6. **Depends on:** LR-08.
**Branch:** `lab-record/lr12-docs`. **Worktree:** `lr12`.
**Subject:** `docs(lab): LR-12 lab record in the authoring and lab guides`

Files and rows ([doc-update map](../../agents/doc-update-map.md): new
command, new MCP tools, new folder):

- `docs/guides/automated-uat.md`: a "Recording a draft" section: when to
  record instead of writing by hand, what the draft contains, the TODO
  convention, why a draft is UNCONFIRMED, the promote path through
  `lab golden record` (D-LR12), and the known gaps (D-LR9, including that
  keys are never recorded).
- `docs/guides/live-research-lab.md`: `lab record` in the commands table;
  the four tools in the tool table with their lease gate; "don't call
  `lab_timeline` on a recording instance" (D-LR2).
- `tools/README.md`: the `record.ps1` row.
- `docs/guides/uat-specs/drafts/README.md`: link back to the authoring guide.

Collect the deltas from `worknotes/LR-07.md` and `worknotes/LR-08.md`.

---

## LR-13 Live acceptance: record FS-P3

**Implementer:** coordinator, with a person playing. **Size:** M. **Wave:** 6.
**Depends on:** LR-08, SV-05, SV-07, SV-08, GD-08, D-LR12, D-LR15 (the
user's OK). **No branch** unless values change; results go in
`worknotes/LR-13.md` and the README.

Ask the user first, and wait until the lab is free (`lab status`). Use one
instance (`p2`); keep the other idle (#1341 shows under two-client load).

1. Bring a character to FS-P3's start: `lab uat first-session -Instance p2
   -Rows FS-01,FS-02,FS-P1,FS-P2`, and note the run id (the character is
   `Px<run id>`). With FX-08 merged, instead put the fixture character on
   FS-P3's fixture.
2. `lab record start first-session -Instance p2 -Character Px<run id>`.
   The person walks or teleports to Corporal Frost, right-clicks him, reads
   the dialog and closes it. Nothing else.
3. `lab record stop -Instance p2 -CharacterExpr 'Px${run_id}'` (with FX-08:
   `-Profile praxis`). Check the draft by reading it, not by running it:
   one row, Frost's tag, a pose within 0.5 m and 5 deg of FS-P3's calibrated
   one, the dialog-3995 and the two mission clauses, no TODO lines.
4. Run it: without FX-08, D-LR17's scratch folder (a copy of
   `first-session.toml` with FS-P3 replaced by `REC-01`, outside the repo)
   and `lab uat first-session -SpecsDir <scratch> -Instance p2
   -Rows FS-01,FS-02,FS-P1,FS-P2,REC-01`; with FX-08,
   `lab uat rec-first-session-<stamp> -SpecsDir docs/guides/uat-specs/drafts -Rows REC-01`.
5. `lab golden record` on the same rows and specs folder, 5 runs. The
   acceptance is that all 5 pass and agree. A run that fails on a hover miss
   is what `lab calibrate` fixes (lab-spec-vocab); calibrate the pose, note
   the change, and rerun the 5.
6. Record the outcome, the pose delta and any transform bug in the
   worknote. A transform bug gets its own packet; never patch a draft by hand
   to pass.

---

## LR-14 Rolling recorder mode

**Implementer:** packet-coder. **Size:** M. **Wave:** 7. **Depends on:** LR-07, D-LR13 (a).
**Branch:** `lab-record/lr14-rolling`. **Worktree:** `lr14`.
**Subject:** `feat(lab): LR-14 a rolling recorder that keeps the last minutes`

Why: D-LR13, D-LR14. `.bug spec` needs the minutes before it was typed.

Files:

1. `record/rolling.rs`:

   ```rust
   /// Keeps the start line, the newest snapshot older than the window, and
   /// the ticks of the last `minutes` (1 to 30).
   pub struct Rolling { /* start, minutes, ticks: VecDeque<Tick>, base_snapshot: Option<DbSnapshot> */ }
   impl Rolling {
       pub fn new(start: RecordStart, minutes: u32) -> Result<Self, String>;
       pub fn push(&mut self, tick: Tick);
       /// A `RecordLog` of what is kept, with the base snapshot put on its
       /// first tick, so the transform's "snapshot before the anchor" exists.
       pub fn to_log(&self) -> RecordLog;
   }
   ```

   Evicting a tick that holds a snapshot moves that snapshot into
   `base_snapshot`, so diffs stay right for the oldest kept row.
2. `server/record.rs`: `lab_record_start` gains `rolling_min: Option<u32>`.
   In rolling mode the loop writes nothing to disk; it pushes into `Rolling`
   (the start line records `rolling_min`). `lab_record_stop` on a rolling
   recording writes the kept window as `record.jsonl`, then transforms as
   usual. `status` reports `rolling_min`.
3. `tools/lab/cli/record.ps1`: `start -Rolling <minutes>`.

Tests (unit): `rolling_keeps_only_the_window`;
`an_evicted_snapshot_becomes_the_base`; `to_log_reads_back_through_read_log`;
in `test-record.ps1`, `-Rolling 0` and `-Rolling 31` are usage errors.

---

## LR-15 .bug spec trigger and capped draft

**Implementer:** packet-coder. **Size:** M. **Wave:** 7. **Depends on:** LR-14, D-LR13 (a).
**Branch:** `lab-record/lr15-bug-spec`. **Worktree:** `lr15`.
**Subject:** `feat(lab): LR-15 .bug spec writes a capped draft from the rolling recorder`

Why: D-LR13, D-LR14, F10. No server change.

Files:

1. `record/bug_spec.rs`:

   ```rust
   #[derive(Debug, Clone, PartialEq)]
   pub struct BugSpecTrigger { pub bookmark_id: i64, pub tick: u32 }
   /// In one tick's tap rows: an inbound chat whose text is `.bug spec` or
   /// starts `.bug spec ` (case-insensitive), and, in this tick or the next
   /// four, an outbound `onPlayerCommunication` whose `Text` matches
   /// `^Bookmark (\d+) recorded`. Only the first reply after the chat counts.
   pub fn find_bug_spec(pending: &mut Option<u32>, tick: &Tick) -> Option<BugSpecTrigger>;
   ```
2. `server/record.rs`: in rolling mode, run `find_bug_spec` on each tick.
   On a trigger: `transform(rolling.to_log(), opts)` with
   `max_actions = Some(30)`, `max_bytes = Some(32 * 1024)`, writing
   `<specs_dir>/drafts/bug-<bookmark_id>.toml` (never overwrite), and emit one
   tracing event:
   `tracing::info!(target: "lab.record", event = "bug_spec_written", bookmark_id, path = %rel, rows, todo, "lab record: .bug spec draft written")`.
   The recording keeps running. It types nothing into chat and posts
   nowhere (D-LR14).
3. `record/emit.rs`: honour `max_actions` and `max_bytes`: drop whole rows
   from the oldest until both fit, and write
   `# dropped <n> earlier rows to stay under 30 actions / 32 KiB` in the
   header. Movement and noise are already gone (LR-02).

Tests (unit):

- `bug_spec_needs_the_chat_and_the_bookmark_reply`: the chat alone, the
  reply alone, and `.bug uat FS-P3` (the runner's anchor) all give `None`;
  both in order give the id.
- `a_long_window_is_capped_at_30_actions_keeping_the_latest_rows`: a
  synthetic log of 20 interact rows; the draft keeps the newest rows that fit,
  says how many it dropped, and parses.
- `the_capped_draft_is_under_32_kib`.

Docs: worknote for LR-17 (`.bug spec` in `docs/commands.md`'s `.bug` row as
"lab: with a rolling recorder armed, also writes a draft spec").

---

## LR-16 Live check: one .bug spec

**Implementer:** coordinator. **Size:** S. **Wave:** 8. **Depends on:** LR-15, D-LR15 (the user's OK).

Ask first. On `p2`: `lab record start first-session -Instance p2 -Rolling 10`;
the person plays FS-P3 and types `.bug spec frost check`. Pass when: the chat
shows only the server's one bookmark line; `drafts/bug-<id>.toml` exists,
parses and has at most 30 actions; one `lab.record` `bug_spec_written` row
carries the same `bookmark_id` as the `playtest.bookmark` row in SigNoz; no
board or issue post exists. `lab record stop -Instance p2 -NoDraft`. Delete
the draft unless the user wants it kept.

---

## LR-17 Close-out

**Implementer:** documentation-writer. **Size:** S. **Wave:** 9. **Depends on:** all.

- This README: status table, Review outcomes, campaign status line.
- `docs/guides/unified-uat.md`: the LR-13 and LR-16 steps the owner still has
  to run, if any.
- `docs/project-status.md` and `docs/gap-analysis.md` (the lab area file in
  `docs/gap-analysis/`): once, here.
- `docs/commands.md`: the `.bug spec` note from `worknotes/LR-15.md`.
- lab-watch: a note in its ledger that a recording shows as the lease
  purpose `recording a human: <section>` (no `/status` field needed).
- Main-session memory: one reference note (what a recording captures, the
  gaps, where drafts go), committed with this packet.
- Retire every worker worktree (`pwsh -NoProfile -File tools/build-lane/rm-worktree.ps1 --merged`)
  and post the handoff to the campaign's board subcategory.
