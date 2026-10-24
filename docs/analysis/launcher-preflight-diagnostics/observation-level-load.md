# Observation: stuck on the level loading screen (macOS, Wine, 2026-10-24)

Read-only observation of a live session. Nothing was killed, clicked or edited.

- Host: macOS 27.0.1 on Apple Silicon. SGW.exe runs as an x86-64 Wine 11.13 process under Rosetta, with DXVK, on the built-in display.
- Install: `~/Documents/cimmeria-test/game/Working/binaries/SGW.exe`. SGW.exe was PID 17225, launched at 12:06:56 local time (CDT).
- Telemetry session id: `3f6b796a-4606-4418-b5dc-3d7442ba9de3`. Launcher build `de83747a1`.
- Server: play.cimmeria.app (47.45.19.7).

## Verdict

**The client crashed on its side. It is not waiting on the server, and it is not disconnected.** UE3's rendering thread hit a General Protection Fault 33 s after the client left character select. The world map had already started streaming in when it crashed. The process is still alive, so the loading screen stays up, but it will not recover.

## Timeline

| Local time | Evidence | Event |
|---|---|---|
| 12:06:57 | launcher log `game_started operation=2` | Client started |
| 12:09:57 | `Stats/Login_Map.log` "Dumping FPS chart" | Login_Map unloaded (the user pressed Play) |
| 12:09:49 to 12:09:56 | `Logs/Interface.log` | In-world CEGUI layouts loaded (Vendor, Vault, Trade and so on) |
| about 12:10 | `lsof` | `Maps/Castle_CellBlock/Castle_CellBlock.umap` and 17 streaming sublevels (`-fffcfffc` to `-fffffffe`) opened |
| **12:10:30** | `cimmeria-client-telemetry.log` `+212781ms` | **`UE3 fatal error [Log]: Rendering thread exception: General protection fault! History:`** (empty history) |
| 12:10:30 | `SGWDebugLog.log` | The last read and write lock lines from threads 380 and 452, then silence |
| 12:10:30 onward | all logs | No further lines in any log |

`+212781 ms` after the hook install (about 12:06:57) is 12:10:30, which matches the last `SGWDebugLog.log` write to the second.

## 1. Process

| Time | CPU % | RSS (KB) | Uptime |
|---|---|---|---|
| 12:10:22 | 148.6 | 854,184 | 03:24 |
| 12:10:28 | 118.3 | 909,000 | 03:30 |
| 12:10:38 | 128.9 | 1,026,464 | 03:40 |
| 12:12:59 | 120.0 | 1,118,420 | 06:13 |
| 12:13:36 | 122.3 | 1,138,624 | 06:50 |

The process is alive (state `Rs`). CPU holds at about 1.2 cores and RSS keeps creeping up (peak footprint 1.5 GB).

`sample 17225 2` (written to `/tmp/sgw-sample.txt`, not committed) found 53 threads. Most are parked in Wine syscall waits, and all the DXVK threads (`dxvk-submit/queue/cs/shader/writer`) are idle. Two threads run translated game code. `Thread_9146049`, the first Wine thread and likely the Windows main/game thread, is in Rosetta JIT for 339 of 339 samples, under `_sigtramp`, which suggests it is spinning in an exception or signal path. `Thread_9147062` is in JIT for about half the samples. Rosetta hides the PE symbols, so we can't name the functions. The pattern fits a game thread that keeps ticking or spinning after the render thread died, rather than a thread loading content.

## 2. Disk

`lsof -p` showed 579 descriptors and 108 `.upk`/`.umap` files open. Two snapshots 20 s apart (12:12:24 and 12:12:44) listed **the same files**, so no new packages were opening. The open set is the Castle_CellBlock map, its 17 sublevels, character, armor and VFX packages, and the leftover `Login_Map_MapData.upk`. I skipped `fs_usage` because it needs root. Map loading reached the target map and then stopped.

## 3. Network

`lsof -nP -i -a -p 17225` showed two sockets:

- `TCP 10.10.10.148:57348 -> 47.45.19.7:8081 ESTABLISHED`, the telemetry uploader (`http://play.cimmeria.app:8081/api/telemetry`).
- `UDP *:49668`, the Mercury world socket.

