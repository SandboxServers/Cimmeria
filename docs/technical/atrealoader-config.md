# AtreaLoader.config.xml — Binary Patch Definitions

> [!NOTE]
> **Scope banner — companion to `atrea-editor.md`, not superseded by it.**
> The top-level entry point for the whole Atrea toolchain — including the in-game
> UnrealEd editor that these patches unlock inside `SGW.exe` — is
> [`docs/reverse-engineering/findings/atrea-editor.md`](../reverse-engineering/findings/atrea-editor.md).
> Read that first for orientation, then return here for the patch/symbol/NVP table.
>
> `atrea-editor.md` describes this page as one of an "apocryphal trio" that it supersedes.
> **That framing did not survive the 2026-07-25 audit.** This page was *revised* in the
> same 2026-05-25 campaign — `atrea-editor.md`'s own §"Apocryphal docs to retire" table
> records the corrections applied here (patch count `19` → `18`, the `-SHOWLOG` removal,
> the `STD_OUTPUT_HANDLE = -11` arithmetic) and its §Cross-references cites this page as a
> live "Ghidra-anchored Editor-group patches" reference. It is the only place transcribing
> the non-editor patch groups (Debug, AppearanceLogging, UCC, Mercury), the 13-symbol hook
> table, and the NVP settings. Treat the two as complementary halves.

> **Last updated**: 2026-07-25 (accuracy audit — scope banner added; symbol-table RVA/VA mixing documented)
> **Previously revised**: 2026-05-25
> **Audience**: reverse engineers + emulator developers touching the SGW.exe patch surface
> **Doc type**: reference (transcription of the patch/symbol/NVP tables with binary-level annotations)
> **Status**: revised against the Wave 2 byte-verified Editor-group table in [atrea-editor.md](../reverse-engineering/findings/atrea-editor.md)
> **Source of truth**: `binaries/AtreaLoader.config.xml` (editable XML) — but see "Runtime read vs. editable source" below

Complete analysis of the AtreaRL configuration that defines all binary patches, symbol hooks, and runtime settings applied to `SGW.exe` at load time.

## Runtime read vs. editable source

The loader reads the *binary* file `binaries/AtreaLoader.config` at runtime. The companion `AtreaLoader.config.xml` is the editable source — changes to the XML do **not** take effect until the binary form is regenerated. On at least one observed installation the binary `.config` predated the XML by over a year (binary 2013-05-20 vs. XML 2014-06-23), and the two had drifted on the `Sniffer` NVP (see [mercury-wire-format.md §S9](../drafts/spec/mercury-wire-format.md)). When editing patches, regenerate the binary or your changes will appear to do nothing.

## Modern-Windows prerequisite — `AtreaFixASLR.bat`

Every absolute virtual address in this config assumes `SGW.exe` loads at its PE-declared preferred base `0x00400000`. On modern Windows, the image instead loads at an ASLR-randomized base because the PE header has `IMAGE_DLLCHARACTERISTICS_DYNAMIC_BASE` (the `0x40` bit at file offset `0x186`). Under ASLR, all `OriginalBytes` patterns miss and the session log shows `0 patch(es) of N applied`.

Run `AtreaFixASLR.bat` once before first use. It flips the single byte at file offset `0x186` from `0x40` to `0x00`, clearing `DYNAMIC_BASE` so the image loads at `0x00400000` and the absolute addresses resolve. Full analysis: [mercury-wire-format.md §S9](../drafts/spec/mercury-wire-format.md).

## Configuration structure

The XML has four sections:

1. **Patches** — Binary byte replacements at specific addresses.
2. **Symbols** — Named function/data addresses for hooking.
3. **NVPs** (Name-Value Pairs) — Runtime settings.
4. **PathSubstitutions** — File path redirection.

## Patch groups

Patches are organized into groups, selectable via `AtreaLoader.exe` command line:

- `--enable-group=<name>` enables a group.
- `--disable-group=<name>` disables a group.

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

### Batch files

