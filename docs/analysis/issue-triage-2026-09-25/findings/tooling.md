# Triage findings — batch `tooling`

Code of record: `main-ro` (origin/main 059d6038, 2026-09-25). Research only; nothing posted.

## #684 — Live research lab 1/6: client bridge core (inbound channel + Lua eval in the telemetry DLL)

- Verdict: CLOSE (completed)
- Priority: P3
- Labels: no change
- Summary: The issue asks for an off-by-default `lab-bridge` feature on `cimmeria-client-telemetry` with a double gate, framed JSON-RPC over TCP, main-thread dispatch from the `FEngineLoop::Tick` hook, `lua_eval`/`module_info`/`mem_read`, and a minimal `cimmeria-lab` stdio MCP proxy. PR #692 (merged 2026-09-19, fcfdcd29) shipped all of it. PR #706 (#686) later added `lua_eval` return-value and `print` capture, which #692 had left as stubs. The PR bodies used "Refs", not "Closes", so the issue never auto-closed. The only open item is the live exit criterion (evaluate Lua in a running client), which belongs to epic-level validation.
- Evidence:
  - `crates/client-telemetry/Cargo.toml:29` `lab-bridge = ["dep:microseh"]`. Bridge modules are under `crates/client-telemetry/src/bridge/` (`transport.rs`, `dispatch.rs`, `lua_eval.rs`, `lua_capture.rs`, `memory.rs`).
  - `crates/lab/src/server.rs` has the `client_*` proxy tools. `cimmeria-lab` appears in the CLAUDE.md exclusion lists (CLAUDE.md:70,95,100,104).
  - PR #692 lists tests for framing, token rejection, second-client rejection, queue overflow, slide math, and the no-`lab`-block guard. PR #706 adds `lua_capture`.
- Related/duplicates: #690 (epic), #686 (lua capture landed there)

### Action text

Closing as completed. PR #692 (fcfdcd29) landed the bridge core behind the off-by-default `lab-bridge` feature: the double gate (feature plus a `lab` block in `current-session.json`), JSON-RPC framing with a 4-byte LE length prefix, single-client token auth, the bounded queue drained on the main thread from the `FEngineLoop::Tick` hook, `lua_eval`/`module_info`/`mem_read`, and the `cimmeria-lab` stdio proxy. Lua return-value and `print` capture, stubbed in #692, landed with #686 in PR #706. The live exit criterion (evaluate Lua in a running client from Claude Code) is still unvalidated. It is now tracked in the live-validation checklist on epic #690.

## #685 — Live research lab 2/6: cimmeria-lab supervisor (lifecycle, autologin, screenshots, crash recovery)

- Verdict: CLOSE (completed)
- Priority: P3
- Labels: no change
- Summary: The requested scope landed in PR #696 (merged 2026-09-19, 1c428f84): the shared launch/inject/patch_rdata library (`crates/client-launch`), the supervisor tools (start/stop/restart/status/login/screenshot/crash_report), the heartbeat watchdog, WER suppression, the outer-tier unhandled-exception filter with a minidump, the command journal with quarantine, the 3-crashes-in-10-minutes cap, Lua autologin, and the gitignored lab-account file. ADR open question 2 (chain or replace the filter) is resolved to "replace". What remains is live-only: the Task-Manager-kill exit criterion, and confirming one persistent Lua VM at the login screen (ADR §10 Q1 is still open).
- Evidence:
  - `crates/client-launch/src/{launch,inject,patch_rdata}.rs`.
  - `crates/lab/src/supervisor/{autologin,autologin_bridge,heartbeat,recovery,screenshot,crash_report,session_file,process}.rs`.
  - `crates/client-telemetry/src/bridge/{crash,journal}.rs`.
  - `docs/architecture/live-research-lab.md` §10: Q2 is resolved and Q1 is still open.
  - PR #696 says autologin screen reads were blocked on `lua_eval` capture. That capture shipped in PR #706.
- Related/duplicates: #690, #686

### Action text

Closing as completed. PR #696 (1c428f84) delivered the full scope:

- the shared `cimmeria-client-launch` crate for launch, inject, and patch_rdata
- the supervisor tools: `lab_client_start/stop/restart/status`, `lab_login`, `lab_screenshot`, and `lab_crash_report`
- the heartbeat watchdog with WER suppression
- the replace-not-chain unhandled-exception filter with a minidump (ADR §10 Q2 resolved)
- the journal with quarantine and the 3-in-10-min recovery cap
- Lua-driven autologin

The autologin screen reads needed `lua_eval` return capture, which landed in PR #706. Two items can only be checked live and move to the validation checklist on epic #690: the Task-Manager-kill exit criterion, and ADR §10 Q1 (is the Lua VM alive at the login screen, checkable from the `client.lua.newstate` count in SigNoz).

## #686 — Live research lab 3/6: native probes (dynamic logging hooks, memory write, native calls, quarantine)

- Verdict: CLOSE (completed)
- Priority: P3
- Labels: no change
- Summary: PR #706 (merged 2026-09-19, c6aebb38) landed the microseh inner-tier SEH guard, `client_mem_write`, `client_call_native`, `client_hook_install/remove/list`, `client_events_read`, `client_console`, and persistent-hook re-application after a crash. The scope limits are documented in the code and match the issue's "function-entry-only is acceptable" clause. Hooks are function-entry and cdecl-only, and thiscall/stdcall hooks are refused cleanly. `client_console` degrades with a reason until a `ConsoleCommand(FString)` address is confirmed. CEGUI lines are not yet in the event ring. The live exit criterion (re-derive the black-market window through lab tools) is unvalidated.
- Evidence:
  - `crates/client-telemetry/src/bridge/{seh,mem_write,native_call,events,console}.rs` and `bridge/dynamic_hooks/`.
  - `bridge/dynamic_hooks/mod.rs:74-81` refuses thiscall/stdcall.
  - `bridge/console.rs:19-20` degrades until an address is supplied.
  - `crates/lab/src/supervisor/recovery.rs` (`PersistentHooks`).
  - `crates/lab/src/server.rs:281-289` (`client_events_read` proxy).
- Related/duplicates: #690; residual follow-ups listed in the #690 rewrite

### Action text

Closing as completed. PR #706 (c6aebb38) landed the native-probe tier:

- the microseh per-dispatch guard
- `client_mem_write` and `client_call_native`
- function-entry dynamic hooks, with capture spec, hit limit, sampling, and a persistent flag
- the `client_events_read` ring (hook hits, Lua prints, and Mercury dispatch)
- `client_console`
- journal and quarantine, with only persistent hooks replayed after a crash

Three limits are deliberate and documented: hooks are cdecl, function-entry only (thiscall/stdcall are refused cleanly), `client_console` degrades until the `ConsoleCommand(FString)` address is confirmed in Ghidra, and CEGUI log lines are not in the ring yet. These and the live exit criterion (re-derive the black-market window with no relaunch) are carried as unchecked items on epic #690.

## #687 — Live research lab 4/6: in-server MCP endpoint v1 (console passthrough, sessions, logs, read-only SQL)

