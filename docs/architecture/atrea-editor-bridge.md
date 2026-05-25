# ADR: Atrea Editor Bridge — MCP server for in-game UnrealEd

> **Last updated**: 2026-05-25
> **Audience**: Engineers wiring an AI agent (Claude Code) to drive the in-game UnrealEd editor in SGW.exe for map authoring
> **Type**: Architecture decision record
> **Status**: Proposed — Phase A scope ready, awaiting sign-off on §5 open questions
> **Owner**: Reverse-engineering / tools
> **Companion docs**: [reverse-engineering/findings/atrea-editor.md](../reverse-engineering/findings/atrea-editor.md), [reverse-engineering/editor-source-mapping.md](../reverse-engineering/editor-source-mapping.md)

## TL;DR

Inject a small native DLL (`CimmeriaEditorBridge.dll`) into SGW.exe alongside `AtreaRL.dll` when the user launches `AtreaEditor.bat`. The DLL listens on a localhost-only TCP socket (loopback-bound, single-client, token-gated), accepts JSON-RPC 2.0 requests, and marshals them onto the wxWidgets main thread where it invokes `UEditorEngine::ExecBrush` (`0x00BF5F30`), `UUnrealEdEngine::Exec` (`0x00EDB0C0`), and vtable methods on `GEditor` (`0x01EF134C`). A separate stdio MCP server (Python or Rust, child-process of Claude Code) proxies MCP tool calls into JSON-RPC requests against the DLL's socket. Phase A proves the loop with a single `editor_exec` tool; Phase B adds typed actor lifecycle; Phase C adds visual feedback (screenshots) and the SGW-specific BigWorld chunk operations.

## 1. Why this exists