```text
AtreaEditor.bat:     AtreaLoader --enable-group=Editor
AtreaFixASLR.bat:    AtreaLoader --fix-aslr
AtreaGameDebug.bat:  AtreaLoader --enable-group=Mercury -SHOWLOG -LOG
```

`AtreaEditor.bat` does **not** pass `-SHOWLOG` (only `AtreaGameDebug.bat` does). Earlier revisions of this doc claimed it did; that was wrong.

## Binary patches (18 total)

The XML contains 18 `<Patch>` entries. All Editor-group patches in the table below have been byte-verified at their target addresses (Wave 2 verification pass). The "Function" column cites the Ghidra-recovered containing function in `SGW.exe`; the RVA + image base `0x00400000` gives the absolute VA used by the loader.

### Debug / logging patches

| Patch | Address | Description |
|-------|---------|-------------|
| **EnableUnicodeLogger** | `0x01AF2224` | `00→01` — Enables BigWorld Mercury message logging. |
| **EnableLocalizedDebug** | `0x01AF28C0` | `00→01` — Enables localization token debug parsing. |
| **EnableAppearanceLogger** | `0x01AF22F4` | `00→01` — Enables character appearance job logging. |

These are simple boolean flag toggles in `SGW.exe`'s `.data` section.

### Editor-group patches (byte-verified)

Image-relative RVAs; absolute VA = RVA + `0x00400000`. Editor-group patches apply when `--enable-group=Editor` is passed (default for `AtreaEditor.bat`). Sourced from the Wave 2 verified table in [atrea-editor.md](../reverse-engineering/findings/atrea-editor.md) §"Patch reference (Editor group)".

| Patch | RVA | Containing function | Effect |
|---|---|---|---|
| **EditorMode** | `0x00018AF0` | `FUN_004185e0` (ParseCommandLine/InitGlobals) | Swap MOV-source bytes (`89 35 BC D7 EA 01` ↔ `89 1D AC D7 EA 01` etc.) so `GIsServer=1`, `GIsEditor=1`, `GIsGame=0`. ESI=1, EBX=0 at this point. |
| **EditorCallbacks** | `0x000186D2` | inside `FUN_004185e0` | Swap 4 `PUSH imm32` args to engine init so it installs editor-flavored `FCallbackEventDeviceEditor`, `FCallbackQueryDeviceEditor`, `FFeedbackContextEditor`, `FOutputDeviceFile`. |
| **EditorCallbackVMT** | `0x0198F52C` | `.data` VMT slot | Rewrite VMT ptr from game (`0x017F8D80`) to editor (`0x017F8DD8`) variant. |
| **EditorCurrentPackage** | `0x0198F4A0` | `.data` UTF-16 string | Replace `L"Launch"` with `L"UnrealEd"`. Drives `UObject::CreatePackage`, `FOutputDeviceFile` log naming. |
| **EditorSettings** | `0x001757BA` | `FUN_00575730` (config-string parser) | `SETZ DL` → `SETNZ DL` — inverts the `wcsicmp(cfg, L"EDITOR")` test so the editor-settings bool at struct offset `+0x28` is set regardless of config. |
| **EditorUnknownUi** | `0x00166919` | `FUN_00566910` | `CMP [ESP+0x4], 0` immediate `00 → 01` — forces editor-UI branch even when called with arg=0. |
| **DisablePrefabSerialize** | `0x001CE8E1` | `FUN_005CE7E0` (prefab serializer) | `JGE 0x005CE9B6` → `NOP; JMP` unconditional — always skips the prefab `Serialize()` loop, preventing partial re-serialization on map open. |
| **EditorSplash** | `0x013FA350` | `.data` UTF-16 string (`Splash` group) | Replace `PC\EdSplash.bmp` with `PC\Splash.bmp` (despite the patch name, this *removes* the editor splash). |
| **EditorChunkLimit** | `0x007FDA41` | editor map-load engine method | `Editor-Disabled` group — `JLE` → `JMP` removes the "chunk count ≤ 100" guard. |
| **EditorMyGamesDir** | `0x0008D1E8` | `FUN_0048D080` | `Editor-Disabled` group — NOP the `chdir` into `My Games\FireSky\SGWGame` so packages save next to the binary. |
| **HideEditorBrowserPane** | `0x00AD56BA` | `WxUnrealEdApp::vfunc_25` chain | `Silent` group — NOP `BrowserPane::Show(true)` virtual call. |
| **HideEditorBrowserPane2** | `0x00B5E789` | `WxUnrealEdApp::vfunc_25` chain | `Silent` group — NOP second `BrowserPane::Show(true)` virtual call. |
| **HideEditorWindow** | `0x00B5F639` | `FUN_00F5F580` | `Silent` group — NOP `WxEditorFrame::Show(true)`. |

