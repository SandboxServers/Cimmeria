# ADR: `Transport` trait for the Mercury send side

> **Last updated**: 2026-05-23
> **Audience**: Engineers touching any BaseApp handler that emits wire bytes,
> or anyone writing a byte-exact fan-out test
> **Type**: Architecture decision record
> **Owner**: Network / test-infra
> **Tracking**: issue #351 (Tier 1). Recv-side harness is #352 (Tier 2).

## Status

**Accepted** — implemented for the whole `crates/services/src/base/` handler
surface. Recv side and the `crates/services/src/cell/` direct-emit path are
explicitly out of scope (see *Consequences* and *Out of scope*).

## TL;DR

- New trait `cimmeria_mercury::transport::Transport` with two methods:
  `async fn send_to(&self, bytes, addr)` and `fn local_addr(&self)`.
- Production impl `UdpTransport` wraps an `Arc<UdpSocket>`.
- Test recorder `cimmeria_mercury::test_transport::TestTransport` records every
  `(SocketAddr, Vec<u8>)` send; behind the `test-support` Cargo feature.
- Every BaseApp handler now takes `&Arc<dyn Transport>` instead of
  `&Arc<UdpSocket>`. The recv loop in `connect_loop/mod.rs` keeps the concrete
  `UdpSocket` (it is the only reader of the wire) and wraps it in a
  `UdpTransport` once for the send side.

## Context

Before this change, ~40 production files in `base/` took `&Arc<UdpSocket>`
directly. The 25 `socket.send_to(&pkt, addr).await` call sites across those
files each produce wire bytes — and **none of them were byte-asserted by a
unit test**, because there was no seam to inject a fake socket at. The only
test affordance was binding a real loopback `UdpSocket` (≈17 test files did
this purely as ceremony to satisfy a signature) and, in three files, binding a
second `receiver` socket to make a **negative** assertion via
`try_recv_from() == WouldBlock` ("no packet was sent"). There was no canonical
way to make a **positive** assertion: "exactly these N packets, in this order,
with these bytes, went to these N addrs."

That positive assertion class is the highest-value one for this codebase: it
catches witness-list amplification bugs (send to N±1 witnesses), wrong-recipient
routes (owner-only packet leaks to witnesses), stale-addr sends, and
encryption-state divergence — failure modes that today only surface in
production or in fragile real-loopback timing tests.

