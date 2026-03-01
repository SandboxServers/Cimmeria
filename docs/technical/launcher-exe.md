# Launcher.exe — Patch Client

Analysis of the official CME (Cheyenne Mountain Entertainment) game launcher for Stargate Worlds. Despite the name, this is purely a **patch/update client** — it does NOT launch SGW.exe or handle authentication.

## Overview

| Property | Value |
|----------|-------|
| **Type** | Win32 GUI (32-bit, MFC 8.0 Unicode) |
| **Size** | ~1.3 MB |
| **Compiler** | MSVC 2008 (VC80) |
| **Framework** | MFC (CWinApp/CDialog) |
| **Build System** | Perforce (`F:\perforce3\SGW\Common\`) |
| **Build Date** | Jun 30, 2009 |

## Statically Linked Libraries

| Library | Version | Purpose |
|---------|---------|---------|
| **libcurl** | 7.17.0 | HTTP/HTTPS client (patch downloads, crash uploads) |
| **OpenSSL** | 0.9.8g (Oct 2007) | TLS/SSL for HTTPS |
| **gSOAP** | 2.7 | SOAP XML client for patch server protocol |
| **ZipArchive** | (Artpol Software) | ZIP extraction for patches |

## DLL Imports

- **MFC80U.DLL** / **MSVCR80.DLL** / **MSVCP80.DLL** — MFC + C/C++ runtime
- **TBBMALLOC.DLL** — Intel TBB scalable allocator
- **WS2_32.DLL** — Winsock 2
- **VERSION.DLL** — `GetFileVersionInfoW` for reading exe versions
- **OLE32.DLL** / **OLEAUT32.DLL** — COM for embedded IE WebBrowser control
- **SHLWAPI.DLL** / **SHELL32.DLL** — Shell paths, `SHGetFolderPathW`, `SHCreateDirectoryExW`
- Standard Win32: KERNEL32, USER32, GDI32, ADVAPI32, COMCTL32, WINMM

## Exports

53 `curl_*` functions — the full libcurl 7.17.0 public API. Launcher.exe doubles as a curl DLL for other CME components.

## RTTI Class Hierarchy

```
CLauncherApp (CWinApp)              — Main application
CLauncherDlg (CDialog)              — Main dialog window
CBrowser                            — Embedded IE WebBrowser wrapper
CBrowserSink                        — COM event sink for browser
CHTMLDocEvent                       — HTML DOM event handler (button clicks)
CBaseProcess                        — Base class for threaded operations
  CGetPatchProcess                  — Downloads patch list from server
  CInstallPatchProcess              — Applies downloaded patches
  CUpdateLocalVersionProcess        — Version check + orchestration thread
CProgressSink                       — Progress bar update interface
CThreadProgressSink                 — Thread-safe progress reporting
CZipArchive                         — ZIP file reader
  CZipFile / CZipFileHeader / CZipCentralDir / CZipStorage
  CZipCompressor → CDeflateCompressor
LogData / LogFile / LogThread       — Structured logging system
crash::CrashReport                  — Crash dump collection
crashapp::CrashFileEntry / CrashFileList — Crash file management
IPatchEnumeratorSink                — Patch enumeration callback
_PatchGenEnum                       — Patch difference enumerator
SPatchFileNode                      — Patch file data (CList)
CME::Exception / CME::ExceptionImpl — Framework exceptions
```

## Complete Launcher Workflow

### Phase 1: Startup (`CLauncherApp::InitInstance` at 0x00412530)

1. **Path setup** (`FUN_00412080`):
   - Gets launcher.exe directory → `DAT_0052e970`
   - Gets `My Documents` via `SHGetFolderPathW(CSIDL_PERSONAL)`
   - Appends `\My Games\Firesky` → `DAT_0052e96c`
   - Creates user data directory if needed

2. **Logging init** (`FUN_0040f1a0`): LogFile/LogThread system

3. **Single-instance mutex** (`DAT_0052e918`): Prevents multiple launchers

4. **Crash upload**: Checks for `.upload.sentinel` and `.manifest.xml` in crash archives. If found, uploads via SOAP/CURL to:
   - `http://www.cheyenneme.com/common/win_crash_report` (initial report)
   - `xml.crashupload.sgw.cheyenneme.com` (file upload)

5. **Version check thread** (`CUpdateLocalVersionProcess::Init` at 0x00406690): Starts background update

6. **Show dialog** (`CLauncherDlg`, resource ID 102)

### Phase 2: Dialog Window

**UI Layout:**
- Embedded IE WebBrowser (news/content from local HTML file)
- Play button (ID 0x409) — enables when update completes
- Options button (ID 0x40A) — stub
- Download progress bar
- Install progress bar
- Window title: "Stargate Worlds"

**Embedded Browser:**
- Loads a local HTML file (filename from string resource 0xA9)
- Path: `<CWD>\<resource_filename>`
- Falls back to a remote URL if file not found
- `CHTMLDocEvent` attaches click handlers to HTML DOM elements

**Play Button** (`FUN_00416190`): Checks if update thread is done. If not: "The Play button will enable once the update process is finished."

### Phase 3: Background Version Check (`FUN_00406870`)

1. **`VerifyServerVersion`** (`FUN_00409d10`):
   - Constructs SOAP endpoint URL from string resources
   - Sends SOAP request via gSOAP + CURL
   - Logs: "Attempting to connect to URL: %s"
   - On success: "Server has returned %d messages."

2. **Check result** (`FUN_0040ac40`):
   - **1**: Up-to-date ("none"), no patches needed
   - **2**: Patches available, version is out of date
   - **3**: Connection failure

3. **On connection failure**: MessageBox with retry option

4. **On patches available**: Two-phase download:
   - **Phase A**: "Attempt Launcher Patch Downloads." — patches to Launcher.exe/postinstall.exe
   - **Phase B**: "Attempting Unreal Patch Downloads." — patches to SGW.exe and game content

5. **Patch verification** per file:
   - Check if file already exists locally
   - Generate local hash, compare with server hash
   - If match: "Patch is verified." — skip download
   - If mismatch/missing: download via CURL with resume support
   - Downloads go to temp file, then moved to final destination

6. **Binary version interrogation**:
   - `GetFileVersionInfoW` on SGW.exe → logs SKU + Version
   - `GetFileVersionInfoW` on postinstall.exe → logs SKU + Version
   - Fallback: "Unable to get access to the Post-Installer! Defaulting it to version 0.0.0.0"

### Phase 4: Post-Dialog

- **Play clicked (result=1)**: Checks if launcher restart is needed
  - If launcher was patched: launches `postinstall.exe` with current PID (to apply patch + re-launch)
  - If no restart needed: processes pending installation results
  - **Does NOT launch SGW.exe**
- **Aborted (result=3)**: User cancelled
- **Unexpected close**: Logged

## SOAP Patch Protocol

### Request Types
| XML Type | Fields | Purpose |
|----------|--------|---------|
| `VERSIONDATA` | userName, sku, buildConfig, version, timeStamp, hash | Client version info |
| `REQVERSIONDATA` | (wrapper) | Version check request |
| `REQPATCHINFO` | (wrapper) | Patch list request |
| `REQVERSIONINFO` | (wrapper) | Version info request |
| `REQPATCH` | (wrapper) | Individual patch request |

### Response Types
| XML Type | Purpose |
|----------|---------|
| `VERSIONINFO` | Server version response |
| `VERSION` / `NEWVERSION` / `DELVERSION` | Version numbers/deltas |
| `VERSIONS` | Version collection |
| `PATCHINFO` / `PATCH` / `NEWPATCH` / `DELPATCH` | Patch metadata/deltas |
| `PATCHES` | Patch collection |
| `PATCHENTRY` | Download location |
| `FILEENTRY` | File within patch |

### Crash Report Types
| XML Type | Fields | Purpose |
|----------|--------|---------|
| `crashapp:CrashFileEntry` | file data | Individual crash file |
| `CRASHFILES` | CRASH_ID, FILE[] | Crash collection |

**Crash upload fields**: machine, user, client_version, crash_hash, crash_timestamp, crash_id, summary_txt, zip_file

## Key Addresses

| Address | Function | Purpose |
|---------|----------|---------|
| 0x004ce335 | `__tmainCRTStartup` | CRT entry → AfxWinMain |
| 0x00412530 | `CLauncherApp::InitInstance` | Main launcher function |
| 0x00412080 | `SetupPaths` | Exe dir + user data path |
| 0x004141a0 | `CLauncherDlg::CLauncherDlg` | Dialog constructor |
| 0x004150b0 | `OnInitDialog` | Browser setup, HTML navigation |
| 0x00416190 | `OnPlayClick` | Play button handler |
| 0x00416b70 | `SetWindowTitle` | Sets "Stargate Worlds" |
| 0x00411a30 | `GetGameDir` | Returns `"..\SGWGame"` |
| 0x00406690 | `CUpdateLocalVersionProcess::Init` | Version check init |
| 0x00406870 | `UpdateThread::Run` | Main update orchestrator |
| 0x00409d10 | `VerifyServerVersion` | SOAP version check |
| 0x00407e60 | `PatchOrchestrator` | Patch download/verify |
| 0x0040ac40 | `CompareVersion` | 1=current, 2=outdated, 3=error |
| 0x0043bac0 | `CrashReportUpload` | Crash archive + SOAP upload |
| 0x004442d0 | `VERSIONDATA_serialize` | gSOAP serializer |

## Source Files Referenced

| Source Path | Purpose |
|-------------|---------|
| `.\Launcher.cpp` | Main app class |
| `.\LauncherDlg.cpp` | Dialog UI handlers |
| `.\Patch\BaseProcess.cpp` | Thread management base |
| `.\Patch\GetPatchProcess.cpp` | Patch list download |
| `.\Patch\InstallPatchProcess.cpp` | Patch installation |
| `.\Patch\UpdateLocalVersionProcess.cpp` | Version check orchestrator |
| `.\PatchDownload.cpp` | CURL download logic |
| `.\VerifyServerVersion.cpp` | SOAP version check |
| `.\CrashDump.cpp` | Crash dump collection |
| `.\Logger\LogCURL.cpp` | CURL logging bridge |

## What Launcher.exe Does NOT Do

- **Does NOT launch SGW.exe** — the game client is launched separately
- **No authentication** — no login, password, AES, tokens, or session management
- **No BigWorld connection** — no LoginApp, BaseApp, CellApp, Mercury references
- **No config files** — no .ini/.cfg/.config reading
- **No registry access** — user data goes to `My Documents\My Games\Firesky`
- **No game-specific logic** — purely version checking, patching, and crash reporting

## Emulator Implications

1. **Patch server is needed for official flow**: The launcher expects a SOAP-based patch server. For emulator use, either:
   - Run AteraLoader.exe directly (bypasses the launcher entirely)
   - Stand up a mock SOAP patch server that returns "version current" (result=1)
2. **Crash reporting endpoint**: `cheyenneme.com` is long dead — crash uploads will silently fail
3. **User data path `My Games\Firesky`**: The "Firesky" name confirms this was CME's internal project codename for SGW
4. **postinstall.exe handles restarts**: After launcher self-patching, postinstall.exe kills the old launcher (by PID) and re-launches it
5. **The launcher is optional**: Since it doesn't launch the game, it's only needed for patching — AteraLoader.exe or direct SGW.exe launch bypasses it entirely