After all Editor-group patches apply: `GIsClient=1, GIsServer=1, GIsEditor=1, GIsUCC=0, GIsGame=0`. The engine self-identifies as a single-process editor (client + server + editor, not game).

### UCC commandlet mode patch

**Address**: `0x00018AF0` (same EditorMode location, different byte pattern applied when `--enable-group=UCC` is passed)

| Flag | Normal | UCC mode |
|------|--------|----------|
| GIsClient | 1 | **0** |
| GIsServer | 0 | 0 |
| GIsEditor | 0 | 0 |
| GIsUCC | 0 | **1** |
| GIsGame | 1 | **0** |

### ConsoleStdHandle (UCC console fix)

**Address**: `0x000CC91F` (absolute VA `0x004CC91F`)

Fixes the console handle passed to `GetStdHandle()` so console output works correctly in UCC commandlet mode.

**Patch bytes**: `6A F5` followed by NOPs. `6A imm8` is `PUSH imm8` with the byte sign-extended to 32 bits; `F5h` sign-extended is `0xFFFFFFF5`, which is **`-11` as a signed 32-bit integer**.

The standard handle constants in `winbase.h`:

| Handle | Value |
|---|---|
| `STD_INPUT_HANDLE` | `-10` (`0xFFFFFFF6`) |
| `STD_OUTPUT_HANDLE` | `-11` (`0xFFFFFFF5`) |
| `STD_ERROR_HANDLE` | `-12` (`0xFFFFFFF4`) |

So `PUSH -11` selects `STD_OUTPUT_HANDLE`. The patch substitutes a `GetStdHandle(STD_OUTPUT_HANDLE)` call sequence in place of whatever the original code path emitted (the original bytes pushed a different handle constant — likely `STD_INPUT_HANDLE` (`-10`, `6A F6`), which would have been wrong for a console-write context). Trailing NOPs pad out the remainder of the original instructions so subsequent control flow stays aligned. The net effect: UCC commandlet output flows to the console instead of being silently dropped.

Earlier revisions of this doc said "STD_OUTPUT_HANDLE = -11 → uses -5 for STD_ERROR_HANDLE" — that was wrong twice over. `STD_ERROR_HANDLE` is `-12`, not `-5`; and the patched value is `-11`, which **is** `STD_OUTPUT_HANDLE`, not a substitution away from it.

### Engine-mode globals (UE3 standard, confirmed in this build)

| Flag | VA | Editor value | Notes |
|---|---|---|---|
| `GIsClient` | `0x01EAD7BC` | 1 | UClass/UObject readers, ~50 xrefs |
| `GIsServer` | `0x01EAD7C0` | 1 | UGameEngine readers, gated server path |
| `GIsEditor` | `0x01EAD7AC` | 1 | Atrea patch target |
| `GIsUCC` | `0x01EAD7B0` | 0 | UnrealScript compiler mode (off in editor); asserted at `0x0049F714` |
| `GIsGame` | `0x01EB0830` | 0 | FEdObjectPropagator toggles, ACoverLink/FTerrainObject readers |

## Symbol hooks (13 total)

AtreaRL.dll hooks these functions/addresses in `SGW.exe`:

