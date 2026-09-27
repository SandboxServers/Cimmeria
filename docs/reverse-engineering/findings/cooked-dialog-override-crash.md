# RE Finding: Cooked-Dialog Override Crash on Map Load (dialog ids 100100/100101)

```
Confidence: HIGH (the >65535 / 16-bit element-key hypothesis is REFUTED for the traced pipeline — live decompile) / LOW (true root cause — still open)
Last verified: 2026-09-27 (updated same day with headless-Ghidra live decompile results)
Sources:
  - Live headless-Ghidra decompile, 2026-09-27 (analyzeHeadless against SGW.gpr, -readOnly -noanalysis):
    FUN_004435c0 @ 0x004435c0 (category-5 `onVersionInfo` handler, confirmed by its `CategoryId != 5` guard)
    FUN_00443a30 @ 0x00443a30 (category-5 `onCookedDataError` handler, confirmed by its `categoryID != 5` guard)
    FUN_0043c2b0 @ 0x0043c2b0 (category-5 element-commit function, called from FUN_004435c0's pending-vector walk)
    FUN_0043b550 @ 0x0043b550 (shared per-key invalidate/delete function, called by every category's onVersionInfo)
    FUN_0043a9d0 @ 0x0043a9d0 (shared "_<key>" ZIP-entry-name builder + persist-to-disk function)
    FUN_0047a690 @ 0x0047a690 (shared InvalidateAll flush function — NOT reached by this incident's per-key path)
    Detail_ZipStorageBase_OpenArchive_1 @ 0x00479340, Detail_ZipStorageBase_WriteStreamToFile_1 @ 0x00479930,
    FUN_00479e10 @ 0x00479e10 (WriteMetaDataVersion)
    FUN_00df27f0 @ 0x00df27f0 (`GameProxyPlayer::HandleOnClientMapLoad`) and BW_client_entity_manager_2 @ 0x00dd0b00
    (`EntityManager::PostLoadMap`) — both decompiled fresh and confirmed to have zero Dialog/CookedData references
    FUN_0044e5d0 @ 0x0044e5d0 (category-5 LibCategory CME-subscription ctor, confirms FUN_004435c0/FUN_00443a30 wiring)
    FUN_0044f680 @ 0x0044f680 (full category-5 LibCategory ctor: vtable stamp + FUN_0044e5d0 + LibCategoryBase_Ctor, no element enumeration)
    LAB_0044a680 (force-created as FUN_0044a680) @ 0x0044a680 (category-5 Event_Entity_ProxyPlayerBaseCreated handler: fires a second versionInfoRequest)
    docs/reverse-engineering/decompiled/14_standalone_named.c:5079-5169 (all six category-5 MemberCallback instantiations; grep for external Event_Cache_ElementReady<long,Dialog> subscribers returns zero matches)
  - docs/reverse-engineering/decompiled/13_other_game.c:3028, 14_standalone_named.c:5139-5151 — `ServerSource<5,long,Dialog,...>` / `ZipStorage<5,long,Dialog,...>` mangled RTTI (key type is `long`, confirmed structurally by the live decompile above)
  - docs/reverse-engineering/decompiled/14_standalone_named.c:512789-512967 — `CME_UIScreen_UIScreenType_1` / `FUN_015e4d10`, the record and per-`<Screens>` parsers
  - docs/reverse-engineering/findings/cooked-data-pipeline.md, docs/reverse-engineering/findings/dialog-portrait-lookup.md, docs/reverse-engineering/findings/world-entry-pipeline.md (`GameProxyPlayer_HandleEvent_Level_PostLoad`/`FUN_00de8430`, `Event_World_Loaded` subscribers)
  - crates/base-session/src/base/cooked_data.rs:34-178, crates/wire/src/mercury/protocol/resources.rs:70-120, crates/resources/src/base/mission_overrides.rs:85-117, crates/resources/src/base/resources/mod.rs:195-317, crates/resources/src/base/resources/metadata_bump.rs:81-109
  - SigNoz colo telemetry, 2026-09-27 16:19 UTC deploy and the two subsequent login attempts (summarized by team-lead; not independently queried — signoz MCP was unreachable this session)
Related findings: cooked-data-pipeline.md, dialog-portrait-lookup.md, dialog-controller-wire-flow.md, world-entry-pipeline.md
Implementation status: **RE-OPENED, 2026-09-27 (same day).** The >65535 element-key-width
hypothesis stays REFUTED (see "Verdict" below) — the tester crashed again at 17:52 on build
`38296335f` (the `#938` renumber to `60100`-`60104`), confirming id value is not it. This finding
was briefly closed as "not the cause" on the theory that CREATE_BASE_PLAYER/`onClientMapLoad`
carried a regressed payload instead, but `world-entry-bisect` (a parallel worker) then confirmed
those bytes are **byte-identical** between the good build (`707950271`) and every bad build back
to `b22907eb5` — there is no world-entry payload regression. SigNoz also confirms the crashing
client's on-disk `CookedDataDialogs.pak` still holds `100100`/`100101` (never invalidated by any
build, `#938` included) and that dialogs are the *only* thing that changed for this client since
its last good session. **Dialogs are back as the carrier — the open variable is which *content*
field, not which id.** New candidates per team-lead: nonzero `speaker_id` (754/843, vs. every
working override's `speaker_id: 0`), a **two-screen** dialog (vs. one screen), `ScreenID`s in the
`200000`-`200005` range (vs. `96108`-`96109` max previously), and button **type 4**. See "Round 2"
below for what this session's headless-Ghidra work found and ruled out chasing these.
```

## Verdict (2026-09-27, after live headless-Ghidra decompile)

**The >65535 / 16-bit element-key-width hypothesis is refuted.** A follow-up session got live decompile access via headless Ghidra (`analyzeHeadless.bat ... -readOnly -noanalysis -postScript`, no GUI, no MCP) and traced every function this finding's Open Questions named. The first decompile pass in that session hit `cooked-data-pipeline.md`'s two documented addresses (`0x00441630`/`0x00441aa0`), which turned out to be the **category-6** (`CookedDataKismetSetEvent.pak`) instantiations, not category 5 — each has a `CategoryId`/`categoryID` literal-compare guard baked into the decompile (`!= 0x6`), a fact `cooked-data-pipeline.md`'s own `_cat6` suffix on those symbol names already flagged, but which this earlier finding's write-up had glossed over by treating that doc's RTTI-only evidence as sufficient. Both categories' code is generated from the same C++ template, so the RTTI evidence in "What is confirmed" §1 below was never wrong about the *type*, but the mechanism claims needed the actual category-5 instantiation to be load-bearing. Locating it (`FUN_004435c0` and `FUN_00443a30`, confirmed via their own `!= 0x5` guards, found by walking the 21 callers of the shared per-key delete function `FUN_0043b550`) let this session trace the real category-5 path end to end. None of it narrows, masks, or otherwise mistreats the element-key value:

- The category-5 `onVersionInfo` handler (`FUN_004435c0`) reads `InvalidKeys` as a `CME::Detail::PropertyNode::Property<long>` list (RTTI-cast confirmed in the decompile) and passes each entry's raw `long` value to the shared per-key delete function unmodified.
- The shared per-key delete function (`FUN_0043b550`) and the shared persist-to-disk function (`FUN_0043a9d0`) both build the ZIP entry name identically: a `wostringstream`, write the literal `L'_'`, then `operator<<` the raw `long` key — the C++ standard integer-stream insertion operator, which formats up to 10 decimal digits with no width limit. `"_100100"` is exactly as valid a name to this code as `"_5861"`.
- The category-5 element-commit function (`FUN_0043c2b0`, reached from `FUN_004435c0`'s pending-request-vector walk) delegates straight to the same shared `FUN_0043a9d0` — no category-specific narrowing layered on top.
- The one place a 16-bit mask (`& 0xffff`) actually appears is inside `FUN_0047a690`, the **`InvalidateAll` flush** path — and it masks a **loop position counter** over a per-category sub-vector (bounding iteration at 65536 *positions*, not element-key *values*), not the key. This incident's push used `invalidate_all = false` (per-key `InvalidKeys`, per `crates/base-session/src/base/cooked_data.rs`'s three-way response logic), so `FUN_0047a690` never ran for this incident regardless.
- `GameProxyPlayer::HandleOnClientMapLoad` (`FUN_00df27f0`, the actual `onClientMapLoad` method-117 handler) and `EntityManager::PostLoadMap` (`BW_client_entity_manager_2` @ `0x00dd0b00`, the UE3 terrain-streaming completion callback that fires `Event_Level_PostLoad`) were both decompiled fresh: neither touches Dialog data, CookedData, or any `ServerSource`/`ZipStorage` method. `HandleOnClientMapLoad` only reads `areaName`/`mapPath`/`WorldID`/`Location`/`Direction` and kicks off the UE3 level-streaming request (`L"127.0.0.1/" + mapPath + L".umap"`). `Event_Level_PostLoad`'s only known subscriber body (`GameProxyPlayer_HandleEvent_Level_PostLoad`/`FUN_00de8430`, per `world-entry-pipeline.md`) only touches player-controller input mode and vehicle/mount transforms.

So: the wire type, the cache key type, the ZIP entry-name formatting, the persist-to-disk path, and the two map-load-lifecycle handlers this session could actually trace are all clean. **Root cause for why `100100`/`100101` specifically crash the client remains open** — see "What is not confirmed" below, now sharpened with a new lead (`SpeakerID=754`) and a shorter list of remaining places to look.

## Round 2 (2026-09-27, same day): dialogs confirmed as carrier by content, not id — ruling out three more mechanisms, one new lead

After `#938` (renumber to `60100`-`60104`) shipped and the tester crashed again, `world-entry-bisect`
(a parallel worker diffing server-side wire bytes) reported two facts that reframe this finding:
(1) `CREATE_BASE_PLAYER`/`onClientMapLoad`/time-sync are **byte-identical** between the good build
(`707950271`) and every bad build — there is no world-entry payload regression to chase; (2) SigNoz
confirms the crashing client's on-disk cache still holds `100100`/`100101` (never invalidated by
any build so far) and that dialogs are the only thing that changed for this client since its last
good session. So dialogs are confirmed as the carrier again — the open question is which *content*
field differs from every dialog that has worked fine for days (`3995`/`3996`, `speaker_id: 0`, one
screen), not the id. Team-lead named four candidates: nonzero `speaker_id` (754/843), a two-screen
dialog, `ScreenID`s in `200000`-`200005` (vs. `96108`-`96109` max previously), and button type 4.

