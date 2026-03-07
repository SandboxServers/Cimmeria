# SGW Game Launcher — Design Decisions

Supplements [`docs/client/sgw-launcher.md`](../client/sgw-launcher.md) with architecture decisions made during brainstorming.

---

## Scope

**In scope:**
- Download client RAR from archive.org with progress
- Extract RAR → CABs → game files via bundled 7za.exe
- Patch SGW.exe `.rdata` section with configurable server address
- Config file for install path, server address, patch server URL
- Play button (spawn SGW.exe)
- Delta patch system: static manifest + ZIP files from configurable server
- Launcher self-update via tauri-plugin-updater

**Out of scope:**
- Account creation (use auth server web UI or test/test)
- Cross-platform (game client is Windows-only)
- Admin panel integration

---

## Architecture Decisions

### Standalone Tauri 2 app

Lives at `tools/SGWLauncher/` with its own Cargo workspace. Not a member of the server workspace. Zero dependency on `cimmeria-*` crates. Can be built and distributed independently.

### Windows-only

The game client is `SGW.exe` — a Windows binary. No reason to build cross-platform extraction or launch logic.

### Vanilla HTML/CSS/JS frontend

No framework, no build step. The UI is ~6 states (not installed, downloading, extracting, ready, updating, error). A framework adds overhead without benefit.

### Bundled 7za.exe sidecar

Tauri sidecar for RAR and CAB extraction. ~1MB. Handles both formats without native Rust crate complexity.

### Configurable server address

Editable text field, defaults to a canonical public Cimmeria address. Persisted in `config.json`. Changing it re-patches SGW.exe.

### Delta patch system

Static file host serving `manifest.json` + ZIP archives. Launcher fetches manifest, compares SHA-256 hashes of local files, downloads only changed files. Patch server URL is configurable (can point at S3, GitHub Releases, nginx, etc.).

### Launcher self-update

Uses `tauri-plugin-updater` — checks a JSON endpoint for new versions, downloads, applies on restart. No custom implementation needed.

---

## Directory Layout

```
tools/SGWLauncher/
├── src-tauri/
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── binaries/
│   │   └── 7za-x86_64-pc-windows-msvc.exe
│   └── src/
│       ├── main.rs
│       ├── commands.rs
│       ├── download.rs
│       ├── extract.rs
│       ├── patch.rs
│       ├── launch.rs
│       ├── config.rs
│       └── updater.rs
├── ui/
│   ├── index.html
│   ├── main.js
│   └── styles.css
└── README.md
```

---

## Rust Crates

```toml
[dependencies]
tauri = { version = "2", features = ["devtools"] }
tauri-plugin-updater = "2"
tauri-plugin-dialog = "2"
tauri-plugin-shell = "2"
reqwest = { version = "0.12", features = ["stream"] }
sha2 = "0.10"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
tracing = "0.1"
tracing-subscriber = "0.3"
```

---

## Tauri IPC Commands

```rust
// Installation
check_installation(path: String) -> InstallState
get_default_install_path() -> String
browse_for_folder() -> Option<String>
download_and_install(path: String, server: String, window: Window)
cancel_install()

// Patching
patch_server_address(path: String, address: String) -> Result<()>

// Launch
launch_game(path: String) -> Result<()>

// Updates
check_for_updates(path: String, patch_url: String) -> UpdateCheckResult
apply_updates(path: String, patch_url: String, window: Window)

// Config
load_config() -> LauncherConfig
save_config(config: LauncherConfig) -> Result<()>
```

---

## Events (Rust → Frontend)

```
download-progress   { downloaded, total, percent, speed_bps, eta_secs }
extract-progress    { phase: "rar"|"cab", current, total, filename }
update-progress     { current, total, filename, bytes }
install-complete    {}
update-complete     {}
install-error       { message }
```

---

## UI States

| State | Condition | Controls |
|-------|-----------|----------|
| Not installed | SGW.exe absent | Path picker, Install button |
| Downloading | Download in progress | Progress bar (bytes + speed + ETA), cancel |
| Extracting | RAR/CAB in progress | Progress bar (file count + name) |
| Ready | SGW.exe present, URL patched | PLAY button, server address, check updates |
| Updating | Delta patch in progress | Progress bar (file count) |
| Error | Any failure | Error message, retry |

Window: 600x400, not resizable. Dark theme (`#1a1a2e` palette).

---

## Config File

```json
{
  "install_path": "C:\\Games\\Stargate Worlds",
  "server_address": "play.cimmeria.gg",
  "patch_server_url": "https://patches.cimmeria.gg",
  "last_patch_check": "2026-03-06T12:00:00Z"
}
```

---

## Error Handling

- **Network errors:** Show error + retry. Resume downloads via HTTP Range header.
- **Extraction errors:** Map 7za exit codes to messages. Clean up partial extraction, retry from extraction step.
- **Patch errors:** If "www.stargateworlds.com" not found, warn but allow proceeding (may already be patched). Reject server addresses > 22 bytes in UI.
- **Update errors:** Re-download on hash mismatch, skip after 2 failures.
- **Offline:** Launcher works fully offline if game is installed. Update checks fail gracefully.