| Symbol name | Address | Group | Patch | Purpose |
|-------------|---------|-------|-------|---------|
| **UnicodeLoggerStart** | `0x00866860` | (default) | true | BigWorld entity event logging — start |
| **UnicodeLoggerParam** | `0x00866880` | (default) | true | BigWorld entity event logging — parameter |
| **UnicodeLoggerEnd** | `0x00866870` | (default) | true | BigWorld entity event logging — end |
| **AppearanceLoggerWchar** | `0x000250D0` | AppearanceLogging | false | Character appearance (wchar) |
| **AppearanceLoggerWstring** | `0x00304750` | AppearanceLogging | false | Character appearance (wstring) |
| **AnsiLogger** | `0x00635210` | (default) | true | SGW generic ANSI logger |
| **MercuryLogger** | `0x0041C2E0` | Mercury | false | BigWorld Mercury protocol debug |
| **UnrealAssertionLogger** | `0x00086000` | (default) | true | UE3 `check()`/`verify()` handler |
| **FFileManager::MoveFile** | `0x000C43A0` | Editor | false | File move intercept (editor fix) |
| **UPrefab::Serialize** | `0x00812D30` | EditorDebugPrefab | false | Prefab debug (unused) |
| **FArchive::PostLoad** | `0x000E9870` | EditorPartialSerializePrefabs | false | Post-load hook (OLD, DO NOT USE) |
| **UObject::Serialize** | `0x000A42F0` | EditorDebugPrefab | false | Object serialization debug (huge logs) |
| **FName::GNames** | `0x01ACADE0` | — | — | Global name table (data address, not function) |

`Patch=true` means AtreaRL replaces the function with its own implementation. `Patch=false` means AtreaRL hooks (wraps) the function, calling the original after logging.

> **The "Address" column above mixes RVAs and absolute VAs — verified 2026-07-25.**
> The XML transcribes whatever each `<Symbol>` entry declares, and those entries are not
> uniform. The five with leading zeros are **image-relative**; add `0x00400000` to resolve:
>
> | Declared | Resolves to | What is actually there |
> |---|---|---|
> | `0x00086000` | `0x00486000` | `FUN_00486000(char*, char*, undefined4)` — function entry; the `(Expr, File, Line)` signature matches `appFailAssertFunc` |
> | `0x000C43A0` | `0x004C43A0` | `FUN_004c43a0(this, LPCWSTR, …)` — function entry, consistent with `FFileManager::MoveFile` |
> | `0x000E9870` | `0x004E9870` | `FUN_004e9870(this, int*)` — function entry |
> | `0x000A42F0` | `0x004A42F0` | `UTestIpDrv__vfunc_12` — a `Serialize` vtable slot, consistent with `UObject::Serialize` |
> | `0x000250D0` | `0x004250D0` | interior of `FUN_004250b0` (not an entry) — a mid-function hook point |
>
> The remaining entries (`0x00866860`/`70`/`80`, `0x00635210`, `0x0041C2E0`, `0x00812D30`,
> `0x00304750`, `0x01ACADE0`) are already absolute VAs. Reading the whole column as VAs —
> the natural assumption — puts five of them below `.text`'s `0x00401000` floor or in the
> wrong function entirely.

### Notable addresses for RE

- `0x00866860`–`0x00866880` — three interior hook points inside a **single** function,
  `FUN_00866850` (body `0x00866850`–`0x00866894`, 69 bytes). They are the start / parameter-emit /
  end phases of one BigWorld entity-event log call, not three separate functions.
  *(Corrected 2026-07-25 — previously described as "functions", plural.)*
- `0x00635210` — SGW's main ANSI debug logger. Confirmed a function entry (`FUN_00635210`).
- `0x0041C2E0` — **not a function entry.** It is an interior address inside
  `FFeedbackContextWindows__vfunc_1` (body `0x0041C0C0`–`0x0041C43F`), UE3's
  `FFeedbackContext::Serialize` log sink. Hooking here intercepts log output as it is
  written; there is no distinct "Mercury debug output function" at this address.
  *(Corrected 2026-07-25.)*
