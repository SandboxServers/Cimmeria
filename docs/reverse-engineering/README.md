---
type: reference
audience: Contributors orienting themselves in the docs/reverse-engineering/ tree
last_updated: 2026-05-24
companion_docs:
  - ../guides/re-toolchain-setup.md
  - ../guides/reverse-engineering-with-claude.md
  - evidence-standards.md
---

# Reverse-Engineering — Cimmeria's Ghidra Work

This directory holds everything the Cimmeria project has recovered from the SGW client binary: the plan, the progress tracker, the annotation scripts that produced ~101,909 named functions, and the per-system findings docs. It's the section-1 evidence pool that every Cimmeria Bible chapter (under [`docs/spec/`](../spec/)) ultimately cites.

**New to RE on this project?** Start here:

1. [`../guides/re-toolchain-setup.md`](../guides/re-toolchain-setup.md) — Install Ghidra, x64dbg, the MCP bridges, and wire `.mcp.json`. End-to-end. ~30 minutes if the bootstrap path works.
2. [`../guides/reverse-engineering-with-claude.md`](../guides/reverse-engineering-with-claude.md) — How to use the `game-archaeology-specialist` agent, what to verify yourself, what to hand off to `documentation-writer`.
3. [`evidence-standards.md`](evidence-standards.md) — Confidence tiers (HIGH / MEDIUM / LOW), citation grammar, the rules every finding doc must follow.
4. [`../guides/reading-decompiled-code.md`](../guides/reading-decompiled-code.md) — How to interpret Ghidra decompiler output without being misled.
5. [`../guides/sgw-live-debugging.md`](../guides/sgw-live-debugging.md) — Manual dynamic-analysis techniques in x32dbg. The pybag warning lives here.

Then read [`PLAN.md`](PLAN.md) for the campaign-level methodology and [`STATUS.md`](STATUS.md) for what's been done.

## Directory map

| Path | Purpose | Status |
|---|---|---|
| [`toolchain/`](toolchain/) | Install references for the RE toolchain (Ghidra MCP, x64dbg MCP) | Active |
| [`PLAN.md`](PLAN.md) | Campaign-level RE plan: phases, targets, methodology | Phases 1–5 complete |
| [`STATUS.md`](STATUS.md) | Progress tracker — what's been recovered, by phase | Phase 6 (V5 Function Documentation Campaign) in progress |
| [`address-map.md`](address-map.md) | Key addresses: vtables, global objects, important functions in `SGW.exe` | Active reference |
| [`function-naming-progress.md`](function-naming-progress.md) | Naming-script results, conventions, coverage metrics | Phase 1 reference |
| [`editor-source-mapping.md`](editor-source-mapping.md) | ServerEd ↔ binary correlation notes | Reference |
| [`annotation-scripts/`](annotation-scripts/) | 10 Jython scripts that produced ~5,878 high-confidence + ~96,031 medium-confidence function names | Reference, all run |
| [`findings/`](findings/) | Per-system wire-format and behavior findings (Phases 2–6) | V5 campaign in progress |
| [`binaries/`](binaries/) | Per-binary RE notes (Launcher.exe, AtreaLoader.exe, etc.) | Reference |
| [`decompiled/`](decompiled/) | Raw decompile dumps + index | Internal — see [`decompiled/00_INDEX.md`](decompiled/00_INDEX.md) |
| [`v5-campaign/`](v5-campaign/) | V5 Function Documentation Campaign artifacts (status, worker briefs, checkpoints) | In progress — internal workflow files |

## Toolchain subdirectory

The [`toolchain/`](toolchain/) directory holds installation references for everything the RE workflow depends on. Today it contains:

| Document | Purpose |
|---|---|
| [`toolchain/install-ghidra-mcp.md`](toolchain/install-ghidra-mcp.md) | GhidraMCP plugin install — manual + bootstrap-driven paths, paths-on-disk reference, the Windows port-fallback gotcha |

The end-to-end "from `git clone` to MCPs reachable" walkthrough lives one level up at [`../guides/re-toolchain-setup.md`](../guides/re-toolchain-setup.md). The `toolchain/` files are the components that walkthrough installs.

## Findings — the V5 evidence pool

