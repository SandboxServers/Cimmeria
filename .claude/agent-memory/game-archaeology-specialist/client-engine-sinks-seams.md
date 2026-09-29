---
name: client-engine-sinks-seams
description: Engine-layer (BigWorld/UE3/log4cxx/OS/middleware) log sinks and silent-failure seams recovered for the client-telemetry DLL, with the non-obvious findings (compiled-out UE3 logging, dead UnrealAppender, discarded PhysX stream, GNames address, movie-from-memory)
metadata:
  type: project
---

Full evidence: `docs/reverse-engineering/findings/client-engine-sinks-and-seams.md`; code: `crates/client-telemetry/src/hooks/sinks/` and `seams/`. 2026-09-28, all statically verified (Ghidra + PE tables + log4cxx.dll bytes), none seen live.

Non-obvious facts worth not re-deriving:

- **UE3 `debugf`/`warnf` are compiled out.** Real `Launch.log` = "Log file open" + "Release logging initialized." The `GLog` redirector hook (`0x004ce0b0`) is low volume by design. `check()` failures (`0x00486000`) are reported and *survived* (returns), so that hook is the only view of them.
- **`log4cxx::UnrealAppender` forwards nothing** (its targets are the compiled-out functions), so log4cxx is not duplicated in `GLog`. The real `SGWDebugLog.log` is 65k lock-trace lines (`DEBUG common - inside writeLock`) plus a few real ERRORs; rate-limit keys must collapse digits (`text::message_shape`).
- **BigWorld's message choke point is `0x00a36460`** (only two xrefs, both wrappers); the filter is `header[0] + impl[0x3c] <= header[1]`.
- **UObject layout in this build**: Outer `+0x28`, Name `+0x2c/+0x30`, Class `+0x34`. `FName::Names` is `0x01ecade0` (data) / `0x01ecade4` (num); `address-map.md` said `0x01eadbc0` (wrong, corrected). FNameEntry: flags `+8` (bit `0x1000` = suppressed log name), wide name `+0x10`.
- **PhysX errors are discarded by the client**: `FNxOutputStream::reportError` (vtable `0x01839d94` slot 0) only string-compares two texts. Hooking it is the only view of PhysX warnings.
- **Bink movies open from memory** (`_BinkOpen_8(buffer, 0x4004400)`), so there is no filename at the import; the name is upstream at `FUN_00509820`.
- **`__stdcall` members (FMOD `QAG`) pass `this` on the stack**, not in ECX (confirmed at the call site `0x00903698`).
- The Ghidra MCP `list_imports` is polluted by wx imports and paged; get IAT slot VAs from the PE import directory with `pefile` (same file as Ghidra's program).
- Tool gotchas: the harness refuses compound `cd ... && git`/`python - <<EOF` shell lines in a worktree; put scripts in the scratchpad and run them with PowerShell. The Dev Drive can fall under the lane's 10 GB free floor when other agents build the workspace; `LANE_MIN_FREE_GB=4` is enough for a single-crate build.
