# Player capture work packets: wave 3b (docs) and the live gates

> Type: work packets. Rules, contract and index: [work-packets.md](work-packets.md). Ledger: [README.md](README.md). Wave 3a: [work-packets-w3-camera.md](work-packets-w3-camera.md) and [work-packets-w3-join.md](work-packets-w3-join.md).

Wave 3b has two documentation-writer packets, PC-05 and PC-18. They start from `origin/main` **after wave 3a merges**, so they document what shipped. They run in parallel and ship as two PRs. The orchestrator copies PC-06's S-CAM line into PC-18's brief.

The live gates PC-07, PC-19 and PC-07b follow. **Releases are pre-authorized** (owner, 2026-10-18): ops cuts them once the PRs they carry are merged to `main`. **Lab use (PC-19) and colo data export (PC-25) still need the user's yes first.**

## Rules for both docs packets

1. **PowerShell only**, never bash, with absolute paths. Start every shell call with `Set-Location <your worktree>;`.
2. **Docs are CRLF.** After writing, check `[IO.File]::ReadAllText(<abs path>).Contains("`r`n")` on every touched file.
3. **File size.** Soft cap 500 lines, hard cap 700. `docs/architecture/client-telemetry.md` (733) and `docs/operations/telemetry.md` (799) take **net-zero-line** edits: fold text into existing lines. `docs/architecture/instrumentation-discipline.md` (697) takes no added lines.
4. **No IPs, hostnames, account names or character names.** Never copy a host from a recipe.
5. **Never hand-edit a `<!-- gen:NAME -->` block.**
6. **Walk `docs/agents/doc-update-map.md`** and list the rows you touched in the report.
7. **Git limits.** Never use `git stash`, `git worktree prune`, `--no-verify`, push or force. Commit once, locally, with the given subject and your brief's attribution lines.
8. **Scope:** edit only the named files. If an assumption is false, return `blocked` with the reason.

---

## PC-05 Docs, SigNoz recipes and stale-fact fixes

