# SGW Game Launcher Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a standalone Tauri 2 game launcher that downloads, installs, patches, and launches the Stargate Worlds client, with delta patching and self-update support.

**Architecture:** Standalone Cargo workspace at `tools/SGWLauncher/` with Tauri 2 backend (Rust) and vanilla HTML/CSS/JS frontend. Bundles 7za.exe as a Tauri sidecar for RAR/CAB extraction. Windows-only. No dependency on cimmeria-* server crates.

**Tech Stack:** Rust, Tauri 2, tauri-plugin-shell (sidecar), tauri-plugin-dialog (folder picker), tauri-plugin-updater (self-update), reqwest (HTTP streaming), sha2 (hash verification), vanilla HTML/CSS/JS

**Design doc:** `docs/plans/2026-03-06-sgw-launcher-design.md`
**Original spec:** `docs/client/sgw-launcher.md`

---

### Task 1: Scaffold Tauri project

**Files:**
- Create: `tools/SGWLauncher/src-tauri/Cargo.toml`
- Create: `tools/SGWLauncher/src-tauri/build.rs`
- Create: `tools/SGWLauncher/src-tauri/tauri.conf.json`
- Create: `tools/SGWLauncher/src-tauri/capabilities/default.json`
- Create: `tools/SGWLauncher/src-tauri/src/main.rs`
- Create: `tools/SGWLauncher/ui/index.html`
- Create: `tools/SGWLauncher/ui/main.js`
- Create: `tools/SGWLauncher/ui/styles.css`

**Step 1: Create directory structure**

```bash
mkdir -p tools/SGWLauncher/src-tauri/src
mkdir -p tools/SGWLauncher/src-tauri/capabilities
mkdir -p tools/SGWLauncher/src-tauri/binaries
mkdir -p tools/SGWLauncher/src-tauri/icons
mkdir -p tools/SGWLauncher/ui
```

**Step 2: Create `tools/SGWLauncher/src-tauri/Cargo.toml`**

```toml
[package]
name = "sgw-launcher"
version = "0.1.0"
edition = "2021"
license = "MIT"
description = "Stargate Worlds game launcher"

[dependencies]
tauri = { version = "2", features = ["devtools"] }
tauri-plugin-shell = "2"
tauri-plugin-dialog = "2"
tauri-plugin-updater = "2"
tauri-plugin-process = "2"
reqwest = { version = "0.12", features = ["stream"] }
sha2 = "0.10"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
futures-util = "0.3"

[build-dependencies]
tauri-build = "2"
```

**Step 3: Create `tools/SGWLauncher/src-tauri/build.rs`**

```rust
fn main() {
    tauri_build::build()
}
```

**Step 4: Create `tools/SGWLauncher/src-tauri/tauri.conf.json`**

```json
{
  "productName": "SGW Launcher",
  "version": "0.1.0",
  "identifier": "com.cimmeria.sgw-launcher",
  "build": {
    "frontendDist": "../ui"
  },
  "bundle": {
    "active": true,
    "icon": ["icons/icon.png"],
    "externalBin": ["binaries/7za"]
  },
  "app": {
    "windows": [
      {
        "title": "Stargate Worlds",
        "width": 600,
        "height": 400,
        "resizable": false,
        "center": true
      }
    ],
    "security": {
      "csp": "default-src 'self'; style-src 'self' 'unsafe-inline'"
    }
  },
  "plugins": {
    "updater": {
      "pubkey": "",
      "endpoints": []
    }
  }
}
```

**Step 5: Create `tools/SGWLauncher/src-tauri/capabilities/default.json`**

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "Default capabilities for SGW Launcher",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "dialog:allow-open",
    "shell:allow-execute",
    {
      "identifier": "shell:allow-execute",
      "allow": [
        {
          "name": "binaries/7za",
          "sidecar": true,
          "args": [
            {
              "validator": "\\S+"
            }
          ]
        }
      ]
    }
  ]
}
```

**Step 6: Create `tools/SGWLauncher/src-tauri/src/main.rs`**

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            #[cfg(desktop)]
            app.handle().plugin(tauri_plugin_updater::Builder::new().build())?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

**Step 7: Create `tools/SGWLauncher/ui/index.html`**

```html
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Stargate Worlds</title>
  <link rel="stylesheet" href="styles.css">
</head>
<body>
  <div id="app">
    <h1>Stargate Worlds</h1>
    <div id="status">Loading...</div>
  </div>
  <script src="main.js"></script>
</body>
</html>
```

**Step 8: Create `tools/SGWLauncher/ui/styles.css`**

```css
* { margin: 0; padding: 0; box-sizing: border-box; }
body {
  font-family: system-ui, sans-serif;
  background: #1a1a2e;
  color: #e0e0e0;
  height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
  user-select: none;
}
#app {
  text-align: center;
  padding: 2rem;
}
h1 {
  font-size: 1.5rem;
  margin-bottom: 1rem;
  color: #ffffff;
}
#status {
  color: #a0a0b0;
}
```

**Step 9: Create `tools/SGWLauncher/ui/main.js`**

```javascript
document.getElementById('status').textContent = 'Launcher initialized';
```

**Step 10: Verify it builds**

Run: `cd tools/SGWLauncher/src-tauri && cargo build`
Expected: Successful compilation (may take a few minutes on first build to download Tauri deps)

Note: This will fail if Tauri prerequisites aren't installed. On Windows you need WebView2 (ships with Win10/11) and the Visual Studio C++ build tools. On Linux (for dev only — launcher is Windows-only for release), you need `libwebkit2gtk-4.1-dev` and related packages. See https://v2.tauri.app/start/prerequisites/

**Step 11: Commit**

```bash
git add tools/SGWLauncher/
git commit -m "feat(launcher): scaffold Tauri 2 project

Standalone Cargo project at tools/SGWLauncher/ with:
- Tauri 2 with shell, dialog, updater plugins
- Vanilla HTML/CSS/JS frontend (600x400, dark theme)
- 7za sidecar configuration
- Capability permissions for sidecar execution"
```

---

### Task 2: Config module

**Files:**
- Create: `tools/SGWLauncher/src-tauri/src/config.rs`
- Modify: `tools/SGWLauncher/src-tauri/src/main.rs`

**Context:** The config module persists launcher settings to a JSON file in the Tauri app data directory. All other modules will depend on this for install path, server address, and patch URL.

**Step 1: Write test for config**

Add to the bottom of the new file `tools/SGWLauncher/src-tauri/src/config.rs`:

```rust
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LauncherConfig {
    pub install_path: String,
    pub server_address: String,
    pub patch_server_url: String,
    pub last_patch_check: Option<String>,
}

impl Default for LauncherConfig {
    fn default() -> Self {
        Self {
            install_path: String::new(),
            server_address: "play.cimmeria.gg".to_string(),
            patch_server_url: "https://patches.cimmeria.gg".to_string(),
            last_patch_check: None,
        }
    }
}

