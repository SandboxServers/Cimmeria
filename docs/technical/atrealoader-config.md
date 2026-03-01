# AtreaLoader.config.xml — Binary Patch Definitions

Complete analysis of the AtreaRL configuration file that defines all binary patches, symbol hooks, and runtime settings applied to SGW.exe at load time.

## Configuration Structure

The XML config has four sections:
1. **Patches** — Binary byte replacements at specific addresses
2. **Symbols** — Named function/data addresses for hooking
3. **NVPs** (Name-Value Pairs) — Runtime settings
4. **PathSubstitutions** — File path redirection

## Patch Groups

Patches are organized into groups, selectable via AteraLoader.exe command line:
- `--enable-group=<name>` enables a group
- `--disable-group=<name>` disables a group

| Group | Purpose | Default |
|-------|---------|---------|
| Mercury | BigWorld Mercury protocol logging | Enabled via bat |
| Debug | Localization debug logging | Apply=true |
| AppearanceLogging | Character appearance logging | Apply=false |
| Editor | UnrealEd editor mode | Enabled via bat |
| UCC | UCC commandlet mode | Apply=false |
| Splash | Editor splash screen | Apply=false |
| Silent | Hide editor UI panes | Apply=false |
| Editor-Disabled | Extra editor patches (chunk limit, My Games dir) | Apply=false |

### Batch Files

```
AtreaEditor.bat:     AtreaLoader --enable-group=Editor -SHOWLOG
AtreaFixASLR.bat:    AtreaLoader --fix-aslr
AtreaGameDebug.bat:  AtreaLoader --enable-group=Mercury -SHOWLOG -LOG
```

## Binary Patches (19 total)

### Debug/Logging Patches

| Patch | Address | Description |
|-------|---------|-------------|
| **EnableUnicodeLogger** | 0x01AF2224 | `00→01` — Enables BigWorld Mercury message logging |
| **EnableLocalizedDebug** | 0x01AF28C0 | `00→01` — Enables localization token debug parsing |
| **EnableAppearanceLogger** | 0x01AF22F4 | `00→01` — Enables character appearance job logging |

These are simple boolean flag toggles in SGW.exe's .data section.

### Editor Mode Patch (GIsServer/GIsEditor/GIsUCC/GIsGame)

**Address**: 0x00018AF0 (relative to image base)

This patch modifies the UE3 global flag initialization to enable UnrealEd:

| Offset | Original | Patched | Effect |
|--------|----------|---------|--------|
| +0x00 (GIsClient) | `89 35` (mov [GIsClient], esi=1) | `89 35` (unchanged) | Client=1 |
| +0x06 (GIsServer) | `89 1D` (mov [GIsServer], ebx=0) | `89 35` (mov [GIsServer], esi=1) | Server=0→**1** |
| +0x0C (GIsEditor) | `89 1D` (mov [GIsEditor], ebx=0) | `89 35` (mov [GIsEditor], esi=1) | Editor=0→**1** |
| +0x12 (GIsUCC) | `89 1D` (mov [GIsUCC], ebx=0) | `89 1D` (unchanged) | UCC=0 |
| +0x18 (GIsGame) | `89 35` (mov [GIsGame], esi=1) | `89 1D` (mov [GIsGame], ebx=0) | Game=1→**0** |

The trick: ESI=1 and EBX=0 at this point. Swapping `89 1D` (store EBX=0) with `89 35` (store ESI=1) flips the flag. Net result:

| Flag | Normal | Editor Mode |
|------|--------|-------------|
| GIsClient | 1 | 1 |
| GIsServer | 0 | **1** |
| GIsEditor | 0 | **1** |
| GIsUCC | 0 | 0 |
| GIsGame | 1 | **0** |

### UCC Commandlet Mode Patch

**Address**: 0x00018AF0 (same location, different byte pattern)

| Flag | Normal | UCC Mode |
|------|--------|----------|
| GIsClient | 1 | **0** |
| GIsServer | 0 | 0 |
| GIsEditor | 0 | 0 |
| GIsUCC | 0 | **1** |
| GIsGame | 1 | **0** |

### UCC Console Fix

**Address**: 0x000CC91F

Fixes the console stdout handle for UCC commandlet mode:
- Replaces the original console setup code with `PUSH -5; CALL GetStdHandle` (STD_OUTPUT_HANDLE = -11 → uses -5 for STD_ERROR_HANDLE)
- NOPs out the remaining original bytes
- Allows console output to work correctly in commandlet mode

### Editor Settings Flag

**Address**: 0x001757BA

Changes `SETZ DL` (0F 94) to `SETNZ DL` (0F 95) — inverts the result of a `wcsicmp("EDITOR")` comparison, enabling editor-specific system configuration.

### Editor Callback VMT

**Address**: 0x0198F52C

Replaces the GCallback vtable pointer from 0x017F8DXX to 0x017F8DD8, switching to the editor-specific callback implementations (`FCallbackEventDeviceEditor`, `FCallbackQueryDeviceEditor`, `FFeedbackContextEditor`).

### Editor Callbacks

**Address**: 0x000186D2

Replaces the pushed addresses for callback/feedback context objects:
- Original: game-mode callback objects at 0x01D8F534, 0x01EA4B70, 0x01EA4874, 0x01EA4818
- Patched: editor-mode objects at 0x01D8F52C, 0x01EA4898, 0x01EA4874, 0x01EA4848

### Editor Chunk Limit

**Address**: 0x007FDA41

Changes `JLE` (7E) to `JMP` (EB) — removes the 100-chunk upper limit for opening chunked maps in the editor.

### Editor My Games Dir

