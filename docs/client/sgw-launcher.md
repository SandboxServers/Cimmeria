# SGW Custom Launcher — Design Document

A replacement for CME's original `Launcher.exe`. The original launcher is dead (CME's SOAP patch server is offline), its installer (`SetupQA.exe`) cannot be automated, and we can do better. This document covers architecture, the install pipeline, the login redirect strategy, and all viable implementation tech stacks.

---

## What the Launcher Does

| Function | Notes |
|----------|-------|
| **First-run install** | Downloads client from archive.org, extracts RAR → CABs |
| **Login redirect** | Patches SGW.exe in place to point to our auth server |
| **Launch** | Spawns SGW.exe directly |
| **Future: delta updates** | Manifest-based patch system (separate infrastructure, not in scope here) |

**AteraLoader.exe is not used.** Its only player-relevant function was redirecting the hardcoded SOAP login endpoint (`www.stargateworlds.com`) in SGW.exe at runtime via DLL injection. We replace that with a one-time static byte-patch of the string in SGW.exe's `.rdata` section during install — simpler, no DLL injection required, no ASLR dependency.

---

## Client Source

The beta client is preserved on the Internet Archive:

```
https://archive.org/download/StargateWorlds_0.8348.1.4046/
  Stargate%20Worlds%20%280.8348.1.4046%29%20%282009-06-30%29%20%28beta%29.rar
```

| Property | Value |
|----------|-------|
| Version | 0.8348.1.4046 |
| Build date | 2009-06-30 (beta) |
| Container | RAR archive containing CAB installer files |
| Installer | `SetupQA.exe` — **cannot be silently automated** |

`SetupQA.exe` is bypassed entirely. The launcher extracts game files directly from the CAB files inside the RAR. 7-Zip handles both formats.

---

## Install Pipeline

```
1.  HEAD archive.org URL → read Content-Length for progress display
2.  Stream-download → %TEMP%\sgw-install\StargateWorlds.rar
    (emit progress events: bytes downloaded, speed, ETA)
3.  7za.exe e StargateWorlds.rar -o%TEMP%\sgw-install\extracted\ -y
    (emit progress events: filename being extracted)
4.  Glob extracted directory for *.cab files, sort by name
5.  For each .cab:
        7za.exe e <cab> -o<install_path>\ -y
        (emit progress: cab N of M, filename)
6.  Verify SGW.exe present at install_path
7.  Patch SGW.exe login URL (see below)
8.  Delete %TEMP%\sgw-install\
9.  Emit install-complete
```

7-Zip standalone (`7za.exe`, ~1MB) is bundled with the launcher. It handles both RAR extraction and CAB expansion, avoiding the need for system-level cabinet.dll invocation or the UnRAR SDK.

---

## SGW.exe Login URL Patch

CME's SOAP login hostname is hardcoded in SGW.exe's `.rdata` section. Ghidra analysis confirmed `/SGWLogin/UserAuth` at RVA `0x019cec40`; the full hostname `www.stargateworlds.com` (22 bytes) is in the same region.

**Patch approach:**

1. Open SGW.exe as a byte buffer
2. Search for the null-terminated UTF-8 sequence `www.stargateworlds.com`
3. Overwrite with the configured server hostname, zero-padded to the same 22-byte length
4. Write back — no PE checksum recalculation needed for `.rdata` edits

The replacement hostname must be ≤ 22 bytes, or the URL including scheme/path must be located and patched as a unit. Any reasonable emulator server address fits (e.g. `auth.example.com` = 16 bytes).

This patch is idempotent — the launcher checks whether the original CME string is still present before deciding whether to apply it. The target server address is user-configurable (defaults to the canonical Cimmeria emulator address when one is published).

**Why not hosts file?** Hosts file modification requires admin elevation and affects the whole system. Direct PE patching is scoped to the game directory and requires no elevated privileges.

**Why not DLL injection (AtreaRL)?** Runtime injection requires ASLR to be disabled on SGW.exe (because AtreaRL uses hardcoded patch addresses). The static `.rdata` patch has no such constraint — data section strings are not subject to ASLR address randomization.

