---
title: Live Research Lab — the research rulebook
type: how-to
audience: engineers and AI agents doing RE / live verification against a running SGW.exe + cimmeria-server
last_updated: 2026-09-19
companion_docs:
  - ../architecture/live-research-lab.md
  - ../architecture/client-telemetry.md
  - ../architecture/observability.md
  - re-toolchain-setup.md
  - reverse-engineering-with-claude.md
  - ../operations/colo-deploy.md
---

# Live Research Lab — the research rulebook

The Live Research Lab gives an agent hands, eyes, and a body inside a
running Stargate Worlds session: a client bridge that evaluates Lua and
installs non-freezing hooks on the live `SGW.exe`, a supervisor that owns
the process and merges timelines, and a token-gated in-server endpoint
that reads live entity/witness/packet state. The design is the ADR at
[../architecture/live-research-lab.md](../architecture/live-research-lab.md);
this guide is the **operating manual and the rules of the road**.

The one rule everything else serves:

> **Do not infer client behavior when the running client can answer the
> question.**

If a probe can answer in minutes, a decompilation-only inference is a
draft, not a finding.

## The rulebook

These seven rules expand ADR §7. They apply to every session, local or
colo.

### 1. Ask the running game first

Static analysis produces a hypothesis. A probe produces an answer. If
`client_lua_eval` or a logging hook can confirm the behavior in minutes,
run it before you publish anything derived from Ghidra alone. Inference is
allowed — it is just labeled `inferred` until a probe upgrades it.

### 2. Static finds the *where*, runtime proves the *what*

Ghidra (or x64dbg for single-step work) locates the address, the Lua
global, the message handler. The lab confirms what actually happens there
at runtime. The two are complementary, not competing: never skip the
static step (you need the address), never stop at it (you need the proof).

### 3. Every finding cites its probe

A finding is not done until it records, together:

- the **probe** — the exact Lua chunk or the hook capture spec (address,
  registers/args captured, hit limit, sample rate);
- the **dev-session id** — the id every client and server event was
  tagged with, so the run is findable in SigNoz;
- the **client build** — the SGW.exe build the probe ran against.

A finding without a replayable probe is marked `inferred`. A finding with
one can be re-run by anyone. This is the difference between "we think" and
"we checked".

### 4. Non-freezing hooks only

Logging hooks, never breakpoints. The client's Mercury channel dies if the
main thread stalls past the heartbeat window, and a dead channel means the
server disconnects you — you lose the session you were measuring. Every
lab hook is a log-and-continue trampoline that cannot stall the Tick
drain. Reach for the x64dbg MCP **only** when single-stepping is genuinely
unavoidable, and expect the disconnect when you do.

### 5. Writes and native calls are experiments, not fixes

`client_mem_write` and `client_call_native` exist to *learn* — to prove
that flipping a byte or calling a function produces the effect you
predicted. Nothing that works this way ships as a memory poke. Anything
worth keeping gets re-derived as a launcher patch (client side) or a
server change (server side), with its own test. The lab is the workbench,
not the product.

### 6. On the colo, touch only the lab character and what it spawns

The colo runs with other, non-telemetry players connected. Anything that
reaches beyond your own lab character — `reloadmap`, `respawnall`, a
shutdown, a world-wide content reload — needs the owner's explicit say-so
*in that session*. Spawning an NPC next to your lab character is fine;
anything a stranger three zones away would notice is not. The token does
not enforce this; it is a discipline. (The audit trail does record it —
see "Audit" below.)

### 7. Authored from scratch

AteraLoader and AtreaRL are reference material for how a bridge *can* be
built. They are not code to extend or vendor. Everything in the lab is our
own, so the trust boundary is one we control end to end.

## How to run an experiment

The loop, concretely (ADR §4):

1. **Find a candidate** in Ghidra: an address, a Lua global, a message
   handler. Note the client build.
2. **Place a probe** with `client_hook_install` (a capture spec at the
   address) or `client_lua_eval` (a chunk). No rebuild, no relaunch.
3. **Cause the behavior.** Drive it from the real client body: a slash
   command via `client_console`, a Lua call into the UI, or a server-side
   `server_console_exec` — e.g. spawn a mob beside the lab character.
4. **Read what happened.** `lab_timeline` for the merged client+server
   window (below), `lab_screenshot` for the eyes, `server_packet_tap_read`
   / `server_witnesses` / `server_entity_get` for the server's view.
5. **Write it up** with the probe, the dev-session id, and the client
   build (rule 3). Then re-derive anything worth keeping as a patch or a
   server change (rule 5).

### Reading the merged timeline

