# ADR: Live Research Lab — MCP access to the running client and server

> **Last updated**: 2026-09-19
> **Audience**: Engineers and AI agents (Claude Code) doing reverse engineering and live verification against a running SGW.exe and a running `cimmeria-server`
> **Type**: Architecture decision record
> **Status**: Proposed — owner decisions in §2 are settled; spikes in §8 gate Phases 2 and 3
> **Owner**: Reverse-engineering / tools
> **Companion docs**: [atrea-editor-bridge.md](atrea-editor-bridge.md) (same bridge shape, editor target), [client-telemetry.md](client-telemetry.md) (the injected DLL this extends), [observability.md](observability.md) (the shared timeline), [../guides/sgw-live-debugging.md](../guides/sgw-live-debugging.md) (the x32dbg workflow this mostly replaces), [../reverse-engineering/findings/black-market-client-window-patch.md](../reverse-engineering/findings/black-market-client-window-patch.md) (the main-thread Lua primitive)

## TL;DR

Give the agent hands, eyes, and a body in a live game session, so the research rule becomes: **do not infer client behavior when the running client can answer the question.**

Three pieces:

1. **Client bridge.** An inbound command channel added to the already-injected `cimmeria-client-telemetry` DLL, behind a `lab-bridge` cargo feature. It evaluates Lua on the client main thread, reads and writes memory, installs non-freezing logging hooks at any address, and calls native functions. Lua eval is the hot-loadable probe layer: new client probes need no rebuild.
2. **Lab supervisor** (`cimmeria-lab`). A stdio MCP server on the dev box. It launches, injects, logs in, watches, screenshots, and restarts SGW.exe, and proxies tool calls to the bridge. It outlives client crashes, which is what makes unattended recovery possible.
3. **Server lab endpoint** (`cimmeria-lab-mcp`). An MCP endpoint inside `cimmeria-server` with a fixed tool set: dot-console passthrough with captured output, live entity and witness queries, per-session packet taps, log tail, read-only SQL. Token-gated, bound only to an address the operator names, reachable on the colo over WireGuard only.

Sequencing is RE first: the client track (Phases 1 to 3) lands before the server track (Phases 4 and 5), though the two tracks are independent and can run in parallel.

## 1. Why this exists

The idea comes from a Star Trek: Bridge Commander mod developer whose tooling gives an agent direct access to a running stock game: hot-reloaded script probes, a two-way IPC relay so the agent can add its own in-game handlers, injected native instrumentation, one merged trace timeline, screenshots, and automated build, deploy, and restart. The key property is that the agent is not limited to endpoints defined up front.

Cimmeria already has most of the hard parts, but they do not connect into a loop:

| Capability | BCMM | Cimmeria today | Gap |
|---|---|---|---|
| Hot-loadable script probes | Python 1.5 hot reload | Main-thread Lua eval proven by the black-market patch | No on-demand channel; the chunk is baked into a launch-time patch |
| Two-way IPC into the game | Named-pipe relay | Telemetry DLL is **emit-only** (`crates/client-telemetry/src/session.rs`) | No inbound channel at all |
| Injected native instrumentation | C# EasyHook trampolines | 23 hooks, four techniques, reusable primitives under `hooks/primitives/` | Hooks are compiled in; none installable at runtime |
| Single timeline | Script stdout merged into the native trace | Client and server events both land in SigNoz under one dev-session id | Pivot is server-receive time, a caveat [observability.md](observability.md) already flags |
| Runtime inspection | WinDbg, x32dbg, CE | x64dbg MCP | Breakpoints must be non-freezing or the heartbeat dies and the server disconnects |
| Eyes | DX11 mirror, screenshots | None | No capture tooling anywhere |
| Lifecycle | Build, deploy, restart, known PID | Launcher knows the PID (`telemetry/process_watch.rs`) | No stop, restart, or autologin |
| Server control | n/a (single-player game) | 88 dot-console commands, in-process admin API | Console reachable only from an in-world GM's chat; live `SpaceManager` state is owned by the cell loop and unreadable from outside |

A server is the one thing BCMM does not need and we do. Half of every SGW question is "what did the server send, and what did the client do with it", so the lab has to reach both ends.

## 2. Owner decisions (interview, 2026-09-19)

