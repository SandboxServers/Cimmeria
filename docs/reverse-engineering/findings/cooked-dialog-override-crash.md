# RE Finding: Cooked-Dialog Override Crash on Map Load (dialog ids 100100/100101)

```
Confidence: MEDIUM (timeline reasoning, element-key type) / LOW (root-cause mechanism — see Open Questions)
Last verified: 2026-09-27
Sources:
  - docs/reverse-engineering/decompiled/13_other_game.c:3028 — ServerSource<5,long,Dialog,...> / ZipStorage<5,long,Dialog,...> mangled RTTI
  - docs/reverse-engineering/decompiled/14_standalone_named.c:5139-5151 — same template instantiation, second occurrence
  - docs/reverse-engineering/decompiled/14_standalone_named.c:512789-512967 — CME_UIScreen_UIScreenType_1, the top-level <COOKED_DIALOG> record parser (DialogID/UIScreenType/KismetEventSetID/DialogFlags)
  - docs/reverse-engineering/decompiled/14_standalone_named.c:512881-512967 — CME_UIScreen__unknown_015e4d10 (= FUN_015e4d10), the per-<Screens> parser (ScreenID/Text/SpeakerID, Buttons sub-parse, button-count guard)
  - docs/reverse-engineering/findings/cooked-data-pipeline.md — LibCategory/ServerSource struct layout, versionInfo/InvalidKeys handshake, ZipStorageBase archive path
  - docs/reverse-engineering/findings/dialog-portrait-lookup.md — SpeakerID/ScreenID field offsets, confirms FUN_015e4d10 address
  - crates/base-session/src/base/cooked_data.rs:34-149 — server-side versionInfoRequest handler and push_overridden_elements
  - crates/wire/src/mercury/protocol/resources.rs:70-120 — build_version_info wire layout (InvalidKeys as ARRAY<u32>)
  - crates/resources/src/base/mission_overrides.rs:85-117 — StepID 80623 / ObjectiveID 90623, an existing shipped sub-field > 65535 in a different cooked-data category
  - SigNoz colo telemetry, 2026-09-27 16:19 UTC deploy and the two subsequent login attempts (summarized by team-lead; not independently queried — signoz MCP was unreachable this session)
Related findings: cooked-data-pipeline.md, dialog-portrait-lookup.md, dialog-controller-wire-flow.md
Implementation status: Root cause NOT confirmed. Recommends renumbering (see Recommendation) pending live-decompile confirmation.
```

## Summary

A server deploy pushed two new Cimmeria-authored `CookedDataDialogs.pak` overrides (dialog ids
`100100` and `100101`, category 5) to a tester's client via the standard per-key invalidation
handshake (`onVersionInfo(InvalidKeys=[...])` → `resourceFragment` pushes). The client accepted the
push, acknowledged normally, and reached `Castle_CellBlock`: it processed `CREATE_BASE_PLAYER` and
`onClientMapLoad`, acked both, exchanged two keepalives, then went silent about 4 seconds later and
never sent `mapLoaded`. The process died. On relog — with no push this time, since the client's
locally cached version now matched — the same map load died the same way, confirming the crash is
triggered by data already sitting in the client's writable cooked-data cache, not by the push itself.

**This finding could not reach the binary directly for new decompilation.** Ghidra was not running
at the start of this session; it was launched mid-session against the existing `SGW.gpr` project
(`C:\Users\Steve\source\projects\SGW\Stargate Worlds-QA\Working\binaries\SGW.gpr`) and the GhidraMCP
plugin auto-started on port 8100, so `mcp__ghidra__connect_instance` succeeded. However, the
195 analysis tools the bridge registers on connect (`decompile_function`, `get_xrefs_to`,
`search_functions`, etc.) were not visible to this session's tool-discovery mechanism — the bridge's
own `check_tools` call reported them "callable," but this agent's tool search never surfaced their
schemas, so none of them could actually be invoked. This is exactly the caveat the bridge's own
docstring names: "Clients that cache the initial tools/list and don't honor `tools/list_changed`
must re-list tools after this call." Everything below therefore comes from the **pre-extracted**
decompiled dumps under `docs/reverse-engineering/decompiled/` (produced by an earlier annotation
pass and already checked into the repo) plus the existing findings docs — not from a fresh live
decompile. The debugger-proxy tools (`debugger_attach`, `debugger_read_memory`, etc.) remained
available throughout but require an attached live process, which this investigation intentionally
avoided per the static-analysis-only instruction.

