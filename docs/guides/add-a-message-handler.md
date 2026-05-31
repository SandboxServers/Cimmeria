---
title: How to add a message handler
type: how-to
audience: engineers (first server-side feature)
last_updated: 2026-05-27
companion_docs:
  - ../protocol/client-method-dispatch-table.md
  - ../protocol/message-catalog.md
  - ../../TESTING.md
  - ../../crates/README.md
  - ../architecture/service-architecture.md
---

# How to add a message handler

This is the most common "first real feature" a new contributor builds: the game client sends a new message we haven't handled yet, and you need to route it to a handler, decode it, do the work, and reply. This guide walks through the pattern using the existing dispatcher seams.

If you haven't yet run the server end-to-end, do [the getting-started tutorial](getting-started.md) first. You need to know what "world entry" and "Mercury" mean before this will make sense.

---

## Decide which service handles it

There are three services and the message goes to exactly one of them:

| Service | What it handles | Where the dispatcher lives |
|---|---|---|
| **Auth** | Pre-game: login, shard select. SOAP/HTTP. | [`crates/services/src/auth/handlers.rs`](../../crates/services/src/auth/handlers.rs) |
| **Base** | Account-level state, chat, persistence, world entry. Most non-spatial messages. | [`crates/services/src/base/dispatch.rs`](../../crates/services/src/base/dispatch.rs) and the world-entry methods under [`crates/services/src/base/world_entry_methods/`](../../crates/services/src/base/world_entry_methods/) |
| **Cell** | Spatial / runtime: movement, combat, abilities, AoI. | [`crates/services/src/cell/`](../../crates/services/src/cell/) (per-system dispatchers) |

If you're not sure which it is, search [`docs/protocol/client-method-dispatch-table.md`](../protocol/client-method-dispatch-table.md) for the method index — the table identifies the target service for every documented method.

---

## Find the method index

Every client-to-server method is identified by an integer (the **method index** for entity methods, or a top-level message id for protocol messages). The dispatch tables in [`docs/protocol/`](../protocol/) catalogue what's known:

- [`client-method-dispatch-table.md`](../protocol/client-method-dispatch-table.md) — the client receives; what the server sends.
- [`sgwplayer-base-method-dispatch-table.md`](../protocol/sgwplayer-base-method-dispatch-table.md) — base-side methods on `SGWPlayer`.
- [`cell-method-dispatch-table.md`](../protocol/cell-method-dispatch-table.md) — cell-side methods.
- [`message-catalog.md`](../protocol/message-catalog.md) — the full 420-message catalog with IDs, directions, and payload shapes.

The canonical *source* of the method index is the entity definition in [`entities/defs/`](../../entities/defs/) — that XML is what the client and server both bind against. If you're adding a brand-new method, you'll need to add it to the relevant `.def` file too; consult [`docs/engine/entity-def-guide.md`](../engine/entity-def-guide.md).

---

## The dispatcher pattern

Take the base-side dispatcher as the canonical shape. From [`crates/services/src/base/dispatch.rs`](../../crates/services/src/base/dispatch.rs):

```rust
pub(crate) mod sgw_player_base {
    pub const CHAT_JOIN: u8 = 0xC0;
    pub const CHAT_LEAVE: u8 = 0xC1;
    pub const SEND_PLAYER_COMMUNICATION: u8 = 0xC2;
    // ... add your new constant here, named after the .def method
    pub const YOUR_NEW_METHOD: u8 = 0xD9;
}

#[tracing::instrument(
    name = "base.player_method",
    level = "debug",
    skip_all,
    fields(peer = %addr, msg_id, payload_len = payload.len()),
)]
pub(crate) async fn dispatch_sgw_player_base_method(
    msg_id: u8,
    payload: &[u8],
    /* ... */
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match msg_id {
        sgw_player_base::CHAT_JOIN => { /* existing */ }
        sgw_player_base::YOUR_NEW_METHOD => {
            handle_your_new_method(payload, /* deps */).await?;
        }
        _ => {
            warn!(msg_id, "unhandled base method");
        }
    }
    Ok(())
}
```

Three pieces:

1. **Constant** — name it after the `.def` method exactly. Future readers will grep for the constant to find the handler.
2. **Match arm** — keep it thin. Decode arguments and dispatch to a free function. Inline logic gets messy fast.
3. **Handler function** — does the work, sends responses via the channel, returns errors.

---

## Decode the payload

Mercury payloads are byte-exact: every field has a fixed wire format derived from the `.def`. The decode utilities live in [`crates/mercury/`](../../crates/mercury/) and the per-domain helpers in [`crates/services/src/mercury/`](../../crates/services/src/mercury/).

Common decoders:

```rust
use crate::mercury::{read_u8, read_u16, read_u32, read_i32, read_wstring};

fn decode_your_method(payload: &[u8]) -> Result<YourArgs, DecodeError> {
    let mut cursor = std::io::Cursor::new(payload);
    let arg1 = read_i32(&mut cursor)?;
    let arg2 = read_wstring(&mut cursor)?;  // length-prefixed UTF-16
    Ok(YourArgs { arg1, arg2 })
}
```