This session used headless Ghidra to chase "does anything walk the cached Dialog records after
`onClientMapLoad`, resolve `SpeakerID` via a fixed table, or index by `ScreenID`." Three more
mechanisms were checked and ruled out; one new, unverified lead was found:

**Ruled out — the category-5 `LibCategory` constructor does not enumerate persisted elements.**
`FUN_0044f680` @ `0x0044f680` (found as the caller of `FUN_0044e5d0`, the CME-subscription wiring
from the Verdict above; confirmed by its own `LibCategory<LibCategoryKey<5,long,Dialog,...>>::vftable`
stamp) does exactly three things: stamp the vtable, wire the CME subscriptions, call the shared
`LibCategoryBase` base-constructor (`FUN_004786e0`) to set the category id. No enumeration of
already-cached or newly-opened ZIP entries happens at construction time — team-lead's "does
`ServerSource` load of a persisted category do a post-load pass over all elements" is answered: no,
not here.

**Ruled out — nothing external subscribes to "a Dialog element became ready."** Every category-5
`MemberCallback` instantiation in the pre-extracted decompiled dumps (`14_standalone_named.c:5079`-
`5169`) is `Detail::ServerSource<5,...>` subscribing to something *else* (`Event_Net_Connected`,
`Event_Entity_ProxyPlayerBaseCreated`, `Event_Net_Disconnected`, `onVersionInfo`, `Event_Net_ProxyData`,
`onCookedDataError`) — a search for any *other* class subscribing to
`Event_Cache_ElementReady<long,Dialog>` or `Event_Cache_ElementError<long,Dialog>` as its event of
interest (`grep -n "),struct_Event_Cache_ElementReady<long,class_Dialog>"` across every pre-extracted
dump) returns **zero matches**. Compare category 6 (`CookedKismetEventSetData`), where
`DialogController`'s `FUN_00d25310` *does* subscribe to that category's `ElementReady` (per the
correction in `dialog-portrait-lookup.md`). Category 5 has no such consumer. This is consistent
with dialogs being read lazily, only at `Event_NetIn_DialogDisplay` time (the wire path
`dialog-controller-wire-flow.md` and `dialog-portrait-lookup.md` already trace) — there is no eager
map-load-time reader of freshly-cached Dialog data via the CME event system.