**Address**: 0x0008D1E8

NOPs out `CALL EDX` (FF D2) — prevents the game from changing the working directory to `My Games\FireSky\SGWGame`, keeping packages saved relative to the install directory.

### Editor Unknown UI

**Address**: 0x00166919

Changes a CMP operand from 0x00 to 0x01, enabling an editor UI element.

### Editor Current Package

**Address**: 0x0198F4A0

Replaces the Unicode string `"Launch"` with `"UnrealEd"` — changes the current package name from the game launcher to the editor.

### Disable Prefab Serialize

**Address**: 0x001CE8E1

Changes `JGE rel32` (0F 8D) to `NOP; JMP rel32` (90 E9) — skips prefab serialization in `UEditor::LoadPrefabs`. Prevents editor crashes when loading maps with prefabs.

### Editor Splash

**Address**: 0x013FA350

Replaces Unicode path `"PC\EdSplash.bmp"` with `"PC\Splash.bmp"` — redirects the editor splash screen to the game splash.

### Hide Editor Browser/Window Panes

| Patch | Address | Effect |
|-------|---------|--------|
| HideEditorBrowserPane2 | 0x00B5E789 | NOPs out `PUSH 1; CALL EAX` (ShowWindow) |
| HideEditorBrowserPane | 0x00AD56BA | NOPs out `PUSH ESI; CALL EAX` (ShowWindow) |
| HideEditorWindow | 0x00B5F639 | NOPs out `PUSH 1; CALL EAX` (ShowWindow) |

These suppress editor window creation for "Silent" mode (headless editor).

## Symbol Hooks (13 total)

AtreaRL.dll hooks these functions/addresses in SGW.exe:

| Symbol Name | Address | Group | Patch | Purpose |
|-------------|---------|-------|-------|---------|
| **UnicodeLoggerStart** | 0x00866860 | (default) | true | BigWorld entity event logging — start |
| **UnicodeLoggerParam** | 0x00866880 | (default) | true | BigWorld entity event logging — parameter |
| **UnicodeLoggerEnd** | 0x00866870 | (default) | true | BigWorld entity event logging — end |
| **AppearanceLoggerWchar** | 0x000250D0 | AppearanceLogging | false | Character appearance (wchar) |
| **AppearanceLoggerWstring** | 0x00304750 | AppearanceLogging | false | Character appearance (wstring) |
| **AnsiLogger** | 0x00635210 | (default) | true | SGW generic ANSI logger |
| **MercuryLogger** | 0x0041C2E0 | Mercury | false | BigWorld Mercury protocol debug |
| **UnrealAssertionLogger** | 0x00086000 | (default) | true | UE3 `check()`/`verify()` handler |
| **FFileManager::MoveFile** | 0x000C43A0 | Editor | false | File move intercept (editor fix) |
| **UPrefab::Serialize** | 0x00812D30 | EditorDebugPrefab | false | Prefab debug (unused) |
| **FArchive::PostLoad** | 0x000E9870 | EditorPartialSerializePrefabs | false | Post-load hook (OLD, DO NOT USE) |
| **UObject::Serialize** | 0x000A42F0 | EditorDebugPrefab | false | Object serialization debug (huge logs) |
| **FName::GNames** | 0x01ACADE0 | — | — | Global name table (data address, not function) |

**Patch=true** means AtreaRL replaces the function with its own implementation. **Patch=false** means AtreaRL hooks (wraps) the function, calling the original after logging.

### Notable Addresses for RE

- **0x00866860-0x00866880**: BigWorld entity event logging functions — these are the CME-added wrappers around BigWorld's event system
- **0x00635210**: SGW's main ANSI debug logger
- **0x0041C2E0**: Mercury protocol debug output function
- **0x00086000**: UE3 assertion handler (`appFailAssertFunc` equivalent)
- **0x01ACADE0**: `FName::GNames` — the global UE3 name hash table

## NVP Settings

| Name | Default | Purpose |
|------|---------|---------|
| **Sniffer** | `true` | Enable packet sniffer (PCAP + AES key capture) |
| **ExitOnAssert** | `false` | Terminate process on assertion failure |
| **IgnoreBulkDataErrors** | `false` | Suppress assertion dialog on bulk data serialization errors |
| **DisableErrorReporting** | `false` | Suppress Windows Error Reporting dialog on crash |

## Path Substitutions

```xml
<PathSubstitutions>
    <!-- <Path Pattern="SGWGame\Content\UI" RootReplacement="D:\Dev\WUI\UI" /> -->
</PathSubstitutions>
```

Commented out, but reveals the mechanism: AtreaRL can redirect file system paths at runtime, allowing developers to load UI content from a development directory (`D:\Dev\WUI\UI`) instead of the game installation. The `CreateFileA`/`CreateFileW` hooks in AtreaRL implement this redirection.

## Emulator Implications

1. **Editor mode is fully unlockable**: The GIsEditor patch enables the wxWidgets-based UnrealEd, allowing map viewing/editing with SGW's content
2. **UCC commandlet mode**: Enables Unreal command-line tools for content cooking, package inspection, etc.
3. **Mercury logging**: The `MercuryLogger` hook at 0x0041C2E0 provides BigWorld protocol debug output — extremely valuable for protocol RE
4. **All addresses are absolute**: Patches assume SGW.exe loads at its preferred base address (ASLR must be disabled via `AtreaLoader --fix-aslr`)
5. **Path substitution**: Could be used to redirect content loading to modded/emulator-specific files
6. **Sniffer enabled by default**: Every AtreaRL-loaded session captures PCAP traffic and AES keys to the `sessions/` directory