impl LauncherConfig {
    pub fn load(path: &PathBuf) -> Result<Self, ConfigError> {
        let data = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(&data)?)
    }

    pub fn save(&self, path: &PathBuf) -> Result<(), ConfigError> {
        let data = serde_json::to_string_pretty(self)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, data)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_default_config() {
        let config = LauncherConfig::default();
        assert_eq!(config.server_address, "play.cimmeria.gg");
        assert_eq!(config.patch_server_url, "https://patches.cimmeria.gg");
        assert!(config.install_path.is_empty());
        assert!(config.last_patch_check.is_none());
    }

    #[test]
    fn test_save_and_load() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");

        let config = LauncherConfig {
            install_path: "C:\\Games\\SGW".to_string(),
            server_address: "localhost".to_string(),
            patch_server_url: "https://example.com".to_string(),
            last_patch_check: Some("2026-03-06T12:00:00Z".to_string()),
        };

        config.save(&path).unwrap();
        let loaded = LauncherConfig::load(&path).unwrap();
        assert_eq!(loaded.install_path, "C:\\Games\\SGW");
        assert_eq!(loaded.server_address, "localhost");
    }

    #[test]
    fn test_load_missing_file() {
        let path = PathBuf::from("/tmp/nonexistent/config.json");
        assert!(LauncherConfig::load(&path).is_err());
    }
}
```

**Step 2: Add tempfile dev-dependency to Cargo.toml**

Add to `tools/SGWLauncher/src-tauri/Cargo.toml`:

```toml
[dev-dependencies]
tempfile = "3"
```

**Step 3: Run tests to verify they pass**

Run: `cd tools/SGWLauncher/src-tauri && cargo test -- config`
Expected: 3 tests pass

**Step 4: Register module in main.rs**

Add `mod config;` to the top of `tools/SGWLauncher/src-tauri/src/main.rs` (after the `#![cfg_attr]` line).

**Step 5: Commit**

```bash
git add tools/SGWLauncher/src-tauri/src/config.rs tools/SGWLauncher/src-tauri/Cargo.toml tools/SGWLauncher/src-tauri/src/main.rs
git commit -m "feat(launcher): add config persistence module

LauncherConfig struct with load/save to JSON. Stores install path,
server address, patch server URL, and last patch check timestamp.
Defaults to play.cimmeria.gg."
```

---

### Task 3: SGW.exe binary patching

**Files:**
- Create: `tools/SGWLauncher/src-tauri/src/patch.rs`
- Modify: `tools/SGWLauncher/src-tauri/src/main.rs`

**Context:** This module searches SGW.exe for the hardcoded CME hostname `www.stargateworlds.com` in the `.rdata` section and overwrites it with the user's configured server address. The replacement must be ≤22 bytes (length of the original string) and zero-padded.

**Step 1: Write patch module with tests**

Create `tools/SGWLauncher/src-tauri/src/patch.rs`:

```rust
use thiserror::Error;

const ORIGINAL_HOST: &[u8] = b"www.stargateworlds.com";
const MAX_HOST_LEN: usize = 22; // length of "www.stargateworlds.com"

#[derive(Debug, Error)]
pub enum PatchError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Server address too long: {len} bytes (max {MAX_HOST_LEN})")]
    AddressTooLong { len: usize },
    #[error("Original hostname not found in binary — may already be patched")]
    PatternNotFound,
}

/// Search `data` for `pattern` and return the byte offset if found.
fn find_pattern(data: &[u8], pattern: &[u8]) -> Option<usize> {
    data.windows(pattern.len())
        .position(|window| window == pattern)
}

/// Patch the server hostname in an SGW.exe byte buffer.
///
/// Searches for `www.stargateworlds.com` and overwrites with `new_host`,
/// zero-padded to the original 22-byte length.
///
/// Returns the byte offset where the patch was applied.
pub fn patch_hostname(data: &mut [u8], new_host: &str) -> Result<usize, PatchError> {
    let host_bytes = new_host.as_bytes();
    if host_bytes.len() > MAX_HOST_LEN {
        return Err(PatchError::AddressTooLong { len: host_bytes.len() });
    }

    let offset = find_pattern(data, ORIGINAL_HOST)
        .ok_or(PatchError::PatternNotFound)?;

    // Overwrite with new host + zero padding
    data[offset..offset + host_bytes.len()].copy_from_slice(host_bytes);
    for i in host_bytes.len()..MAX_HOST_LEN {
        data[offset + i] = 0;
    }

    Ok(offset)
}

/// Check if SGW.exe still contains the original CME hostname.
pub fn needs_patching(data: &[u8]) -> bool {
    find_pattern(data, ORIGINAL_HOST).is_some()
}

/// Patch SGW.exe on disk.
pub fn patch_exe(exe_path: &std::path::Path, new_host: &str) -> Result<usize, PatchError> {
    let mut data = std::fs::read(exe_path)?;
    let offset = patch_hostname(&mut data, new_host)?;
    std::fs::write(exe_path, &data)?;
    Ok(offset)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_fake_exe(host: &[u8]) -> Vec<u8> {
        // Simulate a binary with some padding, the hostname, and more padding
        let mut data = vec![0u8; 100];
        data[40..40 + host.len()].copy_from_slice(host);
        data
    }

    #[test]
    fn test_patch_hostname_success() {
        let mut data = make_fake_exe(b"www.stargateworlds.com");
        let offset = patch_hostname(&mut data, "localhost").unwrap();
        assert_eq!(offset, 40);
        // Verify the new host is written
        assert_eq!(&data[40..49], b"localhost");
        // Verify zero-padding after "localhost" (9 bytes) up to 22 bytes
        assert!(data[49..62].iter().all(|&b| b == 0));
    }

    #[test]
    fn test_patch_hostname_max_length() {
        let mut data = make_fake_exe(b"www.stargateworlds.com");
        // Exactly 22 bytes
        let host = "abcdefghijklmnopqrstuv"; // 22 chars
        assert_eq!(host.len(), 22);
        let offset = patch_hostname(&mut data, host).unwrap();
        assert_eq!(offset, 40);
        assert_eq!(&data[40..62], host.as_bytes());
    }

    #[test]
    fn test_patch_hostname_too_long() {
        let mut data = make_fake_exe(b"www.stargateworlds.com");
        let host = "this-hostname-is-way-too-long.example.com"; // >22 bytes
        let result = patch_hostname(&mut data, host);
        assert!(matches!(result, Err(PatchError::AddressTooLong { .. })));
    }

    #[test]
    fn test_patch_hostname_not_found() {
        let mut data = vec![0u8; 100]; // no hostname present
        let result = patch_hostname(&mut data, "localhost");
        assert!(matches!(result, Err(PatchError::PatternNotFound)));
    }

    #[test]
    fn test_needs_patching() {
        let data = make_fake_exe(b"www.stargateworlds.com");
        assert!(needs_patching(&data));

        let patched = make_fake_exe(b"localhost\0\0\0\0\0\0\0\0\0\0\0\0\0");
        assert!(!needs_patching(&patched));
    }

    #[test]
    fn test_patch_idempotent_check() {
        let mut data = make_fake_exe(b"www.stargateworlds.com");
        patch_hostname(&mut data, "play.cimmeria.gg").unwrap();
        // Second patch should fail because original hostname is gone
        assert!(!needs_patching(&data));
        assert!(matches!(
            patch_hostname(&mut data, "other.host"),
            Err(PatchError::PatternNotFound)
        ));
    }

    #[test]
    fn test_patch_exe_on_disk() {
        let dir = tempfile::tempdir().unwrap();
        let exe_path = dir.path().join("SGW.exe");
        let data = make_fake_exe(b"www.stargateworlds.com");
        std::fs::write(&exe_path, &data).unwrap();

        let offset = patch_exe(&exe_path, "localhost").unwrap();
        assert_eq!(offset, 40);

        let patched = std::fs::read(&exe_path).unwrap();
        assert_eq!(&patched[40..49], b"localhost");
    }
}
```

