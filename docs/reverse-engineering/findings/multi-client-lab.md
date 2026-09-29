# Two SGW.exe clients on one workstation

> **Last updated**: 2026-09-29
> **Audience**: lab operators and engineers who want a two-player scenario driven from one machine
> **Type**: RE finding (static client analysis, lab code audit, headless DLL tests) with a fix
> **Confidence**: HIGH for the lab-side collisions (proven in code and by test), MEDIUM for the client-side shared state (Ghidra evidence, no live two-client run yet)

## Summary

The client has **no single-instance guard**. A second `SGW.exe` starts fine; what goes wrong is everything around it. The lab was written for one client, so a second one collides with the first on the session file, the bridge port, the credentials, the crash markers and the logs, and the supervisor refuses to start it at all. Separately, two clients share writable state under `Documents\My Games\Firesky\SGWGame\` and the client throttles whichever window is not in the foreground.

**Nothing in this document was observed on two live clients.** The coordinator paused live `SGW.exe` runs on 2026-09-29 while this investigation was in progress, so every client-side claim below is Ghidra evidence, and the [live test plan](#live-test-plan-not-yet-run) lists what would confirm or refute each one. The lab-side collisions are different: they are in our own code, and two headless tests reproduce them.

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
| Cooked-data cache `Documents\...\SGWGame\Cache.en-US\*.pak` | 22 `.pak` files, all rewritten at every login by the server's version push (timestamps equal the launch time). They are ZipArchive files (`ZipStorageBase::OpenArchive` `0x00479340`, `WriteStreamToFile` `0x00479930`, [cooked-data-pipeline.md](cooked-data-pipeline.md)). The archive open goes through `ZipPlatform` `FUN_0139d680`, which maps its share argument onto `_wsopen_s` share flags (`_SH_DENYRW`, `_SH_DENYWR` or `_SH_DENYNO`). | Two clients write the same archives. Which share flag the cache archives use decides between "second client fails to open and logs `Error opening static cache archive`" and "both write". Not determined statically. | needs live check |
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
| One credentials file | `sessions\lab-account.json` | A second client would log into the same account, and the server evicts the older session (`crates/base/src/base/login/mod.rs:110`, `duplicate_login`). |
| One crash marker and minidump set | The DLL writes beside the session file | Two clients overwrite each other's evidence. |
| Two DLL logs | `cimmeria-client-telemetry.log`, `cimmeria-client-patches.log`, both `File::create` in `binaries\` | Each client truncates the other's log. |
| Named hook lock | `client-hookgate` `hook_lock_name(pid)` | Per process: fine. |
| Bridge auth | One 64-hex token per launch | Per launch: fine, and a wrong-port connect is refused, not misrouted. |
| Screenshots, input | `PrintWindow` on the window found by pid; input state and virtual focus are process statics in the DLL | Per process: fine. |
| Telemetry mint | One `grant_for_launch` per launch, own `session_id` | Fine: each client is its own SigNoz session. |

## Root causes, ranked by how likely they explain "odd behavior"

1. **Same account logged in twice.** The server evicts the older session on a duplicate login. The first client is logged off and looks like it crashed. Expected, and the reason a second player needs a second account.
2. **Background throttling.** A window not in the foreground runs at lower priority with a 5 ms sleep per tick (`0x00417100`). Two windows side by side always leave one throttled. Symptoms: slower animation and Mercury processing in the client you are not looking at.
3. **Lab collisions.** Proven in code and by test; see the table above.
4. **Shared writable state** (cache `.pak` files, the named `ZipArchive Mapping File`, shader cache). Plausible source of rare corruption or `Error opening static cache archive` lines. **Unconfirmed**; the live plan below decides it.

## What was fixed (lab-only, no client patch)

Named lab instances, in [`crates/lab/src/supervisor/instance.rs`](../../../crates/lab/src/supervisor/instance.rs). A second `cimmeria-lab` MCP server entry with `CIMMERIA_LAB_INSTANCE=p2` drives a second client:

- **Session file**: `sessions\instances\<name>\current-session.json`, found by the DLL through `CIMMERIA_LAB_SESSION_FILE`, which the supervisor passes through `sgw-start32` (`start32::run_with_env`). `CreateProcessW` is called with a null environment block (`crates/client-launch/src/process.rs`), so the game inherits it. No client patch and no change to the default layout.
- **Bridge port**: `CIMMERIA_LAB_BRIDGE_PORT` per instance; `CIMMERIA_LAB_BRIDGE` now defaults to that port.
- **Crash marker and minidumps**: in the instance directory (the DLL already used the session file's directory); `lab_crash_report` reads it.
- **Credentials**: `sessions\lab-account.<name>.json`. A named instance never falls back to `lab-account.json`, because that would log both clients into one account.
- **Logs**: `cimmeria-client-telemetry-<name>.log` and `cimmeria-client-patches-<name>.log`.
- **Start guard**: refuses only for an `SGW.exe` that is not another lab instance's, for a bridge port a peer already uses, and past `CIMMERIA_LAB_MAX_CLIENTS` (default 2, ceiling 4). Peers are found through `lab-instance.json` files.
- **Telemetry**: the session carries an `instance:<name>` tag.

The default instance (no `CIMMERIA_LAB_INSTANCE`) is byte-for-byte the old layout.

Tests: 14 unit tests in `instance.rs` (name validation, paths, environment, the guard rules including a peer still launching and a port clash, registry round trip), `write_session_at`, the DLL's `resolve_session_path` and `sanitize_instance`, log file naming in both DLLs, and two boot tests in `sgw-testhost` that run two real lab-bridge DLL instances in one install directory. Reverting the DLL's session override makes `two_lab_instances_in_one_install_each_get_their_own_bridge_and_log` fail (checked).

## Needs an owner decision

1. **A `lab2` account.** *Done 2026-09-29: `lab2` to `lab5` are seeded (ids 11 to 14) and muted in Discord.* Before that only `lab` existed, and a duplicate login evicts the first client, so a true two-player run needs a second account. Proposed, not committed (seeds ship with a release, and the account list is the owner's call):

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
| Trade, squads and teams, mail, chat, visibility, where the second player only needs to act | Two full clients: two `cimmeria-lab` entries (`lab-p1`, `lab-p2`), two accounts. See [the guide](../../guides/live-research-lab.md#two-clients-two-player-scenarios). |
| A second player who only needs to exist and answer (duel partner, someone to be visible) | `sparbot` (`crates/wireclient`), headless, no second window, no throttling, no shared cache. It also needs its own account. |
| Anything that only needs bytes on the wire | `crates/wireclient` `GameSession` with `cell_method` and `base_method`, as the two-client visibility and social tests do. |

Use a second client when the UI of the second player is part of what is being tested. Use `wireclient` when it is not: it cannot hit any of the client-side shared state above.

## Live test plan (not yet run)

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

## Open questions

- What share flag do the cache archives use (LT-3, LT-4)? Decides whether shared cache writes fail loudly or interleave.
- Is the Mercury local port ephemeral (LT-2)? Only matters if a firewall rule pins a port.
- Does the named `ZipArchive Mapping File` ever collide in practice (LT-5)?
- Does `Launch.log` exist? The client formats a `Launch.log` path (`0x017fa1f0`, `0x01814d20`) but the `Logs` folder holds only `Appearance-backup-*.log`; the UE3 log appears disabled.

## Evidence trail

| Claim | Where |
|---|---|
| No single-instance mutex | Ghidra xrefs to IAT `0x017ef148` (`CreateMutexW`), `0x017ef1b4` (`CreateMutexA`), `0x017ef1b0` (`CreateSemaphoreA`), `0x017ef11c` (`CreateEventW`): every call decompiled takes a `NULL` name; the five undecoded `CreateMutexW` sites are the residual uncertainty |
| Named mapping | `0x0139ede0` decompile: literal `ZipArchive Mapping File`; caller `0x0139f460` |
| Focus throttle | `FEngineLoop::Tick` `0x00417100`, instructions `0x004171a7` to `0x004171fe`; `0x00491320` decompile |
| Bind loop | `FUN_01584870` decompile; callers `Mercury_Nub_3`, `Mercury_Nub_9`, `FUN_01589f80` |
| UE3 share mode | `FFileManagerWindows::CreateFileWriter` `0x004c6090` |
| `SGW.lock` | file content `#Ghidra Lock File`, `SGW.gpr`/`SGW.rep` beside it |
| Duplicate login | `crates/base/src/base/login/mod.rs` lines 110 to 139 |
| Environment reaches the game | `crates/client-launch/src/process.rs` `CreateProcessW(..., lpEnvironment = NULL, ...)` |
| Two-instance isolation | `crates/sgw-testhost/tests/dll_boot.rs`, both new tests |

## Cross-reference targets

[live-research-lab.md](../../guides/live-research-lab.md) (updated), [live-research-lab ADR](../../architecture/live-research-lab.md) (single-client assumption), [cooked-data-pipeline.md](cooked-data-pipeline.md) (cache archive writers), [.mcp.json.example](../../../.mcp.json.example) (second entry).