- `0x00486000` (declared as RVA `0x00086000`) — UE3 assertion handler (`appFailAssertFunc`
  equivalent). *(Corrected 2026-07-25 — `0x00086000` read as a VA falls below the `.text`
  floor at `0x00401000` and resolves to nothing.)*
- `0x01ACADE0` — `FName::GNames`, the global UE3 name hash table. Data, not a function.

## NVP settings

| Name | Default | Purpose |
|------|---------|---------|
| **Sniffer** | `true` | Enable packet sniffer (PCAP + AES key capture). See [mercury-wire-format.md §S9](../drafts/spec/mercury-wire-format.md) for the gate that prevents this from taking effect on modern Windows without an extra `AtreaRL.dll` patch. |
| **ExitOnAssert** | `false` | Terminate process on assertion failure. |
| **IgnoreBulkDataErrors** | `false` | Suppress assertion dialog on bulk data serialization errors. |
| **DisableErrorReporting** | `false` | Suppress Windows Error Reporting dialog on crash. |

## Path substitutions

```xml
<PathSubstitutions>
    <!-- <Path Pattern="SGWGame\Content\UI" RootReplacement="D:\Dev\WUI\UI" /> -->
</PathSubstitutions>
```

Commented out, but the mechanism is documented in source: AtreaRL can redirect filesystem paths at runtime, allowing developers to load UI content from a development directory (`D:\Dev\WUI\UI`) instead of the game installation. The `CreateFileA` / `CreateFileW` hooks in AtreaRL implement this redirection.

## Emulator implications

1. **Editor mode is fully unlockable.** The `EditorMode` + companion patches enable the wxWidgets-based UnrealEd, allowing map viewing/editing with SGW's content. For what the Editor group of patches actually unlocks (UnrealEd inside `SGW.exe`), see [atrea-editor.md](../reverse-engineering/findings/atrea-editor.md). For the SGW.exe addresses targeted by editor-group patches, see [editor-source-mapping.md](../reverse-engineering/editor-source-mapping.md).
2. **UCC commandlet mode** enables Unreal command-line tools for content cooking, package inspection, etc.
3. **Mercury logging.** The `MercuryLogger` hook at `0x0041C2E0` provides BigWorld protocol debug output — extremely valuable for protocol RE.
4. **All addresses are absolute.** Patches assume `SGW.exe` loads at its preferred base address. ASLR must be disabled via `AtreaFixASLR.bat` once per install; see prerequisite note above.
5. **Path substitution** could be used to redirect content loading to modded or emulator-specific files.
6. **Sniffer enabled by default** — but does not actually capture on modern Windows without the `AtreaRL.dll` runtime patch documented in [mercury-wire-format.md §S9](../drafts/spec/mercury-wire-format.md).

## Cross-references

- [docs/reverse-engineering/findings/atrea-editor.md](../reverse-engineering/findings/atrea-editor.md) — what the Editor group of patches actually unlocks (UnrealEd inside `SGW.exe`); top-level editor archaeology.
- [docs/reverse-engineering/editor-source-mapping.md](../reverse-engineering/editor-source-mapping.md) — function-by-function VA-to-source map for the SGW.exe addresses targeted by editor-group patches.
- [docs/drafts/spec/mercury-wire-format.md §S9](../drafts/spec/mercury-wire-format.md) — ASLR prerequisite (`AtreaFixASLR.bat`) and the sniffer-gate runtime patch in `AtreaRL.dll`.
- [docs/technical/atrealoader-exe.md](atrealoader-exe.md) — AtreaLoader.exe injector analysis.
- [docs/technical/atrearl-loader.md](atrearl-loader.md) — AtreaRL.dll loader analysis.
- Source of truth (runtime-read binary): `binaries/AtreaLoader.config` (regenerated from `AtreaLoader.config.xml`).
