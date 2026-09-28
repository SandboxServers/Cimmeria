# Cimmeria — GitHub Copilot Instructions

Cimmeria is a server emulator for the cancelled MMO **Stargate Worlds**. Active development is in **Rust** under `crates/`.

## Safety rules (review blockers)

- Active schemas live in `db/database.sql`, `db/sgw/`, `db/resources/`.
- **Seeds are the source of truth.** Seeded-data changes edit `db/resources/<area>/Seed/` directly. Flag any new `db/scripts/*.sql` migration that a maintainer did not ask for.
- **A change to a wire constant, method index, or message layout must cite evidence** (a `docs/protocol/` section or a Ghidra address), and must not contradict `docs/protocol/` without correcting it in the same PR. Issue text and draft chapters under `docs/drafts/spec/` are claims, not evidence.
- **Wire entity typeIDs are the client's clientIndex** (`<ServerOnly/>` entities skipped): `SGWPlayer = 0x02`, `SGWGmPlayer = 0x03`, `Account = 0x07`. Flag any PR that changes `Account` to `0x08`; see `docs/protocol/client-verified-wire-formats.md` "Entity Class IDs".
- **A test that compares a constant with the same literal is not a regression guard.** Ask what independent source the expected value comes from.
- **New opcodes, wire-crypto changes, or anything needing a client patch** need a maintainer decision recorded in the PR. Server-authoritative changes that reuse existing messages are preferred.
- **Memory files under `.claude/agent-memory/` are public.** Flag any that contain an IP address, a private hostname, a credential or token, a player or tester account name, or a local absolute path. Also flag a PR that overwrites or wholesale-rewrites a `MEMORY.md` index instead of adding lines to it.

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
- **Builds**: development builds run natively on Windows; the WSL cross-compile is retired, and CI builds on Linux runners. `rust-toolchain.toml` pins Rust 1.98.1 for local builds and CI alike ([.github/actions/rust-toolchain](actions/rust-toolchain/action.yml) reads it), so a version bump is its own PR: the pin change plus the lint fixes it forces. Iterate with `cargo check -p <crate>` on the crate you changed: `cimmeria-services` is only a facade over the split crates. Agent and worker `cargo` calls go through the build lane, `tools/build-lane/lane.sh` (`--exclusive` for workspace-wide runs). Workspace builds must `--exclude cimmeria-app --exclude cimmeria-content-editor --exclude cimmeria-scene-editor --exclude sgw-launcher --exclude cimmeria-client-telemetry --exclude cimmeria-client-patches --exclude cimmeria-lab` — the same seven exclusions CI uses ([.github/workflows/test.yml](workflows/test.yml)) — to avoid the Tauri/egui linker and the Windows-only cdylibs. Rationale: [docs/architecture/build-system.md](../docs/architecture/build-system.md). A dependency change must also regenerate the cargo-hakari workspace-hack (`cargo hakari generate && cargo hakari manage-deps --yes`); CI fails when it is stale. Worktrees are retired with `tools/build-lane/rm-worktree.sh` once their PR merges; flag instructions that delete a worktree with a recursive `rm`/`rmdir /s`, which can follow the `external/` junction.

## Comments

Default to **none**. Add a comment only when the **why** is non-obvious: a hidden constraint, a subtle invariant, a workaround for a specific bug, surprising behaviour. Don't restate what identifiers already convey. Don't reference the current PR or task ("added for X flow", "fixes #123") — that rots in source; put it in the PR description.

## Wire format & protocol

When adding a client method call, confirm the index against `docs/protocol/client-method-dispatch-table.md` and byte layout against `entities/defs/*.def`. Notable trap: `onPlayerTeleport` (method 116) is a streaming-load hint, not an authoritative move — use `BASEMSG_FORCED_POSITION` (`build_forced_position` in `mercury/aoi/update.rs`) for actual avatar snaps.

**Handlers must take `&Arc<dyn Transport>`, never `&Arc<UdpSocket>`, outside the recv loop.** Outbound sends go through `cimmeria_mercury::transport::Transport`; only `connect_loop::run_connect_loop` accepts the wider `Arc<dyn BidirectionalTransport>` (production wraps `UdpTransport`; chaos tests wrap `LossyTransport`). A new handler that reaches for `UdpSocket` directly is a review block — it defeats the byte-exact fan-out test seam. See [docs/architecture/transport-trait.md](../docs/architecture/transport-trait.md).

**Mercury-protocol changes** (channel state-machine, retransmit, fragmentation, keepalive, ack, RTO) **require a paired-channel test in the loopback harness**, not just a unit test on the state machine in isolation. The harness lives in `crates/mercury/src/test_harness/` behind the `test-harness` feature; use `LoopbackSession::connected(None)` (or `Some(enc)` for encryption tests) and drive time via `peer.clock.advance(...)` instead of `tokio::time::sleep`. See [docs/architecture/mercury-loopback-harness.md](../docs/architecture/mercury-loopback-harness.md) and TESTING.md type 9 (Mercury session tests).

