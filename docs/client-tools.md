# Client Tools

The Stargate Worlds client installation comes with several tools beyond just the game itself. Here's what each one does and how to use it.

## The Three Executables

### Launcher.exe — Patch Client

The official CME launcher. Despite the name, it **does not launch the game**. It's purely a patch/update client that:

1. Checks a SOAP-based patch server for updates
2. Downloads and installs patches to the game files
3. Handles crash dump collection and upload

Since the CME servers are long dead, the launcher has no server to talk to. **For emulator use, skip the launcher entirely** and use AteraLoader instead.

### AteraLoader.exe — Game Launcher + DLL Injector

This is what actually starts the game. It:

1. Launches `SGW.exe` with any command-line arguments you pass
2. Injects `AtreaRL.dll` into the running game process
3. Optionally fixes ASLR (one-time setup)

**Usage:**
```
AteraLoader.exe                     Normal launch
AteraLoader.exe --slowinit          Launch with 700ms delay before injection (if normal fails)
AteraLoader.exe --fix-aslr          One-time: disable ASLR in SGW.exe (required for patches to work)
```

The `--fix-aslr` step only needs to be done once. It modifies the SGW.exe file so that it always loads at the same memory address, which is required for the binary patches to work correctly.

### AtreaRL.dll — The "Remote Library"

Once injected into the game process, this DLL does the heavy lifting:

- **Binary patching** — Applies patches defined in `AtreaLoader.config` (login URL redirect, editor mode, debug logging, etc.)
- **Network capture** — Hooks the game's network functions to capture all traffic to PCAP files (viewable in Wireshark)
- **AES key extraction** — Captures the encryption keys used for Mercury protocol traffic
- **Logging** — Hooks into Unreal Engine and BigWorld logging systems

All captured data goes to the `sessions/` directory:
- `sessions/YYYY-MM-DD_HH-MM.pcap` — Full network capture
- `sessions/YYYY-MM-DD_HH-MM-keys.txt` — AES encryption keys

## Editor Mode

One of the most interesting features: the binary patches can **unlock UnrealEd** inside SGW.exe. The game client contains a full copy of the Unreal Editor (the same wxWidgets-based editor used by UE3 developers), and it can be activated by running:

```
AtreaEditor.bat
```

Or manually:
```
AteraLoader.exe --enable-group=Editor -SHOWLOG
```

This sets the UE3 global flags `GIsEditor=1` and `GIsServer=1` while clearing `GIsGame=0`, which switches the engine from game mode to editor mode. You can then browse maps, view assets, and inspect the game's content.

**Warning: ASEditor can corrupt your client cache.** If you use ASEditor to edit or save a `.umap` file, it can invalidate the client's shader cache and cooked data stored in `Documents/My Games/Firesky/`. This will cause the game client to crash on subsequent launches. The fix is to delete the entire `Documents/My Games/Firesky/` folder — the client will regenerate it on next launch.

### UCC Commandlet Mode

Unreal's command-line tool mode can also be enabled for content cooking, package inspection, and other utilities. This is configured in `AtreaLoader.config` but is disabled by default.

## Configuration File

`AtreaLoader.config` is an XML file that defines all binary patches, organized into groups:

| Group | What It Does | Default |
|-------|-------------|---------|
| Mercury | Enables BigWorld Mercury protocol logging | Enabled via batch file |
| Debug | Enables localization debug logging | Enabled |
| AppearanceLogging | Logs character appearance data | Disabled |
| Editor | Enables UnrealEd mode | Enabled via batch file |
| UCC | Enables UCC commandlet mode | Disabled |
| Splash | Editor splash screen | Disabled |
| Silent | Hides editor UI panes (headless mode) | Disabled |

### Settings (NVPs)

| Setting | Default | Purpose |
|---------|---------|---------|
| Sniffer | true | Capture network traffic to PCAP files |
| ExitOnAssert | false | Crash on assertion failure instead of showing a dialog |
| DisableErrorReporting | false | Suppress Windows Error Reporting on crashes |

### Path Substitution

The config supports redirecting file paths at runtime. For example, you could redirect the UI content folder to a development directory to test UI changes without modifying the game installation. This uses the `CreateFileA`/`CreateFileW` hooks that AtreaRL installs.

## Debug Logging

When launched with Mercury logging enabled (`--enable-group=Mercury`), the game outputs BigWorld protocol debug information. Combined with the `-SHOWLOG` flag, you get a real-time view of what the networking layer is doing.

The logging hooks capture output from:
- BigWorld entity events (Unicode/ANSI loggers)
- Mercury protocol messages
- UE3 assertion failures
- Character appearance changes (if enabled)

## Batch Files

| File | What It Does |
|------|-------------|
| `AtreaEditor.bat` | Launch with UnrealEd enabled + log output |
| `AtreaGameDebug.bat` | Launch game with Mercury protocol logging |
| `AtreaFixASLR.bat` | One-time ASLR fix for SGW.exe |

## For Emulator Use

The typical workflow for connecting to the Cimmeria server:

1. Run `AteraLoader.exe --fix-aslr` once (first time only)
2. Make sure Login.lua is patched to point to your server (the `setup.ps1` script can do this)
3. Run `AteraLoader.exe` (or use a batch file for debug logging)
4. The game launches and connects to the emulator server instead of the dead CME servers

## Troubleshooting

### Client Crashes on Launch

If the client crashes during startup or the loading screen, the most likely cause is a **corrupted shader cache**. This happens if you have previously used ASEditor to edit a `.umap` file, which invalidates cached data.

**Fix:** Delete the folder `Documents/My Games/Firesky/` and relaunch. The client will regenerate the cache.

