---
title: "CM-00: scriptable editor bootstrap probe"
type: analysis
audience: engineers, map authors
last_updated: 2026-10-06
---

# CM-00 automation probe

The goal is a reproducible command-driven editor path for constructing an
original map, rather than relying on mouse placement. **No playable, authored
`.umap` exists yet.** This note records what was actually tested on 2026-10-06.

## Results

1. Copied the QA `Working` tree into a disposable local temp directory:
   7,977 files and 9,784,988,987 bytes on both sides. No stock QA file was
   edited. The copied `SGW.exe` has ASLR byte `0x186 = 0x00` already.
2. Invoked `AtreaLoader.exe --enable-group=Editor -SHOWLOG` in the copy.
   Its session log reported **8 Editor-group actions, 9 of 18 patches
   applied, 8 symbols patched, initialization finished**. The process did
   not reach a usable editor window in this shell. Two symbol-size warnings
   appeared. This disproves the initial assumption that absence of a visible
   `AtreaLoader.config` alone prevented patch initialization. The companion
   XML exists; the exact configuration-loading path remains to be reconciled
   with older loader documentation.
3. Ghidra `SGW.exe` `FUN_00417fe0` reads command-line `EXEC=` at
   `0x004183c8` and constructs `exec <value>` at `0x004183e3`. The decompiled
   startup block does **not clearly show the resulting command dispatched**;
   execution in editor mode has not been observed. `MAP NEW`, actor placement
   and package save commands must be tested through it, not assumed.
4. Built [a fail-closed PE patcher](../../../tools/map-lab/patch_editor.py)
   for the XML's seven Editor groups. All 11 chunks matched the copied QA
   executable, including wildcard bytes; 21 bytes changed. Python unit tests
   and the real-file dry run passed. The separately patched executable and
   the unmodified one both stopped at an early `Message` modal from this
   automated shell, with no new map file or editor log. That comparison does
   not attribute the modal to our patcher.
5. The native Computer Use service could not inspect the modal: launch
   returned `GetCursorPos failed: Access is denied`. The visual/error text
   remains unknown. No client-load test was possible.
6. Began the direct package fallback with `upk_patch retain-level-actors`.
   Applied to the 5,459-byte `Tollana_Curia-00000000.umap` in disposable
   scratch, it reduced the Level actor refs **27 → 1** (retaining `WorldInfo`).
   All 61 exports remain, 60 export serializations are byte-identical and
   only the Level export changed. The uncompressed 71,409-byte output reopens
   and passes the patcher's structural verification. This is an empty
   **technical scaffold**, not a custom playable map: orphaned terrain
   exports remain, its package name/GUID are stock, no authored geometry is
   present and no client has loaded it. A hermetic regression test for the
   actor-array operation passes.
7. Rewrote the three small `Tollana_Curia` package scaffolds under the
   same-length `Cimmeria_Lab1` identity, with distinct deterministic package
   GUIDs. The scratch persistent level, sublevel and MapData package reopen
   in the parser, and their name tables contain the new map name. This is a
   package-identity experiment, **not** the requested new map: the persistent
   level still holds stock logic/objects, the sublevel retains unreferenced
   terrain exports, and no client load has been observed. `alias_package.py`
   deliberately handles only same-length names in uncompressed packages;
   it has not audited all cross-package dependencies.
8. Cloned one `GA-Props.GA-Cover_INT_Med03` mesh actor from the cooked
   `Harset_CmdCenter-00000000.umap` into that stripped sublevel at UE
   `(0, 0, 0)`. The Level's placed-actor list grew **1 → 2**, and the output
   reopens with 16 new names, six imports and two exports. The property-name
   audit passed. This is the first demonstrated cross-package asset
   placement, but rendering, collision and cover behavior remain untested.
9. `upk_info --mesh-actors` found `SGC_Interior.SGC-RoundRoom_Floor00` at
   export 1132 in `SGC-00000002.umap`. A clone attempt identified a raw
   `IrrelevantLights` array as 16-byte GUIDs (196 and 36-byte observed
   values); the remapper now accepts only this exact shape, with a regression
   test. The next guarded failure is **276 bytes of unknown post-property
   data** on export 1877. The floor has **not** been placed. That native
   component tail needs format recovery before the direct route can use this
   floor asset. No override or byte pass-through was used.

## `EXEC` file format, recovered from the executable

This is a text file of native Unreal/SGW console commands, **not** JSON,
Python or a separate map description language. `APlayerController`'s `EXEC`
branch at `0x005d5520` reads a filename and calls `FUN_005d3370`. That
function prepends `..\Binaries\` unless the supplied path already contains
`Binaries`, loads the file as text, iterates commands, and sends each to its
`Exec` virtual dispatch. The line iterator at `0x00487530` gives the exact
lexical rules:

- CR and LF separate commands. A pipe `|` also separates commands outside
  double quotes.
- `//` begins a comment outside double quotes; the rest of that command is
  skipped.
- Double quotes protect `|` and `//` and toggle quote state. There is no
  observed backslash-escape rule in that iterator.
- Empty lines are accepted by the iterator. The file loader converts bytes
  to the engine's wide string representation; accepted on-disk encodings
  have not been tested.

For example, a candidate smoke file is:

```text
// CM-00 startup probe
QUIT
```

The **file language is established** by the handler, while the startup
`EXEC=` route and individual editor verbs are still unproven. In particular,
the New Level dialog invokes `0x00bf7d40` directly; no `MAP NEW` text command
to that same function has been found. A script cannot yet be said to create a
new level. The editor's `OBJ` command parser at `0x00bf57ed` also recognizes
`EXEC` but returns false so the controller/common dispatcher can handle it.

## Next executable experiment

Run the copied QA editor in an interactive Windows desktop, capture the
startup modal text and its log, then try a one-line `EXEC=` command file with
a harmless `QUIT` first. If it executes, use an authored command file to make
one floor, wall, light and stock mesh, save under a new package name, reopen
and load in a clean game client. Record every output file and dependency.
Only after this succeeds does [CM-02](work-packets.md) build rooms and terrain.

If the editor's startup or command-file path cannot be made reliable, the
alternative is a direct package authoring tool. That must create a fresh
world/level and geometry exports with correct name/import/export/depends
tables, object references, collision and cooker flags. The existing
append-only `crates/upk` patcher now has one scaffold operation but does not
perform full authoring today. A copied and renamed map would not satisfy the
goal.