## What is confirmed

### 1. The Dialog category's element key is a 32-bit `long` at the template level, not a 16-bit type

The client's cooked-data cache classes are C++ templates parameterized on category id, key type,
and element type. The mangled RTTI names for the Dialog category (5) spell out the key type
explicitly:

```
Detail::ServerSource<5,long,class Dialog,class CME::RefCountedObj<class Dialog>,
                     class Detail::ZipStorage<5,long,class Dialog,class CME::RefCountedObj<class Dialog>>,
                     struct Event_Cache_ElementReady<long,class Dialog>,
                     struct Event_Cache_ElementError<long,class Dialog>>
```

(docs/reverse-engineering/decompiled/13_other_game.c:3028, and again at
14_standalone_named.c:5139-5151, mangled symbol for
`CME_EventSignal_...Event_Net_ProxyData...ServerSource<5,long,Dialog,...>___MemberCallback__vfunc_3`).

`long` here is the MSVC 32-bit signed integer, matching every other cooked-data category (compare
`ServerSource<3,long,Mission,...>` at 14_standalone_named.c:4827, `ServerSource<4,long,DBInvItem,...>`
at 14_standalone_named.c:4935). This is the element-key type used by `ServerSource`, `ZipStorage`,
`Event_Cache_ElementReady`, and `Event_Cache_ElementError` uniformly — the same class the version
handshake, the request queue (`ServerSource+0x3C..0x40`, per cooked-data-pipeline.md Finding 2), and
the `Event_Net_ProxyData` fragment-delivery callback all key off. There is no template evidence of a
narrower (16-bit) key type anywhere in this chain.

On the Cimmeria side, the wire carries the same width: `build_version_info`
(crates/wire/src/mercury/protocol/resources.rs:86-120) serializes `InvalidKeys` as `ARRAY<u32>`
(4-byte count + 4-byte entries), and `build_resource_fragment` (same file, lines 23-68) serializes
`elementId` as a full `u32`. Neither truncates. The module doc in
crates/base-session/src/mercury/... — actually crates/wire's own comment — states this was
"confirmed to parse `InvalidKeys` as a `PropertyList<long>`" on the client side, consistent with the
RTTI evidence above.

**This is evidence against a blanket "ids above 65535 are silently truncated somewhere in the
general cooked-data cache pipeline" theory.** If the top-level `long` key were narrowed to 16 bits
anywhere in `ServerSource`/`ZipStorage`, every category would be affected, and the mechanism would
need to be a per-use-site narrowing cast rather than a structural template limitation — which is
possible, but not what the template parameterization itself shows.

### 2. The per-field XML parser does not narrow width either, and a sibling field already ships above 65535

The `<COOKED_DIALOG>` record parser (`CME_UIScreen_UIScreenType_1`,
14_standalone_named.c:512789-512873) reads `DialogID` into `piVar2[5]` (offset `+0x14`),
`UIScreenType` into `piVar2[6]` (`+0x18`), `KismetEventSetID` into `piVar2[7]` (`+0x1C`), all via the
generic int accessor `CME_UIScreen__unknown_00a3d050`, and `DialogFlags` into `piVar2+8` (`+0x20`)
via a different accessor (`00a3d190`). The per-`<Screens>` parser
(`CME_UIScreen__unknown_015e4d10` = `FUN_015e4d10`, confirmed by address match against
dialog-portrait-lookup.md, at 14_standalone_named.c:512881-512967) reads `ScreenID` into
`piVar2[5]` (`+0x14`), `Text` into `piVar2[6]` (`+0x18`, via the literal at `DAT_01b23c98`), and
`SpeakerID` into `piVar2[7]` (`+0x1C`) — `ScreenID` and `SpeakerID` go through the *same*
`00a3d050` int accessor as `DialogID`. Nothing in these functions truncates to 16 bits; they read a
plain 32-bit int and store it at a 4-byte-aligned struct offset.

