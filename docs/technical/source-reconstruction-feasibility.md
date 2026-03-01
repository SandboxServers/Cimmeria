# Source Code Reconstruction Feasibility

Comprehensive analysis of what's recoverable from sgw.exe for full client source reconstruction.

## Binary Scale

| Metric | Value |
|--------|-------|
| **Total functions (Ghidra)** | ~169,420 |
| **Total RTTI entries** | 9,567 |
| **Unique non-template classes** | 4,943 |
| **Unique event types** | 954 |
| **Source files referenced** | 608 unique .cpp files |
| **Total assertion sites** | ~5,534 |

## Evidence Sources for Function Identification

### 1. Assertion Macros (~5,534 total)

Four distinct assertion systems embed file references:

| System | Pattern | Count |
|--------|---------|-------|
| BigWorld `MF_ASSERT` | `ASSERTION FAILED: <expr>\n<file>(%d)%s%s\n` | 17 |
| Unreal `check()`/`verify()` | Standalone `.cpp` filename string via `%s(%i): Assertion failed: %s` | ~5,497 |
| Unreal `TEXT()` expressions | `!TEXT("message")` or `expr && TEXT("message")` | ~15 |
| 3rd party (OpenSSL, etc.) | Various | ~5 |

**Key difference**: BigWorld assertions embed the **full expression AND line number** in the string. Unreal assertions embed only the **filename** — the line number is a runtime parameter passed via `%s(%i)` format at addresses `0x01801fd0` / `0x01802020`. Line numbers are NOT statically extractable from Unreal assertions, but each assertion xref still maps its containing function to a source file.

### 2. UnrealScript Exec Stubs (1,017 functions)

Every `intUClassexecMethod` string maps 1:1 to a C++ native function via `DECLARE_FUNCTION(execMethodName)`. Found across **136 unique UObject classes** including 9 SGW-specific classes:

- `USGWAnimController`
- `USGWAudioComponent`
- `USGWLoadingScreen`
- `USGWMeshCompositor`
- And 5 more

These provide **class::method names at VERY HIGH confidence**.

### 3. CME EventSignal MemberCallback RTTI (5,570 entries)

The CME framework's template-based event system generates RTTI strings that reveal:
- The **owning class** (e.g., `SGWNetworkManager`, `CombatQueue`, `GamePlayer`)
- The **event type handled** (e.g., `Event_NetOut_UseAbility`)
- The **method pointer type** (full signature including parameters)

This exposes **68 unique handler classes** with their method rosters. SGWNetworkManager alone has callbacks for 200+ distinct network events.

### 4. Fully Qualified C++ Method Signatures (~54 strings)

CME framework assertions embed complete decorated signatures:
```
struct CME::ErrorType __thiscall CME::Win32ReadWriteMutex::readLock(unsigned long)
bool __thiscall Detail::ZipStorageBase::OpenArchive(void)
unsigned int __stdcall CME::Win32ThreadEx::ThreadEntry<...>(void *)
```

### 5. Debug Log Context Strings (~200+)

Pattern: `"ClassName::MethodName: error message"` — provides function identity from error/debug logging.

## Coverage Estimate

| Evidence Source | Functions Identifiable | Confidence |
|----------------|----------------------|------------|
| Assertions mapping to .cpp files | ~5,500 | HIGH |
| UnrealScript exec stubs | ~1,017 | VERY HIGH |
| CME MemberCallback RTTI | ~2,500 unique methods | HIGH |
| RTTI vtable reconstruction | ~5,000+ classes | MEDIUM |
| C++ method signatures | ~54 | VERY HIGH |
| Debug log context strings | ~200+ | MEDIUM |
| **Subtotal from strings alone** | **~9,300** | |

### With Reference Source Amplification

| Category | Est. Functions | % of 169,420 |
|----------|---------------|-------------|
| Directly nameable from strings | ~9,300 | ~5.5% |
| Attributable to source file (via assertion xrefs) | ~15,000-20,000 | ~9-12% |
| Class-identifiable via RTTI (vtable analysis) | ~30,000-40,000 | ~18-24% |
| Identifiable with UE3 reference source | ~60,000-80,000 | ~35-47% |
| **Total potentially mappable** | **~100,000-120,000** | **~59-71%** |

## Key Amplifiers