The user wants the agent (me) to facilitate map edits in the SGW editor — placing actors, editing properties, saving chunked sublevels, running PIE smoke tests — without the user driving every mouse click. The editor lives in-process inside SGW.exe (it is UE3's UnrealEd repurposed by Atrea byte patches; see [atrea-editor.md](../reverse-engineering/findings/atrea-editor.md) for the full archaeology). Reaching it requires an in-process surface.

## 2. Architecture options considered

| Option | Pros | Cons | Decision |
|---|---|---|---|
| **(a) Injected DLL + JSON-RPC over localhost TCP** | Direct typed access to `GEditor`/`GWorld`/selection; `FOutputDevice` swap captures real Exec output; stdio shim is hot-reloadable. | New native code to build/sign/ship; DLL crash = SGW crash; threading discipline mandatory. | **Recommended** |
| (b) AtreaRL.dll extension (new hook group) | One fewer DLL to ship; reuses AtreaRL hook framework. | Couples MCP feature to AtreaRL's release cadence; mixes patching + RPC responsibilities. | Rejected — coupling cost too high. |
| (c) External controller via `x64dbg-automate` | Zero new binaries; already wired in `.mcp.json`. | Catastrophically slow — every RPC pauses the whole process; editor GUI freezes on every break; cannot capture Exec output cleanly. | Rejected for sustained use. Keep for spot one-offs during RE. |
| (d) wxWidgets event injection via `SendMessage` | No injection. | Couples agent to wx control IDs + layout; modal dialogs eat the queue silently; cannot read `GWorld`/`GEditor` state at all. | Rejected. |
| (e) Hybrid: minimal DLL (`exec` + `read_prop` only) + smart external driver | Tiny native surface; complexity in fast-iterating Python. | Forces every op through Exec text grammar; round-trip-chatty for typed ops. | Kept as fallback for ops that resist Exec grammar. |

**Recommendation:** ship (a). Fall back to (d) from inside the DLL for any SGW-specific dialog (e.g. NewWorldMap.xrc) that doesn't have a clean Exec equivalent — we own the process, so we can post wx events with valid IDs ourselves.

### Security model (loopback socket)

A loopback socket is the default but is still IPC any local-user process can poke. Lock it down:

1. `bind(INADDR_LOOPBACK)` only — never `INADDR_ANY`.
2. First message must present a 32-byte token written to `%LOCALAPPDATA%\Cimmeria\editor-bridge.token` with a DACL scoped to the current user (`SetNamedSecurityInfo` granting only `SID_CURRENT_USER`).
3. Single concurrent client; second connection attempt is closed immediately.
4. Token is regenerated on each editor launch.
5. The MCP stdio shim reads the token from the same path and presents it on connect.

This matches the threat model of Ghidra MCP (`http://127.0.0.1:8100/`) but with an explicit auth step Ghidra doesn't bother with. Upgrade path: Windows named pipe with explicit DACL instead of TCP — see open question §5.3.

## 3. Recommended architecture detail

### 3.1 Process layout

```text
+--------------------+        stdio (MCP)        +-----------------------+
|   Claude Code      | <-----------------------> | editor-bridge-mcp.exe |
|   (this agent)     |                           |  (stdio MCP server,   |
+--------------------+                           |   Rust or Python)     |
                                                 +-----------+-----------+
                                                             | JSON-RPC 2.0
                                                             | over TCP
                                                             | 127.0.0.1:8765
                                                             | + bearer token
                                                             v
+------------------------------- SGW.exe (PID N) ----------------------------+
|                                                                            |
|   wx main thread                                                           |
|   +-----------------------------+    SPSC queues    +------------------+   |
|   | FEngineLoop::Tick @0x416EC0 |<---- requests ----| IO thread        |   |
|   |   drain request queue;      |                   |  (accept loop,   |   |
|   |   dispatch to handler;      |---- responses --->|   JSON parse,    |   |
|   |   UEditorEngine::ExecBrush  |                   |   token check)   |   |
|   |   or vtable call;           |                   +------------------+   |
|   |   capture FOutputDevice     |                            ^             |
|   +-----------------------------+                            | bind:8765   |
|                                                              |             |
|   AtreaRL.dll (existing) -- patches GIsEditor=1, etc.        |             |
|   CimmeriaEditorBridge.dll (NEW) -- this ADR ----------------+             |
|                                                                            |
+----------------------------------------------------------------------------+
```

### 3.2 Wire protocol

**DLL ↔ stdio shim:** JSON-RPC 2.0 over a length-prefixed framed TCP stream. 4-byte little-endian length, then UTF-8 JSON body. Framed (not newline-delimited) so payloads can contain newlines safely.

**Shim ↔ Claude Code:** standard MCP over stdio. The shim is a thin translator: one MCP tool call = one JSON-RPC request.

Why not gRPC: protobuf-c runtime in a 32-bit MSVC DLL is annoying, and we don't need streaming.
Why not MCP-over-stdio directly: SGW.exe is the parent process (the user double-clicked `AtreaEditor.bat`); Claude Code cannot spawn it without losing interactive use.

**Example — typed request:**

```json
{"jsonrpc":"2.0","id":1,"method":"actor.spawn",
 "params":{"class":"StaticMeshActor",
           "location":[1024.0, 2048.0, 64.0],
           "rotation":[0, 16384, 0]}}
```

**Example — response:**

```json
{"jsonrpc":"2.0","id":1,
 "result":{"actor_id":"PersistentLevel.StaticMeshActor_42",
           "name":"StaticMeshActor_42",
           "guid":"4F2A1B...","ok":true}}
```

**Example — raw Exec passthrough:**

```json
{"jsonrpc":"2.0","id":2,"method":"editor.exec","params":{"command":"BRUSH ADD"}}
```

```json
{"jsonrpc":"2.0","id":2,
 "result":{"return_value":true,
           "log":["Log: Brush Add","Log: 1 brush(es) added"],
           "warnings":[],"errors":[],"duration_ms":34}}
```

**Example — error:**

```json
{"jsonrpc":"2.0","id":3,
 "error":{"code":-32001,"message":"editor busy: modal dialog open",
          "data":{"dialog":"wxFileDialog","since_ms":12000}}}
```

### 3.3 Initial MCP tool surface

| Tool | Signature (abbreviated) | Implementation hook |
|---|---|---|
| `editor_exec` | `{command, timeout_ms?} -> {return_value, log, warnings, errors, duration_ms}` | `UEditorEngine::ExecBrush` @ `0x00BF5F30` with `FOutputDevice` swap |
| `editor_state` | `{} -> {map_path, dirty, mode, pie_active, selection_count, viewport_count}` | Reads `GEditor` @ `0x01EF134C` fields + `GWorld` @ `0x01EE2684` |
| `actor_spawn` | `{class, location, rotation?, name?, layer?} -> {actor_id, name, guid}` | `UEditorEngine::vfunc_102` @ `0x00B78B30` (SpawnActor) |
| `actor_list` | `{class_filter?, layer?, bounds?, limit?} -> Actor[]` | Iterate `GObjObjects` filtered by `IsA(AActor)` and outer == `GWorld->PersistentLevel` |
| `actor_get` | `{actor_id, properties?} -> {class, location, rotation, properties}` | Reflection over `UClass` property chain |
| `actor_set_property` | `{actor_id, property, value} -> {ok, prev_value?}` | `SET <Class> <Property> <Value>` Exec, or direct `UProperty::ImportText` |
| `actor_destroy` | `{actor_id} -> {ok}` | `EDIT DELETE` Exec on transient selection, or direct `UWorld::DestroyActor` |
| `selection_get` | `{} -> {actors}` | `USelection` from `GEditor->GetSelectedActors()` |
| `selection_set` | `{actor_ids} -> {ok, count}` | `SELECT NONE` then per-actor `SELECT NAME=...` |
| `map_load` | `{path, force?} -> {ok, map_path}` | `MAP LOAD FILE="..."` → `UnEdSrv__HandleMapLoad` @ `0x00EF9780` |
| `map_save` | `{path?, silent?} -> {ok, written_path}` | `OBJ SAVEPACKAGE FILE="..." SILENT=TRUE` → `UUnrealEdEngine__PromptAndSavePackage` @ `0x00FD6720` |
| `chunk_save_bigworld` | `{layer?, region?} -> {ok, chunks_written, paths}` | SGW-specific SaveBigWorldChunks (cmd `0x6774`, handler in WxMainMenu @ `0x00FF91C0`) |
| `camera_get` | `{viewport} -> {location, rotation, fov, ortho}` | Read `WxEditorFrame->ViewportConfigData` via `0x00FF3560` |
| `camera_set` | `{viewport, location?, rotation?, fov?} -> {ok}` | `CAMERA ALIGN` / direct viewport field write |
| `screenshot` | `{viewport, path, width?, height?} -> {ok, path, bytes}` | `SHOT TILEDSHOT` Exec or direct `FViewport::ReadPixels` |
| `editor_undo` / `editor_redo` | `{} -> {ok, label}` | `TRANSACTION UNDO` / `REDO` Exec |
| `pie_start` / `pie_stop` | `{spawn_at?} -> {ok}` | `PLAY FROM_HERE` / Esc via `UEditorEngine__PlayMap_Enter` @ `0x00B20EC0` |
| `wait_idle` | `{timeout_ms?} -> {idle, blocking_dialog?}` | Polls modal-dialog count; returns when wx is idle |
| `pie_script_*` | (load, start, step, toggle_pause) | SGWPIEScriptManager CME events @ `0x00D3D060`–`0x00D3D260` (see [atrea-editor.md](../reverse-engineering/findings/atrea-editor.md) §SGWPIEScriptManager) |

### 3.4 Threading model

The editor is single-threaded on the wx main thread. We add exactly two threads to the process: the IO thread and (optionally) a watchdog thread.

```text
IO thread (DLL):
  loop:
    fd = accept()
    auth(fd)                                       # token exchange
    while connected:
      len  = read_u32_le(fd)
      body = read_n(fd, len)
      req  = parse_json(body)
      slot = request_queue.push(req)               # SPSC, fixed N=64
      resp = slot.await(timeout = req.timeout)
      write_framed(fd, serialize(resp))

Main thread (tick hook on FEngineLoop::Tick @ 0x00416EC0):
  pre-tick:
    while req = request_queue.try_pop():
      out = capture_output(|| dispatch(req))       # FOutputDevice swap
      req.slot.complete(out)
  ... normal Tick runs ...
```

Rules:

1. The IO thread **never** touches `GEditor`, `GWorld`, or any UObject. It only parses, queues, and serializes.
2. Dispatch on the main thread runs inside the `FOutputDevice` swap so `Ar.Logf` from inside the Exec call is captured per-request.
3. The queue is bounded (64 slots). Overflow returns `error.code = -32010 ("queue full, retry")` from the IO thread — never blocks accept.
4. Per-request timeout (default 5 s) lets the IO thread return `-32011 ("dispatch timeout")` if the main thread is stuck in a modal dialog or autosave.
5. The hook into `FEngineLoop::Tick` is the same pattern as AtreaRL's existing patches — a 5-byte JMP trampoline to a code cave, call our drain, then jump back to the original prologue.

### 3.5 Failure modes

| Scenario | Behavior |
|---|---|
| Modal dialog open (wx blocked) | Main thread doesn't drain queue; IO thread times out per-request and returns `-32011` with `blocking_dialog` hint from a probe of `wxModalEventLoop` count. `wait_idle` tool lets the agent poll. |
| `Exec` returns no value but logs an error | We always return `{return_value, log, warnings, errors}`. `warnings`/`errors` are classified by the `Log:` / `Warning:` / `Error:` prefix UE3 already emits. |
| User clicks Save in GUI mid-call | Their Save runs on the same main thread between our drain ticks; no interleaving. The next `editor_state` reflects the new clean state. |
| SGW.exe crashes | TCP socket closes. Shim surfaces `transport closed`, returns MCP error for every subsequent tool call. Shim attempts one reconnect per minute; user must relaunch via `AtreaEditor.bat`. |
| DLL load order (need AtreaRL patches first) | Bridge `DllMain` posts an APC that waits for `GIsEditor` @ `0x01EAD7AC == 1` and `GEditor` @ `0x01EF134C != NULL` before installing the tick hook. Hard 10 s timeout. |
| Re-entrant Exec (Exec calls Exec) | Depth counter; nested calls execute synchronously inline and inherit the parent's `FOutputDevice` — no requeue, no deadlock. |
| Editor in PIE | `editor_state.pie_active = true`; tools that mutate the editor world return `-32020 ("editor in PIE, call pie_stop first")` unless `force=true`. |

### 3.6 State the DLL exposes for read

`editor_state` (and an embedded `_state` field on every response for cheap cache invalidation):

| Field | Source |
|---|---|
| `map_path` | `GWorld->GetOutermost()->LinkerLoad->Filename` |
| `dirty` | Any package in `GObjLoaders` with `PKG_Dirty` flag |
| `mode` | `GEditorModeTools()->GetCurrentModeID()` (Geometry / Terrain / Matinee / Default) |
| `pie_active` | `GIsPlayInEditorWorld` global |
| `selection_count` / `selection_ids` | `GEditor->GetSelectedActors()` — full path-name strings |
| `viewport_count`, `viewports[].camera` | `GApp->EditorFrame->ViewportConfigData` |
| `current_layer` | SGW-specific `GameMap` browser state (WxMapLayer panel field) |
| `bigworld_chunks_loaded` | SGW chunk loader manifest from `g_EntityManager` @ `0x01EF244C` |

## 4. Phased implementation plan

### Phase A — "Hello, editor" (1–2 days)

**Goal:** prove the injection + socket + MCP-bridge loop end-to-end with one tool.

1. `CimmeriaEditorBridge.dll` (MSVC x86) — exports `DllMain` only.
2. Loader integration — extend `AtreaLoader.config.xml` to inject the bridge alongside `AtreaRL.dll` when the `Editor` group is enabled.
3. Tick hook on `FEngineLoop::Tick` @ `0x00416EC0` that drains a queue and calls `UEditorEngine::ExecBrush` @ `0x00BF5F30` with a captured `FOutputDevice`.
4. Localhost TCP server on `127.0.0.1:8765` with token auth, framed JSON-RPC.
5. `editor-bridge-mcp` stdio server exposing one tool: `editor_exec`.
6. Smoke test: from Claude Code, invoke `editor_exec("HELP")`, observe UnrealEd help text in `log[]`.

**Exit criteria:** I can issue `editor_exec("MAP LOAD FILE=\"…\"")` and observe the editor load it.

### Phase B — Typed actor lifecycle (~1 week)

1. `actor_spawn`, `actor_list`, `actor_get`, `actor_set_property`, `actor_destroy`.
2. `selection_get`, `selection_set`.
3. `map_save` (silent) and `map_load`.
4. `editor_state`.
5. `wait_idle`.
6. Stable actor IDs: full path-name (`Level.ActorName`); rename = new ID.
7. Property reflection — enumerate `UClass` property chain so the agent can discover what's settable.
8. Test fixture: a deterministic map plus a Python script that drives a scripted edit and diffs the resulting `.umap` against a golden.

**Exit criteria:** I can author a Python recipe that loads a map, spawns N actors, sets properties, saves, and exits — and the saved map opens cleanly in a clean editor session.

### Phase C — Visual feedback + SGW BigWorld + PIE (2+ weeks)

1. `screenshot` — writes PNG, shim streams as an MCP resource (so I can see the viewport).
2. `camera_get`, `camera_set`.
3. `editor_undo`, `editor_redo`.
4. `pie_start`, `pie_stop`, `pie_script_*` (uses `SGWPIEScriptManager`).
5. `chunk_save_bigworld` — handler for command id `0x6774` (SaveBigWorldChunks toolbar button) in WxMainMenu @ `0x00FF91C0`.
6. Optional: MCP "subscription" channel so the bridge pushes selection-changed / map-loaded events — agent reacts without polling.
7. Hardening: telemetry, structured panic recovery (SEH frame around every dispatch), reconnect, version handshake.

## 5. Open questions (require sign-off before Phase A)

1. **Same-machine only, or cross-machine?** Should Claude Code on Mac/Linux ever drive an SGW editor on a Windows VM? If yes, bridge needs WSS + mTLS, not just a localhost socket. If no (recommended), keep loopback-only.
2. **Bridge lifetime vs editor lifetime.** Should `CimmeriaEditorBridge.dll` live and die with each `AtreaEditor.bat` launch (matches AtreaRL behavior), or should the MCP stdio shim be capable of launching the editor itself? Latter is convenient ("Claude, open the editor and load Castle") but means the shim invokes `AtreaEditor.bat` and waits for the socket.
3. **Risk profile for the localhost socket.** Are you OK with "any local process running as your user can poke the bridge if it can read the token file"? That matches Ghidra MCP. Stricter alternative: Windows named pipe with explicit DACL instead of TCP.
4. **Editor reliability bar.** When SGW crashes mid-edit, should the bridge attempt to save the working map to `.umap.bridge-recovery` before the process dies (via vectored exception handler)? Adds complexity but might save grief.
5. **Output capture scope.** `editor_exec` captures *only* output produced during the Exec call (cleanest), or a tail of `Editor.log` (closer to what the user sees)?

## 6. Appendix — address quick-reference

Curated from [reverse-engineering/editor-source-mapping.md](../reverse-engineering/editor-source-mapping.md) and Wave-2 deep dives.

| Symbol | VA |
|---|---|
| `FEngineLoop::Tick` (hook site) | `0x00416EC0` |
| `UEditorEngine::ExecBrush` (BRUSH dispatcher) | `0x00BF5F30` |
| `UUnrealEdEngine::Exec` (EDITDEFAULT/OBJECT/ACTOR) | `0x00EDB0C0` |
| `UEditorPlayer::Exec` (viewport) | `0x00D3E060` |
| `UnEdSrv__HandleMapLoad` | `0x00EF9780` |
| `UnEdSrv__BRUSH_ADD` | `0x00BF7D40` |
| `SpawnActor` (vfunc_102) | `0x00B78B30` |
| `UUnrealEdEngine__PromptAndSavePackage` | `0x00FD6720` |
| `UUnrealEdEngine__SaveDirtyPackages` | `0x00FD7800` |
| `UEditorEngine__PlayMap_Enter` | `0x00B20EC0` |
| `WxMainMenu` ctor (SaveBigWorldChunks handler) | `0x00FF91C0` |
| `GEditor` | `0x01EF134C` |
| `GWorld` | `0x01EE2684` |
| `GIsEditor` / `GIsServer` / `GIsGame` | `0x01EAD7AC` / `0x01EAD7C0` / `0x01EB0830` |
| `UUnrealEdEngine` vtable | `0x019F4E6C` |
| Bridge socket (localhost) | `127.0.0.1:8765` (default) |
| Bridge token file | `%LOCALAPPDATA%\Cimmeria\editor-bridge.token` |
