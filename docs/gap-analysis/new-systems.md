---
title: "Gap Analysis: Systems New Since the Original Audit (§31-§37)"
type: explanation
audience: engineers
last_updated: 2026-10-03
companion_docs:
  - ../gap-analysis.md
  - ../project-status.md
---

# Gap Analysis: Systems New Since the Original Audit (§31-§37)

> Part of the [Gap Analysis](../gap-analysis.md), split out of it on 2026-10-03 with no change to any row. The status taxonomy, the evidence bar and the Summary Completion Matrix are in the main file; each matrix row counts the feature rows of its section here, so change both together.

## Systems New Since the Original Audit (March 2026)

These didn't exist in the deprecated Python codebase and so weren't in the audit. They're substantial in Rust today.

### 31. Content Engine --- CW

- **Confidence**: HIGH (re-read 2026-09-25)
- **Documentation**: [content/content-engine.md](../content/content-engine.md), [content/extending-the-engine.md](../content/extending-the-engine.md), [architecture/data-driven-content-engine.md](../architecture/data-driven-content-engine.md), [content/dialog-ui-client-contract.md](../content/dialog-ui-client-contract.md)
- **Rust code**: [`crates/content-engine/`](../../crates/content-engine/) (7,630 lines, 254 tests) + [`crates/cell-content/src/cell/content/`](../../crates/cell-content/src/cell/content/) (43,386 lines, of which about 11,200 is non-test and 27,618 is `chain_replay_tests/`; 707 tests). 277 seeded chains across 11 files in `db/resources/Content/Seed/`.
- **Recent PRs**: **#618 (`MoveEntity` + `GrantXP` arms)**, **#619 (`launch_ability` / `apply_effect` server-authoritative entry point)**, #646-#671 (Cellblock rebuild: `destroy_tagged_entity`, `player_flanked_npc`, cover triggers), #659/#660/#668 (Castle), **#662/#682 (spawn/despawn actions, health trigger, world condition, `grant_stargate_address`, `mission_abandoned`, step-activation region replay)**, #663 (`stargate_dialed` / `stargate_crossed`), #748 (cover replay on step activation), #755 (per-key delivery of new Kismet sequences, so `play_sequence` can target new ids), **#769/#772 (`npc_bark`)**, #768 (dialog button linter)
- **In-client record**: the 2026-09-18 colo playtest drove about 22 missions through chains on two characters. Region, interact, entity-death, dialog-choice, cover, minigame-victory and deferred (`delay_ms`) chains all fired, and the timeline logs `matched` for each.
- **Path forward**: Dispatch the `effect_*` triggers so `effects_chains.sql` and the `apply_effect` arm can run (#610); `remove_effect`, `start_timer` / `cancel_timer`, `roll_loot_table`; persist counters to `content_counters`; the `system_message` wire format (#268); client UAT of the barks and the new triggers.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Trigger / condition / action chain | CW | -- | content-engine/chain.rs | Castle Cellblock and Castle 701-706 end to end in client (2026-09-18 playtest) |
| Loader (DB → Action enum) | CW | -- | content-engine/loader/ | Boundary validation here (e.g. `start_minigame` difficulty range-checked at load, #652) |
| Executor (action → side effects) | CW | -- | cell/content/executor/ | Dispatched arms covered by chain-replay tests that now run through `execute_actions` (#618 pattern) |
| Event dispatch | CW | -- | cell/content/event_dispatch/ | OnEntityDeath, OnInteract, OnDialog, region, cover, minigame victory: client-verified. Triggers added since 2026-07-25 (`stargate_dialed/crossed`, `mission_abandoned`, `player_flanked_npc`, health) are not yet seen in a client |
| Mission-context populator | CW | -- | cell/content/mission_context.rs | -- |
| Chain replay tests | CW | -- | cell/content/chain_replay_tests/ | Pins observed chain behavior (50 modules) |
| Action::ApplyEffect / RemoveEffect | IM | #610 | cell/content/executor/mod.rs:644, content/effect_apply.rs | **KM → IM 2026-09-25.** `ApplyEffect` and `LaunchAbility` arms wired (#619). The only seeded `apply_effect` row sits on an effect-scoped chain that no dispatched trigger reaches (#610). `RemoveEffect` has no arm. Playtest: `launch_ability` 1597 fired but was a no-op because the effect is scriptless. Re-verified 2026-09-25 |
| Action::StartTimer / CancelTimer | KM | -- | -- | Still no arm (falls to `other =>`). Per-action `delay_ms` deferral (executor/deferred.rs) covers one-shot delays only |
| Action::GrantXP | NT | Content | cell/content/executor/mod.rs:512 | **KM → NT 2026-09-25.** Loader + executor arms (#618, closes #611). Refuses `amount == 0`, error-logs send failure, and has an executed chain-replay guard. Zero seed rows, so it has never fired in a client. Re-verified 2026-09-25 |
| Persistent counters | KM | -- | content_counters table | Counters still live in `CellEntity::counters` in memory (executor/counter.rs); the table is only read and written by the admin editor |
| NPC barks (non-modal NPC lines) | NT | -- | cell/content/executor/bark.rs | **New 2026-09-25.** `npc_bark` action sends a dialog screen's text as `onPlayerCommunication` (method 28) on `say`, with no window (#769). Marsh escort lines use it in chains 1176-1178 (#772). Byte-exact wire test; not relogged-replayed by design. UATPending (Cellblock UAT T32). Re-verified 2026-09-25 |

### 32. Mercury Bundle / ChannelBundle --- CW

- **Confidence**: HIGH (re-read 2026-09-25; the only post-July activity is open issue #733 against `bundle.rs`)
- **Documentation**: [architecture/mercury-bundle.md](../architecture/mercury-bundle.md), [architecture/transport-trait.md](../architecture/transport-trait.md)
- **Rust code**: [`crates/mercury/src/channel_bundle/`](../../crates/mercury/src/channel_bundle/mod.rs) (split into `bundle/` + `channel/` by #538), [`crates/mercury/src/bundle.rs`](../../crates/mercury/src/bundle.rs)
- **Recent PRs**: #361 (ChannelBundle + AoI burst), #363 (bundle onClientReady), #365 (bundle progression + teleport), #410 (backpressure), #538 (module split)
- **Path forward**: Fix the latent >64 KiB clamp in the legacy `Bundle::encode`/`decode` (#733). This is not the ChannelBundle path.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Cross-entity bundling | CW | -- | channel_bundle/ | -- |
| AoI burst bundling | CW | -- | PR #361 | -- |
| onClientReady appearance/chat bundling | CW | -- | PR #363 | -- |
| Progression + teleport bundling | CW | -- | PR #365 | -- |
| Backpressure handling | CW | -- | PR #410 | -- |

### 33. Observability Pipeline --- CW

- **Confidence**: HIGH (re-read 2026-09-25: `server/src/logging/`, `cell/console/bookmark.rs`, `cell/player_journal.rs`, `cell/playtest_friction*.rs`, `cell/service/npc_ai/detectors/`; evidence in the NPC-AI UAT-1 worknote and the SigNoz mining record)
- **Documentation**: [architecture/observability.md](../architecture/observability.md), [architecture/instrumentation-discipline.md](../architecture/instrumentation-discipline.md), [operations/signoz-deployment.md](../operations/signoz-deployment.md), [operations/signoz-remote-access.md](../operations/signoz-remote-access.md), [operations/telemetry.md](../operations/telemetry.md), [operations/npc-ai-telemetry-runbook.md](../operations/npc-ai-telemetry-runbook.md) (new), [operations/signoz/](../operations/signoz/) (dashboard JSON + saved views, new), [architecture/negative-logging-convention.md](../architecture/negative-logging-convention.md), [analysis/playtests/2026-09-18-colo-castle/telemetry-design.md](../analysis/playtests/2026-09-18-colo-castle/telemetry-design.md)
- **Rust code**: OTLP exporter in [`crates/server/src/otel.rs`](../../crates/server/src/otel.rs) and [`crates/server/src/logging/`](../../crates/server/src/logging/) (now a directory: `filters`, `parity_tests`, `target_scan_tests`); metrics facade [`crates/observability/`](../../crates/observability/) (435 lines); Mercury packet instrumentation in [`crates/mercury/src/instrumentation.rs`](../../crates/mercury/src/instrumentation.rs); playtest tooling in [`cell/console/bookmark.rs`](../../crates/cell-console/src/cell/console/bookmark.rs), [`cell/player_journal.rs`](../../crates/wire/src/cell/player_journal.rs), [`cell/playtest_friction.rs`](../../crates/cell-world/src/cell/playtest_friction.rs) + `playtest_friction_watch.rs`; NPC AI detectors in [`cell/service/npc_ai/detectors/`](../../crates/cell-world/src/cell/service/npc_ai/detectors/); negative-logging convention enforced by the `LogCapture` test helper
- **Recent PRs**: #396, #398, #400, #402, #404, #410, #414, #483 (full pipeline); **#676 (`.bug` bookmark, stuck-player detectors, NPC nav seams)**, **#678 (remaining stuck-player detectors + outbound-intent logging)**, **#679 (cover + trigger-ordering seams)**, **#680 (per-player journal, deferred-action reports, `npc_ai.tick`)**, **#700 / #726 (navmesh observability + review fixes)**, **#776 (NPC AI transition helper, aggro cause, OTLP identity)**, **#781 (NPC AI stuck/float/path/LoS/leash/cover detectors)**, **#782 (NPC AI health dashboard, 9 saved views, runbook)**, **#792 (disk-to-SigNoz parity: `cimmeria-trace` index, sampled firehoses)**, #791 (`.bug` witness fix)
- **Path forward**: The colo still reports `cimmeria.deploy_env = dev` because Watchtower does not re-apply the compose file. This is an owner action (UAT-1 finding 9). Filter on `service.version` until it is fixed. Also: confirm the friction detectors fire in a real session, and populate or confirm the NPC AI dashboard panels.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| OTLP log appender | CW | -- | server/otel.rs, server/logging/ | PR #398. Colo sessions reconstructed from SigNoz: playtest 2026-09-18 (~110k rows), NPC-AI UAT-1 (2026-09-25) |
| Hot-path tracing spans | CW | -- | mercury/, base/, cell/ | PR #398 |
| Mercury packet logging | CW | -- | mercury/instrumentation.rs | Per-packet OTLP |
| Wire-log capture stream | CW | -- | PR #404 | Per-message decode to SigNoz. Since #792 the AoI position-update firehose is sampled 1-in-101 to SigNoz and stays complete on disk |
| SigNoz self-hosted overlay | CW | -- | operations/signoz-deployment.md | ClickHouse-backed |
| Cloudflare Tunnel + Access | CW | -- | operations/signoz-remote-access.md | No inbound ports |
| Dev-session telemetry | CW | -- | architecture/dev-session-telemetry.md | HMAC-signed token. Mint quota added by #740 |
| Negative-logging convention | CW | -- | architecture/negative-logging-convention.md | LogCapture regression-guard helper. Now includes the credential-field rule (#698) and cross-IP seams (#738) |
| `.bug` playtest bookmark + per-player journal | CW | -- | cell/console/bookmark.rs, cell/player_journal.rs | **New 2026-09-25.** #676/#680, witness fix #791. `.bug <note>` snapshots the tester's scene (`playtest.bookmark` + `playtest.bookmark.entity`) and attaches the last 24 `player.journal` events. Written in-client record: 19 NPC-relevant bookmarks from colo sessions 2026-09-19..21 (analysis/npc-ai-restoration/evidence/signoz-npc-mining.md §A), and the owner's `.bug` notes drove UAT-1 (worknotes/uat-1.md). UAT-1 finding 5 (empty witness list) was fixed by #791 |
| NPC AI telemetry + anomaly detectors | CW | -- | cell/service/npc_ai/detectors/, npc_ai.tick | **New 2026-09-25.** #680/#776/#781: `npc_ai.tick`, aggro cause, off-mesh/stuck/float/path/LoS/leash/cover detectors. UAT-1 (worknotes/uat-1.md, 2026-09-25, colo, build `059d6038`) diagnosed findings 1-8 from these rows (`npc_ai.off_mesh`, `cover.flank_check`, `los_policy`). Finding 8 (idle tick volume) was fixed by #791 |
| Stuck-player friction detectors | NT | -- | cell/playtest_friction.rs, playtest_friction_watch.rs | **New 2026-09-25.** #676/#678: `repeat_interact_no_effect`, `repeat_item_use_no_chain`, `console_reject_streak`, `escort_separated`, `escort_leader_teleported`, plus outbound-intent logging. Unit-tested. No written record of a detector firing in a live session |
| NPC AI health dashboard + saved views | NT | -- | docs/operations/signoz/ | **New 2026-09-25.** #782: 11-panel SigNoz dashboard and 9 saved Logs Explorer views, created via MCP, with re-importable JSON and an operator runbook. The PR states that most panels populate only after the first colo session on an NA00/NA02 build. No record of the dashboard itself being read after a session |
| Disk-to-SigNoz log parity (`cimmeria-trace`) | NT | -- | server/logging/ | **New 2026-09-25.** #792: a third, TRACE-only SigNoz service carries ~70 formerly disk-only `trace!` sites. Prime-N sampled firehoses (`DECRYPT_OK`, `UDP_IN` 1-in-53; the `UDP_IN` sample carries no hex). 14 previously invisible DEBUG targets exported. `parity_tests` + `target_scan_tests` guard it. Merged 2026-09-25, not yet exercised in a recorded session |

### 34. Wireclient + Network Chaos Testing --- IM

- **Confidence**: HIGH (re-read 2026-09-25; `crates/wireclient` has no commits since 2026-07-25)
- **Documentation**: [architecture/wireclient.md](../architecture/wireclient.md), [architecture/network-chaos-testing.md](../architecture/network-chaos-testing.md), [architecture/mercury-loopback-harness.md](../architecture/mercury-loopback-harness.md)
- **Rust code**: [`crates/wireclient/`](../../crates/wireclient/): 1,947 lines across 8 files (auth, handshake, session_trace, client, error, lib + 2 test files), **30 tests**; LossyTransport in mercury; loopback harness for Tier 2; 9 chaos scenarios under `mercury/src/test_harness/tests/chaos/`
- **Recent PRs**: #370 (Tier 2 loopback harness), #374 (network chaos L1+L2+L3), #376 (Tier 3 wireclient scaffold), **#716 (flaky `tx_window_overflow` chaos scenario fixed: 7/30 → 30/30 under load, #713)**
- **Path forward**: Phase 1.5 socket loop (`client.rs:56-59`), then pcap replay against a live server (#281). Also: `Credentials`/`AuthSession` derive `Debug`, so one `{:?}` log would leak the password hash or session key (#698 follow-up).

> **Correction, 2026-07-25 (still true 2026-09-25): wireclient cannot send a UDP packet.** There is **no `UdpSocket` anywhere in `crates/wireclient`**. The only textual hit is a doc comment at handshake.rs:92. `Client::connect()` does not exist, and `client.rs:56-59` says Phase 1 "stops at *produce the bytes*". The Tier 2 loopback and chaos rows below are unaffected.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| SOAP auth client (Phase 1+2) | IM | -- | wireclient/src/auth.rs | 357 lines, driven against an in-process `AuthService` over real TCP by tests/it/auth_smoke.rs. This is a live SOAP client, not replay |
| Mercury phase-3 handshake | IM | Socket loop | wireclient/src/handshake.rs | 546 lines: `build_baseapp_login` + reply parser. It produces and consumes bytes but **cannot perform a handshake**, because nothing sends them |
| Pcap+key replay | KM | Socket loop | tools/pcap_to_session.py only | The Python tool converts `.pcap` + `keys.txt` → JSONL. No replay engine exists on either side |
| Session-trace JSONL | IM | -- | wireclient/src/session_trace.rs | 567 lines: `Trace::from_jsonl_path`, c2s/s2c iterators, `Diff` + `DefaultPolicy`. 10 tests + tests/it/trace_load.rs |
| LossyTransport drop/dup/reorder/latency | CW | -- | mercury/lossy_transport.rs | -- |
| Loopback paired-channel tests | CW | -- | mercury/test_harness/ | 22 tests |
| Network-chaos scenarios | CW | -- | mercury/test_harness/tests/chaos/ | 9 scenarios incl. `replay_lomiada`, `sustained_5pct_loss_60s`, `tx_window_overflow_with_recovery`. The last was flaky until #716 |

### 35. Discord Notifications --- CW

- **Confidence**: HIGH (re-read 2026-09-25; `crates/discord` has no commits since 2026-06-19)
- **Documentation**: [architecture/discord-notifications.md](../architecture/discord-notifications.md)
- **Rust code**: [`crates/discord/`](../../crates/discord/): 6,079 lines, 76 tests (config/, embed/, event/, sender/, layer.rs, router.rs, color.rs)
- **Recent PRs**: #397 (notification crate + tracing-layer harvest + panic hook), #540 (module split), #554 (account names, new gameplay events, player IPs dropped), #560 (config test coverage). No change since 2026-07-25. #676 routes `.bug` notes to the GM channel through the existing console audit relay

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| EventKind enum | CW | -- | discord/src/event/ | Variant-count pinning test |
| Channel routing | CW | -- | discord/src/router.rs | channel_for() |
| Embed formatting | CW | -- | discord/src/embed/ | format_event() |
| Panic hook capture | CW | -- | discord/src/ | -- |
| Per-channel toggles | CW | -- | EventToggles | -- |
| Colo deploy wiring | CW | -- | docker/compose.discord.yml | -- |

### 36. Tauri Admin App + Tools --- IM

- **Confidence**: HIGH for admin-api (every route file read for TODO/"not implemented" stubs 2026-09-25); MEDIUM for the Tauri front ends (no changes since 2026-07-25, not re-read)
- **Documentation**: [tools/admin-api.md](../tools/admin-api.md), [tools/admin-panel.md](../tools/admin-panel.md), [client/sgw-launcher.md](../client/sgw-launcher.md), [architecture/live-research-lab.md](../architecture/live-research-lab.md), [guides/live-research-lab.md](../guides/live-research-lab.md), [engine/ue3-package-format.md](../engine/ue3-package-format.md#writing-packages--the-append-only-patcher), [analysis/ring-transport-cellblock-castle/README.md](../analysis/ring-transport-cellblock-castle/README.md)
- **Rust code**: [`crates/admin-api/`](../../crates/admin-api/) (axum REST + WS, 5,217 lines); `src-tauri/` (admin panel app, `cimmeria-app`); [`tools/ContentEditor`](../../tools/ContentEditor/), [`tools/SceneEditor`](../../tools/SceneEditor/) (Tauri); [`crates/launcher/`](../../crates/launcher/) (sgw-launcher, egui, 7,252 lines); [`crates/upk/`](../../crates/upk/) (4,724 lines, incl. `patcher/` + `upk_patch` CLI); live research lab: [`crates/lab/`](../../crates/lab/), [`crates/lab-mcp/`](../../crates/lab-mcp/) (1,187), [`crates/client-launch/`](../../crates/client-launch/) (6,606 lines together), plus the `lab-bridge` feature of `crates/client-telemetry`
- **Recent PRs**: **#724 (admin API binds 127.0.0.1 by default until JWT lands, #439)**, **#740 (dev-session mint quota)**, **#751 / #753 (UPK append-only patcher + Kismet rig cloning, ring transport Phases 0-1)**, **#692 / #696 / #706 / #693 / #695 / #703 (live research lab 1-6/6)**
- **Path forward**: JWT middleware and login (#439, #25). Entity WebSocket stream (#27). Entity, player, content and config endpoints still return "not implemented" in places (#26/#28/#29). Tighten CORS from `Any`. Live validation of the research lab exit criteria. Ring transport Phase 1 in-client test. Three.js space viewer.
- **Development tooling, not counted in the matrix**: the token profiler, [`tools/token-profile/`](../../tools/token-profile/README.md) (token-usage campaign, #1122 to #1140, closed 2026-10-03). It ingests local Claude Code transcripts, reports cost, context and attribution per PR and campaign, reconciles against Claude Code's own totals, and posts one stats comment on every merged PR (backfilled over 406 PRs). It runs on a developer workstation, never in the server, so it has no feature row. How-to: [guides/token-profiling.md](../guides/token-profiling.md); ledger: [analysis/token-usage/](../analysis/token-usage/README.md).

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| REST admin API | IM | JWT | crates/admin-api/routes/ | Real: players, spaces, audit, editor, telemetry, dev-session. Stubs returning "not implemented": entities.rs:38/62/86, content.rs:73/94/352, config.rs:105, editor.rs:553/604 (issues #26/#28/#29 open). Binds loopback by default since #724 |
| WebSocket entity stream | KM | -- | admin-api/ws/entity_stream.rs | **Re-verified 2026-09-25 (was IM).** Stub only: `handle_entity_socket` accepts the upgrade and logs, and its body is three `TODO` comments (entity_stream.rs:26-30). Issue #27 open |
| WebSocket log stream | IM | JWT | admin-api/ws/log_stream.rs, ws/broadcast_layer.rs | Real: ring-buffer replay + live broadcast. Unauthenticated (`/ws/logs` is named in #439) |
| JWT auth for remote | KM | -- | admin-api/middleware.rs:19, routes/auth.rs:41-81 | **Re-verified 2026-09-25 (was IM).** No code: the middleware is a `// TODO` block, and `/api/auth/login`, `/logout` and `/me` return `"not implemented"`. `jsonwebtoken` is declared but unused. Critical issue #439 is open (re-rated P3 on 2026-09-25 because the colo edge keeps 8443 private); #25 open. #724 is containment only |
| Content editor (Tauri) | IM | -- | tools/ContentEditor | React + xyflow visual chain editor |
| Scene editor (Tauri) | IM | -- | tools/SceneEditor | -- |
| Admin panel (Tauri) | IM | -- | src-tauri/ | -- |
| SGW launcher (egui) | CW | -- | crates/launcher/ | Seed + patch manifest, Ed25519 signed |
| Three.js space viewer | KM | -- | -- | Phase 2 of the admin UI plan |
| UPK append-only package patcher | CW | -- | crates/upk/src/patcher/, bin/upk_patch.rs | **New 2026-09-25.** #751. Written in-client record: ring-transport README "Phase 0 status", **2026-09-19 PASSED**. The owner loaded CellBlock with the patched stasis-hall chunk, and the cloned ring station rendered lit and at floor height |
| UPK Kismet rig cloning | NT | -- | crates/upk/src/patcher/ (`clone_objects`) | **New 2026-09-25.** #753 (re-land of #752): clones region 3's 32-object ring rig onto the Armory pad (62 new exports). README "Phase 1 status" says **built, awaiting the in-client test** (`Install-Phase1.ps1 rig`, then `.net_seq 10187 3`) |
| Live research lab: client bridge, native probes, supervisor | IM | -- | crates/client-telemetry (`lab-bridge`), crates/lab/, crates/client-launch/ | **New 2026-09-25.** #692/#696/#706/#703. Lua eval bridge, dynamic hooks and memory write/call probes, crash-recovery supervisor with autologin, merged `lab_timeline`. The PR bodies record that each exit criterion still needs live validation on the owner's box. The Lua C API export and the console-exec address are unconfirmed (#706), and autologin screen reads depend on `lua_eval` capture (#696). Issues #684-#690 still open |
| Live research lab: in-server MCP endpoint | NT | -- | crates/lab-mcp/ | **New 2026-09-25.** #693/#695: console passthrough, sessions, logs, read-only SQL, LabQuery snapshots, witness reports, packet taps. The end-to-end AoI reproduction is left to a UAT step (#695). Colo exposure is WireGuard-only (#703) |

### 37. Ring Transport --- IM

**Added 2026-07-25.** Cross-region and cross-world transporter rings. A player steps onto a ring pad and picks a destination. The server then drives a multi-second state machine: it plays Kismet sequences at both ends, hides the players, teleports them and shows them again.

- **Confidence**: HIGH (re-read 2026-09-25)
- **Documentation**: [gameplay/ring-transport-system.md](../gameplay/ring-transport-system.md) (includes "Bounded aborts"), [analysis/ring-transport-cellblock-castle/README.md](../analysis/ring-transport-cellblock-castle/README.md) (mission 688 client-patch plan), [engine/ue3-package-format.md](../engine/ue3-package-format.md) (append-only patcher)
- **Rust code**:
  - [`crates/cell-content/src/cell/ring_transport/`](../../crates/cell-content/src/cell/ring_transport/) has **5,856 lines**: 3,191 production and 2,665 test. It spans `regions.rs` (349), `transporter/{mod, manager, source, destination, effects}`, `runtime/{entry, tick, teardown}`, `dispatch.rs`, `wire.rs` and `wire_helpers.rs`.
  - Chains enter through the content action at `cell/content/executor/transport.rs:14`, which calls `handle_interact`.
  - Sequence delivery for the new rig is in `base/sequence_overrides.rs` (169).
  - The package patcher is `crates/upk/src/patcher/` (2,213).
  - `python/cell/RingTransporter.py` remains the spec for the state graph and timings.
- **Recent PRs**:
  - Server:
    - **#662 / Harset H02**: bounded FSM stall timeouts, departing-player cleanup, four FSM review fixes, ring-arrival validation.
    - **#755**: sequences 10187 / 10188 delivered per key.
    - **#754**: Kismet PAK version restored. #753 had wiped every client's sequence table.
  - Client patch:
    - **#750**: 688 ring-ceremony audit.
    - **#751**: package writer, Phase 0.
    - **#752 / #753**: Kismet rig clone onto the Armory pad, Phase 1.
- **In-client record**: P1 rode the Cellblock rings four times with a real client: regions 1 → 2 and 2 → 3 on both characters. In each case the chain triggered the transporter and the client returned a destination selection. The FSM ran, the player was teleported to the destination pad (`TeleportPlayer` snap `[-192.66, 55.26, -154.84]` = region 2), `teleport_in` fired, and mission 640 completed or 680 advanced ([appendix-session-timeline.md](../analysis/playtests/2026-09-18-colo-castle/appendix-session-timeline.md) rows 00:08:37, 00:11:31, 01:10:12). That build predates #662. The timeout and cleanup changes are unit-tested only. Separately, the Phase 0 client-patch load test passed on 2026-09-19 (the PR #751 owner comment).
- **Why still IM**:
  - No record says the ring *animation* or the hide/show was seen.
  - `setRingTransporterDestination` does not check that the caller is on or near the pad (#461 CAT-B-03).
  - Cross-world ring travel (Omega Site ↔ Command Center, regions 14/17) has never run.
  - The mission 688 ceremony is still a direct `cross_world_teleport`.
- **Path forward**:
  - Close CAT-B-03.
  - Run the Phase 1 in-client test: repair the client `Cache.en-US` Kismet PAK, then `.net_seq 1951 3` at region 3 and `.net_seq 10187 3` at the Armory pad.
  - Phase 2 (Castle pad) and Phase 3 (route chain 1109 through the FSM).
  - Run an Omega cross-world ring smoke.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Ring region loading | CW | -- | ring_transport/regions.rs | **Promoted 2026-09-25.** `ring_transport_regions` → `RingRegion`, 30 seeded rows. In-client: P1's hops landed on the seeded region coordinates, for example region 2 at `(-192.657, 55.258, -154.844)` ([appendix-session-timeline.md](../analysis/playtests/2026-09-18-colo-castle/appendix-session-timeline.md) 01:10:12). Re-verified 2026-09-25 |
| Destination list to client | CW | -- | ring_transport/wire.rs; dispatch.rs:201 | **Promoted 2026-09-25.** `interact()` emits `SendDestinationList` → `onRingTransporterList`. In-client: in all four P1 hops the client answered with a valid `setRingTransporterDestination`, which the FSM requires before it leaves Idle (`validate_destination`, transporter/mod.rs:397). The trips completed (appendix rows 00:08:37, 00:11:31). Re-verified 2026-09-25 |
| Transport state machine | CW | -- | ring_transport/transporter/mod.rs | **Promoted 2026-09-25.** 454 lines; the manager is transporter/manager.rs. In-client: P1 trigger → teleport_in in 18 s (00:08:37 → 00:08:55), four successful intra-world trips over two characters, each followed by the chain that expects arrival. Caveat: tested on the 2026-09-18 build. #662's stall timeouts landed after and are covered by unit tests (tests/stall.rs, deadline_scan.rs). Re-verified 2026-09-25 |
| Kismet sequence playback | NT | -- | ring_transport/wire_helpers.rs | `onSequence` at both origin and destination (event sets 10000/874/875). Sent in P1, but no record says the animation was seen. #754 fixed a regression (#753's PAK version bump) that would have stopped every ring animation for any client connecting to that release. **Status changed 2026-09-25 (was IM).** |
| Hide / show + movement lock | NT | -- | ring_transport/wire_helpers.rs | `onVisible`, `onStateFieldUpdate`, `BSF_MOVEMENT_LOCK`. The abort path releases only `ShowPlayer` / `UnlockMovement` (`dispatch_release_effects`). Sent in P1; not recorded visually. **Status changed 2026-09-25 (was IM).** |
| Region-trigger entry | IM | -- | ring_transport/runtime/entry.rs | `handle_interact` (:38), `handle_select_destination` (:94), `handle_region_trigger` (:276). All three ran in P1's hops. **Stays IM on a recorded defect:** `setRingTransporterDestination` never checks that the caller is on or near the source pad or in its world, so any client can start or grief any ring. Open in #461 (CAT-B-03). Re-verified 2026-09-25 |
| Cross-world ring travel | NT | -- | ring_transport/dispatch.rs:126; runtime/tick.rs | **Changed 2026-09-25 (was IM).** `Effect::TeleportCrossWorld` goes through GateTravel. The destination waits for `AdvanceRingDestination` or `REMOTE_LOAD_WAIT_TIMEOUT` (90 s, a judgement value), and the traveller is not stranded if the source is destroyed (tests/disconnect.rs:93). One seeded route (Omega Site 14 ↔ Cmd Center 17) has never run in-client. The Cellblock → Castle exit does **not** use it (chain 1109 is a direct `cross_world_teleport`) |
| Stall timeouts + departure cleanup | NT | -- | ring_transport/transporter/mod.rs:105-114; runtime/teardown.rs | **New 2026-09-25.** Bounded per-state stall timeouts (`SEND_WAIT` 60 s, `RECV_WAIT` 65 s, `RECV_WARMUP` 15 s, `REMOTE_LOAD_WAIT` 90 s). The 2009 server had none. A departing or disconnecting player is removed from the ring (`forget_player`), and aborts release the lock and visibility without firing arrival content (#662 H02). Unit and negative-log tested; not client-exercised |
| CellBlock → Castle ring ceremony (mission 688 client patch) | IM | -- | crates/upk/src/patcher/; base/sequence_overrides.rs; db/resources/Events/Seed/sequences.sql (10187/10188) | **New 2026-09-25.** Owner-approved exception to "no client patch". Phase 0 **passed in-client** on 2026-09-19: the cloned station rendered, lit and at floor height (PR #751 owner comment). Phase 1 is built: region 3's Kismet rig is cloned onto the Armory pad, and sequences 10187 / 10188 are delivered per key (#753, #755). Its in-client animation test is pending. Phases 2–3 (Castle pad, route chain 1109 through the FSM) have not started, and the exit is still a direct teleport |