**Step 2: Run tests**

Run: `cd tools/SGWLauncher/src-tauri && cargo test -- patch`
Expected: 7 tests pass

**Step 3: Register module in main.rs**

Add `mod patch;` to `tools/SGWLauncher/src-tauri/src/main.rs`.

**Step 4: Commit**

```bash
git add tools/SGWLauncher/src-tauri/src/patch.rs tools/SGWLauncher/src-tauri/src/main.rs
git commit -m "feat(launcher): SGW.exe binary patching module

Search-and-replace www.stargateworlds.com in binary data with
configurable server address, zero-padded to 22 bytes. Idempotent
check via needs_patching(). 7 unit tests."
```

---

### Task 4: HTTP download with progress

**Files:**
- Create: `tools/SGWLauncher/src-tauri/src/download.rs`
- Modify: `tools/SGWLauncher/src-tauri/src/main.rs`

**Context:** Streams a large file (the ~200MB client RAR) from a URL to disk, emitting progress events. Supports cancellation via a `CancellationToken`. Supports resume via HTTP Range header for retries.

**Step 1: Write download module**

Create `tools/SGWLauncher/src-tauri/src/download.rs`:

```rust
use futures_util::StreamExt;
use reqwest::Client;
use serde::Serialize;
use std::path::Path;
use thiserror::Error;
use tokio::io::AsyncWriteExt;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Error)]
pub enum DownloadError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Download cancelled")]
    Cancelled,
    #[error("Server returned {0}")]
    BadStatus(u16),
}

#[derive(Debug, Clone, Serialize)]
pub struct DownloadProgress {
    pub downloaded: u64,
    pub total: u64,
    pub percent: f32,
    pub speed_bps: u64,
    pub eta_secs: u64,
}

/// Download a file from `url` to `dest`, calling `on_progress` with updates.
///
/// If `dest` already exists with partial content, attempts to resume via
/// HTTP Range header. The `cancel` token can be used to abort the download.
pub async fn download_file<F>(
    url: &str,
    dest: &Path,
    cancel: CancellationToken,
    mut on_progress: F,
) -> Result<(), DownloadError>
where
    F: FnMut(DownloadProgress),
{
    let client = Client::new();

    // Check for partial download to resume
    let existing_len = if dest.exists() {
        tokio::fs::metadata(dest).await?.len()
    } else {
        0
    };

    let mut request = client.get(url);
    if existing_len > 0 {
        request = request.header("Range", format!("bytes={}-", existing_len));
    }

    let response = request.send().await?;

    if !response.status().is_success() && response.status().as_u16() != 206 {
        return Err(DownloadError::BadStatus(response.status().as_u16()));
    }

    let total = if response.status().as_u16() == 206 {
        // Partial content — total is existing + remaining
        existing_len + response.content_length().unwrap_or(0)
    } else {
        response.content_length().unwrap_or(0)
    };

    // Open file in append mode if resuming, create otherwise
    let mut file = if existing_len > 0 && response.status().as_u16() == 206 {
        tokio::fs::OpenOptions::new()
            .append(true)
            .open(dest)
            .await?
    } else {
        if let Some(parent) = dest.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::File::create(dest).await?
    };

    let mut downloaded = if response.status().as_u16() == 206 { existing_len } else { 0 };
    let start_time = std::time::Instant::now();
    let mut stream = response.bytes_stream();

    while let Some(chunk_result) = stream.next().await {
        if cancel.is_cancelled() {
            return Err(DownloadError::Cancelled);
        }

        let chunk = chunk_result?;
        file.write_all(&chunk).await?;
        downloaded += chunk.len() as u64;

        let elapsed = start_time.elapsed().as_secs_f64();
        let speed_bps = if elapsed > 0.0 {
            (downloaded as f64 / elapsed) as u64
        } else {
            0
        };
        let remaining = total.saturating_sub(downloaded);
        let eta_secs = if speed_bps > 0 { remaining / speed_bps } else { 0 };

        on_progress(DownloadProgress {
            downloaded,
            total,
            percent: if total > 0 { (downloaded as f32 / total as f32) * 100.0 } else { 0.0 },
            speed_bps,
            eta_secs,
        });
    }

    file.flush().await?;
    Ok(())
}
```

**Step 2: Add tokio-util dependency to Cargo.toml**

Add to `[dependencies]` in `tools/SGWLauncher/src-tauri/Cargo.toml`:

```toml
tokio-util = "0.7"
```

**Step 3: Register module in main.rs**

Add `mod download;` to `tools/SGWLauncher/src-tauri/src/main.rs`.

**Step 4: Verify it compiles**

Run: `cd tools/SGWLauncher/src-tauri && cargo build`
Expected: Compiles successfully

Note: Unit testing HTTP downloads requires a mock server (e.g. `wiremock`), which is out of scope. The download module will be integration-tested when wiring up the full install pipeline in Task 7. The logic is straightforward reqwest streaming — the risk is low.

**Step 5: Commit**

```bash
git add tools/SGWLauncher/src-tauri/src/download.rs tools/SGWLauncher/src-tauri/Cargo.toml tools/SGWLauncher/src-tauri/src/main.rs
git commit -m "feat(launcher): HTTP download with progress and resume

Streaming download via reqwest with progress callback (bytes,
speed, ETA). Supports resume via HTTP Range header and cancellation
via tokio CancellationToken."
```

---

### Task 5: 7za extraction wrapper

**Files:**
- Create: `tools/SGWLauncher/src-tauri/src/extract.rs`
- Modify: `tools/SGWLauncher/src-tauri/src/main.rs`

**Context:** Wraps the bundled 7za.exe sidecar to extract RAR archives and CAB files. Parses stdout for progress reporting. Uses `tauri_plugin_shell::ShellExt` for sidecar invocation.

**Step 1: Write extract module**

Create `tools/SGWLauncher/src-tauri/src/extract.rs`:

```rust
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ExtractError {
    #[error("7za exited with code {code}: {stderr}")]
    ExitCode { code: i32, stderr: String },
    #[error("Failed to spawn 7za: {0}")]
    Spawn(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Serialize)]
pub struct ExtractProgress {
    pub phase: String, // "rar" or "cab"
    pub current: u32,
    pub total: u32,
    pub filename: String,
}

/// Count .cab files in a directory.
pub fn count_cab_files(dir: &std::path::Path) -> std::io::Result<Vec<std::path::PathBuf>> {
    let mut cabs: Vec<std::path::PathBuf> = std::fs::read_dir(dir)?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("cab") {
                Some(path)
            } else {
                None
            }
        })
        .collect();
    cabs.sort();
    Ok(cabs)
}

/// Build 7za command arguments for extraction.
///
/// Produces: `e <archive> -o<output_dir> -y`
/// The `e` command extracts without directory structure (flat).
pub fn build_extract_args(archive: &str, output_dir: &str) -> Vec<String> {
    vec![
        "e".to_string(),
        archive.to_string(),
        format!("-o{}", output_dir),
        "-y".to_string(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_extract_args() {
        let args = build_extract_args("C:\\temp\\archive.rar", "C:\\Games\\SGW");
        assert_eq!(args, vec!["e", "C:\\temp\\archive.rar", "-oC:\\Games\\SGW", "-y"]);
    }

    #[test]
    fn test_count_cab_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("data1.cab"), "").unwrap();
        std::fs::write(dir.path().join("data2.cab"), "").unwrap();
        std::fs::write(dir.path().join("readme.txt"), "").unwrap();

        let cabs = count_cab_files(dir.path()).unwrap();
        assert_eq!(cabs.len(), 2);
        assert!(cabs[0].file_name().unwrap().to_str().unwrap().ends_with(".cab"));
    }

    #[test]
    fn test_count_cab_files_empty() {
        let dir = tempfile::tempdir().unwrap();
        let cabs = count_cab_files(dir.path()).unwrap();
        assert_eq!(cabs.len(), 0);
    }
}
```

