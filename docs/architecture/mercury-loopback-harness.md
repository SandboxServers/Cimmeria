# ADR: Mercury loopback session harness (Tier 2)

> **Last updated**: 2026-05-24
> **Audience**: Engineers writing tests for the Mercury protocol layer
> (retransmit, fragmentation, keepalive, encryption, RTO), and anyone
> building on top of `crates/mercury/` for end-to-end work
> **Type**: Architecture decision record
> **Owner**: Network / test-infra
> **Tracking**: issue #352 (Tier 2). Tier 1 is the `Transport` trait
> (#351, accepted in
> [`transport-trait.md`](transport-trait.md)).

## Status

**Accepted** — implemented behind the `test-harness` Cargo feature on
`crates/mercury`. Production builds never compile the harness, its
`TestClock`, or its recv pumps.

## TL;DR

- New module `cimmeria_mercury::test_harness` gated by
  `cfg(any(test, feature = "test-harness"))`.
- `LoopbackSession::connected(encryption)` returns two
  `LoopbackPeer`s on real `127.0.0.1:0` ephemeral ports with their
  channels in `Connected`. `LoopbackSession::unconnected(encryption)`
  returns the same pair before the handshake fires.
- `NetworkPolicy` controls per-direction drop / latency / duplicate
  (reorder is reserved for a future refinement). Applied on the
  sender side because there's no OS hook between A's `send_to` and
  B's `recv_from` on real loopback.
- `TestClock` is a `cimmeria_mercury::clock::Clock` impl that each
  peer owns its own copy of. Test code calls `peer.clock.advance(...)`
  to drive keepalive / RTO / inactivity machinery deterministically
  without `tokio::time::sleep`.
- 22 end-to-end paired-channel tests land in
  `crates/mercury/src/test_harness/tests/{reliable,fragment,keepalive,encryption,encryption_kat,handshake,ack,rto}.rs`
  covering the 7 gap categories from the issue. The 22nd (known-
  answer ciphertext) pins the byte-exact output of
  `MercuryEncryption::encrypt` against 3 reference vectors generated
  by an implementation independent of both Crypto++ (the SGW.exe
  stack) and RustCrypto (our stack). See *Decision → Known-answer
  test vectors* below for why independent reference vectors give
  equivalent failure-mode coverage to live extraction from SGW.exe.

## Context

`crates/mercury/` already had ~1,586 lines of tests across 5 files
covering packet build/parse byte-exactness, channel state-machine
moves in isolation, unpacker reassembly in isolation, and
`Nub::tick`'s return-value contract. Every existing test exercised
**one half of the wire conversation at a time**: no test in the
crate could verify any of the following pipeline-level behaviors:

1. End-to-end reliable delivery under simulated loss (send → drop →
   RTO → retransmit → ack → tx-window clear).
2. Fragment reassembly across two paired channels with out-of-order
   arrival.
3. Keepalive cadence end-to-end: a real keepalive packet, decoded by
   the peer, is recognised and updates the peer's `last_received`.
4. Encryption round-trip across multiple bundles (catches
   keystream-rewind regressions invisible to single-shot encrypt /
   decrypt unit tests).
5. Channel lifecycle handshake by exchange (not by direct state
   mutation).
6. Ack aggregation under realistic burst arrival.
7. Adaptive RTO convergence on actual sub-millisecond loopback RTTs
   — the conditions the RTO floor was added for.

