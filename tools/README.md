# tools/ — Development Tools

Editor applications and reverse-engineering utilities.

## Tauri Applications

| Tool | Directory | Purpose |
|---|---|---|
| **ContentEditor** | `ContentEditor/` | GUI editor for game content (missions, dialogs, effects) |
| **SceneEditor** | `SceneEditor/` | Visual scene/zone editor |
| **SGWLauncher** | `SGWLauncher/` | Player-facing game launcher — downloads, patches, and launches the client |

These are standalone Tauri 2 apps (Rust + WebView2). They are part of the workspace but excluded from the default `cargo build --workspace` to avoid unnecessary builds.

```bash
# Build a specific tool:
cd tools/ContentEditor
cargo tauri build

# Or from workspace root:
cargo build -p cimmeria-content-editor
```

See each tool's own README for details.

## Python RE Utilities

Scripts for reverse-engineering the game client (UE3/BigWorld binaries).

| Script | Purpose |
|---|---|
| `upk_parser.py` | Parse Unreal Package (.upk) files — extract assets, classes, object lists |
| `kismet_extractor.py` | Extract Kismet sequence data from UPK files |
| `extract_actors.py` | Extract actor placements from map packages |
| `ue3_extract_cover_nodes.py` | Extract UE3 cover-node placements from map packages |
| `pcap_dissect.py` | Dissect captured Mercury UDP network traffic |
| `pcap_to_session.py` | Convert a pcap capture into a wireclient `session_trace` JSONL |
| `wire_decoder_codegen.py` | Generate wire-format decoder code from the message catalog |
| `mercury_dispute_resolver.py` | Reconcile Mercury wire-format disputes against captures |
| `entity_property_sync_resolver.py` | Resolve entity property-sync ordering questions |
| `generate-mercury-kat.py` | Generate Mercury known-answer test (KAT) vectors |
| `generate_effect_stubs.py` | Generate Rust effect stub scripts from DB data |
| `backfill_template_speaker_ids.py` | Backfill dialog template speaker IDs |
| `apply_speaker_id_inplace.py` | Apply resolved speaker IDs to dialog templates in place |
| `add_doc_metadata.py` | Add/normalize front-matter metadata across docs |
| `frag_debug.py` / `frag_debug2.py` | Debug Mercury packet fragmentation |
| `investigate_corruption.py` | Investigate packet/data corruption patterns |
| `re_parity.py` | LLM-free structural parity check for reverse-engineered functions — compares a reconstruction against Ghidra decompile/disasm (11 parity signals + objective call/control-flow gap verifier). Drives the `/re-verify` reverser/checker loop. Run `python tools/re_parity.py --selftest`. |
| `re/ghidra-headless/Probe.java` | Read-only headless-Ghidra probe of `SGW.exe` (decompile, xrefs, string and function-name search, vtable dumps) run through `analyzeHeadless.bat -noanalysis -readOnly`, for when the Ghidra MCP bridge is unavailable. Java, not Python: see [re/ghidra-headless/README.md](re/ghidra-headless/README.md). |

These scripts run standalone with Python 3.x — they don't need the server running.

## Repo Maintenance Scripts