- **Agent:** documentation-writer.
- **Worktree:** `pc-05`. **Branch:** `player-capture-bots/pc-05-docs`.
- **Commit subject:** `docs(observability): interaction.pose and approach targets, export recipes, stale movement facts`.
- **Inputs:** [work-packets.md § Contract](work-packets.md#contract), including addendum A1 (`mercury_seq`), and PC-06's finding doc, `docs/reverse-engineering/findings/avatar-update-rotation-source.md`.

**Files.**

1. `docs/architecture/observability-target-catalog.md` (658 lines; add at most 40). Add `interaction.pose` and `interaction.approach` rows to the stable target table. Under `## Target details`, add one entry for each:
   - the level (INFO);
   - when the row is written: an accepted interact or item use on an entity, with the 250 ms throttle; the approach row only follows an admitted pose row that has a trail;
   - the field list and its order, from the contract, ending in `pose_seq` and `mercury_seq` (A1);
   - the privacy rule (D-PC2), and that content names are kept (D-PC10);
   - the conventions: bearing (D-SV3), server yaw, and the derived target position (D-PC9);
   - PC-06's meaning of `player_yaw_deg` and `wire_pitch_deg`;
   - the joins: `pose_seq` to the approach row; `mercury_seq` to the client camera (PC-18's doc);
   - "filter `cimmeria.session_kind = 'player'` for humans".

   Add one clause to the existing `movement.position_sample` entry: it now carries `cimmeria.session_kind` (PC-10).
2. `docs/operations/signoz/interaction-pose-views.md` (new; model it on `npc-ai-views.md`). Sections:
   - **Human poses per target:** `interaction.pose` rows with `cimmeria.session_kind = 'player'`, grouped by `world` and `target_tag`. Show the median and p10/p90 of `radius_m`, `dy_m` and `facing_error_deg`. Bearing is circular, so say to use the circular mean (PC-20 does it) rather than a median across 0/360.
   - **Lab vs human for one target:** group by `cimmeria.session_kind`.
   - **Approach trails:** join `interaction.approach` to `interaction.pose` on `pose_seq`.
   - **From a pose to a spec line:** the Frost worked example, mapped onto the `pose = [{ id, tag, radius_m, bearing_deg, dy_m, ... }]` line in `docs/guides/automated-uat.md`. Point to `lab pose-library` (wave 4).
   - **Export for the lab tools (D-PC15):** a ClickHouse `FORMAT JSONEachRow` query per input, flattening attributes to their tracing field names plus `timestamp`. There are four inputs:
     1. pose rows;
     2. approach rows;
     3. `playtest.friction` rows (for `lab friction-drafts`);
     4. `client.native` rows with `client_target LIKE 'client.camera.%'` (both `at_interact` and `sent_seq`, which `lab pose-library --camera` needs for the sequence join).

     Take the table and column names from the existing ClickHouse recipe in `docs/operations/signoz-deployment.md`, and point to it for access. Copy no host.
   - **Friction is DEBUG now (D-PC11):** a saved view or alert that filters `playtest.friction` on WARN severity shows nothing. Say that the owner updates the live "Playtest: friction" saved view.
3. `docs/engine/navmesh-build-pipeline.md`, lines 608-609: replace the stale sentence. `movement.position_sample` is exported at DEBUG through its own `OTEL_FILTER` row (`crates/server/src/logging/filters.rs`) and carries `cimmeria.session_kind`, so the accepted set is available and lab walks can be excluded. Keep the same line count.
4. `.claude/agent-memory/rust-gameserver-dev/cell-entity-direction-semantics.md`. Verify both changes against `crates/cell-world/src/cell/space_manager/entities.rs` before writing.
   - Rewrite the "Inbound client direction is never unpacked" bullet: since P49, `update_entity_position` unpacks the `(yaw, pitch, roll)` bytes into `[pitch, yaw, roll]` radians.
   - Rewrite the "`update_entity_position` zeroes facing" section: teleports use `update_position_preserving_facing`.
5. `docs/architecture/instrumentation-discipline.md` (697 lines): **add no lines.** If the sampled-telemetry section should name `interaction.pose`, fold one clause into an existing sentence.

**Checks:**
- `git diff --check`.
- CRLF on every touched `docs/` file.
- The line count of each capped file: `(Get-Content <abs path>).Count`.
- `python tools/docs-gen/regen.py --check`, if that flag exists. Otherwise skip it and say so in the report.

---

## PC-18 Camera docs and the join recipe

- **Agent:** documentation-writer.
- **Worktree:** `pc-18`. **Branch:** `player-capture-bots/pc-18-camera-docs`.
- **Brief input:** the S-CAM line.
- **Commit subject:** `docs(telemetry): client camera at interaction, its consent scope and the pose join`.
- **Inputs:** the merged code of PC-15, PC-16, PC-16b, PC-26, PC-27 and PC-17, and the contract in [work-packets.md](work-packets.md) (the camera section and addendum A1).

**Files.**

1. `docs/architecture/client-camera-telemetry.md` (new, under 250 lines), with these sections:
   - **What is read, and when:**
     - only on an outbound `interact` or `useItem`, before the router sends it;
     - the camera block, the camera actor pose and, under S-CAM A, the pawn pose;
     - the source of each field (PC-15's layout table, with the lab findings cited).
   - **Scope and consent:**
     - it rides the existing opt-in "Send game diagnostics while playing" (F13, D-PC3). The owner ruled on 2026-10-18 that this opt-in covers the camera, so the launcher wording is unchanged;
     - ids only (D-PC2, D-PC14);
     - it reaches player data only after a launcher release, because the DLL hash is compiled into the launcher.
   - **The rows**, as replayed by admin-api under target `client.native`: `client_target = 'client.camera.at_interact'` (the camera, with `send_id`) and `client_target = 'client.camera.sent_seq'` (the Mercury sequence range of that send), with their fields, and `server_ts_ms` / `clock_offset_ms` (PC-27, PC-17).
   - **Joining it to `interaction.pose`,** in this order (D-PC20):
     1. **Sequence:** camera row to `sent_seq` row on `session_id` and `send_id`; then the pose row with the same `entity_id`, a matching kind (`interact` to `interact`, `useItem` to `use_item`), and `mercury_seq` inside `mercury_seq_first..=mercury_seq_last` under the 28-bit wrap rule (`seq_in_range` in `crates/client-telemetry/src/hooks/ability_trace/seq_join.rs`). Guard against sequence reuse across logins by requiring the two times within 60 s.
     2. **Server time:** the nearest pose row with the same `entity_id` and kind, with its `timestamp` within 500 ms of the camera row's `server_ts_ms`.
     3. **Fallback,** for rows from before PC-27 (no `server_ts_ms`): the client `ts_ms` within 1 s.
     - Say what bounds each: the sequence is exact for `interact` and best effort for `useItem` (PC-26); the server-time error is the upload's network delay, because the DLL stamps each upload (PC-16, PC-27).
     - A ClickHouse sketch of the sequence join, in the shape of PC-05's recipes.
     - The mapping to the lab pose keys: `pitch_deg` comes from `pitch_offset_deg`; `zoom` from `zoom`; `yaw_offset_deg` from `atan2(tx - cx, tz - cz)` in degrees minus `camera_yaw_deg`, which needs the camera pose (absent when it is zero) and the derived target position (D-PC9).
   - **What the S-CAM verdict means:** PC-06's finding, linked.
2. `docs/architecture/client-telemetry.md`: fold one link to the new doc into an existing sentence about what the DLL records, and name the `X-Cimmeria-Client-Sent-Ms` upload header in the sentence that describes the upload. Net zero lines.
3. `docs/operations/telemetry.md`: add one clause to the existing opt-in description naming the camera at interaction. Net zero lines.

**Checks:**
- `git diff --check`.
- CRLF on every touched file.
- The line counts of the two capped files are unchanged: `(Get-Content <abs path>).Count`.

---

## Live gates

Workers never touch the lab, colo or SigNoz on their own. Every gate writes `docs/analysis/player-capture-bots/worknotes/<gate>.md` (CRLF) with no hosts, IPs or names.

### Release procedure (pre-authorized, D-PC22)

The owner pre-authorized the releases this campaign needs (2026-10-18). Ops cuts each one as soon as the PRs it carries are merged to `main`; the orchestrator does not ask the user.

- **Colo** (`docs/operations/colo-deploy.md` § releases): a `/release` comment on a merged PR. `release-on-comment.yml` checks it and dispatches `release-container.yml`, which builds `main` HEAD, so one release carries everything merged so far. Merging alone never restarts the colo. The colo database is rebuilt from the seeds on every start, and players are disconnected by the restart.
- **Launcher** (`docs/client/launcher-distribution-setup.md` § release, and `crates/launcher/desktop/docs/release.md`): a `/release-launcher` comment on a merged PR runs `launcher-release.yml`, which tags `launcher-YYYYMMDD-<sha7>` and moves `launcher-current`. The new DLL ships inside it, because the launcher pins the DLL hash.
- **Post from PowerShell only:** `gh pr comment <PR> --body "/release"` (or `"/release-launcher"`). Git Bash rewrites `/release` into a Windows path (`docs/agents/rules-and-gotchas.md`, the Git Bash rule).
- **Verify:** `gh pr view <PR> --json comments --jq '.comments[-1].body'` prints exactly `/release` (or `/release-launcher`). Then `gh run list --workflow release-on-comment.yml --limit 3` shows a run for that comment, and `gh run list --workflow release-container.yml --limit 3` (or `launcher-release.yml`) shows the dispatched run. Report the run URLs and their conclusions.
- **Who posts:** the `ops` agent. Its allowlist may not include `gh pr comment`; if ops refuses, give a general-purpose agent the exact commands above.

### PC-07 Human rows in SigNoz (D-PC8)

- **When:** wave 2 is merged. Ops posts `/release` on the last merged wave-2 PR and verifies the run (procedure above). Then players play.
- **Agent:** a read-only general-purpose agent. It uses the SigNoz MCP, or the ClickHouse-over-ssh recipe in `.claude/agent-memory/` if the MCP is down. No lab and no client.
- **Report**, for the last 24 h:
  1. `interaction.pose` row count by `cimmeria.session_kind`, `kind` and `route`. Include five sample rows with ids only.
  2. `interaction.approach`: the count, and the share of pose rows with a matching `pose_seq`.
  3. The median `radius_m` and the circular mean `bearing_deg` for the three most-interacted `target_tag` values.
  4. Any row with `radius_m > 5.0`. The server range is 5.0 m in 3D, so such a row flags a geometry bug.
  5. Confirm that no pose or approach row has `account_name`, `player_name` or `entity_name`.
  6. `movement.position_sample` rows carry `cimmeria.session_kind`.
  7. `playtest.friction` rows arrive at DEBUG in SigNoz, and none reached Discord (D-PC11; friction goes to SigNoz only). Check whether the "Playtest: friction" saved view filters on severity, but do not edit it; report it for the owner.
- **Done** when human rows exist and checks 5 to 7 pass. Any field drift goes back to the architect before wave 4.

### PC-19 Lab UAT of the camera rows

- **When:** wave 3a is merged, **and the user has approved lab use.** Ask first.
- **Agent:** lab-driver (Haiku), with an exact step list from the orchestrator. The lab must run the wave 3a DLL, installed with `lab install` from a merged main checkout, and a lab server built from the same `main` (PC-26).
- **Steps:**
  1. Acquire a lease and ensure the client is in the world.
  2. Stand 4-6 m from an interactable NPC and clear any dialogs.
  3. Read `client_camera`.
  4. Interact by world click.
  5. Read the event ring (`client_events_read`) for kinds `camera.at_interact` and `camera.sent_seq`.
  6. Release the lease.
- **Pass when:**
  - one `camera.at_interact` event follows the click, with a `send_id`;
  - its `zoom` is within 1 of the `client_camera` readout, and its `yaw_offset_deg` within 0.5°;
  - `camera_pose_zero` is reported honestly;
  - under S-CAM A, the `pawn_*` fields are present;
  - the server log shows the matching `interaction.pose` row with a `mercury_seq`;
  - **sequence join (proves PC-16b live):** a `camera.sent_seq` event with the same `send_id`, and the pose row's `mercury_seq` inside its `mercury_seq_first..=mercury_seq_last`.
- Grade against the server log, not the driver's claim. If the camera row passes but the sequence join fails, record it: the `Channel::send` / `Nub::send` hooks are not working live, the join falls back to server time, and the fix goes back to the architect.

### PC-07b Camera joined to pose on colo

- **When:** PC-19 has passed. Ops posts `/release-launcher` on the merged DLL PR (PC-15/16/16b) and `/release` on the last merged wave-3a PR (PC-26 and PC-27/17 on colo), and verifies both. Players with diagnostics on have played.
- **Agent:** a read-only general-purpose agent, as for PC-07.
- **Report:**
  - the count of `client.native` rows with `client_target = 'client.camera.at_interact'` by `cimmeria.session_kind`, and of `client.camera.sent_seq`;
  - the share of camera rows joined to an `interaction.pose` row by each method (sequence, server time, fallback), in the D-PC20 order;
  - for rows that join by sequence, the spread of `pose.timestamp - server_ts_ms` (this measures the clock join's real error);
  - the share with `camera_pose_zero = true`;
  - that no camera row carries `entity_name`.
- **Done** when joined human rows exist. Wave 4b's PC-21b uses them.
