# Player capture for lab UAT: 18 October 2026

> Type: campaign ledger. Revised 2026-10-18 after the owner's decisions, and amended the same day after the owner's rulings on consent, releases, friction and the camera join. Packets: [work-packets.md](work-packets.md) (rules, contract, index) and one file per wave. Resume card: [RESUME.md](RESUME.md). Ideas and their status: [IDEAS.md](IDEAS.md). Reports: [revision](architect-revision-report.md), [amendment](architect-amend-report.md).

Automated lab UAT still fails where a human succeeds without thinking. It stands at the wrong distance, faces the wrong way, aims the camera at its own avatar, or clicks a target from where the client will not pick it. Lab record captures a human driving a **lab** client. Nothing captures what real players do on the colo server.

This campaign captures human play and turns it into lab inputs:

- **Server rows, no client patch.** `interaction.pose` is written on every accepted interact and every item used on another entity. `interaction.approach` follows it with the player's last 8 seconds of path. `movement.position_sample` gains `cimmeria.session_kind`.
- **Camera from the opt-in DLL.** `client.camera.at_interact` records zoom, the camera offsets and the pawn pose at each outbound `interact` or `useItem`. It is joined to the server row by the Mercury packet sequence of the call, else by server time, with no wire change (D-PC19, D-PC20).
- **Lab tools that read exported rows.**
  - `lab friction-drafts`: human friction becomes bug cards.
  - `lab pose-library`: human stand-off poses become pose lines.
  - `lab coverage`: lists interactions humans perform that no spec covers.
  - A wireclient replay of a human interaction: server-side only.
- **Friction goes to SigNoz only, never Discord** (owner, 2026-10-18). `playtest.friction` moves from WARN to DEBUG and keeps reaching SigNoz through its own `OTEL_FILTER` row (PC-11).

## Findings