---

## Launch Chain

```
SGWLauncher.exe
  └─ CreateProcess("SGW.exe")        ← direct, no intermediary
```

AteraLoader, AtreaRL, postinstall — none are required for the player-facing flow. They remain available as developer/RE tools in `docs/technical/`.

---

## UI States

| State | Condition | Controls |
|-------|-----------|----------|
| **Not installed** | SGW.exe absent | Install path picker, Download & Install button |
| **Downloading** | Download in progress | Progress bar (bytes + speed), cancel |
| **Extracting** | RAR/CAB in progress | Progress bar (file count + name) |
| **Ready** | SGW.exe present, URL patched | Play button, server address field |
| **Error** | Any failure | Error message, retry |

Server address is editable in the Ready state (re-patches SGW.exe on change).

---

## Tech Stack Options

All options below produce a self-contained Windows EXE distributable. Tradeoffs are around output size, dev speed, UI flexibility, and cross-platform potential.

---

### Option A — Tauri (Rust + WebView2)

**Recommended.**

| Property | Value |
|----------|-------|
| Output size | ~4–6 MB (uses system WebView2, present on Win10/11) |
| Language | Rust (backend) + HTML/CSS/JS (frontend) |
| Cross-platform | Windows, macOS, Linux |
| UI flexibility | Full web stack — CSS animations, any JS framework |
| Dev speed | Medium (Rust learning curve offset by excellent tooling) |

```
tools/SGWLauncher/
├── src-tauri/
│   ├── src/
│   │   ├── main.rs         — app entry, command registration
│   │   ├── download.rs     — streaming HTTP with progress events
│   │   ├── extract.rs      — 7za subprocess wrapper
│   │   ├── patch.rs        — SGW.exe byte-patch
│   │   └── launch.rs       — CreateProcess(SGW.exe)
│   ├── binaries/
│   │   └── 7za.exe         — Tauri sidecar (extracted at runtime)
│   └── tauri.conf.json
└── ui/
    ├── index.html
    ├── main.js             — tauri.invoke() + event listeners
    └── styles.css
```

**Key crates:** `tauri 2`, `reqwest` (async HTTP), `sha2`, `tokio`, `serde`

**Tauri commands** (Rust → JS bridge):
```rust
#[tauri::command] check_installation(path: String) -> InstallState
#[tauri::command] download_and_install(path: String, server: String, window: Window)
#[tauri::command] launch_game(path: String) -> Result<()>
#[tauri::command] get_default_install_path() -> String
#[tauri::command] browse_for_folder() -> Option<String>
```

**Progress events** (emitted from Rust to frontend via `window.emit`):
```
download-progress  { downloaded: u64, total: u64, percent: f32, speed_bps: u64 }
extract-progress   { phase: "rar"|"cab", current: u32, total: u32, name: String }
install-complete   {}
install-error      { message: String }
```

```powershell
# Dev
cargo install tauri-cli
cargo tauri dev

# Release
cargo tauri build --target x86_64-pc-windows-msvc
```

The name "Tauri" has an appreciated thematic resonance with the Tau'ri (the humans of Earth in Stargate lore).

---

### Option B — C# .NET 8 WinForms or WPF

| Property | Value |
|----------|-------|
| Output size | ~60–80 MB self-contained (bundles .NET runtime) or ~1 MB if runtime pre-installed |
| Language | C# |
| Cross-platform | Windows only (WinForms/WPF) |
| UI flexibility | Good (WPF) / adequate (WinForms) |
| Dev speed | Fast — rich BCL, HttpClient, async/await, ZipFile, Process |

Self-contained publish bundles the .NET 8 runtime:
```
dotnet publish -r win-x64 --self-contained -p:PublishSingleFile=true
```

Async HTTP download with progress is idiomatic in C#:
```csharp
using var response = await httpClient.GetAsync(url, HttpCompletionOption.ResponseHeadersRead);
await using var stream = await response.Content.ReadAsStreamAsync();
// read in chunks, report progress
```

WPF with MVVM gives a polished native feel. Good choice if the team is more comfortable with C# than Rust.

