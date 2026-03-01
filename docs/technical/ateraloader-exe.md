# AteraLoader.exe — DLL Injector

Analysis of the AteraLoader executable — the launcher component that starts SGW.exe and injects the AtreaRL.dll "Remote Library" into the game process.

## Overview

| Property | Value |
|----------|-------|
| **Type** | Win32 EXE (32-bit) |
| **Base Address** | 0x00400000 |
| **Code Size** | ~55 KB (.text: 0x00401000–0x0040e7ff) |
| **Compiler** | MSVC (static CRT) |
| **DLL Imports** | KERNEL32.DLL (49 functions), USER32.DLL (1 function) |
| **Export** | `entry` at 0x00403041 |
| **Manifest** | `requestedExecutionLevel level='asInvoker'` (no admin required) |
| **Security** | `/GS` buffer security check enabled |

This is the **launcher half** of the two-part Atrea system:
1. **AteraLoader.exe** (this binary) — starts SGW.exe, injects AtreaRL.dll
2. **AtreaRL.dll** ("Remote Library") — once inside SGW.exe, hooks Winsock, captures PCAP, extracts AES keys, applies binary patches

The "RL" in AtreaRL stands for "Remote Library" — confirmed by error messages: *"the remote library loader (AtreaRL.dll) reported an internal error"*.

## Command-Line Modes

### Normal Mode
```
AteraLoader.exe [sgw_args...]
```
Launches SGW.exe with passthrough arguments, then injects AtreaRL.dll immediately.

### Slow Init Mode
```
AteraLoader.exe --slowinit [sgw_args...]
```
Same as normal mode but inserts a **700ms delay** before injection. The `--slowinit` flag is stripped before passing args to SGW.exe. This exists because SGW.exe may need time to fully initialize its import table and memory layout before DLL injection can succeed.

### ASLR Fix Mode
```
AteraLoader.exe --fix-aslr
```
Patches `SGW.exe` on disk to disable ASLR:
1. Opens SGW.exe in read-write binary mode
2. Reads DOS header to find PE header offset (`e_lfanew`)
3. Reads PE optional header
4. Checks `DllCharacteristics` for bit `0x40` (`IMAGE_DLLCHARACTERISTICS_DYNAMIC_BASE`)
5. If set: clears the entire `DllCharacteristics` word and writes back
6. Reports success/already-disabled/error via MessageBox

**Note**: The clearing is aggressive — `DllCharacteristics & 0xFFFF0000` zeros ALL characteristic flags (NX compat, no SEH, etc.), not just the ASLR bit. For a game executable this is harmless.

## Injection Pipeline

### Main Function: `FUN_00401250` (WinMain)

```
AteraLoader.exe [args]
    │
    ├─ "--fix-aslr" → Patch SGW.exe PE header, exit
    │
    └─ Normal / --slowinit:
         1. CreateProcessA("SGW.exe [args...]")
         2. If --slowinit: Sleep(700ms)
         3. CloseHandle(hThread)  — let main thread run
         4. InjectDLL(hProcess)
         5. Check result:
              0  → success, close handle, exit
             -1  → LoadLibrary failed → TerminateProcess, MessageBox
             -2  → 10s timeout → TerminateProcess, MessageBox
            else → status code error → TerminateProcess, MessageBox
```

### Injector Function: `FUN_00401a10`

Classic **CreateRemoteThread + LoadLibraryA** injection:

| Step | Operation | Details |
|------|-----------|---------|
| 1 | `VirtualAllocEx` | 0x418 bytes RW in SGW.exe (data buffer) |
| 2 | Prepare payload | DLL path + 4 function pointers + 2 output slots |
| 3 | `WriteProcessMemory` | Copy data payload into SGW.exe |
| 4 | `VirtualAllocEx` | 0x200 bytes RW in SGW.exe (code buffer) |
| 5 | Copy shellcode | `FUN_004019d0` with JMP-thunk unwrapping |
| 6 | `VirtualProtectEx` | Make code buffer `PAGE_EXECUTE` |
| 7 | `CreateRemoteThread` | Execute shellcode with data pointer as parameter |
| 8 | `WaitForSingleObject` | 10-second timeout |
| 9 | `ReadProcessMemory` | Read back results from data buffer |
| 10 | `VirtualFreeEx` | Clean up both allocations |