| # | Finding | Evidence | Packet |
|---|---|---|---|
| F1 | The client reports position, velocity and three packed angle bytes at about 10 Hz in `avatarUpdateExplicit`. Nothing else about view or camera. | `docs/protocol/position-updates.md` | PC-01 |
| F2 | Inbound angles are unpacked since P49 (`direction = [pitch, yaw, roll]` radians, yaw = `atan2(dx, dz)`). The project memory note still says they are never unpacked. | `crates/cell-world/src/cell/space_manager/entities.rs:621-640`; `.claude/agent-memory/rust-gameserver-dev/cell-entity-direction-semantics.md` | PC-01, PC-05 |
| F3 | No interaction method carries camera, aim or cursor data. | entity defs; `docs/protocol/` | PC-02 |
| F4 | Nobody has checked whether the wire yaw and pitch are the pawn rotation or the view rotation. The answer sets the camera scope (S-CAM). | `ServerConnection::addMove` 0x00dd9330, packer 0x00de1720 | PC-06 |
| F5 | The camera state is in client memory only: `ASGWCamera_Player` +0x348 flags, +0x358 gain, +0x360 zoom (100..775), +0x368/+0x36C pitch/yaw offsets (65536 per turn). The pitch offset is stored unclamped. The camera actor pose can read zero on this build. | `crates/lab/src/supervisor/world/memory.rs:240-429` | PC-15 |
| F6 | The successful-interaction seams are on the cell: `handle_interact` after the range gate, and `handle_item_used`. | `.../interaction/interact.rs:108-240`; `.../base_messages/inventory_events.rs:120` | PC-03, PC-04 |
| F7 | `CellEntity.session_kind` (`lab` or `player`) exists. `movement.position_sample` does not carry it. | `crates/entity/src/cell_entity/entity_struct.rs:162-167`; `.../movement_telemetry/position_sample.rs:99-118` | PC-02, PC-10 |
| F8 | `tracing` caps an event at 32 fields, including `message`. The first contract had 33 fields plus the message and would not compile. The revised row has 29 fields plus the message. | `crates/admin-api/src/routes/telemetry/replay_native.rs:260-266` | PC-02 |
| F9 | **Friction reaches Discord.** Ten `warn!` sites feed `DiscordLayer`, which ignores anything below WARN. Colo sets `should_post(Warning)`, so friction posts to the errors channel. No lab or tool code reads friction rows. | `crates/server/src/logging/mod.rs:233-241`; `crates/discord/src/layer/mod.rs:92-96`; `docker/compose.discord.yml:87`; `crates/discord/src/router.rs:59` | PC-11 |
| F10 | `OTEL_FILTER` exports a target-named row only when it has its own entry, and `target_scan_tests.rs:344` enforces that. A friction row demoted to DEBUG needs `playtest.friction=debug`. `filters.rs` is 734 lines, so the entry goes on an existing line. | `crates/server/src/logging/filters.rs:314-409` | PC-11 |
| F11 | No `.bug` file format exists. `.bug` is a GM console bookmark (`playtest.bookmark`), and LR-15's `.bug spec` is a stub. Drafts under `docs/guides/uat-specs/drafts/` are guarded by `committed_drafts_parse_and_validate`. | `crates/lab/src/uat/mod.rs:185`; `docs/guides/uat-specs/drafts/README.md` | PC-14b |
| F12 | `cimmeria-lab` is a bin-only crate, so `cimmeria-lab-cli` cannot reuse its pose or spec code. The tools reimplement small pure pieces. | `crates/lab/Cargo.toml`; `crates/lab-cli/Cargo.toml` | PC-14a, PC-20 |
| F13 | **The opt-in covers the camera.** It gates the injection of a DLL that only reads memory, and the player DLL already ships actor locations and outbound method names. The owner ruled on 2026-10-18 that no disclosure change is needed ("telemetry is telemetry"). The DLL hash is compiled into the launcher, so a DLL change reaches players only through a launcher release. | `docs/architecture/client-telemetry.md:19-35`; `crates/client-telemetry/src/hooks/seams/actors.rs:121-122` | PC-16, PC-07b |
| F14 | Lab record derives a pose's `yaw_offset_deg` from the camera actor's position and yaw, not from the raw yaw offset. When the camera pose reads zero, the human `yaw_offset_deg` cannot be derived. | `crates/lab/src/uat/record/pose.rs:63-85` | PC-21b |
| F15 | The wireclient can `enter_world`, move (`avatar_update_explicit`) and `interact`. `praxis_start_live_db` already drives the Frost interact. Replay can therefore test the server path, but never client picking or the camera. | `crates/wireclient/src/calls.rs:48`; `tests/it/praxis_start_live_db/` | PC-23, PC-24 |
| F16 | Four files the packets touch are over the hard cap. `filters.rs` (734), `client-telemetry.md` (733) and `operations/telemetry.md` (799) take net-zero-line edits. Packets avoid `interaction/mod.rs` (871). | line counts on 2026-10-18 | all |
| F17 | **The server already has a per-call join key.** The cell router receives each client cell method call with its Mercury `packet_seq`, and logs it for abilities. Nothing records it for `interact` or `useItem`. | `crates/cell/src/cell/dispatch/router.rs:38-61` | PC-26 |
| F18 | **The DLL can report the sequence of its own sends.** The ability trace (AB) tags a routed call, and the `Channel::send` / `Nub::send` hooks report the packet sequence range as `client.ability.sent_seq`. The 28-bit wrap rule is `seq_in_range`. These hooks are not yet proven live. | `crates/client-telemetry/src/hooks/ability_trace/seq_join.rs:29-40`; `.../inline_hooks/ability/route.rs` | PC-16b, PC-19 |
| F19 | **Client rows reach the server clock only roughly.** admin-api's `SessionClock` (NT-40) takes the offset from the chunk's newest row, so its error is up to one flush interval plus network delay. No row stores the server time. | `crates/admin-api/src/routes/telemetry/entity_labels.rs`; `chunk.rs` | PC-27, PC-17 |

## Decisions

