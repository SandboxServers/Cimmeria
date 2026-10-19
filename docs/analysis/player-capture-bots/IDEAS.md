# Ideas: human play as lab UAT input

> Type: proposal list for the owner. The owner ruled on these on 2026-10-18; the status table records each ruling and the packets that build it. Campaign ledger: [README.md](README.md).

## Status (owner, 2026-10-18)

| Idea | Status | Packets |
|---|---|---|
| 1 Human pose library | Approved; idea 7's facing error folded in | PC-20, PC-21, PC-21b (camera join), PC-21c (camera keys) |
| 2a `session_kind` on `movement.position_sample` | Approved | PC-10 |
| 2b Approach trails | Approved | PC-12, PC-13 |
| 3 Human friction as lab rows | Approved: friction moves from WARN to DEBUG and goes to SigNoz only, never Discord; then drafts or `.bug` files | PC-11, PC-14a, PC-14b |
| 4 Camera pose at interaction | Approved: the opt-in covers it ("telemetry is telemetry", 2026-10-18; no disclosure change). Joined to the pose row by Mercury sequence, then server time (D-PC18 to D-PC20) | PC-15, PC-16, PC-16b, PC-26, PC-27, PC-17, PC-18, PC-19, PC-07b |
| 5 Playerbots | Stages 0-1 are ideas 1 and 2b. Stages 2-4 wait for a later architect pass | none yet |
| 6 UAT coverage map | Approved | PC-22 |
| 7 Facing and turn calibration | Facing error folded into idea 1. The turn rate is not sliced | PC-20 |
| 8 Dialog and UI think time | Deferred | none |
| 9 Headless replay | Approved; server-only, and says so | PC-23, PC-24 |
| 10 Region dwell heatmaps | Deferred | none |

The sections below are the original proposals, kept as they were written. The status table above supersedes each section's "Recommendation".

## What already exists, so nothing is duplicated

| Source | Captures | Sessions | Gap it leaves |
|---|---|---|---|
| Lab record ([ledger](../lab-record/README.md)) | Every 500 ms while a human drives a **lab** client: taps, server position, the camera readout, UI state, target entities, DB snapshots. Transforms into draft spec TOML with pose rows (D-LR6). `lab_route_record` records route legs. | Lab only; player sessions are out of scope (F11, D-LR13) | Real players on colo, at scale, playing however they like |
| `movement.position_sample` | One accepted position every 5 s per moving player; exported (DEBUG row in `OTEL_FILTER`) | Every player | No `session_kind`; too sparse for an approach path |
| `movement.validation_reject` | Where movement was refused | Every player | Negative space only |
| `playtest.friction` | No-effect interacts, repeated failures, dwell without a hint; carries `cimmeria.session_kind` | Every player | Says where humans got stuck, not how they succeeded |
| `interaction.pose` (this campaign) | Player pose, target pose and stand-off geometry at every accepted interaction | Every player, tagged | Camera (D-PC3), approach path (idea 2) |
| Goldens | Fingerprints of five agreeing lab runs | Lab | Built from lab runs only, not from human play |

There is no playerbot backlog anywhere in `docs/analysis/`.

## Ranked ideas

Cost: S is one or two packets; M is a small campaign of three to six packets; L is a multi-wave campaign. Dependency is on the core (`interaction.pose`, PC-01..PC-04).

### 1. Human pose library for `@stand` and `@camera`

`automated-uat.md` poses carry `radius_m`, `bearing_deg` and `dy_m`; today they come from a hand measurement or from `lab calibrate`. A small tool (a `lab` CLI subcommand) reads `interaction.pose` rows with `session_kind = player` for a `target_tag`. It takes the median radius and dy and the circular mean of bearing, with the spread. It writes or proposes the pose line with `calibrated = "human <date> n=<N>"`. Lab runs then stand where people actually stand, which addresses the 3.4 m camera block and the 6.4 m dead zone directly. A wide spread is also useful: it says a target works from many sides, and the lab can pick the side with the clearest camera line. The camera keys stay lab-calibrated until idea 4.

