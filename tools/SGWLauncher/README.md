# SGW Launcher

Player-facing game launcher for Stargate Worlds. Downloads, installs, patches, and launches the game client.

## Prerequisites

- **Rust toolchain** (`rustup`, `cargo`)
- **Windows 10/11** with WebView2 (ships by default)
- **7za.exe** placed at `src-tauri/binaries/7za-x86_64-pc-windows-msvc.exe` (see `src-tauri/binaries/README.md`)

## Development

```bash
cd tools/SGWLauncher/src-tauri
cargo build           # Build Rust backend
cargo tauri dev       # Run with hot-reload (opens window)
```

No frontend build step — `ui/` is served directly as static files.

## Release Build

```bash
cargo tauri build --target x86_64-pc-windows-msvc
```

Output: `src-tauri/target/release/bundle/` contains the installer.

## Architecture

Standalone Tauri 2 app. Not part of the Cimmeria server workspace.

| Module | Purpose |
|--------|---------|
| `config.rs` | Persistent JSON config (install path, server address, patch URL) |
| `download.rs` | HTTP streaming download with progress and resume |
| `extract.rs` | 7za sidecar wrapper for RAR/CAB extraction |
| `patch.rs` | SGW.exe binary patching (server hostname) |
| `launch.rs` | Game process spawning |
| `updater.rs` | Delta patch system (manifest + SHA-256 verification) |
| `commands.rs` | Tauri IPC command handlers |
| `ui/` | Vanilla HTML/CSS/JS frontend |

## Config

Stored in `%APPDATA%/com.cimmeria.sgw-launcher/config.json`:

```json
{
  "install_path": "C:\\Games\\Stargate Worlds",
  "server_address": "play.cimmeria.gg",
  "patch_server_url": "https://patches.cimmeria.gg",
  "last_patch_check": "2026-03-06T12:00:00Z"
}
```
