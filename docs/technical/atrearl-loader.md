# AtreaRL.dll — The Runtime Patcher

> [!NOTE]
> **Scope banner — companion to `atrea-editor.md`, not superseded by it.**
> The top-level entry point for the whole Atrea toolchain — including the in-game
> UnrealEd editor that this DLL's patches unlock inside `SGW.exe` — is
> [`docs/reverse-engineering/findings/atrea-editor.md`](../reverse-engineering/findings/atrea-editor.md).
> Read that first for orientation, then return here for the DLL's runtime behaviour.
>
> `atrea-editor.md` describes this page as one of an "apocryphal trio" that it supersedes.
> **That framing did not survive the 2026-07-25 audit.** This page was *revised* in the
> same 2026-05-25 campaign — `atrea-editor.md`'s own §"Apocryphal docs to retire" table
> records the corrections applied here (symbol-hook count `10` → `13`, the sniffer-init
> function swap, the removal of the false "login redirect" claim) and its §Cross-references
> cites this page as a live "AtreaRL.dll runtime hooks" reference. It is the only place
> documenting the Winsock IAT hooks, the synthetic-L2 pcap writer, and the
> `<ServerLocation SessionKey="` AES-key scrape. Treat the two as complementary halves.

> **Last updated**: 2026-07-25 (accuracy audit — scope banner added; no address changes)
> **Previously revised**: 2026-05-25
> **Audience**: reverse engineers and emulator developers working on the SGW client patch surface
> **Doc type**: reference (DLL responsibilities, runtime gating, verified anchors)
> **Status**: revised — internal hook-function addresses from the previous revision were unverified and have been marked speculative or removed pending a fresh Ghidra pass on `AtreaRL.dll`

This document covers `AtreaRL.dll`, the community-built DLL that AtreaLoader.exe injects into SGW.exe. For the editor-mode behavior it enables, see [docs/reverse-engineering/findings/atrea-editor.md](../reverse-engineering/findings/atrea-editor.md). For the injector executable, see [docs/technical/atrealoader-exe.md](atrealoader-exe.md). For the patch/symbol/NVP tables it consumes, see [docs/technical/atrealoader-config.md](atrealoader-config.md).

## What it does

`AtreaRL.dll` is the "Remote Library" half of the two-part Atrea system. AtreaLoader.exe launches SGW.exe suspended, performs a classic `CreateRemoteThread` + `LoadLibraryA` injection of this DLL, then resumes the main thread. Once mapped into the SGW.exe process, `AtreaRL.dll` runs five responsibilities from its entry point:

1. Parse its own command-line forwarded via `CreateProcess` (`--enable-group=`, `--disable-group=`, `--nvp-`).
2. Read `AtreaLoader.config` (the runtime binary form — see below) and apply the patch table to the running SGW.exe image.
3. Install hooks at the 13 `<Symbol>` entries declared in the config (those with `Patch="true"`).
4. Hook `CreateFileA` and `CreateFileW` for path-substitution and access logging.
5. Conditionally start the network sniffer if the runtime config's `Sniffer` NVP is `"true"`.

The DLL itself does **not** handle login-server redirection — that lives in client-side `Login.lua` and is applied by `setup.ps1` during emulator setup. See [docs/client-tools.md](../client-tools.md) for the redirect mechanism.

## Overview

| Property | Value |
|---|---|
| Full name | Atrea Remote Loader rev. 36 |
| Build date | Feb 21, 2014 19:38:46 |
| Type | Win32 DLL (32-bit) |
| Preferred base address | `0x10000000` |
| Single export | `entry` at `0x1000ec72` (speculative — verify against AtreaRL.dll) |
| Source code | not public; reverse-engineered from binary |

## Prerequisite — modern Windows needs `AtreaFixASLR.bat`

Every byte-patch in `AtreaLoader.config.xml` uses **absolute virtual addresses** that assume SGW.exe loads at its PE-declared preferred base `0x00400000`. On modern Windows the image instead loads at an ASLR-randomized base because SGW.exe's PE header has `IMAGE_DLLCHARACTERISTICS_DYNAMIC_BASE` (the `0x40` bit at file offset `0x186`). Under ASLR, every `OriginalBytes` pattern misses; the session log reports `0 patch(es) of N applied` and **none of AtreaRL's behavior activates** — no logger hooks, no editor mode, no sniffer.

