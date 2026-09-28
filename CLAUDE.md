# Cimmeria — Stargate Worlds Emulator

A server emulator for Stargate Worlds. Active development is in Rust (`crates/`).

For human-readable project overview, see [README.md](README.md).

## Repo invariants (non-obvious)

- `external/` is **not in git** — populated by `setup.ps1`. A fresh checkout looks broken until setup runs.
- Active schemas: `db/database.sql`, `db/sgw/`, `db/resources/`.
- Frontend convention: every meaningful frontend change requires a REPL-style logic UAT in addition to tests/builds — see [AGENTS.md](AGENTS.md).
- The game client is **not in git** either — `game/sgw/` is a placeholder. Map, prefab, and navmesh work needs your own client copy.

## Project rules (read before designing)

The full list, with the reasons, is [docs/agents/rules-and-gotchas.md](docs/agents/rules-and-gotchas.md). The ones that cost the most when missed:

- **Check `docs/` first.** Search the docs and the `docs/protocol/*-dispatch-table.md` files before Ghidra, before counting `.def` entries, before guessing.
- **A ticket or draft chapter is a claim, not evidence.** Reconcile it against `docs/protocol/` and the RE findings before changing a wire constant, index, or layout. If sources disagree, verify against the binary and fix the losing doc in the same PR — see [docs/agents/domain.md](docs/agents/domain.md).
- **Wire entity typeIDs are the client's clientIndex** (`<ServerOnly/>` entries skipped): `Account = 0x07`, not its `entities.xml` row.
- **Seeds are the source of truth.** Change seeded data in `db/resources/`; do not add `db/scripts/*.sql` migrations without asking.
- **Prefer server-authoritative changes that need no client patch.** New opcodes, wire-crypto changes, and new client UI need a maintainer decision first.
- **Every button press gets visible feedback on the first press**, whatever the original server did.
- **Status docs change once per campaign.** `docs/gap-analysis.md` and `docs/project-status.md` are updated in the campaign's close-out or release packet, not per packet. Per-packet progress goes in the campaign's ledger under `docs/analysis/<campaign>/`.
- **Never hand-edit a generated block.** Text between `<!-- gen:NAME -->` markers (and the crate graph) is owned by `tools/docs-gen/regen.py`, which a workflow reruns on `main` after every merge. Run it locally to see the numbers, but commit the result only when your PR adds the marker.
- **Parallel agents: one worktree each, one test database each**, and every compiling `cargo` call goes through the build lane below. Workflow and agent roster: [docs/agents/development-workflow.md](docs/agents/development-workflow.md).
- **Project memory is committed.** Project and reference facts you learn while researching or writing code go in `.claude/agent-memory/main-session/` (subagents use their own folder), committed with the change that produced them. Personal preferences, local paths and in-flight session state stay in your personal memory. The repo is public, so no IPs, credentials or account names. Verified facts graduate to `docs/`. Rules: [development-workflow.md § Project memory](docs/agents/development-workflow.md#project-memory).

Shared project memory index, loaded every session: @.claude/agent-memory/main-session/MEMORY.md

## Build rules

Always target **Windows** — the server runs on Windows alongside the game client. Build natively on Windows, from PowerShell or Git Bash. The WSL cross-compile is retired; CI still builds and tests on Linux runners.

```bash
cargo build -p cimmeria-server --release
cp target/release/cimmeria-server.exe .
```

After building, copy the exe to the project root. A build that went through the build lane on a Dev Drive writes to the target dir the lane prints when it starts (`$CIMMERIA_TARGET_ROOT/<worktree>/release/`), not to `target/`.

- **One pinned toolchain.** [rust-toolchain.toml](rust-toolchain.toml) pins Rust 1.98.1 with rustfmt and clippy. rustup picks it up automatically, and CI installs the same version through [.github/actions/rust-toolchain](.github/actions/rust-toolchain/action.yml), so your clippy is CI's clippy. Bump the version in its own PR: change the file, run the pre-PR checklist, and fix the new lints in that PR.
- **Debug info.** The dev profile keeps line tables only (`debug = "line-tables-only"`), so panics and backtraces keep file:line but a debugger sees no variables. For a debugger session, build with `cargo build --profile dev-debug`: full debug info, built into `target/dev-debug/`, so it never invalidates the normal dev build.

The reasons behind all of this are in [docs/architecture/build-system.md](docs/architecture/build-system.md).

### Build lane and concurrency

Several sessions and agents build on one workstation at once. Every agent or worker `cargo` call that compiles (`check`, `build`, `test`, `nextest`, `clippy`) goes through the build lane, [tools/build-lane/lane.sh](tools/build-lane/lane.sh):

```bash
bash tools/build-lane/lane.sh cargo check -p cimmeria-cell
bash tools/build-lane/lane.sh --exclusive cargo build --workspace ...   # workspace-wide or measurement runs
```

1. **One `cargo` per lane slot.** The lane is a machine-wide semaphore. The slot count lives in `%LOCALAPPDATA%\cimmeria-build\lane\SLOTS` (currently 4), `--exclusive` takes every slot, and `CARGO_BUILD_JOBS` defaults to cores ÷ slots. A slot whose holder died is freed by the next caller, so there is nothing to kill.
2. **Iterate per crate with `-p`.** `cimmeria-services` is a small facade over about 20 crates, so check and test the crate you changed (`-p cimmeria-cell`, `-p cimmeria-cell-content`, `-p cimmeria-base-methods`, …). Build the workspace only for final validation.
3. **The lane sets up the build environment:** a target dir per worktree (on the Dev Drive when one is set up), incremental builds for workspace crates, and sccache for third-party crates. A direct `cargo` call bypasses the slot count, which is why agents never make one.
4. **Every lane job is logged.** `python tools/build-lane/lane_stats.py` reports on the log (`--recent 20` for the last jobs, `--html` for charts, `--csv` for a spreadsheet).

Worktrees, per-worktree test databases and Dev Drive seeding: [docs/agents/development-workflow.md](docs/agents/development-workflow.md). **Retire a worktree the day its PR merges** with `bash tools/build-lane/rm-worktree.sh <name>`: it deletes the target dir and the test database, and unlinks `external/` safely. A session that dispatched workers retires their worktrees too; `--merged` sweeps every merged, idle one. Stale target dirs filled the Dev Drive once and stopped every lane build.

Quick reference:

```bash
# Iteration: check the crate you changed. cimmeria-services is only the
# facade now (the orchestrator, the database pool and the cross-track
# round trips); the service code and its tests are in the split crates.
bash tools/build-lane/lane.sh cargo check -p cimmeria-cell

# Single-crate test: name the crate you changed.
bash tools/build-lane/lane.sh cargo test -p cimmeria-cell

# Full workspace check — skip the GUI apps (Tauri editors and the egui
# launcher), and the Windows-only client-telemetry and client-patches cdylibs
# so CI's Linux runners don't need xkbcommon/xcb dev packages. Same seven
# exclusions as CI.
bash tools/build-lane/lane.sh --exclusive cargo check --workspace \
  --exclude cimmeria-app \
  --exclude cimmeria-content-editor \
  --exclude cimmeria-scene-editor \
  --exclude sgw-launcher \
  --exclude cimmeria-client-telemetry --exclude cimmeria-client-patches --exclude cimmeria-lab

# Full debug info for a debugger session (builds into target/dev-debug/)
bash tools/build-lane/lane.sh cargo build -p cimmeria-server --profile dev-debug

# What the lane has been doing
python tools/build-lane/lane_stats.py --recent 20
```

## Pre-PR checklist

Run everything in the block below before pushing, or the pipeline will fail and you'll round-trip. Not all of it blocks a merge — here's what each command is actually worth:

| Check | Workflow | Blocking? | Runs when |
|---|---|---|---|
| `fmt`, `clippy`, `build`, `test`, `test-live-db` | [test.yml](.github/workflows/test.yml) | **Yes** — five gating jobs | Every PR |
| `figure-sources-in-sync` | [figure-sources.yml](.github/workflows/figure-sources.yml) | **Yes** | Only when `docs/drafts/spec/figures/**` changes |
| `figure-style-lint` | [figure-style.yml](.github/workflows/figure-style.yml) | **Yes** | Only when figures or `docs/drafts/spec/**.md` change |
| `markdownlint` | [markdownlint.yml](.github/workflows/markdownlint.yml) | No — warn-only annotations | Any `**/*.md` change |
| `spec-lint` | [spec-lint.yml](.github/workflows/spec-lint.yml) | No — warn-only annotations | `docs/spec/**`, `crates/**`, manifest changes |

A sixth job in `test.yml`, `coverage`, uploads to Codecov and does not gate. After a merge, [regen-docs.yml](.github/workflows/regen-docs.yml) reruns `tools/docs-gen/regen.py` on `main` and commits any generated doc blocks that changed; nothing in a PR checks them. The test runner in CI is [`cargo-nextest`](https://nexte.st/); install it once with `cargo install cargo-nextest --locked` (or `taiki-e/install-action@nextest` if you already use that pattern).

Agents run the compiling commands below through the build lane, with `--exclusive` for the workspace-wide ones (`bash tools/build-lane/lane.sh --exclusive cargo clippy …`).

```bash
cargo fmt --all -- --check
# The workspace-hack (cargo-hakari) must match the manifests; after a
# dependency change, regenerate with `cargo hakari generate && cargo hakari manage-deps --yes`:
cargo hakari generate --diff && cargo hakari manage-deps --dry-run
cargo clippy --workspace \
  --exclude cimmeria-app --exclude cimmeria-content-editor \
  --exclude cimmeria-scene-editor --exclude sgw-launcher \
  --exclude cimmeria-client-telemetry --exclude cimmeria-client-patches --exclude cimmeria-lab \
  --all-targets -- -D warnings
cargo build --workspace \
  --exclude cimmeria-app --exclude cimmeria-content-editor \
  --exclude cimmeria-scene-editor --exclude sgw-launcher \
  --exclude cimmeria-client-telemetry --exclude cimmeria-client-patches --exclude cimmeria-lab --all-targets
cargo nextest run --profile=ci --workspace \
  --exclude cimmeria-app --exclude cimmeria-content-editor \
  --exclude cimmeria-scene-editor --exclude sgw-launcher \
  --exclude cimmeria-client-telemetry --exclude cimmeria-client-patches --exclude cimmeria-lab
# Doctests aren't run by nextest — only cimmeria-commands has runnable
# doctests today, so this is a one-crate sanity check:
cargo test --doc -p cimmeria-commands

# Live-DB tests — start the bundled Postgres first, then run every crate
# with live-DB tests in one serialised nextest run (the crate list lives in
# the script; tools/test-live-db.ps1 on Windows PowerShell):
DATABASE_URL=postgres://w-testing:w-testing@localhost:5433/sgw \
  tools/test-live-db.sh
# From a worktree, use the worktree's own database instead: this reloads
# sgw_<worktree> from db/database.sql and runs the same tier in a lane slot.
tools/build-lane/live-db-test.sh <test-name filter>

# Markdown lint (warn-only — CI surfaces violations as PR annotations but
# never blocks). Same rules as CodeRabbit's review:
tools/lint-md.sh                    # macOS / Linux / WSL
tools/lint-md.ps1                   # Windows PowerShell
tools/lint-md.sh --fix              # auto-fix what's auto-fixable

# Figure source ↔ render sync check (BLOCKING — CI fails if a source DSL
# under docs/drafts/spec/figures/sources/ was committed without its
# regenerated SVG):
tools/check-figure-sources.sh       # macOS / Linux / WSL
tools/check-figure-sources.ps1      # Windows PowerShell

# Figure style + format lint (BLOCKING — CI fails on Mermaid init-directive
# omissions, theme-aware backdrop misses, non-sequential Figure numbering,
# dangling image refs, generic alt text. Rule catalog inside the script.):
tools/lint-figure-style.sh          # macOS / Linux / WSL
tools/lint-figure-style.ps1         # Windows PowerShell
```

`cargo test -p <crate>` still works for quick crate-level iteration. Use nextest for anything you'd be uploading to CI.

The markdown lint runs via [`markdownlint-cli2`](https://github.com/DavidAnson/markdownlint-cli2) against [`.markdownlint-cli2.yaml`](.markdownlint-cli2.yaml) at the repo root. CI mirrors local invocation via [`DavidAnson/markdownlint-cli2-action`](.github/workflows/markdownlint.yml). First local run downloads the binary on-demand via `npx`; running `npm install` once pins the version from `package.json` for offline reuse. Phase 2 hardens the lint from warn-only to blocking — until then, fix what's easy and let reviewers nudge the rest.

- **fmt fails** → `cargo fmt --all` and commit the result. The CI job tells you exactly that.
- **clippy fails** → fix the warning. Project-level thresholds for `too_many_arguments` (14) and `type_complexity` (500) live in `clippy.toml`; bumping those further requires the same kind of justification any other lint suppression would. Don't sprinkle `#[allow(clippy::…)]` per call site. **Passes locally but fails in CI?** That is no longer toolchain drift: [rust-toolchain.toml](rust-toolchain.toml) pins the version CI uses, so the old advice to install CI's newer clippy side by side is obsolete. Check that you ran the exact command above (`--workspace --all-targets -- -D warnings`), not a narrower `-p` run. Bumping Rust is its own PR: change `rust-toolchain.toml`, run this checklist, and fix the new lints there.
- **build fails** → typically a stale path or unused-symbol cleanup needed; check matches `cargo check`. If the failing step is **workspace-hack is current**, a dependency change didn't regenerate the hakari crate: run `cargo hakari generate && cargo hakari manage-deps --yes` (install it once with `cargo install cargo-hakari --locked`) and commit the result. See [docs/architecture/build-system.md](docs/architecture/build-system.md) §6.
- **test fails (no DB)** → unit + non-DB integration tests. Live-DB tests self-skip via `require_db_or_skip!` when `DATABASE_URL` is unset, so this run can be green even with broken DB code.
- **test-live-db fails** → CI runs `tools/test-live-db.sh` (`cargo nextest run --profile=ci-live-db --lib` over every crate in its list) against a fresh `postgres:17.9` service container loaded from `db/database.sql`. A crate with a `cimmeria-test-support` dev-dependency must be in that list, or `live_db_wrapper_lists_every_test_support_crate` fails. The `ci-live-db` profile in `.config/nextest.toml` serialises every test (`threads-required = "num-test-threads"`) because some live-DB tests share sentinel id ranges and would collide under parallel execution against a single shared DB. To repro locally, start the bundled Postgres on `:5433` and run the command in the snippet above.
- **figure-sources-in-sync fails** → A source DSL under `docs/drafts/spec/figures/sources/` was committed more recently than its rendered SVG one directory up. Re-render the affected diagram (Prixmaviz, or the local renderer per [docs/drafts/spec/figures/sources/README.md](docs/drafts/spec/figures/sources/README.md)) and commit the regenerated SVG alongside the source change. Pairing rule: `sources/<slug>.<ext>` pairs with `<slug>.svg`.
- **figure-style-lint fails** → A figure source, rendered SVG, or chapter convention violated the style rule catalog inside [tools/lint-figure-style.sh](tools/lint-figure-style.sh). Common causes: Mermaid `flowchart`/`sequenceDiagram` missing the `htmlLabels:false` init directive (rules M1/M2), an SVG missing the cimmeria-bg theme-aware backdrop marker (S1), Graphviz intrinsic `fill="white"` backdrop polygon not stripped (S3), non-sequential `*Figure N:*` captions (C1), generic image alt text (C2), or a dangling image reference (C3). Run the script locally to see the specific rule code and remediation hint.

## Required testing for every PR

A PR that changes runtime behavior without adding or updating a test will be sent back. **Before writing a test, read [TESTING.md](TESTING.md)** — it covers the twelve test types we use (unit / wire-format / live-DB / smoke / concurrency / chain-replay / legacy reference / fan-out byte / Mercury session / network chaos / wire-level replay / negative-log), the picker for which type fits which bug shape, and the gotchas mined from PR reviews #131 onwards.

The non-negotiables:

- **Pick the right type.** If you change a `WHERE` clause or `rows_affected` invariant, you need a live-DB regression guard, not a unit test. If you change a serializer, you need a byte-exact wire-format test. The picker table is in TESTING.md.
- **Reproduce the bug shape.** A regression guard must fail when the fix is reverted; if it doesn't, it's a happy-path test, not a guard. PR reviewers will check.
- **One feature can need multiple tests.** Vendor stack changes typically need unit + wire-format + live-DB + smoke. Don't skip a layer because "the next layer up will catch it" — that's the bug shape TESTING.md exists to prevent.
- **Live-DB tests use `require_db_or_skip!`** and run serialised. Under nextest the `ci-live-db` profile pins this with `threads-required = "num-test-threads"`; with `cargo test`, pass `-- --test-threads=1`. Sentinels fit in `i32`. Cleanup deletes by exact sentinel, not by range. The gate lives in `crates/test-support/` (`cimmeria-test-support`), re-exported from each crate's `crate::test_support`.

## Required documentation for every PR

A PR that changes user-visible behavior, public surface, file layout, build steps, or test policy must include the corresponding doc update. **CI does not gate this; reviewers do.** When updating, prefer to use the **Documentation Writer agent** (Diátaxis-aware: tutorials / how-to / reference / explanation) rather than freehand edits — it keeps voice and structure consistent with the rest of `docs/`.

The map of "what changed → what to update":

| If you change… | Update… |
|---|---|
| The README's listed feature set, status, or structure | [README.md](README.md) |
| The pre-PR checklist, build commands, or repo invariants | [CLAUDE.md](CLAUDE.md) and [.github/copilot-instructions.md](.github/copilot-instructions.md) |
| The toolchain pin, cargo profiles, `.cargo/config.toml`, or the build tooling under `tools/build-lane/`, `tools/dev-drive/`, `tools/build-hygiene/` or `tools/build-metrics/` | [docs/architecture/build-system.md](docs/architecture/build-system.md) (the ADR), "Build rules" in this file and [.github/copilot-instructions.md](.github/copilot-instructions.md), the worktree and lane rules in [docs/agents/development-workflow.md](docs/agents/development-workflow.md), and [docs/troubleshooting.md](docs/troubleshooting.md) if the change adds a new failure mode |
| Test conventions, types, or gotchas | [TESTING.md](TESTING.md) (and re-link from README if a new section is added) |
| Markdown lint rules, exclusions, or the wrapper scripts | [.markdownlint-cli2.yaml](.markdownlint-cli2.yaml), [tools/lint-md.sh](tools/lint-md.sh), [tools/lint-md.ps1](tools/lint-md.ps1), and the workflow at [.github/workflows/markdownlint.yml](.github/workflows/markdownlint.yml) |
| Add or remove ≥5% of workspace tests in one PR (~<!-- gen:tests-threshold -->393<!-- /gen:tests-threshold --> tests at the current <!-- gen:tests-total -->7,869<!-- /gen:tests-total --> baseline; both generated) | [docs/testing/inventory/<crate>.md](docs/testing/inventory/). The totals in [docs/testing/inventory/README.md](docs/testing/inventory/README.md) are generated, so leave them alone. Smaller drifts roll up via periodic sweep updates rather than per-PR churn. |
| Live-DB infra or local setup | [docs/architecture/integration-test-infra.md](docs/architecture/integration-test-infra.md) |
| Crate layout, dependency graph, or new crate | [crates/README.md](crates/README.md) (crate table). The crate diagrams in [README.md](README.md) and crates/README.md are generated and regenerated on `main` after the merge, so don't commit a regenerated graph; add a new crate to a layer in [tools/crate-graph/groups.toml](tools/crate-graph/groups.toml) |
| A new crate split out of an existing one, or a module moved between crates | A new crate with live-DB tests goes in [tools/test-live-db.sh](tools/test-live-db.sh) and [.ps1](tools/test-live-db.ps1). An in-process crate also needs an `IN_PROCESS_CRATES` entry in `crates/server/src/logging/target_scan_tests.rs`, and an `OTEL_FILTER` row of its own (never a bare name that prefixes a sibling crate's, such as `cimmeria_cell`), or a `NO_OWN_ROW` entry that says why not. `crates/server/src/logging/parity_tests/crate_rows.rs` checks both. Moved tracing targets follow the move in the `FILE_LAYERS` rows of `crates/server/src/logging/filters.rs`. Update the crate table and graph as the row above says, and regenerate the workspace-hack (`cargo hakari generate && cargo hakari manage-deps --yes`). How the services split was done: [docs/architecture/services-crate-split.md](docs/architecture/services-crate-split.md). The layering guard that policed the split's planned crate DAG is retired: the crates now exist, and a new dependency edge shows up in the regenerated crate graph |
| Wire format, method indices, or message catalog | [docs/protocol/client-method-dispatch-table.md](docs/protocol/client-method-dispatch-table.md), [docs/protocol/message-catalog.md](docs/protocol/message-catalog.md), the rest of [docs/protocol/](docs/protocol/), the canonical entity definitions under [entities/defs/](entities/defs/), and the `method_idx` constants module in `crates/wire/src/mercury/mod.rs` |
| Mercury protocol-layer behavior (channel state, retransmit, fragmentation, keepalive, ack, RTO) or the loopback harness itself | [docs/architecture/mercury-loopback-harness.md](docs/architecture/mercury-loopback-harness.md), TESTING.md type 9, and (if the harness API surface changes) the `test_harness` module under [crates/mercury/src/test_harness/](crates/mercury/src/test_harness/) plus the `cimmeria-mercury` row in [crates/README.md](crates/README.md) |
| Network-chaos primitives, lossy-socket wrappers, pcap-replay infra, or any new chaos scenario | [docs/architecture/network-chaos-testing.md](docs/architecture/network-chaos-testing.md), TESTING.md type 10, plus the `cimmeria-mercury` row in [crates/README.md](crates/README.md) if the L2 trait surface widens. New scenarios drop under [crates/mercury/src/test_harness/tests/chaos/](crates/mercury/src/test_harness/tests/chaos/). |
| Wireclient (`cimmeria-wireclient`) public API, the `session_trace` JSONL schema, or the pcap exporter | [docs/architecture/wireclient.md](docs/architecture/wireclient.md), TESTING.md type 11, the `cimmeria-wireclient` row in [crates/README.md](crates/README.md), and (if the pcap exporter shape changes) [tools/pcap_to_session.py](tools/pcap_to_session.py) |
| Figure source DSL under `docs/drafts/spec/figures/sources/` | Must re-render and commit the matching SVG in [docs/drafts/spec/figures/](docs/drafts/spec/figures/) in the same PR — gated by [tools/check-figure-sources.sh](tools/check-figure-sources.sh) and the [figure-sources-in-sync workflow](.github/workflows/figure-sources.yml). |
| Architecture decisions (cell/base split, outbox, state-flag conventions, etc.) | New or amended doc under [docs/architecture/](docs/architecture/) |
| Dev-session telemetry pipeline, the `/auth/dev-session` HMAC token, or any change to the launcher's `telemetry/` module tree | [docs/architecture/dev-session-telemetry.md](docs/architecture/dev-session-telemetry.md) (design), [docs/operations/telemetry.md](docs/operations/telemetry.md) (operator runbook + secret rotation), and the `CIMMERIA_TELEMETRY_HMAC_SECRET` row in the env-var table at the top of [crates/server/src/main.rs](crates/server/src/main.rs) when the secret-handling code changes |
| Server-side observability — OTLP exporter, Mercury packet instrumentation, SigNoz overlay, Cloudflare Tunnel, or the SigNoz↔Cimmeria-MCP integration surface | [docs/architecture/observability.md](docs/architecture/observability.md) (ADR — includes target catalog and `decision_outcome` enum), [docs/architecture/instrumentation-discipline.md](docs/architecture/instrumentation-discipline.md) (success-side ADR — span placement, event level rules, metric label cardinality), [docs/operations/signoz-deployment.md](docs/operations/signoz-deployment.md) (runbook), [docs/operations/signoz-remote-access.md](docs/operations/signoz-remote-access.md) (tunnel + Access auth), and the `OTEL_*` rows in the env-var table at the top of [crates/server/src/main.rs](crates/server/src/main.rs) when the exporter contract changes |
| Adding or modifying a negative log on an expectation seam (silent `let _ = .send(...)`, `rows_affected == 0`, witness/lookup miss) | [docs/architecture/negative-logging-convention.md](docs/architecture/negative-logging-convention.md) — field-naming rules, level discipline, `LogCapture` test helper |
| Adding a new Discord notification event type, channel, or toggling default | [docs/architecture/discord-notifications.md](docs/architecture/discord-notifications.md) (design + ops), [config/discord.toml.example](config/discord.toml.example) (schema + defaults), and the `cimmeria-discord` row in [crates/README.md](crates/README.md) — add the new variant to `EventKind`, `EventToggles`, `router::channel_for`, `embed::format_event`, and one typed helper in `crates/discord/src/lib.rs` (the `event_kind_all_matches_variant_count` test pins the count so forgetting a step trips it). **If the new channel should also fire on the colo**: add a `[discord.channels.<X>]` block + `__DISCORD_<X>_WEBHOOK__` sentinel to [docker/compose.discord.yml](docker/compose.discord.yml), a matching awk arm in the render step of [.github/workflows/release-container.yml](.github/workflows/release-container.yml), and the new `DISCORD_<X>_WEBHOOK` GitHub Actions secret. Update [docs/operations/colo-deploy.md](docs/operations/colo-deploy.md#optional-discord-notifications) if the operator surface changes. |
| Mission PAK overrides / new client-visible mission steps not in the canonical PAK | [docs/architecture/mission-pak-overrides.md](docs/architecture/mission-pak-overrides.md) and [docs/content/equip-from-inventory-pattern.md](docs/content/equip-from-inventory-pattern.md) (when the new step is part of an equip flow); cross-link from the mission's row in [docs/content/mission-chains.md](docs/content/mission-chains.md) |
| Game systems, content chains, or content-engine actions | [docs/game-systems.md](docs/game-systems.md) and/or [docs/content/](docs/content/), plus [.github/instructions/content-chains.instructions.md](.github/instructions/content-chains.instructions.md) if review rules shift |
| Abilities, effects, target collection, pulsing, stacking, shields, channels, stuns, or any `cell/effects/` or `cell/abilities/cone_aoe/` work | [docs/architecture/abilities-and-effects-system.md](docs/architecture/abilities-and-effects-system.md) — the cross-cutting ADR (EffectScript trait shape, refcount via state_flag_counts, channel cancellation triggers, AF_CHANNEL_ALLOWS_MOVEMENT default, absorption pool drain ordering). New scripts go in [crates/cell-world/src/cell/effects/scripts.rs](crates/cell-world/src/cell/effects/scripts.rs) with a matching `match` arm in [registry.rs](crates/cell-world/src/cell/effects/registry.rs). |
| Project status, gap analysis, or roadmap | [docs/project-status.md](docs/project-status.md) and [docs/gap-analysis.md](docs/gap-analysis.md), **once per campaign, in its close-out or release packet**; per-packet progress goes in the campaign ledger under `docs/analysis/<campaign>/`. **Not** [docs/architecture/migration-roadmap.md](docs/architecture/migration-roadmap.md) — that is a historical C++-only dependency plan, and its "CRITICAL OpenSSL" row describes the deprecated tree, never Cimmeria (Rust uses `tokio-rustls`). |
| Cross-cutting server-only infrastructure (session lifecycle, rate limiting, world-state persistence, scheduling, currency-flow logging) | [docs/architecture/server-infrastructure-proposals.md](docs/architecture/server-infrastructure-proposals.md) for the design, [docs/gap-analysis.md](docs/gap-analysis.md) §"Server Infrastructure (Cross-Cutting)" for status. [docs/architecture/server-systems.md](docs/architecture/server-systems.md) is a superseded pointer page — do not add content to it |
| UE3 package binary format, `.upk`/`.umap` parsing, or [`crates/upk-objects/`](crates/upk-objects/) | [docs/engine/ue3-package-format.md](docs/engine/ue3-package-format.md) and the `engine/` row in [docs/engine/README.md](docs/engine/README.md) |
| Reverse-engineering toolchain (Ghidra MCP, x64dbg MCP, `.mcp.json`, the RE workflow with Claude) | [docs/guides/re-toolchain-setup.md](docs/guides/re-toolchain-setup.md), [docs/guides/reverse-engineering-with-claude.md](docs/guides/reverse-engineering-with-claude.md), [docs/reverse-engineering/toolchain/install-ghidra-mcp.md](docs/reverse-engineering/toolchain/install-ghidra-mcp.md), [`.mcp.json.example`](.mcp.json.example), and the bootstrap module ([bootstrap/CimmeriaBootstrap/Public/Install-CimmeriaReToolchain.ps1](bootstrap/CimmeriaBootstrap/Public/Install-CimmeriaReToolchain.ps1)) if you change the install steps |
| New RE finding or addition to `docs/reverse-engineering/` tree | [docs/reverse-engineering/README.md](docs/reverse-engineering/README.md) (top-level index), [docs/reverse-engineering/findings/README.md](docs/reverse-engineering/findings/README.md) (if adding a finding), and the relevant per-system row in this map |
| Live research lab — bridge (`cimmeria-client-telemetry` `--features lab-bridge`), supervisor (`cimmeria-lab`), or in-server endpoint (`cimmeria-lab-mcp`) | [docs/architecture/live-research-lab.md](docs/architecture/live-research-lab.md) (ADR), [docs/guides/live-research-lab.md](docs/guides/live-research-lab.md) (rulebook + operating manual), the `lab` / `lab-mcp` / `client-telemetry` rows in [crates/README.md](crates/README.md), [`.mcp.json.example`](.mcp.json.example), and — for the colo WireGuard-only port — [docker/compose.lab.yml](docker/compose.lab.yml) + [docs/operations/colo-deploy.md](docs/operations/colo-deploy.md). Client-telemetry stops being emit-only under the feature: update [docs/architecture/client-telemetry.md](docs/architecture/client-telemetry.md). The `CIMMERIA_LAB_MCP_*` env rows live in [crates/server/src/main.rs](crates/server/src/main.rs) |
| First-time build / setup flow, `setup.ps1` phases, prerequisites, or any user-visible bootstrap behavior | [docs/building.md](docs/building.md) (Rust how-to), [docs/guides/getting-started.md](docs/guides/getting-started.md) (tutorial), [bootstrap/README.md](bootstrap/README.md), and [docs/troubleshooting.md](docs/troubleshooting.md) if the change introduces a new first-day failure mode |
| Contribution scope, PR conventions, code-style rules, or first-issue guidance | [CONTRIBUTING.md](CONTRIBUTING.md) and (if the change affects what reviewers check) this `CLAUDE.md` |
| Common first-day problems or known failure modes operators / contributors hit | [docs/troubleshooting.md](docs/troubleshooting.md) |
| Per-system game-mechanic docs (combat, abilities, missions, inventory, social, ...) | [docs/gameplay/<system>.md](docs/gameplay/) and [docs/game-systems.md](docs/game-systems.md) for cross-system summaries |
| UAT steps for a restored system (new or changed tester-facing behaviour) | The campaign's own checklist under [docs/analysis/](docs/analysis/) (its ledger stays canonical) **and** the matching section of [docs/guides/unified-uat.md](docs/guides/unified-uat.md), keeping the campaign's step ids; add a new system's section, its row in the guide's checklist table, and any new known issue to the guide's "Current known issues" table |
| Engine internals (BigWorld, CME, cooked-data pipeline, entity LOD, watchers, space management) | [docs/engine/<file>.md](docs/engine/) and the relevant `docs/reverse-engineering/findings/` entries if evidence shifts |
| Client-side analysis (launcher design, audio/voice, FaceFX, UI/Scaleform, asset inventories) | [docs/client/<file>.md](docs/client/) and the `docs/client-tools.md` index if a new tool surface is added |
| Admin-API / Admin-panel / Tauri-app surface (REST routes, WebSocket streams, IPC commands) | [docs/tools/admin-api.md](docs/tools/admin-api.md), [docs/tools/admin-panel.md](docs/tools/admin-panel.md), and the `cimmeria-admin-api` row in [crates/README.md](crates/README.md) if the public surface shifts |
| Developer how-to guides (adding a handler, extending the content engine, writing a migration) | [docs/guides/add-a-message-handler.md](docs/guides/add-a-message-handler.md), [docs/guides/extend-the-content-engine.md](docs/guides/extend-the-content-engine.md), [docs/guides/write-a-database-migration.md](docs/guides/write-a-database-migration.md). When adding a new how-to, also link from [docs/readme.md](docs/readme.md) → `guides/` and from [CONTRIBUTING.md](CONTRIBUTING.md). |
| Operations / deployment runbooks (container image, colo deploy, telemetry, SigNoz) | [docs/operations/<file>.md](docs/operations/) and the `Top-Level Documents` table in [docs/readme.md](docs/readme.md) if a new operator-facing entry is added |
| AI-harness configuration: issue-tracker, triage-label, or domain-doc settings read by agent skills; the agent workflow or roster; a new project rule, maintainer decision, or gotcha a contributor off this machine would need | [docs/agents/](docs/agents/) (`issue-tracker.md`, `triage-labels.md`, `domain.md`, `development-workflow.md`, `rules-and-gotchas.md`) and its table in [docs/readme.md](docs/readme.md). If the rule is one of the few that must load every session, also the "Project rules" list in this file, [AGENTS.md](AGENTS.md), and [.github/copilot-instructions.md](.github/copilot-instructions.md). A new subagent goes in `.claude/agents/` with a roster row in `development-workflow.md`. |

Index entries in [docs/readme.md](docs/readme.md) and the per-section `README.md` files (`docs/content/README.md`, `docs/protocol/README.md`, etc.) must stay in sync with the documents they list — adding or renaming a doc means updating the index in the same PR.

## File organization

Files should "do what it says on the tin" — a reader (human or LLM) should predict a file's contents from its name. Split large files along natural seams to keep both LLM context and human review tractable.

- **Soft cap: 500 lines. Hard cap: 700 lines.**
  - Under 500: leave alone.
  - 500–700: split if a natural seam exists (handler groups, lifecycle phases, message-type families, etc.). If the file is one cohesive concept with no seam, leave it.
  - Over 700: must split.
- **Split along natural seams, not arbitrary line counts.** Group methods that share state, lifecycle, or call patterns. Line count is a *signal to look for seams*, not a target.
- **Flat names for 2–3 siblings; directory for 4+.** Prefer `inventory_grant.rs` + `inventory_move.rs` (2 siblings, flat) over `inventory/grant.rs` + `inventory/move.rs`. Promote to a directory once you cross 4 files on the same theme.
- **Re-export discipline.** When a file becomes a directory, the new `mod.rs` should `pub use` the submodules' public types so external callers' imports don't change. Splits are internal refactors, not public-surface changes.
- **Foresight rule.** When creating a new file you can already see will accumulate siblings (handler-per-message, method-per-feature), start it as a directory from day one. Heuristic: *if you can name 3+ logical sibling files now, make the directory now.* See `crates/base-methods/src/base/world_entry/methods/vendor/` for the canonical example.
- **Naming.** Avoid `helpers.rs`, `utils.rs`, `misc.rs`, `extra.rs` — they hide content. Use `cooldowns.rs`, `damage_resolution.rs`, `witness_list.rs`.
- **Module style.** The repo uses `foo/mod.rs` (not the modern `foo.rs` + `foo/` style). Stay consistent.

## Agent skills

Per-repo configuration for agent skills that triage, write tickets, or model the domain lives in [docs/agents/](docs/agents/):

- **Issue tracker:** GitHub Issues for `SandboxServers/Cimmeria` via the `gh` CLI — [issue-tracker.md](docs/agents/issue-tracker.md), including the ticket body contract.
- **Triage labels:** `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix` — [triage-labels.md](docs/agents/triage-labels.md), including what `ready-for-agent` requires here. Unattended agents pick work only from issues a maintainer has labeled `ready-for-agent` — see [docs/guides/autonomous-agent-kickoff.md](docs/guides/autonomous-agent-kickoff.md).
- **Domain docs:** glossary is `docs/spec/glossary.md`, ADRs are `docs/architecture/`. Do **not** create `CONTEXT.md` or `docs/adr/` — [domain.md](docs/agents/domain.md).
