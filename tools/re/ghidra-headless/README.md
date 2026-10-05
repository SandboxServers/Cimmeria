---
title: Headless Ghidra probe
type: how-to
audience: contributors and agents doing static RE on SGW.exe without the Ghidra GUI or the Ghidra MCP bridge
last_updated: 2026-10-03
companion_docs:
  - ../../../docs/guides/re-toolchain-setup.md
  - ../../../docs/guides/reverse-engineering-with-claude.md
  - ../../../docs/reverse-engineering/findings/cme-event-signal.md
---

# Headless Ghidra probe

[Probe.java](Probe.java) is a read-only `GhidraScript` you run through Ghidra's `analyzeHeadless.bat` against an already analyzed `SGW.exe` project. It decompiles, lists xrefs, searches strings and function names, and dumps vtables, with no Ghidra window open and no MCP bridge. Use it when the Ghidra MCP tools are unreachable from your session, or when you want to batch many lookups into one run.

It covers static analysis only. For anything on the running client, use the x64dbg MCP or the Live Research Lab (see [reverse-engineering-with-claude.md](../../../docs/guides/reverse-engineering-with-claude.md#static-vs-runtime--ghidra-x64dbg-and-the-lab)).

## Before you start

- **Ghidra** 12.0.4. The command below uses `C:\ghidra_12.0.4_PUBLIC`, the default install path: adjust it to yours. `analyzeHeadless.bat` is in its `support\` folder.
- **An analyzed project.** The script opens the existing Ghidra project `SGW` in your client's `<SGW client>\Working\binaries` folder with `-noanalysis`, so auto-analysis must already have been run there from the GUI. The client is not in git.
- **No other Ghidra holding the project.** Close the GUI's copy of the project first (see the lock caveat below).

## Run it

From PowerShell or Git Bash (Windows paths either way, since this calls a `.bat`):

```bat
C:\ghidra_12.0.4_PUBLIC\support\analyzeHeadless.bat "<SGW client>\Working\binaries" SGW ^
  -process SGW.exe -noanalysis -readOnly ^
  -scriptPath <repo>\tools\re\ghidra-headless ^
  -postScript Probe.java D:<addr> X:<addr> S:<text> FNSUB:<substr> VT:<addr>,<n> FINDPTR:<addr> ^
  > probe-out.txt 2>&1
```

Replace `<SGW client>` with your client's `Stargate Worlds-QA` folder, the Ghidra path with your install, and `<repo>` with your Cimmeria checkout. Addresses are hex (`0x00a5c150`). Everything after `Probe.java` is a token list, processed in order.

The script's output lands in the redirected file with each line prefixed `INFO  Probe.java>`. Every token prints a header line starting with `===`, so `grep "=== "` finds your sections.

For example, to look for native subscribers to the trade-result event (the kind of search behind [trade-result-client-handling.md](../../../docs/reverse-engineering/findings/trade-result-client-handling.md)):

```bat
... -postScript Probe.java S:Event_NetIn_TradeResults FNSUB:TradeResult X:0x01e5e7f8
```

## Tokens

| Token | What it does |
|---|---|
| `D:<addr>` | Decompile the function containing `<addr>`. |
| `X:<addr>` | List references to `<addr>`, with the function each one sits in. |
| `S:<text>` | Case-insensitive substring search over defined strings. This is how you find RTTI type-name strings (`.?AV?$MemberCallback@...`). |
| `FNSUB:<substr>` | Case-insensitive substring search over function names. |
| `VT:<addr>,<n>` | Dump `n` 4-byte slots at `addr` as a vtable, naming each target function (default 12). |
| `FINDPTR:<addr>` | Scan initialized memory for the 4-byte little-endian value `addr`: a raw data xref for when `-noanalysis` left no recorded reference. See the caveat below. |
| `F:<addr>` | Create a function at `addr` if none exists, then decompile it. The function is not saved (`-readOnly`). |
| `N:<addr>` | Decompile, printing only small-integer compare lines. Fast triage across near-identical template instantiations. |
| `ND:<name>` / `NX:<name>` | Decompile, or list references to, the function with this exact name. |
| `PREVFN:<addr>` | Decompile the nearest function that starts before `addr`. |
| `PTR:<addr>` | Read the pointer at `addr` and decompile its target. |
| `I:<addr>,<n>` | Disassemble `n` instructions from `addr` (default 30). |
| `DATAAT:<addr>` | Show the defined data at `addr` and its components. |
| `DEM:<mangled>` | Demangle an MSVC symbol. |
| `IF:<addr>` | Disassemble the whole function containing `<addr>`. |
| `RE:<regex>` | Scan every instruction's text for the regex and print the function and address of each hit (capped at 400). Avoid `^`, `&`, `<`, `>` and the pipe character: `cmd` treats them as operators (use `\s` for spaces and no anchors). |
| `BYTES:<addr>+<n>` | Hex dump `<n>` bytes (default 64) where no code or function is defined. Use it to find a function prologue in an undisassembled region, then `F:` it. |
| `U16:<text>` | Find `<text>` as UTF-16LE in initialized memory and list each hit with its references. The string search does not define wide literals. |
| `BLOCKS` | List the memory blocks. |

## Caveats

- **`FINDPTR` is not reliable for RTTI chains in this binary.** It is fine for ordinary pointer references. For the RTTI Complete Object Locator chain it gives false positives: a scan for the `Trade` trade-result callback's locator led to vtable `0x019d8928`, which belongs to an unrelated `MemberCallback` specialization. Treat its hits as leads to check, never as results.
- **`DEM` does not demangle raw RTTI type-name strings** (`.?AV?$...`). Demangle those by hand; the finding docs show worked examples.
- **Always pass `-readOnly`.** It guarantees nothing in the project changes. The project is still opened exclusively: a killed or crashed run can leave `SGW.lock` and `SGW.lock~` in the project folder. If no Ghidra or `java` process is running, delete those two files and retry.
- **Each run pays JVM start and project open**, whatever the token count: 7-30 s in [experiment G](../../../docs/analysis/token-usage/experiment-g.md) on a warm machine, up to a minute or two cold. Batch tokens into one run.
- **A comma ends the token.** `analyzeHeadless.bat` passes its arguments through `cmd`, which splits at commas even inside quotes, so `VT:<addr>,<n>` and `I:<addr>,<n>` lose `<n>` and fall back to 12 slots and 30 instructions. Write `VT:<addr>+<n>` and `I:<addr>+<n>` instead: `+` is accepted as the separator.
- **A compile error skips the whole script** but the run still exits 0. Check the output for `error:` before trusting a quiet result.
- The Java class name must match the file name (`Probe`).

## Related

- [docs/guides/re-toolchain-setup.md](../../../docs/guides/re-toolchain-setup.md) — installing Ghidra, the MCP bridges and x64dbg
- [docs/guides/reverse-engineering-with-claude.md](../../../docs/guides/reverse-engineering-with-claude.md) — the RE workflow, and when to fall back to this probe
- [docs/reverse-engineering/findings/cme-event-signal.md](../../../docs/reverse-engineering/findings/cme-event-signal.md) — finding native event subscribers from RTTI strings