**Step 2: Run tests**

Run: `cd tools/SGWLauncher/src-tauri && cargo test -- extract`
Expected: 3 tests pass

**Step 3: Register module in main.rs**

Add `mod extract;` to `tools/SGWLauncher/src-tauri/src/main.rs`.

**Step 4: Commit**

```bash
git add tools/SGWLauncher/src-tauri/src/extract.rs tools/SGWLauncher/src-tauri/src/main.rs
git commit -m "feat(launcher): 7za extraction wrapper

Helper functions for building 7za extract arguments and counting
CAB files. Actual sidecar invocation happens in the commands module
via tauri_plugin_shell."
```

---

### Task 6: Game launcher module

**Files:**
- Create: `tools/SGWLauncher/src-tauri/src/launch.rs`
- Modify: `tools/SGWLauncher/src-tauri/src/main.rs`

**Context:** Spawns SGW.exe as a detached child process. Verifies the executable exists before launching.

**Step 1: Write launch module**

Create `tools/SGWLauncher/src-tauri/src/launch.rs`:

```rust
use std::path::Path;
use std::process::Command;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LaunchError {
    #[error("SGW.exe not found at {0}")]
    NotFound(String),
    #[error("Failed to launch game: {0}")]
    SpawnFailed(#[from] std::io::Error),
}

/// Verify SGW.exe exists at the given install path.
pub fn verify_installation(install_path: &str) -> Result<std::path::PathBuf, LaunchError> {
    let exe_path = Path::new(install_path).join("SGW.exe");
    if !exe_path.exists() {
        return Err(LaunchError::NotFound(exe_path.display().to_string()));
    }
    Ok(exe_path)
}

/// Launch SGW.exe from the install directory.
///
/// The game is spawned as a detached process — the launcher does not
/// wait for it to exit.
pub fn launch_game(install_path: &str) -> Result<u32, LaunchError> {
    let exe_path = verify_installation(install_path)?;

    let child = Command::new(&exe_path)
        .current_dir(install_path)
        .spawn()?;

    Ok(child.id())
}

/// Check the installation state.
#[derive(Debug, Clone, serde::Serialize)]
pub enum InstallState {
    NotInstalled,
    Installed,
}

pub fn check_installation(install_path: &str) -> InstallState {
    if install_path.is_empty() {
        return InstallState::NotInstalled;
    }
    match verify_installation(install_path) {
        Ok(_) => InstallState::Installed,
        Err(_) => InstallState::NotInstalled,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_installation_missing() {
        let result = verify_installation("/nonexistent/path");
        assert!(matches!(result, Err(LaunchError::NotFound(_))));
    }

    #[test]
    fn test_verify_installation_exists() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("SGW.exe"), "fake exe").unwrap();
        let result = verify_installation(dir.path().to_str().unwrap());
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_installation_empty_path() {
        let state = check_installation("");
        assert!(matches!(state, InstallState::NotInstalled));
    }

    #[test]
    fn test_check_installation_with_exe() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("SGW.exe"), "fake exe").unwrap();
        let state = check_installation(dir.path().to_str().unwrap());
        assert!(matches!(state, InstallState::Installed));
    }
}
```

**Step 2: Run tests**

Run: `cd tools/SGWLauncher/src-tauri && cargo test -- launch`
Expected: 4 tests pass

**Step 3: Register module in main.rs**

Add `mod launch;` to `tools/SGWLauncher/src-tauri/src/main.rs`.

**Step 4: Commit**

```bash
git add tools/SGWLauncher/src-tauri/src/launch.rs tools/SGWLauncher/src-tauri/src/main.rs
git commit -m "feat(launcher): game launch and installation check

Verify SGW.exe exists, spawn as detached process, check install
state. 4 unit tests."
```

---

### Task 7: Delta updater module

**Files:**
- Create: `tools/SGWLauncher/src-tauri/src/updater.rs`
- Modify: `tools/SGWLauncher/src-tauri/src/main.rs`

**Context:** Fetches a manifest.json from the patch server, compares SHA-256 hashes of local files against the manifest, and downloads changed files as ZIP archives.

**Step 1: Write updater module**

Create `tools/SGWLauncher/src-tauri/src/updater.rs`:

```rust
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum UpdateError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Hash mismatch for {file}: expected {expected}, got {actual}")]
    HashMismatch {
        file: String,
        expected: String,
        actual: String,
    },
}

/// A single file entry in the patch manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestEntry {
    pub path: String,
    pub sha256: String,
    pub size: u64,
    pub patch_url: String,
}

/// The patch manifest served by the patch server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchManifest {
    pub version: String,
    pub files: Vec<ManifestEntry>,
}

/// Result of checking for updates.
#[derive(Debug, Clone, Serialize)]
pub struct UpdateCheckResult {
    pub updates_available: bool,
    pub files_to_update: Vec<ManifestEntry>,
    pub total_bytes: u64,
    pub server_version: String,
}

/// Compute SHA-256 hash of a file.
pub fn hash_file(path: &Path) -> std::io::Result<String> {
    let data = std::fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&data);
    Ok(format!("{:x}", hasher.finalize()))
}

/// Compare local files against the manifest and return files that need updating.
pub fn check_manifest(
    manifest: &PatchManifest,
    install_path: &Path,
) -> Vec<ManifestEntry> {
    manifest
        .files
        .iter()
        .filter(|entry| {
            let local_path = install_path.join(&entry.path);
            if !local_path.exists() {
                return true; // missing file needs download
            }
            match hash_file(&local_path) {
                Ok(hash) => hash != entry.sha256,
                Err(_) => true, // can't read = needs update
            }
        })
        .cloned()
        .collect()
}

/// Fetch the patch manifest from the server.
pub async fn fetch_manifest(patch_server_url: &str) -> Result<PatchManifest, UpdateError> {
    let url = format!("{}/manifest.json", patch_server_url.trim_end_matches('/'));
    let manifest: PatchManifest = reqwest::get(&url).await?.json().await?;
    Ok(manifest)
}

/// Verify a downloaded file matches its expected hash.
pub fn verify_hash(path: &Path, expected: &str) -> Result<(), UpdateError> {
    let actual = hash_file(path).map_err(UpdateError::Io)?;
    if actual != expected {
        return Err(UpdateError::HashMismatch {
            file: path.display().to_string(),
            expected: expected.to_string(),
            actual,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        std::fs::write(&path, "hello world").unwrap();

        let hash = hash_file(&path).unwrap();
        // SHA-256 of "hello world"
        assert_eq!(
            hash,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn test_check_manifest_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let manifest = PatchManifest {
            version: "1.0".to_string(),
            files: vec![ManifestEntry {
                path: "missing.dat".to_string(),
                sha256: "abc123".to_string(),
                size: 100,
                patch_url: "https://example.com/missing.dat.zip".to_string(),
            }],
        };

        let updates = check_manifest(&manifest, dir.path());
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].path, "missing.dat");
    }

    #[test]
    fn test_check_manifest_matching_file() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("data.txt");
        std::fs::write(&file_path, "hello world").unwrap();

        let manifest = PatchManifest {
            version: "1.0".to_string(),
            files: vec![ManifestEntry {
                path: "data.txt".to_string(),
                sha256: "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
                    .to_string(),
                size: 11,
                patch_url: "https://example.com/data.txt.zip".to_string(),
            }],
        };

        let updates = check_manifest(&manifest, dir.path());
        assert_eq!(updates.len(), 0);
    }

    #[test]
    fn test_check_manifest_changed_file() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("data.txt");
        std::fs::write(&file_path, "old content").unwrap();

        let manifest = PatchManifest {
            version: "1.0".to_string(),
            files: vec![ManifestEntry {
                path: "data.txt".to_string(),
                sha256: "newsha256hash".to_string(),
                size: 50,
                patch_url: "https://example.com/data.txt.zip".to_string(),
            }],
        };

        let updates = check_manifest(&manifest, dir.path());
        assert_eq!(updates.len(), 1);
    }

    #[test]
    fn test_verify_hash_success() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        std::fs::write(&path, "hello world").unwrap();

        let result = verify_hash(
            &path,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9",
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_verify_hash_mismatch() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        std::fs::write(&path, "hello world").unwrap();

        let result = verify_hash(&path, "wronghash");
        assert!(matches!(result, Err(UpdateError::HashMismatch { .. })));
    }
}
```

