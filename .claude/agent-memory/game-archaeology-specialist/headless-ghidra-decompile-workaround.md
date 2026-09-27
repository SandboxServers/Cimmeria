---
name: headless-ghidra-decompile-workaround
description: How to get live Ghidra decompile/xref/string-search access from an agent session WITHOUT the GUI or the GhidraMCP bridge, using analyzeHeadless.bat + a custom GhidraScript — the fix for the tools/list_changed tool-discovery gap
metadata:
  type: reference
---

The GhidraMCP bridge (see [[reference-mcp-servers]]) requires Ghidra's GUI to be open with the
plugin's MCP server started, and even then this harness's tool-discovery mechanism doesn't reach
the 195 tools the bridge registers dynamically on connect (`decompile_function`, `get_xrefs_to`,
`search_functions`, etc.) — see [[cooked-dialog-override-crash-na-unnumbered]] for a session that
hit this dead end. **Headless Ghidra sidesteps the whole problem**: no GUI, no MCP bridge, just
`analyzeHeadless.bat` running a `GhidraScript` non-interactively against the existing analyzed
project. Confirmed working 2026-09-27.

## Recipe

1. Write a `GhidraScript` (plain Java) with a `run()` method. Minimal template used successfully:

```java
import ghidra.app.script.GhidraScript;
import ghidra.app.decompiler.*;
import ghidra.program.model.listing.*;
import ghidra.program.model.address.*;
import ghidra.program.model.symbol.*;

public class Probe extends GhidraScript {
  public void run() throws Exception {
    DecompInterface d = new DecompInterface();
    d.openProgram(currentProgram);
    for (String s : getScriptArgs()) {
      // s.startsWith("D:") -> decompile the function containing that address
      // s.startsWith("X:") -> currentProgram.getReferenceManager().getReferencesTo(addr)
      // s.startsWith("N:") -> decompile then grep the C text for a pattern (fast triage
      //   across many near-identical template instantiations without dumping full bodies)
      // s.startsWith("S:") -> currentProgram.getListing().getDefinedData(...) string search
    }
  }
}
```

Full working version (with all four token types) is at
`C:\Users\Steve\AppData\Local\Temp\claude\C--Users-Steve-source-projects-Cimmeria\
13c6afcb-2df4-4497-b1aa-157737231c3c\scratchpad\ghs2\Probe.java` as of this session — copy it
rather than starting from the minimal template above.

2. Save the script under its own directory (NOT inside a Ghidra install/project dir) —
   `-scriptPath` takes that directory.

3. Invoke from Git Bash (or PowerShell — use Windows-style paths either way since this is calling a
   `.bat`):

```bash
cd "/c/Users/Steve/source/projects/SGW/Stargate Worlds-QA/Working/binaries"
/c/ghidra_12.0.4_PUBLIC/support/analyzeHeadless.bat \
  "C:\\Users\\Steve\\source\\projects\\SGW\\Stargate Worlds-QA\\Working\\binaries" SGW \
  -process SGW.exe -noanalysis -readOnly \
  -scriptPath "C:\\...\\ghs2" \
  -postScript Probe.java D:0x00441630 X:0x00441630 N:0x00443c10 S:CookedDataDialogs \
  > out.txt 2>&1
```

`-noanalysis` skips re-running auto-analysis (the project was already fully analyzed by a prior
GUI session — this just opens it read-only). `-readOnly` guarantees nothing in the project gets
modified. Output (including your script's `println`) lands in the redirected file, prefixed
`INFO  Probe.java> ` per line — grep for `=== ` to find your own section markers.

## Gotchas

- **~1-2 minute fixed cost per invocation** (JVM start + project open), independent of how many
  addresses you pass. **Batch many addresses/tokens into one run** rather than one-address-per-run
  — this is the main lever for keeping the total wall-clock down.
- **Exclusive project lock, one run at a time.** If a run is killed or crashes, it can leave
  `SGW.lock`/`SGW.lock~` in the project directory. If no `javaw`/`analyzeHeadless` process is
  actually running (`tasklist | grep -i java`), it's safe to delete those two files and retry.
- **Java class name must match the file name exactly** (`Probe.java` → `public class Probe`), or
  you get `GhidraScriptLoadException: ... not found by <hash>`.
- **Package availability varies by Ghidra version.** `ghidra.program.model.listing.DataIterator`
  exists; `ghidra.program.model.data.DataIterator` does not (in Ghidra 12.0.4) — a compile error
  here silently skips your *entire* script for that run (the error appears in the output but the
  run still exits 0), so check the output for `error:`/`skipping` before trusting a clean-looking
  result.
- **Template-instantiated functions need per-instantiation triage, not one address.** SGW.exe's
  cooked-data `ServerSource<N,...>` code (and likely other C++ templates) is compiled once per
  category — `cooked-data-pipeline.md`'s documented addresses for `onVersionInfo`/
  `onCookedDataError` turned out to be **category 6's** copies, not a generic/shared body. The
  `N:` token type (decompile, then grep the C text for the category-id literal-compare line) is
  the fast way to triage which of ~20 near-identical functions is the one you actually want,
  without dumping full decompiled bodies for all of them. Found this way: category 5's
  `onVersionInfo` at `0x004435c0`, `onCookedDataError` at `0x00443a30` — see
  [[cooked-dialog-override-crash-na-unnumbered]].
- **`X:` (xrefs to a shared helper) is the fastest way to enumerate all N template
  instantiations at once** — e.g. every category's `onVersionInfo` calls the same shared per-key
  delete function, so `getReferencesTo` on that shared function's address returns all N callers in
  one shot, which you then triage with `N:`.

## When this fully replaces the MCP bridge, and when it doesn't

Use this for anything the bridge's `function`/`xref`/`listing` tool groups would do: decompile,
xrefs, string search, symbol lookup — read-only static analysis. It does **not** cover the
`debugger` group (needs a live attached process, which the MCP debugger-proxy tools already
handle and which stayed reachable through this harness even when the analysis tools didn't), and
it does not give you a GUI to look at (no CodeBrowser window, no visual xref graph) — for that,
someone still needs to open Ghidra normally.
