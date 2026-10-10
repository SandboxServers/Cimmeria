# Two SGW.exe clients on one workstation

> **Last updated**: 2026-10-10
> **Audience**: lab operators and engineers who run several lab clients, or a two-player scenario, from one machine
> **Type**: RE finding (static client analysis, lab code audit, headless DLL tests, live two- to five-client runs) with a fix
> **Confidence**: HIGH. The lab-side collisions are proven in code and by test; the cache lock, the resync it causes and the per-instance `USERPROFILE` fix were measured on live clients on 2026-10-10 (#1312). The named `ZipArchive Mapping File` and the focus throttle are still Ghidra evidence only.
> **Companion docs**: [live-research-lab.md, Parallel clients](../../guides/live-research-lab.md#parallel-clients-up-to-five) (runbook), [live-research-lab ADR §12](../../architecture/live-research-lab.md#12-addendum-2026-10-10-one-daemon-many-instances-a-lease-per-instance) (design), [lab-parallel-clients ledger](../../analysis/lab-parallel-clients/README.md), [cooked-data-pipeline.md](cooked-data-pipeline.md)

## Summary

The client has **no single-instance guard**. A second `SGW.exe` starts fine; what goes wrong is everything around it. The lab was written for one client, so a second one collided with the first on the session file, the bridge port, the credentials, the crash markers and the logs, and the supervisor refused to start it at all. Those were fixed on 2026-09-29 with named lab instances.

The freeze that remained is the client's own. **A running client locks the whole cooked-data cache** (`Documents\My Games\Firesky\SGWGame\Cache.en-US\*.pak`) for its lifetime, so a second client on the same Documents folder runs with no cache: it reports cooked version 0 for every category and the server answers each of its logins with a full resync of about 59,000 entries, which pins its main thread for tens of seconds. The fix is lab-only: every lab client launches with its own `USERPROFILE`, which moves its whole Firesky folder into a seeded per-instance profile. Four lab clients then logged in and entered the world at once with zero cache errors (2026-10-10).

The first version of this document (2026-09-29) was written while live runs were paused, so its client-side claims were Ghidra evidence. The [live results](#live-results-2026-10-10) section records what the 2026-10-10 runs confirmed; what they didn't cover is marked.

## Plain-language version

Think of the lab as a remote control that is wired to exactly one television. Buy a second television and the remote still talks to the first one, both remotes fight over the same battery cover, and the shop assistant refuses to sell you the second television because "one is already running". The fix gives each television its own remote, its own channel list and its own name.

Independently of the remote, the two games themselves also share a filing cabinet (the cached game data on disk), and the game deliberately slows itself down when you are not looking at its window.

## Inventory of shared resources and single-instance assumptions

### The client (Ghidra, `SGW.exe`, image base `0x00400000`, no ASLR on disk)

| Resource | Evidence | Two-client consequence | Class |
|---|---|---|---|
| Single-instance mutex or window check | Every `CreateMutexW/A`, `CreateSemaphoreA` and `CreateEventW/A` call site inside a decompiled function takes a `NULL` name: `0x00a67780`, `0x0045f450`, `0x00460380`, `0x004667b0`, `0x00a35fd0` (Mutex W), `0x00453a00`, `0x00455f80`, `0x013b1180` (Mutex A / Semaphore A). Five more `CreateMutexW` references (`0x0179a9d3`, `0x0179aa26`, `0x0179aa73`, `0x017b80d6`, `0x017ba866`) sit outside any function Ghidra recovered and were not decompiled. No `FindWindow` import (only the wx member `FindWindow`), no `OpenMutex` import, no "already running" or "multiinstance" string. | None: a second client starts. | none needed |
| `OpenEventW` | One caller, `0x004cc970`, opens an event whose name is built from `GetCurrentProcessId()` (`FUN_00490e10`), so it is per process. | None. | none needed |
| `SGW.lock`, `SGW.lock~` in `binaries\` | The files are Ghidra project lock files (`#Ghidra Lock File`, `SGW.gpr`, `SGW.rep` beside them), not written by the client. No `.lock` string exists in `SGW.exe`. | None. Ignore them. | none needed |
| Named file mapping `ZipArchive Mapping File` | `FUN_0139ede0` (`0x0139ede0`) calls `CreateFileMappingW(hFile, NULL, PAGE_READWRITE, 0, 0, L"ZipArchive Mapping File")`, then `MapViewOfFile`. Its only caller `FUN_0139f460` compacts a ZipArchive central directory in place with `memmove` over the mapped view (called twice from `FUN_0139fe60`). | **A named mapping is machine-wide.** If two clients are inside this routine at once, the second `CreateFileMappingW` returns the first client's mapping object (the `hFile` argument is ignored when the name exists), so the second client compacts the wrong `.pak`. The window is the length of one compaction. Inferred, not reproduced. | hypothesis, needs live repro |
| Cooked-data cache `Documents\...\SGWGame\Cache.en-US\*.pak` | 22 `.pak` files, all rewritten at every login by the server's version push (timestamps equal the launch time). They are ZipArchive files (`ZipStorageBase::OpenArchive` `0x00479340`, `WriteStreamToFile` `0x00479930`, [cooked-data-pipeline.md](cooked-data-pipeline.md)). The archive open goes through `ZipPlatform` `FUN_0139d680`, which maps its share argument onto `_wsopen_s` share flags (`_SH_DENYRW`, `_SH_DENYWR` or `_SH_DENYNO`). | Two clients write the same archives. Which share flag the cache archives use decides between "second client fails to open and logs `Error opening static cache archive`" and "both write". Not determined statically. **Live, 2026-10-10: the first is true.** The first client holds all 22 open `GENERIC_READ\|GENERIC_WRITE` with read-only sharing; the second cannot open any and gets a full resync at every login ([live results](#live-results-2026-10-10)). | **root cause of the freeze; fixed in lab** (per-instance `USERPROFILE`) |
| UE3 file writers | `FFileManagerWindows` `CreateFileWriter` (`0x004c6090`): share mode is `FILE_SHARE_READ` only when the caller asked for `FILEWRITE_AllowRead`, else `0` (exclusive). | Any file a client holds open for write through this path is unwritable by the other client until it closes. | needs live check |
| `SGWDebugLog.log` in `binaries\` | `SGWLogConfig.xml`: log4cxx `FileAppender`, `append=true`, relative name. 6 MB and growing. | Both clients append to one file, lines interleave. Harmless to the game; unreadable for evidence. | tooling |
| `SavedSystemOptions.xml`, `WindowStates.xml`, root `* - Saved Vars.lua` | Written by the client under `SGWGame\`; the per-character `Saved Vars` live in `<account>\<character>\`. | Last writer wins for the shared ones. Per-character files are separate as long as the two clients use different accounts. Same account: they share one directory. | tolerated |
| `Content\LocalShaderCache-*.upk`, `LocalTerrainMaterialCache.upk` | UE3 local caches, rewritten when new shaders compile. | Concurrent writes are possible. Not observed. | needs live check |
| `Stats\<map>.log`, `CrashDumps\` | Per-map stats logs, and the client's dump folder. | Two clients in one map append/overwrite one stats file. Minor. | tolerated |
| The UDP socket | Mercury binds through `FUN_01584870` (`0x01584870`), which tries ports `min..max` in order until `bind` succeeds; the range is the nub's `+0x2c/+0x2e` (`Mercury_Nub_9` `0x01577b80`). Off-range, a fixed port would fail, but the loop steps to the next port. | A second client cannot fail on a busy local port unless the range is exhausted. Whether `min=max=0` (ephemeral) is what the shipped client uses is not confirmed. | needs live check (`netstat`) |
| Audio | FMOD Ex (`fmodex.dll`), Bink on DirectSound. Both clients on the lab account have `volume=0`, `musicvolume=0`. | Shared mixer only; silent in the lab. | none needed |
| GPU | Direct3D 9 windowed (`windowedMode=true`, `StartupFullscreen=False`). | Two windowed D3D9 devices coexist. **Exclusive fullscreen would not**; the lab options are windowed. | none needed |
| Focus throttle | `FEngineLoop::Tick` (`0x00417100`): after the message pump it compares the process id owning `GetForegroundWindow()` to its own. Not foreground: the thread priority goes to `THREAD_PRIORITY_BELOW_NORMAL` and every tick calls `FUN_00491320(0.005)`, a 5 ms `Sleep` (`0x017f9004` = 0.005f, `0x01805f30` = 1000.0). The flag `DAT_01ead7ac` gating it is the engine-initialised flag, set once. | **A client whose window is not in the foreground runs slower** (lower priority, at most about 200 ticks per second, less under load). Not a pause. This is the most likely reading of "odd behavior" for a human running two windows. | fix in lab (virtual focus) |
| D3D reset on focus loss | `0x00ec4c90`: if `GetFocus()` is `NULL` or the window is iconic, the device-reset path returns without acting. | A minimised second window never resets its device. | none needed |
| DirectInput | Created and read through `DINPUT8` (`DirectInput8Create` IAT `0x017ef024`); real input reaches only the foreground window. | Real keyboard and mouse go to whichever window is in front. Lab input is per process (see below). | none needed |

### The lab (code audit)

| Assumption | Where | Effect on a second client |
|---|---|---|
| One `current-session.json` | `session_file::current_session_path`; the DLL reads `<Binaries>\sessions\current-session.json` at boot | Two launches rewrite one file: both DLLs get the same port and token, or the second overwrites the first before it has read it. |
| Bridge port 8770 | `LabConfig` default, `CIMMERIA_LAB_BRIDGE` default | The second DLL cannot bind. Reproduced: `lab_second_client_on_the_same_session_cannot_bind` in `crates/sgw-testhost/tests/dll_boot.rs`. |
| One supervisor pid | `SupervisorState.pid`, `running_sgw_pids` guard in `Supervisor::start` | `lab_client_start` refused while any `SGW.exe` ran. |
| One credentials file | `sessions\lab-account.json` | A second client would log into the same account, and the server evicts the older session (`crates/base/src/base/login/eviction.rs`, `duplicate_login`). |
| One crash marker and minidump set | The DLL writes beside the session file | Two clients overwrite each other's evidence. |
| Two DLL logs | `cimmeria-client-telemetry.log`, `cimmeria-client-patches.log`, both `File::create` in `binaries\` | Each client truncates the other's log. |
| Named hook lock | `client-hookgate` `hook_lock_name(pid)` | Per process: fine. |
| Bridge auth | One 64-hex token per launch | Per launch: fine, and a wrong-port connect is refused, not misrouted. |
| Screenshots, input | `PrintWindow` on the window found by pid; input state and virtual focus are process statics in the DLL | Per process: fine. |
| Telemetry mint | One `grant_for_launch` per launch, own `session_id` | Fine: each client is its own SigNoz session. |

## Root causes, ranked by how likely they explain "odd behavior"

The 2026-09-29 ranking put the shared cache last, as unconfirmed. The live runs moved it to the top.

1. **The cooked-data cache lock** (confirmed live, 2026-10-10). The second client on one Documents folder cannot open any `Cache.en-US\*.pak`, reports cooked version 0 for all 21 categories, and gets a full resync at every login: a 20 to 40 s hang on loopback, minutes over the WAN. Its writes fail too, so the next login starts from version 0 again. This is "the client froze".
2. **Same account logged in twice.** The server evicts the older session on a duplicate login. The first client is logged off and looks like it crashed. Expected, and the reason a second player needs a second account.
3. **Background throttling.** A window not in the foreground runs at lower priority with a 5 ms sleep per tick (`0x00417100`). Two windows side by side always leave one throttled. Symptoms: slower animation and Mercury processing in the client you are not looking at. Not measured live (LT-6).
4. **Lab collisions.** Proven in code and by test; see the table above. Fixed.
5. **The rest of the shared writable state** (the named `ZipArchive Mapping File`, the shader cache). The shader cache and `Config\*.ini` are not held open (live check); the named mapping is still a hypothesis (LT-5).

## What was fixed (lab-only, no client patch)

### 2026-09-29: named lab instances

Named lab instances, in [`crates/lab/src/supervisor/instance.rs`](../../../crates/lab/src/supervisor/instance.rs). A second `cimmeria-lab` MCP server entry with `CIMMERIA_LAB_INSTANCE=p2` drives a second client:

- **Session file**: `sessions\instances\<name>\current-session.json`, found by the DLL through `CIMMERIA_LAB_SESSION_FILE`, which the supervisor passes through `sgw-start32` (`start32::run_with_env`). `CreateProcessW` is called with a null environment block (`crates/client-launch/src/process.rs`), so the game inherits it. No client patch and no change to the default layout.
- **Bridge port**: `CIMMERIA_LAB_BRIDGE_PORT` per instance; `CIMMERIA_LAB_BRIDGE` now defaults to that port.
- **Crash marker and minidumps**: in the instance directory (the DLL already used the session file's directory); `lab_crash_report` reads it.
- **Credentials**: `sessions\lab-account.<name>.json`. A named instance never falls back to `lab-account.json`, because that would log both clients into one account.
- **Logs**: `cimmeria-client-telemetry-<name>.log` and `cimmeria-client-patches-<name>.log`.
- **Start guard**: refuses only for an `SGW.exe` that is not another lab instance's, for a bridge port a peer already uses, and past `CIMMERIA_LAB_MAX_CLIENTS` (default: the number of hosted instances, at least 2; ceiling 5 since 2026-10-10, was 4). Peers are found through `lab-instance.json` files.
- **Telemetry**: the session carries an `instance:<name>` tag.

The default instance (no `CIMMERIA_LAB_INSTANCE`) is byte-for-byte the old layout.

Tests: 14 unit tests in `instance.rs` (name validation, paths, environment, the guard rules including a peer still launching and a port clash, registry round trip), `write_session_at`, the DLL's `resolve_session_path` and `sanitize_instance`, log file naming in both DLLs, and two boot tests in `sgw-testhost` that run two real lab-bridge DLL instances in one install directory. Reverting the DLL's session override makes `two_lab_instances_in_one_install_each_get_their_own_bridge_and_log` fail (checked).

### 2026-10-10: a Firesky folder per client, one daemon for all of them

The [lab parallel clients campaign](../../analysis/lab-parallel-clients/README.md) (#1312):

- **Per-instance `USERPROFILE`** ([`instance_profile.rs`](../../../crates/lab/src/supervisor/instance_profile.rs)). `SGW.exe` resolves My Documents in exactly one place, `SHGetFolderPathW(CSIDL_PERSONAL)` (`0x004c6333`); its only other folder lookup is `CSIDL_LOCAL_APPDATA` (`0x004935ad`). It imports no `SHGetKnownFolderPath` and has no `USERPROFILE` or user-dir string. Windows expands the default `Personal` value `%USERPROFILE%\Documents` with the calling process's own `USERPROFILE`, so every lab client, the default one included, launches with `USERPROFILE` at `Binaries\sessions\instances\<label>\profile`. The profile's `SGWGame` is seeded once from the real one (top-level files, `Config`, `Content`, `Cache.en-US`; never the per-account folders, `Logs`, `CrashDumps` or `Stats`), so the first login finds a warm cache. `CIMMERIA_LAB_SHARED_USER_DIR=1` opts out. A `Personal` value that is not `%USERPROFILE%`-relative (OneDrive Known Folder Move, a policy), a failed registry read or a failed seed falls back to the shared folder with a `user_dir_shared` warning.
- **One daemon hosts every instance** (`CIMMERIA_LAB_INSTANCES`, at most five), each with its own supervisor, watchdog, bridge port (base + position) and lease book. Calls route by the `instance` argument (label or account name), else by the lease id, else to the first instance.
- **Client cap** raised to the five seeded lab accounts (`CEILING_MAX_CLIENTS = 5`).
- **Watchdog boot grace**: 90 s before a client that has never answered its bridge can be killed.

The durable fix for the OneDrive case is a `SHGetFolderPathW` hook in the lab DLL (proposal 3 on #1312), on the [tooling backlog](../../analysis/lab-automation/tooling-backlog.md#backlog-lab-bridge-and-supervisor-for-the-session-that-owns-them).

## Needs an owner decision

1. **A `lab2` account.** *Done 2026-09-29: `lab2` to `lab5` are seeded (ids 11 to 14) and muted in Discord. `tools/lab/instances.ps1 init` writes their account files.* Before that only `lab` existed, and a duplicate login evicts the first client, so a true two-player run needs a second account. Proposed, not committed (seeds ship with a release, and the account list is the owner's call):

   ```sql
   -- db/sgw/Accounts/Seed/account.sql, after account_id 10
   INSERT INTO account (account_id, account_name, password, accesslevel, enabled) VALUES (11, 'lab2', 'a94a8fe5ccb19ba61c4c0873d391e987982fbbd3', 2, true);
   -- and: SELECT pg_catalog.setval('accounts_account_id_seq', 11, true);
   ```

   Same password scheme as `lab` (SHA-1 of `test`). Also add `"lab2"` to `muted_accounts` in `docker/compose.discord.yml` and the example in `config/discord.toml.example`, or its play floods the Discord channels. Both seeds reach the colo only with a release.
2. **A client patch is not proposed.** There is no single-instance check to neutralise. The one candidate for a patch, the named `ZipArchive Mapping File` mapping, is a hypothesis; it would need the live repro first and a maintainer decision (new client patch).

## Recommended way to run two-player scenarios

| Need | Use |
|---|---|
| Trade, squads and teams, mail, chat, visibility, where the second player only needs to act | Two (up to five) full clients: one daemon hosting `default,p2,...`, one account and one lease per client. See [the guide](../../guides/live-research-lab.md#parallel-clients-up-to-five). |
| A second player who only needs to exist and answer (duel partner, someone to be visible) | `sparbot` (`crates/wireclient`), headless, no second window, no throttling, no shared cache. It also needs its own account. |
| Anything that only needs bytes on the wire | `crates/wireclient` `GameSession` with `cell_method` and `base_method`, as the two-client visibility and social tests do. |

Use a second client when the UI of the second player is part of what is being tested. Use `wireclient` when it is not: it cannot hit any of the client-side shared state above.

## Live test plan (2026-09-29)

The plan as written before any live run. What happened to each step is in [Live results](#live-results-2026-10-10).

Run only two `SGW.exe`, each through its own named instance, on two different accounts unless the step says otherwise. Read `SGWDebugLog.log` and the per-instance DLL logs afterwards.

| Id | Step | Confirms or refutes |
|---|---|---|
| LT-1 | Start `p1`, then `p2` with the fixed supervisor; `lab_client_status` on each. | The instance fix end to end: two pids, two ports, two session files. |
| LT-2 | While both are at the login screen, `netstat -ano -p udp` after each reaches server select. | Whether the Mercury bind is ephemeral or in a range (`0x01577b80`). |
| LT-3 | Log both in at the same moment (same map), then `Get-ChildItem Cache.en-US` timestamps and `SGWDebugLog.log` for `Error opening static cache archive` and `ZipStorage` lines. | Whether concurrent cache writes fail, succeed, or corrupt. |
| LT-4 | Repeat LT-3 five times with `Process Monitor` filtered to `Cache.en-US`, looking at the share mode of each `CreateFile` and any `SHARING VIOLATION`. | The share flags `FUN_0139d680` produced. |
| LT-5 | Force an entry replacement on both at once (log in twice while a server version bump is pending) and check the `.pak` files open cleanly (`unzip -t`). | The `ZipArchive Mapping File` hypothesis. |
| LT-6 | Leave `p2` un-focused without `client_input_focus`, and measure `heartbeat` tick rate on each (`lab_client_status`, 30 s). | The focus throttle: expect the un-focused client lower. Then repeat with `client_input_focus` on both. |
| LT-7 | Same account on both instances (copy `lab-account.json` to `lab-account.p2.json`). | The `duplicate_login` eviction, seen from the client. |

## Live results (2026-10-10)

Run on the lab workstation against a local server on `main`, recorded on #1312 and in the [campaign ledger](../../analysis/lab-parallel-clients/README.md). Handle and share checks used `NtQueryInformationProcess(ProcessHandleInformation)` and trial `CreateFileW` opens, not Process Monitor.

### Two clients, one Documents folder

p1 was the default instance (`lab`), p2 a named instance (`CIMMERIA_LAB_INSTANCE=p2`, bridge 8771, `lab2`), run in both launch orders.

**The first client locks the cache for its lifetime.** From start-up it holds all 22 `Cache.en-US\*.pak` open with `GENERIC_READ|GENERIC_WRITE` (access `0x0012019f`), and `Logs\Launch.log`, `Appearance.log` and `Interface.log` (access `0x00120196`), sharing read only. Trial opens of `TextStrings.pak`, `CookedWorldInfo.pak` and `Launch.log` while it ran:

| Open | Result |
|---|---|
| read, share read/write/delete | ok |
| read+write, share read/write/delete | `ERROR_SHARING_VIOLATION` (32) |
| read, share read only | 32 |

`Config\*.ini`, `SavedSystemOptions.xml`, `WindowStates.xml` and `Content\LocalShaderCache-PC-D3D-SM3.upk` are not held open; trial read/write opens succeed. The logs are not a blocker: the second client renames its own to `Launch_2.log` and `Appearance_2.log`.

**The second client runs without a cache.** `SGWDebugLog.log` has, 22 times each at start-up, `Error opening static cache archive ...\Cache.en-US\<name>.pak` and `Error copying source archive ...\SourceCache.en-US\<name>.pak to archive ...`. At login it reports cooked version 0 for all 21 categories, and the server answers `full_resync` for every one (`cooked_data.version_reply`). One login's push (`cooked_data.sync_finish`):

| Category | Entries | Push (ms) |
|---|---:|---:|
| 10 (TextStrings) | 29,126 | 37,277 |
| 4 | 6,074 | 8,617 |
| 5 | 5,406 | 7,598 |
| 8 | 4,663 | 6,028 |
| 9 | 3,216 | 4,199 |
| 1 | 1,993 | 2,579 |
| 2 | 1,886 | 2,391 |
| 3 | 1,040 | 2,929 |
| all 21 | about 59,000 | about 75 s on loopback |

A solo login of the same client re-sent one category (11, 216 entries, 282 ms). Every entry the second client receives goes to an archive it cannot open, so its next login starts from version 0 again.

**That push is the freeze.** While it runs, the second client's main thread stops pumping messages: every bridge call times out (`dispatch timeout`), and Windows flagged the window `Responding = False` for 21 s (about 20 s of main-thread CPU in that window), exactly while categories 5 and 4 were pushed. World entry took 25 to 30 s instead of about 6 s, and the client then recovered and played normally. The **first** client stayed responsive the whole time, in both launch orders. Over the WAN the push is far slower: the Wine client's cooked-version-0 case is the same mechanism and took about 8 minutes. "Both clients freeze" was not reproduced.

### Per-instance `USERPROFILE`, up to five clients

A client launched with `USERPROFILE` at a seeded per-instance profile opened its own 22 archives (checked with the same handle query). Four lab clients (`lab` to `lab4`, each with its own profile) then logged in within about 7 s each and entered the world in 6 to 10 s, all at once: 20 categories `up_to_date`, one routine one-category resend each, zero cache errors. All four travelled to the Debug Area together. A fifth client (bare, with the `lab5` profile) booted to the login screen alongside them; the lab's cap at the time (4) refused a fifth lab client.

### Lab problems found on the way

- With other clients running, a new client's bridge took about 25 s to come up, and the watchdog killed it at about 17 s as `heartbeat unreachable`, three times, then hit its recovery cap. Fixed by the boot grace.
- The daemon hosted one supervisor and refused a second daemon (`another lab daemon holds Local\cimmeria-labd`), so p2 needed a stdio supervisor beside it. Fixed: one daemon hosts every instance.
- Character creation failed in the second client: the Create button never opened `CharCreateWin`, because character creation needs `CookedCharCreation.pak`, which a client without its cache doesn't have.
- Once, a client started while the other sat in the world hung for about 30 s **before** the login screen (heartbeat tick count 1, low main-thread CPU, no cache handles), then recovered. Not explained; seen once.

### The 2026-09-29 plan, step by step

| Id | Status | Result |
|---|---|---|
| LT-1 | run | Two clients on two instances, two ports, two accounts, in both launch orders. Five instances in one daemon: pending LP-07. |
| LT-2 | not run | Not recorded. |
| LT-3 | run | Concurrent cache writes fail loudly: the second client cannot open any archive (above). |
| LT-4 | run, by handle query | `GENERIC_READ\|GENERIC_WRITE`, sharing read only. |
| LT-5 | not run | The named-mapping hypothesis is untested. Separate profiles give each client its own archives, but the mapping name is machine-wide, so the hypothesis still stands. |
| LT-6 | not run | Not recorded. Virtual focus stays the lab's answer to the throttle. |
| LT-7 | not run | Not recorded; every run used separate accounts. |

The end-to-end check of the merged code (five clients in one daemon with five leases, a lease from one instance refused on another, zero cache errors, no full resync, no watchdog kill during boot) is pending LP-07.

## Open questions

- Is the Mercury local port ephemeral (LT-2)? Only matters if a firewall rule pins a port.
- Does the named `ZipArchive Mapping File` ever collide in practice (LT-5)?
- What was the one 30 s hang before the login screen?
- On a machine whose Documents folder is redirected (OneDrive), the `USERPROFILE` redirect does nothing; the lab DLL hook (proposal 3 on #1312) is still to be built.

Answered on 2026-10-10: the cache archives' share mode (LT-3, LT-4: read-only sharing, so a second writer fails), and `Launch.log` exists and is held open (the 2026-09-29 question whether the UE3 log is disabled).

## Evidence trail

| Claim | Where |
|---|---|
| No single-instance mutex | Ghidra xrefs to IAT `0x017ef148` (`CreateMutexW`), `0x017ef1b4` (`CreateMutexA`), `0x017ef1b0` (`CreateSemaphoreA`), `0x017ef11c` (`CreateEventW`): every call decompiled takes a `NULL` name; the five undecoded `CreateMutexW` sites are the residual uncertainty |
| Named mapping | `0x0139ede0` decompile: literal `ZipArchive Mapping File`; caller `0x0139f460` |
| Focus throttle | `FEngineLoop::Tick` `0x00417100`, instructions `0x004171a7` to `0x004171fe`; `0x00491320` decompile |
| Bind loop | `FUN_01584870` decompile; callers `Mercury_Nub_3`, `Mercury_Nub_9`, `FUN_01589f80` |
| UE3 share mode | `FFileManagerWindows::CreateFileWriter` `0x004c6090` |
| `SGW.lock` | file content `#Ghidra Lock File`, `SGW.gpr`/`SGW.rep` beside it |
| Duplicate login and relaunch takeover | `crates/base/src/base/login/eviction.rs` (`evict_prior_sessions`), gate in `relaunch.rs` |
| Environment reaches the game | `crates/client-launch/src/process.rs` `CreateProcessW(..., lpEnvironment = NULL, ...)` |
| Two-instance isolation | `crates/sgw-testhost/tests/dll_boot.rs`, both new tests |
| Cache lock, resync, freeze, four-client run | #1312 (2026-10-10 comments); [lab-parallel-clients ledger](../../analysis/lab-parallel-clients/README.md#what-the-2026-10-10-runs-proved) |
| My Documents lookup | `SHGetFolderPathW` import (IAT `0x017efd48`), `CSIDL_PERSONAL` call `0x004c6333`, `CSIDL_LOCAL_APPDATA` call `0x004935ad` |
| Profile seed and fallbacks | `crates/lab/src/supervisor/instance_profile.rs` and its unit tests |
| Hosting and routing | `crates/lab/src/server/instances.rs` (routing tests), `crates/lab/src/main.rs` `build_hosted_server` |

## Cross-reference targets

[live-research-lab.md](../../guides/live-research-lab.md) (updated), [live-research-lab ADR](../../architecture/live-research-lab.md) (single-client assumption; §12 for the hosted instances), [cooked-data-pipeline.md](cooked-data-pipeline.md) (cache archive writers), [.mcp.json.example](../../../.mcp.json.example).