**Ruled out — no Castle_CellBlock content references these dialog ids.**
`grep -rn "100100\|100101\|60104" db/resources/` finds them only in `debug_hub_chains.sql` (space
12, the debug hub) and the matching `dialogs.sql`/`dialog_screens.sql`/`entity_templates.sql` rows —
nothing in Castle_CellBlock's own seed data (missions, entity templates, dialog_set_maps) points at
them. The debug-hub NPC (template 302, "Airman Lance") is not placed in Castle_CellBlock and nothing
there has a reason to touch these records by content.

**New, unverified lead: `Event_Entity_ProxyPlayerBaseCreated` fires a *second* `versionInfoRequest`
per category, right around `CREATE_BASE_PLAYER` time.** Category 5's sixth CME subscription (found
via `14_standalone_named.c:5097`, the `Event_Entity_ProxyPlayerBaseCreated` instantiation, missed in
the first pass because the Verdict above only decompiled five of the six subscribe calls in
`FUN_0044e5d0`) wires handler `LAB_0044a680`. Force-decompiled (`0x0044a680`): it builds and fires
*another* `versionInfoRequest(CategoryId=5, Version=this+0x24)`, then re-subscribes itself to the
same event. This is stock 2009 client behavior (not something Cimmeria added), and it fires whenever
the client's base-player entity is (re)created — i.e., a second version check right around
`CREATE_BASE_PLAYER`, in addition to the one `Event_Net_Connected` already sent at connect time.
**This is a real, newly-documented mechanism, but not a confirmed cause**: `handle_version_info_request`
should treat it as a no-op once the client's locally-persisted version already matches the server's
(which the version write inside `onVersionInfo` sets synchronously, before this second request would
normally arrive) — so on paper this second request changes nothing. The only way it matters is if the
two requests race (the second one reaching the server before the first's version-bump write has
"taken" client-side, or before the first push's fragments have all landed), in which case the server
could re-push the *same* `InvalidKeys`/fragments a second time in quick succession, and two
overlapping resource transfers for the same category could plausibly corrupt a ZIP entry if
`next_data_id`/fragment interleaving isn't safe under that overlap. **This session could not verify
whether such a race actually occurs** (would need a live network trace showing two
`onVersionInfo`/push cycles per login, which `world-entry-bisect` is better placed to check in
SigNoz than a static decompile can settle) — flagging it as a lead, not a finding. If SigNoz shows
only one version transition and one push per session (as the evidence so far suggests — `44069 ->
37653` mentioned once, not twice), this lead is probably a dead end and the mechanism is elsewhere
(most likely in the client's speaker-name/portrait resolution or a screen-navigation index that this
session did not reach — the `Event_Level_PostLoad` subscribers `GameAppearanceManager` (RTTI
`0x00e9a480`) and `Minimap`/`GameProxyPlayer`'s `Event_World_Loaded` handlers (RTTI `0x00e2af30`/
`0x00df7b80`) remain untraced; finding their handler bodies needs the vtable slot arithmetic this
session didn't have time to work out — see Next Steps).

## Summary

A server deploy pushed two new Cimmeria-authored `CookedDataDialogs.pak` overrides (dialog ids
`100100` and `100101`, category 5) to a tester's client via the standard per-key invalidation
handshake (`onVersionInfo(InvalidKeys=[...])` → `resourceFragment` pushes). The client accepted the
push, acknowledged normally, and reached `Castle_CellBlock`: it processed `CREATE_BASE_PLAYER` and
`onClientMapLoad`, acked both, exchanged two keepalives, then went silent about 4 seconds later and
never sent `mapLoaded`. The process died. On relog — with no push this time, since the client's
locally cached version now matched — the same map load died the same way, confirming the crash is
triggered by data already sitting in the client's writable cooked-data cache, not by the push itself.

**This session got live decompile access via headless Ghidra** — see "Verdict" above for the
result. The original pass (below, still preserved for its correct parts and its explicit
correction) could not reach the binary directly: Ghidra was not running at the start of that
session, and although it was launched mid-session and `mcp__ghidra__connect_instance` succeeded,
the 195 dynamically-registered MCP analysis tools were never reachable through that session's
tool-discovery mechanism. The follow-up session that produced the Verdict above used
`analyzeHeadless.bat` directly instead — no GUI, no MCP bridge, just a `GhidraScript` run
non-interactively against the read-only project — which sidesteps that limitation entirely and is
the recommended path for any future session that needs live decompile without a GUI/MCP already
running.

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

**Confirmed by live decompile, 2026-09-27 (see "Verdict" above).** The category-5-specific
`onVersionInfo` handler (`FUN_004435c0` @ `0x004435c0`) casts each `InvalidKeys` entry to
`CME::Detail::PropertyNode::Property<long>` (an explicit `__RTDynamicCast` against that exact RTTI
descriptor in the decompiled body) and passes the raw `long` value through unmodified to the shared
delete function. This is the same "per-use-site narrowing cast" this paragraph flagged as the only
way the RTTI-only evidence could still hide a bug — checked, and it isn't there.

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

### 4. The renumbering fix does NOT clean an already-affected client — the stale entries persist on disk and the version handshake never revisits them

Added 2026-09-27 in response to a question from the `fix/dialog-ids-below-65536` renumber work.

**Yes, the client persists pushed cooked-dialog fragments to a writable local PAK, the same way it
already does for the Kismet-sequence category** (`crates/base-session/src/base/cooked_data.rs:105`'s
comment about `Cache.en-US/CookedDataMissions.pak` describes the identical mechanism for Missions;
`cooked-data-pipeline.md` Finding 4 and Finding 6 give the binary-side confirmation: `onVersionInfo`
calls `ServerSource_SetVersion` → `ZipStorageBase_WriteMetaDataVersion` (`0x00479e10`) to persist the
category's version stamp, and each received `resourceFragment` gets written into the category's ZIP
archive via `ZipStorageBase_WriteStreamToFile`). **This finding's own incident is itself the proof**:
on the tester's second login attempt, no fragments were pushed at all (`client_version` already
matched `server_version`), yet the exact same crash reproduced — that is only possible if the
`_100100`/`_100101` entries written during the first, crashing session survived the crash and were
still present when the client reopened its local `CookedDataDialogs.pak` archive on the next login.