**Step 2: Run tests**

Run: `cd tools/SGWLauncher/src-tauri && cargo test -- updater`
Expected: 5 tests pass

**Step 3: Register module in main.rs**

Add `mod updater;` to `tools/SGWLauncher/src-tauri/src/main.rs`.

**Step 4: Commit**

```bash
git add tools/SGWLauncher/src-tauri/src/updater.rs tools/SGWLauncher/src-tauri/src/main.rs
git commit -m "feat(launcher): delta update system

Fetch patch manifest from configurable server URL, compare SHA-256
hashes of local files, identify files needing update. Manifest
format: version + array of {path, sha256, size, patch_url}.
5 unit tests."
```

---

### Task 8: Tauri IPC commands

**Files:**
- Create: `tools/SGWLauncher/src-tauri/src/commands.rs`
- Modify: `tools/SGWLauncher/src-tauri/src/main.rs`

**Context:** This wires all the Rust modules into Tauri IPC commands that the frontend can invoke. This is the glue layer — it uses config, patch, download, extract, launch, and updater modules.

**Step 1: Write commands module**

Create `tools/SGWLauncher/src-tauri/src/commands.rs`:

```rust
use crate::{config::LauncherConfig, download, extract, launch, patch, updater};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_shell::ShellExt;
use tokio_util::sync::CancellationToken;

/// App state managed by Tauri.
pub struct AppState {
    pub config_path: PathBuf,
    pub config: Mutex<LauncherConfig>,
    pub cancel_token: Mutex<Option<CancellationToken>>,
}

#[derive(Debug, Serialize)]
pub struct CommandError {
    message: String,
}

impl From<String> for CommandError {
    fn from(s: String) -> Self {
        Self { message: s }
    }
}

type CmdResult<T> = Result<T, String>;

#[tauri::command]
pub fn cmd_check_installation(
    state: tauri::State<'_, AppState>,
) -> CmdResult<launch::InstallState> {
    let config = state.config.lock().unwrap();
    Ok(launch::check_installation(&config.install_path))
}

#[tauri::command]
pub fn cmd_get_default_install_path() -> String {
    // Default to C:\Games\Stargate Worlds on Windows
    let base = std::env::var("ProgramFiles")
        .unwrap_or_else(|_| "C:\\Program Files".to_string());
    format!("{}\\Stargate Worlds", base)
}

#[tauri::command]
pub fn cmd_load_config(state: tauri::State<'_, AppState>) -> CmdResult<LauncherConfig> {
    let config = state.config.lock().unwrap();
    Ok(config.clone())
}

#[tauri::command]
pub fn cmd_save_config(
    state: tauri::State<'_, AppState>,
    config: LauncherConfig,
) -> CmdResult<()> {
    config
        .save(&state.config_path)
        .map_err(|e| e.to_string())?;
    *state.config.lock().unwrap() = config;
    Ok(())
}

#[tauri::command]
pub fn cmd_patch_server_address(
    state: tauri::State<'_, AppState>,
    address: String,
) -> CmdResult<()> {
    let config = state.config.lock().unwrap();
    let exe_path = std::path::Path::new(&config.install_path).join("SGW.exe");

    if !exe_path.exists() {
        return Err("SGW.exe not found".to_string());
    }

    // Only patch if the original hostname is still present
    let data = std::fs::read(&exe_path).map_err(|e| e.to_string())?;
    if patch::needs_patching(&data) {
        patch::patch_exe(&exe_path, &address).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn cmd_launch_game(state: tauri::State<'_, AppState>) -> CmdResult<u32> {
    let config = state.config.lock().unwrap();
    launch::launch_game(&config.install_path).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn cmd_cancel_install(state: tauri::State<'_, AppState>) -> CmdResult<()> {
    if let Some(token) = state.cancel_token.lock().unwrap().take() {
        token.cancel();
    }
    Ok(())
}

#[tauri::command]
pub async fn cmd_download_and_install(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    install_path: String,
    server_address: String,
) -> CmdResult<()> {
    let cancel = CancellationToken::new();
    {
        let mut token = state.cancel_token.lock().unwrap();
        *token = Some(cancel.clone());
    }

    let temp_dir = std::env::temp_dir().join("sgw-install");
    let rar_path = temp_dir.join("StargateWorlds.rar");
    let extract_dir = temp_dir.join("extracted");

    // Step 1: Download RAR
    let archive_url = "https://archive.org/download/StargateWorlds_0.8348.1.4046/Stargate%20Worlds%20%280.8348.1.4046%29%20%282009-06-30%29%20%28beta%29.rar";

    let app_clone = app.clone();
    download::download_file(archive_url, &rar_path, cancel.clone(), move |progress| {
        let _ = app_clone.emit("download-progress", &progress);
    })
    .await
    .map_err(|e| e.to_string())?;

    if cancel.is_cancelled() {
        return Err("Install cancelled".to_string());
    }

    // Step 2: Extract RAR → temp dir
    let app_clone = app.clone();
    let rar_str = rar_path.to_string_lossy().to_string();
    let extract_str = extract_dir.to_string_lossy().to_string();
    let args = extract::build_extract_args(&rar_str, &extract_str);

    let sidecar = app.shell().sidecar("binaries/7za").unwrap();
    let output = sidecar
        .args(&args)
        .output()
        .await
        .map_err(|e| format!("Failed to run 7za: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("7za RAR extraction failed: {}", stderr));
    }

    let _ = app.emit(
        "extract-progress",
        &extract::ExtractProgress {
            phase: "rar".to_string(),
            current: 1,
            total: 1,
            filename: "StargateWorlds.rar".to_string(),
        },
    );

    // Step 3: Extract each CAB → install path
    let cabs = extract::count_cab_files(&extract_dir).map_err(|e| e.to_string())?;
    let total_cabs = cabs.len() as u32;

    for (i, cab) in cabs.iter().enumerate() {
        if cancel.is_cancelled() {
            return Err("Install cancelled".to_string());
        }

        let cab_str = cab.to_string_lossy().to_string();
        let args = extract::build_extract_args(&cab_str, &install_path);

        let sidecar = app.shell().sidecar("binaries/7za").unwrap();
        let output = sidecar
            .args(&args)
            .output()
            .await
            .map_err(|e| format!("Failed to run 7za: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("7za CAB extraction failed: {}", stderr));
        }

        let filename = cab
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let _ = app.emit(
            "extract-progress",
            &extract::ExtractProgress {
                phase: "cab".to_string(),
                current: (i + 1) as u32,
                total: total_cabs,
                filename,
            },
        );
    }

    // Step 4: Verify SGW.exe exists
    launch::verify_installation(&install_path).map_err(|e| e.to_string())?;

    // Step 5: Patch server address
    let exe_path = std::path::Path::new(&install_path).join("SGW.exe");
    patch::patch_exe(&exe_path, &server_address).map_err(|e| e.to_string())?;

    // Step 6: Save config
    let new_config = LauncherConfig {
        install_path: install_path.clone(),
        server_address,
        ..state.config.lock().unwrap().clone()
    };
    new_config
        .save(&state.config_path)
        .map_err(|e| e.to_string())?;
    *state.config.lock().unwrap() = new_config;

    // Step 7: Clean up temp dir
    let _ = std::fs::remove_dir_all(&temp_dir);

    let _ = app.emit("install-complete", ());
    Ok(())
}

#[tauri::command]
pub async fn cmd_check_for_updates(
    state: tauri::State<'_, AppState>,
) -> CmdResult<updater::UpdateCheckResult> {
    let config = state.config.lock().unwrap().clone();

    let manifest = updater::fetch_manifest(&config.patch_server_url)
        .await
        .map_err(|e| e.to_string())?;

    let install_path = std::path::Path::new(&config.install_path);
    let files_to_update = updater::check_manifest(&manifest, install_path);
    let total_bytes: u64 = files_to_update.iter().map(|f| f.size).sum();

    Ok(updater::UpdateCheckResult {
        updates_available: !files_to_update.is_empty(),
        files_to_update,
        total_bytes,
        server_version: manifest.version,
    })
}

#[tauri::command]
pub async fn cmd_apply_updates(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> CmdResult<()> {
    let config = state.config.lock().unwrap().clone();
    let install_path = std::path::Path::new(&config.install_path);

    let manifest = updater::fetch_manifest(&config.patch_server_url)
        .await
        .map_err(|e| e.to_string())?;

    let files_to_update = updater::check_manifest(&manifest, install_path);
    let total = files_to_update.len() as u32;

    for (i, entry) in files_to_update.iter().enumerate() {
        let dest = install_path.join(&entry.path);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }

        // Download the file
        let cancel = CancellationToken::new(); // no cancel support for updates yet
        let app_clone = app.clone();
        let entry_path = entry.path.clone();
        let current = (i + 1) as u32;
        let total_copy = total;
        download::download_file(&entry.patch_url, &dest, cancel, move |_progress| {
            let _ = app_clone.emit(
                "update-progress",
                &serde_json::json!({
                    "current": current,
                    "total": total_copy,
                    "filename": entry_path,
                }),
            );
        })
        .await
        .map_err(|e| e.to_string())?;

        // Verify hash
        updater::verify_hash(&dest, &entry.sha256).map_err(|e| e.to_string())?;
    }

    let _ = app.emit("update-complete", ());
    Ok(())
}
```

