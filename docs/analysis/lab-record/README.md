# Lab record: a human plays, out comes a spec

> Type: ledger. Audience: the coordinator, packet workers and reviewers.
> Opened 2026-10-10 against `main` @ `6996c9403`. Prefix `LR-`. Source brief:
> [lab-roadmap handoff § 4](../lab-roadmap/handoff.md#4-lab-record-a-human-plays-out-comes-a-spec).
> Packet specs: [work-packets.md](work-packets.md) (contract, LR-01 to LR-05)
> and [work-packets-2.md](work-packets-2.md) (LR-06 to LR-17).
>
> **Campaign status (2026-10-10): planned, nothing built.** Three decisions
> are BlockedDecision (D-LR12, D-LR13, D-LR15). LR-01 and LR-10 can start
> now; the transform packets wait on SV-03 and GD-02.

## Purpose

A person plays one lab client by hand. `lab record start <section> -Instance p2`
arms a recorder on that instance; `lab record stop` turns what it saw into
`docs/guides/uat-specs/drafts/<section>-<yyyyMMdd-HHmm>.toml`: one row per
meaningful interaction, written in the lab-spec-vocab vocabulary (poses,
`@stand`, absolute camera, server-state clauses with `timeout_ms`).

The transform is pure Rust over a recorded log. It infers nothing: a fixed
table maps each recorded fact to a step or a clause, and anything outside the
table becomes a `# TODO` comment. A draft is unconfirmed until
`lab golden record` agrees on it five times (lab-golden, owner rule).

A later, opt-in wave adds `.bug spec`: on a lab instance with a rolling
recorder armed, typing `.bug spec <note>` writes a capped draft of the last
minutes beside the bug bookmark.

Out of scope: specs synthesised from seed content chains (deferred by the
handoff until several long-form specs exist), any client patch, any server
write path, and recording player sessions outside the lab.

## What was found

Against `main` @ `6996c9403`. Paths are repo-relative.

| # | Finding | Packets |
|---|---|---|
| F1 | **The lab DLL records no human UI input.** No click, window, dialog-choice, chat-typed or item-moved event exists. The only `client.ui.*` / `client.input.*` events are `client.ui.cegui_log` (`crates/client-telemetry/src/hooks/cegui_log.rs:25`) and `client.input.console_command`, which carries no fields, not even the command text (`hooks/inline_hooks/console_command.rs:38-50`). The DirectInput hooks only merge synthetic input and keep counters (`src/bridge/input/dinput.rs:52-63`, `:381-464`); there is no WndProc hook. | D-LR3, D-LR9, LR-10 |
| F2 | **No event carries the entity under the cursor.** `mouse_over` exists only in the lab's own click results (`crates/lab/src/supervisor/world/click.rs:7`, `:98`, `:573`). The server tap does carry it for right-click interactions: `interact`'s `INT32 overrideTarget` is the clicked entity. In the Praxis capture, record 19 is `interact` with args `00000000 0d 8f890100`: entity prefix 0, sub-slot `0x0d`, target 100751 (Frost) (`crates/wireclient/tests/fixtures/praxis_start_tap.json`; layout `docs/protocol/cell-method-dispatch-table.md:290`, `:416`). | D-LR4, LR-02 |
| F3 | **The tap has every client call a recording needs**, in order, with server timestamps: `dir`, `msg_id`, `method_index`, `msg_name`, `target_entity_id`, `args_hex` (first 256 bytes) and `decoded` (`crates/wire-log/src/wire_log/tap.rs:73-92`). Inbound calls are not decoded (`decoded` is null for `interact`, `dialogButtonChoice`, `gmGotoXYZ`), so the recorder decodes `args_hex` itself. A buttonless dialog close sends `dialogButtonChoice(3995, -1)` (record 33: `0e 9b0f0000 ffffffff`). | LR-02 |
| F4 | **The tap is a draining ring with one reader.** Start takes `entity_id` and `capacity` (1 to 10000, default 500) (`crates/lab-mcp/src/tools/mod.rs:105-117`); a read drains it and resets `dropped` (`wire_log/tap.rs:190-203`); `lab_timeline` reads the same ring (`crates/lab/src/timeline/packet_tap.rs:1-7`); a reconnect stops inbound capture until the tap restarts (`wire_log/tap.rs:27-30`). | D-LR2, LR-06 |
| F5 | **Server effects arrive in the same millisecond.** Record 19 `interact` at `…906213`, record 20 `onDialogDisplay {dialog_id: 3995, entity_id: 100751}` at `…906214`, then `onStepUpdate` 2113 → 1 and 80623 → 0 and `onMissionUpdate 1360` at `…906220-22`. Persistence to `sgw_mission` is later and unobserved, which is why polled clauses exist (lab-spec-vocab F9). | D-LR7, LR-04 |
| F6 | **The camera has a read path that moves nothing**: `camera_readout` (`crates/lab/src/supervisor/world/io.rs:365-373`) returns `zoom`, `yaw_offset_deg`, `pitch_offset_deg`, `gain`, `pitch_gain` (`memory.rs:316-324`) and the camera actor `pose` (client and server position, `yaw_deg`; `geometry.rs:103-108`). `client_camera` itself turns virtual focus on (`camera.rs:222`, `io.rs:441-443`) and needs the lease, so the recorder must call the readout, never the tool. | D-LR2, LR-06 |
| F7 | **The face controller's geometry is reusable.** `yaw_error(from, yaw, to)` and `bearing` work in client X/Y with the actor-yaw convention (`supervisor/world/geometry.rs:82-91`); `server_to_client` converts (`:52-55`). A recorded yaw offset is the negative of the yaw error from the camera to the target. | D-LR6, LR-03 |
| F8 | **The open UI read is enough for windows and dialogs.** `client_ui_state` (no lease, `crates/lab/src/lease/policy.rs:66`) returns the visible top-level windows, the open dialog (title, text, visible buttons), prompts, the mission tracker and a chat tail, each section under its own `pcall` (`crates/lab/src/server/client_state.rs:76-84`; `supervisor/flows/ui_state.rs:312`). | LR-06 |
| F9 | **Server reads need no lease and are bounded.** `server_db_query` is a single statement, wrapped as a sub-select, run `READ ONLY` and rolled back, capped at 500 rows (`crates/services/src/database.rs:85`, `:102-160`). `server_entity_get` returns the entity's `tag` (`crates/wire/src/cell/messages/lab.rs:168`). The resources tables (`items.name`, `db/resources/Items/Tables/items.sql:6-11`) are in the same database (`db/database.sql:10-13`). | D-LR5, LR-04, LR-06 |
| F10 | **The `.bug` command is a GM `.`-console command** (`crates/cell-console/src/cell/console/bookmark.rs:562-613`, routed at `dispatch.rs:445`, spec `registry/commands/meta.rs:14-20`). It writes only tracing rows: one `playtest.bookmark` (`bookmark.rs:495-555`), up to 32 `playtest.bookmark.entity` rows, and `abilities.snapshot` rows, all sharing `bookmark_id`, the epoch ms (`:363-365`). It replies one line, `Bookmark {id} recorded: …` (`:596-609`). Like every accepted console command, it is posted to the Discord GM-command channel (`dispatch.rs:165-169`). There is no bug table in `db/`. The runner already types `.bug uat <row>` as its anchor (`crates/lab/src/uat/runner/session.rs:163-198`). | D-LR13, D-LR14, LR-15 |
| F11 | **A player session outside the lab has none of the inputs:** no camera readout, no window list, no tap unless one was started, and `client.net.out` gives method names without arguments (`crates/client-telemetry/src/hooks/inline_hooks/net_out.rs:78-90`). A draft from a player's `.bug` would have to guess. | D-LR13 |
| F12 | **Instances are addressed by an `instance` argument.** The router resolves it by label or lab account, then by the lease's instance, then the first instance, and strips it before the tool runs (`crates/lab/src/server/instances.rs:21`, `:149-180`). Only `lab uat` and `lab logs` take `-Instance` today (`tools/lab/cli/uat.ps1:48`). | LR-07, LR-08 |
| F13 | **`lab_uat_run` is the model for a daemon tool that holds a lease for a long job:** an own lease (`RunLease::acquire_own`), a keep-alive that renews it, and stop on revocation (`crates/lab/src/server/uat.rs:345-358`, `:465-471`). It is the only `OWN_LEASE` tool (`lease/policy.rs:42`). Routers are summed in `LabServer::new` (`server/mod.rs:270-278`). | LR-07 |
| F14 | **Specs load non-recursively** (`crates/lab/src/uat/mod.rs:66-73`), so a `drafts/` folder is never run by accident, and `committed_specs_parse_and_validate` (`:125-138`) does not see it. | D-LR10, LR-05 |
| F15 | **`lab` subcommands are discovered** from `tools/lab/cli/*.ps1` (`tools/lab/lab.ps1:32-65`), and `lab.yml` runs every `tools/lab/**/test-*.ps1`. A `record.ps1` and `test-record.ps1` need no wiring. The MCP client helpers are in `uat-lib.ps1:101-152`. | LR-08 |
| F16 | **The Praxis capture already holds a real FS-P3**: records 4 to 34 run from the teleport to Frost to the dialog's close (lab-golden uses the same slice for `fs_p3_tap.json`). Frost stands at (-328.30, 73.472, -210.27) and the stand-off is (-325.0, 73.6, -212.8): radius 4.16 m, bearing 322.5°, dy 0.13 (lab-spec-vocab F20). | LR-01, LR-03, LR-05 |

## Decisions

| ID | Status | Decision | Reason |
|---|---|---|---|
| D-LR1 | PROPOSED (coordinator) | **Record raw, transform later.** The daemon writes a raw log (`record.jsonl`, one start line, one line per 500 ms tick, one stop line) under `<uat root>\records\<recording id>\`. `lab record stop` runs the pure transform on that file; `lab record transform <dir>` reruns it. The raw log is local evidence and is never committed. | The transform can be fixture-tested from logs, fixed and rerun on an old recording without the lab, the same split lab-golden chose (D-GD1). |
| D-LR2 | PROPOSED (coordinator) | **The recorder holds the instance's lease and only reads.** `lab_record_start` takes an own lease (owner `lab_record`, purpose `recording a human: <section>`) and keeps it alive until stop, so no agent drives the client mid-recording. Each tick it makes only reads: the tap drain, `server_entity_get`, `server_db_query`, `client_ui_state`'s supervisor call, and `camera_readout`. It never calls `client_camera` or any input tool, and never touches focus. It owns the player's tap; `lab_timeline` must not be called on that instance while recording. | F4, F6. A second tap reader would drop rows silently; virtual focus would fight the person's mouse. |
| D-LR3 | PROPOSED (coordinator) | **Inputs, per tick (500 ms):** the drained tap rows; the player's server entity (position, space); the camera readout; the visible windows and the open dialog's buttons; target entities the first time a call names them (`server_entity_get`: tag, template, position); and a DB snapshot when the tick or the one before it carried a non-noise server message, plus one at start and stop. Client UI events are not an input until LR-10 (F1). | Tick-bucketing puts every input on the host clock in one order, so no client/server clock alignment is needed. Snapshots only after server activity keep the query rate low on the colo. |
| D-LR4 | PROPOSED (coordinator) | **Rows are cut at anchors.** An anchor is an inbound `interact`, `useItem`, `moveItem` or `useAbility`, or a `triggerClientHintedGenericRegion` with `entering = 1` that is followed by a state diff before the next anchor. Everything between one anchor and the next belongs to the first: dialog choices, window changes, server messages, snapshot diffs. Movement and teleports never anchor: the pose and `@stand` replace them. Dot-command chat lines (`sendPlayerCommunication` text starting `.`, except `.bug`) go into the setup of the row they precede. | Each anchor is one thing a person did that the server saw; the region rule keeps walk-in steps (a region that advances a mission) and drops plain walking. |
| D-LR5 | PROPOSED (coordinator) | **One fixed mapping table** turns facts into steps and clauses (contract § Mapping). Each row gets `state = "any"`, a pose per interacted tag, `@stand`, `@camera { pose }`, the action step, and the clauses from its diffs: `sgw_mission` (status and step), `sgw_player` (level, exp, naquadah, world), carried item counts by design id, and `onDialogDisplay` packet clauses. | The handoff: one row per interaction, clauses from the server-state diffs, no inference. |
| D-LR6 | PROPOSED (coordinator) | **Pose from geometry.** `radius_m`, `bearing_deg` and `dy_m` invert D-SV3 from the player's and target's server positions in the tick before the anchor. `pitch_deg` and `zoom` are the readout's own numbers (D-SV4). `yaw_offset_deg = -yaw_error(camera, camera_yaw, target)` in degrees, times -1 when the readout's yaw gain is negative. `aim` is always `entity`. `calibrated = "recorded <date>"`. A pose outside the SV-03 ranges (radius 1 to 8 m) makes the row a comment block. | F7, F16. The face controller stops inside its margin, so a recorded yaw offset can differ from what `@camera` reproduces by up to that margin, and a point-aimed body (lab-spec-vocab F13) is recorded as an entity aim; `lab calibrate` and the golden runs are what confirm a pose. |
| D-LR7 | PROPOSED (coordinator) | **Timeouts from observed timing plus a margin**, rounded up to 250 ms: a polled clause `clamp(max(2t, t + 3000), 3000, 60000)` where `t` is the anchor to the first snapshot showing the diff; a dialog wait `clamp(max(2t, t + 2000), 2000, 30000)`; a `wait_server` `clamp(max(2t, t + 5000), 5000, 120000)`. No `wait_ms` is ever emitted. | The handoff's "waits from the observed timing plus a margin". Doubling covers a slower run; the floor covers a fast recording. |
| D-LR8 | PROPOSED (coordinator) | **Nothing guessed.** A fact outside the table is a `# TODO <what was seen>` line in the row where it happened. A row that cannot be written validly (a pose out of range, a target with no tag) is written whole as TOML comments under a `# TODO` header. The output is a pure function of the log and the options: two runs give identical bytes. | The handoff's "unknown event types become commented `# TODO` lines". A draft that parses is always runnable; a commented row is a visible gap. |
| D-LR9 | PROPOSED (coordinator) | **Telemetry gaps, and the one packet that closes one.** G1 UI-only clicks (a title-bar X, a tab) send no server call: LR-10 adds a lab Lua ring for CEGUI window clicks (the lab's own injected Lua, as `lua_rings.rs` already wraps chat and combat text; no client patch), and LR-11 maps it to `@window_click`. G2 keys (bag, Esc, hotkeys other than abilities) are **never** recorded: a key logger on a client that types a password is not acceptable; windows that open with no click and no call stay TODO. G3 console (`~`) commands carry no text (F1) and stay TODO. Entity under the cursor (F2), camera (F6) and chat typed (tap) need nothing new. | F1 to F3. Only G1 is common in UAT rows (FS-P4 closes the tutorial with its X). |
| D-LR10 | PROPOSED (coordinator) | **Drafts.** Path `docs/guides/uat-specs/drafts/<section>-<yyyyMMdd-HHmm>.toml`; section id `rec-<section>-<yyyyMMdd-HHmm>`; `[section]` copied from the named section when it exists (else from the instance's lab account and character); row ids `REC-01`…; the recorded character's name in SQL replaced by `${character}`, or by `-CharacterExpr` (for example `Px${run_id}`). Drafts are committed only when a person chooses to. A committed test parses and validates every file in `drafts/`, seed tags included (SV-04). | F14. A draft is never run by a plain `lab uat`, and a committed one cannot rot. |
| D-LR11 | PROPOSED (coordinator) | **Optional `[row.fixture]`.** With `-Profile <name>` and FX-01 merged, each row gets a fixture built from the snapshot at its start: the missions its diffs touch at their start state, the item design ids whose count changed at their start count. Without `-Profile` the same block is written as comments. Position is never written into the fixture (`@stand` places the player). | lab-fixtures makes a row independent; a recording knows exactly the starting state the fixture needs. |
| D-LR12 | **BlockedDecision** (owner). Recommended: **(a)** | **How a recorded draft becomes a spec.** (a) Only through `lab golden record` agreeing 5 times on the draft (owner rule), after which a person moves the rows into the real spec and records its golden; the draft header says `UNCONFIRMED` until then. (b) As (a), and a draft that passes one `lab uat` run may be committed as a spec at once, marked unconfirmed. | (a) is the handoff's follow-up line. (b) is faster but puts single-run evidence in a committed spec, which the owner rule forbids for goldens. |
| D-LR13 | **BlockedDecision** (owner). Recommended: **(a)** | **`.bug spec` feasibility.** (a) Lab instances only: a rolling recorder (`lab record start -Rolling 10`) keeps the last N minutes; when the tap shows the person's `.bug spec …` chat line and the server's `Bookmark <id> recorded` reply, the daemon writes `drafts/bug-<id>.toml` through the same transform and logs one `lab.record` event with `bookmark_id` and the path. No server change: the bookmark row already carries the note, and the path is named by its id. (b) Also for player sessions: not deterministic (F11), so the campaign would stop at `lab record`. | F10, F11. (a) needs no inference and no new server write. |
| D-LR14 | PROPOSED (coordinator; applies if D-LR13 is (a)) | **`.bug spec` gating and size.** Opt-in twice: the rolling recorder must be armed, and the person types `spec` as the first word. Output: no chat line beyond the server's existing bookmark reply; no board or issue post (the Discord GM-command line every `.bug` already gets is unchanged). Caps: the last 10 minutes (at most 30), movement and noise dropped, at most 30 actions across all rows (the latest rows that fit), at most 32 KiB; the header names what was dropped. | The handoff: opt-in, one line, no spam, about 30 steps. |
| D-LR15 | **BlockedDecision** (user) | **The live packets drive the lab.** LR-13 needs a person to play FS-P3 on `p2` and then runs the draft 5 times; LR-16 tries `.bug spec` once. Both run only with the user's OK, when the lab is free. | Repo rule: ask before any lab use. |
| D-LR16 | PROPOSED (coordinator) | **Output budget.** `lab record start/status/stop -Json` print one object under 400 characters with no nulls or empty values: `ok`, `recording`, `instance`, `section`, `age_s`, `ticks`, `anchors`; stop adds `draft`, `rows`, `steps`, `todo`, `dropped`. Failures: `{"ok":false,"exit":N,"error":"..."}`. | The `lab uat -Json` contract. |
| D-LR17 | PROPOSED (coordinator) | **Acceptance without fixtures.** If FX-08 has not merged, LR-13 copies `first-session.toml` into a scratch spec folder, replaces FS-P3 with the recorded row (`-CharacterExpr 'Px${run_id}'`), and runs `FS-01,FS-02,FS-P1,FS-P2,REC-01` there. With FX-08, the draft row carries a fixture and runs alone. | Both prove the same thing; neither commits the scratch copy. |

## Packets

| ID | Packet | Implementer | Size | Wave | Depends on | Status |
|---|---|---|---|---|---|---|
| LR-01 | Record log types, JSONL reader and the FS-P3 log fixture | packet-coder | S | 1 | none | Ready |
| LR-02 | Inbound decoder and row segmenter | packet-coder | M | 2 | LR-01, GD-02 | BlockedDependency |
| LR-03 | Pose and camera derivation | packet-coder | S | 1 | SV-03 | BlockedDependency |
| LR-04 | Clauses from snapshot diffs, packet clauses, timeouts | packet-coder | M | 2 | LR-01, LR-02, SV-03 | BlockedDependency |
| LR-05 | TOML emitter, TODO blocks, caps, the drafts guard | packet-coder | M | 3 | LR-02, LR-03, LR-04, SV-04 | BlockedDependency |
| LR-06 | Recorder sampler and the daemon's read-only probes | rust-gameserver-dev | L | 2 | LR-01 | BlockedDependency |
| LR-07 | MCP tools `lab_record_start`, `lab_record_status`, `lab_record_stop` | packet-coder | M | 4 | LR-05, LR-06 | BlockedDependency |
| LR-08 | `lab record` command and `test-record.ps1` | packet-coder | S | 5 | LR-07 | BlockedDependency |
| LR-09 | Optional `[row.fixture]` emission | packet-coder | S | 4 | LR-05, FX-01 | BlockedDependency |
| LR-10 | Lab Lua ring for CEGUI window clicks | rust-gameserver-dev | M | 1 | none | Ready |
| LR-11 | Sampler and transform: window clicks become `@window_click` | packet-coder | S | 4 | LR-05, LR-06, LR-10 | BlockedDependency |
| LR-12 | Docs: authoring guide, lab guide, tools README | documentation-writer | S | 6 | LR-08 | BlockedDependency |
| LR-13 | Live acceptance: record FS-P3, transform, 5 agreeing runs | coordinator (live UAT) | M | 6 | LR-08, SV-05, SV-07, SV-08, GD-08, D-LR12, D-LR15 | BlockedDecision |
| LR-14 | Rolling recorder mode | packet-coder | M | 7 | LR-07, D-LR13 | BlockedDecision |
| LR-15 | `.bug spec` trigger and capped draft | packet-coder | M | 7 | LR-14, D-LR13 | BlockedDecision |
| LR-16 | Live check: one `.bug spec` | coordinator (live UAT) | S | 8 | LR-15, D-LR15 | BlockedDecision |
| LR-17 | Close-out | documentation-writer | S | 9 | all | BlockedDependency |

Size: S under about 40k tokens, M 40k to 70k, L 70k to 100k.

Waves (packets in a wave touch disjoint files):

1. LR-01 (`uat/record/{mod,log}.rs`, fixtures), LR-03 (`uat/record/pose.rs`), LR-10 (`supervisor/events/lua_rings.rs`). LR-03 waits for SV-03 to merge.
2. LR-02 (`record/{decode,segment}.rs`), LR-04 (`record/clauses.rs`), LR-06 (`record/sampler.rs`, `supervisor/world/camera_read.rs`, `supervisor/record_slot.rs`).
3. LR-05 (`record/emit.rs`, `drafts/README.md`, the guard test).
4. LR-07 (`server/record.rs`, `server/mod.rs`, `lease/policy.rs`), LR-09 (`record/fixture.rs`), LR-11 (`record/sampler.rs` click read, `record/emit.rs` click arm; merges after LR-09).
5. LR-08 (`tools/lab/cli/record*.ps1`).
6. LR-12, LR-13.
7. LR-14, LR-15 (the `.bug spec` wave; only if D-LR13 is (a)).
8. LR-16. 9. LR-17.

Cross-campaign order: effort 4 runs after efforts 1 and 3 (handoff). LR-01 and LR-10 have no outside dependency and can start any time.

## Dependencies on sibling campaigns

| Needs | From | Used for |
|---|---|---|
| SV-03 `PoseSpec`, `PoseSpec::to_inline_toml`, `ServerWait`, `timeout_ms` on server clauses | lab-spec-vocab | LR-03, LR-04, LR-05 emit and validate the vocabulary |
| SV-04 seed tag validation | lab-spec-vocab | the drafts guard rejects a tag the seed lacks |
| SV-02 `server_log_tail { target, since_ms }` | lab-spec-vocab | LR-06's known-server-event probe (skipped, with a log line, when absent) |
| SV-05, SV-07, SV-08 | lab-spec-vocab | running a draft (LR-13) |
| GD-02 `decode_inbound`, `InArg` | lab-golden | LR-02 decodes the five calls lab-golden already decodes |
| GD-03 `normalise::DEFAULT_IGNORE` | lab-golden | the noise list LR-02 and LR-15 drop (LR-02 copies the three names if GD-03 is not merged, and a later packet swaps) |
| GD-08 `lab golden record` with `specs_dir` | lab-golden | D-LR12, LR-13 |
| FX-01 `FixtureSpec`; FX-08 | lab-fixtures | LR-09; D-LR17 |
| LW-01 `/status` `uat` | lab-watch | none: a recording is shown by the lease purpose; LR-17 notes it for lab-watch |

## Dispatch rules

- **Workers.** One packet each, in its own worktree and test database:
  `pwsh -NoProfile -File tools/build-lane/mk-worktree.ps1 lab-record/<packet>-<slug> <worktree>`.
  `packet-coder` (Haiku) unless the table says otherwise; `rust-gameserver-dev`
  for LR-06 and LR-10; `documentation-writer` for LR-12 and LR-17. The brief
  carries the worktree path, the packet section, the contract section and the
  commit subject with the attribution lines.
- **Review.** Each packet gets a Sonnet `packet-reviewer` on its commit
  range. LR-02, LR-04 and LR-05 also get `testing-validation-engineer` (does
  the fixture test prove the transform, or echo it). LR-06 also gets
  `server-authority-enforcer` (does the recorder stay read-only).
- **Shell.** PowerShell only: no bash, WSL or Git Bash, no direct `cargo`, no
  `git worktree prune`, no `git stash`. Every compiling command goes through
  `pwsh -NoProfile -File tools/build-lane/lane.ps1`.
- **The lab.** No packet but LR-13 and LR-16 touches the lab, a lab MCP tool
  or a running client; those two run only after the user says yes (D-LR15).
- **Ship.** `python tools/build-lane/ship.py pr -C <worktree> -m <msg>`, then
  `python tools/build-lane/ship.py merge <PR> --retire <worktree>` once the
  build-proving jobs and `lab.yml` pass. Update this table; write
  `worknotes/<packet>.md` when anything is left over.
- **Shared files.** Only LR-12 edits `docs/guides/automated-uat.md`,
  `docs/guides/live-research-lab.md` and `tools/README.md`; only LR-17 edits
  `docs/guides/unified-uat.md`, `docs/project-status.md` and the main-session
  memory. Other packets record their doc deltas in their worknote.

## Review outcomes

None yet. Where merged code differs from the packet specs, record it here;
the code is then the reference, not the spec.
