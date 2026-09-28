---
name: telemetry-anchor-audit-and-hookgate
description: Telemetry DLL anchors were never run in the client and five were wrong (IAT = on-disk hint/name RVAs, a string, mid-function, CEGUI destructor, ret-8 signatures); how to verify an anchor offline; the shared hookgate crate and install lock
metadata:
  type: reference
---

Found 2026-09-27 (client-io/hardening). The telemetry DLL had never been
injected into the real `SGW.exe`, so none of its anchors had run.

**Verify an anchor offline** against the QA client at
`..\SGW\Stargate Worlds-QA\Working\Binaries\SGW.exe` (image base 0x400000,
ASLR off). A Python PE parse (sections + import directory) plus capstone in
the *64-bit* Python (the 32-bit Python's capstone DLL fails to load) is
enough: check the bytes before the address are padding/`ret` (a real entry),
and count `ret N` against the detour's stack args. No Ghidra needed.

Traps that produced wrong anchors:
- **An IAT slot's on-disk value is a hint/name RVA.** Reading it as a VA
  gives an address inside `.rdata` strings. Take slot addresses from the
  import directory (`FirstThunk + 4*i + base`). Real slots are 4-byte
  aligned; the wrong ones were not.
- **A vtable address from an RTTI walk may be the COL pointer**, one slot
  before slot 0. CEGUI DefaultLogger: COL ptr `0x01ac1ba8`, slot 0 (dtor)
  `0x01ac1bac`, slot 1 logEvent `0x01ac1bb0`.
- **UE3 `exec*` thunks are `(this, FFrame&, void* Result)` = `ret 8`.**
- A "string anchor" address can be the string itself (`Mercury::Nub::
  handleMessage` at `0x01b18be0`).

Removed hooks (handleMessage, cooked-data PAK load) are #989.

**Shared crate `cimmeria-client-hookgate`**: prologue classify + hook owners
(both DLLs, both file spellings) + the two shared sites + i686 `os::` reader,
`loaded_hook_owners()` (excludes the calling DLL via
`GetModuleHandleExW(FROM_ADDRESS)`), and `HookLock` = named mutex
`Local\cimmeria-client-hooks-<pid>` held from listing owners to the last
hook. Tests use per-test mutex names: `cargo test` threads share a pid.

The lab bridge's dynamic hooks moved to MinHook: the old 5-byte prologue
copy split `push -1; push imm32` (2+5 bytes), the shape of most SGW.exe
functions. Lab-bridge i686 tests had never run in CI; three assumed
"off-target stubs" and executed arbitrary addresses in the test exe.

Related: [[injected-dll-unwind-and-lua-error-rules]].