Separately, `crates/mercury/src/nub.rs` carried `send_to`/`recv_from` as
`todo!()` stubs (#57) because the actual I/O path lives in
`services/src/base/connect_loop/mod.rs`, leaving no clean home for the
byte-emitting layer.

## Decision

1. **Add a send-only `Transport` trait in `cimmeria-mercury`** (`transport.rs`):

   ```rust
   #[async_trait]
   pub trait Transport: Send + Sync {
       async fn send_to(&self, bytes: &[u8], addr: SocketAddr) -> io::Result<usize>;
       fn local_addr(&self) -> io::Result<SocketAddr>;
   }
   ```

   `UdpTransport` is the production impl (thin wrapper over `Arc<UdpSocket>`).

2. **Add `TestTransport`** (`test_transport.rs`, behind the `test-support`
   feature) — a `Mutex<Vec<(SocketAddr, Vec<u8>)>>` recorder with
   `drain`/`clear`/`filter_to`/`len`/`is_empty`/`total_bytes_sent`/`send_count_to`.
   It lives in mercury (next to the trait), not in `services`, so any consumer
   crate (mercury's own tests, the Tier 2 harness, future crates) can use it
   without re-implementing it. `cimmeria-services` pulls it in via a
   dev-dependency: `cimmeria-mercury = { path = "../mercury", features =
   ["test-support"] }`, and re-exports it as
   `crate::test_support::TestTransport`.

3. **Refactor every BaseApp handler signature** from `&Arc<UdpSocket>` to
   `&Arc<dyn Transport>` (and the two owned `Arc<UdpSocket>` parameters —
   `run_tick_loop`, the cell-message handler path — to `Arc<dyn Transport>`).

4. **Keep the recv loop concrete.** `run_connect_loop` still owns
   `Arc<UdpSocket>` because it is the only code that calls `recv_from`. It
   constructs one `UdpTransport` wrapping that socket and hands `&Arc<dyn
   Transport>` to every handler. `BaseService::start` does the same for the
   cell→base message handler.

5. **Resolve #57 by deletion, not implementation.** The `Nub::send_to`/
   `recv_from` `todo!()` stubs were removed and replaced with a doc-comment
   redirect: outbound I/O is `UdpTransport`'s job; the inbound decode path is
   the recv loop's; the Nub owns only pure Mercury logic (tick, channels,
   fragments).

### Why split send and recv asymmetrically

Handlers **only emit** — they never read from the socket. The single reader is
the recv loop. Putting recv behind the same trait would mean either a
do-nothing `recv_from` on `TestTransport` (dead surface) or dragging the whole
channel/decrypt/dispatch pipeline into the trait (scope creep). End-to-end
recv-side testing wants paired Nubs on loopback driving real channel state —
that is the **Tier 2 Mercury loopback harness (#352)**, a fundamentally
different shape from "record what got sent." Keeping Tier 1 send-only keeps the
refactor mechanical and the trait surface tiny.

### Why a trait object (`Arc<dyn Transport>`) over a generic (`<S: Transport>`)

Trait objects keep handler signatures readable — `&Arc<dyn Transport>` reads
the same everywhere, no generic parameter threaded through 67 signatures and
their callers. The cost is one `Box::pin` allocation per `send_to` (an
`async_trait` artifact) plus a vtable indirection. Per the measurement below
that cost is immaterial on our send path. **If profiling ever shows it hot, the
switch to `<S: Transport>` is a follow-up refactor — the call sites are
identical, only the trait/wrapper definitions change.**

### Why `async_trait`

Native `async fn` in traits (stable since Rust 1.75) is not `dyn`-compatible:
you cannot make `dyn Transport` from a trait with a native `async fn`. Since
the whole point is a trait object injected at 67 call sites, `async_trait`
(which desugars to `-> Pin<Box<dyn Future>>`) is required.

## Alternatives considered

| Alternative | Why not |
|---|---|
| Generic `<S: Transport>` on every handler | More invasive (generic propagates through every sig + caller); harder to read. Re-evaluate only if profiling shows the boxed future is hot — the migration is non-breaking for call sites. |
| Native `async fn` in trait (no `async_trait`) | Not object-safe; can't form `dyn Transport`. The trait object is the entire mechanism. |
| `TestTransport` in `cimmeria-services` under `#[cfg(test)]` | Invisible to every other crate (mercury's own tests, the Tier 2 harness, any new crate). Mercury already owns the trait; the fake belongs next to it. |
| Implement `Nub::send_to`/`recv_from` for real | Duplicates what `UdpTransport` + the recv loop already do. The stubs added no value; deletion + redirect is the honest resolution of #57. |
| Add a `Receiver` trait too (symmetric split) | Recv has exactly one caller and needs full channel state to be meaningful — that's #352's job, not a recorder fake. |

## Consequences

**Positive**

- Unlocks the **fan-out byte test** type (the 8th in `TESTING.md`): assert the
  exact ordered `(addr, bytes)` set a handler emits. This PR ships proof-of-
  concept tests for domains A (AoI witness fan-out), B (teleport position
  snap), C (reanchor burst), F (login phase 1→4 sequence); domains D–N are
  follow-ups gated on this trait.
- ~17 test files drop their loopback `bind` ceremony; the three negative-
  assertion `receiver`-socket tests become `assert!(transport.is_empty())`.
- #57 closed; the Nub no longer pretends to own I/O.

**Negative / watch-outs**

- One `Box::pin` allocation per `send_to` call (`async_trait`). See measurement.
- `TestTransport::local_addr()` returns a **synthetic** address (default
  `127.0.0.1:0`). A handler that reads `local_addr()` to build a reply addr
  must be tested with `TestTransport::with_local(addr)` set to a meaningful
  synthetic, or the assertion will be against the placeholder.
- `is_empty()`/`drain()`/etc. are inherent methods on `TestTransport`, not on
  the `dyn Transport` trait. A test that needs both to call a handler
  (`&Arc<dyn Transport>`) and to inspect afterward keeps a typed
  `Arc<TestTransport>` handle and clones it to a `Arc<dyn Transport>` for the
  call (the **two-handle** pattern used in `teleport.rs`/`character.rs`):

  ```rust
  let transport = Arc::new(TestTransport::new());
  let dyn_transport: Arc<dyn Transport> = transport.clone();
  handler(&dyn_transport).await;
  assert_eq!(transport.len(), 1);
  ```

  Without the typed handle the test can hand a handler `&Arc<dyn Transport>` but
  has no way back to `drain`/`filter_to`/etc. on the same recorder.

## Performance measurement

`async_trait` allocates a `Box::pin` per call, and `send_to` is per-packet.
The measurement question is whether that allocation + the vtable indirection
regress the send path enough to justify static dispatch.

**Assessment for this codebase:** below any threshold worth acting on.

- The BaseApp send path already does, per packet: a Mercury bundle build, an
  AES-256-CBC encrypt, an `Arc<Mutex>` lock for channel/seq state, and a real
  `UdpSocket::send_to` syscall (microseconds). A single `Box::pin` of a small
  future (one pointer + a few captured args, well under a cache line) is
  nanoseconds against that — comfortably under the issue's "≥1 extra
  allocation per send the optimizer can't elide / ≥5% packets/sec" trip wire in
  relative terms, and dwarfed in absolute terms by the encrypt + syscall.
- The hot retransmit path (`tick_sync.rs`) sends **pre-encrypted cached bytes**
  and is bounded by `RETRANSMIT_BUDGET_PER_TICK` (5) per tick per channel, so
  the boxed future there is not a throughput concern either.

**Decision:** ship `Arc<dyn Transport>`. The empirical Criterion bench
(`Arc<UdpSocket>` baseline vs `Arc<dyn Transport>`, 100k packets to a
black-hole addr, allocations via `dhat`) is deferred as a follow-up — it would
gate a *generic* migration, which is itself a follow-up that does not change
any call site. If a future profiling pass flags the boxed future, file the
generic refactor with this ADR as the dependency.

## Confidence Level

**High** for the trait shape, the send/recv split, and the `#57` resolution —
the change is mechanical, fully type-checked, and the new test type is already
exercised. **Medium** on the deferred-bench call: the reasoned analysis is
sound, but no `dhat`/Criterion numbers are attached yet; if anyone later
measures a regression, the generic-parameter escape hatch is pre-designed and
non-breaking.