| Script | Purpose |
|---|---|
| `extract_tests.py` | Regenerate the test inventory under [`docs/testing/inventory/`](../docs/testing/inventory/). Walks the workspace `members` from the root `Cargo.toml`, catalogues every `#[test]` / `#[tokio::test]` with the line its `fn` is actually on and whether the body is a live-DB guard, and preserves the hand-curated table columns across regeneration. `--check` and `--verify-links` are drift gates that exit non-zero and write nothing; only `--write` modifies the repo. Stock Python 3, no dependencies. See [maintenance.md](../docs/testing/inventory/maintenance.md). |
| `ability_trees/generate_seed.py` | Validate the canonical ability-tree workbook and regenerate `archetype_ability_tree.sql`, `trainer_abilities.sql` and the committed JSON export. `--check` exits 1 on drift. Needs openpyxl unless run with `--from-json`. See [ability_trees/README.md](ability_trees/README.md). |
| `telemetry-coverage/abilities.py` | The ability telemetry coverage gate (ability-mechanics AB-C7): crosses every ability method in the dispatch tables with the server receipt row, the server send row and the client send and receive hooks, by scanning the code tables each side keeps for it, and writes [telemetry-coverage.md](../docs/analysis/ability-mechanics/telemetry-coverage.md). `--check` exits 1 on drift or on an empty cell with no listed exception; CI runs it with its unit tests. Stock Python 3. |
| `signoz/abilities/` | The ability SigNoz saved views (one cast in order, refusals by reason, wire sends for a player) and the ability-metrics dashboard, as reviewable JSON for import (AB-T7). See [signoz/abilities/README.md](signoz/abilities/README.md). |
| `token-profile/` | Token profiler for AI-assisted work (#957). Wave 0 holds the data contract: the SQLite `schema.sql`, the transcript format and trigger rules, the PR attribution rules, and a synthetic fixture with contract tests. See [token-profile/README.md](token-profile/README.md). |

## Build Tooling

How the Rust workspace is built on developer and agent machines. Why each piece exists, with the measurements, is in [`docs/architecture/build-system.md`](../docs/architecture/build-system.md); how to use them day to day is in [`docs/agents/development-workflow.md`](../docs/agents/development-workflow.md#builds-worktrees-and-test-databases).

| Script | Purpose |
|---|---|
| `build-lane/lane.sh` | The build lane: a machine-wide semaphore every agent `cargo` call goes through (`bash tools/build-lane/lane.sh [--exclusive] cargo …`). Sets the job count, sccache, and a target dir per worktree, and logs every job. |
| `build-lane/lane_stats.py` | Reports on the lane's job log (`--recent`, `--html`, `--csv`). |
| `build-lane/mk-worktree.sh` | Creates a buildable worktree under `.claude/worktrees/`: junctions `external/` in and seeds the target dir on a Dev Drive. |
| `build-lane/rm-worktree.sh` | Retires a worktree once its PR merges (`<name>`, or `--merged` for every merged, idle one): deletes its target dir and `sgw_<worktree>` database, unlinks `external/` without following the junction, and removes the worktree and branch. Refuses while a lane job builds there. Runs `git worktree prune` only with `--prune`. |
| `build-lane/rebase-pr.sh` | Rebases a PR branch onto `origin/main` in a throwaway worktree (a PR number, branch or worktree; `--push` to push). Silent when clean; resolves generated doc blocks and `Cargo.lock`; otherwise aborts with the branch untouched and prints `status=conflict` and the semantic files. |
| `build-lane/ship.sh` | The mechanical end of a packet, one line of output. `pr -C <worktree> -m <msg>` refuses anything but a registered worktree on a feature branch, then commits (with `$SHIP_TRAILERS`), pushes and opens or finds the PR. `merge <PR> [--retire <name>]` merges a Markdown/memory-only PR at once and a code PR once the gating checks pass, rebasing only when GitHub says it is behind or conflicting, then retires the worktree. |
| `build-lane/reload-db.sh` / `build-lane/live-db-test.sh` | Reload the worktree's own test database (`sgw_<worktree>`); run the live-DB tier against it in a lane slot. |
| `dev-drive/New-CimmeriaDevDrive.ps1` | Creates a Windows Dev Drive for build output (run elevated). |
| `dev-drive/Copy-WarmTarget.ps1` | Seeds a new worktree's target dir from a warm one by ReFS block cloning. |
| `build-hygiene/sweep.ps1` | Runs `cargo-sweep` over every target dir on the machine. Don't run it while anything builds. |
| `build-metrics/measure-build.ps1` | Controlled build measurements: cold build, edit loop, `cargo check`, peak memory, target size. |

## Lab Tooling

| Script | Purpose |
|---|---|
| `lab/install.ps1` | Builds the Live Research Lab from a worktree through the build lane (`cimmeria-lab`; the `lab-bridge` telemetry DLL, `sgw-start32` and the patch DLL for i686) and installs them to `%LOCALAPPDATA%\cimmeria-lab\bin\` and the game's `Binaries\`. Refuses while an `SGW.exe` runs and names its supervisor; keeps each replaced file as `<name>.<yyyymmdd>.old`; `-DryRun` changes nothing. See [Install or update the lab](../docs/guides/live-research-lab.md#install-or-update-the-lab). |
| `lab/daemon.ps1` | Runs the shared lab supervisor (`cimmeria-lab --http`) as the per-user scheduled task `CimmeriaLabDaemon`: `install` (copy the exe to `%LOCALAPPDATA%\cimmeria-lab\labd\`, generate `CIMMERIA_LAB_DAEMON_TOKEN`, import `labd.env` from `.mcp.json`, register and start), `start`, `stop`, `restart` (picks up a newer build), `status`, `uninstall`. See [The shared daemon](../docs/guides/live-research-lab.md#the-shared-daemon-cimmeria-lab---http). |
| `lab/labd-lib.ps1`, `lab/test-labd-lib.ps1` | The daemon script's helpers (UTF-8 `labd.env` read and write, the stale-`labd.pid` check by exe path and start time, the recorded bind) and their tests: `pwsh -NoProfile -File tools/lab/test-labd-lib.ps1`, and the same under `powershell` (the task runs Windows PowerShell 5.1). |
| `lab/labd-headers.ps1` | The `headersHelper` of the daemon's `.mcp.json` entry: prints the bearer header from the user environment, so the token never lands in `.mcp.json`. |

## Lint & Check Scripts

Load-bearing scripts run as part of the pre-PR checklist (see [`CLAUDE.md`](../CLAUDE.md)). Each ships in both a POSIX (`.sh`) and PowerShell (`.ps1`) flavor:

| Script | Purpose |
|---|---|
| `lint-md.sh` / `lint-md.ps1` | Markdown lint via `markdownlint-cli2` (warn-only; `--fix` auto-fixes) |
| `check-figure-sources.sh` / `check-figure-sources.ps1` | Verify each figure source DSL has a re-rendered SVG (blocking in CI) |
| `lint-figure-style.sh` / `lint-figure-style.ps1` | Figure style + format lint — Mermaid init directives, theme backdrops, caption numbering (blocking in CI) |
| `test-live-db.sh` / `test-live-db.ps1` | The live-DB test tier: runs the lib tests of every crate in its list in one `--profile=ci-live-db` nextest invocation (blocking in CI; `--llvm-cov` for the coverage job). Needs `DATABASE_URL`. |

`docs-gen/regen.py` (Python 3.11+) owns every generated number and block in the Markdown: test counts, the RE findings count, the gap-analysis totals, and the crate graph. The `regen-docs` workflow runs it on `main` after every merge and commits what changed; PRs don't. See [docs-gen/README.md](docs-gen/README.md). `crate-graph/crate_graph.py` renders the crate dependency graph in `README.md` and `crates/README.md` from `cargo metadata` and still runs on its own; see [crate-graph/README.md](crate-graph/README.md).

`spec-lint/` is a small Rust crate (`cargo run -p spec-lint`) used for spec-document linting.

## ServerEd (Qt Legacy)

The legacy Qt 5.x server administration editor is **not** under `tools/`. Its Visual Studio solution lives at [`deprecated/cpp-build/W-NG.sln`](../deprecated/cpp-build/W-NG.sln) and predates the Tauri admin apps. It is reference-only — the Tauri apps above are the supported editors.
