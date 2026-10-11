# Lab spec vocabulary and self-calibration

> Type: ledger. Audience: the coordinator, packet workers and reviewers.
> Opened 2026-10-10 against `main` @ `6996c9403`. Prefix `SV-`. Source brief:
> [lab-roadmap handoff](../lab-roadmap/handoff.md), effort 1. Packet specs:
> [work-packets.md](work-packets.md) (contract, SV-01 to SV-05),
> [work-packets-2.md](work-packets-2.md) (SV-06 to SV-12, with the
> calibration contract) and [work-packets-3.md](work-packets-3.md) (SV-13
> to SV-16).
>
> **Campaign status (2026-10-10): planned, nothing built.** Three decisions
> (D-SV7, D-SV8, D-SV9) are BlockedDecision. Wave 1 (SV-01, SV-02, SV-03,
> SV-05) can start now.

## Purpose

Make a UAT spec step say *what* it acts on and *how the view should look*,
not where a person once measured it. After this campaign:

- a step names its target by the server's spawn tag
  (`ArmYourself_GuardBody`), and the runner finds that entity in the
  player's own space;
- the camera is set to an absolute pose (pitch, yaw offset after facing,
  zoom), closed loop on the client's own readout, so a row no longer
  depends on where the previous row left the camera;
- waits end on a server event (the first-login hold's release) or on a
  clause that polls until it passes, instead of fixed sleeps;
- `lab calibrate <section> <row>` finds a stand-off and camera pose that
  hover-verifies, with no click, and writes it into the spec as a diff for
  review;
- `lab uat -Heal` proposes that diff when a row fails on a hover miss, and
  exits non-zero. Nothing is committed automatically.

This is the base the other lab-roadmap campaigns build on: `lab-fixtures`
reuses the stand mechanism, `lab-record` emits steps in this vocabulary
(the [contract](work-packets.md#contract) gives the exact TOML and Rust
names), and `lab-golden` fingerprints runs of specs written in it.

Out of scope: row fixtures and rewind (`lab-fixtures`), run fingerprints
(`lab-golden`), any new client patch, and any server write path. Every
server call this campaign adds is a read.

## What was found

Against `main` @ `6996c9403`. Paths are repo-relative.

| # | Finding | Packets |
|---|---|---|
| F1 | Every interaction point in `first-session.toml` is hand-measured: the Frost and Guard stand-offs and the Guard click point (header lines 64-71, steps at 270, 381, 386-388), and the SGU points, which the header says are "still computed from the spawn rows" and uncalibrated (lines 36-38; steps at 570, 595, 620, 656-660). | SV-03, SV-07, SV-13, SV-15 |
| F2 | The server already carries the tag: `LabEntitySnapshot.tag` (`crates/wire/src/cell/messages/lab.rs:168`). But `LabEntityFilter` has no tag field (`lab.rs:67-77`), the filter loop does not test one (`crates/cell-world/src/cell/space_manager/lab_snapshots.rs:62-89`), and `server_entity_query` takes no tag argument (`crates/lab-mcp/src/tools/mod.rs:71-90`). The query walks every space (`lab_snapshots.rs:62`). Runtime entity ids change per login (wireclient-ci F5), so one tag can match in several players' spaces at once: resolution must be scoped to the lab player's space. | SV-01, SV-07 |
| F3 | Tags are `spawnlist.tag` (`db/resources/Worlds/Tables/spawnlist.sql:14`, `varchar(100)`, no unique constraint), seeded by 632 `INSERT INTO spawnlist` rows across 8 files in `db/resources/Worlds/Seed/` (`spawnlist.sql` and seven `spawnlist_*` / `debug_area_rings.sql` files). The rows this campaign targets: `ArmYourself_FrostBody` (`spawnlist.sql:86`), `ArmYourself_GuardBody` (`:68`), `SGCW1_GenHammond` (`:115`), `SGC_W1_Tealc` (`:117`), `SGC_W1_ElevatorButton1` (`:121`), `SGC_W1_FirearmBody` (`:165`). Note the two spellings, `SGCW1_` and `SGC_W1_`: a typo in a spec is exactly what seed validation must catch. | SV-04 |
| F4 | `client_camera` takes relative counts, a wheel count or `zoom_to`, and a face target (`crates/lab/src/supervisor/world/camera.rs:27-54`). The face controller stops once the target is inside its margin, so the pitch it ends on depends on the pitch it started from: -28.8 deg from a +22 deg start, about -19 deg from level (`first-session.toml:52-58`, `:89-92`, `:402`). That is why the spec header needs a resume rule ("call `client_camera {pitch_counts: 200}` once first"). | SV-05, SV-13 |
| F5 | The camera readout has what an absolute pose needs: `zoom`, `yaw_offset_deg`, `pitch_offset_deg`, `gain` and `pitch_gain` (`crates/lab/src/supervisor/world/memory.rs:316-323`). Pitch inversion (flag `0x8`) is folded into `pitch_gain` (`:308-314`); yaw inversion (`0x4`, `:300-301`) is not exposed. The degrees-to-counts formula already exists in `restore_pitch_counts` (`click.rs:372-384`), and `clamped_pitch_counts` keeps a turn inside 78.75 deg (`memory.rs:352-369`). | SV-05 |
| F6 | The readout's pitch sign is not documented as up or down. The header calls +22 "tilted down" after +200 counts, and the face that re-centres a floor target then reads -28.8 (`first-session.toml:52-58`; memory note `reference_first_session_lab_calibration_2026_10_10.md`, line 11). An absolute pitch therefore uses the readout's own number, the one the existing `pitch` and `aim-pitch` clauses grade, and never reinterprets it. | D-SV4, SV-05 |
| F7 | `+230` yaw counts is `+25.27` deg at gain 20 and `+200` pitch counts is `+21.97` deg (`65536` rotator units per turn). Whether the readout's `yaw_offset_deg` moves when mouse-look turns the pawn is not recorded anywhere; the camera actor's world yaw (`camera_pose`, `camera.rs:105`) is what the face controller already trusts. | D-SV4, SV-05 |
| F8 | Two 17 s sleeps wait out the first-login hold (`first-session.toml:208`, `:525`), and the section has 21 `wait_ms` steps in all. The server logs the release at `aoi.cinematic_hold` INFO with `event = "hold_released"`, `witness_id` (the player entity) and `witness_name` (`crates/base-world-entry/src/base/world_entry_appearance/cinematic_aoi_hold/mod.rs:217-227`). `server_log_tail` filters by level only (`crates/lab-mcp/src/tools/logs.rs:15-36`), over a ring of 500 entries or 2 MiB (`crates/admin-api/src/ws/broadcast_layer/mod.rs:35`, `:39`). | SV-02, SV-08, SV-13 |
| F9 | `server` and `tool` clauses are evaluated once. A mission clause after a 2000 ms sleep (FS-P3, `first-session.toml:289`, `:355-362`) races the step's persistence; `timeout_ms` exists only for `wait` and `client_event` clauses (`crates/lab/src/uat/spec.rs:324-328`). | SV-08 |
| F10 | The runner already has the two patterns this campaign needs. Runner-level tools that are not router tools: `TARGET_PLAYER_TOOL` (`crates/lab/src/uat/tools.rs:18`) and the lab dot commands (`runner/lab_commands.rs`, routed by `players.rs:89-106`). Recorded server calls: `server_call` (`runner/packet.rs:105-127`). | SV-07, SV-08 |
| F11 | `tap_entity` finds the player's entity by the section's character name in `server_sessions` (`runner/packet.rs:131-152`). The first-session rows play `Px${run_id}`, not the section's `lab` character, so that lookup names the wrong player there. The client knows its own entity id: `client_entity_find` returns `/player/id` (`supervisor/world/find.rs:289-293`). | SV-07 |
| F12 | There is no hover-only primitive. `client_world_click` always presses (`click.rs:483-529`); `expect = "nothing"` still clicks. The hover logic it would reuse is `hover_once` (`click.rs:388-467`) with `HOVER_OFFSETS` (`:30`). | SV-06 |
| F13 | The Guard body never answers an entity hover; a point click on its torso does (`first-session.toml:70-71`). Hover-only scoring of a point aim cannot require `mouse_over == target`; it can require that nothing else (the avatar, another entity, a HUD window) is under the cursor. | D-SV6, SV-06, SV-09 |
| F14 | `lab uat` tags an `FS-P2`/`FS-S2` failure whose reason mentions `movie-over` as known issue #1341 (`tools/lab/cli/uat-lib.ps1:26`). Replacing the sleep must keep the clause id `movie-over`, or #1341 failures stop being tagged. | SV-13 |
| F15 | `RowSummary` carries only `result` and `reasons` (`runner/mod.rs:80-86`), so `-Heal` cannot tell a hover miss from any other failure without parsing prose. | SV-11 |
| F16 | Specs load through `load_sections` (`crates/lab/src/uat/mod.rs:66-99`), guarded by `committed_specs_parse_and_validate` (`:125-138`). That is the hook for seed-tag validation; the repo's seed sits at `<specs_dir>/../../../db/resources/Worlds/Seed`. | SV-04 |
| F17 | `server/uat.rs` is 660 lines (over the soft cap) and its `RouterInvoker` is private (`:119`). `lab_uat_run` is the only `OWN_LEASE` tool (`crates/lab/src/lease/policy.rs:42`); unknown tools fail closed to needing a lease (`:139`). | SV-06, SV-10 |
| F18 | `lab` subcommands are discovered from `tools/lab/cli/*.ps1` (`tools/lab/lab.ps1:32-65`), and `.github/workflows/lab.yml` runs every `tools/lab/**/test-*.ps1` (`:91-106`). A new `calibrate.ps1` and `test-calibrate.ps1` need no wiring. | SV-12 |
| F19 | `cimmeria-lab` depends on `toml` (serde) only, no TOML-editing or diff crate (`crates/lab/Cargo.toml:16-55`). | D-SV2 |
| F20 | The calibrated values convert exactly to the pose form ([D-SV3](#decisions)). Frost: target (-328.30, 73.472, -210.27), stand (-325.0, 73.6, -212.8), so radius 4.16 m, bearing 322.5 deg, dy 0.13 m. Guard: target (-322.51, 73.472, -209.83), stand (-319.0, 73.6, -212.5), so radius 4.41 m, bearing 322.7 deg, dy 0.13, aim point +0.28 m. SGU (uncalibrated): Hammond 3.43 m at 81.4 deg, Teal'c 3.36 m at 90.3 deg, the elevator button 3.40 m at 179.9 deg (dy -1.13), the firearm body 3.41 m at 90.5 deg (aim +0.29). | SV-13 |

## Decisions

| ID | Status | Decision | Reason |
|---|---|---|---|
| D-SV1 | PROPOSED (coordinator) | **Tags are resolved by the runner, never by a client tool.** The runner finds the lab player's entity id from the client (`client_entity_find` `include_player`), its space from `server_entity_get`, then the tagged entity with `server_entity_query { space_id, tag }`, and filters the reply by tag itself as well. It hands the client tool a plain `entity_id` (or `point`). Zero matches and more than one match are both errors that name the tag and the space. | F2, F11. The client has no tags; the server does. Filtering the reply again keeps the runner correct against a server older than SV-01 (the colo lags `main`), which ignores the unknown `tag` argument and returns the whole space. |
| D-SV2 | PROPOSED (coordinator) | **Poses live in the row as single-line inline tables** (`pose = [ { id = "frost", ... }, ]`), one per line, in the canonical key order the contract gives. `lab calibrate` rewrites only those lines, re-parses the whole file to prove nothing else changed, and prints a line diff it builds itself. No new dependency. | F19. TOML 1.0 inline tables must be on one line, so a pose is always exactly one line to replace. Adding `toml_edit` and a diff crate means a hakari change for something a line replace does. |
| D-SV3 | PROPOSED (coordinator) | **Stand-off geometry.** A stand point is `target + radius_m * (cos b, 0, sin b)` in server metres (x, z), with `b = bearing_deg` measured from +x towards +z, and `y = target.y + dy_m`. The target position is the *live* server position, not the seed's. | F20 shows both calibrated Praxis points come out at the same bearing (322.5 and 322.7) and height (+0.13), which is what a geometric description should do. The live position covers anything that moves. |
| D-SV4 | PROPOSED (coordinator) | **Absolute camera semantics.** `client_camera` applies, in order: relative counts, zoom (`zoom` or `zoom_to`), face, then `pitch_deg`, then `yaw_offset_deg`. `pitch_deg` is the readout's `pitch_offset_deg`, closed loop (up to 4 looks, 0.5 deg tolerance). `yaw_offset_deg` is degrees in the direction `+yaw_counts` turns, relative to the yaw the face left (or to the yaw at the call's start without a face), closed loop on the camera actor's world yaw, with the counts-per-degree learned from the first look. Relative counts keep working unchanged. | F4 to F7. Pitch is graded today in readout degrees, so absolute pitch must be the same number. Yaw has no trusted readout, so it is measured where the face controller measures it. |
| D-SV5 | PROPOSED (coordinator) | **Server-event waits poll `server_log_tail`**, which gains three read-only filters (`target`, `contains`, `since_ms`). There is no new lab-mcp tool. A wait takes the ring's newest `timestamp_ms` as its watermark before the action it follows, and passes on the first newer entry whose message contains `log` and whose fields equal `fields`. The runner filters again itself (an older server ignores the new arguments). | F8. It needs no new tool on the lab-server's fixed tool set, and the watermark avoids comparing the server's clock with the host's (rows with `state = "any"` have no `.bug` anchor to give an offset). |
| D-SV6 | PROPOSED (coordinator) | **Calibration scores by hover only, never a click.** A candidate pose passes when the aim pixel hovers the target (`aim = "entity"`) or, for `aim = "point"`, hovers neither the avatar, nor another entity, nor a HUD window. Its score is `jitter_hits * 1000 + min(avatar_px, 400) + min(edge_px, 200) / 2`: eight cursor points 12 px around the aim pixel must give the same verdict, the aim pixel should be far from the avatar's projected body and from the screen edge. Ties go to the earlier candidate in a fixed order. The winner is confirmed 3 times from a re-teleport before it is proposed. | Handoff ("score each pose by hover only"); F13. Jitter measures robustness without knowing the HUD's layout: a pose next to a HUD window or the avatar loses jitter hits. A click would advance the mission and spoil the next candidate. |
| D-SV7 | **BlockedDecision** (owner). Recommended: **(a)** | **Where calibration may run.** A full sweep teleports the GM lab character up to 36 times per pose (3 radii x 12 bearings), each with `/gmgotoxyz`. (a) Allowed on the colo for the lab account, one pose at a time, and on any local server. (b) Local servers only. | (a) is where the specs are run, so its geometry is the geometry that matters (map patches and seeds match the colo). The teleports are GM commands the specs already use for every interaction row; they touch nothing persistent. (b) avoids GM noise in colo telemetry but calibrates against a server that can differ. |
| D-SV8 | **BlockedDecision** (owner). Recommended: **(a)** | **What `lab uat -Heal` does.** (a) After the batch, for each failed row whose summary carries a heal hint (a hover or on-screen failure on a pose), it runs `lab_uat_calibrate` for that pose on the same instance, writes the proposed diff into the batch folder, prints it, and exits 5. It never writes the spec file and never commits. `-Heal` needs `-Leases 1`. (b) It only prints the `lab calibrate` command to run. | (a) is the handoff's "run calibrate for that row and print the proposed spec diff". It adds up to about 8 minutes of lab time per failed pose, which is why it is opt-in. (b) costs nothing but leaves the work to a person. |
| D-SV9 | **BlockedDecision** (user). | **The acceptance packet SV-15 drives the lab.** It recalibrates FS-P3 and FS-P4 from scratch and calibrates FS-S3 to FS-S6, then needs 5 clean runs in a row. It runs only with the user's OK, when the lab is free, as a live UAT packet. | Repo rule: ask the user before any lab use (handoff "Rules every campaign inherits"). |
| D-SV10 | PROPOSED (coordinator) | **Seed validation.** Every tag a spec names (`pose[].tag`, and `tag` / `face_tag` in any action's `args`) must exist in the seed. A committed test checks every spec under `docs/guides/uat-specs/` against every `INSERT INTO spawnlist` in `db/resources/Worlds/Seed/`. `load_sections` also checks, at run time, when it finds that seed folder next to the specs (a checkout); without it, it records nothing and the live resolution's "no entity with tag" is the backstop. | Handoff ("spec validation must reject unknown tags against the seed"); F3, F16. The daemon can run from an installed copy with no seed beside it, so the committed test is the gate and the run-time check is a convenience. |
| D-SV11 | PROPOSED (coordinator) | **`@stand` replaces "teleport, then sleep 2500".** It types `/gmgotoxyz` for the pose's stand point (tier G) and polls `client_entity_find { include_player }` every 250 ms until the player's server position is within 0.5 m horizontally of the point, for up to 5 s. | F1, F8. Arrival is observable, so there is no reason to sleep. `lab-fixtures` can reuse the same function for its `position`. |
| D-SV12 | PROPOSED (coordinator) | **Output budget.** `lab calibrate -Json` prints one object under about 400 characters: `ok`, `section`, `row`, and per pose `pose`, `ok`, `score`, `confirmed`, the changed fields only, and `diff` (the diff file's path); never the candidate list (it goes to `calibrate.jsonl` in the run folder). No nulls, no empty arrays. `lab uat -Json` keeps its contract and adds `heal` (an array of `{row, pose, diff}`) only when `-Heal` proposed something. | Handoff output-budget rule; `ConvertTo-UatCompactJson` (`uat-lib.ps1:276`) is the model. |

## Packets

| ID | Packet | Implementer | Size | Wave | Depends on | Status |
|---|---|---|---|---|---|---|
| SV-01 | Server entity query by tag | packet-coder | S | 1 | none | Ready |
| SV-02 | `server_log_tail` filters: target, contains, since_ms | packet-coder | S | 1 | none | Ready |
| SV-03 | Spec schema: poses, `wait_server`, tag arguments, capability rows | packet-coder | M | 1 | none | Ready |
| SV-04 | Seed tag index and spec tag validation | packet-coder | S | 2 | SV-03 | BlockedDependency |
| SV-05 | Absolute camera in `client_camera` | packet-coder | M | 1 | none | Ready |
| SV-06 | `client_hover_probe`: hover without a click | packet-coder | M | 2 | SV-05 (sim readout) | BlockedDependency |
| SV-07 | Runner target resolution and `@stand` | rust-gameserver-dev | M | 2 | SV-03 (SV-01 live) | BlockedDependency |
| SV-08 | Server-event waits and polling clauses | packet-coder | M | 2 | SV-02, SV-03 | BlockedDependency |
| SV-09 | Calibration core: candidates, score, spec edit and diff | packet-coder | M | 2 | SV-03 | BlockedDependency |
| SV-10 | `lab_uat_calibrate`: the sweep on a live client | rust-gameserver-dev | L | 3 | SV-05, SV-06, SV-07, SV-09, D-SV7 | BlockedDecision |
| SV-11 | Heal hint on a row summary | packet-coder | S | 3 | SV-07 | BlockedDependency |
| SV-12 | `lab calibrate` and `lab uat -Heal` | packet-coder | M | 4 | SV-10, SV-11, D-SV8 | BlockedDecision |
| SV-13 | Migrate `first-session.toml` to the new vocabulary | packet-coder | S | 3 | SV-03, SV-04, SV-07, SV-08 | BlockedDependency |
| SV-14 | Docs: authoring guide, lab guide, lab-driver brief, tool rows | documentation-writer | M | 4 | SV-12, SV-13 | BlockedDependency |
| SV-15 | Acceptance: recalibrate the Praxis rows, calibrate the SGU rows, 5 clean runs | coordinator (live UAT) | M | 5 | SV-12, SV-13, D-SV9 | BlockedDecision |
| SV-16 | Close-out | documentation-writer | S | 6 | all | BlockedDependency |

Size: S under about 40k tokens, M 40k to 70k, L 70k to 100k.

Waves (packets in one wave touch disjoint files and run in parallel):

1. SV-01 (`crates/wire`, `crates/cell-world`, `crates/lab-mcp` entities),
   SV-02 (`crates/lab-mcp` logs), SV-03 (`crates/lab/src/uat/spec*.rs`,
   `tools.rs`, the runner's `ActionKind` arms), SV-05
   (`crates/lab/src/supervisor/world/camera.rs`, `memory.rs`, `sim.rs`).
   SV-01 and SV-02 both touch `crates/lab-mcp/src/tools/mod.rs`, in
   different structs and methods; the second to merge rebases.
2. SV-04 (`uat/seed_tags.rs`, `uat/mod.rs`), SV-06 (`supervisor/world/hover.rs`,
   `server/world.rs`, `lease/policy.rs`), SV-07 (`runner/targets.rs`), SV-08
   (`runner/server_wait.rs`, `runner/clauses.rs`), SV-09 (`uat/calibrate/`).
   SV-07 and SV-08 each add one call in `runner/actions.rs` and one field
   in `runner/mod.rs`'s `RowCtx`; the second to merge rebases.
3. SV-10 (`uat/calibrate/drive.rs`, `server/calibrate.rs`), SV-11
   (`runner/mod.rs`, `runner/heal.rs`), SV-13 (`docs/guides/uat-specs/first-session.toml`).
4. SV-12 (`tools/lab/cli/`), SV-14 (docs).
5. SV-15 (live; spec values).
6. SV-16.

## Contracts other campaigns consume

- **`lab-record`** emits rows in this vocabulary: `pose` lines, `@stand`,
  `@camera { pose }`, `@world_click { pose }`, `wait_server`, and `server`
  clauses with `timeout_ms`. The exact TOML and the Rust type names are in
  [work-packets.md § Contract](work-packets.md#contract). It should write a
  pose's `calibrated` field as `"recorded <date>"`, so a recorded pose is
  visibly unconfirmed until `lab calibrate` or `lab golden` passes it.
- **`lab-fixtures`** also edits `crates/lab/src/uat/spec.rs`. This
  campaign's additions sit in one block of `RowSpec` and `ActionSpec`,
  marked `// lab-spec-vocab (SV-03)`; the fixtures campaign adds its
  `fixture` field after that block. Its `position` is set server-side with
  `.gotoxyz` through `server_console_exec` straight after the fixture relog,
  when the fresh entity has no selected target (lab-fixtures F15, FX-06).
  `runner::targets::stand_at` (SV-07) stays the path for mid-row stand-offs.
  Both are recorded in the [lab roadmap](../lab-roadmap/README.md).
- **`lab-golden`** can key a row's interactions by pose id and tag instead
  of runtime entity ids: the runner records the resolved
  `{tag, entity_id, space_id}` in each action's `calls`.
- **`lab-watch`** needs nothing new; `lab calibrate` writes progress lines
  to `calibrate.jsonl` in its run folder.
- **`client-limits` / `lab-chaos`**: none.

## Dispatch rules

- **Workers.** One packet each, in its own worktree and test database:
  `pwsh -NoProfile -File tools/build-lane/mk-worktree.ps1 lab-spec-vocab/<packet>-<slug> <worktree>`.
  `packet-coder` (Haiku) for packets marked so; `rust-gameserver-dev` for
  SV-07 and SV-10; `documentation-writer` for SV-14 and SV-16. The brief
  carries the worktree path, the packet section, the contract section, and
  the commit subject with the attribution lines.
- **Review.** Each finished packet gets a Sonnet `packet-reviewer` on its
  commit range. SV-07 and SV-10 also get `testing-validation-engineer`
  (do the fakes prove the resolution and the sweep, or only echo them).
- **Shell.** PowerShell only: no bash, WSL or Git Bash, no direct `cargo`,
  no `git worktree prune`, no `git stash`. Every compiling command goes
  through `pwsh -NoProfile -File tools/build-lane/lane.ps1`.
- **The lab.** No packet but SV-15 touches the lab, the lab MCP tools, or a
  running client. SV-15 runs only after the user says yes (D-SV9).
- **Ship.** `python tools/build-lane/ship.py pr -C <worktree> -m <msg>`,
  then `python tools/build-lane/ship.py merge <PR> --retire <worktree>` once
  the build-proving jobs and `lab.yml` pass. Update this table and write
  `worknotes/<packet>.md` when anything is left over.
- **Shared files.** Only SV-14 edits `docs/guides/automated-uat.md`,
  `docs/guides/live-research-lab.md`, `.claude/agents/lab-driver.md` and
  `tools/README.md`; only SV-16 edits `docs/guides/unified-uat.md`,
  `docs/project-status.md` and the main-session memory note. Other packets
  record their doc deltas in their worknote for SV-14.

## Review outcomes

None yet. Where merged code differs from the packet specs, record it here;
the code is then the reference, not the spec.