**The wire format must match the `.def`.** A wrong type, wrong endianness, or wrong length-prefix encoding silently desyncs the channel. There is no graceful "wrong format" — the client just stops responding.

If you're guessing the format because no doc exists yet, **stop and verify**. Either:

- Decompile the client handler in Ghidra to read the exact serialization shape, or
- Find a working server-to-client message that uses the same arg types and copy that pattern.

See [`docs/guides/reading-decompiled-code.md`](reading-decompiled-code.md) for the Ghidra workflow.

---

## Do the work

Inside the handler, you'll have access to the dependencies the dispatcher passes through:

- `transport` — for sending replies back to the client.
- `entity_manager` — for looking up the player or other entities.
- `connected` / `entity_to_addr` — for finding sessions.
- `cell_tx` — for cross-service messages (base → cell).

The pattern most handlers follow:

1. **Authorise** — does this player have the right to do this? (Access level, ownership, geometry / range checks.)
2. **Validate** — are the arguments sane? Bound-check anything that came from the client.
3. **Mutate** — do the state change. Persist if needed.
4. **Reply / fan out** — send the response to the originating client, and broadcast to other clients via the witness fanout if other players need to see the effect.

The Server-Authority Enforcer agent (configured in `.claude/agents/server-authority-enforcer.md`) is the canonical voice for "what if the client lies?" — invoke it before merging any handler that mutates state.

---

## Send a reply

Replies use the same Mercury transport. The encoders mirror the decoders:

```rust
use crate::mercury::{write_u8, write_i32, write_wstring};

fn build_reply(/* ... */) -> Vec<u8> {
    let mut buf = Vec::new();
    write_u8(&mut buf, MSG_ID);
    write_i32(&mut buf, status);
    write_wstring(&mut buf, &message);
    buf
}

transport.send_to(addr, &build_reply(/* ... */)).await?;
```

For server-to-client method dispatch (calling a method on a remote entity), use the witness-fanout helper rather than hand-rolling the broadcast — search the codebase for `dispatch_entity_method` or check [`docs/protocol/client-method-dispatch-table.md`](../protocol/client-method-dispatch-table.md) for the framing.

---

## Add tests

Per [`TESTING.md`](../../TESTING.md), pick the right test type. The most common combinations for a new handler:

| What you wrote | Tests you need |
|---|---|
| New decode path | **Wire-format test** — byte-exact input → expected decoded struct. Lives alongside the handler. |
| New DB mutation | **Live-DB regression guard** — fails when the fix is reverted. See [`docs/architecture/integration-test-infra.md`](../architecture/integration-test-infra.md) for the `require_db_or_skip!` pattern. |
| New cross-service flow | **Smoke test** or **wireclient pcap-replay** — covers base → cell hop. |
| Pure logic without DB or wire side effects | **Unit test** — colocated in the module. |

Don't skip a layer because "the next one will catch it." That's the bug shape TESTING.md was written to prevent.

---

## Update the docs

Per the CLAUDE.md doc-update map, every PR that touches a method index, dispatch table, or wire format updates:

- [`docs/protocol/client-method-dispatch-table.md`](../protocol/client-method-dispatch-table.md) — for client-direction methods.
- [`docs/protocol/message-catalog.md`](../protocol/message-catalog.md) — add the new entry.
- The relevant section README under [`docs/protocol/`](../protocol/).
- The canonical entity definitions in [`entities/defs/`](../../entities/defs/) if you added a new method.
- `crates/services/src/mercury/method_idx.rs` — the constants module that pins method indices.

The maintainer reviewing your PR will check.

---

## A worked example

The cleanest recent worked example is the `CANCEL_LOG_OFF` handler (msg id `0xD7`):

- Constant: `crates/services/src/base/dispatch.rs:29`
- Dispatcher arm: nearby in the same file.
- Handler: same file, free function.
- Decode: zero-arg method — just an ack.
- Tests: unit test for the dispatch path; live-DB guard for the session-state cancellation.

Grep for `CANCEL_LOG_OFF` to read the full slice.

---

## When something doesn't work

- **Client connects then disconnects** — wire format almost certainly doesn't match. Compare with the `.def` field-by-field.
- **Handler fires but no reply seen** — check the `transport.send_to` invocation and the AoI / witness list. The witness-fanout helper exists for a reason; bypassing it loses messages.
- **Tests pass locally but client behaviour is wrong** — your test asserts what the *server* does; you may not have tested what the *client* actually receives. The `cimmeria-wireclient` replay tests are the way to close that gap.

See [`troubleshooting.md`](../troubleshooting.md) for the broader catalog.

---

## See also

- [`reading-decompiled-code.md`](reading-decompiled-code.md) — when you need to verify the wire format against the binary.
- [`entity-def-guide.md`](entity-def-guide.md) — how to add a new method to a `.def` file.
- [`../../TESTING.md`](../../TESTING.md) — picker for which test type to use.
- [`../architecture/service-architecture.md`](../architecture/service-architecture.md) — the three-process topology.
- [`extend-the-content-engine.md`](extend-the-content-engine.md) — for content-driven event handling rather than a new wire-protocol method.