---

### Option C — Qt C++ (Widgets)

| Property | Value |
|----------|-------|
| Output size | ~15–25 MB (Qt DLLs) or statically linked ~8 MB |
| Language | C++ (same as rest of Cimmeria server codebase) |
| Cross-platform | Windows, macOS, Linux |
| UI flexibility | Good with Qt Designer / QML |
| Dev speed | Medium — verbose but consistent with project |

**Advantage:** Qt is already a project dependency (ServerEd uses Qt 5.x). The build system already knows how to link it. The team already has it installed.

**Key Qt classes:**
- `QNetworkAccessManager` — HTTP download
- `QProcess` — invoke 7za.exe and SGW.exe
- `QProgressBar` / `QProgressDialog` — progress UI
- `QFile::open` + byte search — SGW.exe patch

Fits naturally in `tools/SGWLauncher/` alongside `tools/ServerEd/`.

---

### Option D — Go + Fyne

| Property | Value |
|----------|-------|
| Output size | ~10–15 MB single binary |
| Language | Go |
| Cross-platform | Windows, macOS, Linux |
| UI flexibility | Limited (Fyne widgets are functional but not polished) |
| Dev speed | Fast — simple concurrency model, good stdlib |

Go's concurrency model (goroutines + channels) makes streaming download + progress reporting very clean. `net/http` handles downloads natively; `os/exec` runs 7za. No external runtime required.

Fyne's widget set is modest. If UI polish matters, this option trails Tauri and WPF.

```go
// Download with progress
resp, _ := http.Get(url)
defer resp.Body.Close()
buf := make([]byte, 65536)
for {
    n, err := resp.Body.Read(buf)
    written += int64(n)
    progress <- float64(written) / float64(total)
    // ...
}
```

---

### Option E — Python + PyQt6 / PyInstaller

| Property | Value |
|----------|-------|
| Output size | ~30–50 MB (PyInstaller bundle) |
| Language | Python 3.12+ |
| Cross-platform | Windows, macOS, Linux |
| UI flexibility | Good (Qt6 via PyQt6 or PySide6) |
| Dev speed | Fast for prototyping |

Viable if rapid iteration is the priority and final size doesn't matter. PyInstaller produces a single-directory or single-file bundle. `requests` handles HTTP; `subprocess` invokes 7za.

Not recommended for the final artifact — PyInstaller bundles are slow to start (~1–3 seconds on first launch) and antivirus false positives are common.

---

### Not Recommended

| Option | Reason |
|--------|--------|
| **Electron** | 150–200 MB for a launcher is unreasonable overhead |
| **Java / JavaFX** | No existing Java tooling in project; JRE dependency |
| **raw Win32 C++** | No gain over Qt given Qt is already present |
| **Flutter** | Dart + Skia for a Windows launcher adds unjustified complexity |

---

## Comparison Summary

| Option | Size | Speed | UI | Platform | Consistency |
|--------|------|-------|----|----------|-------------|
| **A — Tauri** | ~5 MB | Medium | Excellent | Win/Mac/Linux | Low (new lang) |
| **B — C# .NET 8** | ~70 MB | Fast | Good | Windows | Low |
| **C — Qt C++** | ~20 MB | Medium | Good | Win/Mac/Linux | **High** (already in project) |
| **D — Go + Fyne** | ~12 MB | Fast | Modest | Win/Mac/Linux | Low |
| **E — Python** | ~40 MB | Fast | Good | Win/Mac/Linux | Low |

**Recommendation:** Tauri for smallest output and best UI flexibility. Qt C++ if staying within existing project tooling is a priority.

---

## Future: Delta Patch System

Not in scope for the initial launcher but the architecture supports it. When ready:

- Launcher fetches a JSON manifest from our patch server listing files, sizes, and SHA-256 hashes
- Compares against local state
- Downloads only changed files (libcurl-style range-request resume built into all HTTP clients above)
- Extracts ZIPs to final paths
- Updates a local version file

The patch server can be a simple static file host (nginx/S3) serving the manifest JSON and ZIP archives. No SOAP, no proprietary format.
