# player-capture-bots: architect revision report

> Type: design report. Written 2026-10-18 after the owner's decisions; it supersedes [architect-report.md](architect-report.md). Ledger: [README.md](README.md). Packets: [work-packets.md](work-packets.md). Resume card: [RESUME.md](RESUME.md). Ideas: [IDEAS.md](IDEAS.md).

## Owner rulings applied

| Ruling | Effect on the campaign |
|---|---|
| D-PC1: lab sessions write rows too | Rows carry `cimmeria.session_kind`. Human queries filter `player`. |
| D-PC2: ids only for people | No account, character or entity names on new rows. Content names stay (D-PC10). The camera row gets its own admin-api shape (D-PC14). |
| D-PC3: the opt-in covers the camera | Wave 3 is real: PC-15 to PC-19 and PC-07b. The evidence is F13. |
| D-PC8: real colo players are the proof | PC-07 is a SigNoz check after a colo release. The orchestrator asks the user before any release. |
| Ideas approved | 2a goes to PC-10. 3 goes to PC-11 and PC-14a/b. 1, with 7 folded in, goes to PC-20, PC-21 and PC-21b. 2b goes to PC-12 and PC-13. 6 goes to PC-22. 9 (server-only) goes to PC-23 and PC-24. |
| Ideas deferred | 8, 10, and idea 5 stages 2-4 (a later architect pass). |

## Evidence behind the two new findings

- **F13, the opt-in covers the camera.** "Send game diagnostics while playing" gates the injection of the player DLL, which only reads memory (`docs/architecture/client-telemetry.md:19-35`). That DLL already ships actor locations (`crates/client-telemetry/src/hooks/seams/actors.rs:121-122`) and outbound method names. So the camera block and pawn pose are the same kind of data under the same consent.

  The disclosure sentence (`crates/launcher-core/src/client_changes.rs:287-288`, `docs/client/launcher-guide.md:279`) does not name the camera or the character's position. PC-16 adds them (D-PC17). Player data then needs a launcher release, because the DLL hash is compiled into the launcher. That is a user gate.
- **F9, friction reaches Discord.**
  - Ten `warn!` friction sites feed `DiscordLayer`, which ignores anything below WARN (`crates/discord/src/layer/mod.rs:92-96`).
  - Colo sets `should_post(Warning)` (`docker/compose.discord.yml:87`, `crates/discord/src/router.rs:59`), so every friction row posts to the errors channel.
  - No lab or tool code reads friction rows.
  - PC-11 demotes the sites to DEBUG and adds `playtest.friction=debug` to `OTEL_FILTER`, on an existing line because `filters.rs` is 734 lines. SigNoz keeps the rows, and the target scan (`target_scan_tests.rs:344`) guards the filter entry.

## Architect decisions (D-PC9 to D-PC17)

| ID | Decision |
|---|---|
| D-PC9 | Pose row v2 has 29 fields plus the message, under `tracing`'s 32-field cap (F8). It drops the target position, `dist_3d_m`, `yaw_to_target_deg` and `target_entity_name`, and adds `world_id` and `pose_seq`. The target position is derived from radius, bearing and dy. |
| D-PC10 | Content names (template, dialog, item, world) stay. Owner may veto. |
| D-PC11 | Friction is demoted at the call sites, not filtered in the Discord layer. |
| D-PC12 | "`.bug` files" means markdown bug cards per friction cluster, with a commented, unconfirmed TOML skeleton, written to `--out`. Nothing goes into `uat-specs/drafts/`. Owner may veto. |
| D-PC13 | The S-CAM rule. If PC-06 says the wire carries the pawn rotation (A), the DLL reads the pawn. If it carries the view rotation (B), the pawn fields are dropped. If inconclusive, A. |
| D-PC14 | The camera row is replayed in its own id-only admin-api shape, with numbers lifted. |
| D-PC15 | Tools read flat JSONL exported from ClickHouse. No tool queries SigNoz, and no host is committed. |
| D-PC16 | Replay is one live-DB wireclient test, labelled server-only. Load mode is later. |
| D-PC17 | The launcher disclosure gains the camera and position clause, which takes a user-approved launcher release. |

