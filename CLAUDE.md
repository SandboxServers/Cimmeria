# Cimmeria — Stargate Worlds Emulator

A server emulator for Stargate Worlds. Active development is in Rust (`crates/`).

For human-readable project overview, see [README.md](README.md).

## Repo invariants (non-obvious)

- `external/` is **not in git** — populated by `setup.ps1`. A fresh checkout looks broken until setup runs.
- Active schemas: `db/database.sql`, `db/sgw/`, `db/resources/`.
- Frontend convention: every meaningful frontend change requires a REPL-style logic UAT in addition to tests/builds — see [AGENTS.md](AGENTS.md).

## Build rules

Always target **Windows** — the server runs on Windows alongside the game client.

```bash
# WSL/Linux: cross-compile to Windows.
cargo build -p cimmeria-server --target x86_64-pc-windows-gnu --release
cp target/x86_64-pc-windows-gnu/release/cimmeria-server.exe .

# Windows natively:
cargo build -p cimmeria-server --release
cp target/release/cimmeria-server.exe .
```

After building, copy the exe to the project root.

### Rust build memory (WSL)

The full link can consume ~47 GB RAM. The workspace's `[profile.dev.package."*"]` strips dep debug info to bring this down to ~8 GB, but you still need to be careful:

1. **`cargo check -p cimmeria-services`** for iteration (1.5s, <2 GB). Only run full `cargo build`/`cargo test` when you actually need a binary or test results.
2. **Never run multiple `cargo`/`rustc` processes concurrently.** Kill stale ones before starting a new build: `pkill -f rustc`.
3. **Target specific crates** with `-p` rather than `--workspace`. Only build the workspace for final validation.
4. Sanity-check before building: `ps aux | grep -E "cargo|rustc" | grep -v grep`.
5. `CARGO_BUILD_JOBS=2` is set in `.bashrc` to cap parallel codegen.

Quick reference:

```bash
# Iteration
cargo check -p cimmeria-services

# Single-crate test
cargo test -p cimmeria-services

# Full workspace check — skip the Tauri apps so the linker doesn't OOM
cargo check --workspace \
  --exclude cimmeria-app \
  --exclude cimmeria-content-editor \
  --exclude cimmeria-scene-editor

# Kill stale builds
pkill -f "cargo|rustc"
```

## Pre-PR checklist

CI (`.github/workflows/test.yml`) gates five checks on every PR — run all five locally before pushing or the pipeline will fail and you'll round-trip:

```bash
cargo fmt --all -- --check
cargo clippy --workspace \
  --exclude cimmeria-app --exclude cimmeria-content-editor \
  --exclude cimmeria-scene-editor --exclude sgw-launcher \
  --all-targets -- -D warnings
cargo build --workspace \
  --exclude cimmeria-app --exclude cimmeria-content-editor \
  --exclude cimmeria-scene-editor --exclude sgw-launcher --all-targets
cargo test --workspace \
  --exclude cimmeria-app --exclude cimmeria-content-editor \
  --exclude cimmeria-scene-editor --exclude sgw-launcher

# Live-DB tests — start the bundled Postgres first, then:
DATABASE_URL=postgres://w-testing:w-testing@localhost:5433/sgw \
  cargo test -p cimmeria-services --lib -- --test-threads=1
```

- **fmt fails** → `cargo fmt --all` and commit the result. The CI job tells you exactly that.
- **clippy fails** → fix the warning. Project-level thresholds for `too_many_arguments` (14) and `type_complexity` (500) live in `clippy.toml`; bumping those further requires the same kind of justification any other lint suppression would. Don't sprinkle `#[allow(clippy::…)]` per call site.
- **build fails** → typically a stale path or unused-symbol cleanup needed; check matches `cargo check`.
- **test fails (no DB)** → unit + non-DB integration tests. Live-DB tests in `crates/services` self-skip via `require_db_or_skip!` when `DATABASE_URL` is unset, so this run can be green even with broken DB code.
- **test-live-db fails** → CI runs `cargo test -p cimmeria-services --lib -- --test-threads=1` against a fresh `postgres:17.9` service container loaded from `db/database.sql`. `--test-threads=1` is required: some live-DB tests share sentinel id ranges and would collide under parallel execution against a single shared DB. To repro locally, start the bundled Postgres on `:5433` and run the command in the snippet above.

## Required testing for every PR