**Where in the pipeline this happens — receipt, not "cache load" in the startup sense.** The version
stamp write happens synchronously inside the `onVersionInfo` handler, before any element fragments
even arrive (`cooked-data-pipeline.md` Finding 4, step 4 of the flow) — i.e., at connection time,
well before the tester ever reached `Castle_CellBlock`. The per-element `_100100`/`_101` writes
happen as each `resourceFragment` chain completes, also during the login sequence, also before the
map load that actually crashes. Neither of those write points is the crash site (see finding #3
above — the client visibly survives past both of them). The crash itself happens later, during
`onClientMapLoad` processing on `Castle_CellBlock` — a *third*, distinct point in the pipeline that
this session could not decompile (see Open Question 1). So to directly answer "fragment receipt or
cache load": **the disk write happens at fragment receipt (confirmed); the crash happens at a
separate, later map-load-time consumption of the cache (confirmed by timing; the specific code path
is NOT confirmed)** — it is not the initial PAK-file-open-at-startup step either, since the client's
normal boot (reading its already-existing local cache) does not crash on its own; something specific
to entering a map re-touches category 5.

**Why the renumber alone will not fix an already-affected client.** The server's per-category
version bump (`crates/resources/src/base/resources/metadata_bump.rs`,
`compute_dialog_metadata_bump`) is a hash over every field of the *current*
`DIALOG_OVERRIDES`/`DIALOG_PATCH_TABLES` content. Renumbering `100100`/`100101` to `60100`-`60104`
changes that hash, so an affected client (whose locally-stored version reflects the *old* hash, from
before the fix) will mismatch on its next login and receive a fresh `onVersionInfo`. But
`ResourceCache::overridden_elements()` (`crates/resources/src/base/resources/mod.rs:312-317`) only
returns the ids present in the *current* override table — `60100`-`60104` (and any other dialog ids
still in `DIALOG_OVERRIDES`/patches) — never `100100`/`100101`, because those ids no longer exist
anywhere in the server's override data once the fix lands. The resulting `InvalidKeys` array
therefore tells the client to drop and refetch `60100`-`60104`; it says nothing about `100100`/
`100101`, so the client's writable cache keeps those two stale, still-present entries indefinitely.
Once this one mismatch-triggered handshake completes, `client_version == server_version` again (the
version write happens as soon as `onVersionInfo` is processed, matching the pattern in finding
above), and every subsequent login takes the "versions match → no keys, no push" branch of
`handle_version_info_request` (`crates/base-session/src/base/cooked_data.rs:70-88`) forever — the
server has no mechanism to single out a key that isn't in `overridden_elements` any more, and
`handle_version_info_request`'s branch structure only reaches `invalidate_all=true` when
`overridden_elements(category_id)` is *empty* for that category (branch 4), which is never true for
Dialogs (3995/3996/60100-104 keep it non-empty). **If the map-load crash really is triggered by the
mere presence of `_100100`/`_101` in the client's cache (as opposed to something transient at the
first receipt), any tester who already got the bad push before the fix ships will keep crashing on
every future map load, on every future login, until that client's local
`Cache.en-US/CookedDataDialogs.pak` is manually cleared** — the server-side renumber only prevents
*new* exposure, it does not remediate testers already exposed.