### Incremental Linker Detection

Before copying the shellcode, the injector checks if the function starts with a JMP thunk (common with MSVC incremental linking):

```c
lpShellcode = FUN_004019d0;
if (*lpShellcode == 0xE9 && *(lpShellcode+5) == 0xE9) {
    // Follow the JMP to get the real function body
    lpShellcode = lpShellcode + 5 + *(int*)(lpShellcode + 1);
}
```

This ensures the actual function body (not a debug jump stub) gets copied into the target process.

## Injection Data Structure

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

The function pointers are resolved in the loader process via `GetModuleHandleA("KERNEL32.DLL")` + `GetProcAddress`. Since KERNEL32.DLL loads at the same base address in all processes on Windows, these pointers are valid in the target process.

**Note**: `pfnSleep` at +0x408 is resolved but never used by the shellcode. This is vestigial — likely from an earlier version that had a retry loop or initialization delay in the shellcode itself.

## Shellcode: `FUN_004019d0`

51 bytes of position-independent code that executes inside SGW.exe's process:

```asm
004019d0: PUSH EBP
004019d1: MOV  EBP, ESP
004019d3: PUSH ESI
004019d4: MOV  ESI, [EBP+8]          ; ESI = lpParameter (InjectionData*)
004019d7: PUSH ESI                    ; arg1 = dllPath ("AtreaRL.dll")
004019d8: MOV  EAX, [ESI+0x400]      ; EAX = pfnLoadLibraryA
004019de: CALL EAX                    ; LoadLibraryA("AtreaRL.dll")
004019e0: MOV  [ESI+0x414], EAX      ; Store HMODULE at loadResult
004019e6: MOV  EAX, [ESI+0x404]      ; EAX = pfnGetLastError
004019ec: CALL EAX                    ; GetLastError()
004019ee: MOV  [ESI+0x410], EAX      ; Store error at errorResult
004019f4: MOV  EAX, [ESI+0x40C]      ; EAX = pfnExitThread
004019fa: PUSH 0                      ; exit code = 0
004019fc: CALL EAX                    ; ExitThread(0)
004019fe: XOR  EAX, EAX              ; (unreachable — ExitThread never returns)
00401a00: POP  ESI
00401a01: POP  EBP
00401a02: RET  4
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

## Error Messages

| Condition | MessageBox Title | Message |
|-----------|-----------------|---------|
| ASLR cleared | "AtreaLoader" | "ASLR successfully disabled in SGW.exe." |
| ASLR already clear | "AtreaLoader" | "ASLR is already disabled in SGW.exe; no action taken." |
| Can't open SGW.exe | "AtreaLoader Error" | "Failed to open file SGW.exe!" |
| Launch failure | "Error" | "Failed to launch SGW.exe: %s" (FormatMessageA) |
| Injection error (-1) | "AtreaLoader Error" | "The remote library loader (AtreaRL.dll) reported an internal error." |
| Injection timeout (-2) | "AtreaLoader Error" | "Timed out waiting for the remote library loader (AtreaRL.dll) to load." |
| Other injection error | "AtreaLoader Error" | "The remote library loader (AtreaRL.dll) reported an internal error [status code %d]." |

## What AteraLoader Does NOT Do

- No networking (no Winsock imports)
- No encryption or decryption
- No packet capture
- No configuration file parsing
- No logging to disk
- No registry access
- No anti-debug or obfuscation
- No persistence mechanism

All of these capabilities live in AtreaRL.dll, which takes over once injected.

## Emulator Implications

1. **Launch sequence**: Users run AteraLoader.exe, NOT SGW.exe directly. The loader is the entry point for the entire emulator client experience.
2. **Command-line passthrough**: All SGW.exe arguments (graphics, resolution, etc.) pass through the loader. The loader adds the `--enable-group=`, `--disable-group=`, and `--nvp-` arguments that AtreaRL.dll parses from within SGW.exe's `GetCommandLineA()`.
3. **ASLR dependency**: SGW.exe must have ASLR disabled for the emulator's binary patches (hardcoded addresses) to work. The `--fix-aslr` mode is a one-time setup step.
4. **Timing sensitivity**: The `--slowinit` flag exists because injection can race with SGW.exe's startup. If injection fails intermittently, this flag is the fix.
