# Cimmeria — GitHub Copilot Instructions

Cimmeria is a server emulator for the cancelled MMO **Stargate Worlds**. Active development is in **Rust** under `crates/`.

## Safety rules (review blockers)

- Active schemas live in `db/database.sql`, `db/sgw/`, `db/resources/`.

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

**Handlers must take `&Arc<dyn Transport>`, never `&Arc<UdpSocket>`, outside the recv loop.** Outbound sends go through `cimmeria_mercury::transport::Transport`; only `connect_loop::run_connect_loop` keeps the concrete `UdpSocket` (it owns `recv_from`) and wraps it in a `UdpTransport` for dispatch. A new handler that reaches for `UdpSocket` directly is a review block — it defeats the byte-exact fan-out test seam. See [docs/architecture/transport-trait.md](../docs/architecture/transport-trait.md).

## Required tests on every PR

A PR that changes runtime behaviour must add or update a test. **Read [TESTING.md](../TESTING.md) before writing one** — it has the picker for the eight test types we use (unit / wire-format / live-DB / smoke / concurrency / chain-replay / legacy reference / fan-out byte) and the gotchas mined from review comments since PR #131. Reviewer non-negotiables:

- The test must fail when the fix is reverted (regression-guard shape, not happy-path).
- Tighten assertions: composite keys, exact final positions, `== 1` not `>= 1`, exact byte strings for serializers.
- Don't hard-code seed ids — re-fetch baselines or assert by relationship (`slot.cur_ammo_type == slot.default_ammo_type`).
- Sentinel ids fit in `i32`; cleanup deletes by exact sentinel, not by range.
- Live-DB tests use `require_db_or_skip!` and run serialised — `cargo nextest run --profile=ci-live-db` (the profile in `.config/nextest.toml` pins `threads-required = "num-test-threads"`), or `cargo test ... -- --test-threads=1` if you're not on nextest.
- Test names match the assertion. If the assertion changes, rename.
- No PR or issue numbers in source comments — provenance lives in the PR body.
- Update [docs/testing/inventory/<crate>.md](../docs/testing/inventory/) **only when a single PR adds or removes ≥5% of the workspace test count** (~68 tests against the current 1351 baseline). Smaller drifts get folded in by periodic sweep updates — don't block review on per-PR inventory churn for a handful of tests in a 1351-test repo.

If a single feature touches several layers (handler logic + serializer + SQL + cross-handler invariant), expect to add several test types — see TESTING.md "When one feature needs more than one test".

## Required docs on every PR

A PR that changes user-visible behaviour, public surface, file layout, build steps, or test policy must update the corresponding doc(s). For non-trivial doc work, prefer the **Documentation Writer** agent over freehand edits — it follows the Diátaxis framework (tutorials / how-to / reference / explanations) and keeps voice consistent with the rest of `docs/`. The mapping of "what changed → what to update" is in [CLAUDE.md](../CLAUDE.md) under "Required documentation for every PR". Index entries in `docs/readme.md` and per-section `README.md` files must stay in sync with the documents they list — adding or renaming a doc means updating the index in the same PR.

Run the markdown lint as the doc-side equivalent of `cargo clippy`: `tools/lint-md.sh` (or `.ps1` on Windows). Same ruleset CodeRabbit applies in PR review — local catches every cosmetic finding before the bot has to type it. Warn-only in CI for now; Phase 2 hardens to blocking. Config: [.markdownlint-cli2.yaml](../.markdownlint-cli2.yaml).

## Where to find more

Full conventions: `CLAUDE.md`. Testing: `TESTING.md`. Architecture: `docs/architecture/`. Content engine: `docs/architecture/data-driven-content-engine.md`. Migration roadmap: `docs/architecture/migration-roadmap.md`. Live-DB infra: `docs/architecture/integration-test-infra.md`.
