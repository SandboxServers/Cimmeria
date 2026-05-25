# AtreaLoader.exe — DLL Injector

> **Last updated**: 2026-05-25
> **Audience**: Engineers reverse-engineering the SGW client toolchain or building tooling that interoperates with the AtreaLoader launch path.
> **Type**: Reference (Diataxis)
> **Status**: Verified for the injector pipeline; the binary `.config` runtime-format claim is documented but unconfirmed by direct decompile here — see notes inline.

This doc covers the **launcher half** of the community-built Atrea toolchain — the small native executable that starts `SGW.exe` and injects `AtreaRL.dll` into it. It does *not* cover the in-game editor that activates downstream once the patches are applied, nor the MCP bridge proposal that drives it.

- For the in-game editor (UnrealEd inside SGW.exe), see [docs/reverse-engineering/findings/atrea-editor.md](../reverse-engineering/findings/atrea-editor.md).
- For the MCP-server architecture proposal that drives the editor, see [docs/architecture/atrea-editor-bridge.md](../architecture/atrea-editor-bridge.md).
- For the user-facing how-to (which `.bat` to double-click, what each one does), see [docs/client-tools.md](../client-tools.md).
- For the ASLR-fix wire-format context (why one byte at file offset `0x186` is what matters), see [docs/drafts/spec/mercury-wire-format.md](../drafts/spec/mercury-wire-format.md) §S9.

A note on naming: the binary on disk is **`AtreaLoader.exe`** (a-t-r-e-a, not a-t-e-r-a). Earlier drafts of this doc misspelled it as "AteraLoader" throughout; that spelling is fixed here, and the surrounding doc tree was swept. The "RL" in `AtreaRL` stands for "Remote Library" — confirmed by the binary's own error message: *"the remote library loader (AtreaRL.dll) reported an internal error"*.

## Overview

| Property | Value |
|----------|-------|
| **Type** | Win32 EXE (32-bit) |
| **Base address** | `0x00400000` |
| **Code size** | ~55 KB (`.text`: `0x00401000`–`0x0040e7ff`) |
| **Compiler** | MSVC (static CRT) |
| **DLL imports** | `KERNEL32.DLL` (49 functions), `USER32.DLL` (1 function) |
| **Export** | `entry` at `0x00403041` |
| **Manifest** | `requestedExecutionLevel level='asInvoker'` (no admin required) |
| **Security** | `/GS` buffer security check enabled |

This is the **launcher half** of the two-part Atrea system:

1. **`AtreaLoader.exe`** (this binary) — starts `SGW.exe`, injects `AtreaRL.dll`.
2. **`AtreaRL.dll`** ("Remote Library") — once inside `SGW.exe`, applies binary patches, hooks Winsock, captures pcap, extracts AES keys, and (when the `Editor` patch group is enabled) flips the UE3 mode flags that turn the game process into the in-game UnrealEd editor.

The injector pipeline below covers the launcher only. The patch-application and editor-activation behavior lives downstream inside `AtreaRL.dll` and is documented in [atrea-editor.md](../reverse-engineering/findings/atrea-editor.md).

## Ghidra anchor caveat

The Ghidra addresses cited in this doc — `FUN_00401250`, `FUN_00401a10`, `FUN_004019d0`, and so on — refer to functions in **AtreaLoader.exe**, not `SGW.exe`. AtreaLoader.exe is imported into the SGW Ghidra project at `/AtreaLoader.exe`, but the program is **not currently loadable in the active Ghidra session** because of a minor language-version skew between the imported file and the installed Ghidra build (`Minor language change 4.1 -> 4.6`). Until that is re-imported, the anchors below are not click-through-verifiable from the live MCP. Each anchor was originally derived from a prior Ghidra session against this binary; the underlying byte sequences in the on-disk `AtreaLoader.exe` have not changed.

When the binary is re-imported and the language version is reconciled, the anchors below should resolve. Until then, treat each as *"address in AtreaLoader.exe per prior decompile session — not click-through-verifiable from the active session"*.

## Command-line modes

### Normal mode

```text
AtreaLoader.exe [sgw_args...]
```

Launches `SGW.exe` with passthrough arguments, then injects `AtreaRL.dll` immediately.

### Slow init mode

```text
AtreaLoader.exe --slowinit [sgw_args...]
```