| ID | Status | Decision | Reason |
|---|---|---|---|
| D-PC1 | **Owner: yes** | Lab sessions also write the rows, tagged `session_kind = lab`. Human queries filter `player`. | The comparison set: lab and human stand-offs for one target in one query. |
| D-PC2 | **Owner: yes** | The rows carry `account_id`, `player_id`, `entity_id`, the template id and the world id. Ids only for people: no account names, character names or IPs. The unpaired-id scan stays green through `// nt:id-only` markers. | Owner privacy rule for real-player capture. |
| D-PC3 | **Owner: resolved** | The existing opt-in covers in-game camera capture (F13). Wave 3 is real. PC-06 may cut the scope (S-CAM). | Owner, 2026-10-18; the evidence is in F13. |
| D-PC4 | Accepted | An item-use row is written only when `target_id` names a live entity other than the user. | A pose needs a target. |
| D-PC5 | Accepted | The rows are INFO, with a 250 ms throttle per actor. | The leading `info` in `OTEL_FILTER` exports them unedited. |
| D-PC6 | Accepted | `player_yaw_deg` and `wire_pitch_deg` are never renamed; PC-06 documents their meaning. | A rename after deploy splits the data. |
| D-PC7 | Accepted | No new metric. | Log counts are enough. |
| D-PC8 | **Owner: resolved** | Real players on colo are the proof. PC-07 is a SigNoz check after a colo release carrying wave 2. Releases are pre-authorized (D-PC22); the orchestrator still asks the user before lab use and colo data export. | Owner, 2026-10-18. |
| D-PC9 | Accepted (architect) | Row v2: drop `target_x/y/z`, `dist_3d_m`, `yaw_to_target_deg` and `target_entity_name`. Add `world_id` and `pose_seq`. The target position is derived from the geometry (contract). | F8. Each dropped field is derivable. `pose_seq` joins the approach row. |
| D-PC10 | Accepted (architect; owner may veto) | D-PC2 bans personal names only. Content names (template, dialog, item, world) stay, so a query reads "Frost", not 700. | The owner listed ids for people, and content names identify nobody. |
| D-PC11 | Accepted (architect) | Friction is demoted at the call sites (WARN to DEBUG) and gets an `OTEL_FILTER` row. The alternative, a Discord-layer skip by target, was rejected. | Friction is not an error. DEBUG keeps it in SigNoz, and the target scan guards it. It leaves `server.log`, which is INFO and up; nothing reads it there. |
| D-PC12 | Accepted (architect; owner may veto) | "`.bug` files" means one markdown bug card per friction cluster, written to an `--out` folder. Each card holds the facts and a commented, UNCONFIRMED TOML row skeleton. Nothing is written into `uat-specs/drafts/`. | F11. A card is a proposal. Committed drafts must validate. |
| D-PC13 | Accepted (architect) | S-CAM rule. If PC-06 proves the wire carries the **pawn** rotation (A), PC-15/16 ship as written. If it carries the **view** rotation (B), drop the pawn read and its fields; zoom, offsets and the camera pose stay. If inconclusive, A. | The scope follows the evidence (D-PC3). |
| D-PC14 | Accepted (architect) | The camera row is replayed in its own admin-api shape: no `entity_name`, and numeric fields lifted. | The game shape names `entity_id` with a character name, which D-PC2 forbids. |
| D-PC15 | Accepted (architect) | The tools read flat JSONL exported from ClickHouse. Keys are the tracing field names plus `timestamp`. No tool queries SigNoz, and no host is committed. | Offline, testable, and no secrets. |
| D-PC16 | Accepted (architect) | Replay (idea 9) is a live-DB wireclient test of one human interaction. It is labelled server-only; load mode is a later follow-up. | F15. Honest scope. |
| D-PC17 | **Withdrawn** (owner, 2026-10-18) | No launcher disclosure change; the opt-in covers the camera ("telemetry is telemetry"). | Owner ruling. |
| D-PC18 | Accepted (architect) | Each DLL upload carries `X-Cimmeria-Client-Sent-Ms` (PC-16). admin-api measures the session's clock offset from it and writes `server_ts_ms` and `clock_offset_ms` on camera rows (PC-27, PC-17). A stamp older than the chunk's newest row is ignored. | F19. Cuts the clock error to the upload's network delay, with no wire change; the header is DLL-to-admin-api HTTP. |
| D-PC19 | Accepted (architect) | The deterministic join key is the Mercury packet sequence: `mercury_seq` on the pose row (PC-26) against the `client.camera.sent_seq` range (PC-16b), linked to the camera row by `send_id`. No correlation id goes on the wire. | F17, F18. The sequence already exists on both sides; a new id would need a client wire patch. |
| D-PC20 | Accepted (architect) | Join order: sequence (with a 60 s guard against reuse), then server time within 500 ms, then raw client time within `--join-window-ms` (default 1 s) for rows without `server_ts_ms`. Every join records its method. | Each tier is exact or bounded; PC-07b measures the real shares and error. |
| D-PC21 | Accepted (architect) | Wave 3 splits: 3a is code (PC-15, PC-16, PC-16b, PC-26, PC-27, PC-17), 3b is docs (PC-05, PC-18) after 3a merges. | Docs describe merged code; 3a is six packets, one review context. |
| D-PC22 | **Owner: yes** (2026-10-18) | Releases are pre-authorized. Ops posts `/release` (colo) and `/release-launcher` (launcher) once the PRs they carry are merged, from PowerShell, and verifies the runs ([procedure](work-packets-w3b-docs-gates.md#release-procedure-pre-authorized-d-pc22)). | Owner ruling. Lab use and colo export still ask first. |

**Still open for the owner:** nothing blocks a packet; no BlockedDecision. D-PC10 and D-PC12 stand unless the owner vetoes them. Ideas 8 and 10 are deferred, and idea 5 stages 2-4 wait for a later architect pass ([IDEAS.md](IDEAS.md)).

## Packet ledger

Status vocabulary: Ready, BlockedDependency, BlockedDecision, Writing, Review, Integrated, UATPending, Done.

| Wave | Packet | Agent | Status | PR | Live proof |
|---|---|---|---|---|---|
| 1 | PC-01 pose geometry | packet-coder | Writing (worktree `pc-01`) | | none |
| 1 | PC-02 capture, throttle, emit (stacked on PC-01) | packet-coder | BlockedDependency: PC-01 commit | | none |
| 1 | PC-06 RE: wire rotation source and S-CAM verdict | game-archaeology-specialist | Writing (worktree `pc-06`) | | Ghidra only |
| 1 | PC-10 `session_kind` on `movement.position_sample` | packet-coder | Writing (worktree `pc-10`) | | PC-07 |
| 1 | PC-11 demote `playtest.friction` to DEBUG | packet-coder | Writing (worktree `pc-11`) | | PC-07 (DEBUG in SigNoz, no Discord post) |
| 2 | PC-03 interact call site | packet-coder | BlockedDependency: wave 1 merged | | PC-07 |
| 2 | PC-04 item-used call site | packet-coder | BlockedDependency: wave 1 merged | | PC-07 |
| 2 | PC-12 approach ring | packet-coder | BlockedDependency: wave 1 merged | | none |
| 2 | PC-13 `interaction.approach` row (stacked on PC-12) | packet-coder | BlockedDependency: PC-12 commit | | PC-07 |
| 2 | PC-14a JSONL rows and friction clustering | packet-coder | Ready (no dependency; runs in wave 2) | | none |
| 2 | PC-14b `lab friction-drafts` (stacked on PC-14a) | packet-coder | BlockedDependency: PC-14a commit | | PC-25 |
| 3a | PC-15 DLL camera reader | packet-coder | BlockedDependency: wave 2 merged, PC-06 verdict | | PC-19 |
| 3a | PC-16 camera at interact, governor keep, upload send stamp (stacked on PC-15) | packet-coder | BlockedDependency: PC-15 commit | | PC-19 |
| 3a | PC-16b camera `send_id` and `client.camera.sent_seq` (stacked on PC-16) | packet-coder | BlockedDependency: PC-16 commit | | PC-19 (sequence join) |
| 3a | PC-26 `mercury_seq` on `interaction.pose` | packet-coder | BlockedDependency: wave 2 merged | | PC-19, PC-07b |
| 3a | PC-27 admin-api session clock from the send stamp | packet-coder | BlockedDependency: wave 2 merged | | PC-07b |
| 3a | PC-17 admin-api camera shape with `server_ts_ms` (stacked on PC-27) | packet-coder | BlockedDependency: PC-27 commit, PC-06 verdict | | PC-07b |
| 3b | PC-05 docs, SigNoz views and export, stale facts | documentation-writer | BlockedDependency: wave 3a merged | | |
| 3b | PC-18 camera docs and the join recipe | documentation-writer | BlockedDependency: wave 3a merged | | |
| UAT | PC-07 SigNoz check of human rows | orchestrator plus a read-only agent | BlockedDependency: wave 2 merged, then ops `/release` (pre-authorized) | | rows with `session_kind = player`; friction DEBUG, no Discord |
| UAT | PC-19 lab UAT of the camera rows and the sequence join | lab-driver (Haiku) | BlockedDependency: wave 3a merged; **ask the user** before lab use | | `camera.at_interact` and `camera.sent_seq` in the event ring; pose `mercury_seq` in range |
| UAT | PC-07b SigNoz join check, camera to pose | orchestrator plus a read-only agent | BlockedDependency: PC-19, then ops `/release-launcher` and `/release` (pre-authorized) | | join shares per method |
| 4a | PC-20 human stats | packet-coder | BlockedDependency: wave 2 merged, PC-07 confirms the row shape | | none |
| 4a | PC-22 `lab coverage` | packet-coder | BlockedDependency: as PC-20 | | PC-25 |
| 4a | PC-23 wireclient replay case loader | packet-coder | BlockedDependency: as PC-20 | | none |
| 4b | PC-21 `lab pose-library` | packet-coder | BlockedDependency: PC-20 and PC-22 merged | | PC-25 |
| 4b | PC-21b camera-to-pose join, three tiers (stacked on PC-21) | packet-coder | BlockedDependency: PC-21 commit, PC-07b | | PC-25 |
| 4b | PC-21c camera keys and `--camera` in `pose-library` (stacked on PC-21b) | packet-coder | BlockedDependency: PC-21b commit | | PC-25 |
| 4b | PC-24 live-DB replay test | rust-gameserver-dev | BlockedDependency: PC-23 merged | | live-DB lane |
| UAT | PC-25 first data runs of the tools | orchestrator plus a read-only agent | BlockedDependency: wave 4 merged, about a week of colo data; **ask the user** before the export and lab calibrate | | worknotes |
| close | PC-09 close-out | documentation-writer | BlockedDependency: last wave | | |

## What this campaign does not do

- No real-input locomotion, camera controller or imitation bot (idea 5 stages 2-4: a later architect pass).
- No dialog think-time (idea 8) and no dwell heatmaps (idea 10): deferred.
- No change to lab record, the lab MCP or the wire. No new opcode.
- No account names, character names or addresses on any new row.