Two mitigations worth considering, neither implemented as part of this finding:
- Ask the affected tester(s) to delete their local writable cooked-data cache (the exact path was
  not confirmed this session — `cooked-data-pipeline.md`'s `SourceCachePath` INI note is the
  starting point) before their next login.
- A one-time server-side `invalidate_all=true` for category 5 would force every client to drop its
  *entire* local Dialog cache and lazily re-request everything via `elementDataRequest`, which would
  naturally never re-request `100100`/`100101` since nothing (client-side content or server data)
  names them any more. The current `handle_version_info_request` logic has no code path that reaches
  `invalidate_all=true` while the category still has any overrides configured — it would need a
  deliberate one-time override (e.g., a temporary flag or a "categories to force-invalidate once"
  list) to use this path without giving up per-key scoping for the categories that don't need it.

### 5. Listing a key in `InvalidKeys` with no follow-up fragment is a clean tombstone, not a hang — with one loose end

Added 2026-09-27, in response to a follow-up question from `dialog-renumber` about whether a
server-side tombstone cleanup (list `100100`/`100101` in `InvalidKeys` once, without ever pushing
a fragment for them) is viable as an alternative to asking testers to clear their local cache.

**Answer: the client deletes the entry and does not refetch it.** Two pieces of evidence, both
already in this repository (no fresh decompile needed):