**No BlockedDecision remains.** D-PC10 and D-PC12 stand unless the owner vetoes them.

## Corrections made while revising

| Correction | Packet |
|---|---|
| Inbound direction has been unpacked since P49. The project memory note says otherwise and is fixed. | PC-05 |
| `navmesh-build-pipeline.md:608-609` says `movement.position_sample` is not exported. It is, at DEBUG, through its own filter row. | PC-05 |
| Lab record derives `yaw_offset_deg` from the camera actor's position and yaw, negated unless the gain is below 0 (`crates/lab/src/uat/record/pose.rs:63-85`). PC-21b copies that rule. When the camera pose reads zero, no yaw offset is produced. | PC-21b |
| `cimmeria-lab` is bin-only, so lab-cli reimplements the small pure pieces (F12). | PC-14a, PC-20, PC-21b |
| The DLL crate must be checked with `--target i686-pc-windows-msvc`, with and without `lab-bridge`. A host build is a false green. | PC-15, PC-16 |

## File-cap notes

These files are over the 700-line hard cap. The packets edit them with net-zero lines or at most one line, or avoid them:

| File | Lines | Edited by | Edit |
|---|---|---|---|
| `crates/server/src/logging/filters.rs` | 734 | PC-11 | same line |
| `docs/architecture/client-telemetry.md` | 733 | PC-18 | net zero |
| `docs/operations/telemetry.md` | 799 | PC-18 | net zero |
| `docs/client/launcher-guide.md` | 874 | PC-16 | in place |
| `crates/cell-methods/.../interaction/mod.rs` | 871 | none | avoided |

Near the cap: `client_changes.rs` (648; at most one line added, PC-16), `space_manager/mod.rs` (666; re-export only, PC-12) and `instrumentation-discipline.md` (697; no new lines, PC-05). Splitting these is out of scope; flag them for a later cleanup.

## Owner items to tell the user

- PC-11 makes friction DEBUG. The live SigNoz "Playtest: friction" saved view, or any alert filtering on WARN, will go empty until the owner edits it. PC-05 documents this, and PC-07 checks it read-only.
- Releases are user gates: a colo release for PC-07, and a launcher plus colo release for PC-07b. Lab use (PC-19, and calibrate in PC-25) and the colo export (PC-25) are also user gates.
- The camera-to-pose join uses the client's `ts_ms` against the server timestamp within ±1 s. Client clock skew could lower the join rate. PC-07b measures it, and PC-21b's `--join-window-ms` widens the window without a code change.

## Waves

| Wave | Packets | Review |
|---|---|---|
| 1 | PC-01, PC-02 (stacked), PC-06, PC-10, PC-11 | one packet-review-fixer |
| 2 | PC-03, PC-04, PC-12, PC-13 (stacked), PC-14a, PC-14b (stacked) | one packet-review-fixer |
| 3 | PC-05, PC-15, PC-16 (stacked), PC-17, PC-18 | one packet-review-fixer, plus server-authority-enforcer read-only on PC-17 |
| Live | PC-07 after wave 2; PC-19 then PC-07b after wave 3 | each asks the user first |
| 4a | PC-20, PC-22, PC-23 | one packet-review-fixer |
| 4b | PC-21, PC-21b (stacked), PC-24 | one packet-review-fixer |
| After | PC-25 data runs (ask), PC-09 close-out | none |

## Files written or revised

All of them are in `C:\Users\Steve\source\projects\Cimmeria\docs\analysis\player-capture-bots\`:

- README.md: findings F1-F16, decisions, ledger.
- work-packets.md: dispatch rules, rules, contract, index.
- work-packets-w1.md, work-packets-w2.md, work-packets-w3-camera.md and work-packets-w4-tools.md: one file per wave.
- RESUME.md, IDEAS.md (with a status table) and this report.
- architect-report.md: now carries a superseded banner.
