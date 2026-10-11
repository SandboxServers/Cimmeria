---
title: "Client limits: engine"
type: reference
audience: engineers, the lab-chaos campaign
last_updated: 2026-10-11
---

# Client limits: engine

The SGW client's Unreal Engine 3 bounds: frame time, the texture streaming keys, frame smoothing, the background throttle and the display loss that stops an idle lab client. Grades, column rules and the summary are in the [limits index](README.md).

| Bound | Value | Grade | Source | Safe level | Break level | Notes |
|---|---|---|---|---|---|---|
| Hitch threshold | 200 ms | config | `crates/client-telemetry/src/hooks/seams/frame_health.rs` `HITCH_MS` | n/a | n/a | `client.engine.hitch` fires on a tick gap at or above it; 100 ms under `firehose` capture (`HITCH_MS_FIREHOSE`), so lab firehose sessions report smaller hitches. |
| Frame time (p50, p95, max per 30 s window) | not measured | n/a | n/a | n/a | n/a | `client.engine.memory` carries `frame_p50_ms`, `frame_p95_ms` and `frame_max_ms` since CL-06; CL-08 measures them per world once the lab DLL is rebuilt. |
| Hitches per world | see [Hitches per world](#hitches-per-world) | measured | SigNoz `client.engine.hitch`, 2026-10-11, 30 days | n/a | n/a | Counts are lower bounds: the sink throttle drops repeats, and firehose lab sessions use the 100 ms threshold. |
| Busy cores at which frame time passes the hitch threshold | not measured | n/a | n/a | not measured | not measured | Chaos profile `cpu`. CL-08 fills the safe level from the five-client p95. |
| Texture pool size (`[TextureStreaming] PoolSize`) | `140` in the ini, **inert: not read by the PC client** | static | `Engine/Config/BaseEngine.ini` line 237; no `PoolSize` string in `SGW.exe` (ASCII and UTF-16 byte scan, plus Ghidra string search `pool`) or in any of the 8 FRScript `.u` packages | n/a | n/a | Changing it does nothing. CL-13 must not target it. Its nine sibling keys are all read, in `FUN_0054a950`: the four margins and the five fudge and mip rules below. |
| Texture streaming margins | `HysteresisLimit` 30, `DropMipLevelsLimit` 24, `StopIncreasingLimit` 18, `StopStreamingLimit` 12 | config | `Engine/Config/BaseEngine.ini` lines 238-241 under `[TextureStreaming]`; read through `GConfig->GetInt(TEXT("TextureStreaming"), ...)` with the key strings at `0x018363E0`, `0x018364B8`, `0x01836598`, `0x01836678` (`GEngineIni`) | n/a | n/a | Unit and meaning (free-texture-memory margins in MB) inferred from UE3 naming; the consumer was not traced. These are the real streaming budget. |
| Texture streaming fudge and mip rules | `FudgeFactorIncreaseRateOfChange` 0.5, `FudgeFactorDecreaseRateOfChange` -0.4, `MinRequestedMipsToConsider` 11, `MinTimeToGuaranteeMinMipCount` 2, `MaxTimeToGuaranteeMinMipCount` 12 | config | `Engine/Config/BaseEngine.ini` lines 242-246; read at `0x01836768`, `0x01836860`, `0x01836960`, `0x01836A68`, `0x01836B78` | n/a | n/a | Same caveat on meaning. |
| Frame smoothing | `bSmoothFrameRate=TRUE`, `MinSmoothedFrameRate=22`, `MaxSmoothedFrameRate=62` | config | `Engine/Config/BaseEngine.ini` lines 113-115 under `[Engine.GameEngine]`; config properties of `Engine.GameEngine` declared in the script package `Engine.u` (no string in `SGW.exe`) | n/a | n/a | Not overridden by `DefaultEngine.ini` or `GameplayEngine.ini`. Behavior inferred from UE3: game-time delta is clamped to 1/62 s .. 1/22 s, so under 22 fps game time slows (relevant to the `cpu` chaos profile). Consumer not traced. |
| Minimum desired frame rate | `35.0` | config | `Engine/Config/BaseEngine.ini` line 182 under `[Engine.Client]`; the key name is a wide string at `0x018497C0` | n/a | n/a | `MinDesiredFrameRate`. Consumer not traced. |
| Background throttle | none in the player client; a 5 ms `Sleep` and a below-normal thread priority run only when `GIsEditor` is set | static | `FEngineLoop::Tick` (`0x00416EC0`), block `0x004171A7` to `0x00417204`: foreground test via `GetForegroundWindow`/`GetWindowThreadProcessId` at `0x004171A7`-`0x004171B4`; `GIsEditor` (`0x01EAD7AC`) guard, the `JZ` at `0x004171CB` skips the block when it is 0 | n/a | n/a | The `Sleep(5)` (`FUN_00491320(0.005f)` at `0x004171F6`, `Sleep((DWORD)(arg * 1000.0))`) runs on every unfocused tick. `SetThreadPriority(-1)` (`0x004171DF`-`0x004171E6`) is edge-triggered on the focus-loss tick (latch `0x01D8F9BC`); normal priority is restored at `0x00417260` on regain. `SetPriorityClass` is not imported, so this is the only focus-driven priority change. Writers of `0x01EAD7AC`: `PreInit` `0x004185E0` (writes 0, `0x00418AFC`) and `0x00B8EF8A`, `0x00B8F0E4` inside `0x00B8E8D0` (writes `!NOEDITOR`); no address-taking references. `0x00B8E8D0` is referenced only from the data word at `0x0197ED58` and is inferred to be a commandlet `Main` from its `NOEDITOR`/`NOCLIENT`/`NOSERVER` parsing. The Ghidra project symbol at `0x01EAD7AC` is stale (`g_dwMapCheckDepEnabled`); it is `GIsEditor` per `PreInit`'s five globals. No ini key or switch reaches the guard. Not confirmed live (LT-6: record thread priority with tick rate, or read the byte with `client_mem_read`). Earlier lab reports of a throttle (#1312) are better explained by an OS-level effect. |
| Idle display loss | about 10 min, then `D3DERR_NOTAVAILABLE` | inferred | lab observation, single report ([live-research-lab.md § The display](../../guides/live-research-lab.md#the-display-screensaver-and-d3d)) | n/a | n/a | The screensaver takes the display because lab input never resets the idle timer. The lab supervisor now holds the display awake. |

## Hitches per world

Tick gaps at or over the hitch threshold, each attributed to the latest visible level package in its session. SigNoz, 30 days to 2026-10-11.

| World | Kind | Hitches | Sessions | p50 ms | p95 ms | Max ms |
|---|---|---|---|---|---|---|
| Castle_CellBlock | player | 1,208 | 20 | 127 | 737 | 2,880 |
| Castle_CellBlock | lab | 3,641 | 11 | 282 | 1,278 | 33,604 |
| Ihpet_Crater_Light | player | 143 | 8 | 624 | 1,684 | 8,189 |
| Ihpet_Crater_Light | lab | 535 | 7 | 331 | 9,269 | 43,215 |
| Castle | player | 37 | 4 | 835 | 3,620 | 3,864 |
| SGC_W1 | player | 32 | 2 | 393 | 976 | 1,134 |
| Login_Map | player | 338 | 29 | 2,133 | 6,751 | 379,763 |

`Login_Map` gaps include level loads (the tick stops while a level loads), so they are not frame times.

## Ini chain and lab override targets

Static read of client build 0.8348 (2026-10-11), file reads and read-only Ghidra only.

```text
SGWGame/Config/DefaultEngine.ini        [Configuration] BasedOn=..\Engine\Config\GameplayEngine.ini
Engine/Config/GameplayEngine.ini        [Configuration] BasedOn=..\Engine\Config\BaseEngine.ini
Engine/Config/BaseEngine.ini            defines every key in the rows above
<Documents>/My Games/Firesky/SGWGame/Config/SGWEngine.ini    the user copy
```

- `DefaultEngine.ini` sets `[Engine.Engine] MyDocumentsSubDirName=SGWGame`, which names the user folder; the user file name is the `Default` prefix replaced by `SGW`.
- `GameplayEngine.ini` sets `NetInactivityTimeout=15` ([network.md](network.md)) and the content paths, and none of the keys above.
- The streaming, smoothing and frame keys are defined in `BaseEngine.ini` only. No other file in `Engine/Config/` or `SGWGame/Config/` mentions them (searched `PoolSize`, `SmoothFrameRate`, `SmoothedFrameRate`, `Background`, `Focus`, `Idle`, `LowerPriority`, `FrameRate`). `BaseGame.ini` `MaxIdleTime` is a server-side game-info field and `bUseBackgroundLevelStreaming` is level streaming; neither is window focus.
- The user `SGWEngine.ini` is a full generated copy (about 25 KB, every section). On the machine checked it repeats the stock values, and any key edited there wins over the whole chain. The lab seeds it per instance (`crates/lab/src/supervisor/instance_profile.rs`, `SEED_DIRS` includes `Config`).

**When the override survives.** `FUN_0048C8E0` walks the `BasedOn` chain and stamps `[IniVersion]`; `FUN_0048CD10` compares the user ini's `[IniVersion]` with the chain files' timestamps. If they match, the user `SGWEngine.ini` is used as the complete config. If they differ, the client prompts "Your ini (%s) file is outdated..." (`FUN_00492470`) and regenerates the user ini from the chain; the `NUKEINIS` and `UPDATEINISAUTO` switches skip or force that. So a lab override lasts only while `[IniVersion]` is intact and the chain files keep their timestamps: a reinstall or a patch that touches a chain file loses it, and a headless lab client could block on the modal prompt.

**Keys CL-13 should target** (the per-instance `SGWEngine.ini`, applied after seeding):

| Section | Keys | Why |
|---|---|---|
| `[TextureStreaming]` | `HysteresisLimit`, `DropMipLevelsLimit`, `StopIncreasingLimit`, `StopStreamingLimit`, `MinRequestedMipsToConsider` | The real streaming budget; raise or lower to measure texture pressure. |
| `[Engine.GameEngine]` | `bSmoothFrameRate`, `MinSmoothedFrameRate`, `MaxSmoothedFrameRate` | Whether low frame rates slow game time (the `cpu` profile). |
| `[Engine.Client]` | `MinDesiredFrameRate` | Candidate; consumer untraced. |
| `[TextureLODSettings]` | `TEXTUREGROUP_*` `MaxLODSize` | Candidate texture-size caps; consumer not traced. |

Do not target `[TextureStreaming] PoolSize` (inert) or any background or focus key (none exists for the player).

The lab applies these keys from a lab override file (absent by default) at each launch; see [the live research lab guide](../../guides/live-research-lab.md#ini-overrides-for-an-instance).