More directly: Cimmeria already ships a cooked-data sub-field above 65535 without incident.
`crates/resources/src/base/mission_overrides.rs:96` injects `StepID="80623"` and
`ObjectiveID="90623"` into a `Mission` (category 3) cooked-data override, and this has been live
since the mission-622 loot-split work with no reported crash. `StepID`/`ObjectiveID` are read by the
Mission category's own generic-int accessor, the same family of function as the Dialog category's
`ScreenID`/`SpeakerID`/`DialogID` accessors (`CME_UIScreen__unknown_00a3d050` and siblings are shared
infrastructure across `CME_UIScreen`-templated categories, not per-category code). **This rules out
"any 32-bit-but-large integer field crashes the generic XML-attribute parser" as the mechanism** —
that class of bug would have already surfaced on `StepID=80623`.

What is *not* ruled out is something specific to the **top-level element key** — i.e., `DialogID`
acting as the map/cache identity for the whole record, the ZIP archive entry name
(`_<dialogId>`, per cooked-data-pipeline.md's Finding 6/description), or a Dialog-specific index
(e.g., the `speakers` name table that `dialog-portrait-lookup.md`'s Track 2 describes as a
CookedData-time lookup) — as opposed to `StepID`, which is just a field value carried inside an
already-keyed Mission record. `100100`/`100101` are element keys; `80623` is not.

### 3. Timeline reasoning rules out a crash during XML-to-record parsing

The SigNoz sequence (team-lead's summary; the signoz MCP endpoint was unreachable this session, so
this investigation could not query it directly and treats the summary as given, not independently
verified) has the fragments for `100100`/`100101` pushed and received **before** the tester entered
`Castle_CellBlock`. The client then successfully processed `CREATE_BASE_PLAYER`, `onClientMapLoad`,
acked both, and exchanged two keepalives — several seconds of otherwise-normal operation — before
going silent. Parsing the pushed XML into the in-memory `Dialog` record (the step where
`CME_UIScreen_UIScreenType_1` and `CME_UIScreen__unknown_015e4d10` run, including the `<Buttons>`
sub-parse for `100100`) happens as part of receiving and caching the `resourceFragment` chain — at
login, not at map load. Since the client visibly survived past that point into normal map-load
processing, **the record-level parse of both dialogs, buttons included, completed without
crashing.**

This is the strongest piece of reasoning in this finding, and it points away from the authored
`<Buttons>` markup on `100100` as the cause: if the button XML shape itself were fatal, the client
would have died at login while parsing the pushed fragment, not four seconds into a later,
unrelated map load.

## What is not confirmed (and could not be confirmed this session)

1. **What code touches Dialog-category elements at `onClientMapLoad`, and whether it iterates the
   whole category-5 cache or just the current map's NPC roster.** The debug-hub NPC that owns
   dialogs `100100`/`100101` (template 302, "Airman Lance," per project memory) is not placed in
   `Castle_CellBlock` — the crash reproduced on a map that has no reason to reference these two
   dialog ids by content. That means, if the crash really is these two entries, whatever touches
   them at map load must run unconditionally on every map load rather than being scoped to the
   dialogs a given map's NPCs actually use. No decompiled function for this step was found in the
   pre-extracted dumps (`grep` for `onClientMapLoad`/`ClientMapLoad` in
   `docs/reverse-engineering/decompiled/*.c` only turns up CME event-registration boilerplate, not
   the handler body), and this session could not reach a fresh Ghidra decompile to trace it (see
   the tool-access limitation in Summary).

2. **Whether the mechanism is a 16-bit-width narrowing at all**, as opposed to some other id-keyed
   structure sized for the historical range (shipped `DialogID`s top out around 5,861 per the
   2026-09-21 PAK census recorded in the dialog-UI reference memory; Cimmeria's own prior overrides
   only reach `3996`). A fixed-size table sized for "a few thousand entries" would break on
   `100100` for a completely different reason than a 16-bit truncation would, and the evidence in
   this finding cannot distinguish the two: it only shows the key type is *declared* `long`
   end-to-end, not that every consumer of that key actually allocates storage proportional to its
   value safely.

3. **Whether `ScreenID`s in the 200000 range (`200000`–`200002`, used by these same two overrides)
   are also implicated.** They go through the identical `00a3d050` accessor as `SpeakerID`/
   `DialogID`, so nothing here singles them out, but they are equally novel (no shipped or
   previously-Cimmeria-authored `ScreenID` has used six digits) and were not tested independently
   of the `DialogID` values.

## Recommendation

Given (a) the timeline evidence that rules out the button markup, and (b) the complete absence of
any prior Cimmeria or shipped-game precedent for a cooked-data **element key** above 65535 (as
distinct from a sub-field, which `StepID=80623` already proves safe) — the leading, unconfirmed
hypothesis is that something keyed specifically by the top-level `DialogID` (not by any inner
field) breaks above some threshold at or below 65,536. This is consistent with, but does not prove,
a 16-bit boundary.

**Renumbering to `60100`–`60104` is a reasonable mitigation and should proceed** — it sidesteps the
16-bit-boundary hypothesis specifically. It is not a *proven*-safe range: it is still an order of
magnitude above every shipped or previously-authored `DialogID` (max ~5,861 shipped, `3996` prior
Cimmeria max), so if the real mechanism is a small fixed-size table sized closer to the historical
maximum rather than a hard 16-bit cliff, `60100`–`60104` would not fix it either. Treat the
renumbering as a plausible fix pending confirmation, not a closed issue: smoke-test the debug-hub
dialog chain (both a login with `InvalidKeys` push and a cold relog against an already-cached
client) before calling this closed, and do not pick a new "just under 65536" id for any *other*
cooked-data category without the same caveat, since every category shares the same `ServerSource`/
`ZipStorage` template family.

## Next steps for a follow-up RE session

1. Get a Ghidra session where the dynamically-registered analysis tools are actually reachable —
   either restart the agent session after Ghidra is already connected (so the tool list is built
   post-connect rather than pre-connect), or have a human click
   `Tools > GhidraMCP > Start MCP Server` in an already-open CodeBrowser before the agent session
   starts, then verify with `check_tools` *and* a live call (not just the "callable" status) before
   relying on it.
2. With live decompile access, find the `onClientMapLoad`/`Event_NetIn_onClientMapLoad` handler body
   (not just its CME event-registration wrapper) and trace what, if anything, touches category-5
   Dialog elements unconditionally.
3. Decompile the caller of `Detail::ZipStorageBase::WriteStreamToFile`
   (docs/reverse-engineering/decompiled/14_standalone_named.c:15429-15520 has the callee body, which
   takes an already-built `wchar_t*` filename — the caller that formats `_<dialogId>` was not found
   in the pre-extracted dumps) to confirm whether the ZIP entry name, or any in-memory index keyed
   directly by `DialogID` value (as opposed to a `std::map`/tree lookup), is where a large id
   actually causes trouble.
4. If time allows, reproduce with a single test id in the `60000`–`65535` range as a smoke test on a
   throwaway character before shipping the `60100`–`60104` renumber broadly.

## Cross-references

- `docs/reverse-engineering/findings/cooked-data-pipeline.md` — category table, `LibCategory`/
  `ServerSource` struct layout, `ZipStorageBase` archive path this finding builds on
- `docs/reverse-engineering/findings/dialog-portrait-lookup.md` — `SpeakerID`/`ScreenID` field
  offsets and the `FUN_015e4d10` address this finding cites
- `docs/reverse-engineering/findings/dialog-controller-wire-flow.md` — the display-time path
  (`onDialogDisplay`, `IsImmediate`) that this finding's timeline reasoning depends on *not* being
  where the crash happens
- `crates/resources/src/base/dialog_overrides/mod.rs` — the Rust generator for the two dialogs in
  question
- `crates/base-session/src/base/cooked_data.rs` — the server-side push path