| Question | Decision |
|---|---|
| Primary goal | Both RE-by-experiment and agent self-serve UAT. **RE first.** |
| Reach | Server **and** client. |
| Extensibility | **Lua eval relay on the client; fixed tools on the server.** No embedded server scripting engine. New server probes are Rust edits, and the owner runs the build. |
| Environment | Full tool power locally and on the colo. The owner is the only telemetry-enabled client, and needs the lab active while other, non-enabled players are connected. |
| Client trust boundary | The agent runs on the owner's dev box with line of sight to the client. **No server-to-client command relay exists.** Other players' clients are unreachable by construction. |
| Colo gating | Shared token, all tools available, log lines as the audit trail. The port is reachable over WireGuard only, never the public internet. |
| Agent body | **Drive the real client.** No bodiless GM principal and no headless wireclient bot in this ADR. |
| Eyes | Screenshot by PID from the supervisor. No render-path hooks. |
| Native reach | Memory read, on-demand logging hooks, **and** memory write and native calls. Fastest loop wins; crash recovery (§6) pays for it. |
| Autologin | Dedicated GM-level lab account, credentials in a gitignored local file. |

## 3. Architecture

### 3.1 Process layout

```text
 dev box                                                        colo or localhost
+--------------------------------------------------------+    +----------------------------+
|  Claude Code                                           |    |  cimmeria-server           |
|    |  stdio MCP                  HTTP MCP + token ------------>  cimmeria-lab-mcp         |
|    v                             (WireGuard to colo)   |    |    |  oneshot request/reply |
|  cimmeria-lab.exe  (supervisor + stdio MCP server)     |    |    v                        |
|    |  launch / inject / kill / screenshot by PID       |    |  cell loop (SpaceManager)  |
|    |  JSON-RPC 2.0, framed TCP, 127.0.0.1:8770 + token |    |  dot-console, packet taps  |
|    v                                                   |    +----------------------------+
|  SGW.exe                                               |                 ^
|    cimmeria-client-telemetry.dll  (feature lab-bridge) |                 |  Mercury
|      IO thread  -> bounded queue -> FEngineLoop::Tick  |-----------------+
|      existing uploader ------------------------------------> /api/telemetry -> SigNoz
+--------------------------------------------------------+
```

### 3.2 Options considered

