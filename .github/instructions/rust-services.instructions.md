---
applyTo: "crates/services/**/*.rs"
---

# Rust services review rules

`crates/services` houses the cell-side and base-side server logic. Each side has a separate message loop and they communicate via `CellToBaseMsg` / `BaseToCellMsg` channels.

## Cell vs base split

- **Cell** (`crates/services/src/cell/`) — entity state, content engine, AoI, NPC AI, abilities. One cell per space.
- **Base** (`crates/services/src/base/`) — client connection lifecycle, world entry, client-method dispatch, persistence, witness broadcasts to the connected client.
- They communicate by enum messages (`crates/services/src/cell/messages/`). Don't reach across the boundary directly — add a message variant if you need a new interaction.

## Content engine actions

Every action type in `Action` (see `crates/content-engine/src/actions.rs`) needs an executor arm in `crates/services/src/cell/content/executor.rs`. Stubs that only log are a known footgun — they make a chain *look* like it's running while doing nothing. If you spot a stub arm during review, ask whether the calling chain actually expects the side effect.

Existing stubs to watch for: `Action::RemoveItem` (logs only — see content-chain rules), `Action::IncrementCounter`, `Action::ResetCounter`. Newer additions should either implement fully or be flagged with a `tracing::warn!` so silent no-ops are visible in logs.

## Wire format

When sending a `CellToBaseMsg::EntityMethodCall` or building a base→client packet:

- Confirm `method_index` against `docs/protocol/client-method-dispatch-table.md`. Indices live in `crates/services/src/mercury/method_idx.rs` — prefer a named constant over a literal.
- Confirm byte layout against `entities/defs/*.def`. Endianness is little-endian; vectors are 3×f32; strings use `write_wstring` (length-prefixed UTF-16).
- Engine-level base messages (`BASEMSG_*` in `mercury/mod.rs`) are handled by the BigWorld client *before* user code runs — use them for authoritative state changes (`FORCED_POSITION` for teleport, etc.). Method-index-dispatched messages (0xBD prefix) hit user code and may be ignored under certain client states (e.g., `BSF_MovementLock`).

## Build memory

A full link of `cimmeria-services` can use ~47 GB RAM. The workspace's `[profile.dev.package."*"]` strips dependency debug info to bring this down to ~8 GB, but you still need to:

1. Iterate with `cargo check -p cimmeria-services` — fast (~1.5s), low memory.
2. Never run multiple `cargo`/`rustc` processes concurrently — `pkill -f rustc` first.
3. Workspace builds for final validation only, with `--exclude cimmeria-app --exclude cimmeria-content-editor --exclude cimmeria-scene-editor` to skip the Tauri linker.
4. `CARGO_BUILD_JOBS=2` is set in `.bashrc` to cap parallel codegen.

## File caps

Soft cap **500 lines**, hard cap **700**. Split on natural seams (handler groups, lifecycle phases, message-type families). Flat names for 2–3 siblings; promote to a `mod.rs` directory only at 4+. Module style is `foo/mod.rs` (not modern `foo.rs` + `foo/`).

Avoid generic names: no `helpers.rs`, `utils.rs`, `misc.rs`, `extra.rs`. Name by behaviour: `cooldowns.rs`, `damage_resolution.rs`, `witness_list.rs`.

## Error handling

Validate at system boundaries only — user input, external APIs, DB roundtrips. Don't `match` on internally-controlled enums for variants the type system already excludes. Don't add `?` to operations that can't fail in this codebase. If you find yourself writing `unwrap_or_default()` for an `Option` that's never `None` in practice, restructure so it doesn't need to be an `Option`.

## Comments

Default to none. Only when the **why** is non-obvious: hidden constraint, subtle invariant, bug workaround, surprising behaviour. Do not narrate what code does, do not reference the current PR or task, do not leave `// TODO: removed for X` markers — delete the code instead.

## Reference

- `crates/services/src/cell/ring_transport/runtime.rs` is a good example of how to dispatch FSM `Effect`s into wire `CellToBaseMsg`s.
- `crates/services/src/base/world_entry/cell_dispatch.rs` shows the base-side handler pattern for a `CellToBaseMsg` variant.
- Reference Python in `python/cell/` and `python/common/` is the behaviour spec — read it for any new feature port.
