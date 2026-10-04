# Pre-PR Checks

> **Last updated**: 2026-10-03
> **Audience**: Contributors and agents about to open a PR, or looking at a red CI job
> **Type**: Reference
> **Companions**: [CLAUDE.md](../../CLAUDE.md) (the short runnable checklist), [doc-update-map.md](doc-update-map.md), [TESTING.md](../../TESTING.md), [build-system.md](../architecture/build-system.md), [troubleshooting.md](../troubleshooting.md#tests--ci)

What each CI check is worth, the full annotated pre-PR command list, and what to do when a check fails. The short version that loads every session is the "Pre-PR checklist" in [CLAUDE.md](../../CLAUDE.md); this page is the long form behind it.

## CI checks

Not every check blocks a merge. Here's what each one is actually worth:

| Check | Workflow | Blocking? | Runs when |
|---|---|---|---|
| `fmt`, `clippy`, `build-and-test`, `test-live-db` | [test.yml](../../.github/workflows/test.yml) | **Yes** — four gating jobs | Every PR that changes more than Markdown, `.claude/` or images under `docs/` (those skip and report as passed; `docs/protocol/` always runs) |
| `figure-sources-in-sync` | [figure-sources.yml](../../.github/workflows/figure-sources.yml) | **Yes** | Only when `docs/drafts/spec/figures/**` changes |
| `figure-style-lint` | [figure-style.yml](../../.github/workflows/figure-style.yml) | **Yes** | Only when figures or `docs/drafts/spec/**.md` change |
| `markdownlint` | [markdownlint.yml](../../.github/workflows/markdownlint.yml) | No — warn-only annotations | Any `**/*.md` change |
| `spec-lint` | [spec-lint.yml](../../.github/workflows/spec-lint.yml) | No — warn-only annotations | `docs/spec/**`, `crates/**`, manifest changes |

Two coverage jobs in `test.yml`, `coverage-workspace` and `coverage-live-db`, run at the same time, upload to Codecov (which merges them) and do not gate. Only runs on `main` save the Rust caches, and PRs restore them (see [build-system.md §12](../architecture/build-system.md#12-ci-caches-are-saved-from-main-only)). After a merge, [regen-docs.yml](../../.github/workflows/regen-docs.yml) reruns `tools/docs-gen/regen.py` on `main` and commits any generated doc blocks that changed; nothing in a PR checks them. The test runner in CI is [`cargo-nextest`](https://nexte.st/); install it once with `cargo install cargo-nextest --locked` (or `taiki-e/install-action@nextest` if you already use that pattern).

## Desktop launcher workspace

Changes under `crates/launcher/desktop/` also run the native Mac/Windows workflow
`.github/workflows/launcher-desktop.yml`. It checks the standalone Rust engine,
Effect frontend, native Tauri shell, and JS logic UAT against the state harness. Root
workspace checks do not cover this directory; follow the
[desktop README](../../crates/launcher/desktop/README.md) commands as well.
Passing these jobs is not packaged UI, signing or game UAT.

## The full checklist

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
# with live-DB tests in one nextest run; the script clones the database into
# one copy per live-DB slot first (the crate list lives in the script;
# tools/test-live-db.ps1 on Windows PowerShell):
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

## Markdown lint

The markdown lint runs via [`markdownlint-cli2`](https://github.com/DavidAnson/markdownlint-cli2) against [`.markdownlint-cli2.yaml`](../../.markdownlint-cli2.yaml) at the repo root. CI mirrors local invocation via [`DavidAnson/markdownlint-cli2-action`](../../.github/workflows/markdownlint.yml). First local run downloads the binary on-demand via `npx`; running `npm install` once pins the version from `package.json` for offline reuse. Phase 2 hardens the lint from warn-only to blocking — until then, fix what's easy and let reviewers nudge the rest.

## When a check fails

- **fmt fails** → `cargo fmt --all` and commit the result. The CI job tells you exactly that.
- **clippy fails** → fix the warning. Project-level thresholds for `too_many_arguments` (14) and `type_complexity` (500) live in `clippy.toml`; bumping those further requires the same kind of justification any other lint suppression would. Don't sprinkle `#[allow(clippy::…)]` per call site. **Passes locally but fails in CI?** That is no longer toolchain drift: [rust-toolchain.toml](../../rust-toolchain.toml) pins the version CI uses, so the old advice to install CI's newer clippy side by side is obsolete. Check that you ran the exact command in [the full checklist](#the-full-checklist) (`--workspace --all-targets -- -D warnings`), not a narrower `-p` run. Bumping Rust is its own PR: change `rust-toolchain.toml`, run the checklist, and fix the new lints there.
- **build fails** → typically a stale path or unused-symbol cleanup needed; check matches `cargo check`. If the failing step is **workspace-hack is current**, a dependency change didn't regenerate the hakari crate: run `cargo hakari generate && cargo hakari manage-deps --yes` (install it once with `cargo install cargo-hakari --locked`) and commit the result. See [docs/architecture/build-system.md](../architecture/build-system.md) §6.
- **test fails (no DB)** → unit + non-DB integration tests. Live-DB tests self-skip via `require_db_or_skip!` when `DATABASE_URL` is unset, so this run can be green even with broken DB code.
- **test-live-db fails** → CI runs `tools/test-live-db.sh` (`cargo nextest run --profile=ci-live-db --lib` over every crate in its list) against a fresh `postgres:17.9` service container loaded from `db/database.sql`; the load runs in the background while the test binaries compile (`tools/live-db-schema-load.sh`, `tools/test-live-db.sh --build-only`), and a failed load fails the job with psql's log. A crate with a `cimmeria-test-support` dev-dependency must be in that list, or `live_db_wrapper_lists_every_test_support_crate` fails. The script clones the loaded database into one database per slot of the `live-db` nextest test group (`sgw_0` .. `sgw_<N-1>`, N being the group's `max-threads` in `.config/nextest.toml`), and the `ci-live-db` profile runs the tests whose name contains `live_db` up to N at a time, each on its own clone, because some live-DB tests share sentinel id ranges and would collide in one database; every other test runs in parallel. A live-DB test without `live_db` in its fn or module name fails `every_live_db_test_is_in_the_live_db_group`, and test code that reads `DATABASE_URL` itself instead of `test_support::database_url()` fails `database_url_is_only_resolved_by_the_gate` (both in `cimmeria-test-support`, so they fail the no-DB `build-and-test` job too). A test that passes alone but fails here often depended on rows an earlier test left in the shared database: give it its own fixtures. To repro locally, start the bundled Postgres on `:5433` and run the command in the snippet above.
- **figure-sources-in-sync fails** → A source DSL under `docs/drafts/spec/figures/sources/` was committed more recently than its rendered SVG one directory up. Re-render the affected diagram (Prixmaviz, or the local renderer per [docs/drafts/spec/figures/sources/README.md](../drafts/spec/figures/sources/README.md)) and commit the regenerated SVG alongside the source change. Pairing rule: `sources/<slug>.<ext>` pairs with `<slug>.svg`.
- **figure-style-lint fails** → A figure source, rendered SVG, or chapter convention violated the style rule catalog inside [tools/lint-figure-style.sh](../../tools/lint-figure-style.sh). Common causes: Mermaid `flowchart`/`sequenceDiagram` missing the `htmlLabels:false` init directive (rules M1/M2), an SVG missing the cimmeria-bg theme-aware backdrop marker (S1), Graphviz intrinsic `fill="white"` backdrop polygon not stripped (S3), non-sequential `*Figure N:*` captions (C1), generic image alt text (C2), or a dangling image reference (C3). Run the script locally to see the specific rule code and remediation hint.
