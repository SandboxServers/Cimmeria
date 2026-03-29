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
| `upk_parser.py` (31KB) | Parse Unreal Package (.upk) files — extract assets, classes, object lists |
| `kismet_extractor.py` (31KB) | Extract Kismet sequence data from UPK files |
| `extract_actors.py` | Extract actor placements from map packages |
| `pcap_dissect.py` | Dissect captured Mercury UDP network traffic |
| `generate_effect_stubs.py` | Generate Python stub scripts for effects from DB data |
| `frag_debug.py` / `frag_debug2.py` | Debug Mercury packet fragmentation |
| `investigate_corruption.py` | Investigate packet/data corruption patterns |

These scripts run standalone with Python 3.x — they don't need the server running.

## ServerEd (Qt Legacy)

`ServerEd/` — Qt 5.x-based server administration editor. Provides:
- Configuration dialog for service settings
- Database-backed object browser
- Visual node graph editor (for scripting/logic visualization)
- PostgreSQL integration via Qt5Sql

This is a legacy tool that predates the Tauri admin apps. Build with Visual Studio (see `W-NG.sln`).