`netstat -anv -p udp` byte counters on `*:49668`:

| Time | rxbytes | txbytes |
|---|---|---|
| 12:10:22 | 21,848,704 | 84,921 |
| 12:11:18 | 23,159,584 | 85,497 |
| 12:11:33 | 23,692,448 | 85,753 |
| 12:12:59 | 27,106,496 | 87,769 |
| 12:13:36 | 28,428,064 | 88,377 |

- **Inbound is live at a steady ~35 KB/s** (about 177 KB per 5 s), so the server is still sending to this client.
- **Outbound is about 20 B/s** (about 96 B per 5 s), which is keepalive or ack sized.
- Recv-Q was 2,160 B at first and 0 later, so the socket is still being drained (the `recvfrom` hook thread or Wine is still reading).
- Open question: is 35 KB/s normal Castle_CellBlock entity and AoI traffic, or the server resending reliable packets the crashed client no longer acks properly? Only a server-side packet tap or log can answer that, and it doesn't change the verdict.

## 4. Logs (new since sign-in)

- `cimmeria-client-telemetry.log`: hook install at +0 to 185 ms. The only runtime line is the fatal error quoted in the timeline. It has no map-load, disconnect, entity, space or teleport lines. The `mercury_process_ordered_packet` hook was disabled by `CIMMERIA_CLIENT_HOOKS_DISABLE`.
- `cimmeria-client-patches.log`: normal. Patches attached, Black Market Lua registered in lua_State #1 (+13 s, login UI) and #2 (+202 s, world UI, about 12:10:19). So the world UI Lua came up 11 s before the crash.
- `SGWDebugLog.log`: only log4cxx read and write lock chatter at 12:10:30, no errors.
- `Logs/Interface.log`: all CEGUI layouts loaded successfully, the last at 12:09:56 (`Vendor.layout`).
- `Logs/Appearance.log`: header only. `Logs/Launch.log`: empty.
- Launcher log `launcher.2026-10-24.log`: operation 2 started cleanly (`game_started startup_ms=3946`), with no exit event yet. The earlier operation 1 (16:56Z to 17:04Z) exited with code 0 after 493 s.
- `state/game-telemetry-status.json`: `"outcome":"attached"`.

## 5. What ends the loading screen (from repo docs, quick pass)

World entry is the BigWorld flow on the Mercury UDP channel. `createBasePlayer` sets the player entity id (`ServerConnection+0x16c`). `spaceViewportInfo` (0x08) sets the current viewport and space refcount. `spaceData` names the space and map, and `createCellPlayer` places the avatar. Ghost entities pick up their viewport association from `avatarUpdate` (0x10 to 0x2F). Sources: `.claude/agent-memory/bigworld-engine-advisor/viewport-system.md` and `voicedata-handler-and-recv-loop-stall.md`. The usual server-side stalls are:

- A BaseApp handler that sleeps inline. `run_connect_loop` awaits `handle_datagram` for every client, so a sleep in `dispatch_deferred` from `onClientReady` stalls all inbound traffic.
- A missing `createCellPlayer`/viewport step, which leaves the map loaded with no avatar.

Neither applies here. The client got far enough to open the Castle_CellBlock umaps and build the world UI, so the server had already sent the space and map. The client then died in its own render thread.

## Next step

1. Close SGW.exe yourself (it won't recover), then press Play again to see whether the GPF repeats on the same map. If it repeats, this is a client rendering crash under Wine, DXVK and MoltenVK on Castle_CellBlock (a client-side render or PhysX bug). It belongs to the launcher, Wine and graphics workstream, not the server.
2. You don't need server-side logs from play.cimmeria.app for the verdict. They are optional, for checking whether the ~35 KB/s inbound UDP after the crash is retransmission. If wanted, query SigNoz for session `3f6b796a-4606-4418-b5dc-3d7442ba9de3` (the `telemetry-triage` skill): the fatal line should have uploaded, and the server trace shows how far world entry got.
3. The UE3 fatal `History:` is empty. A repeat run with a minidump or Wine `+seh` trace on the render thread would be the next client-side evidence.
