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

## Ghidra Analysis: Confirmed Details

### PostInstall.exe Launch Mechanism

From live decompile of `InitInstance` at `0x00412530`. When the launcher was itself patched during the session and needs a restart:

```c
GetModuleFileNameW(NULL, path, 260)    // Get our own path
PathRemoveFileSpecW(path)              // Strip filename → directory
PathAppendW(path, L"postinstall.exe") // Build postinstall.exe path
LoadStringW(&cmdline, 0x85)           // Load cmdline format string (resource 0x85)
GetCurrentProcessId()                  // Get own PID — passed to postinstall
CStringT::Format(cmdline, ...)         // Format PID into cmdline
args = _wcsdup(cmdline)               // Dup for CreateProcessW
CreateProcessW(postinstall.exe, args, NULL, NULL, FALSE, 0, NULL, NULL, &si, &pi)
// Launcher does NOT wait — fires postinstall and continues to exit
CloseHandle(pi.hProcess); CloseHandle(pi.hThread)
```

The format string in resource `0x85` encodes the current PID. Postinstall kills the old launcher by that PID, applies the patch, then re-launches launcher.exe.

### Startup Log Sequence

Confirmed execution order from string analysis of `InitInstance`:

| Step | Level | Message |
|------|-------|---------|
| 1 | INFO | `"Creating update thread."` |
| 2 | FATAL | `"Unable to create update thread."` (error path) |
| 3 | INFO | `"Created update thread."` |
| 4 | INFO | `"Starting update thread."` |
| 5 | FATAL | `"Unable to start update thread."` (error path) |
| 6 | INFO | `"Started update thread."` |
| 7 | INFO | `"Starting Launcher Dialog Window."` |
| 8 | INFO | `"Closed Launcher's Dialog Window."` |
| 9 | INFO | `"Testing for launcher restart."` |
| 10a | INFO | `"Launcher restart not necessary."` |
| 10b | INFO | `"Launcher restart necessary."` |
| 11 | INFO | `"Exiting the Launcher."` |

### Unknown Early Exit Check (`FUN_00411280`)

The very first call inside `InitInstance`. When it returns `>= 0`, the launcher exits immediately — no UI is shown, no mutex is created, no threads are started. Only when it returns `< 0` does full initialization proceed.

