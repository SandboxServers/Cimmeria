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

These scripts run standalone with Python 3.x — they don't need the server running.

## Repo Maintenance Scripts

| Script | Purpose |
|---|---|
| `extract_tests.py` | Regenerate the test inventory under [`docs/testing/inventory/`](../docs/testing/inventory/). Walks the workspace `members` from the root `Cargo.toml`, catalogues every `#[test]` / `#[tokio::test]` with the line its `fn` is actually on and whether the body is a live-DB guard, and preserves the hand-curated table columns across regeneration. `--check` and `--verify-links` are drift gates that exit non-zero and write nothing; only `--write` modifies the repo. Stock Python 3, no dependencies. See [maintenance.md](../docs/testing/inventory/maintenance.md). |

## Lint & Check Scripts

Load-bearing scripts run as part of the pre-PR checklist (see [`CLAUDE.md`](../CLAUDE.md)). Each ships in both a POSIX (`.sh`) and PowerShell (`.ps1`) flavor:

| Script | Purpose |
|---|---|
| `lint-md.sh` / `lint-md.ps1` | Markdown lint via `markdownlint-cli2` (warn-only; `--fix` auto-fixes) |
| `check-figure-sources.sh` / `check-figure-sources.ps1` | Verify each figure source DSL has a re-rendered SVG (blocking in CI) |
| `lint-figure-style.sh` / `lint-figure-style.ps1` | Figure style + format lint — Mermaid init directives, theme backdrops, caption numbering (blocking in CI) |

`spec-lint/` is a small Rust crate (`cargo run -p spec-lint`) used for spec-document linting.

## ServerEd (Qt Legacy)

The legacy Qt 5.x server administration editor is **not** under `tools/`. Its Visual Studio solution lives at [`deprecated/cpp-build/W-NG.sln`](../deprecated/cpp-build/W-NG.sln) and predates the Tauri admin apps. It is reference-only — the Tauri apps above are the supported editors.
