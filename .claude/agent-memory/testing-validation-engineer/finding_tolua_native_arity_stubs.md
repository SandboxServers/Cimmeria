---
name: finding-tolua-native-arity-stubs
description: SGW.exe Lua natives are tolua++ shims that raise on extra args (0x00403280 = isnoobj); Lua UAT stubs that accept any args hide arity bugs — PR #1213 getAbilityList
metadata:
  type: project
---

SGW.exe Lua natives are tolua++-generated shims with strict arity. `CEGUI__unknown_00403280(L, n, &err)` is `tolua_isnoobj` (`return lua_gettop(L) < |n|`), `0x00403330` is `tolua_isnumber`, `0x00402f40` is `tolua_error` (calls `luaL_error`). A shim that begins `if (!isnoobj(L,1)) error` takes ZERO args and raises when given one; a one-arg native checks `isnumber(L,1)` then `isnoobj(L,2)`.

2026-10-04, PR #1213 (009-starter-hotbar): the hook called `getAbilityList(2)`; the shim `0x00aa2740` is zero-arg, so on a real client it always raises (pcall swallowed it, feature silently dead). The UAT stub accepted any args, so 42/42 passed. An arity-faithful stub (`if select('#', ...) > 0 then error(...) end`) turned it into 27/42 FAIL; the PR's RE doc had misread `isnoobj(L,1)` as "requires one argument".

**Why:** clean-room / stub-based Lua UATs (client-patches overlay, 009 hotbar) are the only automated gate for client Lua; a permissive stub makes the most likely bug (wrong native call shape) invisible.

**How to apply:** when reviewing any client Lua UAT, check each stubbed native's arity and arg types against its shim decompile (headless Ghidra, see main-session reference_ghidra_headless) and ask for stubs that raise the same way tolua does. Related: [[workflow-revert-audit]], [[finding-self-skipping-asset-tests]].