Same as normal mode but inserts a **700 ms delay** before injection. The `--slowinit` flag is stripped before passing arguments to `SGW.exe`. This exists because `SGW.exe` may need time to fully initialize its import table and memory layout before DLL injection can succeed.

### ASLR fix mode

```text
AtreaLoader.exe --fix-aslr
```

Patches **exactly one byte** of `SGW.exe` on disk to disable ASLR. The full mechanism:

1. Opens `SGW.exe` in read-write binary mode.
2. Reads the DOS header to find the PE header offset (`e_lfanew`).
3. Reads the PE optional header.
4. Inspects the `DllCharacteristics` field. The `IMAGE_DLLCHARACTERISTICS_DYNAMIC_BASE` bit is `0x0040`; its low byte sits at file offset `0x186` in `SGW.exe`.
5. If the `0x40` bit is set, the loader writes back `0x00` at file offset `0x186` — flipping that single byte from `0x40` to `0x00`. Only the `DYNAMIC_BASE` bit is cleared; the other DLL-characteristic flags (`NX_COMPAT`, `NO_SEH`, etc.) are untouched.
6. Reports success / already-disabled / error via a MessageBox.

**Cross-reference for the byte-level claim:** see [docs/drafts/spec/mercury-wire-format.md](../drafts/spec/mercury-wire-format.md) §S9 (`§2.10 Gotchas and surprises`, item S9), which documents the ASLR fix in the broader context of why AtreaLoader's byte-patch table needs the image to load at its preferred `ImageBase=0x00400000`. The mercury-wire-format chapter is the canonical source for this claim; this doc cites it rather than restating the derivation.

> Earlier drafts of this doc claimed the loader "clears the entire `DllCharacteristics` word" via `& 0xFFFF0000`. That was wrong — and the correction matters, because the wider sweep would have stripped `NX_COMPAT` and other flags whose absence would actually break the running image. The fix is precisely one byte at file offset `0x186`.

## Injection pipeline

### Main function: `FUN_00401250` (WinMain)

