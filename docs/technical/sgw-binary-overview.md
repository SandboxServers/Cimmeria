# SGW Binary Overview

Reverse engineering analysis of `sgw.exe` — the Stargate Worlds UnrealEd build.

## Binary Profile

| Property | Value |
|----------|-------|
| **File** | sgw.exe |
| **Type** | Win32 PE executable (32-bit) |
| **Base Address** | 0x00400000 |
| **Engine** | Unreal Engine 3 (modified by CME) |
| **Build Path** | `c:\build\qa\sgw\working\development\src\` |
| **Executables** | SGWGame (client), SGWEditor (editor) |
| **Config Files** | SGWLocal.ini, SGWLogConfig.xml |
| **Website** | http://www.stargateworlds.com/ |
| **Beta URL** | http://beta.stargateworlds.com/ |

## Memory Sections

| Section | Start | End | Size |
|---------|-------|-----|------|
| .text | 0x00401000 | 0x017ee3ff | ~20.4 MB (code) |
| .rdata | 0x017ef000 | 0x01d8e3ff | ~5.8 MB (read-only data) |
| .data | 0x01d8f000 | 0x01f159ef | ~1.5 MB (initialized data) |
| .tls | 0x01f16000 | 0x01f161ff | 512 bytes (thread-local) |
| .rsrc | 0x01f17000 | 0x01fe41ff | ~820 KB (resources) |
| .reloc | 0x01fe5000 | 0x0223d5ff | ~2.4 MB (relocations) |

## Technology Stack

### Unreal Engine 3
- wxWidgets UI framework (for UnrealEd panels)
- Standard UE3 object system (UObject, UClass, UProperty, etc.)
- UE3 content streaming, shader management, rendering thread
- Kismet visual scripting (USeqEvent_Stargate, etc.)

### BigWorld Technology (Server Architecture)
CME licensed BigWorld Technology for their server backend. The client contains references to the full server topology:

- **LoginApp** — Handles initial authentication
- **BaseAppMgr** — Manages BaseApp allocation
- **BaseApp** — Per-player server process (handles persistent state)
- **CellApp** — Spatial simulation server (handles physics/combat)
- **DBMgr** — Database manager

Evidence: `..\..\..\..\Server\bigworld\src\client\entity_manager.cpp` and `..\..\..\..\Server\bigworld\src\common\servconn.cpp` compiled into the client binary.

### Mercury Protocol (CME Proprietary Networking)
Custom networking layer built on top of BigWorld's infrastructure. See [mercury-protocol.md](mercury-protocol.md).

### OpenSSL (Embedded)
Full OpenSSL crypto library compiled in. AES used for session encryption:
- AES-128/192/256 in CBC, ECB, CFB, OFB, CFB1, CFB8 modes
- TLS cipher suites: ECDHE, ECDH, DHE, ADH variants
- Cipher preference string at 0x01b29118: `AES:ALL:!aNULL:!eNULL:+RC4:@STRENGTH`
- Key setup source: `.\crypto\evp\e_aes.c` (at 0x01b345e8)

### Intel TBB
Thread Building Blocks used for concurrent message queuing:
- `tbb::concurrent_queue` templates for Mercury ClientMessage dispatch

### Third-Party Libraries
- zlib (ZIP storage for cooked data packages)
- WinZip AES encryption support (for archive decompression)

## Source File References

The following source files are referenced via assertion macros and debug strings. All paths relative to `c:\build\qa\sgw\working\development\src\`:

### Core Engine (Unreal)
| File | Address Range | Purpose |
|------|--------------|---------|
| LaunchEngineLoop.cpp | 0x017f7ea8+ | Engine startup sequence |
| LaunchMisc.cpp | 0x017f9628+ | Misc launch utilities |
| UnMisc.cpp | 0x01801dc0+ | Core utility functions |
| UnAnsi.cpp | 0x018040dc | ANSI string handling |
| UnVcWin32.cpp | 0x0180456f+ | Win32 platform layer |
| UnCoreNative.cpp | 0x01808ef0+ | Core native functions |
| UnName.cpp | 0x0180a4ec+ | FName system |
| UnObj.cpp | 0x0180c98c+ | UObject system (extensive) |
| UnClass.cpp | 0x01810178+ | UClass reflection |
| UnLinker.cpp | 0x01811348+ | Package linker |
| UnCorSc.cpp | 0x01815dbc+ | Core script |
| UnArchive.cpp | 0x01816aec+ | Serialization archives |
| UnProp.cpp | 0x018179a0+ | UProperty system |
| UnMath.cpp | 0x018198e8 | Math utilities |
| UnMem.cpp | 0x01819a2f | Memory allocator |
| UnCoreNet.cpp | 0x0181a1f2+ | Core networking |
| UnBulkData.cpp | 0x0181b1ec+ | Bulk data loading |
| UnEngine.cpp | 0x018329bc+ | Engine tick loop |
| UnWorld.cpp | 0x018371e4+ | World management |
| UnObjGC.cpp | 0x01814270+ | Garbage collection |
| UnAsyncLoading.cpp | 0x01813ad0+ | Async content loading |
| UnObjectRedirector.cpp | 0x0181aff4+ | Object redirectors |
| UnOutputDevices.cpp | 0x01814a48+ | Log output devices |
| Distributions.cpp | 0x01819d58+ | Statistical distributions |

### Platform
| File | Address Range | Purpose |
|------|--------------|---------|
| UnThreadingWindows.cpp | 0x01812f3e+ | Win32 threading |
| UnThreadingBase.cpp | 0x0181332c+ | Thread base classes |
| FFileManagerGeneric.cpp | 0x01812e70+ | Generic file I/O |
| FFileManagerWindows.cpp | 0x01813544+ | Win32 file I/O |
| UnAsyncWork.cpp | 0x0181b788+ | Async task system |

### Rendering
| File | Address Range | Purpose |
|------|--------------|---------|
| ShaderManager.cpp | 0x01835518+ | Shader compilation/caching |
| RenderingThread.cpp | 0x01836f50+ | Render thread management |
| UnContentStreaming.cpp | 0x018363bc+ | Content streaming system |

### Networking
| File | Address Range | Purpose |
|------|--------------|---------|
| TcpNetDriver.cpp | 0x018002d8+ | TCP network driver |
| BWNetDriver.cpp | 0x01801450 | BigWorld network driver |
| BWConnection.cpp | 0x01801648 | BigWorld connection |
| SGWTestIpDrv.cpp | 0x01800124 | Test network driver |
| SGWNetworkManager.cpp | 0x019abbc4+ | SGW network manager |
| DebugCommunication.cpp | 0x017ffcb8+ | Debug comm channel |

### SGW Game
| File | Address Range | Purpose |
|------|--------------|---------|
| SGWTaskManager.cpp | 0x0181b8b8 | Task/job management |
| CombatQueue.cpp | 0x019eac40+ | Combat ability queue |
| BaseAppearanceJob.cpp | 0x019eb8f0 | Character appearance loading |
| BigWorldEntity.cpp | 0x018cacb8+ | Entity integration |
| ZipStorage.cpp | 0x017fe5d8+ | Cooked data packages |

### BigWorld (Server SDK, compiled into client)
| File | Purpose |
|------|---------|
| `Server\bigworld\src\client\entity_manager.cpp` | Entity lifecycle (at 0x019cd8e0+) |
| `Server\bigworld\src\common\servconn.cpp` | Server connection (at 0x019cec60+) |

### Factories & Serialization
| File | Address Range | Purpose |
|------|--------------|---------|
| UFactory.cpp | 0x0181a630+ | Asset import factories |
| UExporter.cpp | 0x0181a90c+ | Asset export |
| UHelpCommandlet.cpp | 0x01819b98 | Help commandlet |

## Cooked Data Packages

The game uses `.pak` files for cooked (pre-processed) data:
- `CookedDataStargates.pak` (at 0x017f9de0) — Stargate definitions
- `COOKED_STARGATE` flag (at 0x019bb7b4)
- CookedData types include: `CookedData::StargateType`