| Option | Verdict |
|---|---|
| **(a) Extend the telemetry DLL with a feature-gated inbound bridge, plus a supervisor that owns the process** | **Chosen.** Reuses injection, hook primitives, the Tick hook, and the uploader. One DLL in the process. |
| (b) A second DLL just for the bridge | Rejected. Two Tick hooks and two hook frameworks in one process. The Atrea ADR rejected coupling to AtreaRL because that DLL is not ours; this one is. |
| (c) Keep driving everything through the x64dbg MCP | Rejected for sustained use. Every RPC pauses the process, and any freeze beyond the heartbeat window disconnects the client. Kept for single-step work. |
| (d) Mount lab tools on the existing admin API router | Rejected. That router binds all interfaces with no auth and is published to players for telemetry ingest (issue #439). The lab endpoint gets its own listener. |
| (e) Embed Rhai or Lua in the cell loop for server probes | Rejected by the owner. Fixed tools only on the server. |
| (f) Relay client commands through the server | Rejected. It would turn a server compromise into code execution on every bridged client. The agent is on the same box as the client, so loopback is sufficient. |

### 3.3 Client bridge

Lives at `crates/client-telemetry/src/bridge/` (directory from day one: `mod.rs`, `transport.rs`, `dispatch.rs`, `lua_eval.rs`, `memory.rs`, `dynamic_hooks.rs`, `native_call.rs`, `journal.rs`).

**Activation is double-gated.** The code exists only when the DLL is built with `--features lab-bridge`, which is off by default, so any telemetry DLL handed to someone else physically lacks it. Even when present, it starts only if `current-session.json` carries a `lab` block, which only the supervisor writes.

**Transport.** Same shape as the Atrea bridge so both can share framing code: JSON-RPC 2.0 over TCP, 4-byte little-endian length prefix, single client, 32-byte token regenerated per launch and passed through the session file. Default bind `127.0.0.1:8770`; the bind address is configurable for a second PC on the LAN or VPN. Port 8765 is avoided because both the Atrea ADR and the SigNoz MCP already claim it.

**Threading.** The IO thread parses and queues only. All Lua, all UObject access, and all native calls run in the existing `FEngineLoop::Tick` hook (`hooks/inline_hooks/engine_frame.rs`), which drains a bounded queue before the original tick. Memory reads run on the IO thread behind `VirtualQuery` checks and never fault. Every main-thread dispatch is wrapped in a structured exception guard; Rust has no native SEH, so this needs `microseh` or a small C shim (see §8).

**Tools exposed through the supervisor:**

| Tool | Purpose |
|---|---|
| `client_lua_eval` | Run a Lua chunk on the main thread via `Lua_doString_wide`. Returns serialized results, captured `print` output, and the Lua error if any. The probe layer. |
| `client_module_info` | Image base, ASLR slide, loaded modules. All other tools accept Ghidra addresses and apply the slide. |
| `client_mem_read` / `client_mem_write` | Typed or raw. Writes are journaled (§6). |
| `client_hook_install` / `_remove` / `_list` | Non-freezing logging hook at an address with a capture spec: registers, stack args, typed dereferences, optional hit limit and sample rate. Cannot stall the heartbeat. |
| `client_call_native` | Call a function by address with a stated calling convention, on the main thread, exception-guarded. Journaled. |
| `client_events_read` | Drain the local event ring: hook hits, Lua prints, CEGUI log lines, Mercury dispatch events. Same events still upload to SigNoz. |
| `client_console` | Submit a native slash command, for the GM console path. |

### 3.4 Lab supervisor (`crates/lab`, binary `cimmeria-lab`)

A Windows-only stdio MCP server. It reuses the launcher's `launch`, `inject`, and `patch_rdata` modules, which move into a library target so both binaries share them. It owns the SGW.exe process handle for the whole session.

| Tool | Purpose |
|---|---|
| `lab_client_start` / `_stop` / `_restart` | Launch suspended, inject, resume; or terminate. Target server (local or colo) is a parameter. |
| `lab_client_status` | PID, uptime, bridge heartbeat age, login state, crash count. |
| `lab_login` | Autologin with the lab account and enter the world on a named character. |
| `lab_screenshot` | Window capture by PID, returned as an MCP image. |
| `lab_crash_report` | Last minidump path, last N bridge commands before the crash, quarantined commands. |
| `lab_timeline` | Merge local client events with server packet-tap rows for a time window (§5). |

Credentials live in `<install>/Binaries/sessions/lab-account.json`, gitignored, next to the existing session file.

### 3.5 Server lab endpoint (`crates/lab-mcp`, `cimmeria-lab-mcp`)

In-process, started from `crates/server/src/main.rs` beside the admin API but on its **own listener**. MCP over streamable HTTP (the `rmcp` crate).

**Fail-closed startup.** It starts only when both `CIMMERIA_LAB_MCP_BIND` and `CIMMERIA_LAB_MCP_TOKEN` are set. There is no default bind address, and a token shorter than 32 bytes is refused. On the colo the container binds inside its namespace and compose publishes the port on the WireGuard address only (`"<wg-ip>:8444:8444"`), never in the public `ports:` list.

**Audit.** Every tool call emits one `info` event on target `lab.tool_call` with tool name, arguments, caller address, and outcome. That reaches SigNoz through the existing exporter, which is the whole audit trail per the owner's decision.

**Getting at live state.** `SpaceManager` is owned by the cell loop task and `cell_tx` is fire-and-forget. The one existing precedent for a reply is `CreateEntity { reply_tx }`. The lab adds request/reply variants in the same style:

- `BaseToCellMsg::LabQuery { query, reply_tx }` — read-only snapshots, answered between ticks.
- `BaseToCellMsg::LabConsoleExec { entity_id, line, reply_tx }` — runs a dot-console line as the named in-world entity and returns the output. `console::exec` currently writes its output as chat to the player, so it needs an output-sink parameter. The agent's entity is its own logged-in lab character, consistent with the "real client body" decision.

| Tool | Purpose |
|---|---|
| `server_console_list` / `server_console_exec` | The 88-command dot-console (`cell/console/registry`), with captured output. Covers spawn, travel, give, mission, patrol, and net-debug families on day one. |
| `server_sessions` | Connected accounts, characters, entity ids, spaces, addresses. |
| `server_entity_get` / `server_entity_query` | Live entity snapshot; filter by space, type, radius, template. |
| `server_witnesses` | Who witnesses an entity, and whom it witnesses. Directly targets the invisible-corpse class of AoI bug. |
| `server_packet_tap_start` / `_read` / `_stop` | Decoded Mercury messages for one session, both directions, into a bounded ring. |
| `server_log_tail` | Filtered read of the existing `LogBuffer`. |
| `server_content_reload` | Existing `ReloadContentEngine` path. |
| `server_db_query` | One statement in a read-only transaction with a row cap. |

## 4. The experiment loop

1. Static work in Ghidra finds a candidate: an address, a Lua global, a message handler.
2. `client_hook_install` or `client_lua_eval` places a probe. No rebuild, no relaunch.
3. The agent causes the behavior: a slash command, a Lua call into the UI, or a server-side `server_console_exec` such as spawning an NPC next to the lab character.
4. `client_events_read`, `server_packet_tap_read`, and `lab_screenshot` show what actually happened.
5. The finding is written up with the probe definition and the dev-session id, so anyone can replay it.

## 5. One timeline

Client events carry client-generated `ts_ms`; packet-tap rows carry server time. The supervisor estimates the clock offset at login (bridge ping against a server `lab` ping) and `lab_timeline` merges both rings locally for low-latency reads. SigNoz remains the durable copy under the existing dev-session id. This sidesteps, rather than fixes, the server-receive-time pivot noted in `observability.md`.

## 6. Crash recovery

Memory writes and native calls will crash the client regularly. The design goal is that a crash costs the agent a minute, not the session.

| Stage | Mechanism | Confidence |
|---|---|---|
| Detect exit | Supervisor owns the process handle. | High. Exists today in `process_watch.rs`. |
| Detect hang or crash dialog | Bridge heartbeat is the Tick-drain counter. No advance for N seconds means hung or sitting in a crash dialog; supervisor terminates the process. WER UI is suppressed for SGW.exe. | High. |
| Capture evidence | Unhandled-exception filter in the DLL writes a minidump (`MiniDumpWriteDump`) + a crash marker, then terminates fast. A vectored handler + `SetThreadStackGuarantee` cover stack overflow (unrecoverable). WER is suppressed via `SetErrorMode` in the DLL. | Built (#685); **replaces** UE3's filter rather than chaining — see open question 2. Needs live validation. |
| Relaunch and inject | Existing suspended-launch path. | High. Runs on every launch today. |
| Server accepts the relogin | Duplicate login evicts the stale session (`crates/services/src/base/login/mod.rs`, KI-7). | High. Verified in code. |
| Autologin | Lua-driven login and character select via the screens' own module handlers (spike-confirmed recipe). State machine built + unit-tested (#685). | **Live path blocked on lua_eval return-value capture** (a Phase-3 bridge TODO): the screen *reads* (`isVisible`, `getCharacterInfo`) need it; the actions work today. |
| Restore probes | The journal re-applies hooks marked persistent. It never replays writes or native calls. The command in flight at crash time is quarantined and reported, and recovery stops after three crashes in ten minutes. | High once built. |
| Restore world state | The agent's job, through `server_console_exec` (`goto`, mission state). | n/a |

## 7. Research rulebook (short form)

1. **Ask the running game first.** If a probe can answer in minutes, do not publish an inference from decompilation alone.
2. **Static finds the where, runtime proves the what.** Ghidra locates the address; a probe confirms the behavior.
3. **Every finding cites its probe.** Record the hook spec or Lua chunk, the dev-session id, and the client build. A finding without a replayable probe is marked inferred.
4. **Non-freezing only.** Logging hooks, never breaks. Use the x64dbg MCP only when single-stepping is unavoidable, and expect a disconnect.
5. **Writes and native calls are experiments, not fixes.** Anything that works gets re-derived as a launcher patch or a server change.
6. **On the colo, touch only the lab character and what it spawns.** Anything that affects other players, such as `reloadmap`, `respawnall`, or shutdown, needs the owner's say-so in that session. This is convention; the token does not enforce it.
7. **Authored from scratch.** AteraLoader and AtreaRL stay reference material, not code to extend.

The full rulebook ships as `docs/guides/live-research-lab.md` in Phase 6.

## 8. Phased plan

| Phase | Scope | Exit criteria |
|---|---|---|
| **1. Bridge core** | `lab-bridge` feature, transport, token, Tick-drain dispatch, `client_lua_eval`, `client_module_info`, `client_mem_read`. Minimal `cimmeria-lab` stdio server proxying those three. | From Claude Code, evaluate a Lua expression in a live client and read back the result. |
| **2. Supervisor** | Start, stop, restart, status, heartbeat watchdog, WER suppression, minidump and journal, `lab_screenshot`, autologin. **Spike first:** can Lua drive login and character select, and is the Lua VM up at the login screen? | Kill SGW.exe from Task Manager; the agent is back in the world on the lab character with no human input. |
| **3. Native probes** | Dynamic logging hooks, `client_events_read`, `client_mem_write`, `client_call_native`, quarantine logic. **Spike first:** structured exception guarding from Rust on i686 (`microseh` versus a C shim). | Re-derive one known finding, such as the black-market window open, entirely through lab tools with no relaunch. |
| **4. Server endpoint v1** | `cimmeria-lab-mcp` crate, fail-closed config, audit events, `server_console_*` with the output-sink refactor, `server_sessions`, `server_log_tail`, `server_content_reload`, `server_db_query`. | Spawn an NPC beside the lab character from Claude Code and read the console output back. |
| **5. Server live state** | `LabQuery` variants, `server_entity_*`, `server_witnesses`, packet taps. | Reproduce an AoI question end to end: server says entity X is witnessed, tap shows the create packet, client hook shows whether it was consumed. |
| **6. Timeline, docs, colo** | `lab_timeline`, full rulebook guide, `.mcp.json.example`, toolchain setup docs, compose wiring for the WireGuard-only port, env-var rows in `main.rs`. | A colo session with other players connected, driven from the dev box. |

Phases 1 to 3 and Phases 4 to 5 touch disjoint crates and can run as parallel tracks. Out of scope here, candidates for a follow-up ADR: a bodiless GM principal, the headless wireclient as a UAT and CI body, and in-DLL frame capture.

## 9. Testing and documentation impact

- **Tests.** Bridge framing, token check, and queue overflow are unit-testable off-process. Hook capture-spec parsing and slide math are unit tests. `LabQuery` and `LabConsoleExec` get cell-loop tests in the style of the `CreateEntity` reply test. The fail-closed startup rules get a regression guard each. `server_db_query` read-only enforcement needs a live-DB test. See [TESTING.md](../../TESTING.md) for the picker.
- **Build rules.** `cimmeria-lab` is Windows-only and joins the workspace exclusion list in [CLAUDE.md](../../CLAUDE.md) and `.github/copilot-instructions.md`.
- **Docs to touch as phases land.** [crates/README.md](../../crates/README.md) rows for the two new crates and the `cimmeria-client-telemetry` row; [client-telemetry.md](client-telemetry.md) (no longer emit-only under the feature); the env-var table in `crates/server/src/main.rs`; [../operations/colo-deploy.md](../operations/colo-deploy.md); [../guides/re-toolchain-setup.md](../guides/re-toolchain-setup.md); a new row in the CLAUDE.md doc-update map.

## 10. Open questions

1. **Is the Lua VM alive at the login screen?** If not, autologin falls back to synthesized input until the VM appears. Phase 2 spike.
2. **Unhandled-exception filter ordering.** *Resolved (#685): replace, don't chain.* Our top-level filter installs last (runs first), writes a minidump + crash marker, and calls `TerminateProcess` — it never returns to UE3's filter or the CRT default. Rationale: UE3's handler pops a crash dialog / runs its own reporter, which would block the supervisor's fast relaunch (the point of §6). The previous filter pointer is captured for diagnostics but not invoked.
3. **Hook capture at arbitrary addresses.** Mid-function hooks need instruction-length decoding for the trampoline. Function-entry-only in Phase 3 is acceptable if that proves fragile.
4. **Share framing code with the Atrea bridge?** That ADR is still unbuilt. If the lab lands first, the Atrea bridge should adopt its transport module rather than define a second one.