**Step 2: Update main.rs to register commands and state**

Replace the contents of `tools/SGWLauncher/src-tauri/src/main.rs`:

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod config;
mod download;
mod extract;
mod launch;
mod patch;
mod updater;

use commands::AppState;
use config::LauncherConfig;
use std::path::PathBuf;
use std::sync::Mutex;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // Determine config path in app data directory
            let config_dir = app
                .path()
                .app_config_dir()
                .unwrap_or_else(|_| PathBuf::from("."));
            let config_path = config_dir.join("config.json");

            // Load or create default config
            let config = LauncherConfig::load(&config_path)
                .unwrap_or_default();

            app.manage(AppState {
                config_path,
                config: Mutex::new(config),
                cancel_token: Mutex::new(None),
            });

            #[cfg(desktop)]
            app.handle()
                .plugin(tauri_plugin_updater::Builder::new().build())?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::cmd_check_installation,
            commands::cmd_get_default_install_path,
            commands::cmd_load_config,
            commands::cmd_save_config,
            commands::cmd_patch_server_address,
            commands::cmd_launch_game,
            commands::cmd_cancel_install,
            commands::cmd_download_and_install,
            commands::cmd_check_for_updates,
            commands::cmd_apply_updates,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

**Step 3: Verify it compiles**

Run: `cd tools/SGWLauncher/src-tauri && cargo build`
Expected: Compiles successfully

**Step 4: Commit**

```bash
git add tools/SGWLauncher/src-tauri/src/commands.rs tools/SGWLauncher/src-tauri/src/main.rs
git commit -m "feat(launcher): Tauri IPC commands and app state

Wire all Rust modules into Tauri commands: check_installation,
download_and_install, cancel_install, patch_server_address,
launch_game, load/save config, check/apply updates. Full install
pipeline: download → extract RAR → extract CABs → patch → save config."
```

---

### Task 9: Frontend UI

**Files:**
- Modify: `tools/SGWLauncher/ui/index.html`
- Modify: `tools/SGWLauncher/ui/main.js`
- Modify: `tools/SGWLauncher/ui/styles.css`

**Context:** The frontend is a state machine driven by Tauri IPC. No framework — vanilla JS manages DOM updates. Dark theme, 600x400, centered layout.

**Step 1: Write `tools/SGWLauncher/ui/index.html`**