[`findings/`](findings/) holds the per-system findings docs that earlier phases produced. Its <!-- gen:re-findings-count -->85<!-- /gen:re-findings-count --> docs span the wire-format pool (combat, inventory, missions, organizations, crafting, gate travel, minigames, chat, mail, black market, contact list, group, trade, duel, pet, entity types, entity creation, position/movement, space/viewport, system protocol), the Phase 6/7 behavior docs (CME EventSignal pipeline, state-flag broadcast, respawn lifecycle), and the V5 deep dives (ability resolution, mission/inventory/crafting/NPC-AI state machines, mercury internals, cooked-data pipeline, dialog-portrait lookup, character creation, combat damage, effects, cover, loot, faction alignment, stat scaling, spawn mechanics, struct field layouts, weapon/ammo pipeline, the pet client contract (`Unit.PetN` slot binding), and more). Most are rated HIGH confidence at time of writing — but per [`evidence-standards.md`](evidence-standards.md) and the [`reverse-engineering-with-claude.md`](../guides/reverse-engineering-with-claude.md) "verify load-bearing claims" rule, pre-V5 docs are hypotheses; re-verify before pinning a claim into a bible chapter or production Rust.

Beyond the V5 evidence pool, [`findings/`](findings/) also carries issue-scoped
de-risking findings — e.g. [`auth-and-crypto-modernization-targets.md`](findings/auth-and-crypto-modernization-targets.md)
(issue #434), which maps the client's login transport, SHA-1 password site,
anti-debug posture, and Mercury crypto to exact `SGW.exe` addresses for the
encryption-modernization patch work.

The `castle.nav` spike (issue #46) adds three more: [`bsp-model-polys-serialize.md`](findings/bsp-model-polys-serialize.md) (the `UModel`/`UPolys` wire layout), [`castle-bsp-geometry-location.md`](findings/castle-bsp-geometry-location.md) (which Castle packages actually hold collidable BSP), and [`terrain-serialize-real-data-validation.md`](findings/terrain-serialize-real-data-validation.md) (the `ATerrain::Serialize` recipe re-validated on a second map).

The dialog UI redesign adds [`dialog-controller-wire-flow.md`](findings/dialog-controller-wire-flow.md) — the native `DialogController` display path, covering what `IsImmediate` actually decides, the two active-dialog slots and their eviction behaviour, and how a button click resolves to the cooked `ButtonID` on the wire. It corrects one handler label in [`dialog-portrait-lookup.md`](findings/dialog-portrait-lookup.md) and disputes that document's speaker-name track. The author-facing rules it produces live in [`docs/content/dialog-ui-client-contract.md`](../content/dialog-ui-client-contract.md).

The NPC AI restoration campaign's NA20 packet adds [`cover-world-placement.md`](findings/cover-world-placement.md) — decoding `SGWSpecCoverNode`/`SGWCoverNodeComponent` directly out of the Castle/Castle_CellBlock `.umap` chunks shows 4,024 cover nodes that are already in world space with no owner-transform composition needed, correcting the prefab-pak-transform hypothesis in [`cover-system.md`](findings/cover-system.md) for those two maps and substantially shrinking the scope of the NA21 extractor packet.

Its NA31 packet adds [`being-eye-heights.md`](findings/being-eye-heights.md). The script pawn gives every being one stock UE3 cylinder, so eye heights come from each body set's reference skeletal-mesh bounds instead: 1.81 m for a human male, 2.12 m for a Jaffa male and 0.15 m for a rat. The finding also shows that `ErrorStrings.pak` code 39 (`CONDITION_FEEDBACK_LOS`) is the line-of-sight feedback the client has text for.

The ability-trees campaign's AT-E1 packet adds [`ability-trainer-ui.md`](findings/ability-trainer-ui.md) — the Trainer/Ability window's native Lua bindings decompiled (`getTrainableList`/`getTrainableInfo`/`getTrainingTreeCount`/`buyTrainable`/`respecAbilities`), confirming the tree/trainer join is hidden-not-greyed at the byte level and that the client has no client-side level/XP table. The `onErrorCode` client-side rendering question is left explicitly UNRESOLVED — no Lua consumer exists anywhere in the client, but whether a native listener renders it was not traced.

[`ability-animation-links.md`](findings/ability-animation-links.md) explains why most abilities hit with no animation. The client plays whatever sequence id the server sends and has no ability-keyed lookup, so the link from an ability to its event set was CME server data that Project Giza's seed only partly recovered. The 35 recovered links follow a weapon-family rule, which the seed extends to 220 more abilities in three labelled tiers; the finding lists the few left open.

The 2026-09-26 colo-log fix pass adds [`client-generic-region-hit-test.md`](findings/client-generic-region-hit-test.md). The client decides on its own when to send `triggerClientHintedGenericRegion`, and its hit test differs from the server gate on the vertical axis: the ceiling is exact and the floor reaches 100 units down. It explains two colo warnings: Mess Hall entries refused from the corridor below, and `region_dwell_no_hint` false positives for players on the floor above.

The crafting campaign's CR-17 packet adds [`trade-result-client-handling.md`](findings/trade-result-client-handling.md). The trade window closes only on `onTradeResults` `Completed` or `Cancelled`; the space and cash codes (3-6) leave it open and locked, so the server now answers every trade refusal with `Cancelled` and a feedback line. The native handler is not live-traced yet.

The Bank and Vault campaign's BV-E1 packet adds [`bank-vault-client.md`](findings/bank-vault-client.md). The world-entry `onBagInfo` declaration for container 17 is enough on its own — `onVaultOpen` triggers no fresh bag request — and the Vault subscribes to `Events.InventoryUpdateContainerSize`, so a later `onBagInfo` that changes the declared size is *inferred* to resize an open window (the native emit site is not yet traced). Most load-bearing: a Banker can already offer a single-button "Expand vault" choice through the existing dialog seed tables and the existing `dialogButtonChoice`/content-engine path, no new wire method or client patch required, so long as the offered dialog carries exactly one clickable button. `onVaultOpen`'s `Position` argument has no client-side consumer found in Lua or in the traced native path, so proximity enforcement must stay entirely server-side.

Client-render findings sit here too: [`render-thread-options.md`](findings/render-thread-options.md) maps `RenderThreadOptionManager` and shows which client shadow settings reach the renderer (the `max`/`minShadowResolution` system options do not; the shadow depth buffer is a fixed 1024).

A 2026-09-27 colo incident adds [`cooked-dialog-override-crash.md`](findings/cooked-dialog-override-crash.md). Pushing two Cimmeria-authored `CookedDataDialogs.pak` overrides with ids `100100`/`100101` crashed the client on the next map load. A first pass ruled out the authored `<Buttons>` markup as the cause via SigNoz timeline reasoning, and flagged the >65535 element-key value as the leading, unconfirmed suspect. A same-day follow-up got live decompile access via **headless Ghidra** (`analyzeHeadless.bat` with a `GhidraScript`, no GUI, no MCP bridge — a workaround for the tool-discovery gap the first pass hit) and traced the actual category-5 `onVersionInfo`/`onCookedDataError`/element-commit functions plus the `onClientMapLoad` handler itself: **the >65535 hypothesis is refuted** for every code path reachable from a `resourceFragment` push or a map load — the element key is a `long` end to end, the ZIP entry name is built by plain decimal-digit streaming with no narrowing, and neither `HandleOnClientMapLoad` nor the `Event_Level_PostLoad` callback touches Dialog data. The finding also answers a question from the renumber hotfix: the client persists pushed cooked-data fragments to a writable local PAK, and the server's version-mismatch handshake only ever targets the *current* override table — so renumbering the ids alone does not remediate a client that already received the bad push. The tester crashed again on the renumbered build (`60100`-`60104`), confirming id value was never the mechanism; a parallel `world-entry-bisect` worker then confirmed `CREATE_BASE_PLAYER`/`onClientMapLoad` bytes are identical across builds, so dialogs are back as the confirmed carrier — the open question is which *content* field differs from every override that's worked for days (nonzero `speaker_id` 754/843, a two-screen dialog, `ScreenID`s in `200000`-`200005`, or button type 4). A same-day Round 2 ruled out three more mechanisms (the category-5 constructor doesn't enumerate persisted elements; nothing subscribes to "a Dialog element became ready" outside `ServerSource` itself; no Castle_CellBlock content references these ids) and flagged one lead — `Event_Entity_ProxyPlayerBaseCreated` fires a second `versionInfoRequest` per category around `CREATE_BASE_PLAYER` time — that SigNoz then killed: sessions show exactly one version cycle, and a control client's harmless repeated re-push over two days proves double-pushing isn't fatal by itself. Round 3 found the generic CME `DataType`-registry dispatch behind the record parser is a dead end, and surfaced a structural tension worth stating plainly: Dialog-record XML parsing is lazy (only at actual display time), so the debug-hub dialogs' content should never even be parsed during a session that never opens them — in genuine tension with "which content field crashes it." Round 4 then traced every remaining map-load-lifecycle CME handler end to end (`GameAppearanceManager`'s full six-event subscription set, `Minimap`'s `Event_World_Loaded` handler, and — a new finding — `GameProxyPlayer`'s `Event_World_Loaded` subscription, which turns out to be wired lazily from *inside* `onClientMapLoad` itself, on the first map load) and found zero Dialog-category touches anywhere in that graph. **Final status: Inconclusive** — no dialog consumer exists on the traced map-load path, and counter-evidence surfaced after Round 4 weakens the hub-dialog correlation itself: the same tester hung the same way entering three other maps on the *old* build, before the debug-hub dialogs existed. Static RE work stopped here; the next step is empirical (delete the local cache and retry after the `#943` quarantine release, cross-check with a clean client), with live x64dbg tracing as a fallback only if that stays ambiguous.

See [`findings/README.md`](findings/README.md) for the full per-doc index.

## Bible relationship

These findings are *upstream* of the Cimmeria Bible. The flow is:

```text
RE session   →  game-archaeology-specialist  →  findings/<system>.md
                                              ↓
                                      documentation-writer
                                              ↓
                                      docs/spec/<chapter>.md (bible)
```

When a bible chapter contradicts a finding doc, the bible wins by default — but the finding is the *path to changing canon*. See [`docs/spec/how-to-write.md`](../spec/how-to-write.md) for the promotion gate.

## Annotation scripts — what they did

The 10 Jython scripts under [`annotation-scripts/`](annotation-scripts/) ran during Phase 1 and produced the named-function baseline that everything since has relied on. Cumulative result: **101,909 / 168,239 non-thunk functions named (60.6%)**.

Counts mirror [`STATUS.md`](STATUS.md), which is authoritative — update both together if you re-run a script.

| Script | Functions renamed | Confidence | Status |
|---|---:|---|---|
| `01_rtti_annotator.py` | 4,364 (+ 8,961 vtable labels) | HIGH | DONE |
| `02_ue3_exec_annotator.py` | 1,006 | HIGH | DONE |
| `03_bigworld_source_annotator.py` | 23 | HIGH | DONE |
| `04_event_signal_annotator.py` | 419 | HIGH | DONE |
| `05_mercury_annotator.py` | 38 (+ 79 vtable xrefs) | HIGH | DONE |
| `06_cme_framework_annotator.py` | 28 | HIGH | DONE |
| `07_vtable_annotator.py` | ~9,600 | MEDIUM | DONE (partial, cancelled) |
| `08_lua_binding_annotator.py` | 0 | — | DONE (Lua vestigial in this binary) |
| `09_string_discovery.py` | 1,364 | MEDIUM | DONE |
| `10_xref_propagation.py` | 3,333 | LOW (call-graph inference) | DONE |

Re-running them on a fresh Ghidra project takes ~1 hour. If you're picking up a system that hasn't been touched in a while, also re-run any script whose strings table may have grown — [`annotation-script-shift-bugs.md`](findings/annotation-script-shift-bugs.md) documents past shift-bug incidents to watch for.

## Cross-references

- [`../guides/re-toolchain-setup.md`](../guides/re-toolchain-setup.md) — install everything
- [`../guides/reverse-engineering-with-claude.md`](../guides/reverse-engineering-with-claude.md) — how to use the toolchain
- [`evidence-standards.md`](evidence-standards.md) — confidence rules
- [`../guides/reading-decompiled-code.md`](../guides/reading-decompiled-code.md) — decompile interpretation
- [`../guides/sgw-live-debugging.md`](../guides/sgw-live-debugging.md) — manual x32dbg techniques
- [`../analysis/event-net-mapping.md`](../analysis/event-net-mapping.md) — 420 Event_NetIn/NetOut → .def methods → Ghidra addresses
- [`../analysis/bigworld-reference-index.md`](../analysis/bigworld-reference-index.md) — BigWorld 2.0.1 → SGW.exe symbol map
- [`../spec/`](../spec/) — the Cimmeria Bible (chapters cite findings under `findings/`)
- [`.mcp.json.example`](../../.mcp.json.example) — MCP config template
