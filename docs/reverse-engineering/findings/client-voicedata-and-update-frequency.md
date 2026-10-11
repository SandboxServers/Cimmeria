# Client `voiceData` (0x33) and `updateFrequencyNotification` (0x02): what the handlers do

> **Date**: 2026-10-11
> **Method**: headless Ghidra (`tools/re/ghidra-headless/Probe.java`, read-only) on `SGW.exe`: decompile plus disassembly and vtable dumps of every function named below. No live client, no x64dbg.
> **Status**: statically verified. Items marked **inferred** are read from code shape or library contract and have not been observed live.
> **Feeds**: lab-chaos CH-07 (a harmless filler message in the first-login bundle); corrects the VOICE_DATA section of [space-viewport-wire-formats.md](space-viewport-wire-formats.md).
> **Related**: [client-mercury-receive-path.md](client-mercury-receive-path.md), [client-entity-lifecycle.md](client-entity-lifecycle.md), [system-protocol-wire-formats.md](system-protocol-wire-formats.md).

## Summary

`voiceData` is not dropped and not log-only in the world. `ServerConnection+0x168` is `pHandler_`, set at `logOn` to the `SGWMessageQueue`'s `ServerMessageHandler` subobject, so every 0x33 message is copied into a `Detail::VoiceDataMessage`, queued to the game thread, and handed to `EntityManager::onVoiceData` (`0x00dd0980`). That function reads the remaining length, runs four empty logging stubs, retrieves the whole body and discards it. A zero-length body and a 1 to 33 byte all-zero body have no visible effect, no state change and no crash path found. `updateFrequencyNotification` stores one byte as a float in a global; resending the same value is idempotent.

Verdicts: **voiceData as a filler: yes** (static; the only inferred step is TBB `scalable_malloc(0)` returning non-null). **updateFrequencyNotification resent with the same value: yes.**

## 1. The handler installed as `pHandler_` and its slot 13

| Fact | Evidence |
|---|---|
| `ServerConnection_voiceData` (`0x00dd68b0`) tests `this+0x168`; non-null: calls `(*(*h + 0x34))(param_1, param_2)` (vtable slot 13), null: logs `"Got voice data before a handler has been set."` | decompile |
| `this+0x168` is set by `ServerConnection_logOn` (`0x00dd8ec0`); its only caller `FUN_00c6edf0` passes `*(this+0x30) + 0x1c` as the third argument | decompile of `0x00c6edf0`; docs `client-entity-lifecycle.md` |
| The object at `+0x30` is an `SGWMessageQueue`; its constructor `FUN_015622b0` stores the `ServerMessageHandler` vtable `0x01b14f3c` at `+0x1c` | disassembly `0x01562314` (`MOV [ESI+0x1c],0x1b14f3c`) |
| Slot 13 of `0x01b14f3c` is `0x01b14f70` -> `FUN_01563990`: allocates a `0x2c`-byte `Detail::VoiceDataMessage` (`FUN_01561af0`), takes the queue's critical section, pushes the message (`FUN_00571fc0`) | decompile |
| `FUN_01561af0` (the message constructor): copies the 8-byte source address, sizes a `MemoryOStream` from `stream->remainingLength()` (vtable `+8`), then `stream->retrieve(len)` (vtable `+4`) and appends it (`addBlob`, vtable `+0xc`). Vtable `0x01b14ec4`. | decompile and disassembly |
| `process` (`FUN_01561b90`) runs on the game thread and calls `EntityManager` (singleton `0x01ef244c`, getter `0x00dd05a0`) vtable `+0x34` = slot 13 = `0x019aaef8` -> **`FUN_00dd0980`** | decompile, vtable dump of `0x019aaec4` |
| `FUN_00dd0980(addr, stream)`: `len = stream->remainingLength()`; if the entity-manager debug logger `DAT_01ef2224` is set, calls `0x00c66860`, `0x00c66880`, `0x00c66870`, which are each a bare `RET` / `RET 8` (compiled-out logging); then `stream->retrieve(len)` and returns. | disassembly of `0x00dd0980` and the three stubs (`RET`, `RET 0x8`, `RET`) |

So slot 13 is the stock BigWorld `onVoiceData`, implemented by `EntityManager` as "consume and discard". The data is never stored, parsed, or forwarded to an audio path. (`FUN_00c66840`, the lazy logger init, is also a bare `RET`.)

### Zero-length and 1 to 33 byte zero bodies