- Verdict: CLOSE (completed)
- Priority: P3
- Labels: no change
- Summary: PR #693 (merged 2026-09-19, 91408926) shipped `crates/lab-mcp`. It runs on its own listener, never on the admin router. Startup is fail-closed: both env vars are required and the token must be at least 32 bytes. Auth uses a constant-time bearer check, and every call writes a `lab.tool_call` audit event. It includes all six tools and the `BaseToCellMsg::LabConsoleExec` reply with the GM gate on the acting entity. The live-DB tests prove that `server_db_query` rejects writes. One design deviation: console output is captured by teeing the cell→base channel, not by an output-sink refactor. The observable outcome is the same. Known v1 limitation: the async `.spawn` confirmation arrives as chat, not in the synchronous reply. The exit criterion is unvalidated live.
- Evidence:
  - `crates/lab-mcp/src/{config,auth,audit,state}.rs` and `tools/{console,sessions,logs,content,db}.rs`.
  - `crates/services/src/cell/service/base_messages/lab_console.rs:32-87` (GM gate at :52).
  - `crates/server/src/main.rs:43` (`CIMMERIA_LAB_MCP_BIND` env row). CLAUDE.md:188 has the doc-map row.
- Related/duplicates: #690, #439 (admin router deliberately not shared)

### Action text

Closing as completed. PR #693 (91408926) shipped `cimmeria-lab-mcp`:

- its own listener, with fail-closed startup (bind and token both required, token of 32 bytes or more)
- a constant-time bearer check and the `lab.tool_call` audit event
- the six v1 tools
- `LabConsoleExec`, with the GM gate enforced on the acting entity
- live-DB guards showing that `server_db_query` rejects `DELETE` and data-modifying CTEs

Console output is captured by teeing the cell→base channel. The issue asked for an output-sink refactor, but the tee gives the same result and leaves the in-world path unchanged. Known v1 limitation: `.spawn`'s async confirmation arrives as chat to the lab character, not in the synchronous reply. The live exit criterion moves to epic #690's validation checklist.

## #688 — Live research lab 5/6: server live state (LabQuery, entity and witness queries, packet taps)

- Verdict: CLOSE (completed)
- Priority: P3
- Labels: no change
- Summary: PR #695 (merged 2026-09-19, beaf7947) added:
  - `BaseToCellMsg::LabQuery`, which answers read-only snapshots between ticks with a 256-entity cap
  - the `server_entity_get` / `server_entity_query` / `server_witnesses` tools
  - per-session packet taps on the decoded `wire_log` seams, with a bounded ring and per-session isolation

  The tests cover each query, the size cap, the ring bound, and isolation. The end-to-end AoI exit criterion (the invisible Cellblock corpse) needs a live client and is unvalidated.
- Evidence:
  - `crates/lab-mcp/src/tools/{entities,witnesses,packet_tap}.rs`.
  - PR #695 names `space_manager/lab_snapshots.rs` and `wire_log/tap.rs` in `cimmeria-services`.
- Related/duplicates: #690; the corpse bug is tracked in agent memory `project_aoi_static_npc_missing_until_relog`

### Action text

Closing as completed. PR #695 (beaf7947) landed `LabQuery` snapshots answered between ticks. They are read-only against `&SpaceManager` and capped at 256 entities. The PR also added the entity, entity-query, and witness tools, and per-session decoded packet taps built on the existing `wire_log` seams: a bounded ring, zero cost when no tap is active, and per-session isolation in both directions. The motivating end-to-end check still needs a live client and moves to epic #690's validation checklist: the server reports the Cellblock corpse as witnessed, the tap shows the create packet, and a client hook shows whether the client consumed it.

## #689 — Live research lab 6/6: merged timeline, research rulebook, setup docs, colo WireGuard wiring

- Verdict: CLOSE (completed)
- Priority: P3
- Labels: no change
- Summary: PR #703 (merged 2026-09-19, 8a203a40) shipped:
  - `lab_timeline`: clock-offset estimate, merge, and window
  - `docs/guides/live-research-lab.md`
  - `.mcp.json.example` entries
  - updates to the RE guides and to `client-telemetry.md`
  - the opt-in `docker/compose.lab.yml` overlay, published on the WireGuard IP only, with a colo-deploy doc update

  Three gaps remain, and one is a real code seam. #703 was written while #686 was "stopped", but #686 then merged in #706, and nobody went back to wire the seam. `drain_event_ring()` in `crates/lab/src/timeline/client_events.rs:68-71` still returns an empty Vec with `TODO(#686)`. As a result, the timeline merges only heartbeats, not the hook-hit / Lua-print / Mercury events that `client_events_read` now provides. The clock offset is coarse because the server has no dedicated ping tool. The streamable-HTTP MCP handshake (`initialize` plus `Mcp-Session-Id`) in `PacketTapClient::fetch` is unverified. None of these warrants keeping a phase ticket open. They move to the epic's checklist.
- Evidence:
  - `crates/lab/src/timeline/{clock,event,client_events,packet_tap,mod}.rs`.
  - `client_events.rs:68-71` is the stub. `mod.rs:88` calls it.
  - `crates/lab/src/server.rs:281-289` shows `events_read` is already proxied.
  - `docker/compose.lab.yml`.
  - CLAUDE.md:188 has the doc-map row.
- Related/duplicates: #690, #686

### Action text

Closing as completed. PR #703 (8a203a40) shipped `lab_timeline` (NTP-style offset estimate plus merge and window), the research rulebook at `docs/guides/live-research-lab.md`, the `.mcp.json.example` entries, the RE-guide and `client-telemetry.md` updates, and the WireGuard-only `docker/compose.lab.yml` overlay. One seam was left behind: #703 assumed #686 was stopped, but #686 then landed in PR #706. So `timeline::client_events::drain_event_ring()` (`crates/lab/src/timeline/client_events.rs:68`) still returns an empty Vec, and the timeline merges only heartbeats. Wiring it to the bridge's `events_read` is a one-site change. It is now an unchecked item on epic #690, along with a dedicated server ping for the clock offset, verification of the MCP HTTP handshake, and the colo exit criterion.

## #690 — Epic: Live research lab — MCP access to the running client and server