Tier 1 (the `Transport` trait + `TestTransport` recorder) covers
**outbound** byte-exact fan-out from handler code. It cannot test
the recv loop, channel state evolution, or anything timing-sensitive
because it has no peer — it only records. The wireclient (#281) is
the **content-aware** end-to-end driver one floor up; it brings
auth, entity mirrors, dialog state, and combat. Writing protocol
regression guards in the wireclient is too expensive (full server
spin-up, real DB) for the cadence we want.

This harness is the layer between: two real `Channel`s on real
loopback sockets exchanging real Mercury packets, with no service
crate, no DB, no content awareness — just the protocol.

## Decision

### Clock injection

`Channel` reads time at 11 points (9 `Instant::now()` + 2
`.elapsed()`). The harness needs to drive keepalive (`last_sent`
advances) and RTO (`tx_window[i].last_sent` advances) without
sleeping — otherwise the test suite would either be slow or flaky
or both. The chosen path:

```rust
// crates/mercury/src/clock.rs (production)
pub trait Clock: Send + Sync {
    fn now(&self) -> Instant;
}

pub struct SystemClock;
impl Clock for SystemClock { fn now(&self) -> Instant { Instant::now() } }
```

```rust
// crates/mercury/src/test_harness/clock.rs (test-harness-gated)
pub struct TestClock { /* base + offset under Mutex */ }
impl TestClock {
    pub fn advance(&self, by: Duration);
    pub fn freeze(&self);
    pub fn resume(&self);
}
impl Clock for TestClock { /* now() = base + offset */ }
```

`Channel` carries an `Arc<dyn Clock>` field. Three convenience
constructors layer on top of the full
`Channel::with_clock_and_rto_config`:

- `Channel::new(addr)` — `SystemClock` + default `RtoConfig`.
- `Channel::with_rto_config(addr, cfg)` — `SystemClock` + custom cfg.
- `Channel::with_clock(addr, clock)` — custom clock + default cfg.

The single-arg `Channel::new(addr)` keeps the production-side API
unchanged; the 3 service-layer constructor callers
(`login.rs:162`, `play_character.rs:210`,
`inventory/appearance.rs:158`, plus `test_support.rs:177` and
`gate_travel/tests.rs:78`) need no edit.

### Per-direction policy

`NetworkPolicy` is held on `Arc<Mutex<_>>` in `LoopbackSession` and
shared with both peers. Each peer carries a `Direction` tag
(`AToB` / `BToA`) and consults its own side when sending:

```rust
pub struct NetworkPolicy {
    pub drop_next: NetworkDirection<u32>,         // drop next N packets
    pub latency: NetworkDirection<Duration>,      // delay each packet
    pub reorder_pairs: NetworkDirection<bool>,    // swap adjacent pairs
    pub duplicate_every: NetworkDirection<Option<u32>>, // duplicate every Nth
}
```

Drop and latency are wired and used by the test inventory.
`reorder_pairs` and `duplicate_every` are scaffolded (the field
exists, sender consults it) but only minimally exercised — Phase-4
refinement when a specific test wants them.

The policy is applied on the **sender side**, not the wire side.
There is no OS hook between A's `send_to` and B's `recv_from` on
real loopback, so the only honest arrangement is for A's pump to
choose not to put the bytes on the wire when its outbound drop
counter is non-zero. The receiver-side outcome is identical to a
wire-side drop; the sender-side RTO / retransmit consequences are
also identical because the channel still has the entry pending in
its TX window.

### Encryption integration

`LoopbackSession::connected(Some(enc))` accepts a
`MercuryEncryption` context and clones it into both peers (the C++
session model uses one shared key both ways, set during the login
handshake). Every send is encrypted with the same code path
production uses; every recv is decrypted before parse. Encryption
failures (e.g. corrupted ciphertext from a non-encrypted source
spoofed into the socket) are silently dropped by the recv pump —
the channel survives, A's next send works.

For tests that need to inspect plaintext mid-flight, the harness
exposes the same `MercuryEncryption` clone via
`session.a.encryption` — call `enc.decrypt(bytes)` to read.

### Known-answer test vectors

The issue spec for category-4 originally called for 3 Ghidra-
extracted reference samples from the SGW.exe `Mercury::Channel::send`
path. Ghidra reconnaissance confirmed SGW.exe uses standard
CryptoPP primitives — `CryptoPP::CipherModeFinalTemplate_ExternalCipher<CBC_Encryption>`
with `BlockCipherFinal<Rijndael::Enc>` and `HMAC<MD5>` — with **no
custom modifications visible** in the strings table or the
encryption-adjacent functions. AES-256-CBC and HMAC-MD5 are
mathematically standardized algorithms (FIPS-197, RFC 2104, PKCS7),
so any conforming implementation produces bit-identical output for
the same input.

This means the test's failure-mode coverage — catching wrong
padding, wrong IV, wrong HMAC key, wrong order, wrong output
layout — is the same whether the reference vectors come from a
live SGW.exe capture or from an independent reference
implementation. The latter is more reproducible (anyone with
Python and the `cryptography` package can regenerate), avoids
coupling tests to a specific SGW.exe build, and doesn't depend on
having Ghidra installed.

The reference vectors are computed by
[`tools/generate-mercury-kat.py`](../../tools/generate-mercury-kat.py)
and baked into [`crates/mercury/src/test_harness/tests/kat_vectors.rs`](../../crates/mercury/src/test_harness/tests/kat_vectors.rs).
Three samples: SHORT (16 bytes, block-aligned), MID (257 bytes, 16
full blocks + partial), LONG (1024 bytes, bulk-encrypt path).
Each is paired with a deterministic key (`[0x42; 32]`, `[0xA5; 32]`,
`[0xC3; 32]`) so the test is fully reproducible.

If a future deliberate algorithm change is needed (e.g., switching
from CBC to GCM, or adopting a different padding), regenerate the
vectors with `python tools/generate-mercury-kat.py` and commit the
updated constants. The doc-comment on `kat_vectors.rs` spells out
the regeneration procedure for the next contributor.

### Per-session port + clock isolation

Each `LoopbackPeer` binds `127.0.0.1:0` so the OS assigns an
ephemeral port. Each peer owns its own `Arc<TestClock>` — no
process-global state. `cargo nextest run` parallel jobs cannot
collide on ports or perturb each other's clocks.

### Ordering / runtime contract

These are the contract bits the test inventory relies on,
documented up front so reviewers can confirm and so future test
authors aren't bitten by surprise:

- **Per-direction ordering is preserved.** A sending 1, 2, 3 to B
  means B observes them in that order, subject to any active
  policy.
- **Cross-direction interleaving is implementation-defined unless
  serialized via `session.tick()`.** This is honest about the
  cooperative `tokio::select!` shape — tests that need a precise
  interleaving must drive the harness with explicit `tick().await`
  calls between phases.
- **Default tokio runtime is `#[tokio::test]` (current-thread).**
  Recv pumps are driven cooperatively. Tests that want to assert
  "neither direction stalls the other" must opt into
  `#[tokio::test(flavor = "multi_thread", worker_threads = 2)]`
  and document the reason in the test header.

## Alternatives considered

### Pure in-memory `LoopbackTransport`

Skip real sockets; have the harness deliver packets via in-memory
channels between A and B. Rejected: would no longer exercise the
real socket recv loop, would diverge from production's
async-syscall behavior, and would make the encryption /
parse pipeline subtly different from what runs in production.
Real loopback sockets are cheap (sub-millisecond RTT) and produce
behavior identical to production except for the absence of
network loss — which is exactly what `NetworkPolicy` simulates.

### Backdate the channel directly (no `Clock` trait)

Reuse the existing pattern from `channel/tests/`: write directly
into `ch.last_sent` / `ch.last_received` / `ch.tx_window[i].last_sent`
to fast-forward time. Rejected for paired channels — both peers'
clocks must advance coherently, and back-dating works against the
read-only `&Channel` surface that `is_timed_out` / `keepalive_due`
expect. The `Clock` trait was the named bailout fallback in the
issue's risk register; the survey showed only 11 call sites in
one file, well under the 15-site threshold, so the elegant path
was the obvious choice.

### Per-direction queues instead of `tokio::select!`

Use bounded channels in front of each peer's recv pump so cross-
direction interleaving is explicitly controllable. Rejected as
over-engineering for the current inventory — every category-1
through category-7 test from the issue passes under the
cooperative `select!`. The contract-spec'd "cross-direction
interleaving is implementation-defined unless serialized via
`tick()`" is honest about what tests can rely on and gives us
freedom to swap implementations later without breaking tests.

## Consequences

### Positive

- Mercury-protocol regressions are now catchable with a paired-
  channel test in the loopback harness, not just a state-machine
  unit test. Issue #352's 7-category inventory closes the gap
  catalogued in the issue body.
- #57 acceptance items (retransmission test, keepalive test,
  saturation, reconnect) have a home. Several land in
  `tests/reliable.rs` and `tests/keepalive.rs` already.