Run `AtreaFixASLR.bat` once before first use of any AtreaLoader feature. It calls `AtreaLoader.exe --fix-aslr`, which flips the single byte at file offset `0x186` of SGW.exe from `0x40` to `0x00`. After this one-time fix the image loads at `0x00400000` and the patch table resolves correctly. Full analysis in [mercury-wire-format.md §S9](../drafts/spec/mercury-wire-format.md).

## Initialization sequence (entry / DllMain)

Driven from the DLL's entry point (`FUN_1002aa80` in the previous revision — speculative — verify against AtreaRL.dll):

1. Log banner: `"Atrea Remote Loader rev. 36 (built: Feb 21 2014 19:38:46)"`.
2. Parse the command line passed by `AtreaLoader.exe` (`GetCommandLineA()`).
3. Load `AtreaLoader.config` (binary form — see next section) and apply enabled patch groups.
4. Hook `CreateFileA` and `CreateFileW` for path-substitution.
5. Install hooks at all `<Symbol>` entries marked `Patch="true"` in the config (13 entries total — see "Symbol hooks" below).
6. Run the NVP gate for the sniffer and call its init function if the gate passes.
7. Optionally suppress Windows Error Reporting based on the `DisableErrorReporting` NVP.
8. Show a warning `MessageBox` if initialization reported failures.

## Runtime config — binary `.config`, not the XML

The on-disk `AtreaLoader.config.xml` is the **editable source**; AtreaRL.dll reads a separate **binary `AtreaLoader.config`** at runtime. On at least one observed installation the binary form predated the XML by over a year (binary 2013-05-20 vs. XML 2014-06-23) and the two had drifted — most notably on the `Sniffer` NVP value, where editing the XML alone did not enable the sniffer. When making config changes, regenerate the binary or your edits will appear to do nothing.

The XML is the human-readable transcription documented in [atrealoader-config.md](atrealoader-config.md); the binary is what's actually consulted at the runtime gates documented in this file.

## Symbol hooks — 13 entries, not 10

Earlier revisions of this doc claimed "10 Unreal Engine symbol hooks". The XML actually defines **13 `<Symbol>` entries**, of which the subset with `Patch="true"` are wrapped unconditionally and the rest activate only when their owning group is enabled. The three previously conflated `"UnicodeLogger"` entries are in fact three distinct symbols (`UnicodeLoggerStart`, `UnicodeLoggerParam`, `UnicodeLoggerEnd`) covering the entry, parameter-emit, and end phases of a single log call.

The 13 symbols in `binaries/AtreaLoader.config.xml` (lines 183–207):

| Symbol | SGW.exe address | Group | `Patch` |
|---|---|---|---|
| `UnicodeLoggerStart` | `0x00866860` | (default) | true |
| `UnicodeLoggerParam` | `0x00866880` | (default) | true |
| `UnicodeLoggerEnd` | `0x00866870` | (default) | true |
| `AppearanceLoggerWchar` | `0x000250d0` | AppearanceLogging | false |
| `AppearanceLoggerWstring` | `0x00304750` | AppearanceLogging | false |
| `AnsiLogger` | `0x00635210` | (default) | true |
| `MercuryLogger` | `0x0041c2e0` | Mercury | false |
| `UnrealAssertionLogger` | `0x00086000` | (default) | true |
| `FFileManager::MoveFile` | `0x000c43a0` | Editor | false |
| `UPrefab::Serialize` | `0x00812d30` | EditorDebugPrefab | false |
| `FArchive::PostLoad` | `0x000e9870` | EditorPartialSerializePrefabs | false |
| `UObject::Serialize` | `0x000a42f0` | EditorDebugPrefab | false |
| `FName::GNames` | `0x01acade0` | (default — data, not a hook) | — |

Plus the unconditional Winsock-adjacent hooks on `CreateFileA` and `CreateFileW`, that totals **15 runtime hooks** when all opt-in groups are enabled. (`FName::GNames` is a data-symbol address rather than a function to hook — included in the count only for completeness.)

The companion `MercuryLogger` symbol at `0x0041c2e0` is gated by the `EnableUnicodeLogger` byte-patch at `0x01af2224`[^mercury-logger-anchor] — the patch flips a 4-byte enable flag from `00 00 00 00` to `01 00 00 00`, and only then does the wrapped logger actually emit. See [mercury-wire-format.md §2.9](../drafts/spec/mercury-wire-format.md) for the MercuryLogger anchor and how the patch toggles it.

