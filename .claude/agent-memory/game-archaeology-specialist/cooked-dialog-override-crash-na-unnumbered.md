---
name: cooked-dialog-override-crash-na-unnumbered
description: Client crash on dialog ids 100100/100101; >65535 element-key hypothesis REFUTED by live headless-Ghidra decompile (2026-09-27); root cause still open, new lead is SpeakerID=754
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

**UPDATE 2026-09-27 (same day, follow-up session, headless Ghidra): the >65535 hypothesis is
REFUTED.** Live decompile of the actual **category-5** `onVersionInfo` (`FUN_004435c0` @
`0x004435c0`, found via its `CategoryId != 5` guard — NOT the `0x00441630` address
`cooked-data-pipeline.md` documents, which is category 6's copy) and `onCookedDataError`
(`FUN_00443a30`) instantiations, plus the shared per-key delete (`FUN_0043b550` @ `0x0043b550`)
and persist-to-disk (`FUN_0043a9d0` @ `0x0043a9d0`) functions, shows: `InvalidKeys` entries are
cast to `CME::Detail::PropertyNode::Property<long>` and passed through unmodified; the ZIP entry
name is built as `L"_" + operator<<(raw long key)` — plain decimal streaming, no narrowing, no
indexing. The one 16-bit mask in this whole area (`FUN_0047a690`, the `InvalidateAll` flush path)
bounds a *loop position counter*, not a key value, and isn't even reached by this incident's
per-key push. `GameProxyPlayer::HandleOnClientMapLoad` (`0x00df27f0`) and `EntityManager::
PostLoadMap` (`0x00dd0b00`, fires `Event_Level_PostLoad`) were also decompiled fresh: neither
touches Dialog/CookedData at all. **Root cause is still open.** New lead:
`speaker_id: 754` in the same two overrides (`dialog_overrides/mod.rs`) is the first *novel*
speaker id Cimmeria has ever authored (every prior override uses `speaker_id: 0`) — a separate,
undecompiled "speakers" CookedData name table (per `dialog-portrait-lookup.md` Track 2, LOW/MEDIUM
confidence, Lua-side and not reachable from `SGW.exe`) is the next thing to chase if the crash
recurs after the `60100`-`60104` renumber (#938, already merged). Full writeup with all addresses:
`docs/reverse-engineering/findings/cooked-dialog-override-crash.md` — "Verdict" section.

**Ghidra tool-access limitation, and how it was actually solved**: connecting Ghidra mid-session via
GUI+MCP left the 195 dynamically-registered analysis tools unreachable through this agent's
`ToolSearch` (see below for the original account) — but **headless Ghidra sidesteps this
completely**, no MCP bridge involved. See [[headless-ghidra-decompile-workaround]] for the
reusable recipe (script, invocation, gotchas). That memory supersedes the "connect Ghidra before
session start" workaround suggested below — headless is simpler and doesn't require any
pre-session setup at all.

**Original account (superseded by the headless-Ghidra workaround above, kept for context)**:
Ghidra was not running at session start; launched mid-session against the existing project
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
honor `tools/list_changed` must re-list tools after this call" — this harness is such a client. See
also [[reference-mcp-servers]] for the general Ghidra MCP wiring notes (that memory predates this
specific tools/list_changed gotcha).