* **Zero length.** `remainingLength() = 0`; `FUN_00dd3c60` builds the `MemoryOStream` with `scalable_malloc(0)` and throws only on a null return. TBB's `scalable_malloc(0)` returns a minimum-size block (**inferred** from the library contract; the call is an import thunk `0x00457e00` and was not traced into TBB). `retrieve(0)` on a `MemoryOStream` (`FUN_00dd3f00`, vtable `0x019ce738`) tests `pRead + n <= pCurr`, true for `n = 0`, and does not set the error flag. `addBlob(.., 0)` copies nothing.
* **1 to 33 bytes.** Same path with `n` bytes copied into the heap block and discarded; the bytes are never interpreted, so all-zero is irrelevant. The message object is freed by its scalar-deleting destructor `0x015614e0`.
* **Crash risk.** None found in this path. The one inferred step is TBB's zero-size behaviour above.
* **Side effects.** `SGWMessageQueue` growth of one transient message per 0x33. No entity, UI or sound call is reachable from `FUN_00dd0980`.

## 2. Length prefix of 0x33

Confirmed from the client's own registration, not from the dispatch-table doc. The static initializer at `0x017bcc10` fills the handler object `0x01e51e58` with `{vtable 0x019d12c8, handler 0x00dd68b0}` (`0x017bcc2d`, `0x017bcc37`), then registers it with the client interface (`ECX = 0x01ef2518`, `0x015785c0`) as `(name 0x019d0f34 "voiceData", style 1, width 2, handler 0x01e51e58)` (`0x017bcc77`-`0x017bcc90`). The sibling `updateFrequencyNotification` registers as `(name, style 0, length 1)` (`0x017bb0e7`-`0x017bb100`). Style 1 is variable length and the number is the width of the length field: the header length helper `0x0158aa40` returns `1 + width` for a variable element (see [client-mercury-receive-path.md](client-mercury-receive-path.md) step 2), so `voiceData` is `WORD_LENGTH`, a 3-byte header: `33 lo hi`, then `n` body bytes. The id `0x33` itself comes from registration order (`FUN_015785c0` uses the vector's current element count as the id); the name-to-id pairing here rests on the live 57-entry table capture in `message-dispatch-table.md`, not re-derived in this pass.

## 3. `Bundle::iterator::data()` (`0x01579a50`) for a zero-length body

Returns a non-null pointer. Its first test is `if (length + bodyOffset <= packetDataLength) return packet + bodyOffset + 0x54;` (decompile lines `param_1[3] + *(ushort*)(param_1+2) <= *(ushort*)(param_1+1)`). With `length = 0` the left side is the body offset, which `unpack` already proved lies within the packet, so the in-place branch is taken and the pointer is `packet + 0x54 + offset`, never null. The null returns are the run-out-of-packets branches, reachable only for a body that does not fit. The loop then dispatches normally, and `next()` advances by `bodyOffset + 0` (the next message starts right after the 3-byte header).

## 4. `updateFrequencyNotification` (0x02)

Handler `FUN_00dd62a0` (previously un-created in Ghidra; bytes: `MOV EAX,[ESP+4]; MOVZX ECX,byte [EAX]; CVTSI2SS XMM0,ECX; MOVSS [0x01e51cb8],XMM0; RET 4`). It reads one `u8` and stores it as a float in the global `DAT_01e51cb8`, the divisor the game-time getter `FUN_00dd6c60` uses (see [system-protocol-wire-formats.md](system-protocol-wire-formats.md) and [crafting-client-ui.md](crafting-client-ui.md)). It runs directly in the Nub loop (a `ClientMessageHandler`, vtable `0x019d13b0`, not via `SGWMessageQueue`). Nothing else is touched, there is no change detection and no edge-triggered side effect, so resending the same value (10) mid-login rewrites the same float: idempotent and harmless. Only a different value would shift the clock divisor.

## Open questions

* TBB `scalable_malloc(0)`: a one-line live or import-level confirmation would turn the single inferred step into a confirmed one.
* The 0x33 name-to-id pairing was taken from the earlier live-capture table, not re-derived here.

## Evidence index

`0x00dd68b0`, `0x00dd8ec0`, `0x00c6edf0`, `0x015622b0` (ctor, `0x01562314`), vtables `0x01b14f3c` and `0x01b14ec4`, `0x01563990`, `0x01561af0`, `0x01561b90`, `0x015614e0`, `0x00dd0980`, `0x00c66840`-`0x00c66880`, `0x00dd3c60`, `0x00dd3f00`, `0x017bcc10`, `0x017bb0a7`, `0x015785c0`, `0x01579a50`, `0x00dd62a0`.