## Network sniffer

The sniffer is the "second product" of AtreaRL beyond the editor and logger hooks — it produces a Wireshark-readable `.pcap` of the entire Mercury session plus a text dump of the AES session key extracted from the auth stream. Both outputs land in `binaries/sessions/` next to the loader.

### Two-gate activation

On modern Windows the sniffer requires **both** of these to pass:

1. The ASLR fix above (otherwise no byte-patches apply and no hooks install).
2. The binary `.config`'s `Sniffer` NVP must be exactly the literal string `"true"` (4 chars). The runtime gate is in `FUN_10026F30`, which calls an NVP lookup (`FUN_10022C10`), compares the returned value-length and bytes against `"true"`, and only then calls the sniffer init function `FUN_10021FB0`. If the gate is closed, the sniffer code path is never entered even though every other patch and hook is in place. Full decompile and the two-byte `JZ +5 → NOP NOP` workaround at file offset `0x269d7` / runtime VA `0x100275d7` are documented in [mercury-wire-format.md §S9](../drafts/spec/mercury-wire-format.md).

### Sniffer responsibilities (when the gate opens)

When `FUN_10021FB0` runs, the sniffer:

1. Opens an output session-key file at `binaries/sessions/YYYY-MM-DD_HH-MM-keys.txt`.
2. Hooks four Winsock functions via IAT trampolines: `send`, `sendto`, `WSARecv`, `WSARecvFrom`. The per-hook trampoline addresses, the IAT-installer function, and the per-function storage slots from the previous revision are *not* verified against the current binary (Ghidra could not open `AtreaRL.dll` in this pass due to a language-version mismatch — `Minor language change 4.1 -> 4.6`) — see "Hook table layout (pending verification)" below.
3. For each captured packet, constructs a synthetic Ethernet/IPv4/UDP header (constant source MAC `00:00:12:34:56:78`, dest MAC `00:00:9a:bc:de:f0`, EtherType `0x0800`) so the PCAP opens cleanly in Wireshark.
4. Writes a standard pcap file with magic `0xa1b2c3d4`, version 2.4, link type 1 (Ethernet), snap length `0xffff`.
5. Scans inbound data for the literal string `<ServerLocation SessionKey="`, extracts the 64-byte hex AES key that follows, and writes it to the `-keys.txt` file. This is the key needed to decrypt the captured `.pcap` offline.

### Hook table layout (pending verification)

The previous revision of this doc presented the following internal hook-function and storage-slot addresses as fact. Without a working Ghidra session against `AtreaRL.dll` on the current build, treat them as **speculative — verify against AtreaRL.dll** before relying on any one of them:

| Original function | Hook function | Storage slot | Purpose |
|---|---|---|---|
| `send` | `FUN_1002c830` (speculative) | `DAT_10042cd8` (speculative) | Capture outbound TCP |
| `sendto` | `FUN_1002c750` (speculative) | `DAT_10042ce0` (speculative) | Capture outbound UDP |
| `WSARecv` | `FUN_1002cc00` (speculative) | `DAT_10042ce4` (speculative) | Capture inbound TCP |
| `WSARecvFrom` | `FUN_1002c870` (speculative) | `DAT_10042cdc` (speculative) | Capture inbound UDP |
| IAT-trampoline installer | `FUN_1002b950` (speculative) | — | Replace IAT entry, save original pointer |
| PCAP writer | `FUN_1002c0c0` (speculative) | — | Write pcap magic + per-record header |
| Per-packet capture builder | `FUN_1002beb0` (speculative) | — | Build synthetic L2/L3 headers |
| Mercury link registration | `FUN_1002c270` (speculative) | — | Allocate per-connection tracking object |
| AES key extraction | `FUN_1002c3f0` (speculative) | — | Search for `<ServerLocation SessionKey="` |

The Mercury link log string from the previous revision was `"Sniffer: Registering Mercury link: %08x:%d -> %08x:%d"` — that string is plausible but unconfirmed in the current binary; if you re-run the Ghidra pass, search for it as the anchor for `FUN_1002c270`.

## Command-line parser

