---
name: reference_physxloader_local_core
description: The client's bundled PhysXLoader.dll loads the bundled PhysXCore.dll only if HKLM enableLocalPhysXCore equals the last network adapter's MAC; otherwise it needs the system 2.x engine
metadata:
  type: reference
---

Verified 2026-10-03 by decompiling `Working\binaries\PhysXLoader.dll` (headless Ghidra, image base 0x10000000), while validating #1150 (macOS/Wine) and #1121 (Windows prerequisites).

- `FUN_100010e0` / `FUN_100011b0` load PhysXCore.dll / NxCooking.dll. They first call `FUN_100015e0`. If it returns true they `LoadLibraryA("PhysXCore.dll")` / `("NxCooking.dll")` by bare name, so the copies bundled next to SGW.exe are used.
- `FUN_100015e0` reads REG_BINARY `HKLM\Software\AGEIA Technologies\enableLocalPhysXCore` (6-byte buffer, so a longer value fails) and compares it with 6 bytes from `FUN_10001530`.
- `FUN_10001530` calls `GetAdaptersInfo` and keeps the **last** adapter's MAC address. If the call fails, it uses the ASCII bytes `"AGEIA\0"`.
- Otherwise the loader reads REG_SZ `HKLM\Software\Ageia Technologies\PhysXCore Path`, appends `\vMAJ.MIN.PATCH\` (from the SDK version) and loads the core from there. That is the system software path and the reason for #1121's `NxCreatePhysicsSDK` failure.
- The loader is 32-bit, so on 64-bit Windows the key lives under `WOW6432Node`. Writing it needs elevation on Windows, but not inside a Wine prefix.

Result: one registry value lets the client use its own bundled PhysX, with no PhysX System Software installer. This works on Windows and under Wine, but the value is tied to the MAC address, so the launcher must recompute it on every launch.