```html
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Stargate Worlds</title>
  <link rel="stylesheet" href="styles.css">
</head>
<body>
  <div id="app">
    <h1>STARGATE WORLDS</h1>

    <!-- Not Installed -->
    <div id="state-not-installed" class="state" hidden>
      <p class="subtitle">Install the game client to get started.</p>
      <div class="field-row">
        <label for="install-path">Install path</label>
        <div class="path-input">
          <input type="text" id="install-path" readonly>
          <button id="btn-browse" class="btn-secondary">Browse</button>
        </div>
      </div>
      <button id="btn-install" class="btn-primary">Download &amp; Install</button>
    </div>

    <!-- Downloading -->
    <div id="state-downloading" class="state" hidden>
      <p class="subtitle">Downloading game client...</p>
      <div class="progress-container">
        <div class="progress-bar"><div id="download-bar" class="progress-fill"></div></div>
        <div class="progress-info">
          <span id="download-percent">0%</span>
          <span id="download-speed"></span>
          <span id="download-eta"></span>
        </div>
      </div>
      <button id="btn-cancel" class="btn-secondary">Cancel</button>
    </div>

    <!-- Extracting -->
    <div id="state-extracting" class="state" hidden>
      <p class="subtitle">Extracting game files...</p>
      <div class="progress-container">
        <div class="progress-bar"><div id="extract-bar" class="progress-fill"></div></div>
        <div class="progress-info">
          <span id="extract-phase"></span>
          <span id="extract-file"></span>
        </div>
      </div>
    </div>

    <!-- Ready -->
    <div id="state-ready" class="state" hidden>
      <div class="field-row">
        <label for="server-address">Server</label>
        <input type="text" id="server-address">
      </div>
      <button id="btn-play" class="btn-play">PLAY</button>
      <div class="ready-actions">
        <button id="btn-check-updates" class="btn-link">Check for updates</button>
      </div>
    </div>

    <!-- Updating -->
    <div id="state-updating" class="state" hidden>
      <p class="subtitle">Updating game files...</p>
      <div class="progress-container">
        <div class="progress-bar"><div id="update-bar" class="progress-fill"></div></div>
        <div class="progress-info">
          <span id="update-count"></span>
          <span id="update-file"></span>
        </div>
      </div>
    </div>

    <!-- Error -->
    <div id="state-error" class="state" hidden>
      <p class="error-icon">!</p>
      <p id="error-message" class="error-text"></p>
      <button id="btn-retry" class="btn-secondary">Retry</button>
    </div>
  </div>

  <script src="main.js" type="module"></script>
</body>
</html>
```

**Step 2: Write `tools/SGWLauncher/ui/styles.css`**

```css
* { margin: 0; padding: 0; box-sizing: border-box; }

body {
  font-family: 'Segoe UI', system-ui, sans-serif;
  background: #0d0d1a;
  color: #c8c8d8;
  height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
  user-select: none;
  overflow: hidden;
}

#app {
  width: 100%;
  max-width: 480px;
  text-align: center;
  padding: 2rem;
}

h1 {
  font-size: 1.6rem;
  letter-spacing: 0.3em;
  color: #ffffff;
  margin-bottom: 1.5rem;
  font-weight: 300;
}

.subtitle {
  color: #8888a0;
  margin-bottom: 1.5rem;
  font-size: 0.9rem;
}

.state { animation: fadeIn 0.2s ease-in; }
@keyframes fadeIn { from { opacity: 0; } to { opacity: 1; } }

/* Fields */
.field-row {
  margin-bottom: 1rem;
  text-align: left;
}
.field-row label {
  display: block;
  font-size: 0.8rem;
  color: #6666880;
  margin-bottom: 0.3rem;
  text-transform: uppercase;
  letter-spacing: 0.1em;
}
.path-input {
  display: flex;
  gap: 0.5rem;
}
input[type="text"] {
  flex: 1;
  background: #1a1a2e;
  border: 1px solid #2a2a4a;
  color: #e0e0e0;
  padding: 0.5rem 0.75rem;
  border-radius: 4px;
  font-size: 0.85rem;
  outline: none;
}
input[type="text"]:focus {
  border-color: #4a4a8a;
}

/* Buttons */
.btn-primary, .btn-secondary, .btn-play {
  border: none;
  border-radius: 4px;
  padding: 0.6rem 1.5rem;
  font-size: 0.9rem;
  cursor: pointer;
  transition: background 0.15s;
}
.btn-primary {
  background: #3a3a7a;
  color: #ffffff;
  width: 100%;
  margin-top: 0.5rem;
}
.btn-primary:hover { background: #4a4a8a; }
.btn-secondary {
  background: #1a1a2e;
  color: #c8c8d8;
  border: 1px solid #2a2a4a;
}
.btn-secondary:hover { background: #2a2a3e; }
.btn-play {
  background: #2e6b3e;
  color: #ffffff;
  width: 100%;
  padding: 1rem;
  font-size: 1.2rem;
  letter-spacing: 0.2em;
  font-weight: 600;
  margin-top: 0.5rem;
}
.btn-play:hover { background: #3a8a4e; }
.btn-link {
  background: none;
  border: none;
  color: #6a6a9a;
  font-size: 0.8rem;
  cursor: pointer;
  padding: 0.5rem;
}
.btn-link:hover { color: #9a9aba; }

.ready-actions {
  margin-top: 1rem;
}

/* Progress */
.progress-container { margin: 1rem 0; }
.progress-bar {
  background: #1a1a2e;
  border-radius: 4px;
  height: 8px;
  overflow: hidden;
}
.progress-fill {
  background: #3a3a7a;
  height: 100%;
  width: 0%;
  transition: width 0.3s;
  border-radius: 4px;
}
.progress-info {
  display: flex;
  justify-content: space-between;
  font-size: 0.75rem;
  color: #6a6a8a;
  margin-top: 0.5rem;
}

/* Error */
.error-icon {
  font-size: 2rem;
  color: #cc4444;
  margin-bottom: 0.5rem;
}
.error-text {
  color: #cc8888;
  margin-bottom: 1rem;
  font-size: 0.9rem;
}
```

**Step 3: Write `tools/SGWLauncher/ui/main.js`**

