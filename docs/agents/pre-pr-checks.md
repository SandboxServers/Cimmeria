# Pre-PR Checks

> **Last updated**: 2026-10-23
> **Audience**: Contributors and agents about to open a PR, or looking at a red CI job
> **Type**: Reference
> **Companions**: [CLAUDE.md](../../CLAUDE.md) (the core that points here), [doc-update-map.md](doc-update-map.md), [TESTING.md](../../TESTING.md), [build-system.md](../architecture/build-system.md), [troubleshooting.md](../troubleshooting.md#tests--ci)

What each CI check is worth, the full annotated pre-PR command list, and what to do when a check fails. This page is the checklist: [CLAUDE.md](../../CLAUDE.md) only points here, and the `lane-build` skill runs the compiling lines.

## CI checks

Not every check blocks a merge. Here's what each one is actually worth:

| Check | Workflow | Blocking? | Runs when |
|---|---|---|---|
| `fmt`, `clippy`, `build-and-test` (no-DB and live-DB tiers in one job, so the test binaries compile once; it ends with three `lab-chaos` feature steps, a clippy run and a nextest run over base-session, base, base-world-entry and lab-mcp and a `cargo check` of the server, because no other build turns that feature on) | [test.yml](../../.github/workflows/test.yml) | **Yes** — through `ci gate`, the check main's ruleset requires ([Merge queue](#merge-queue)) | `fmt` and `clippy` on every PR push; `build-and-test` in the merge queue, nightly, and on a PR labelled `full-ci` (on other PR pushes it reports skipped, which counts as passing). All three skip and report as passed when the change is only Markdown, `.claude/` or images under `docs/`; `docs/protocol/` always runs |
| `figure-sources-in-sync` | [figure-sources.yml](../../.github/workflows/figure-sources.yml) | **Yes** | Only when `docs/drafts/spec/figures/**` changes |
| `figure-style-lint` | [figure-style.yml](../../.github/workflows/figure-style.yml) | **Yes** | Only when figures or `docs/drafts/spec/**.md` change |
| `cargo clippy -D warnings (cimmeria-lab)`, `cargo nextest (cimmeria-lab, UAT specs)`, `lab CLI PowerShell tests` | [lab.yml](../../.github/workflows/lab.yml) | **Yes** when they run, enforced by `ship.py merge` (not branch protection: a manual `gh pr merge` ignores them) | `crates/lab/**`, `crates/client-launch/**`, `docs/guides/uat-specs/**`, `tools/lab/**`, `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`. The lab supervisor is excluded from the workspace jobs, so this is its only build-and-test |
| `build-lane PowerShell tests` | [build-lane-scripts.yml](../../.github/workflows/build-lane-scripts.yml) | **Yes** when it runs, enforced by `ship.py merge` | `tools/build-lane/**` (the Python tests there run in `test.yml`) |
| `build-lane shell scripts (Linux)` | [build-lane-scripts.yml](../../.github/workflows/build-lane-scripts.yml) | **Yes** when it runs, enforced by `ship.py merge` | `tools/build-lane/**`: `bash -n`, shellcheck and the build-lane Python tests on Linux |
| `build + smoke-test (no push)` | [pr-container.yml](../../.github/workflows/pr-container.yml) | No on a PR. On `main`, a failure opens a `main-red` issue that blocks code merges ([Main is red](#main-is-red)) | On a PR, only container files: `docker/**`, `Cargo.lock`, `.trivyignore`, the two container workflows, `tools/container-smoke.sh`. On push to `main`, every image input (crates, `db/`, `entities/defs/`, cooked data, `.cargo/`, …), coalesced to the newest pending commit |
| `wireclient e2e (live DB)` | [wireclient.yml](../../.github/workflows/wireclient.yml) | No, reports only, until WC-16 | Any change under `crates/**` except the launcher, lab, client-hookgate, client-launch, client-patches, client-telemetry, sgw-testhost and start32 crates; `db/**`, `entities/**`, `data/spaces/**`, `data/cache/**` (the cooked resource paks the server loads); `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `.config/nextest.toml`, `.cargo/config.toml`, `.github/actions/rust-toolchain/**`; `tools/test-live-db.sh`, `tools/live-db-schema-load.sh`; or the workflow file itself |
| `bootstrap artifacts`, `database refresh tests (PostgreSQL 17)` | [bootstrap-lint.yml](../../.github/workflows/bootstrap-lint.yml) | Not in `main`'s ruleset | `.mcp.json.example`, `bootstrap/**`, `setup.ps1`, `db/persistent/**`. `database-refresh` runs `bootstrap/tests/test-database-refresh.ps1` against a `postgres:17` service with `CI_REQUIRE_PG=1`, so a missing server fails the job instead of skipping the test |
| `markdownlint` | [markdownlint.yml](../../.github/workflows/markdownlint.yml) | No — warn-only annotations | Any `**/*.md` change |
| `spec-lint` | [spec-lint.yml](../../.github/workflows/spec-lint.yml) | No — warn-only annotations | `docs/spec/**`, `crates/**`, manifest changes |
| `senpai review` | [senpai-review.yml](../../.github/workflows/senpai-review.yml) | No — opt-in, and not required by `main`'s ruleset. It fails (red) when a requested run posts no review, for example a token that won't mint; skipped runs report skipped | Only when someone adds the `senpai` label to a same-repo PR, or runs `gh workflow run senpai-review.yml -f pr_number=<N>`. It posts one in-character Claude review comment as the Senpai GitHub App, then removes the label, so adding it again asks for a fresh review. Any other label event reports it skipped. Each run spends Anthropic API money (Sonnet, at most 25 turns, about $1.50), so ask for it on purpose. GitHub-hosted `ubuntu-latest` |

The `test.yml`, `wireclient.yml` and `pr-container.yml` jobs run on the self-hosted `cimmeria-linux` runners (five build runners and two quick-job runners on each of node5, node2 and node1), except fork PRs and Dependabot PRs, which get GitHub-hosted `ubuntu-latest`. When fewer than `CI_OVERFLOW_MIN_IDLE` build runners are idle (0 since 2026-10-11, which turns overflow off), the `changes` job overflows that run's build jobs to GitHub-hosted runners; the repository variable `CI_HOSTED_JOBS` pins named jobs there. Setting the repository variable `CI_LINUX_RUNNER=hosted` sends them all back to GitHub-hosted. Those runners use sccache instead of the rust-cache step. The launcher workflows' Windows jobs (`launcher-desktop.yml`, `client-launch.yml`, `launcher-runtime-probe.yml`) run on three self-hosted `cimmeria-windows` runners on node3 the same way (the check names keep `windows-latest`); `CI_WINDOWS_RUNNER=hosted` sends them back to `windows-latest`, and `launcher-release.yml` always builds GitHub-hosted. See [tools/ci-runners/README.md](../../tools/ci-runners/README.md).

Two coverage jobs in `test.yml`, `coverage-workspace` and `coverage-live-db`, run at the same time in the nightly run on `main` (not per merge or per PR: each rebuilds the workspace instrumented), upload to Codecov (which merges them) and do not gate. A PR that wants its own coverage report gets the `coverage` label, then a push. Only runs on `main` save the Rust caches, and PRs and merge groups restore them (see [build-system.md §12](../architecture/build-system.md#12-ci-caches-are-saved-from-main-only)). Every six hours, [regen-docs.yml](../../.github/workflows/regen-docs.yml) reruns `tools/docs-gen/regen.py` on `main` and commits any generated doc blocks that changed, so they can lag `main` by up to six hours (`gh workflow run regen-docs.yml` runs it now); nothing in a PR checks them. The test runner in CI is [`cargo-nextest`](https://nexte.st/); install it once with `cargo install cargo-nextest --locked` (or `taiki-e/install-action@nextest` if you already use that pattern).

The Windows lab clippy and nextest jobs hydrate `external/recast` before building.
Lab tests that depend on the Rust minigame crate compile Detour through
`cimmeria-entity`; a checkout without that source fails before Rust tests run.

## Merge queue

`main` is protected by a repository ruleset. Here's what it enforces:

- **A merge queue.** PRs land by squash, in groups of up to 5, with up to 5 groups building at once. Only the head of a group has to pass: its run builds the merged result of every PR in the group.
- **One required check, `ci gate`.** It is `test.yml`'s aggregate of `cargo fmt --check`, `cargo clippy -D warnings` and `cargo build + nextest (workspace + live DB)`: it always runs, and passes only when the `changes` job succeeded and each of the three succeeded or was skipped by design. Requiring the three directly would not be safe: a job whose dependency failed is skipped, and a skipped required check passes, so a failed `changes` job would have let a group land untested. In the queue it runs on the merged result, which is the code that actually lands.
- **No direct pushes to `main`.** Deploy keys may bypass the merge-queue ruleset, and the only one is `regen-docs`' write key (secret `REGEN_DOCS_DEPLOY_KEY`; a repository ruleset can't name the GitHub Actions app). Adding another write deploy key would give it the same bypass. Repository admins may bypass it, through a PR, for emergencies only. A second ruleset forbids force-pushing or deleting `main`, with no bypass at all.
- **Head branches are deleted on merge**, which makes GitHub retarget PRs stacked on a merged branch to `main`.

What that means for you:

- **A PR push runs fmt, clippy and the quick jobs.** The full build + nextest + live-DB job runs in the queue, so run nextest locally through the lane before you push ([the full checklist](#the-full-checklist)). To run the full job on a PR before queueing it, add the `full-ci` label, then push (a re-run reuses the labels the run started with).
- **Merge with `ship.py merge`.** It adds the PR to the queue with a plain `gh pr merge` and waits up to `--queue-timeout` (default 60m) for it to land, then retires the worktree. Nothing goes around the queue with `--admin`, docs-only PRs included: a merge around the queue makes it rebuild every group in flight. If the queue drops the PR because it now conflicts, `ship.py` rebases and queues it again (three tries); a failed `merge_group` check gives `status=queue-failed` (exit 1), and the PR's checks say which. A timeout gives `status=queued` (exit 3): the PR still merges on its own, so rerun `ship.py merge` later to retire the worktree. `--no-wait` queues and returns `status=queued` (exit 0).
- **Stacked PRs.** `ship.py pr --base <branch>` opens a PR on top of another PR's branch. `ship.py merge` refuses a stacked PR (its base isn't `main`): merge the base first. After a merge, `ship.py` restacks every open PR built on the merged branch onto `main` (`rebase-pr.sh --upstream <merged head> --push`, which replays only the stacked PR's own commits) and retargets it to `main`; the output lists them as `restacked=...`, and any that hit a conflict as `restack_conflict=...`.

## Main is red

Two checks run on `main` after the queue, because they're too expensive per PR: `pr-container.yml` (the container build and smoke, on every merge that touches an image input, coalesced so a burst of merges is checked once at the newest commit) and the nightly `test.yml` run. When either fails on `main`, [`tools/main-red/main-red.sh`](../../tools/main-red/main-red.sh) opens an issue labelled `main-red` (`main is red: container build + smoke` or `main is red: nightly ci`), or comments on the open one. The next passing run closes it.

While any `main-red` issue is open, `ship.py merge` refuses `kind=code` PRs with `status=main-red` (exit 1). Docs-only PRs still merge. To land the fix:

1. Read the failing run linked from the issue and reproduce it locally (the container smoke is [`tools/container-smoke.sh`](../../tools/container-smoke.sh); see [container distribution § CI login smoke](../operations/container.md#ci-login-smoke)).
2. Open the fix as a normal PR.
3. Merge it with `ship.py merge <PR> --allow-red-main`. Use the flag only for the PR that fixes `main`.
4. The next passing run on `main` closes the issue, and merges reopen. For the container issue that's the `pr-container.yml` run your merge starts, if it touched an image input. For the nightly issue it's the next scheduled run, or a manual dispatch on `main` (`gh workflow run test.yml --ref main`), which reports the same way.

## Desktop launcher workspace

Changes under `crates/launcher/desktop/` also run the native Mac/Windows workflow
`.github/workflows/launcher-desktop.yml`. It checks the standalone Rust engine,
Effect frontend, native Tauri shell, and JS logic UAT against the state harness. Root
workspace checks do not cover this directory; follow the
[desktop README](../../crates/launcher/desktop/README.md) commands as well.
The separate `.github/workflows/launcher-runtime-probe.yml` builds and tests the
x86 prerequisite probe on native Windows; it does not wait for the desktop matrix.
Its artifact still needs managed Wine/SDK validation. Passing these jobs is not
packaged UI, signing or game UAT.

## The full checklist

Agents run the compiling commands below through the build lane, with `--exclusive` for the workspace-wide ones: `pwsh tools/build-lane/lane.ps1 --exclusive cargo clippy …` from PowerShell, or `bash tools/build-lane/lane.sh --exclusive cargo clippy …` on Linux and WSL.

```bash
cargo fmt --all -- --check
# The workspace-hack (cargo-hakari) must match the manifests; after a
# dependency change, regenerate with `cargo hakari generate && cargo hakari manage-deps --yes`:
cargo hakari generate --diff && cargo hakari manage-deps --dry-run
cargo clippy --workspace \
  --exclude cimmeria-app --exclude cimmeria-content-editor \
  --exclude cimmeria-scene-editor \
  --exclude cimmeria-client-telemetry --exclude cimmeria-client-patches --exclude cimmeria-lab \
  --all-targets -- -D warnings
cargo build --workspace \
  --exclude cimmeria-app --exclude cimmeria-content-editor \
  --exclude cimmeria-scene-editor \
  --exclude cimmeria-client-telemetry --exclude cimmeria-client-patches --exclude cimmeria-lab --all-targets
cargo nextest run --profile=ci --workspace \
  --exclude cimmeria-app --exclude cimmeria-content-editor \
  --exclude cimmeria-scene-editor \
  --exclude cimmeria-client-telemetry --exclude cimmeria-client-patches --exclude cimmeria-lab
# Doctests aren't run by nextest — only cimmeria-commands has runnable
# doctests today, so this is a one-crate sanity check:
cargo test --doc -p cimmeria-commands

# Lab chaos is compiled only with --features lab-chaos, which the workspace
# runs above never enable. When you touch lab chaos (base-session's
# lab_chaos/, base's lab_chaos_transport, the flush modes, lab-mcp's chaos
# tools) or code they call, run the same three steps as build-and-test:
cargo clippy -p cimmeria-base-session -p cimmeria-base -p cimmeria-base-world-entry \
  -p cimmeria-lab-mcp -p cimmeria-server --features lab-chaos --all-targets -- -D warnings
cargo nextest run --profile=ci -p cimmeria-base-session -p cimmeria-base \
  -p cimmeria-base-world-entry -p cimmeria-lab-mcp --features lab-chaos
cargo check -p cimmeria-server --features lab-chaos

# Live-DB tests — start the bundled Postgres first, then run every crate
# with live-DB tests in one nextest run; the script clones the database into
# one copy per live-DB slot first (the crate list lives in the script;
# tools/test-live-db.ps1 on Windows PowerShell):
DATABASE_URL=postgres://w-testing:w-testing@localhost:5433/sgw \
  tools/test-live-db.sh
# From a worktree, use the worktree's own database instead: this reloads
# sgw_<worktree> from db/database.sql and builds the tier in a heavy slot, then runs it in a db slot.
tools/build-lane/live-db-test.sh <test-name filter>
# (pwsh tools/build-lane/live-db-test.ps1 <filter> from PowerShell)

# Effect NVP generator and weapon-requirement audit (when db/resources/
# seeds or tools/ability_mechanics/ change; CI runs all three in the
# build-and-test job). --check fails when a generated block or the audit doc
# differs from what the tool writes:
python tools/ability_mechanics/effect_nvps_from_desc.py --check
python tools/ability_mechanics/weapon_requirement_audit.py --check
python -m unittest discover -s tools/ability_mechanics -p "test_*.py"

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

## Desktop launcher CI policy checks

When changing launcher workflows, profiles or release scripts, run
`python tools/launcher-release/test_workflow_policy.py` as well as
python tools/launcher-release/test_release_identity.py. The workflow-policy
suite needs PyYAML 6.0.3; CI installs it in its test virtual environment.
The semantic guards check preserved validation coverage, standalone main-only
cache writes, PR cancellation, `release-tools` profile use and signing isolation.
The Linux script harness also runs shellcheck and stubbed release dry runs.
Neither those checks nor package dry runs publish a release.

Native launcher validation still requires Windows and macOS results for the
current commit. Superseded PR runs cancel; main runs complete and save caches.
CI-only dev/test debug overrides do not change local or shipped binaries.
See [build-system.md](../architecture/build-system.md#standalone-desktop-launcher-validation)
and the [release contract](../../crates/launcher/desktop/docs/release.md#build-time-and-signing-isolation).

## Markdown lint

The markdown lint runs via [`markdownlint-cli2`](https://github.com/DavidAnson/markdownlint-cli2) against [`.markdownlint-cli2.yaml`](../../.markdownlint-cli2.yaml) at the repo root. CI mirrors local invocation via [`DavidAnson/markdownlint-cli2-action`](../../.github/workflows/markdownlint.yml). First local run downloads the binary on-demand via `npx`; running `npm install` once pins the version from `package.json` for offline reuse. Phase 2 hardens the lint from warn-only to blocking — until then, fix what's easy and let reviewers nudge the rest.

## When a check fails

- **fmt fails** → `cargo fmt --all` and commit the result. The CI job tells you exactly that.
- **clippy fails** → fix the warning. Project-level thresholds for `too_many_arguments` (14) and `type_complexity` (500) live in `clippy.toml`; bumping those further requires the same kind of justification any other lint suppression would. Don't sprinkle `#[allow(clippy::…)]` per call site. **Passes locally but fails in CI?** That is no longer toolchain drift: [rust-toolchain.toml](../../rust-toolchain.toml) pins the version CI uses, so the old advice to install CI's newer clippy side by side is obsolete. Check that you ran the exact command in [the full checklist](#the-full-checklist) (`--workspace --all-targets -- -D warnings`), not a narrower `-p` run. Bumping Rust is its own PR: change `rust-toolchain.toml`, run the checklist, and fix the new lints there.
- **build fails** → typically a stale path or unused-symbol cleanup needed; check matches `cargo check`. If the failing step is **workspace-hack is current**, a dependency change didn't regenerate the hakari crate: run `cargo hakari generate && cargo hakari manage-deps --yes` (install it once with `cargo install cargo-hakari --locked`) and commit the result. See [docs/architecture/build-system.md](../architecture/build-system.md) §6.
- **build-and-test fails in the no-DB pass** → unit + non-DB integration tests. Live-DB tests self-skip via `require_db_or_skip!` when `DATABASE_URL` is unset, so this run can be green even with broken DB code.
- **build-and-test fails in a `lab-chaos feature` step** → the workspace passes compile without the `lab-chaos` feature, so a change to a signature that lab chaos calls breaks only here. Rerun the three `--features lab-chaos` commands in [the full checklist](#the-full-checklist) through the lane. (A related failure in the ordinary no-DB pass: `lab_chaos_callers_stay_on_the_allowlist`, in `crates/server/tests/lab_chaos_callers.rs`, means new code reaches the chaos registry's `install` or `current` from outside its allowlist. Widening that list is a security-reviewed change; see the [lab-chaos ledger](../analysis/lab-chaos/README.md).)
- **build-and-test fails on `unpaired_id_fields_only_shrink`** → a log event in that file carries an ID field with no name next to it ([instrumentation-discipline.md § Rule 6](../architecture/instrumentation-discipline.md#rule-6--every-id-field-is-paired-with-its-name)). The failure lists the file's unpaired fields as `file:line key`: pair the new one, or, when the ID has no name, put it on its own line marked `// nt:id-only <reason>`. If a count *fell* (you paired fields), or a file moved or split, re-bless with `$env:NT_BASELINE_BLESS = '1'; pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-server unpaired_id` (then `Remove-Item Env:NT_BASELINE_BLESS`), or `NT_BASELINE_BLESS=1 bash tools/build-lane/lane.sh cargo nextest run -p cimmeria-server unpaired_id` on Linux and WSL, and commit `crates/server/src/logging/unpaired_id_baseline.txt`. The bless refuses if the `# total` line would rise. Failures listed as "never baselined" (a marker with no reason or on a line with two ID fields, a renamed `use tracing::… as`, a forwarding wrapper with no local call site) can only be fixed in the code.
- **build-and-test fails in the live-DB pass** → after the no-DB pass, the same job runs `tools/test-live-db.sh` (`cargo nextest run --profile=ci-live-db --lib` over every crate in its list) against a fresh `postgres:17.9` service container loaded from `db/database.sql`; the load runs in the background while the test binaries compile (`tools/live-db-schema-load.sh`, `tools/test-live-db.sh --build-only`), and a failed load fails the job with psql's log. A crate with a `cimmeria-test-support` dev-dependency must be in that list, or `live_db_wrapper_lists_every_test_support_crate` fails. The script clones the loaded database into one database per slot of the `live-db` nextest test group (`sgw_0` .. `sgw_<N-1>`, N being the group's `max-threads` in `.config/nextest.toml`), and the `ci-live-db` profile runs the tests whose name contains `live_db` up to N at a time, each on its own clone, because some live-DB tests share sentinel id ranges and would collide in one database; every other test runs in parallel. A live-DB test without `live_db` in its fn or module name fails `every_live_db_test_is_in_the_live_db_group`, and test code that reads `DATABASE_URL` itself instead of `test_support::database_url()` fails `database_url_is_only_resolved_by_the_gate` (both in `cimmeria-test-support`, so they fail the no-DB pass too). A test that passes alone but fails here often depended on rows an earlier test left in the shared database: give it its own fixtures. To repro locally, start the bundled Postgres on `:5433` and run the command in the snippet above. If the failing step is **ability-mechanics generators** (`test.yml` step), either the effect NVP generator or the weapon-requirement audit failed. For the **effect NVP generator**, a block between `-- ability-mechanics generated <family>` markers in `db/resources/Effects/Seed/effect_nvps.sql` (or a generated `script_name` in `effects.sql`) no longer matches the generator: run `python tools/ability_mechanics/effect_nvps_from_desc.py` and commit both seed files, never a hand edit inside the markers ([tools/ability_mechanics/README.md](../../tools/ability_mechanics/README.md)). For the **weapon-requirement audit** (`weapon_requirement_audit.py --check`), `docs/analysis/class-start-v6/weapon-requirement-audit.md` no longer matches the seeds: a seed changed an ability's `item_monikers`, an item's `moniker_ids` or `items_event_sets`, or a player grant, or someone edited the doc by hand. Run `python tools/ability_mechanics/weapon_requirement_audit.py` and commit the doc; never edit it by hand. Line endings do not count as drift. A PR that changes only that doc still runs the full pipeline (the `changes` filter treats it as code), so the check sees it.
- **figure-sources-in-sync fails** → A source DSL under `docs/drafts/spec/figures/sources/` was committed more recently than its rendered SVG one directory up. Re-render the affected diagram (Prixmaviz, or the local renderer per [docs/drafts/spec/figures/sources/README.md](../drafts/spec/figures/sources/README.md)) and commit the regenerated SVG alongside the source change. Pairing rule: `sources/<slug>.<ext>` pairs with `<slug>.svg`.
- **figure-style-lint fails** → A figure source, rendered SVG, or chapter convention violated the style rule catalog inside [tools/lint-figure-style.sh](../../tools/lint-figure-style.sh). Common causes: Mermaid `flowchart`/`sequenceDiagram` missing the `htmlLabels:false` init directive (rules M1/M2), an SVG missing the cimmeria-bg theme-aware backdrop marker (S1), Graphviz intrinsic `fill="white"` backdrop polygon not stripped (S3), non-sequential `*Figure N:*` captions (C1), generic image alt text (C2), or a dangling image reference (C3). Run the script locally to see the specific rule code and remediation hint.
