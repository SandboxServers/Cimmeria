---
applyTo: "crates/services/**/*.rs,crates/cell/**/*.rs,crates/cell-*/**/*.rs,crates/base/**/*.rs,crates/base-*/**/*.rs,crates/wire/**/*.rs,crates/content-engine/**/*.rs,crates/entity/**/*.rs,crates/game/**/*.rs,crates/server/**/*.rs"
---

# Rust services review rules

The cell-side and base-side server logic lives in the crates split out of `crates/services`, which is now a thin facade (see `docs/architecture/services-crate-split.md`). Each side has a separate message loop and they communicate via `CellToBaseMsg` / `BaseToCellMsg` channels.

## Cell vs base split

- **Cell** (the cell-track crates: `crates/cell/`, `crates/cell-world/`, `crates/cell-combat/`, `crates/cell-content/`, `crates/cell-interactions/`, `crates/cell-methods/`, `crates/cell-console/`, the feature plugins `crates/cell-pets/`, `crates/cell-duel/` and `crates/cell-org/` (#962), with `cell-catalog` and `cell-cover` below them) — entity state, content engine, AoI, NPC AI, abilities. One cell per space.
- **Base** (the base-track crates: `crates/base/`, `crates/base-world-entry/`, `crates/base-methods/`, `crates/base-session/`) — client connection lifecycle, world entry, client-method dispatch, persistence, witness broadcasts to the connected client.
- They communicate by enum messages (`crates/wire/src/cell/messages/`). Don't reach across the boundary directly — add a message variant if you need a new interaction.

## Content engine actions

Every action type in `Action` (see `crates/content-engine/src/actions.rs`) needs an executor arm in `crates/cell-content/src/cell/content/executor/mod.rs`. Stubs that only log are a known footgun — they make a chain *look* like it's running while doing nothing. If you spot a stub arm during review, ask whether the calling chain actually expects the side effect.

`Action::RemoveItem`, `Action::IncrementCounter` and `Action::ResetCounter`, once stubs, are implemented (`executor/inventory.rs`, `executor/counter.rs`). New actions should either implement fully or be flagged with a `tracing::warn!` so silent no-ops are visible in logs.

## Wire format

When sending a `CellToBaseMsg::EntityMethodCall` or building a base→client packet:

- Confirm `method_index` against `docs/protocol/client-method-dispatch-table.md`. Indices live in `crates/wire/src/mercury/mod.rs` — prefer a named constant over a literal.
- Confirm byte layout against `entities/defs/*.def`. Endianness is little-endian; vectors are 3×f32; strings use `write_wstring` (length-prefixed UTF-16).
- Engine-level base messages (`BASEMSG_*` in `mercury/mod.rs`) are handled by the BigWorld client *before* user code runs — use them for authoritative state changes (`FORCED_POSITION` for teleport, etc.). Method-index-dispatched messages (0xBD prefix) hit user code and may be ignored under certain client states (e.g., `BSF_MovementLock`).

## Builds

Builds run natively on Windows on the toolchain `rust-toolchain.toml` pins; the build rules are in `CLAUDE.md` ("Build rules"), and the reasons in `docs/architecture/build-system.md`.

1. Iterate with `cargo check -p <crate>` on the crate you changed. `cimmeria-services` is a small facade over the split crates, so `-p cimmeria-services` doesn't cover them.
2. Agent and worker `cargo` calls go through the build lane, `tools/build-lane/lane.sh`, which limits how many builds run on the machine at once.
3. Workspace builds for final validation only, with the seven `--exclude` flags CI uses (`.github/workflows/test.yml`), under `lane.sh --exclusive`.

## File caps

Soft cap **500 lines**, hard cap **700**. Split on natural seams (handler groups, lifecycle phases, message-type families). Flat names for 2–3 siblings; promote to a `mod.rs` directory only at 4+. Module style is `foo/mod.rs` (not modern `foo.rs` + `foo/`).

Avoid generic names: no `helpers.rs`, `utils.rs`, `misc.rs`, `extra.rs`. Name by behaviour: `cooldowns.rs`, `damage_resolution.rs`, `witness_list.rs`.

## Error handling

Validate at system boundaries only — user input, external APIs, DB roundtrips. Don't `match` on internally-controlled enums for variants the type system already excludes. Don't add `?` to operations that can't fail in this codebase. If you find yourself writing `unwrap_or_default()` for an `Option` that's never `None` in practice, restructure so it doesn't need to be an `Option`.

## Comments

Default to none. Only when the **why** is non-obvious: hidden constraint, subtle invariant, bug workaround, surprising behaviour. Do not narrate what code does, do not reference the current PR or task, do not leave `// TODO: removed for X` markers — delete the code instead.

## Reference

- `crates/cell-content/src/cell/ring_transport/dispatch.rs` is a good example of how to dispatch FSM `Effect`s into wire `CellToBaseMsg`s.
- `crates/base-world-entry/src/base/world_entry/cell_dispatch/mod.rs` shows the base-side handler pattern for a `CellToBaseMsg` variant.
- Reference Python in `python/cell/` and `python/common/` is the behaviour spec — read it for any new feature port.
