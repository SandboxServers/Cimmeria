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

[`findings/`](findings/) holds the per-system findings docs that earlier phases produced. As of 2026-07-25, 63 docs span the wire-format pool (combat, inventory, missions, organizations, crafting, gate travel, minigames, chat, mail, black market, contact list, group, trade, duel, pet, entity types, entity creation, position/movement, space/viewport, system protocol), the Phase 6/7 behavior docs (CME EventSignal pipeline, state-flag broadcast, respawn lifecycle), and the V5 deep dives (ability resolution, mission/inventory/crafting/NPC-AI state machines, mercury internals, cooked-data pipeline, dialog-portrait lookup, character creation, combat damage, effects, cover, loot, faction alignment, stat scaling, spawn mechanics, struct field layouts, weapon/ammo pipeline, and more). Most are rated HIGH confidence at time of writing — but per [`evidence-standards.md`](evidence-standards.md) and the [`reverse-engineering-with-claude.md`](../guides/reverse-engineering-with-claude.md) "verify load-bearing claims" rule, pre-V5 docs are hypotheses; re-verify before pinning a claim into a bible chapter or production Rust.

Beyond the V5 evidence pool, [`findings/`](findings/) also carries issue-scoped
de-risking findings — e.g. [`auth-and-crypto-modernization-targets.md`](findings/auth-and-crypto-modernization-targets.md)
(issue #434), which maps the client's login transport, SHA-1 password site,
anti-debug posture, and Mercury crypto to exact `SGW.exe` addresses for the
encryption-modernization patch work.

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