A PR that changes runtime behavior without adding or updating a test will be sent back. **Before writing a test, read [TESTING.md](TESTING.md)** — it covers the seven test types we use (unit / wire-format / live-DB / smoke / concurrency / chain-replay / legacy reference), the picker for which type fits which bug shape, and the gotchas mined from PR reviews #131 onwards.

The non-negotiables:

- **Pick the right type.** If you change a `WHERE` clause or `rows_affected` invariant, you need a live-DB regression guard, not a unit test. If you change a serializer, you need a byte-exact wire-format test. The picker table is in TESTING.md.
- **Reproduce the bug shape.** A regression guard must fail when the fix is reverted; if it doesn't, it's a happy-path test, not a guard. PR reviewers will check.
- **One feature can need multiple tests.** Vendor stack changes typically need unit + wire-format + live-DB + smoke. Don't skip a layer because "the next layer up will catch it" — that's the bug shape TESTING.md exists to prevent.
- **Live-DB tests use `require_db_or_skip!`** and run with `--test-threads=1`. Sentinels fit in `i32`. Cleanup deletes by exact sentinel, not by range. See `crates/services/src/test_support.rs`.

## Required documentation for every PR

A PR that changes user-visible behavior, public surface, file layout, build steps, or test policy must include the corresponding doc update. **CI does not gate this; reviewers do.** When updating, prefer to use the **Documentation Writer agent** (Diátaxis-aware: tutorials / how-to / reference / explanation) rather than freehand edits — it keeps voice and structure consistent with the rest of `docs/`.

The map of "what changed → what to update":

| If you change… | Update… |
|---|---|
| The README's listed feature set, status, or structure | [README.md](README.md) |
| The pre-PR checklist, build commands, or repo invariants | [CLAUDE.md](CLAUDE.md) and [.github/copilot-instructions.md](.github/copilot-instructions.md) |
| Test conventions, types, or gotchas | [TESTING.md](TESTING.md) (and re-link from README if a new section is added) |
| Add or remove ≥5% of workspace tests in one PR (~55 tests at current 1071 baseline) | [docs/testing/inventory/<crate>.md](docs/testing/inventory/) — and the totals in [docs/testing/inventory/README.md](docs/testing/inventory/README.md). Smaller drifts roll up via periodic sweep updates rather than per-PR churn. |
| Live-DB infra or local setup | [docs/architecture/integration-test-infra.md](docs/architecture/integration-test-infra.md) |
| Crate layout, dependency graph, or new crate | [crates/README.md](crates/README.md) and the crate diagram in [README.md](README.md) |
| Wire format, method indices, or message catalog | [docs/protocol/client-method-dispatch-table.md](docs/protocol/client-method-dispatch-table.md), [docs/protocol/message-catalog.md](docs/protocol/message-catalog.md), the rest of [docs/protocol/](docs/protocol/), the canonical entity definitions under [entities/defs/](entities/defs/), and `crates/services/src/mercury/method_idx.rs` constants |
| Architecture decisions (cell/base split, outbox, state-flag conventions, etc.) | New or amended doc under [docs/architecture/](docs/architecture/) |
| Mission PAK overrides / new client-visible mission steps not in the canonical PAK | [docs/architecture/mission-pak-overrides.md](docs/architecture/mission-pak-overrides.md) and [docs/content/equip-from-inventory-pattern.md](docs/content/equip-from-inventory-pattern.md) (when the new step is part of an equip flow); cross-link from the mission's row in [docs/content/mission-chains.md](docs/content/mission-chains.md) |
| Game systems, content chains, or content-engine actions | [docs/game-systems.md](docs/game-systems.md) and/or [docs/content/](docs/content/), plus [.github/instructions/content-chains.instructions.md](.github/instructions/content-chains.instructions.md) if review rules shift |
| Project status, gap analysis, or roadmap | [docs/project-status.md](docs/project-status.md), [docs/gap-analysis.md](docs/gap-analysis.md), [docs/architecture/migration-roadmap.md](docs/architecture/migration-roadmap.md) |

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
- **Foresight rule.** When creating a new file you can already see will accumulate siblings (handler-per-message, method-per-feature), start it as a directory from day one. Heuristic: *if you can name 3+ logical sibling files now, make the directory now.* See `crates/services/src/base/world_entry_methods/vendor/` for the canonical example.
- **Naming.** Avoid `helpers.rs`, `utils.rs`, `misc.rs`, `extra.rs` — they hide content. Use `cooldowns.rs`, `damage_resolution.rs`, `witness_list.rs`.
- **Module style.** The repo uses `foo/mod.rs` (not the modern `foo.rs` + `foo/` style). Stay consistent.

