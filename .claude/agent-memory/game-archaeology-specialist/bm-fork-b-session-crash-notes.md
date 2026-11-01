---
name: bm-fork-b-session-crash-notes
description: Black Market fork-B session crash postmortem + Phase 2a safe read-repoint proof (2026-06-22)
metadata:
  type: project
---

Trap: `0x00403EC0` is not a standalone lua_register. It is a partial helper that does `lua_rawset(L, -3)`, so the caller must already have `_G` on the Lua stack. Calling it cold (as a flag3 handler once did) accesses an invalid stack index and crashes in lua51.dll.

Correct sequence to register a C function as a Lua global from native code:
1. `0x00403BB0(L, NULL)` (`lua_getglobal_wide`, NULL name = global table) pushes `_G`; cdecl, `add esp,8`.
2. For each function: push `cfunc`, push the wide name string pointer, push `L`, call `0x00403EC0`; `add esp,0xC`.
3. `lua_pop(L, 1)` (that is `lua_settop(L, -2)`) to drop `_G`.

Code caves allocated from the heap do not survive a client restart, so any patch built on them needs a full rebuild each session; only fixed-VA exe space persists. The 2009-client black-market read-repoint proof from the same session is in `docs/reverse-engineering/findings/black-market-client-window-patch.md`.

INVALIDATED WHEN: the 0x00403EC0 / 0x00403BB0 Lua helper addresses change with a different SGW.exe build
checked: unverified 2026-11-01