`lab_timeline` interleaves local client events with server packet-tap
rows on **one clock**. Client events (bridge heartbeat today; the full
hook-hit / Lua-print / Mercury-dispatch ring once #686 lands) arrive on
the dev-box clock; packet-tap rows arrive on the server clock. The
supervisor estimates the offset between the two from the packet-tap round
trip and projects the client events onto the server clock, so a client
observation and the packet that caused it line up.

Call it with the server session whose tap you want merged in:

```jsonc
// tool: lab_timeline
{
  "session_id": "<the lab character's session>",  // omit for client-only
  "window_ms": 30000,                              // lookback; default 60000
  "limit": 1000                                    // tap-row cap; default 1000
}
```

The result carries the estimated `offset` (with its RTT and method), the
resolved `window`, and the ordered `events`. Each event says whether it is
`client` or `server` sourced, its `server_ts_ms` (the merge key), the
original `client_ts_ms` for client events, and a `kind`
(`bridge.heartbeat`, `mercury.<name>`, …).

`lab_timeline` is the **low-latency local read**. SigNoz remains the
durable copy under the dev-session id — use SigNoz for anything you need
to keep or share; use `lab_timeline` for the fast inner loop.

> **Today vs. later.** The client side of the timeline is currently the
> bridge heartbeat ring — enough to answer "did the client's main thread
> keep ticking while the server sent packet X". The rich client-event ring
> (`client_events_read`, #686) was stopped by the owner; when it lands it
> plugs into the same offset/merge machinery with no downstream change.
> Until then, `lab_timeline` is useful but heartbeat-only on the client
> side, and the clock offset is a coarse estimate from the packet-tap
> round trip (there is no dedicated server ping tool yet).

### Did the client accept what the server sent?

Two client events answer this without a probe (2026-09-28, see
[client-telemetry.md](../architecture/client-telemetry.md#gameplay-seams-what-the-client-accepted-and-what-its-ui-complained-about)):

- `client.cme.event` with `kind = net_in` names every inbound entity
  method the client routed (`event = Event_NetIn_onDialogDisplay`, …).
  `client.dispatch.method_dropped` names the ones it discarded. If the
  server's packet tap shows the method sent and neither event appears, it
  was lost below the dispatcher.
- `client.ui.cegui_log` at `error` carries the text of CEGUI errors,
  including `ScriptException`s from failed UI Lua binding calls.

Both reach SigNoz (`service.name = 'cimmeria-client'` and
`cimmeria.session_kind = 'lab'`, the event name in `client_target`) and, with the bridge, the local ring
(`client_events_read`, kinds `cme.event` and `cegui.log`). Both are
throttled per name: a hot name gets 8 in a burst then 4 a second, and the
next event that gets through carries `suppressed`. The backlog of further
client seams is
[docs/analysis/lab-automation/tooling-backlog.md](../analysis/lab-automation/tooling-backlog.md).

### The free Lua-VM check (SigNoz Q1)

Before you build any autologin probe, answer the open question from the
#685 spike — **is the Lua VM alive at the login screen?** — for free, with
no new probe. The client already emits a `client.lua.newstate` event when
the Lua state is created, and it ships to SigNoz under your dev-session id.
Query it:

- **Signal:** Logs (or Traces).
- **Filter:** `service.name = 'cimmeria-client'` AND
  `session_id = <your session>` AND `client_target = 'client.lua.newstate'`.
  `lab_client_start` returns the session id under `telemetry.session_id`.
- **Read:** the timestamp of the first `client.lua.newstate` relative to
  the login-screen render tells you whether the VM exists before or only
  after character select. If it fires at the login screen, Lua-driven
  autologin is viable; if not, autologin falls back to synthesized input
  until the VM appears.

This is the pattern for the whole lab: the cheapest probe is often a query
against telemetry you are already emitting.

## Tools at a glance

Supervisor (`cimmeria-lab`, stdio MCP on the dev box):

| Tool | Purpose |
|---|---|
| `lab_client_start` / `_stop` / `_restart` | Own the SGW.exe lifecycle. |
| `lab_client_status` | PID, uptime, heartbeat age, login state, crashes. |
| `lab_login` | Autologin the lab account, enter the world. |
| `lab_screenshot` | Window capture by PID → MCP image. |
| `lab_crash_report` | Last minidump, last N commands, quarantined command. |
| `lab_timeline` | Merged client+server window (above). |
| `client_lua_eval` / `client_module_info` / `client_mem_read` | Probe tools proxied to the bridge. |
| `client_ui_click` / `client_cursor_move` | Click a named UI window (`Name` or `Parent/Child`) like a player: cursor onto its centre, real button messages. |
| `client_input_key` / `client_type_text` | Key presses and typing as `WM_KEYDOWN`/`WM_KEYUP` (the game translates them; Shift is virtual). Types letters, digits, space, `-_/.`, so `/logout` and `.`-console lines work through chat. |
| `client_input_mouse` | DirectInput relative motion (mouse-look) and button clicks at the UI cursor. |
| `client_input_focus` / `client_input_status` / `client_input_release` | Virtual focus (the game keeps reading input in the background), hook counters, let go of everything. |

Server endpoint (`cimmeria-lab-mcp`, in-server, token-gated HTTP —
WireGuard-only on the colo): `server_console_*`, `server_sessions`,
`server_entity_*`, `server_witnesses`, `server_packet_tap_*`,
`server_log_tail`, `server_content_reload`, `server_db_query`. See ADR
§3.5 for the full set; `docs/operations/colo-deploy.md` for the port.

## Driving the client with its own input

The input tools press nothing through Lua: Lua only reads where a widget is and places the UI cursor. What the live client showed (2026-09-29):

- Keys and typing are window messages. A posted `WM_KEYDOWN`/`WM_KEYUP` reaches the game; a bare posted `WM_CHAR` is ignored, because the game turns keys into characters itself with `GetKeyboardState` + `ToUnicodeEx`. The bridge makes Shift virtual by answering those two calls.
- Mouse buttons are window messages, applied at CEGUI's cursor position, not at the message's coordinates. The cursor does not follow posted mouse moves or DirectInput motion, so the supervisor places it through CEGUI's own cursor and mirrors it into a virtual `GetCursorPos`.
- The DirectInput keyboard is created but never read. The mouse is read while the viewport has it captured (mouse-look), and only while the game thinks it is focused: virtual focus answers `GetForegroundWindow`, `GetFocus`, `GetActiveWindow`, and lets a background `Acquire` succeed.
- Launch skips the intro movies with Escape; on a new character Escape also skips the arrival cutscene, and dialogs are paged with Next to the green checkmark (`Dialog_DoneButton`).
- `lab_client_start` refuses while any `SGW.exe` is running (two clients on one machine misbehave), and injects `cimmeria-client-patches.dll` first when `CIMMERIA_LAB_PATCHES_DLL` is set, as the launcher does.

## Trust, audit, and the colo

- **Lab telemetry reaches SigNoz under a real token.** Each
  `lab_client_start` mints a dev-session token from
  `CIMMERIA_LAB_SERVER_URL` (default `http://127.0.0.1:8443`, the local
  server's admin port) with `session_kind = lab`, and writes it and the
  server's upload endpoint into `current-session.json`
  (`CIMMERIA_LAB_UPLOAD_ENDPOINT` overrides the endpoint). The client's
  events then land in `service.name = 'cimmeria-client'` tagged
  `cimmeria.session_kind = 'lab'`. The start result's `telemetry` block
  says whether the mint worked and, if not, why (a server without
  `CIMMERIA_TELEMETRY_HMAC_SECRET` answers 500); the launch goes ahead
  either way. The telemetry token is not the bridge token, which never
  leaves the machine.
- **Activation is double-gated.** The bridge code exists only in a
  telemetry DLL built `--features lab-bridge` (off by default), and even
  then starts only when `current-session.json` carries a `lab` block that
  only the supervisor writes. A DLL handed to anyone else physically lacks
  the bridge.
- **No server→client command relay exists.** The agent drives the client
  from the same box as the client. Other players' clients are unreachable
  by construction.
- **Fail-closed server endpoint.** `cimmeria-lab-mcp` starts only when
  both `CIMMERIA_LAB_MCP_BIND` and `CIMMERIA_LAB_MCP_TOKEN` are set, with
  a token of at least 32 bytes. Absent env = endpoint absent.
- **Audit is the log trail.** Every server-endpoint tool call emits one
  `info` event on `lab.tool_call` (tool, arguments, caller, outcome),
  which reaches SigNoz through the existing exporter. On the colo, that
  log line is the whole audit trail per the owner's decision — which is
  why rule 6 matters.

## Setup

The lab is wired into `.mcp.json` alongside Ghidra and x64dbg — see
[re-toolchain-setup.md](re-toolchain-setup.md) for the entries and the env
vars, and [reverse-engineering-with-claude.md](reverse-engineering-with-claude.md)
for where the lab sits in the RE workflow (the "ask the running game"
path). Lab-account credentials live in
`<install>/Binaries/sessions/lab-account.json`, gitignored.

`lab_client_start` needs the i686 `sgw-start32.exe` helper beside
`cimmeria-lab.exe` (or at `CIMMERIA_LAB_START32`). The supervisor is 64-bit
and cannot inject into the 32-bit client itself; the helper starts
`SGW.exe` suspended, injects the bridge DLL at the game's bitness and
resumes it (#985). Build it with
`cargo build -p cimmeria-start32 --target i686-pc-windows-msvc`.