1. The doc comment on `build_version_info`
   (`crates/wire/src/mercury/protocol/resources.rs:80-85`) states the client side of this branch
   "was confirmed to parse `InvalidKeys` as a `PropertyList<long>` and per-key invalidate via the
   cache element's destructor" — i.e., each listed id gets its cached element object destroyed
   immediately on receipt of the list, not lazily on some later access. This claim predates this
   session and this session did not re-derive it from a fresh Ghidra trace, so treat the mechanism
   ("destructor," specifically) as MEDIUM confidence — but it agrees with point 2, which is a
   direct behavioral observation, not a decompile claim.
2. `crates/base-session/src/base/cooked_data.rs:100-107`'s comment on `required_updates` records a
   **prior real incident on this exact code path**, for the Missions category: "the runtime cache
   doesn't actually issue [`elementDataRequest`] — it just drops the local entry on `InvalidKeys`
   and waits for our push. Without this, the client's `Cache.en-US/CookedDataMissions.pak` is left
   with the entries removed but never replaced — symptom: missions stop being granted." That is the
   tombstone scenario already having happened once, by accident, in production: entries named in
   `InvalidKeys` got deleted from the local cache, the client never chased them with a refetch, and
   the *only* observed consequence was that the deleted data was gone — no hang, no crash, no other
   reported side effect. This matches the Kismet `invalidate_all` precedent `dialog-renumber`
   already found for 2026-09-20, but is closer evidence: it is the **per-key** path, the same one a
   tombstone push for `100100`/`100101` would use, not the blanket `invalidate_all` path.

**The loose end: `RequiredUpdates`.** `handle_version_info_request` sets the wire `RequiredUpdates`
field to `invalid_keys.len()` (`crates/base-session/src/base/cooked_data.rs:108`), and the client
stores that count at `ServerSource+0x48` (`cooked-data-pipeline.md` Finding 2/4). The only decrement
path found in the existing findings is `onCookedDataError`
(`cooked-data-pipeline.md` Finding 5: "Decrements `this+0x48`... if nonzero"), which fires on a
server-sent `Event_NetIn_onCookedDataError`, not automatically. A tombstone implementation that adds
`100100`/`100101` to `InvalidKeys` (so the count is right for the "real" pushes too) but never sends
either a `resourceFragment` **or** an `onCookedDataError` for those two keys would leave
`RequiredUpdates` permanently elevated by 2 for that client. The Missions incident above didn't
report a distinct symptom from this (its bug report was specifically "missions stop being granted,"
i.e., data absence, not a stuck-loading state), which is *some* evidence `RequiredUpdates` sitting
nonzero has no other consequence by itself — but that is inferred from a bug report's silence, not
from tracing what (if anything) gates on `RequiredUpdates == 0`, so treat it as LOW confidence.
**A conservative tombstone implementation should send `onCookedDataError` for `100100` and
`100101` explicitly** (categoryID=5, elementKey=each) right after the version-mismatch reply, to
zero the counter cleanly rather than relying on that gap being harmless.

One more implementation note for whoever builds this: `push_overridden_elements`
(`crates/base-session/src/base/cooked_data.rs:159-178`) already skips-and-warns on any key its
`cache.get()` returns `None` for ("element missing from cache, skipping") — which is exactly what
happens for a removed override id today. If `ResourceCache::overridden_elements()` is extended with
a small "categories to also tombstone once" list that unions `100100`/`100101` into the `InvalidKeys`
sent to affected clients without adding them back to `categories[5].elements`, the existing
skip-on-missing-data branch already does the right thing for the push side — the only new code
needed is that union, the `onCookedDataError` follow-up above, and (ideally) a way to stop
advertising the tombstone once telemetry shows no more clients report the old version.

## What is not confirmed (and could not be confirmed this session)