Anchor: [`ghidra://AtreaLoader.exe@0x00401250`](ghidra://AtreaLoader.exe@0x00401250) *(AtreaLoader.exe not in active Ghidra session — see caveat above)*.

```text
AtreaLoader.exe [args]
    |
    +-- "--fix-aslr" --> Patch SGW.exe PE header byte 0x186, exit
    |
    +-- Normal / --slowinit:
         1. CreateProcessA("SGW.exe [args...]")
         2. If --slowinit: Sleep(700ms)
         3. CloseHandle(hThread)         (let main thread run)
         4. InjectDLL(hProcess)
         5. Check result:
              0  -> success, close handle, exit
             -1  -> LoadLibrary failed       -> TerminateProcess, MessageBox
             -2  -> 10 s timeout             -> TerminateProcess, MessageBox
            else -> status code error        -> TerminateProcess, MessageBox
```

### Injector function: `FUN_00401a10`

Anchor: [`ghidra://AtreaLoader.exe@0x00401a10`](ghidra://AtreaLoader.exe@0x00401a10) *(AtreaLoader.exe not in active Ghidra session)*.

Classic **CreateRemoteThread + LoadLibraryA** injection:

| Step | Operation | Details |
|------|-----------|---------|
| 1 | `VirtualAllocEx` | `0x418` bytes RW in `SGW.exe` (data buffer) |
| 2 | Prepare payload | DLL path + 4 function pointers + 2 output slots |
| 3 | `WriteProcessMemory` | Copy data payload into `SGW.exe` |
| 4 | `VirtualAllocEx` | `0x200` bytes RW in `SGW.exe` (code buffer) |
| 5 | Copy shellcode | `FUN_004019d0` with JMP-thunk unwrapping |
| 6 | `VirtualProtectEx` | Make code buffer `PAGE_EXECUTE` |
| 7 | `CreateRemoteThread` | Execute shellcode with data pointer as parameter |
| 8 | `WaitForSingleObject` | 10-second timeout |
| 9 | `ReadProcessMemory` | Read back results from data buffer |
| 10 | `VirtualFreeEx` | Clean up both allocations |

### Incremental linker detection

Before copying the shellcode, the injector checks whether the function starts with a JMP thunk (common with MSVC incremental linking):

```c
lpShellcode = FUN_004019d0;
if (*lpShellcode == 0xE9 && *(lpShellcode+5) == 0xE9) {
    // Follow the JMP to get the real function body
    lpShellcode = lpShellcode + 5 + *(int*)(lpShellcode + 1);
}
```

This ensures the actual function body — not a debug jump stub — gets copied into the target process.

## Injection data structure

```c
struct InjectionData {          // Total: 0x418 bytes (1048)
    char     dllPath[0x400];    // +0x000: "AtreaRL.dll\0" (1024 bytes, null-padded)
    FARPROC  pfnLoadLibraryA;   // +0x400: &LoadLibraryA
    FARPROC  pfnGetLastError;   // +0x404: &GetLastError
    FARPROC  pfnSleep;          // +0x408: &Sleep (resolved but UNUSED by shellcode)
    FARPROC  pfnExitThread;     // +0x40C: &ExitThread
    DWORD    errorResult;       // +0x410: GetLastError() output (written by shellcode)
    DWORD    loadResult;        // +0x414: LoadLibraryA() result (written by shellcode)
};
```

The function pointers are resolved in the loader process via `GetModuleHandleA("KERNEL32.DLL")` + `GetProcAddress`. Since `KERNEL32.DLL` loads at the same base address in all processes on Windows, these pointers are valid in the target process.

`pfnSleep` at `+0x408` is resolved but never used by the shellcode. This is vestigial — likely from an earlier version that had a retry loop or initialization delay in the shellcode itself. *(speculative — verify against a Ghidra binary-diff if the AtreaLoader source ever surfaces.)*

## Shellcode: `FUN_004019d0`

Anchor: [`ghidra://AtreaLoader.exe@0x004019d0`](ghidra://AtreaLoader.exe@0x004019d0) *(AtreaLoader.exe not in active Ghidra session)*.

51 bytes of position-independent code that executes inside `SGW.exe`'s process:

```asm
0x004019d0: PUSH EBP
0x004019d1: MOV  EBP, ESP
0x004019d3: PUSH ESI
0x004019d4: MOV  ESI, [EBP+8]          ; ESI = lpParameter (InjectionData*)
0x004019d7: PUSH ESI                    ; arg1 = dllPath ("AtreaRL.dll")
0x004019d8: MOV  EAX, [ESI+0x400]      ; EAX = pfnLoadLibraryA
0x004019de: CALL EAX                    ; LoadLibraryA("AtreaRL.dll")
0x004019e0: MOV  [ESI+0x414], EAX      ; Store HMODULE at loadResult
0x004019e6: MOV  EAX, [ESI+0x404]      ; EAX = pfnGetLastError
0x004019ec: CALL EAX                    ; GetLastError()
0x004019ee: MOV  [ESI+0x410], EAX      ; Store error at errorResult
0x004019f4: MOV  EAX, [ESI+0x40C]      ; EAX = pfnExitThread
0x004019fa: PUSH 0                      ; exit code = 0
0x004019fc: CALL EAX                    ; ExitThread(0)
0x004019fe: XOR  EAX, EAX              ; (unreachable - ExitThread never returns)
0x00401a00: POP  ESI
0x00401a01: POP  EBP
0x00401a02: RET  4
```

Clean C equivalent:

```c
DWORD WINAPI RemoteThreadProc(LPVOID lpParameter) {
    struct InjectionData* data = (struct InjectionData*)lpParameter;
    data->loadResult  = data->pfnLoadLibraryA(data->dllPath);  // Load AtreaRL.dll
    data->errorResult = data->pfnGetLastError();                // Capture any error
    data->pfnExitThread(0);                                     // Clean thread exit
    return 0;  // unreachable
}
```

## Error messages

| Condition | MessageBox title | Message |
|-----------|------------------|---------|
| ASLR cleared | "AtreaLoader" | "ASLR successfully disabled in SGW.exe." |
| ASLR already clear | "AtreaLoader" | "ASLR is already disabled in SGW.exe; no action taken." |
| Can't open `SGW.exe` | "AtreaLoader Error" | "Failed to open file SGW.exe!" |
| Launch failure | "Error" | "Failed to launch SGW.exe: %s" (`FormatMessageA`) |
| Injection error (-1) | "AtreaLoader Error" | "The remote library loader (AtreaRL.dll) reported an internal error." |
| Injection timeout (-2) | "AtreaLoader Error" | "Timed out waiting for the remote library loader (AtreaRL.dll) to load." |
| Other injection error | "AtreaLoader Error" | "The remote library loader (AtreaRL.dll) reported an internal error [status code %d]." |

## Config files: binary `.config` vs editable `.xml`

The loader system has **two** config artifacts that share a name root but differ in role and format:

| Artifact | Role | Format | Edited by humans? |
|---|---|---|---|
| `AtreaLoader.config` | The file the loader actually reads at runtime. | **Compiled binary** (proprietary serialization). | No — opaque. |
| `AtreaLoader.config.xml` | The editable source the binary `.config` was derived from. | XML. | Yes — declarative patch table + NVP settings. |

Editing the `.xml` alone does **not** change runtime behavior; the runtime path consumes the binary `.config`. This is why on at least one observed install, an `.xml` declaration of `<NVP Name="Sniffer" Value="true" />` was present yet the sniffer never activated — the binary `.config` in that install either lacked the NVP entry or carried a non-`"true"` value, and the NVP-gate inside `AtreaRL.dll` (`FUN_10026F30`, decompiled in [mercury-wire-format.md](../drafts/spec/mercury-wire-format.md) §S9 item 2) kept the sniffer init function unreachable. The binary `.config` predates the XML by over a year on at least one observed installation (binary `2013-05-20` vs XML `2014-06-23`).

The mercury-wire-format chapter is the canonical source for this two-file distinction. *(The binary-format details — how to round-trip between `.config` and `.xml` — are not documented; the proprietary serializer has not been reverse-engineered. Treat the `.xml` as read-only documentation rather than authoritative source for now.)*

## What AtreaLoader does NOT do

The loader binary itself is narrowly scoped. The following capabilities are **not** in `AtreaLoader.exe`:

- No networking (no Winsock imports).
- No encryption or decryption.
- No packet capture.
- No configuration file parsing.
- No logging to disk.
- No registry access.
- No anti-debug or obfuscation.
- No persistence mechanism.

All of those capabilities — pcap capture, AES key extraction, binary patch application, hook installation, editor-mode activation — live in `AtreaRL.dll`, which takes over once injected. The AtreaRL DLL is also where the in-game UnrealEd editor activation happens (see [atrea-editor.md](../reverse-engineering/findings/atrea-editor.md)). That broader system is documented elsewhere — this doc intentionally stops at the launcher boundary.

## Emulator implications

1. **Launch sequence**: Users run `AtreaLoader.exe`, **not** `SGW.exe` directly. The loader is the entry point for the entire emulator client experience.
2. **Command-line passthrough**: All `SGW.exe` arguments (graphics, resolution, etc.) pass through the loader. The loader adds the `--enable-group=`, `--disable-group=`, and `--nvp-` arguments that `AtreaRL.dll` parses from within `SGW.exe`'s `GetCommandLineA()`.
3. **ASLR dependency**: `SGW.exe` must have ASLR disabled for the emulator's binary patches (which use absolute virtual addresses) to resolve. The `--fix-aslr` mode is a one-time setup step that flips exactly one byte at file offset `0x186`.
4. **Timing sensitivity**: The `--slowinit` flag exists because injection can race with `SGW.exe`'s startup. If injection fails intermittently, this flag is the fix.

## Companion docs

| Doc | Relationship |
|---|---|
| [docs/client-tools.md](../client-tools.md) | User-facing how-to: which `.bat` to run, what each one does. Start here if you are a player or operator, not an RE. |
| [docs/reverse-engineering/findings/atrea-editor.md](../reverse-engineering/findings/atrea-editor.md) | The downstream in-game UnrealEd editor that activates once the patches are applied. Top-level entry point for the editor surface. |
| [docs/architecture/atrea-editor-bridge.md](../architecture/atrea-editor-bridge.md) | ADR for the MCP-server architecture proposal that drives the editor programmatically. |
| [docs/drafts/spec/mercury-wire-format.md](../drafts/spec/mercury-wire-format.md) §S9 | Canonical source for the single-byte `--fix-aslr` patch and the binary-`.config`-vs-XML distinction. |
| [docs/client/launcher-guide.md](../client/launcher-guide.md) | The CME `Launcher.exe`-driven player flow; cross-links here for "what Atrea actually does at runtime". |
| [docs/client/sgw-launcher.md](../client/sgw-launcher.md) | The reimplementation launcher's view of the Atera/Atrea batch files. |