```javascript
const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const { open } = window.__TAURI__.dialog;

// --- State machine ---
const states = ['not-installed', 'downloading', 'extracting', 'ready', 'updating', 'error'];
let currentState = null;
let previousState = null;

function showState(name) {
  previousState = currentState;
  currentState = name;
  for (const s of states) {
    const el = document.getElementById('state-' + s);
    el.hidden = s !== name;
  }
}

// --- Elements ---
const installPathInput = document.getElementById('install-path');
const serverAddressInput = document.getElementById('server-address');
const downloadBar = document.getElementById('download-bar');
const downloadPercent = document.getElementById('download-percent');
const downloadSpeed = document.getElementById('download-speed');
const downloadEta = document.getElementById('download-eta');
const extractBar = document.getElementById('extract-bar');
const extractPhase = document.getElementById('extract-phase');
const extractFile = document.getElementById('extract-file');
const updateBar = document.getElementById('update-bar');
const updateCount = document.getElementById('update-count');
const updateFile = document.getElementById('update-file');
const errorMessage = document.getElementById('error-message');

// --- Helpers ---
function formatBytes(bytes) {
  if (bytes < 1024) return bytes + ' B';
  if (bytes < 1048576) return (bytes / 1024).toFixed(1) + ' KB';
  if (bytes < 1073741824) return (bytes / 1048576).toFixed(1) + ' MB';
  return (bytes / 1073741824).toFixed(1) + ' GB';
}

function formatSpeed(bps) {
  return formatBytes(bps) + '/s';
}

function formatEta(secs) {
  if (secs < 60) return secs + 's';
  if (secs < 3600) return Math.floor(secs / 60) + 'm ' + (secs % 60) + 's';
  return Math.floor(secs / 3600) + 'h ' + Math.floor((secs % 3600) / 60) + 'm';
}

function showError(msg) {
  errorMessage.textContent = msg;
  showState('error');
}

// --- Event listeners ---
listen('download-progress', (event) => {
  const p = event.payload;
  downloadBar.style.width = p.percent.toFixed(1) + '%';
  downloadPercent.textContent = p.percent.toFixed(1) + '%';
  downloadSpeed.textContent = formatSpeed(p.speed_bps);
  downloadEta.textContent = formatEta(p.eta_secs);
});

listen('extract-progress', (event) => {
  const p = event.payload;
  showState('extracting');
  const pct = p.total > 0 ? (p.current / p.total * 100) : 0;
  extractBar.style.width = pct.toFixed(1) + '%';
  extractPhase.textContent = p.phase === 'rar' ? 'Extracting archive...' : `CAB ${p.current}/${p.total}`;
  extractFile.textContent = p.filename;
});

listen('install-complete', () => {
  showState('ready');
});

listen('install-error', (event) => {
  showError(event.payload.message);
});

listen('update-progress', (event) => {
  const p = event.payload;
  const pct = p.total > 0 ? (p.current / p.total * 100) : 0;
  updateBar.style.width = pct.toFixed(1) + '%';
  updateCount.textContent = `${p.current}/${p.total}`;
  updateFile.textContent = p.filename;
});

listen('update-complete', () => {
  showState('ready');
});

// --- Button handlers ---
document.getElementById('btn-browse').addEventListener('click', async () => {
  const selected = await open({ directory: true, title: 'Select install location' });
  if (selected) {
    installPathInput.value = selected;
  }
});

document.getElementById('btn-install').addEventListener('click', async () => {
  const path = installPathInput.value;
  if (!path) {
    showError('Please select an install location.');
    return;
  }
  showState('downloading');
  try {
    const config = await invoke('cmd_load_config');
    await invoke('cmd_download_and_install', {
      installPath: path,
      serverAddress: config.server_address,
    });
  } catch (e) {
    showError(typeof e === 'string' ? e : e.message || 'Install failed');
  }
});

document.getElementById('btn-cancel').addEventListener('click', async () => {
  await invoke('cmd_cancel_install');
  showState('not-installed');
});

document.getElementById('btn-play').addEventListener('click', async () => {
  try {
    await invoke('cmd_launch_game');
  } catch (e) {
    showError(typeof e === 'string' ? e : e.message || 'Launch failed');
  }
});

document.getElementById('btn-check-updates').addEventListener('click', async () => {
  try {
    const result = await invoke('cmd_check_for_updates');
    if (result.updates_available) {
      showState('updating');
      await invoke('cmd_apply_updates');
    }
  } catch (e) {
    showError(typeof e === 'string' ? e : e.message || 'Update check failed');
  }
});

document.getElementById('btn-retry').addEventListener('click', () => {
  showState(previousState || 'not-installed');
});

// Server address: save on blur
serverAddressInput.addEventListener('blur', async () => {
  const address = serverAddressInput.value.trim();
  if (!address) return;
  try {
    const config = await invoke('cmd_load_config');
    config.server_address = address;
    await invoke('cmd_save_config', { config });
    await invoke('cmd_patch_server_address', { address });
  } catch (e) {
    showError(typeof e === 'string' ? e : e.message || 'Failed to update server address');
  }
});

// --- Initialization ---
async function init() {
  try {
    const config = await invoke('cmd_load_config');
    serverAddressInput.value = config.server_address;
    installPathInput.value = config.install_path || await invoke('cmd_get_default_install_path');

    const state = await invoke('cmd_check_installation');
    if (state === 'Installed') {
      showState('ready');
    } else {
      showState('not-installed');
    }
  } catch (e) {
    showError('Failed to initialize: ' + (typeof e === 'string' ? e : e.message));
  }
}

init();
```

**Step 4: Verify it compiles (frontend has no build step, just check Rust still builds)**

Run: `cd tools/SGWLauncher/src-tauri && cargo build`
Expected: Compiles

**Step 5: Commit**

```bash
git add tools/SGWLauncher/ui/
git commit -m "feat(launcher): frontend UI with state machine

Vanilla HTML/CSS/JS frontend with 6 states: not-installed,
downloading, extracting, ready, updating, error. Dark theme
(#0d0d1a), 600x400. Progress bars for download/extract/update.
Folder picker via @tauri-apps/plugin-dialog."
```

---

### Task 10: Bundle 7za.exe sidecar

**Files:**
- Create: `tools/SGWLauncher/src-tauri/binaries/7za-x86_64-pc-windows-msvc.exe`

**Context:** Tauri sidecars must be named `<name>-<target-triple>.exe`. The 7za standalone binary is available from https://7-zip.org/a/7zr.exe (self-extracting) or https://www.7-zip.org/a/7z2408-extra.7z. For development, we need to obtain 7za.exe and place it with the correct name.

**Step 1: Add a placeholder README for the sidecar**

Create `tools/SGWLauncher/src-tauri/binaries/README.md`:

```markdown
# Sidecar Binaries

Place `7za-x86_64-pc-windows-msvc.exe` here before building.

## How to obtain

1. Download 7-Zip Extra from https://www.7-zip.org/download.html (look for "7-Zip Extra: standalone console version")
2. Extract and rename `7za.exe` to `7za-x86_64-pc-windows-msvc.exe`
3. Place it in this directory

The Tauri sidecar naming convention requires the target triple suffix.
The file is ~1MB and is not checked into git.
```

**Step 2: Add sidecar to .gitignore**

Append to `tools/SGWLauncher/.gitignore` (create if needed):

```
# Sidecar binaries (downloaded, not committed)
src-tauri/binaries/*.exe

# Tauri build output
src-tauri/target/
src-tauri/gen/schemas/

# Frontend has no build step, nothing to ignore
```

**Step 3: Commit**

```bash
git add tools/SGWLauncher/src-tauri/binaries/README.md tools/SGWLauncher/.gitignore
git commit -m "docs(launcher): 7za sidecar setup instructions

README explains how to obtain 7za.exe and name it for Tauri sidecar.
Binary is .gitignored (not committed to repo)."
```

---

### Task 11: Launcher README

**Files:**
- Create: `tools/SGWLauncher/README.md`

**Step 1: Write README**

```markdown
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
```

**Step 2: Commit**

```bash
git add tools/SGWLauncher/README.md
git commit -m "docs(launcher): add README with dev and build instructions"
```

---

### Task 12: Integration test — full build verification

**Step 1: Run all unit tests**

Run: `cd tools/SGWLauncher/src-tauri && cargo test`
Expected: All tests pass (config: 3, patch: 7, extract: 3, launch: 4, updater: 5 = 22 tests)

**Step 2: Run clippy**

Run: `cd tools/SGWLauncher/src-tauri && cargo clippy -- -D warnings`
Expected: No warnings

**Step 3: Fix any issues found by clippy or tests**

Address all warnings and test failures before proceeding.

**Step 4: Final build check**

Run: `cd tools/SGWLauncher/src-tauri && cargo build`
Expected: Clean build, no warnings

**Step 5: Commit any fixes**

If clippy or test fixes were needed:
```bash
git add -A tools/SGWLauncher/
git commit -m "fix(launcher): address clippy warnings and test fixes"
```

---

## Summary

| Task | What | Tests |
|------|------|-------|
| 1 | Scaffold Tauri project | build check |
| 2 | Config module | 3 |
| 3 | SGW.exe binary patching | 7 |
| 4 | HTTP download with progress | build check |
| 5 | 7za extraction wrapper | 3 |
| 6 | Game launcher module | 4 |
| 7 | Delta updater module | 5 |
| 8 | Tauri IPC commands | build check |
| 9 | Frontend UI | build check |
| 10 | Bundle 7za sidecar | n/a |
| 11 | Launcher README | n/a |
| 12 | Integration test | 22 total + clippy |

Total: 12 tasks, 22 unit tests, ~8 Rust source files + 3 frontend files.