- Cost: M (query and aggregation, the TOML line writer reusing `lab calibrate`'s single-line replace, tests).
- Risk: low. Small samples on rarely used targets; the tool must refuse below a minimum n.
- Dependency: core, plus a week of colo play.
- Recommendation: **approve** once PC-07 shows human rows.

### 2. Approach trails and `session_kind` on the walked-surface sample

There are two parts.

- **(a), S.** Add `cimmeria.session_kind` to `movement.position_sample`. The walked-surface map can then exclude lab clients, which teleport and walk scripted lines.
- **(b), M.** Keep a per-player ring of the last eight accepted positions at 1 Hz (server-side; the accept path already has the instant). Attach it to the `interaction.pose` row as a compact `approach` field, or write it as a sibling `interaction.approach` row.

The trails show the paths humans take to reach a target: which doorway, which side, where they stop and turn. They seed route legs (`docs/guides/uat-specs/routes/`) and give the navmesh rebake real walked input (Harset H53). They also become the reference a real-input bot imitates (idea 5).

- Risk: low for (a). For (b), keep the field bounded (eight points, two decimals) and drop trails that contain a teleport (a jump of more than 10 m between samples).
- Dependency: (a) none; (b) core.
- Recommendation: **approve (a) now** as a one-packet add-on, and (b) after idea 1 proves the data useful.

### 3. Human friction as lab regression rows

`playtest.friction` rows with `session_kind = player` (no-effect interacts, out-of-range rejects, dwell without a hint) are bugs humans found by playing. A transform turns the most frequent ones into draft spec rows or `.bug` files: the target, the place and what was expected. Reproducing them becomes lab work instead of a manual triage.

- Cost: S (query plus transform into the existing draft-spec format).
- Risk: low. Some friction is user error; the drafts are proposals, not specs.
- Dependency: none (friction rows already carry `session_kind`); the core adds the success side for comparison.
- Recommendation: **approve**; cheap, and independent of the core.

### 4. Camera pose at interaction (opt-in DLL, D-PC3)

The missing half of a pose is `pitch_deg`, `yaw_offset_deg` and `zoom`. Only the client knows them. Wave 3 (PC-15 to PC-19) reads the camera state once per outbound `interact` or `useItem` in the opt-in player DLL, and joins it to the server row. With it, idea 1 produces whole pose lines, camera included. It is also the only route to a camera model the bot can copy (idea 5, stage 3).

- Cost: M to L.
- Risk: medium to high.
  - Privacy and consent wording for players.
  - The camera actor pose reads zero on this build, though zoom and the offsets read.
  - It needs a live lab UAT.
- Dependency: core plus the D-PC3 decision. **PC-06 may shrink it:** if the wire pitch and yaw prove to be the view rotation, the server already has the aim direction, and the DLL only needs `zoom`.
- Recommendation: **decide after PC-06 returns.**

### 5. Playerbots: automated UAT with real input

The long-term goal is a lab driver that walks the map with W/A/S/D and turns with Q/E and mouse look. It aims the camera like a person and clicks what a person would click, with no `/gmgotoxyz` and no GM shortcuts, so N1 (native) grading is the only grading. Staged path:

| Stage | What | Built from | Cost |
|---|---|---|---|
| 0 | Pose library | idea 1 | M |
| 1 | Route corpus: human approach trails plus `nav_path` (lab-server MCP) as the fallback planner | idea 2b | M |
| 2 | Real-input locomotion: closed loop on the server position, Q/E turn at the measured rate (about 110°/s), stop inside the pose's radius band | `client_move_to`, `client_input_*` | M |
| 3 | Camera controller: reach the pose's pitch, yaw offset and zoom by mouse look, closed loop on the camera readout | idea 4, or PC-06 if view pitch is free | M |
| 4 | Imitation: pick the approach side and path humans pick most for this target, and grade against the human spread | stages 0 to 3 | M |

- Overall cost: L, over two or three campaigns.
- Risk: high.
  - Input fidelity: the movement physics and the camera clamp.
  - A CPU-starved lab makes the closed loops flaky.
  - One driver per lab lease.
  - The pay-off is gated on stages 0 and 1.
- Dependency: core, then ideas 1 and 2.
- Recommendation: **approve stages 0 and 1** (they are ideas 1 and 2). Write the stage 2 to 4 design as its own architect pass after the lab-roadmap campaigns close.

### 6. UAT coverage map

Group human `interaction.pose` rows by session into ordered `target_tag` sequences, then diff them against the targets the spec rows and goldens touch. The output is a ranked list of interactions humans perform that no automated spec covers. It is the backlog for new spec rows.

- Cost: M.
- Risk: low.
- Dependency: core.
- Recommendation: **approve after idea 1**; it reuses the same query plumbing.

### 7. Facing and turn calibration

`facing_error_deg` on human rows shows how far off-axis people stand when a click works. `movement.position_sample` direction deltas give the real Q/E turn rate. Both calibrate `client_move_to`'s face margin and the lab's turn timing.

- Cost: S.
- Risk: low.
- Dependency: core.
- Recommendation: **fold into idea 1** as one more column in the same tool.

### 8. Dialog and UI think time

The time from a dialog opening to the player's choice, and from a vendor window opening to a purchase, both measured server-side, gives the distributions the lab's wait budgets and D-LR7 timeouts should use instead of guesses.

- Cost: S (one paired timestamp on existing dialog-choice rows, plus a query).
- Risk: low.
- Dependency: none.
- Recommendation: **defer**; useful but not on the camera or body-placement problem.

### 9. Headless replay of human traces through the wireclient

Replay a human session's positions and interactions through `crates/wireclient` as a server regression or load test.

- Cost: M.
- Risk: medium. It exercises the server only; it cannot test client picking or the camera, which is where the lab fails.
- Dependency: idea 2b.
- Recommendation: **defer**.

### 10. Region dwell heatmaps

Where humans linger and where they never go. This partly exists as `region_dwell_no_hint`.

- Cost: S.
- Risk: low.
- Dependency: idea 2a.
- Recommendation: **defer**, or fold into idea 3.
