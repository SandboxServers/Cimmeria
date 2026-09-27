---
name: cooked-dialog-override-crash-na-unnumbered
description: Client crash on dialog ids 100100/100101 (>65535 element key); ruled out button XML via timeline; root cause unconfirmed; Ghidra tool-discovery limitation for future sessions
metadata:
  type: project
---

2026-09-27 colo incident: pushing Cimmeria dialog overrides with ids `100100`/`100101`
(category 5, `CookedDataDialogs.pak`) crashed the client on the next map load — reproduced on
relog with no push, so the crash source is data already in the client's writable cooked-data
cache, not the push itself. Full writeup: `docs/reverse-engineering/findings/cooked-dialog-override-crash.md`.

**Confirmed**: the Dialog-category element key is a C++ `long` (32-bit) at the template level
end-to-end (`ServerSource<5,long,Dialog,...>`, `ZipStorage<5,long,Dialog,...>`,
`Event_Cache_ElementReady<long,Dialog>` — mangled RTTI in `13_other_game.c:3028` and
`14_standalone_named.c:5139-5151`). The per-field XML accessors (`00a3d050` et al.) don't narrow
width either, and `crates/resources/src/base/mission_overrides.rs:96` already ships `StepID=80623`
(> 65535) in a *different* cooked-data category without incident — so "any big int field breaks
the generic parser" is ruled out. SigNoz timeline (push at login → clean map-load progress through
two keepalives → crash ~4s later, before `mapLoaded`) rules out the authored `<Buttons>` markup on
100100 as the cause: record-level XML parsing (buttons included) happens at login/push time, and
the client visibly survived past that.

**Not confirmed**: what specifically breaks. Leading unconfirmed hypothesis: something keyed
directly by the top-level `DialogID` value (not a sub-field) — e.g. the ZIP entry name, or a
Dialog-specific index/table the map-load path touches — chokes above some threshold at or below
65536. No Cimmeria override in ANY category has used a top-level element key above 65535 before
this, so this is genuinely uncharted, not previously-tested-safe territory. Renumbering to
60100-60104 sidesteps the 16-bit-boundary hypothesis specifically but is NOT proven safe (still an
order of magnitude above the historical max shipped/authored id, ~5861/3996) — smoke-test before
calling it closed.

**Ghidra tool-access limitation this session** (record for the next RE session): Ghidra was not
running at session start; launched mid-session against the existing project
(`C:\Users\Steve\source\projects\SGW\Stargate Worlds-QA\Working\binaries\SGW.gpr`) via
`Start-Process ghidraRun.bat <project.gpr>` — the GhidraMCP plugin auto-started on port 8100 (no
manual "Start MCP Server" click needed; a prior session apparently left autostart configured) and
`mcp__ghidra__connect_instance` succeeded, reporting `tools_registered: 195`. **But the 195
dynamically-registered analysis tools (`decompile_function`, `get_xrefs_to`, `search_functions`,
etc.) were never reachable via this agent's `ToolSearch`/deferred-tool mechanism** — `check_tools`
on the bridge itself said they were "callable," but every direct call and every `ToolSearch` query
for them came back "No such tool" / "No matching deferred tools found," even after
`unload_tool_group`+`load_tool_group` round-trips intended to force a `tools/list_changed`. This
matches the tool's own docstring warning: "Clients that cache the initial tools/list and don't
honor `tools/list_changed` must re-list tools after this call" — this harness is such a client, and
there is no user-facing "re-list tools" action available mid-session. **Workaround for next time**:
get Ghidra connected and the analysis tools registered *before* the agent session's tool list is
first built (i.e., have Ghidra+plugin already running when the session starts), rather than
connecting mid-session. This session was reduced to grepping the pre-extracted
`docs/reverse-engineering/decompiled/*.c` dumps and the debugger-proxy tools (which stayed static
and worked fine) instead of fresh live decompilation. See also
[[reference-mcp-servers]] for the general Ghidra MCP wiring notes (that memory predates this
specific tools/list_changed gotcha).