1. ~~**What code touches Dialog-category elements at `onClientMapLoad`.**~~ **RESOLVED, 2026-09-27
   (live decompile): nothing does, at least not in the traced call chain.**
   `GameProxyPlayer::HandleOnClientMapLoad` (`0x00df27f0`, the method-117 handler itself) and
   `EntityManager::PostLoadMap` (`0x00dd0b00`, the UE3 terrain-streaming completion callback fired
   afterward) were both decompiled fresh and touch no Dialog/CookedData/`ServerSource` code —
   see "Verdict" above. The debug-hub NPC that owns `100100`/`100101` (template 302, "Airman
   Lance") is not placed in `Castle_CellBlock`, and the traced map-load path gives no reason it
   would be referenced there either: nothing map-load-specific iterates the whole category-5
   cache, at least not through `HandleOnClientMapLoad` → `PostLoadMap` → `Event_Level_PostLoad`.
   That chain is now closed as a lead. **What still isn't traced**: `Event_Level_PostLoad`'s
   *second* subscriber, `GameAppearanceManager` (RTTI accessor `0x00e9a480`, per
   `world-entry-pipeline.md`) — only `GameProxyPlayer`'s handler body was confirmed clean.
   `Event_World_Loaded` (fired once per full streaming settle, subscribers `GameProxyPlayer` again
   and `Minimap`) was not decompiled this session either.

2. ~~**Whether the mechanism is a 16-bit-width narrowing.**~~ **REFUTED, 2026-09-27** — see
   "Verdict." The one 16-bit mask that exists (`FUN_0047a690`) bounds a positional loop counter
   over a per-category sub-vector, not the key value, and isn't reached by this incident's
   per-key `InvalidKeys` path anyway. Neither the per-key delete path nor the persist-to-disk path
   nor the category-5 element-commit function shows any narrowing, indexing, or size-derived
   allocation keyed by the raw `DialogID` value. A "fixed-size table sized for the historical
   range" sub-theory (as distinct from a hard 16-bit cliff) is also now weaker than before: the
   persist/lookup path this session traced is string-keyed (ZIP entry names) and the pending-
   request vector is walked linearly, not indexed by key — neither shape allocates storage
   proportional to the key's numeric value. **This does not prove no such table exists anywhere
   in the client — only that none of the paths this session could reach has one.**

3. **Whether `ScreenID`s in the `200000` range (`200000`–`200002`, used by these same two
   overrides) are implicated.** Unchanged from the first pass: they go through the identical
   `00a3d050` accessor as `SpeakerID`/`DialogID` (confirmed again in the live decompile of
   `CME_UIScreen_UIScreenType_1`/`FUN_015e4d10`, which this session did not need to re-run since
   the pre-extracted dump already covers it), so nothing singles them out, but they are equally
   novel and were not tested independently of the `DialogID` values.

4. **New lead, 2026-09-27: `SpeakerID=754`.** `screen.speaker_id: 754` in
   `crates/resources/src/base/dialog_overrides/mod.rs`'s `100100`/`100101` entries is, as far as
   this session could check, the first time Cimmeria has authored a **novel** speaker id — every
   prior Cimmeria dialog override uses `speaker_id: 0` (narrator/system line). `dialog-portrait-
   lookup.md`'s Track 2 describes a *separate* "speakers" CookedData name table that Lua looks up
   by `SpeakerID` at **display time** — this table's implementation was never decompiled (Track 2
   itself is LOW/MEDIUM confidence, inferred from symptom observation, and the display-time Lua
   isn't reachable from SGW.exe at all — it's a `.lua` script asset, not compiled into the
   binary). If that table (or an eager, non-display-time consumer of it this session didn't find)
   is array-indexed by `SpeakerID` rather than map-keyed, an out-of-range read for a never-before-
   seen id like `754` is a plausible crash shape distinct from anything this session's DialogID-
   width tracing could have caught. **Important caveat**: the timeline argument in finding §3 that
   rules out crashing *during the fragment parse* applies here too — if the speakers table were
   consulted eagerly at parse/persist time, the client would have died at login, not four seconds
   into `Castle_CellBlock`. So this lead only holds together if something *else*, not yet located,
   consults the speakers table (or does something else keyed by `SpeakerID`) later, and that
   something is untraced. Flagged as a lead, not a finding.

## Recommendation

**The >65535 element-key-width theory is refuted for the traced pipeline (see "Verdict").**
`60100`–`60104` (already merged, #938) sidesteps that specific, now-refuted mechanism, so nothing
in this finding argues against it — but nothing in it identifies the actual mechanism the renumber
fixes either. The most direct way to know whether `60100`-`60104` actually resolves the crash is
**empirical**: check whether the crash recurred in colo telemetry for any client that logs in and
loads a map after #938 shipped. If it recurred, the next investigation should start from the
`SpeakerID=754` lead above (What is not confirmed, item 4) rather than from element-key width
again — the RE evidence no longer supports id width as a live hypothesis.

**The renumber still does not, by itself, remediate any client that already received the
`100100`/`100101` push** — see finding §4 above: the stale entries persist in that client's local
`Cache.en-US/CookedDataDialogs.pak`, and the version-mismatch handshake only ever targets the
*current* override table, never a removed id. An already-affected tester needs either a manual
local-cache clear or a one-time `invalidate_all`/tombstone (finding §5) before this is fully closed
for them, independent of whatever the true root cause turns out to be.

## Next steps for a follow-up RE session

1. ~~Get a Ghidra session where the dynamically-registered analysis tools are actually
   reachable~~ — **done, 2026-09-27: use headless Ghidra instead of the GUI+MCP bridge.**
   `analyzeHeadless.bat "<project dir>" <project name> -process SGW.exe -noanalysis -readOnly
   -scriptPath "<dir>" -postScript <Script>.java <args...>` runs a `GhidraScript` non-interactively
   against the existing analyzed project with no GUI and no MCP round-trip, and this session
   confirmed it can decompile (`DecompInterface`), enumerate xrefs (`getReferencesTo`), and search
   defined strings, batched many-addresses-per-run to amortize the ~1-2 minute JVM/project-load
   cost. **Only one run at a time** — the project lock is exclusive; a stale `SGW.lock`/`SGW.lock~`
   with no `javaw`/`analyzeHeadless` process running is safe to delete. Keep `-readOnly` so nothing
   in the project is modified. This is now the recommended path for any RE session that needs live
   decompile without a GUI Ghidra already open.
2. Find the actual `speakers` CookedData name-table implementation (What is not confirmed, item 4)
   — search for "speakers" or "Speaker" as a defined string near category 5/10's code, or trace
   what the client does with a `SpeakerID` after the record-level parse stores it (nothing in
   `FUN_015e4d10` itself resolves a name — that happens somewhere downstream, not yet located).
3. Trace `GameAppearanceManager`'s `Event_Level_PostLoad` handler body (RTTI accessor
   `0x00e9a480`) and the two `Event_World_Loaded` subscribers (`GameProxyPlayer` RTTI
   `0x00df7b80`, `Minimap` RTTI `0x00e2af30`) — the three map-load-lifecycle handler bodies this
   session did not reach, to close out item 1 above completely.
4. ~~Decompile the caller of `Detail::ZipStorageBase::WriteStreamToFile` to confirm whether the ZIP
   entry name, or any in-memory index keyed directly by `DialogID` value, is where a large id
   causes trouble.~~ **DONE, 2026-09-27**: the callers are `FUN_0043a9d0` (shared persist path) and
   `FUN_0043b550` (shared per-key delete), both of which build the entry name as `L"_" +
   operator<<(long key)` — a plain decimal-digit stream insertion, no indexing, no narrowing. See
   "Verdict" above. This line of inquiry is closed; it isn't where a large id causes trouble.
5. If the colo-telemetry check above shows the crash recurred even after `60100`-`60104`, follow
   the `SpeakerID=754` lead (item 2 above) before returning to element-key width — the RE evidence
   built this session doesn't support width as the mechanism, and re-treading it would repeat this
   session's work for a hypothesis that's now refuted for every reachable code path.

## Cross-references

- `docs/reverse-engineering/findings/cooked-data-pipeline.md` — category table, `LibCategory`/
  `ServerSource` struct layout, `ZipStorageBase` archive path this finding builds on
- `docs/reverse-engineering/findings/dialog-portrait-lookup.md` — `SpeakerID`/`ScreenID` field
  offsets and the `FUN_015e4d10` address this finding cites
- `docs/reverse-engineering/findings/dialog-controller-wire-flow.md` — the display-time path
  (`onDialogDisplay`, `IsImmediate`) that this finding's timeline reasoning depends on *not* being
  where the crash happens
- `docs/reverse-engineering/findings/world-entry-pipeline.md` — `onClientMapLoad`
  (`GameProxyPlayer::HandleOnClientMapLoad`), `Event_Level_PostLoad`, and `Event_World_Loaded`
  addresses this session's live decompile traced and cross-checked
- `crates/resources/src/base/dialog_overrides/mod.rs` — the Rust generator for the two dialogs in
  question
- `crates/base-session/src/base/cooked_data.rs` — the server-side push path