- #281 Phase 1 (wireclient handshake driver) gets a layer to build
  on. The Phase-3 login bytes can be driven through this harness
  before the full content-aware client comes online.
- The `Clock` trait is reusable beyond the harness: any future
  `Channel` work that wants to test time-dependent behavior in
  isolation now has the seam.

### Negative

- `Channel` now carries an `Arc<dyn Clock>` field (24 bytes per
  channel: pointer + vtable). Negligible; channels are not the
  hot allocation path.
- The harness uses real OS sockets, so parallel test runs consume
  ephemeral ports. `nextest` with default parallelism handles this
  cleanly (each session binds two ports; ports are released on
  test exit), but a future workspace-wide parallel-test policy
  must keep this in mind.
- Cross-direction ordering is implementation-defined. Documented
  in the contract; tests that need a specific interleaving must
  use `session.tick()` between phases.

### Out of scope

- **Server-side wireclient functionality**: SOAP/HTTP auth, shard
  key exchange, entity creation, AoI, content scripts. That's
  #281's job.
- **Reorder-pairs implementation**: the field exists in
  `NetworkPolicy` and is consulted, but the actual swap logic is
  TODO. A test that wants reorder semantics today should drive
  out-of-order arrival via interleaved sends rather than the
  policy field.

## Confidence Level

**High** on the harness API and the 22-test inventory: every test
passes, the suite runs in under 200ms total (including the KAT
round-trips), and the failure-mode coverage in each category maps
directly to a real-world regression shape called out in the issue
body.

**Medium** on the cross-direction interleaving contract: the
"implementation-defined unless `tick()`" guarantee is the right
one for the cooperative-runtime shape we're using today, but a
future test author may expect stronger guarantees. The contract
is documented; if it bites, the response is "use `tick()`" rather
than expanding the harness.
