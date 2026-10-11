# Lab roadmap: handoff for campaign planning (2026-10-10)

**For:** a fresh coordinator session.

**Job:** turn each effort below into its own campaign ledger, with Haiku-sized work packets, following the `campaign-packet` skill. Open one docs PR per campaign (or one PR for all the ledgers) to `main`.

**Do not implement anything,** and do not use the lab. This is planning only.

The goal behind all of it: **inferenceless lab work**. A model writes or reviews a spec once, and every run after that is deterministic and needs no inference.

The wire-client CI campaign (`docs/analysis/wireclient-ci/`) was planned in the same session and is not part of this handoff.

## What exists today (read these first)

- **`lab uat`** (PR #1345): runs spec rows over the daemon's MCP endpoint, with `-Leases 1-5`, `-RunsPerLease 1-20` and `-Json`.
  - Code: `tools/lab/cli/uat.ps1` and `uat-lib.ps1`.
  - Tests: `tools/lab/cli/test-uat.ps1`.
  - Docs: `docs/guides/live-research-lab.md#commands`.
- **The spec format and runner:** `crates/lab/src/uat/` (`spec.rs` is the schema; `tools.rs` is the capability table), and the authoring guide `docs/guides/automated-uat.md`.
- **The calibrated example:** `docs/guides/uat-specs/first-session.toml`, rows FS-01 to FS-P5. Its header calibration table shows exactly what was tuned by hand (#1330).
  - Facts it records: the first-login AoI hold is 16 s; the camera pitch is +200 counts and the face then lands on -28.8°; the face is followed by a +230 yaw; stand-offs are to the side; the Guard body needs a point click; the tutorial must be closed with its X before the dialog.
- **CI:**
  - `.github/workflows/lab.yml` covers the lab crate, the UAT specs and the `tools/lab` PowerShell.
  - `build-lane-scripts.yml` covers the build-lane PowerShell tests.
  - `ship.py merge` waits for these via `CONDITIONAL` (#1346).
- **Known issues:**
  - **#1341:** the client drops the tail of the first-login AoI flush bundle (an iterator residue bug), so Frost vanishes. It shows up under two-client load.
  - **#1342:** a watchdog relaunch race can orphan an `SGW.exe`.
  - **#1343:** the login password box sometimes reads back empty.
- **Telemetry you can mine without inference:**
  - **Client:** the lab DLL logs `client.entity.*`, `client.mercury.*` (including `request_misparse` and `unpack_fault`), `client.engine.*` and UI events to SigNoz with `cimmeria.session_kind=lab`.
  - **Server:** `server_packet_tap_*` (decoded Mercury both ways), `server_db_query`, `server_log_tail`, `server_entity_query`/`get` (entities carry a `tag`, such as `ArmYourself_FrostBody`), and `server_witnesses`.
  - **Wire capture:** `crates/wireclient/tests/fixtures/praxis_start_tap.json`.
- **Lessons:**
  - `.claude/agent-memory/main-session/reference_first_session_lab_calibration_2026_10_10.md`.
  - The `.claude/agents/lab-driver.md` traps: never lend Haiku a lease; positive pitch counts look down; the `MoviePlayerWin` hide is not the end of the movie.

## Rules every campaign inherits

- **Packets:** each one is sized for the Haiku `packet-coder` (under ~100k tokens). It names exact files and functions, the test and why it fails if the change is reverted, the lane checks through `pwsh tools/build-lane/lane.ps1`, and its doc rows. Judgment-heavy packets go to `rust-gameserver-dev` or a domain advisor.
- **Review and merge:** the Sonnet `packet-reviewer` adversarially reviews every packet. The coordinator verifies, fixes, ships with `python tools/build-lane/ship.py pr` / `merge`, and retires worktrees.
- **Tooling:** PowerShell only, with no bash and no direct `cargo`. One worktree and one test DB per worker. Never `git worktree prune`.
- **The lab is shared:** ask the user before any lab use, and leave the lab alone in packets unless a packet is explicitly a live UAT packet the user approved.
- **Output budget:** tools must stay LLM-context friendly. That means compact `-Json` and summaries capped by default, never walls of text. The `lab uat -Json` contract is the model: no nulls, capped groups, a few hundred characters.

## The efforts, in suggested order

### 1. Spec vocabulary and self-calibration (do first: everything else builds on it)

**Problem:** today's spec steps are brittle. Points are hand-measured; the camera uses relative counts that depend on where the last row left it; waits are fixed sleeps (17 s); and unnamed entities are clicked by point.

**Packets to plan:**

- **Target by server tag:** `@world_click { tag = "ArmYourself_GuardBody" }`, plus `@entity_find { tag }` and `@camera { face_tag }`. Resolve the tag with `server_entity_query`, then the client entity by id. Spec validation must reject unknown tags against the seed (`db/resources`).
- **Absolute camera:** `@camera { pitch_deg, yaw_offset_deg, zoom }`, closed-loop on `view_after`. Keep relative counts working.
- **Server-event waits:** `wait_server = { log = "Cinematic AoI hold: released", timeout_ms }` or a typed event from lab-mcp. Replace the 17 s sleep in `first-session.toml`.
- **`lab calibrate <section> <row>`:**
  - For each interaction step, sweep stand-off points on a ring around the target (3 to 4.5 m, every 30°), pitch and yaw offsets.
  - Score each pose by *hover only*, with no click: `mouse_over` equals the target, and margin from the avatar and the HUD.
  - Write the winning pose into the spec, as a diff for review.
- **`lab uat --heal`:** on a hover-miss failure, run calibrate for that row and print the proposed spec diff (exit non-zero, never auto-commit).

**Acceptance:** recalibrate FS-P3 and FS-P4 from scratch, and calibrate the uncalibrated SGU rows FS-S3 to FS-S6 to 5 clean runs in a row (a live UAT packet that needs the user's OK).

### 2. Row fixtures and rewind ("make rows independent")

**Problem:** rows are chained. FS-P4 only works after FS-P3 has advanced mission 622 *on that character*, and resuming after a failure needs the fiddly rules in the spec header. One failure wastes every later row, and you can't run "FS-P4 twenty times".

**Idea:**

- **A row declares its starting state:**

  ```toml
  [row.fixture]
  character = "fresh-praxis"   # or reuse the run's
  world = "Castle_CellBlock"
  missions = [{ id = 622, step = 80623 }, { id = 1360, step = 4037 }]
  items = []                   # e.g. the pistol for FS-P5
  position = [-319.0, 73.6, -212.5]
  ```

  The runner reaches that state before the row's steps. Plan the mechanism as a Decision with a recommendation:
  - **GM and lab-mcp commands**, server-authoritative and preferred: `/gmgotoxyz`, a lab-only `.mission set <id> <step>` and `.giveitem`.
  - **Or seeded DB rows** through a lab-mcp write tool, which today is read-only, so that's an owner decision.
- **Then every row is idempotent:** it can run alone, in any order, in parallel, or 20 times. `lab uat -Rows FS-P4 -RunsPerLease 20` becomes meaningful.
- **`lab rewind` (later):** snapshot a character's `sgw_mission` and inventory rows at each row boundary, and restore a snapshot to retry one row without replaying the chain. That's the fixture mechanism, fed from a snapshot instead of the spec.

**Acceptance:** FS-P3, FS-P4 and FS-P5 each pass when run alone, five times each; FS-P2 is still the only row that needs a fresh first login.

### 3. Golden runs and run diffing

**Rule (owner):** no single run is golden. A run becomes golden only when **5 runs agree**, as deterministic evidence.

**Plan:**

- **Normalised run fingerprint.** Built from data the lab already captures:
  - the ordered client→server calls and server→client messages, from the packet tap, by name and with key arguments;
  - the mission, step and objective transitions;
  - the windows opened and closed;
  - each row's verdict.
  - Strip runtime noise: entity ids (map them to template or tag), timestamps, item instance ids and session ids.
- **Variance model.** Things legitimately vary between runs: NPC ability timing, combat ordering, ambient chatter, `avatarUpdate*` volume. Define the variance classes in spec metadata (for example `allow_unordered = ["onEffectResults"]`, or ignored message families), so they don't break the agreement.
- **`lab golden`:**
  - `lab golden record <section> -Rows ... -Runs 5` runs 5 times, and fails unless the 5 fingerprints agree under the variance model. When they do, it stores the golden fingerprint, its 5 evidence run ids and the build in a committed file (`docs/guides/uat-specs/golden/<section>.json`; size-check it).
  - `lab uat --diff-golden` compares each run to the golden, and reports the *first divergence* compactly: "row FS-P3: expected `onDialogDisplay 3995` after `interact`, got `onDialogDisplay 3996`".
- **Re-blessing:** a new golden needs 5 fresh agreeing runs, so there's no single-run overwrite.

**Acceptance:** golden for FS-01 to FS-P5 from 5 runs; a deliberately changed server answer (in a test, not live) produces a one-line divergence.

### 4. `lab record`: a human plays, out comes a spec

**Owner:** yes.

**Plan:**

- `lab record start <section> -Instance p2` arms recording on one lab client the human is driving. `lab record stop` writes `docs/guides/uat-specs/drafts/<section>-<time>.toml`.
- **Inputs, all existing telemetry:**
  - client UI and input events (clicks with the entity under the cursor, windows opened and closed, dialog button choices, chat lines typed, items moved);
  - the server packet tap (the client calls);
  - `server_db_query` snapshots of `sgw_mission` and `sgw_player` between actions;
  - the camera pose (`client_camera` `view_after`) and the player position at each interaction.
- **Output:**
  - one row per meaningful interaction, with steps in the effort-1 vocabulary (target by tag, absolute camera, server-event waits);
  - clauses derived from the server-state diffs ("622 step 2113 → 80623");
  - waits from the observed timing plus a margin.
- **No inference:** a deterministic transform. Unknown event types become commented `# TODO` lines; nothing is guessed.
- **Follow-up:** a recorded draft becomes a real spec only after `lab golden record` agrees 5 times (effort 3).

**`.bug` to spec (owner: only if it can be done without inference and without spam):**

- It is an **opt-in flag**, never automatic: `.bug spec` or a lab-only variant.
- It converts the last N minutes of that session's telemetry into one compact draft spec file through the same transform, and links it from the bug record.
- No chat output beyond one line; no board or issue posts.
- Cap the size: drop movement and noise, at most ~30 steps.
- If it can't be done deterministically, the campaign says so and stops at `lab record`.

**Defer:** synthesising specs from seed data (the content chains), until several long-form specs exist as examples.

### 5. Client limits and bounds, then chaos mode

**Owner:** understand the client's limits before deciding how much chaos to cause. And it's 2026, so work out which limits can be *raised*.

**Phase A, a research campaign** (`game-archaeology-specialist` plus lab measurement packets; mostly docs). Measure and document the bounds, each with its source (Ghidra address, ini key, measured value). Start from what's known:

- **Network:**
  - **Datagrams:** a reliable datagram above 1472 B wedged the client once. Fragmentation now exists (`project_mercury_tx_hole_size` in memory).
  - **Pacing:** the client sends about 6 packets/s at idle (`reference_client_idle_send_cadence`), and `NetInactivityTimeout=15` is client-side.
  - **Delivery:** the reliable window and resend behaviour.
  - **Bundles:** the max bundle and fragment counts, and the request-offset residue bug (#1341).
- **AoI and entities:** entity count and AoI radius limits, the create-burst size the client absorbs, and how the cinematic hold interacts with them.
- **Process:** 32-bit SGW.exe means a 2 GB address space unless Large Address Aware is set. Measure working-set peaks per map.
- **Engine:** frame time under load, the UE3 texture streaming pool, the background-window throttle (5 ms sleep per tick; virtual focus defeats it).
- **Timing:** the server tick, client interpolation, and teleport and AoI refresh latency.

**What could be raised** (each is a candidate packet with a risk note and a rollback):

- Set the **LAA flag** on SGW.exe (4 GB on 64-bit Windows). This is a client patch, so it needs a maintainer decision.
- Raise the **UE3 ini pools**: `TextureStreaming PoolSize`, `MaxSmoothedFrameRate`, the background-throttle settings.
- Widen **server-side pacing**: AoI introduction batch sizes, flush pacing, send rate caps.
- Increase **lab capacity**: more than 5 clients across two machines; a low-resolution render mode.

**Phase B, `lab uat --chaos <profile>`,** bounded by Phase A's numbers. Profiles:

- **`cpu`:** stress threads, keeping N cores busy.
- **`net`:** latency, jitter and loss, through a server-side transport shim (preferred, deterministic and seedable) or a Windows packet shaper.
- **`flush`:** server delays or reorders the AoI flush.
- **`burst`:** spawn K extra entities in the AoI.

Each profile has a documented "safe" level and a "break" level. Chaos runs report against the golden fingerprint (effort 3), so a chaos-induced divergence is one line.

**Acceptance:** the bounds table is committed, and the `cpu` and `flush` profiles reproduce #1341 on demand. That last point is the proof chaos mode finds real bugs.

### 6. `lab watch`

A live terminal dashboard for running batches. It's read-only and needs no inference:

- one panel per lab instance: client pid, lease (owner and purpose; never the id), current run and row, last progress line, and the age of the last screenshot;
- a batch panel tailing `uat-runs\batch-*\batch.jsonl`: pass/fail counts, the latest failure signature, known issues tagged;
- `--once` prints a compact snapshot (for agents) instead of the live view.

Data comes from the daemon's `/status`, the batch JSONL, and `lab logs`. There's no new daemon API unless the current row needs one (then add `current_row` to `/status`).

**Acceptance:** watching a `-Leases 2 -RunsPerLease 3` batch shows both lanes progressing; `--once -Json` is under 500 characters.

## Explaining "fixtures and rewind" (effort 2), the owner's question

Today a spec is a **chain**. FS-P3 assumes FS-P2 just ran on the same character, and FS-P4 assumes FS-P3 advanced mission 622 on it. That causes three problems:

1. **One failure ruins the rest.** If FS-P3 misses, FS-P4 and FS-P5 fail too, as noise.
2. **You can't hammer one row.** Testing "the Guard search" 20 times means replaying create, login, Frost, Guard 20 times.
3. **Resuming is fiddly.** The spec header needs special rules: re-run FS-01, FS-02 and FS-P2; never FS-P1; fix the camera first.

A **fixture** puts the row's starting state in the row itself: which character, world and position, which missions and steps, which items. The runner *establishes* that state before the steps run, using GM or lab commands. The state no longer depends on a previous row. Rows become independent units, like unit tests: run one alone, run them in any order, run one 20 times, run them in parallel lanes.

**Rewind** is the same mechanism fed from a snapshot. Save the character's server state at each row boundary, and "retry FS-P4" restores the snapshot taken before FS-P4 instead of replaying the chain.

It costs one lab-only server command (or lab-mcp write tool) to set mission state. That's an owner decision, because it's a write path, even though it's GM- and lab-gated.

## Deliverable from the fresh session

- One ledger per effort under `docs/analysis/<campaign>/`: `README.md` and `work-packets.md`, with Decisions, waves, and packet sizes for Haiku. Suggested slugs:
  - `lab-spec-vocab`
  - `lab-fixtures`
  - `lab-golden`
  - `lab-record`
  - `client-limits` (Phase A) and `lab-chaos` (Phase B)
  - `lab-watch`
- A cross-campaign order: effort 1 first; then 2, 3 and 6 in parallel; 4 after 1 and 3; 5A any time; 5B after 3 and 5A.
- A board subcategory per campaign (`board campaign create`), and a summary post in `handoffs`.
- A PR to `main` with the ledgers (docs-only, so it merges at once via `ship.py merge`).
