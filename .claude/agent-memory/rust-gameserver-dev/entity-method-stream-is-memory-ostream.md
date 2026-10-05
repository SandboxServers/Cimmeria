---
name: entity-method-stream-is-memory-ostream
description: The stream EntityManager::onEntityMethod (0x00dd2b80) gets in the live client is a MemoryOStream's BinaryIStream subobject queued by SGWMessageQueue, not the Nub's MemoryIStream; and how the silent-return bug was found statically
metadata:
  type: reference
---

Learned 2026-10-04 fixing "client.ability.recv never fires" (colo session 55959c2e).

**Fact.** SGW does not let the Nub call `EntityManager` directly. `SGWMessageQueue`
(vtable `0x01b14f3c`) is the `ServerMessageHandler`; its slots copy each message into
a `Detail::*Message` (`EntityMethodMessage` vtable `0x01b14eb8`, ctor `0x01561a20`,
`process` `0x01561ac0`) holding a `MemoryOStream` at `+0xc`. `process` passes `this+0x10`,
the `MemoryOStream`'s `BinaryIStream` subobject: vtable `0x019ce734`, slot 2 `0x00dd3f80`
= `[+0xc] - [+0x14]` → read cursor `+0x14`, end `+0xc`. The Nub's stack `MemoryIStream`
(`0x01b18e38`, cursor `+8`, end `+0xc`) never reaches the EntityManager in practice.
Any hook that reads an EntityManager handler's `stream` by offset must handle this layout
(`hooks/ability_trace/recv_stream.rs`). Calling the stream's own slot 2 works for both,
which is why `client.mercury.entity_method` `len` looked right and hid the problem.

**How it was found without the lab.** Every static check of the Rust path passed, so I
took "the live object is not what the doc says" as the hypothesis: enumerated RTTI
names containing `Stream` (found `MemoryOStream` with an IStream subobject at offset 4),
listed callers of the `MemoryOStream` ctor (`0x00dd3c60`), and one of them was a
`Detail` message ctor, which led to the RTTI family `*Message@Detail@@`. The scripts
(pefile + capstone, `py -V:3.13`) also disassembled the installed DLL to confirm the
detour actually called the decoder. Tip: an `Option`-returning reader that bails before
emitting needs a "skipped" diagnostic; silent `?` chains in hooks cost a whole live run.

Related: [[offline-disasm-and-minhook-detour-tests]], [[client-handler-abi-and-static-disassembly]].
