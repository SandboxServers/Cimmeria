# Cimmeria — GitHub Copilot Instructions

Cimmeria is a server emulator for the cancelled MMO **Stargate Worlds**. Active development is in **Rust** under `crates/`. The C++ in `src/` and Python in `python/` are reference implementations of the original BigWorld server — read them for behaviour, implement in Rust.

## Safety rules (review blockers)

- **Never expose this server to the public internet** — legacy code uses OpenSSL 0.9.8i with active CVEs.
- `db/deprecated/` is reference only. Active schemas live in `db/database.sql`, `db/sgw/`, `db/resources/`.
- `config/*.config` is test creds; real envs use gitignored `*.local` overrides.

## Content-chain review checklist (`db/resources/Content/Seed/*.sql`)

Recurring bugs — flag in review:

1. **Every `interact_tag` trigger needs a matching `set_interaction_type` action somewhere in its mission.** Without the bit, the entity has `interaction_type=0`, the client treats it as scenery, and right-clicks never reach the server. Common masks: `256` (Livewire-clickable), `32` (ring transporter), `8388608` (A-story mission available). Cookbook: `docs/content/interaction-flags.md`.
2. **Every `op:"|"` set must have a paired `op:"~"` clear on completion**, plus a `player_loaded`-triggered restore chain so a relog mid-mission doesn't break interactivity.
3. **Don't add `remove_item` next to a `UseInventoryItem`-driven chain.** The base service already consumes via `UseInventoryItem → ItemUsed`; a redundant `remove_item` double-consumes from any stack >1.
4. **Auto-generated chains in `space_*_chains.sql` (5xxx range) often have converter bugs:** `accept_mission` emitted where `complete_mission` was meant, duplicate actions within one chain, shadow conditions. When a PR regenerates these, diff against the previous version — don't trust the converter.
5. **`sort_order` discipline.** Adding actions to an existing chain → increment past the highest existing value. Don't reuse.

## Rust patterns

- **File caps**: 500 lines soft, 700 hard. Split on natural seams (handler groups, lifecycle phases, message families) — not arbitrarily on line count. Flat names for 2–3 siblings; promote to a directory only at 4+. Use `foo/mod.rs` style. Avoid `helpers.rs`/`utils.rs`/`misc.rs` — name by behaviour.
- **No defensive code for impossible scenarios.** Trust internal code and framework guarantees; validate only at system boundaries (user input, external APIs, DB roundtrips).
- **No scope creep.** Don't add features, refactors, abstractions, feature flags, or backwards-compat shims beyond the task. Delete unused code outright — no commented-out blocks, no `// removed` markers.
- **Build memory**: iterate with `cargo check -p <crate>`. A full link uses ~47 GB; never run multiple `cargo`/`rustc` processes concurrently. Workspace builds must `--exclude cimmeria-app --exclude cimmeria-content-editor --exclude cimmeria-scene-editor` to avoid the Tauri linker.

## Comments

Default to **none**. Add a comment only when the **why** is non-obvious: a hidden constraint, a subtle invariant, a workaround for a specific bug, surprising behaviour. Don't restate what identifiers already convey. Don't reference the current PR or task ("added for X flow", "fixes #123") — that rots in source; put it in the PR description.

## Wire format & protocol

When adding a client method call, confirm the index against `docs/protocol/client-method-dispatch-table.md` and byte layout against `entities/defs/*.def`. Notable trap: `onPlayerTeleport` (method 116) is a streaming-load hint, not an authoritative move — use `BASEMSG_FORCED_POSITION` (`build_forced_position` in `mercury/aoi.rs`) for actual avatar snaps.

## Where to find more

Full conventions: `CLAUDE.md`. Architecture: `docs/architecture/`. Content engine: `docs/architecture/data-driven-content-engine.md`. Migration roadmap: `docs/architecture/migration-roadmap.md`.