This is NOT the single-instance mutex check (that's created later, inside the full-init block). Most likely: crash dump sentinel check or prior-session cleanup. Deserves further decompile work before implementing a mock launcher.

### `VerifyServerVersion` URL Construction (0x00409d10)

The SOAP endpoint URL is **not** a raw embedded string. It is assembled from Win32 string resources in a loop iterating over 5 resource IDs:

| Resource ID | Decimal |
|-------------|---------|
| `0x91` | 145 |
| `0x93` | 147 |
| `0x94` | 148 |
| `0x95` | 149 |
| `0x96` | 150 |

The segments are concatenated via `CStringT::Format`. To recover the full URL, extract string table block 10 from the `.rsrc` section (resource IDs 144–159 live in block 10). The assembled URL is logged before the SOAP call: `"Attempting to connect to URL: %s"`.

## Redirecting to a Custom Patch Server

How to make launcher.exe connect to our server instead of CME's.

### Step 1: Find the Actual SOAP Endpoint URL

The URL is assembled from Win32 string resources (IDs 145–150). To extract:

```
# On Windows — ResourceHacker:
ResourceHacker.exe → Open launcher.exe → String Table → block 10

# Alternatively: run the binary and watch the log for:
"Attempting to connect to URL: <url>"
```

The log line is emitted before the SOAP connection attempt, so even a failed connection reveals the URL.

### Step 2: Choose a Redirect Method

| Method | Effort | Pros | Cons |
|--------|--------|------|------|
| **Hosts file** | Trivial | No binary patching needed | Only works if URL uses a DNS hostname, not an IP |
| **Binary patch .rsrc** | Low | Clean and permanent | Must recalculate PE checksum after edit |
| **Local DNS server** | Medium | Transparent to binary | Infrastructure overhead |
| **HTTP proxy** | Medium | No binary modification | libcurl respects `http_proxy` env var |

For development: hosts file is fastest. For distribution: binary-patch the `.rsrc` string resources.

### Step 3: Understand the Patch Format

**Patches are complete files, not binary diffs.**

Evidence from the binary string table:
```
"Attempting to move temporary patch file (%s) to final destination (%s)."
"Successfully moved temporary patch file to destination."
```

The launcher downloads each file via libcurl to a temp path, then moves it into place. There is no diff application step.

The "content diff" strings (`"Failed to write updated file as content diff"`, etc.) describe the **client-side scanning phase**: the launcher hashes local files, computes which ones differ from the expected state, and sends that inventory to the SOAP server. The server responds with download URLs for only the files that need replacing.

Files are hosted on a plain HTTP server as ZIPs or raw files. No proprietary format required.

### Step 4: Build the SOAP Patch Server

SOAP namespace (confirmed from binary): `http://schemas.xmlsoap.org/soap/envelope/`

**Mode A — Pass-through (client already has current files):**
```xml
<VERSIONINFO>
  <VERSIONS><VERSION>...</VERSION></VERSIONS>
  <PATCHES></PATCHES>   <!-- empty PATCHES = result 1 = "current" -->
</VERSIONINFO>
```

**Mode B — Deliver or update game files:**
```xml
<VERSIONINFO>
  <PATCHES>
    <PATCHENTRY>
      <FILEENTRY>
        <url>http://our-server/SGWGame-1.0.zip</url>
        <hash>sha256_here</hash>
        <destination>..\SGWGame\</destination>
      </FILEENTRY>
    </PATCHENTRY>
  </PATCHES>
</VERSIONINFO>
```

The launcher downloads the ZIP via libcurl (with range-request resume), verifies the hash, extracts via ZipArchive, and moves files to the destination path.

> **Note:** Exact XML element names inside `PATCHENTRY`/`FILEENTRY` need confirmation — the type names come from the binary string table, but the literal XML tag names require either a packet capture against the original server or a decompile of `CGetPatchProcess::ProcessResponse`. Simplest path: run the binary against a mocked server and capture the exchange with Wireshark or Fiddler.

Any HTTP framework works server-side. A Python script using `lxml` to serve hand-crafted SOAP envelope XML is sufficient — no gSOAP installation needed server-side.

## Client Distribution

**launcher.exe CAN be used for client distribution** — and it already has everything needed built in.

### How It Works

The update flow is not binary-diff based. The launcher:

1. Hashes local files and sends the inventory to the SOAP server
2. Receives `PATCHINFO` containing download URLs for files that need replacing
3. Downloads each file via libcurl to a temp path
4. Moves the temp file to its final destination

On a fresh install with no local files, step 1 produces an empty inventory, and step 2 returns the full file list. The launcher then downloads and installs everything. This is a complete distribution mechanism with no special "first-run" mode needed.

### How to Use It for Our Distribution

1. **Host game files** on any HTTP server (nginx, S3, etc.) as ZIP archives or individual files
2. **SOAP server** returns `PATCHINFO` with download URLs, expected hashes, and destination paths relative to the game directory (`"..\SGWGame"`)
3. **Redirect launcher** to our SOAP endpoint via hosts file or binary patch of `.rsrc` string resources 145–150
4. **First run**: SOAP server returns result=2 (outdated) with the full file list → launcher downloads everything
5. **Subsequent runs**: SOAP server returns result=1 (current) → launcher passes through instantly

This gives us a free auto-update system with:
- Progress bars (download + install, separate)
- Hash verification per file
- Resume support (libcurl range requests)
- News/content browser (embedded IE WebBrowser control, loads local HTML or falls back to URL)

### What launcher.exe Does NOT Do

- **Does NOT launch SGW.exe** — the launcher has no knowledge of the game executable
- **No authentication** — no login, password, AES, tokens, or session management
- **No BigWorld connection** — no LoginApp, BaseApp, CellApp, or Mercury references
- **No config files** — no `.ini`/`.cfg`/`.config` reading
- **No registry access** — user data goes to `My Documents\My Games\Firesky`

After the launcher exits, the player still needs to run `AteraLoader.exe → SGW.exe`. Standard approach: the launcher's Play button invokes a wrapper script or shortcut that runs AteraLoader after launcher exits.

### Other Notes

- **`P_rstinstall`** (string at `0x0050f072`, adjacent to `"postinstall.exe"`): likely a postinstall operation name, not a patch type
- **Crash reporting** (`cheyenneme.com`): long dead — crash uploads will silently fail, no action needed
- **User data path `My Games\Firesky`**: confirms CME's internal project codename for SGW was "Firesky"
- **The launcher is optional**: since it doesn't launch the game, AteraLoader.exe or direct SGW.exe invocation bypasses it entirely — useful for development