- Verdict: REWRITE
- Priority: P3
- Labels: no change (optionally add `ready-for-human`: the remaining work is mostly live validation on the owner's box)
- Summary: All six phase PRs merged on 2026-09-19 (#692, #696, #706, #693, #695, #703), but none used a closing keyword, so every child and the epic are still open. Everything automated is done. What remains:
  - (a) Every phase's exit criterion needs live validation on the owner's machine or the colo. None is recorded as done.
  - (b) The `drain_event_ring` seam in `lab_timeline` was never wired after #686 landed.
  - (c) Documented scope limits: cdecl-only hooks, the unconfirmed `ConsoleCommand(FString)` address, no CEGUI lines in the event ring, the coarse clock offset, the unverified MCP HTTP handshake.
  - (d) ADR §10 Q1 (Lua VM alive at the login screen) is still open, and the ADR status still reads "Proposed".

  The body's phase checklist is stale.
- Evidence:
  - merged-prs.tsv rows 692, 693, 695, 696, 703, 706 (no closing refs).
  - `crates/lab/src/timeline/client_events.rs:68-71`.
  - `crates/client-telemetry/src/bridge/dynamic_hooks/mod.rs:74-81` and `bridge/console.rs:19-20`.
  - `docs/architecture/live-research-lab.md:6` (Status: Proposed) and §10.
- Related/duplicates: #480 (overlapping Tier-4 client test driver), #393 (Atrea bridge; ADR §10 Q4 says it should adopt the lab transport), #439

### Action text

Comment:

> All six phases have merged: #684 in PR #692, #685 in PR #696, #686 in PR #706, #687 in PR #693, #688 in PR #695, and #689 in PR #703. The PRs said "Refs" instead of "Closes", so the children stayed open. I'm closing the children and rewriting this epic as the live-validation and residual checklist. Each phase's exit criterion needs the running client or the colo. One code seam was missed: `lab_timeline` never wired the #686 event ring. The remaining items are small, documented follow-ups.

#### New body

```markdown
## Problem
The live research lab (ADR `docs/architecture/live-research-lab.md`) is fully merged: client bridge, supervisor, native probes, in-server MCP endpoint, live server state, timeline, rulebook, and colo overlay. None of the phase exit criteria has been validated against a running SGW.exe or the colo, and a few documented seams remain. This epic now tracks that closeout.

## Evidence
- Merged: #692 (#684 bridge core), #696 (#685 supervisor), #706 (#686 native probes), #693 (#687 lab-mcp v1), #695 (#688 LabQuery + packet taps), #703 (#689 timeline, rulebook, compose.lab.yml).
- `crates/lab/src/timeline/client_events.rs:68` `drain_event_ring()` still returns `Vec::new()` with `TODO(#686)`. The bridge `events_read` exists (`crates/client-telemetry/src/bridge/events.rs`), and so does the proxy (`crates/lab/src/server.rs:281`).
- `crates/client-telemetry/src/bridge/dynamic_hooks/mod.rs:74-81`: thiscall/stdcall hooks and register capture are refused (they need an asm stub).
- `crates/client-telemetry/src/bridge/console.rs:19`: `client_console` degrades until the `ConsoleCommand(FString)` address is confirmed.
- `crates/lab/src/timeline/packet_tap.rs:18`: the streamable-HTTP `initialize` / `Mcp-Session-Id` handshake is unverified. The clock offset is estimated from tap timestamps (`clock.rs`), not a ping.
- ADR §10 Q1 (Lua VM alive at the login screen) is still open. The ADR status reads "Proposed".

## Acceptance criteria
Live validation (owner box or colo):
- [ ] #684: `client_lua_eval` returns a value and captured `print` output from a live client.
- [ ] #685: kill SGW.exe in Task Manager, and the agent is back in the world on the lab character, unattended. Record ADR §10 Q1 from the `client.lua.newstate` count.
- [ ] #686: re-derive the black-market window open with lab tools only, no relaunch.
- [ ] #687: spawn an NPC beside the lab character via `server_console_exec` and read the output.
- [ ] #688: Cellblock corpse AoI question answered end to end (`server_witnesses`, then the tap create packet, then a client hook).
- [ ] #689: colo session over WireGuard with non-enabled players connected; `lab_timeline` returns merged rows.

Residual code:
- [ ] Wire `drain_event_ring()` to the bridge `events_read` and map rows to `TimelineEvent::client`.
- [ ] Verify or implement the MCP HTTP `initialize` handshake in `PacketTapClient::fetch`.
- [ ] Optional: server `lab_ping` tool for the clock offset; CEGUI log lines in the event ring; thiscall/stdcall hook stub; confirm the `ConsoleCommand(FString)` address.
- [ ] Flip the ADR status to Accepted, with §10 answers recorded.

## Test type
Unit (timeline drain mapping, handshake request shape). The rest is in-game UAT.

## Docs to update
Live research lab row of the CLAUDE.md doc map: ADR status and §10, `docs/guides/live-research-lab.md`.

## Client impact
Free. Dev-only DLL feature, off by default.

## Domain advisor
game-archaeology-specialist (client probes); bigworld-engine-advisor (server state).

## Needs a human for
In-game UAT on the owner's box and colo access over WireGuard.
```

## #480 — Tier 4 client-side testing: shim-DLL test-driver IPC via UE3 ProcessEvent (extends #434, complements #281)

- Verdict: NEEDS-OWNER
- Priority: P3
- Labels: add `needs-info`
- Summary: The issue proposes adding a test-driver IPC to an `sgw-tls-shim` DLL from #434, calling UnrealScript through `UObject::ProcessEvent`. Its premise is stale. No `sgw-tls-shim` exists, and #434's login path turned out to be libcurl (agent memory `project_434_encryption_re_findings`). The injection vehicle is now `cimmeria-client-telemetry` plus `crates/client-launch`. The live research lab already delivers most of the Phase 1 plumbing #480 describes:
  - loopback JSON-RPC IPC with a token
  - main-thread marshalling from the `FEngineLoop::Tick` hook
  - memory reads, native calls with SEH guarding, and an event subscription ring
  - Lua eval, which can drive UI and NetOut handlers without UFunction marshalling

  The genuinely new parts are the `ProcessEvent` + `UProperty` parameter marshaller, a `crates/test-driver` harness paired with wireclient, and a CI/nextest profile. The lab ADR ranked "agent self-serve UAT" second and put automated client test tiers out of scope, so the owner must decide whether a Tier-4 automated test harness is still wanted, and on what base.
- Evidence:
  - No `tls-shim` anywhere in the tree (grep over `*.md`/`*.toml`/crates).
  - `crates/client-telemetry/src/bridge/` (transport, dispatch drain in `hooks/inline_hooks/engine_frame.rs`, `native_call.rs`, `events.rs`, `lua_eval.rs`).
  - `docs/architecture/live-research-lab.md` §2 row "Primary goal: RE first", and §2 "no headless wireclient bot".
  - #434 and #281 are both still OPEN.
- Related/duplicates: #690 (lab; overlaps Phase 1), #434, #281, #417

### Action text

Question for @Cadacious:

> The shim DLL this builds on was never built. Since then the live research lab (#690, all phases merged) has shipped most of Phase 1 inside the telemetry DLL: loopback IPC, main-thread marshalling from `FEngineLoop::Tick`, native calls with an SEH guard, an event ring, and Lua eval. Three options:
>
> - (a) Close as superseded by the lab. Agent-driven UAT via Lua eval is enough, and a CI-grade Tier-4 suite is not planned.
> - (b) Rewrite as a lab follow-up: a `ProcessEvent`/`UProperty` marshaller as a new `lab-bridge` method, plus a `crates/test-driver` harness paired with wireclient for a Castle Cellblock round-trip test.
> - (c) Keep it on a separate test-only DLL feature, as originally written.
>
> Recommendation: (a) now, and reopen as (b) if Lua-driven UAT proves too weak.

## #417 — Client-side observability: side-loaded telemetry DLL for full SGW.exe instrumentation

- Verdict: REWRITE
- Priority: P3
- Labels: no change
- Summary: Phases 0-5 have landed:
  - #421: Phase 1 foundation, injector, `ClientNative` variant, CI
  - #504: Phases 2-5, with 23 hooks across CME, inline, IAT, and vtable techniques
  - #620: silently-dropped inbound-method observation

  The architecture doc records what is still deferred:
  - CME RTTI auto-discovery (about 270 more `Event_NetIn/NetOut` classes)
  - the `UObject::ProcessEvent` vtable hook
  - FMOD runtime vtable traversal
  - `PropertyNode<T>` enumeration
  - Phase 6: crash filter plus artifact shipping (the minidump filter exists only in the lab-bridge build, `bridge/crash.rs`, not in the default DLL)
  - Phase 7: per-category toggles and a runbook

  The body is a 20-day plan written as if nothing had shipped, and parts of it are outdated. The inline hook library is MinHook (via `hooks/primitives`), not retour-rs. The queue is crossbeam-channel, not thingbuf. The body should shrink to the residual scope.
- Evidence:
  - `docs/architecture/client-telemetry.md:6` (status line with the deferred list) and :162-172 (phase table; Phase 6 DEFERRED).
  - `crates/client-telemetry/src/hooks/{cme_hooks,iat_hooks,vtable_hooks}.rs` and `hooks/inline_hooks/*`.
  - `crates/client-telemetry/src/lib.rs:52` (`bridge` only under `lab-bridge`, which includes `crash.rs`).
  - PRs #421, #504, #620.
  - `docs/reverse-engineering/findings/client-instrumentation-hookpoints.md` exists (Phase 0).
- Related/duplicates: #690 (lab reuses this DLL), #480

### Action text

Comment:

> Phases 0-5 landed in PRs #421, #504, and #620, with 23 hooks live and the queue, uploader, injector, and `ClientNative` replay all in place (see `docs/architecture/client-telemetry.md`). I'm rewriting the body to the scope that is still deferred, so this stops reading as an unstarted 20-day plan. Two notes. The lab's outer-tier crash filter (`bridge/crash.rs`) exists only in `lab-bridge` builds, so Phase 6 for the default DLL can reuse it. The stack picks in the old body (retour-rs, thingbuf) were superseded during implementation, by MinHook-style primitives and crossbeam-channel.

#### New body

```markdown
## Problem
The client telemetry DLL (`crates/client-telemetry`) ships 23 read-only hooks into SigNoz under `service_name = cimmeria-client` (Phases 0-5, PRs #421/#504/#620). The remaining instrumentation from the original plan is deferred, and the default (non-lab) DLL has no crash capture.

## Evidence
- `docs/architecture/client-telemetry.md` status line and phase table: CME RTTI auto-discovery, the `UObject::ProcessEvent` vtable hook, FMOD traversal, and `PropertyNode<T>` are deferred. Phase 6 is DEFERRED.
- The outer-tier crash filter (`SetUnhandledExceptionFilter`, then `MiniDumpWriteDump`, then terminate) exists in `crates/client-telemetry/src/bridge/crash.rs`, but only under `--features lab-bridge` (`src/lib.rs:52`).
- Addresses and IAT slots are pre-resolved in `docs/reverse-engineering/findings/client-instrumentation-entry-points.md`.

## Acceptance criteria
- [ ] Phase 6: the default DLL installs an unhandled-exception filter that writes a minidump and emits a correlated `client.crash` event. It reuses or moves `bridge/crash.rs` out of the lab-only module. `CrashDumps/*.dmp` and `SGWDebugLog.log` are tail-shipped (opt-in; dumps may hold PII).
- [ ] CME RTTI auto-discovery: scan the `.rdata` TypedEmitInfo descriptors at load and subscribe all `Event_NetIn_*` / `Event_NetOut_*` classes (process-lifetime subscribers).
- [ ] `UObject::ProcessEvent` vtable hook with an FName-integer allowlist and a re-entry guard; sampled.
- [ ] Optional: FMOD event correlators; `PropertyNode<T>` get/set.
- [ ] Phase 7: per-category enable toggles; runbook section in `docs/operations/telemetry.md`.

## Test type
Unit (RTTI scanner over a captured `.rdata` fixture, allowlist filter, crash-marker shape). In-game UAT for the hooks.

## Docs to update
Dev-session telemetry row and the client-telemetry doc (`docs/architecture/client-telemetry.md` phase table), `docs/operations/telemetry.md`, `docs/architecture/observability.md` target catalog.

## Client impact
Free (injected DLL, read-only hooks, no SGW.exe patch).

## Domain advisor
game-archaeology-specialist.

## Needs a human for
In-game UAT on the owner's box; the PII decision on crash-dump shipping.
```

## #393 — MCP server for the in-game Atrea editor (UnrealEd) — full implementation path

- Verdict: NEEDS-OWNER
- Priority: P3
- Labels: add `needs-info`
- Summary: Nothing is implemented. There is no `CimmeriaEditorBridge.dll` and no editor-bridge MCP, and the ADR `docs/architecture/atrea-editor-bridge.md` is still "Proposed, awaiting sign-off on §5 open questions". Since the issue was written, the live research lab has shipped the same bridge shape: framed JSON-RPC over TCP, a token, the `FEngineLoop::Tick` drain, a microseh guard, supervisor screenshots, and the shared `client-launch` injector. Lab ADR §10 Q4 says the Atrea bridge should adopt the lab transport. The body has three stale or conflicting claims:
  - It picks `127.0.0.1:8765`, which the SigNoz MCP now uses (lab ADR §3.3).
  - It injects via `AtreaLoader.config.xml` alongside `AtreaRL.dll`. That conflicts with the standing preference that Atera/AtreaRL are RE references only, and our own launcher/lab now inject from scratch.
  - It plans a separate `CimmeriaEditorBridge.dll` and a Python shim, while the lab offers a Rust DLL and a supervisor that could host editor tools.
- Evidence:
  - `docs/architecture/atrea-editor-bridge.md:6` (Proposed) and :12,54,214,216 (AtreaRL + 8765).
  - `docs/architecture/live-research-lab.md:94` (8765 taken; share framing) and §10 Q4.
  - No editor-bridge code under `crates/` or `tools/`.
  - `crates/client-launch/src/inject.rs`.
- Related/duplicates: #690

### Action text

Question for @Cadacious:

> The ADR's §5 questions were never signed off, and the live research lab has since built the same bridge. Decide:
>
> 1. Should the editor bridge become a lab-bridge target? That means editor methods such as `editor_exec` in the telemetry DLL, gated on `GIsEditor`, plus editor tools on `cimmeria-lab`, with launch via `client-launch`. The alternative is a separate DLL as in the ADR.
> 2. May editor mode still depend on AtreaRL's `GIsEditor` patches, or must that activation be re-authored from scratch per the no-Atera preference?
> 3. Confirm a new port (8765 is taken by SigNoz MCP).
>
> Once decided, I'll rewrite this issue and ADR §3 to match.

## #75 — CI/CD: Azure Pipelines patch publishing pipeline for blob storage

- Verdict: CLOSE (not planned)
- Priority: P3
- Labels: no change
- Summary: The issue specifies an Azure DevOps pipeline with OIDC that uploads patch zips to Azure Blob and upserts a blob-hosted `manifest.json`. The distribution design has since changed. PR #343 replaced the Tauri launcher with egui, and the content manifest is now served from the GitHub Release tag `content-current` with Ed25519 signing. The signing private key is deliberately kept offline, with only the public key in a GitHub Actions secret. Azure Blob remains only for debug-log uploads, and the repo uses GitHub Actions, not Azure Pipelines. A CI job that signs and publishes manifests would contradict the offline-key decision. Publishing is a documented manual procedure.
- Evidence:
  - `crates/launcher/src/config.rs:12-13` (`DEFAULT_MANIFEST_URL` = GitHub Releases `content-current`).
  - `crates/launcher/src/manifest.rs:1-41` (Ed25519 verify).
  - `docs/client/launcher-distribution-setup.md:16-17,43,86-124,156` (private key offline; `gh release upload content-current`).
  - `docs/client/launcher-guide.md:182-186` ("no automation yet; manual procedure").
  - `.github/workflows/` has launcher build/release workflows and no patch-publish workflow. There is no `azure-pipelines/` directory.
- Related/duplicates: #74

### Action text

Closing as not planned. This design is superseded. Since PR #343 the launcher reads a signed manifest from the GitHub Release `content-current` (`crates/launcher/src/config.rs:12`), not from Azure Blob. The Ed25519 signing key is kept offline by design (`docs/client/launcher-distribution-setup.md`), and a pipeline that signs manifests in CI would contradict that. Azure Blob now serves only debug-log uploads, and the project's CI is GitHub Actions. The manual publish procedure is in `docs/client/launcher-guide.md` Part 2. If we later want a helper that builds a manifest locally and verifies hashes (leaving signing offline), that belongs in a new, narrower issue.

## #74 — Refactor sgw-launcher for Azure blob patch delivery + fix atomicity issues

- Verdict: CLOSE (completed)
- Priority: P3
- Labels: no change
- Summary: The issue plans a refactor of the old Tauri launcher (`commands.rs`, `ui/main.js`, `updater.rs`). Its own comments report steps 1-5 done in April. PR #343 then replaced that launcher entirely with an egui app that includes each atomicity fix:
  - a seed plus patch manifest with an applied-patch record in `launcher-installed.json`
  - Range-resumable downloads with SHA-256 verification
  - a zip-slip gate via `enclosed_name()`
  - an atomic `.rdata` patch written to a temp file and then renamed, now in `crates/client-launch`

  Distribution later moved to GitHub Releases with a signed manifest. None of the Tauri-era files the issue names still exist.
- Evidence:
  - `crates/launcher/src/{install,manifest,state,config}.rs`.
  - `crates/launcher/src/install.rs:339-348` (zip-slip gate) and :107-119 (applied patches).
  - `crates/client-launch/src/patch_rdata.rs:146-160` (temp `.exe.patching`, then rename).
  - PR #343 (merged 2026-05-23); PR #548 (launcher split).
- Related/duplicates: #75

### Action text

Closing as completed and superseded. The Tauri launcher this refactor targeted was replaced by the egui launcher in PR #343. The replacement has every goal of this issue:

- a seed plus patch manifest, with applied state in `launcher-installed.json`
- resumable downloads with SHA-256 verification
- a zip-slip gate (`crates/launcher/src/install.rs:339`)
- an atomic `.rdata` patch written to a temp file and renamed (`crates/client-launch/src/patch_rdata.rs:146-160`)

Distribution has since moved to a signed manifest on GitHub Releases (`docs/client/launcher-distribution-setup.md`).

## #25 — Admin API: Implement JWT authentication middleware

- Verdict: REWRITE
- Priority: P1
- Labels: add `security`
- Summary: Still entirely unimplemented. `middleware.rs` has only the CORS layer (`allow_origin(Any)`) plus a TODO block, and the three `/api/auth/*` handlers return "not implemented". The line numbers have drifted slightly: `auth.rs` is now :41/:59/:78, `middleware.rs` :19-28. The body omits why this matters now:
  - #439: the binary defaults `ADMIN_BIND` to loopback (PR #724), but the container sets `ADMIN_BIND=0.0.0.0`, and `docker/compose.yml:64` publishes `8443:8443` on the colo.
  - Anything on the internet can therefore reach `POST /api/config/start|stop`, and `POST`/`DELETE /api/editor/content`, which write to `content_chains`.
  - `jsonwebtoken` is declared but has zero call sites (agent memory `network-security-auth/admin-api-jsonwebtoken-unused.md`).
  - The launcher routes on the same listener (`/api/auth/dev-session`, `/api/telemetry/*`) already use their own HMAC token and must stay exempt or keep that scheme.
  - Both admin UIs send no credentials today: the Tauri admin panel (`frontend/`) and the ContentEditor hot-reload (`tools/ContentEditor/src/commands/hot_reload.rs`).
- Evidence:
  - `crates/admin-api/src/middleware.rs:12-28`; `crates/admin-api/src/routes/auth.rs:41,59,78`.
  - `crates/admin-api/src/routes/config.rs:60-63` (start/stop); `crates/admin-api/src/routes/editor.rs:351-527` (DB writes).
  - `crates/server/src/main.rs:21` (`ADMIN_BIND` row); `docker/Dockerfile:270-275`; `docker/compose.yml:64`.
  - `docs/tools/admin-api.md:445-452`.
- Related/duplicates: #439 (the exposure itself, OPEN, security batch), #26-#30 (all write endpoints should land behind this)

### Action text

Comment:

> Still open and now more urgent. The binary defaults to loopback since #724, but the container image binds `0.0.0.0` and `docker/compose.yml:64` publishes 8443 on the colo. So `POST /api/config/stop` and the `/api/editor/content` DB writes are reachable without auth (#439). I'm rewriting the body with current line numbers, the routes that must stay on the launcher's HMAC scheme, and the two clients that must start sending a token.

#### New body

```markdown
## Problem
Every `/api` route on the admin API is unauthenticated, and CORS is `allow_origin(Any)`. On the colo the port is public: the container sets `ADMIN_BIND=0.0.0.0` and `docker/compose.yml:64` publishes 8443. This is the code-side fix for #439.

## Evidence
- `crates/admin-api/src/middleware.rs:19-28`: JWT middleware is a TODO comment. `routes/auth.rs:41,59,78`: login, logout, and whoami return "not implemented".
- State-changing routes open today: `POST /api/config/start|stop` (`routes/config.rs:62-63`), `POST /api/config`, `POST`/`DELETE /api/editor/content` (write `content_chains`, `routes/editor.rs:351-527`), `POST /api/content/reload`, `/api/telemetry/*`.
- `jsonwebtoken` is declared in `crates/admin-api/Cargo.toml:24` with zero call sites.
- `/api/auth/dev-session` and `/api/telemetry/*` already use a hand-rolled HMAC token (`routes/dev_session/token.rs`) that the launcher depends on.
- Clients that will need a token: the admin panel (`frontend/src/lib/admin-api.ts`, `ws.ts`) and ContentEditor hot reload (`tools/ContentEditor/src/commands/hot_reload.rs`).

## Acceptance criteria
- A tower middleware on every `/api` and `/ws` route except `/api/auth/login`, `/api/auth/dev-session`, and `/api/telemetry/*`, which keep their HMAC scheme. It returns 401 for a missing, invalid, or expired bearer token.
- `POST /api/auth/login` verifies admin credentials (argon2id hash, source to be decided: env or DB `accounts` with an admin access level) and issues a short-lived JWT with a pinned algorithm. `logout` adds the token to a denylist and `whoami` decodes it.
- CORS is restricted to the Tauri origin and the dev server origin.
- The admin panel and ContentEditor send the token. WebSockets authenticate on upgrade.
- Fail closed: if no signing secret is configured, protected routes refuse rather than run open.

## Test type
Unit (middleware accept/reject matrix, algorithm pinning, expiry, denylist). A negative-log guard on 401. A regression guard that `POST /api/config/stop` without a token returns 401. The guard must fail if the layer is removed.

## Docs to update
Admin-API row (`docs/tools/admin-api.md` auth and status tables), `crates/server/src/main.rs` env rows for the new secret, `docs/operations/colo-deploy.md`.

## Client impact
Free (server and dev tools only; the game client is untouched).

## Domain advisor
network-security-auth.

## Needs a human for
The decision on where admin credentials come from; the colo secret provisioning.
```

## #26 — Admin API: Implement entity management endpoints

- Verdict: REWRITE
- Priority: P3
- Labels: no change
- Summary: All three handlers are still stubs, at the same lines (`entities.rs:38,62,86`). The premise is wrong, though. There is no shared `EntityManager`: live entities belong to the cell loop's `SpaceManager` and can only be read by message passing. That plumbing now exists. The lab's `BaseToCellMsg::LabQuery` (PR #695) answers `EntityGet`, `EntityQuery` (capped at 256), and `Witnesses`, and admin-api holds the same `Orchestrator` that `lab-mcp`'s `LabState::lab_query` uses. So list and get are a thin adapter. The "set property" item is a new cell write path, and it must not ship before #25 on a colo-published port.
- Evidence:
  - `crates/admin-api/src/routes/entities.rs:38,62,86`.
  - `crates/services/src/cell/messages/lab.rs:48-56` (`LabQuery`).
  - `crates/lab-mcp/src/state.rs:69-77` (reference adapter).
  - `docs/tools/admin-api.md:311-319`.
- Related/duplicates: #25 (gate), #27, #688

### Action text

Comment:

> The stubs are unchanged, but the body's assumption of a shared `EntityManager` is stale. Cell entities are loop-owned, and the lab (PR #695) added the read path: `BaseToCellMsg::LabQuery`. I'm rewriting the body so the list and get endpoints reuse it, and so the property-write endpoint waits for #25.

#### New body

```markdown
## Problem
`GET /api/entities`, `GET /api/entities/{id}`, and `PUT /api/entities/{id}/property` are stubs (`crates/admin-api/src/routes/entities.rs:38,62,86`).

## Evidence
- There is no shared entity manager. `SpaceManager` is owned by the cell loop. The read path already exists as `BaseToCellMsg::LabQuery { EntityGet | EntityQuery | Witnesses }` (`crates/services/src/cell/messages/lab.rs:48`), answered between ticks and capped at `LAB_ENTITY_QUERY_CAP` (256).
- `crates/lab-mcp/src/state.rs:69` shows the adapter over `Orchestrator`. Admin-api holds the same `Arc<Orchestrator>`.

## Acceptance criteria
- `GET /api/entities` (filters: space, template, class) and `GET /api/entities/{id}` return `LabEntitySnapshot` JSON through `LabQuery`, with `available: false` plus a reason when the cell is down. They follow the existing degraded-mode convention.
- The property-write endpoint is either dropped or specified as a new cell message with an allowlist of settable properties. It must not land before #25.

## Test type
Unit (handler maps reply and degraded mode). A cell-loop request/reply test only if a new write variant is added.

## Docs to update
Admin-API row (`docs/tools/admin-api.md`).

## Client impact
Free.

## Domain advisor
bigworld-engine-advisor.

## Needs a human for
Nothing for the reads. The owner decides whether the property write is wanted.
```

## #27 — Admin API: Implement WebSocket entity property stream

- Verdict: NEEDS-OWNER
- Priority: P3
- Labels: add `needs-info`
- Summary: `/ws/entities` still accepts the upgrade and does nothing (`entity_stream.rs:26-31`). There is no property-change event bus to subscribe to. Building one means a new fan-out from the cell tick, which is a real design task. The admin panel has no page that would consume it. The debugging use it was meant for is now covered by other tools: the lab's `server_entity_get`, `server_witnesses`, and packet taps, and the SigNoz `wire.out` stream.
- Evidence:
  - `crates/admin-api/src/ws/entity_stream.rs:26-31`.
  - `frontend/src/` has no `/ws/entities` consumer (`ws.ts` users: `/ws/logs`, `/ws/events`).
  - `crates/lab-mcp/src/tools/{entities,witnesses,packet_tap}.rs`.
  - `docs/tools/admin-api.md:347`.
- Related/duplicates: #26, #688

### Action text

Question for @Cadacious:

> `/ws/entities` is still a stub, and nothing in the admin panel would consume it. The live-entity debugging it targeted is now served by the lab tools (`server_entity_get`, `server_witnesses`, packet taps) and by SigNoz `wire.out`. Is a player-facing-port WebSocket stream of entity properties still wanted? If not, I suggest closing this and deleting the stub route (one less unauthenticated surface under #439). If yes, the first step is a design for a cell-side property-change fan-out. None exists today.

## #28 — Admin API: Implement player management endpoints

- Verdict: REWRITE
- Priority: P3
- Labels: no change
- Summary: The handlers are still stubs: player detail at `players.rs:107`, kick at `:131`. The body misses that the roster itself is also a stub (`list_players` returns `available: false`, "Live player roster is not implemented yet", `players.rs:76`), even though `BaseService::online_players()` exists and `lab-mcp`'s `server_sessions` already uses it through the same `Orchestrator`. No kick or disconnect primitive exists anywhere server-side. A kick endpoint is a state-changing admin action on a port the colo publishes, so it must wait for #25.
- Evidence:
  - `crates/admin-api/src/routes/players.rs:76,107,131`.
  - `crates/services/src/base/service.rs:90` (`online_players`).
  - `crates/lab-mcp/src/tools/sessions.rs:9-25`.
  - `frontend/src/pages/Players.tsx:176` (placeholder text).
  - `docs/tools/admin-api.md:131-185`.
- Related/duplicates: #25 (gate for kick)

### Action text

Comment:

> Rewriting the body. The roster endpoint is also a stub, and it is now trivial to fill: `BaseService::online_players()` already backs the lab's `server_sessions`. Kick needs a new base-side disconnect path, and it should land behind #25 because the admin port is public on the colo.

#### New body

```markdown
## Problem
`GET /api/players` always returns `available: false` with an empty roster (`crates/admin-api/src/routes/players.rs:76`). `GET /api/players/{id}` (:107) and `POST /api/players/{id}/kick` (:131) are stubs. The admin panel Players page shows placeholders (`frontend/src/pages/Players.tsx:176`).

## Evidence
- `BaseService::online_players()` (`crates/services/src/base/service.rs:90`) exists and is already used through `Orchestrator` by `crates/lab-mcp/src/tools/sessions.rs`.
- No server-side kick or disconnect primitive exists.

## Acceptance criteria
- The roster is populated from `online_players()`: entity id, account, character, space, and address, with `available: true`.
- Player detail joins the live session with account and character rows.
- Kick: a base-side disconnect that cleanly tears down the session and the cell entity (AoI leave to witnesses, position persisted). Only behind #25 auth, with an audit log event.

## Test type
Unit (roster mapping). Kick: a Mercury session test showing the channel closes and a live-DB test showing the position persists on kick. The guard must fail if the teardown is skipped.

## Docs to update
Admin-API row; `docs/tools/admin-panel.md` if the Players page changes. Frontend changes need the REPL-style logic UAT (AGENTS.md).

## Client impact
Free.

## Domain advisor
network-security-auth (kick lifecycle); aoi-witness-broadcast (leave fan-out on kick).

## Needs a human for
Nothing for the roster. Kick waits on #25.
```

## #29 — Admin API: Implement content, space, and config endpoints

- Verdict: REWRITE
- Priority: P3
- Labels: no change
- Summary: Every listed TODO is still present, with drifted lines: `content.rs:73,91,326-329,349`, `spaces.rs:214`, `config.rs:105`. Two of the seven are dead or undefined:
  - The `regions/items/dialogs` fields belong to `get_editor_pickers` (`/api/content/pickers`). Its consumer, the admin-panel chain editor, was deleted in commit 62a6c0ee ("Phase 5: Remove chain editor from admin panel"), and nothing calls `/pickers` now.
  - `POST /api/spaces` ("delegate to CellService") and `POST /api/config` ("apply where possible") have no defined semantics, and both would be state-changing on a publicly published port.

  The worthwhile remainder is the read-only content catalog: category listing, the items list, and item-by-id. The items and dialogs tables exist in the `db/resources` seeds.
- Evidence:
  - `crates/admin-api/src/routes/content.rs:53` (`/pickers` route), :256-331, :73, :91, :349.
  - `spaces.rs:214`; `config.rs:105`.
  - No `/pickers` consumer in `frontend/src` or `tools/*/ui/src`.
  - `db/resources/Items/Tables/items.sql`, `db/resources/Dialogs/Tables/dialogs.sql`.
  - `git show 62a6c0ee8`.
- Related/duplicates: #30 (same orphaned editor surface), #25

### Action text

Comment:

> Rewriting to the part that is still meaningful. The `regions/items/dialogs` TODOs sit in `/api/content/pickers`, whose only consumer, the admin-panel chain editor, was removed in 62a6c0ee. `POST /api/spaces` and `POST /api/config` never had defined semantics, and both would be unauthenticated state changes on the colo-published port (#439). I've scoped this to the read-only content catalog and proposed deleting the dead stubs. Veto if you want runtime space creation or config apply kept.

#### New body

```markdown
## Problem
The admin API's content catalog endpoints are stubs: `GET /api/content` (`crates/admin-api/src/routes/content.rs:73`), `GET /api/content/items` (:91), and `GET /api/content/items/{id}` (:349). Several other stubs have no consumer or no defined behavior.

## Evidence
- Items and dialogs are seeded tables: `db/resources/Items/Tables/items.sql`, `db/resources/Dialogs/Tables/dialogs.sql`.
- `/api/content/pickers` (`content.rs:53,256-331`) served the admin-panel chain editor, which was removed in 62a6c0ee. It has no caller in `frontend/` or `tools/`.
- `POST /api/spaces` (`spaces.rs:214`) and `POST /api/config` (`config.rs:105`) are stubs with no spec.

## Acceptance criteria
- `GET /api/content` returns real per-category counts.
- `GET /api/content/items` is paginated and `GET /api/content/items/{id}` reads the items table, both with the degraded-mode `available/reason` convention.
- Delete `/api/content/pickers`, `POST /api/spaces`, and `POST /api/config` together with their docs rows, unless the owner specifies them.

## Test type
Live-DB test for the items queries (sentinel item id, exact-id cleanup). Unit test for the degraded mode.

## Docs to update
Admin-API row (`docs/tools/admin-api.md` status table).

## Client impact
Free.

## Domain advisor
items-systems-advisor; database-persistence.

## Needs a human for
Owner veto on deleting the space and config POST stubs.
```

## #30 — Admin API: Implement editor draft storage and mission filtering

- Verdict: CLOSE (not planned)
- Priority: P3
- Labels: no change
- Summary: The TODOs remain (`editor.rs:324,552,568,587,603`), but the whole `/api/editor/*` surface is orphaned. Its client, the admin-panel chain editor, was deleted in commit 62a6c0ee. Chain editing now lives in the standalone ContentEditor (Tauri), which reads and writes the DB directly through Tauri commands (`save_chains`, `export_to_seed_file`) and calls the admin API only for `/api/content/reload`. No code calls `/api/editor/*`. Worse, `POST`/`DELETE /api/editor/content` do unauthenticated `INSERT`/`DELETE` on `content_chains` over the colo-published port 8443. Finishing drafts would add surface for a client that no longer exists. The right follow-up is deleting the routes, which belongs with #439/#25.
- Evidence:
  - `crates/admin-api/src/routes/editor.rs:136-147` (routes), :351-527 (DB writes), :552-604 (stubs).
  - `tools/ContentEditor/src/main.rs:24-61` (Tauri save commands).
  - `tools/ContentEditor/src/commands/hot_reload.rs:6` (only admin-API call).
  - No `/api/editor` string in `frontend/src` or `tools/*/ui/src`.
  - `docker/compose.yml:64`.
- Related/duplicates: #439, #25, #29, #18

### Action text

Closing as not planned. The admin-panel chain editor that used `/api/editor/*` was removed in 62a6c0ee. The ContentEditor desktop app now saves chains and exports seeds through its own Tauri commands and never calls these routes. Finishing draft storage here would serve no client. Note for #439: `POST`/`DELETE /api/editor/content` still perform unauthenticated writes to `content_chains` on port 8443, which the colo publishes. Those routes, together with the dead `/api/content/pickers`, should be deleted as part of #439/#25.

## #16 — Frontend: Add multi-select and bulk actions to the chain editor

- Verdict: REWRITE
- Priority: P3
- Labels: no change
- Summary: The touchpoints are gone. `frontend/src/react/ChainFlowWorkbench.react.tsx` (4,805 lines) was deleted in commit 62a6c0ee, and the chain editor now lives at `tools/ContentEditor/ui/src/editors/ChainEditor.tsx` (208 lines). The need is partly still real. The new editor uses React Flow defaults, so box select, modifier multi-select, dragging the selection, and Backspace delete probably work without any code. However, `onSelectionChange` passes only `selected[0]` to the inspector, and there is no undo/redo anywhere, no bulk duplicate/recolor/assign-to-chain, and no selection summary. The acceptance criterion of "a single coherent undo step" needs an undo system that does not exist.
- Evidence:
  - `tools/ContentEditor/ui/src/editors/ChainEditor.tsx:135-139` (`selected[0]` only), :163-180 (ReactFlow props: no selection config, no undo).
  - `git show --stat 62a6c0ee8`.
  - No `undo` in `ChainEditor.tsx` or `AppLayout.tsx`.
- Related/duplicates: #18

### Action text

Comment:

> The admin-panel `ChainFlowWorkbench` this targeted was removed in 62a6c0ee. Chain editing now lives in the ContentEditor app (`tools/ContentEditor/ui/src/editors/ChainEditor.tsx`). React Flow's defaults may already cover basic multi-select, move, and delete. What is missing is a multi-selection inspector, bulk duplicate and assign, and undo/redo, which does not exist at all yet. Rewriting the body against the current editor.

#### New body

```markdown
## Problem
The ContentEditor chain editor supports editing only one node at a time. The inspector receives only the first selected node, and there is no undo/redo, so bulk edits are slow and risky.

## Evidence
- `tools/ContentEditor/ui/src/editors/ChainEditor.tsx:135-139`: `onSelectionChange` forwards `selected[0]` only.
- `ChainEditor.tsx:163-180`: stock `<ReactFlow>` with no `selectionOnDrag`, `multiSelectionKeyCode`, or `deleteKeyCode` configuration and no history. React Flow defaults (shift-drag box select, ctrl/meta-click, drag the selection, Backspace delete) may already work. Verify first.
- The previous implementation (`frontend/src/react/ChainFlowWorkbench.react.tsx`) was deleted in 62a6c0ee.

## Acceptance criteria
- Box and modifier multi-select work and are visually obvious. The inspector shows a selection summary (count by node type) when more than one node is selected.
- Bulk move, delete, duplicate, and assign-to-chain on the selection.
- An undo/redo history (for example, snapshotting nodes and edges) in which a bulk operation is one step.
- Invalid bulk actions (for example, deleting across chains that would orphan a sequence) are blocked with visible feedback on the first press.

## Test type
Frontend unit tests for the selection-state and history reducers, plus the REPL-style logic UAT required by AGENTS.md.

## Docs to update
`docs/tools/` content-editor doc, if one covers the chain editor.

## Client impact
Free (dev tool only).

## Domain advisor
mission-systems-advisor (chain semantics); react-flow skills.

## Needs a human for
Nothing.
```

## #18 — Frontend: Add publish, review, diff, and rollback workflow for content changes

- Verdict: CLOSE (not planned)
- Priority: P3
- Labels: no change
- Summary: The issue proposes an in-app revision model (draft, review, approved, published, rolled back) with diff, audit metadata, and publish/rollback against live content. That conflicts with a standing project rule: the seeds in `db/resources/` are the source of truth, and the colo DB is rebuilt from the seed on every deploy. Publishing therefore already means exporting to the seed and merging a PR, and git plus PR review already provide diff, review, audit (author and timestamp), and rollback. The ContentEditor already has the export half: `export_to_seed_file` writes `<scope>_chains.sql` into the seed directory and wires it into `database.sql`. The listed touchpoints are stale: `ChainFlowWorkbench` was deleted in 62a6c0ee, and `docs/architecture/tauri-rewrite.md` was not checked. A parallel revision store in the DB would be wiped by the next deploy and would duplicate git.
- Evidence:
  - CLAUDE.md "Seeds are the source of truth".
  - Agent memory `reference_colo_db_refreshes_on_deploy`.
  - `tools/ContentEditor/src/commands/seed_export.rs:8-201` (`export_to_seed_file`, auto-wires `database.sql`).
  - `git show --stat 62a6c0ee8`.
- Related/duplicates: #16, #30

### Action text

Closing as not planned. Content "publish" in this project is a seed change: `db/resources/` is the source of truth, and the colo DB is rebuilt from the seed on every deploy. The ContentEditor already exports chains into the seed tree (`tools/ContentEditor/src/commands/seed_export.rs`), and git with PR review supplies the diff, review, audit trail, and rollback this issue asked for. A revision and publish store inside the DB would be discarded on the next deploy and would duplicate git. A narrower follow-up would fit the seed workflow and could get its own issue: an in-editor diff of the working DB against the committed seed before export. (@aurablacklight, reopen if you had a case in mind that the seed and PR flow can't cover.)

## Batch summary

| # | verdict | priority | one-line reason |
|---|---|---|---|
| 690 | REWRITE | P3 | All 6 phases merged (no closing refs); rewrite as live-validation and residual checklist (unwired `drain_event_ring`, handshake, ADR status) |
| 689 | CLOSE (completed) | P3 | PR #703 shipped timeline, rulebook, and compose.lab.yml; `drain_event_ring` seam carried to #690 |
| 688 | CLOSE (completed) | P3 | PR #695 shipped LabQuery, witness tools, and packet taps; live AoI check carried to #690 |
| 687 | CLOSE (completed) | P3 | PR #693 shipped fail-closed lab-mcp and six tools; live exit criterion carried to #690 |
| 686 | CLOSE (completed) | P3 | PR #706 shipped native probes; cdecl-only and console-address limits carried to #690 |
| 685 | CLOSE (completed) | P3 | PR #696 shipped supervisor, autologin, and crash recovery; live exit criterion carried to #690 |
| 684 | CLOSE (completed) | P3 | PR #692 shipped the bridge core; Lua capture landed in #706 |
| 480 | NEEDS-OWNER | P3 | tls-shim base never built; lab covers most of Phase 1. Close as superseded or rewrite as lab follow-up? |
| 417 | REWRITE | P3 | Phases 0-5 landed (#421/#504/#620); shrink to the deferred RTTI/ProcessEvent/crash/toggles scope |
| 393 | NEEDS-OWNER | P3 | Unbuilt; should it fold into the lab bridge, may it depend on AtreaRL, and it needs a new port (8765 taken)? |
| 75 | CLOSE (not planned) | P3 | Azure Pipelines/blob design superseded by the GitHub Releases signed manifest with offline key |
| 74 | CLOSE (completed) | P3 | Tauri launcher replaced by egui launcher (#343), which has every atomicity fix |
| 30 | CLOSE (not planned) | P3 | `/api/editor/*` has no client since 62a6c0ee; its routes should be deleted under #439 |
| 29 | REWRITE | P3 | Only the read-only item catalog is meaningful; pickers are orphaned, space and config POST have no spec |
| 28 | REWRITE | P3 | Roster also a stub but `online_players()` exists (used by lab-mcp); kick needs #25 first |
| 27 | NEEDS-OWNER | P3 | No property event bus and no consumer; lab tools cover the use case. Drop it? |
| 26 | REWRITE | P3 | No `EntityManager`; reuse `LabQuery` for list and get; property write gated on #25 |
| 25 | REWRITE | P1 | Still unimplemented while colo publishes 8443 with unauthenticated start/stop and DB-write routes (#439) |
| 18 | CLOSE (not planned) | P3 | In-DB publish workflow conflicts with seeds-as-truth; seed export plus PR already covers it |
| 16 | REWRITE | P3 | Touchpoints deleted; the new ChainEditor lacks a multi-select inspector, bulk ops, and undo |
