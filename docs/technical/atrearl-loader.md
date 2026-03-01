# AtreaRL Loader DLL

Analysis of the Atrea Remote Loader — the client-side emulator loader DLL that patches the SGW game client at runtime.

## Overview

| Property | Value |
|----------|-------|
| **Full Name** | Atrea Remote Loader rev. 36 |
| **Build Date** | Feb 21, 2014 19:38:46 |
| **Type** | Win32 DLL (32-bit) |
| **Base Address** | 0x10000000 |
| **Text Section** | ~196 KB |
| **Single Export** | `entry` at 0x1000ec72 |

The loader injects into the SGW client process and:
1. Patches binary code via XML configuration
2. Hooks Winsock functions to capture network traffic
3. Extracts AES session keys from the auth stream
4. Hooks Unreal Engine symbols for logging/debugging
5. Writes captured traffic to PCAP files

## Initialization Sequence (DllMain / entry)

Decompiled from `FUN_1002aa80`:

```
1. Log banner: "Atrea Remote Loader rev. 36 (built: Feb 21 2014 19:38:46)"
2. Parse command line (--enable-group=, --disable-group=, --nvp-)
3. Load and apply AtreaLoader.config (XML binary patches)
4. Hook CreateFileA and CreateFileW
5. Install 10 Unreal Engine symbol hooks:
   - UnicodeLogger
   - AppearanceLogger
   - AnsiLogger
   - MercuryLogger
   - UnrealAssertionLogger
   - FFileManager::MoveFile
   - UObject::Serialize
   - FArchive::PostLoad
   (2 more unnamed)
6. Conditionally initialize network sniffer
7. Optionally disable Windows Error Reporting
8. Show warning MessageBox if initialization issues occurred
```

## Binary Patching System

Source: `patch.cpp` and `appmem.cpp`

### Configuration
- Reads `AtreaLoader.config` (XML format)
- Patches are organized in groups (enable/disable via command line)

### Patch Format
Each patch entry contains:
- **Chunk**: Target address/region
- **OriginalBytes**: Expected bytes at target (verification)
- **ReplacementBytes**: New bytes to write

The loader verifies OriginalBytes before patching, preventing patches from applying to wrong binary versions.

### Memory Operations
- `appmem.cpp` handles PE memory manipulation
- Changes page protection (VirtualProtect) for patching
- Supports arbitrary code/data replacement

## Network Sniffer

Source: `sniff.cpp`

### Initialization (FUN_1002ce00)
When enabled, the sniffer:
1. Opens key file: `sessions\YYYY-MM-DD_HH-MM-keys.txt`
2. Hooks 4 Winsock functions via IAT trampolines (FUN_1002b950):

| Original Function | Hook Function | Storage Address | Purpose |
|-------------------|---------------|-----------------|---------|
| `send` | FUN_1002c830 | DAT_10042cd8 | Capture outbound TCP |
| `sendto` | FUN_1002c750 | DAT_10042ce0 | Capture outbound UDP |
| `WSARecv` | FUN_1002cc00 | DAT_10042ce4 | Capture inbound TCP |
| `WSARecvFrom` | FUN_1002c870 | DAT_10042cdc | Capture inbound UDP |

### PCAP Writer (FUN_1002c0c0)
Creates standard pcap capture files:
- Output: `sessions\YYYY-MM-DD_HH-MM.pcap`
- Magic: `0xa1b2c3d4` (standard pcap)
- Version: 2.4
- Link type: 1 (Ethernet)
- Snap length: 0xFFFF (65535 bytes)

### Packet Capture (FUN_1002beb0)
For each captured packet, constructs synthetic headers:
- **Ethernet**: Source MAC `00:00:12:34:56:78`, Dest MAC `00:00:9a:bc:de:f0`, EtherType 0x0800 (IPv4)
- **IP**: Standard IPv4 header with real source/dest addresses
- **UDP**: Standard UDP header with real ports
- Writes pcap record header (timestamp, captured length, original length) followed by packet data

These fake Layer 2/3 headers allow the PCAP to be opened in Wireshark for analysis.

### Mercury Link Registration (FUN_1002c270)
When a Mercury connection is established:
1. Allocates 0xB8-byte connection tracking object
2. Logs: `"Sniffer: Registering Mercury link: %08x:%d -> %08x:%d"`
3. Uses `127.0.0.1` as fallback for NULL addresses
4. Tracks per-connection state for proper PCAP stream separation

### AES Key Extraction (FUN_1002c3f0)
Scans inbound data for session key exchange:
1. Searches for `<ServerLocation SessionKey="` in received data
2. Extracts 64-byte hexadecimal key at offset 0x1C from the match
3. Writes key to `sessions\YYYY-MM-DD_HH-MM-keys.txt`

This captures the AES session key that encrypts all subsequent Mercury traffic, enabling offline PCAP decryption.

## Command Line Parser (FUN_1002b340)

Parses `GetCommandLineA()` for:
| Argument | Purpose |
|----------|---------|
| `--enable-group=<name>` | Enable a patch group |
| `--disable-group=<name>` | Disable a patch group |
| `--nvp-<key>=<value>` | Set name-value pair (config override) |

## Unreal Engine Hooks

10 symbol hooks installed during initialization. These intercept engine functions for logging and debugging:

### Logging Hooks
| Hook Name | Purpose |
|-----------|---------|
| `UnicodeLogger` | Captures Unicode log output |
| `AppearanceLogger` | Logs character appearance data |
| `AnsiLogger` | Captures ANSI log output |
| `MercuryLogger` | Logs Mercury protocol events |
| `UnrealAssertionLogger` | Captures UE3 assertion failures |

### Engine Function Hooks
| Hook Name | Purpose |
|-----------|---------|
| `FFileManager::MoveFile` | Intercepts file move operations |
| `UObject::Serialize` | Intercepts object serialization |
| `FArchive::PostLoad` | Intercepts post-load processing |

### File System Hooks
| Function | Purpose |
|----------|---------|
| `CreateFileA` | Intercepts ANSI file open |
| `CreateFileW` | Intercepts Unicode file open |

These likely redirect file paths to emulator-specific resources or log file access patterns.

## Hook Installation (FUN_1002b950)

The trampoline installer:
1. Locates target function in the IAT (Import Address Table)
2. Saves original function pointer
3. Replaces IAT entry with hook function
4. Hook functions call through to original after processing

## Key Source Files Referenced

| File | Purpose |
|------|---------|
| `appmem.cpp` | PE memory manipulation, page protection |
| `patch.cpp` | XML patch loading, byte verification/replacement |
| `sniff.cpp` | Network sniffer, PCAP writer, key extraction |

## Emulator Implications

The AtreaRL loader reveals the emulator architecture:
1. **Login redirect**: Binary patches likely redirect the SOAP login endpoint from `stargateworlds.com` to the emulator's auth server
2. **Key capture**: AES session keys are captured for traffic analysis/debugging
3. **PCAP logging**: Full traffic capture for protocol analysis
4. **Mercury hooks**: Protocol-level logging for debugging emulator responses
5. **Modular patching**: XML-based patches can be updated without rebuilding the loader
6. **Group system**: Patches can be selectively enabled/disabled for testing

## File Output

| Path | Format | Content |
|------|--------|---------|
| `sessions\YYYY-MM-DD_HH-MM.pcap` | pcap (Wireshark) | Full network capture |
| `sessions\YYYY-MM-DD_HH-MM-keys.txt` | Text | AES session keys |
| `AtreaLoader.config` | XML (input) | Binary patch definitions |
