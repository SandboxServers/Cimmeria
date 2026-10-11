# Lab spec vocabulary: work packets (3 of 3)

> Type: work packets. The header rules, the lane commands and the
> [contract](work-packets.md#contract) are in
> [work-packets.md](work-packets.md); the calibration contract is in
> [work-packets-2.md](work-packets-2.md#calibration-contract). Ledger:
> [README.md](README.md).

## Contents

- [SV-13 Migrate first-session.toml](#sv-13-migrate-first-sessiontoml)
- [SV-14 Docs](#sv-14-docs)
- [SV-15 Acceptance live UAT](#sv-15-acceptance-live-uat)
- [SV-16 Close-out](#sv-16-close-out)

## SV-13 Migrate first-session.toml

**Implementer:** packet-coder. **Size:** S. **Wave:** 3. **Depends on:** SV-03, SV-04, SV-07, SV-08.
**Branch:** `lab-spec-vocab/sv13-first-session`. **Worktree:** `sv13`.
**Subject:** `test(uat): SV-13 first-session rows target by tag, stand at poses and wait on the server`

Why: README F1, F4, F8, F14, F20. The section is the vocabulary's first
user; SV-15 then calibrates it live.

File: `docs/guides/uat-specs/first-session.toml` only.

- **FS-P2, FS-S2:** the `{ wait_ms = 17000, label = "movie-over" }` step
  becomes `{ wait_server = { log = "Cinematic AoI hold: released", target =
  "aoi.cinematic_hold", fields = { witness_id = "${player_entity_id}" },
  timeout_ms = 30000 }, label = "movie-over" }`. Keep the clause id
  `movie-over` (README F14). Update the step's comment: the server's
  release line is the signal, `cancelMovie` or the 16 s timeout either way.
- **FS-P3:** `pose = [ <the contract's frost line> ]`. Setup: the
  `/gmgotoxyz` line and its `wait_ms = 2500` become `{ tool = "@stand",
  args = { pose = "frost" } }`. Steps: the three camera steps become `{
  tool = "@camera", args = { pose = "frost" }, label = "aim" }`; the click
  becomes `{ tool = "@world_click", args = { pose = "frost", button =
  "right", expect = "window" }, label = "frost" }`; drop `wait_ms = 1500`
  and `2000`. Clauses: drop `pitch` (the +22 deg intermediate no longer
  exists); `zoom` moves to `at = "aim"`; `aim-pitch` keeps `-28.8` with
  `tolerance = 1.0` (it is now set, not a by-product); `step-80623` and
  `letter-1360` get `timeout_ms = 5000`.
- **FS-P4:** `pose = [ <the guard line: radius 4.41, bearing 322.7, dy
  0.13, pitch -28.8, yaw 25.3, zoom 250.0, aim point, aim_dy_m 0.28,
  calibrated "2026-10-10 hand"> ]`. The same stand, camera and click
  rewrites. The tutorial and dialog steps stay as they are. `landed`
  stays (`-319.0 ± 0.5`: the pose's stand point); `pitch` keeps `-28.8`
  with tolerance 1.0; `step-80622` gets `timeout_ms = 5000`. Drop the
  `notes` sentence about the spawn point; say the body is point-aimed.
- **FS-P5:** drop `wait_ms = 3000`; `complete-622` gets `timeout_ms = 8000`.
- **FS-S3 to FS-S6:** one pose each from README F20, with no `pitch_deg`,
  `yaw_offset_deg` or `calibrated` (they are uncalibrated until SV-15):
  `hammond` (`SGCW1_GenHammond`, 3.43, 81.4, dy 0.09), `tealc`
  (`SGC_W1_Tealc`, 3.36, 90.3, dy 0.09), `elevator`
  (`SGC_W1_ElevatorButton1`, 3.40, 179.9, dy -1.13), `firearm`
  (`SGC_W1_FirearmBody`, 3.41, 90.5, dy 0.09, aim point 0.29). The stand,
  camera and click rewrites as above (`@camera` with a pose without pitch
  or yaw only faces); `expect` values unchanged; the post-click sleeps go,
  and each row's mission clause gets `timeout_ms = 5000` (FS-S5's `moved`
  clause too).
- **Header:** the calibration table is restated as poses (bearing,
  radius, the absolute pitch and yaw), the 17 s row becomes the server
  release line, and the "FS-P4 starts from FS-P3's camera" resume rule is
  deleted (absolute pitch removes it). State that the converted Praxis
  values are the 2026-10-10 hand calibration and SV-15 re-measures them,
  and that the SGU poses are geometric guesses until then.

Tests:

- `crates/lab/src/uat/mod.rs` `committed_specs_parse_and_validate` and
  SV-04's `every_committed_spec_tag_is_seeded` cover the file (run them).
- `crates/lab/src/uat/runner/tests.rs` `committed_specs_plan_against_main_tools`:
  add `client_hover_probe` to `MAIN_TOOLS` only if a row needs it (none
  should), and assert that with no server every first-session row with a
  pose or `wait_server` is BLOCKED naming the server lab MCP; then plan
  once more with a `FakeServer` and assert FS-P2 to FS-P5 and FS-S2 to
  FS-S6 are SKIPPED (ready). Fails if a migrated row needs a tool nobody
  routes.

Checks: the lane's `nextest` for `-p cimmeria-lab` (no Rust changes besides
the test). No lab run here.

Docs owed: none here (SV-14 explains the vocabulary with this file as the
example).

Reviewer focus: every converted stand point equals the old `/gmgotoxyz`
point to 0.05 m (compute them); clause ids unchanged except the dropped
`pitch`; nothing claims a live run.

## SV-14 Docs

**Implementer:** documentation-writer. **Size:** M. **Wave:** 4. **Depends on:** SV-12, SV-13.
**Branch:** `lab-spec-vocab/sv14-docs`. **Worktree:** `sv14`.
**Subject:** `docs(lab): SV-14 spec vocabulary, calibration and heal in the UAT and lab guides`

Doc-update-map rows: "Live research lab ... the lab scripts and `lab`
command under `tools/lab/`" (the lab guide's `lab` section and the
`tools/README.md` rows), and the lab ADR only if the `/status` shape
changed (it did not).

Files:

1. `docs/guides/automated-uat.md`:
   - **Write a row spec:** a new "Targets, poses and waits" subsection:
     the `pose` line and its fields (D-SV3's geometry with a small figure
     in words: bearing 0 = +x, 90 = +z), `@stand`, `@camera { pose }`
     (D-SV4's order), `@world_click { pose }`, bare `tag` / `face_tag`
     arguments, `aim = "point"` and when to use it, `wait_server`, and
     `timeout_ms` on tool and server clauses. The actions table gains the
     `wait_server` row; the row-field list gains `pose`; the variable list
     notes that `${player_entity_id}` is resolved from the client for
     waits and tags.
   - **The capability table:** `@stand` and `@hover_probe`.
   - **New section "Calibrate a row":** `lab calibrate`, what it sweeps
     and scores (D-SV6, hover only, never a click), `-Prepare`, `-Apply`,
     the run folder (`calibrate.jsonl`, the copy, the diff), reviewing and
     committing the diff by hand.
   - **Run without an agent: `lab uat`:** `-Heal` and exit code 5.
   - **Troubleshooting:** "no entity with tag" (wrong space, not yet
     introduced, or a typo the seed check missed), "stand: still n m"
     (the teleport was refused or the point is inside geometry), a
     `wait_server` timeout (an old server without SV-02, or the event did
     not happen).
2. `docs/guides/live-research-lab.md` § Commands: `lab calibrate` and `lab
   uat -Heal`; the tool table gains `client_hover_probe`, the
   `server_entity_query` `tag` argument and the `server_log_tail` filters
   and `newest_ms`; `client_camera`'s absolute arguments.
3. `tools/README.md`: the `lab/` rows for `calibrate.ps1`,
   `calibrate-lib.ps1` and `test-calibrate.ps1`.
4. `.claude/agents/lab-driver.md`: the 17 s rule (line 51) becomes "a spec
   row waits on the server's release line; outside a spec, wait 17 s as
   before"; add that `lab_uat_calibrate` is never called by the driver
   (it needs the user's OK like any lab use, and the coordinator runs it).
5. `crates/README.md`: the `lab` row mentions calibration in its one line
   if the row lists the UAT runner's features (check first).

Checks: `pwsh -NoProfile -File tools/lint-md.ps1` (warn-only); every link
resolves.

## SV-15 Acceptance live UAT

**Implementer:** the coordinator, driving the `lab` CLI (no agent
inference in the loop). **Size:** M. **Wave:** 5. **Depends on:** SV-12, SV-13, D-SV9 (the user's OK), D-SV7.
**Branch:** `lab-spec-vocab/sv15-acceptance`. **Worktree:** `sv15`.
**Subject:** `test(uat): SV-15 calibrated first-session poses, five clean runs`

**Live UAT packet. Ask the user before starting, and wait until the lab is
free.** Follow the `lab-uat` skill for the lease and the server choice
(D-SV7). Record every run id in `worknotes/SV-15.md`.

Steps:

1. Install the merged lab build on the daemon (`lab install`, `lab
   restart`), and check `lab doctor` is clean and the server the lab
   client logs in to is the one `server_db_query` reads (the stale
   `Local` row trap in `reference_first_session_lab_calibration_2026_10_10.md`).
2. **Praxis from scratch.** Delete the FS-P3 and FS-P4 pose values
   (`calibrated` too) in the worktree's spec, keeping tag, radius and
   bearing as the sweep's start. Then:
   `lab calibrate first-session FS-P3 -Prepare FS-01,FS-02,FS-P1,FS-P2 -Apply`
   and `lab calibrate first-session FS-P4 -Apply` (the client is still in
   world after FS-P3's calibration; FS-P4's target needs no mission state
   to hover, and calibration never clicks).
3. **SGU.** `lab calibrate first-session FS-S3 -Prepare FS-01,FS-02,FS-S1,FS-S2 -Apply`,
   then FS-S4, FS-S5 and FS-S6 the same way without `-Prepare` (each
   teleports to its own pose). Whether every SGU target is in the
   player's space before its mission step is not recorded anywhere: if a
   calibration stops with "no entity with tag", run the rows before it
   (`lab uat first-session -Rows FS-S3,...`) and calibrate again, and note
   which target needed it in the worknote.
4. **Five clean runs in a row:** `lab uat first-session -Rows
   FS-01,FS-02,FS-P1,FS-P2,FS-P3,FS-P4,FS-P5,FS-S1,FS-S2,FS-S3,FS-S4,FS-S5,FS-S6,FS-99 -RunsPerLease 5 -Heal`.
   Passing means 5 of 5, all rows PASS, with no `wait_ms` left on an
   interaction row. A #1341 failure (FS-P2 or FS-S2 `movie-over`, tagged by
   the summary) is a product bug, not a calibration miss: record it and
   restart the count. Any heal proposal: review it, apply it if it makes
   sense, and restart the count.
5. Commit the spec diff (the calibrated poses with their `calibrated`
   stamps) and the worknote: the five run ids, each row's pass rate, the
   calibrate run folders, and the time the full sweep took per pose.

Docs owed: the spec header's "SV-15 re-measures them" note becomes the
measured result (date, build, the 5/5 run ids).

Definition of done: the five clean runs are in the worknote, and the
ledger row moves to Done.

## SV-16 Close-out

**Implementer:** documentation-writer. **Size:** S. **Wave:** 6. **Depends on:** all.
**Branch:** `lab-spec-vocab/sv16-close-out`. **Worktree:** `sv16`.
**Subject:** `docs(lab): SV-16 lab spec vocabulary close-out`

Files:

1. This ledger: every packet row's final status, the "Review outcomes"
   section, and the campaign status line.
2. `docs/guides/unified-uat.md`: the first-session section names the
   calibrated rows and the `lab uat -Heal` path; any UAT step the owner
   still has to run.
3. `docs/project-status.md`: one line under the lab tooling entry (it is
   tooling, not a game system, so `docs/gap-analysis*` does not change;
   say so in the PR body).
4. `.claude/agent-memory/main-session/reference_first_session_lab_calibration_2026_10_10.md`:
   mark the hand-calibration traps that the vocabulary retired (the 17 s
   wait, the camera order, the resume rule) as superseded, with the date
   and a link to `docs/guides/automated-uat.md`; update its line in
   `.claude/agent-memory/main-session/MEMORY.md`.
5. Retire every worker worktree: `pwsh -NoProfile -File tools/build-lane/rm-worktree.ps1 --merged`.
6. Post the close-out handoff in the campaign's board subcategory and in
   `handoffs`.