The command-line parser handles the flags forwarded by AtreaLoader.exe. Address `FUN_1002b340` from the previous revision is speculative — verify against AtreaRL.dll.

| Argument | Effect |
|---|---|
| `--enable-group=<name>` | Enable a patch/symbol group from the config (e.g. `Mercury`, `Editor`) |
| `--disable-group=<name>` | Disable a patch/symbol group |
| `--nvp-<key>=<value>` | Override a runtime NVP value |

## Output files

| Path | Format | Content |
|---|---|---|
| `binaries/sessions/YYYY-MM-DD_HH-MM.pcap` | pcap (Wireshark) | Full Mercury network capture with synthetic L2/L3 headers |
| `binaries/sessions/YYYY-MM-DD_HH-MM-keys.txt` | text | 64-byte hex AES session key extracted from `<ServerLocation SessionKey="...">` |
| `binaries/AtreaLoader.config.xml` | XML (source) | Editable patch/symbol/NVP table — does *not* drive runtime |
| `binaries/AtreaLoader.config` | binary (runtime) | Compiled form of the above — what AtreaRL actually reads |

## What this DLL does not do

- **No SOAP-login endpoint patch.** A previous revision claimed "binary patches likely redirect the SOAP login endpoint from stargateworlds.com to the emulator's auth server." There is **no such patch** anywhere in `AtreaLoader.config.xml` — every patch in the table targets either log-flag bytes (`EnableUnicodeLogger`, `EnableLocalizedDebug`, `EnableAppearanceLogger`), editor-mode flags (`EditorMode`, `EditorCallbacks`, `EditorSettings`, `EditorCurrentPackage`, etc.), or splash/silent UI tweaks. The login-server redirect is **client-side Lua**, applied by `setup.ps1` editing `Login.lua` to point at the emulator's auth server. See [docs/client-tools.md](../client-tools.md) for the redirect flow.
- **No login interception in the DLL.** AtreaRL captures the AES key by scanning inbound network bytes for a fixed XML string — it does not interact with the SOAP login pipeline at all.

## Cross-links

- [docs/technical/atrealoader-exe.md](atrealoader-exe.md) — The injector that loads this DLL into SGW.exe.
- [docs/technical/atrealoader-config.md](atrealoader-config.md) — Full patch/symbol/NVP table transcription of `AtreaLoader.config.xml`.
- [docs/reverse-engineering/findings/atrea-editor.md](../reverse-engineering/findings/atrea-editor.md) — What the editor-group patches activate inside SGW.exe (the dormant UnrealEd build).
- [docs/drafts/spec/mercury-wire-format.md §S9](../drafts/spec/mercury-wire-format.md) — Modern-Windows breakage analysis: the ASLR fix and the sniffer NVP gate.
- [docs/drafts/spec/mercury-wire-format.md §2.9](../drafts/spec/mercury-wire-format.md) — `MercuryLogger` / `AnsiLogger` anchors and how `EnableUnicodeLogger` toggles them.
- [docs/client-tools.md](../client-tools.md) — End-user view of the three executables (Launcher, AtreaLoader, AtreaRL) and the `Login.lua` redirect.

## Documentation debt

Two things are worth a follow-up RE pass:

- **Re-verify every speculative function address** in this document against the current `AtreaRL.dll` (Ghidra refused to open it in this pass due to a language-version mismatch — `Minor language change 4.1 -> 4.6`; resolve by re-importing under the current Ghidra build). The hook-table layout in particular should be re-anchored before being cited from elsewhere.
- **Confirm the previous revision's source-filename claims** (`appmem.cpp`, `patch.cpp`, `sniff.cpp`) were dropped because they appeared as fact without evidence. If those strings exist in the DLL (as `__FILE__` macros or debug data), search for them and cite the address. Otherwise leave them out — the loader is not open source and the filenames are not authoritative.

[^mercury-logger-anchor]: `binaries/AtreaLoader.config.xml` line 192 declares `<Symbol Name="MercuryLogger" Address="0x0041C2E0" Group="Mercury" Patch="false" />`. The companion patch at line 4 declares `<Patch Name="EnableUnicodeLogger" Group="Mercury" Apply="true" BaseAddress="0x01AF2224">`. Both anchors are cross-referenced in [mercury-wire-format.md §2.9](../drafts/spec/mercury-wire-format.md).