### 1. Unreal Engine 3 Source (Available)
**Build 3717** (changelist 208373, package version 530, copyright 1998-2008) matches SGW's development window. Available from Internet Archive (`epic.jan2008`). Contains 719 .cpp files, 603 .h files, wxWidgets-based UnrealEd, D3D9+D3D10, GameSpy, Kismet, Matinee. All SGW-referenced source files confirmed present (`UnObj.cpp`, `UnActor.cpp`, `UnSequence.cpp`, `UnDistributions.cpp`, `UnInterpolation.cpp`, `UnUIStyleEditor.cpp`, etc.). Matching decompiled functions against this reference would identify tens of thousands of functions.

### 2. BigWorld Technology (Available — Open Source)
BigWorld was open-sourced after the company went bankrupt. Multiple versions available: [1.9.1](https://github.com/v2v3v4/BigWorld-Engine-1.9.1), [2.0.1](https://github.com/v2v3v4/BigWorld-Engine-2.0.1), 14.4.1 OSE. SGW uses ~1.9.x post-1.9.1 with CME custom modifications (see [bigworld-version-analysis.md](bigworld-version-analysis.md)). `servconn.cpp`, `entity_manager.cpp`, `nub.cpp`, `memory_stream.ipp` correspond to known source files that can be directly compared.

### 3. CEGUI (Available — Open Source)
478 RTTI entries, 438 real classes. Fully open-source: [github.com/cegui/cegui](https://github.com/cegui/cegui). Version tags `v0-6-0` through `v0-6-2` match SGW's development era (2007-2009). CEGUI 0.6.x was the current stable release during SGW development. Clone with `git clone --branch v0-6-2 https://github.com/cegui/cegui.git`. Source provides complete class definitions, rendering pipeline, widget implementations, and Lua scripting bindings — enabling direct comparison against all 438 decompiled classes.

### 4. Scaleform/GFx (Available — Archive.org)
271 real RTTI classes in sgw.exe. Full SDK source for **GFx 4.0.7** available on Internet Archive: [gfx_4.0.7_src_msvc90](https://archive.org/details/gfx_4.0.7_src_msvc90) (342 MB, MSVC 2008). While GFx 4.0.7 is newer than what SGW shipped with, the core class hierarchy (renderer, text, input, IME, XML parsing) should match closely — Scaleform maintained backward compatibility across minor versions. Covers Flash/SWF rendering, ActionScript VM, font rendering, and input handling classes. Autodesk discontinued Scaleform in 2017; no official source distribution remains.

### 5. Other Identifiable Libraries
| Library | RTTI Classes | Source Available |
|---------|-------------|-----------------|
| wxWidgets | 474 | Yes (open source) |
| PhysX/Novodex | 27 | Yes (open source) |
| TinyXML | 21 | Yes (open source) |
| Crypto++ | 14 | Yes (open source) |
| Intel TBB | 4 | Yes (open source) |
| OpenSSL | embedded | Yes (open source) |

## RTTI Domain Breakdown

### Total by Domain

| Domain | Total | Real (non-template) | Template |
|--------|-------|---------------------|----------|
| **SGW Game-Specific** | 1,624 | **1,188** | 436 |
| **Unreal Engine 3** | 2,594 | **2,189** | 405 |
| CME Framework | 3,117 | 12 | 3,105 |
| Third-Party Libraries | 1,523 | 1,259 | 264 |
| Uncategorized | 709 | 295 | 414 |

### SGW Game-Specific Breakdown (1,624 entries)

| Sub-category | Total | Real | Notes |
|---|---|---|---|
| Network Events (Event_NetOut/NetIn) | 421 | 421 | Client-server message types |
| Other Events (SlashCmd, etc.) | 337 | 337 | 256 slash commands + editor/misc |
| NetworkManager callbacks | 241 | 1 | 240 template callback registrations |
| Core Systems | 166 | 144 | SGWAudioDevice, SGWMeshCompositor, etc. |
| ScriptedWindow callbacks | 151 | 5 | 146 template callback registrations |
| UI Events | 149 | 149 | UI state change, HUD signals |
| CookedData types | 49 | 31 | Serialized game data types |
| Game Logic | 44 | 34 | Mission, Team, Trade, Minigame, etc. |
| Action Events | 33 | 33 | Keybindable actions (MoveForward, etc.) |
| System Options | 7 | 7 | Graphics/audio settings |
| Keybindings | 7 | 7 | Bindable action definitions |

**159 unique SGW-prefixed class names** identified, covering: audio, UI management, mesh/material compositing, animation controllers (30 `USGWAnim_*` classes), region/cover/spawn systems, networking, and login.

### Event Signal System (954 unique Event types)

| Category | Count |
|---|---|
| `Event_SlashCmd_*` | 256 |
| `Event_NetOut_*` | 254 |
| `Event_NetIn_*` | 167 |
| `Event_UI_*` | 149 |
| `Event_Editor_*` | 36 |
| `Event_Action_*` | 33 |
| `Event_Entity_*` | 9 |
| `Event_Option_*` | 8 |
| `Event_Player_*` | 6 |
| `Event_World_*` | 4 |
| Other (15 categories) | 32 |

Each event type produces ~3 template instantiations (MemberCallback, SignalRouter, etc.), explaining how 954 events generate 3,037 RTTI entries.

### Unreal Engine 3 Breakdown (2,594 entries)

| Sub-category | Total | Real |
|---|---|---|
| UObject classes (U* prefix) | 1,158 | 1,158 |
| F* Framework classes | 633 | 633 |
| Rendering (shaders, policies, proxies) | 630 | 225 |
| Actor classes (A* prefix) | 110 | 110 |
| HitProxy (H* prefix) | 53 | 53 |
| Debugger states | 10 | 10 |

## Vtable Tracing Results (Sample)

Successfully traced RTTI TypeDescriptor → RTTICompleteObjectLocator → vtable for these game classes:

| Class | TypeDescriptor | vtable | Ctor | Notes |
|---|---|---|---|---|
| `SGWAudioDevice` | 0x01db51e0 | 0x0183cfcc | FUN_0055f5f0 | FMOD EventSystem, MasterVolume/MusicVolume |
| `SGWUIManager` | 0x01db68e8 | 0x0183fb44 | FUN_0056ec30 | UI management |
| `Team` | 0x01e6dadc | 0x019eb1dc | FUN_00eb35e0 | 8+ vtable slots, inherits base at FUN_00e4c830 |
| `Mission` | 0x01e23a00 | 0x019bae7c | FUN_00d16af0 | CountedBaseTempl → CookedElementBase → Mission, size ≥ 0x6C |
| `NetworkEvent` | 0x01dab4b0 | 0x017fb990 | ~100 subclass ctors | Base for all network events |
| `DSBreakOnChange` | 0x01ddfa1c | 0x01910fb8 | FUN_009f2ab0 | Inherits FDebuggerState |

Ghidra auto-labels vtables from `RTTICompleteObjectLocator` chain (no `__vftable` string references in binary).

## Per-File Assertion Density (Key Files)

| Source File | Path | Assertions | RTTI Methods | Total Anchors |
|---|---|---|---|---|
| `entity_manager.cpp` | `Server\bigworld\src\client\` | 9 | 0 | 9 |
| `servconn.cpp` | `Server\bigworld\src\common\` | 16 | 0 | 16 |
| `CombatQueue.cpp` | `.\Src\` | 10 | 3 event handlers | 13+ |
| `SGWNetworkManager.cpp` | `.\Src\` | 2 | 200+ event handlers | 202+ |
| `BigWorldEntity.cpp` | `.\Src\` | 3 | 1 RTTI class | 4 |

### Top Files by Assertion Density

| File | Assertions | Domain |
|---|---|---|
| UnSequence.cpp | 161 | Kismet sequences |
| UnDistributions.cpp | 135 | Particle distributions |
| UnUIStyleEditor.cpp | 107 | UI style editing |
| UnInterpolation.cpp | 98 | Matinee interpolation |
| UnActor.cpp | 97 | Actor framework |
| UnObj.cpp | 92 | UObject base |
| SGWTextCommandManager.cpp | 48 | SGW slash commands |
| win32_sync_objects.cpp | 38 | CME threading |

## Architectural Findings

### 1. Dual UI Systems
Both **CEGUI** (438 real classes) and **Scaleform/GFx** (271 real classes) are linked. Suggests either a UI system transition mid-development or different UI layers for different purposes. Lua bindings for CEGUI confirmed (FUN_00a94910 = `CEGUI::Listbox::resetList` Lua wrapper).

### 2. CME Framework
The game uses the "CME" (Cheyenne Mountain Entertainment) framework extensively. EventSignal system alone accounts for 3,037 of 9,567 RTTI entries. CME provides: reference counting (`CountedBaseTempl`), property system (`PropertyNode`), thread primitives, event signals, and data containers.

### 3. NetworkEvent Hierarchy
The `NetworkEvent` base class has ~100 direct subclass constructors, making it the largest game-specific class hierarchy. Combined with the 421 `Event_NetOut/NetIn` signal types, this represents the core client-server protocol.

### 4. Inheritance Chains
Verified: `CME::CountedBaseTempl` → `CookedElementBase` → `Mission`, confirming a data-driven content pipeline where game content types extend a serializable base.

## Header Reconstruction Estimate

| Category | Headers Needed | Complexity |
|---|---|---|
| SGW game classes (159 unique) | ~40-60 headers | HIGH — game-specific logic |
| SGW Event signals (954 types) | ~20-30 headers | LOW — mostly trivial structs |
| CookedData types (31 real) | ~5-10 headers | MEDIUM — serialization |
| NetworkEvent subclasses (~100) | ~10-15 headers | HIGH — protocol definitions |
| CME framework (12 real + patterns) | ~5-8 headers | MEDIUM — can be stubbed |
| **Total game-specific headers** | **~80-125** | |

The 2,189 UE3 classes and 1,259 library classes do NOT need reconstruction — they come from SDK headers and third-party source.

## Recommended Reconstruction Approach

### Phase 1: Automated Extraction
1. Script Ghidra to extract ALL assertion xrefs → map functions to source files
2. Extract ALL RTTI vtables → map virtual methods to classes
3. Extract ALL exec stubs → map native UnrealScript functions
4. Extract ALL MemberCallback RTTI → map event handlers to classes

### Phase 2: Reference Source Matching
1. UE3 Build 3717 (`epic.jan2008`) → match all `Un*.cpp` files (~185 source files, ~2,189 classes)
2. BigWorld 1.9.1/2.0.1 open-source → match `servconn.cpp`, `entity_manager.cpp`, `nub.cpp`, etc.
3. CEGUI v0-6-2 from GitHub → match all 438 UI library classes
4. Scaleform GFx 4.0.7 from Archive.org → match 271 Flash/SWF rendering classes
5. Match wxWidgets, TinyXML, PhysX, OpenSSL, Crypto++ from open-source repos

### Phase 3: Game-Specific Reconstruction
1. Decompile SGW-specific classes using Ghidra
2. Organize into original file structure using assertion evidence
3. Reconstruct headers from RTTI + vtable analysis
4. Clean decompiled output into readable C++

### Phase 4: Verification
1. Compare decompiled function sizes against originals
2. Verify vtable layouts match RTTI chains
3. Cross-reference assertion line numbers for ordering within files
4. Build against UE3 SDK headers for type compatibility

## Reference Source Inventory

All major middleware components now have identified reference source:

| Component | RTTI Classes | Version Match | Source Location | Coverage |
|-----------|-------------|---------------|-----------------|----------|
| **Unreal Engine 3** | 2,189 real | Build 3717 (CL 208373) | Internet Archive `epic.jan2008` | 719 .cpp, 603 .h |
| **BigWorld Technology** | ~50 | ~1.9.x post-1.9.1 | [GitHub 1.9.1](https://github.com/v2v3v4/BigWorld-Engine-1.9.1) + [2.0.1](https://github.com/v2v3v4/BigWorld-Engine-2.0.1) | Full client+common libs |
| **CEGUI** | 438 real | 0.6.x (v0-6-0 to v0-6-2) | [GitHub](https://github.com/cegui/cegui) tag `v0-6-2` | Full library source |
| **Scaleform/GFx** | 271 real | ~3.x-4.x | [Archive.org GFx 4.0.7](https://archive.org/details/gfx_4.0.7_src_msvc90) | Full SDK source (342 MB) |
| **wxWidgets** | 474 | 2.8.x era | Open source | Full library source |
| **PhysX/Novodex** | 27 | 2.x era | Open source (NVIDIA) | Full SDK source |
| **TinyXML** | 21 | 2.x | Open source | Full library source |
| **Crypto++** | 14 | 5.x era | Open source | Full library source |
| **Intel TBB** | 4 | 2.x era | Open source (oneAPI) | Full library source |
| **OpenSSL** | embedded | 0.9.x era | Open source | Full library source |

**Third-party coverage**: 3,498 of 3,498 third-party real classes (100%) now have identified reference source. Combined with 2,189 UE3 classes, **5,687 of 9,567 RTTI entries** (59%) can be matched against known source code without any decompilation.

## Bottom Line

This binary is **exceptionally well-suited for source reconstruction**:
- Three separate assertion systems embed filenames
- Massive RTTI from C++ template instantiations
- 1,017 named exec function stubs
- **All major middleware has identified reference source** — UE3, BigWorld, CEGUI, Scaleform/GFx, wxWidgets, PhysX
- **60-70% of ~169,420 functions can plausibly be mapped to original source files**
- Remaining 30-40% is compiler-generated code (template instantiations, exception handlers, thunks, CRT startup, inlined functions)