**Network-recovery shapes** (single-packet drop, burst loss, asymmetric ack loss, sustained probabilistic loss) **require a network-chaos scenario test under `crates/mercury/src/test_harness/tests/chaos/`** in addition to any unit / session tests. Probabilistic scenarios must set `rng_seed` for reproducibility. Pcap-replay regressions go in `replay_*.rs` under the same directory. See [docs/architecture/network-chaos-testing.md](../docs/architecture/network-chaos-testing.md) and TESTING.md type 10.

## Required tests on every PR

A PR that changes runtime behaviour must add or update a test. **Read [TESTING.md](../TESTING.md) before writing one** — it has the picker for the twelve test types we use (unit / wire-format / live-DB / smoke / concurrency / chain-replay / legacy reference / fan-out byte / Mercury session / network chaos / wire-level replay / negative-log) and the gotchas mined from review comments since PR #131. Reviewer non-negotiables:

- The test must fail when the fix is reverted (regression-guard shape, not happy-path).
- Tighten assertions: composite keys, exact final positions, `== 1` not `>= 1`, exact byte strings for serializers.
- Don't hard-code seed ids — re-fetch baselines or assert by relationship (`slot.cur_ammo_type == slot.default_ammo_type`).
- Sentinel ids fit in `i32`; cleanup deletes by exact sentinel, not by range.
- Live-DB tests use `require_db_or_skip!`, have `live_db` in the fn or module name (`live_db_*` fn, or a `*live_db*` module), and never share a database with another running test — `tools/test-live-db.sh` (or `.ps1`) clones the loaded database into one copy per slot of the `live-db` nextest test group (N = its `max-threads` in `.config/nextest.toml`), then runs `cargo nextest run --profile=ci-live-db --lib` over every crate with live-DB tests: up to N `live_db` tests at once, each on its slot's clone, and the rest in parallel. A second pool or helper gets its URL from `test_support::database_url()`, never from `DATABASE_URL` directly; guard tests fail a live-DB test without the marker and a direct `DATABASE_URL` read. Or run `cargo test ... -- --test-threads=1` if you're not on nextest. A new crate with a `cimmeria-test-support` dev-dependency goes in that script's list.
- Test names match the assertion. If the assertion changes, rename.
- No PR or issue numbers in source comments — provenance lives in the PR body.
- Update [docs/testing/inventory/<crate>.md](../docs/testing/inventory/) **only when a single PR adds or removes ≥5% of the workspace test count** (the current threshold is the last row of the generated totals in [docs/testing/inventory/README.md](../docs/testing/inventory/README.md#workspace-totals)). Smaller drifts get folded in by periodic sweep updates — don't block review on per-PR inventory churn for a handful of tests.

If a single feature touches several layers (handler logic + serializer + SQL + cross-handler invariant), expect to add several test types — see TESTING.md "When one feature needs more than one test".

## Required docs on every PR

A PR that changes user-visible behaviour, public surface, file layout, build steps, or test policy must update the corresponding doc(s). For non-trivial doc work, prefer the **Documentation Writer** agent over freehand edits — it follows the Diátaxis framework (tutorials / how-to / reference / explanations) and keeps voice consistent with the rest of `docs/`. The mapping of "what changed → what to update" is in [CLAUDE.md](../CLAUDE.md) under "Required documentation for every PR". Index entries in `docs/readme.md` and per-section `README.md` files must stay in sync with the documents they list — adding or renaming a doc means updating the index in the same PR.

Two rules keep shared docs from conflicting between PRs; flag PRs that break them:

- **Generated blocks are not hand-edited.** Text between `<!-- gen:NAME -->` and `<!-- /gen:NAME -->` markers (test counts, findings counts, the gap-analysis totals) and the crate graph between the `crate-graph` markers belong to `tools/docs-gen/regen.py`. The `regen-docs` workflow reruns it on `main` after every merge. A PR changes a generated block only when it adds the marker.
- **Status docs change once per campaign.** `docs/gap-analysis.md` and `docs/project-status.md` are updated in a campaign's close-out or release packet. Per-packet progress goes in the campaign's ledger under `docs/analysis/<campaign>/`.

Run the markdown lint as the doc-side equivalent of `cargo clippy`: `tools/lint-md.sh` (or `.ps1` on Windows). Same ruleset CodeRabbit applies in PR review — local catches every cosmetic finding before the bot has to type it. Warn-only in CI for now; Phase 2 hardens to blocking. Config: [.markdownlint-cli2.yaml](../.markdownlint-cli2.yaml).

## Where to find more

Full conventions: `CLAUDE.md`. Project rules and known traps: `docs/agents/rules-and-gotchas.md`. Evidence rules when sources disagree: `docs/agents/domain.md`. Testing: `TESTING.md`. Architecture: `docs/architecture/`. Content engine: `docs/architecture/data-driven-content-engine.md`. Roadmap + status: `docs/project-status.md`, `docs/gap-analysis.md` (**not** `docs/architecture/migration-roadmap.md` — that is a historical C++-only dependency plan; its "CRITICAL OpenSSL" row is not a Cimmeria finding). Live-DB infra: `docs/architecture/integration-test-infra.md`.
