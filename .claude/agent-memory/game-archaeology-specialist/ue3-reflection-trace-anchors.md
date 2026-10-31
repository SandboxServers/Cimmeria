---
name: ue3-reflection-trace-anchors
description: UE3 traces, reflection, view point, load flag, state field and CEGUI WindowManager in SGW.exe; static anchors plus live lab results (2026-10-31)
metadata:
  type: project
---

Full write-up with raw values: `docs/analysis/lab-autopilot/ue3-backport-verified.md` § "Static pass 2" and § "Live pass". Live facts were confirmed on the lab client on 2026-10-31; the rest is static.

- **Traces (LIVE):** SingleLineCheck `0x00875970`, thiscall on GWorld with 7 stack args, `ret 0x1c`; returns TRUE on NO hit. MultiLineCheck `0x008721b0` uses GMem `0x01ea5750`.
  - `FCheckResult` is 0x48 bytes: Actor `+4`, Location `+8`, Normal `+0x14`, Time `+0x20` (default 1.0), Item `+0x24` (default -1), Material `+0x28`, Component `+0x30`, LevelIndex `+0x40` (default -1).
  - **Pawns, including BigWorld NPCs and your own body, hit ONLY with flag bit `0x1`**: `0x20bf` hits, `0x2086` and `0x20be` miss. Passing the own pawn as Source excludes it.
  - A BSP hit returns Actor = WorldInfo and Item = the BSP node. A static mesh returns its StaticMeshActor.
  - Flags `0x3fff` raise a guarded AV at `0x00914fd3`; the client survives.
  - The AActor::Trace helper is `0x006e6d20`. The EncroachingWorldGeometry string is absent from the exe entirely.
- **Calling a 7+-arg thiscall from the lab:** `client_call_native` takes at most 6 args. Instead, VirtualAlloc an RWX page (IAT `0x017ef1f8`) and call a cdecl shim: `8b442404 ff701c ff7018 ff7014 ff7010 ff700c ff7008 ff7004 8b08 ff5020 c3`. It sets ECX = blk[0], pushes blk[7..1] and calls blk[8]. Free the page with VirtualFree (IAT `0x017ef1f4`).
- **GWorld = `0x01EE2684`** (a UWorld; name "TheWorld", class "World", LIVE). The pointer changes on every zone change.
- **Reflection:**
  - ProcessEvent is `0x004e06a0`.
  - **FindFunctionChecked is `0x0049ffb0` (LIVE)**: `__thiscall(obj, FName.Index, FName.Number, int)`, `ret 0xc`. Never pass an unknown name: it is the checked variant.
  - UFunction offsets: FunctionFlags `+0x8c`, ParmsSize `+0x9e`, Func `+0xa8`.
  - UProperty offsets: Next `+0x40`, Offset `+0x64`.
  - StaticFindObject is `0x004a2470`. The FName ctor is `0x0049e960`. GNames text is UTF-16 at `+0x10`.
- **View point (LIVE):** `0x005d9690` is the eventGetPlayerViewPoint thunk (FName `0x110c`, ProcessEvent via vtable `+0xe8`, `ret 8`). On the PC it returns the orbit view, one camera distance behind the pawn. The ASGWCamera_Player actor's `+0xDC` is always zero, so do not use it.
- **Load flag `0x01ee2b6c`:** static analysis says it is 1 during a map load. **Live, it read 0 in every sample across two zone changes**: bridge reads run on the main thread at the Tick drain, so a blocking load shows up only as a 0.6-1.4 s gap. Detect loads by a GWorld pointer change instead.
- **State field:** GameBeing `+0x158`. LIVE on an NPC: `0x0` → `0x41` on death (bit 0 Dead). Player death is untested: in SGC the player can't be killed (gmKillTarget refuses players).
- **Mouse (LIVE):** gain `[[PC+0x2B0]+0x358]` = 20.0, giving **521.5 counts/rad**. AddYaw `0x00e7e600`.
- **CEGUI (LIVE):** WindowManager `0x01f010c0`, map size `+8` = **21416** windows after login (not hundreds). The `CEGUI` global is not exposed to Lua.
- **Lab held keys (code read):** DI keys persist in the DLL until `input_release`. Only `client_move_to` has a cancel guard, and a daemon crash leaves keys held.

**Why:** the lab-autopilot back-port needs native traces, by-name reflection and a reliable view point.
**How to apply:** check the doc sections before re-deriving. Live probes need a user OK and the lab lease. Related: [[controller-mouse-input-model]], [[headless-ghidra-decompile-workaround]].
