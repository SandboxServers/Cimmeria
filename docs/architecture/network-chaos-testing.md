# ADR: Network chaos testing apparatus

> **Last updated**: 2026-05-24
> **Audience**: Engineers writing Mercury-protocol regression guards,
> services-layer integration tests, or any code that needs to validate
> behavior under non-trivial network conditions
> **Type**: Architecture decision record
> **Owner**: Network / test-infra
> **Tracking**: issue #355.

## Status

**Accepted** — implemented across three layers:

- **L1** (Channel-level scenarios) — chaos primitives added to
  `NetworkPolicy` + 8 scenario tests + lomiada pcap replay.
- **L2** (Lossy socket wrapper) — `BidirectionalTransport` trait +
  `LossyTransport` with seeded RNG + LAN/Domestic/Transatlantic/Mobile
  presets + services-layer integration tests.
- **L3** (Pcap replay) — `PcapReplay` loader with key-file support;
  validates against the lomiada fixture in
  `debug/lomiada-broke-in-hallway02/`.

## TL;DR

- `cimmeria_mercury::test_harness::policy::NetworkPolicy` grows three
  chaos primitives:
  - `drop_probability: Option<DropProbability>` — seeded ChaCha20
    probabilistic drop, bitwise-deterministic across platforms.
  - `duplicate_next_count: u32` — one-shot N-extra-copies (distinct
    from the cyclic `duplicate_every`).
  - `reorder_buffer_size: u32` — buffer N packets then flush in
    reverse arrival order (generalises `reorder_pairs`).
- `cimmeria_mercury::transport::BidirectionalTransport` extends the
  send-only `Transport` trait with `recv_from`. Production uses
  `UdpTransport` (which implements both); chaos tests use
  `LossyTransport`.
- `cimmeria_mercury::lossy_transport::LossyTransport` wraps any
  `BidirectionalTransport` and applies seeded filters to both send
  and recv with `LossyProfile` presets (`Lan`, `Domestic`,
  `Transatlantic`, `Mobile`).
- `cimmeria_mercury::test_harness::pcap_replay::PcapReplay` loads
  `.pcap` files, parses Ethernet+IPv4+UDP, decrypts via a saved
  session key, and yields ordered (direction, plaintext) events for
  replay against a Channel.
- 8 chaos scenarios in `crates/mercury/src/test_harness/tests/chaos/`
  cover the lomiada gap, TX-window overflow, asymmetric ack loss,
  burst loss, reorder-in-RX-window, duplicate flood, sustained 5%
  probabilistic loss, and the BSF defeat burst.
- 1 lomiada pcap replay test loads the real fixture and asserts
  ≥500 decryptable packets + ≥95% Mercury-parse success rate.
- 3 services-layer integration tests pin the `LossyTransport` +
  `run_connect_loop` seam.

## Context

`debug/lomiada-broke-in-hallway02/` is the source of truth for the
class of bug this apparatus catches. A real player connected
Germany → US, ~6 minutes of clean play, then a single transatlantic
UDP drop of server-sent packet `#1148`. The client buffered 210
subsequent packets above the gap, the server's TX window filled,
the deferred-best-effort path swallowed the rest, and 60s later the
inactivity timer killed the session.

The failure has six discrete protocol-level steps. Pre-#308 +
pre-#354, no test exercised any of them; we only caught it when a
real player surfaced it. Going forward, every protocol-level
regression must be reproducible offline.

### What the loopback harness (PR #370) already delivered

Most of the issue's "L1 infrastructure" arrived ahead of this work
via PR #370's loopback harness:

- `LoopbackSession` / `LoopbackPeer` (= ChannelHarness)
- `Clock` trait + `TestClock` (= MockClock)
- `NetworkPolicy` with drop_next, drop_at_send_count, latency,
  reorder_pairs, duplicate_every
- 22+ paired-channel tests across 7 categories

This ADR documents the **remaining gap** — the named chaos
scenarios, the new probabilistic / N-times / multi-reorder
primitives, the lossy-socket wrapper, and the pcap replay.

## Decision

### Three-layer apparatus

| Layer | Surface | Fidelity | Use when |
|---|---|---|---|
| **L1 — `NetworkPolicy` + `LoopbackSession`** | Two `Channel`s on real loopback sockets with policy filters | High (deterministic, sub-ms RTT) | Asserting Mercury protocol behavior under controlled drop / reorder / duplicate |
| **L2 — `LossyTransport` + `BidirectionalTransport`** | Wraps any `BidirectionalTransport`; production `UdpTransport` runs inside it | Medium-high (real socket, real recv loop) | Services-layer integration: spinning up handlers against simulated wire conditions |
| **L3 — `PcapReplay`** | Loads + decrypts a real captured session, feeds events into a synthetic `Channel` | Maximum (real captured traffic) | Lomiada-class regression guards; investigating production captures |

### Chaos primitives on `NetworkPolicy`

```rust
pub struct NetworkPolicy {
    // Pre-existing primitives (PR #370):
    pub drop_next: NetworkDirection<u32>,
    pub drop_at_send_count: NetworkDirection<Option<u32>>,
    pub latency: NetworkDirection<Duration>,
    pub reorder_pairs: NetworkDirection<bool>,
    pub duplicate_every: NetworkDirection<Option<u32>>,

    // Issue #355 additions:
    pub drop_probability: NetworkDirection<Option<DropProbability>>,
    pub reorder_buffer_size: NetworkDirection<u32>,
    pub duplicate_next_count: NetworkDirection<u32>,
    pub rng_seed: Option<u64>,
    // ... plus internal counters (drop_count, send_count, ...)
}
```

Probabilistic drop uses `(numerator, denominator)` integer ratios
(`DropProbability::pct(5)`) rather than `f32` so the comparison is
bitwise-identical across platforms. The seeded `ChaCha20Rng`
guarantees the same drop pattern across runs.

### `BidirectionalTransport` (recv-side seam)

The send-only `Transport` trait (PR #358) covered handler-side
fan-out. The recv side stayed concrete `UdpSocket` because
handlers don't read sockets — only the connect-loop does. For
chaos integration tests we need to intercept on recv too, so:

```rust
#[async_trait]
pub trait BidirectionalTransport: Transport {
    async fn recv_from(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)>;
}
```

`UdpTransport` implements both. `run_connect_loop` takes
`Arc<dyn BidirectionalTransport>` and projects to
`Arc<dyn Transport>` for handler hand-off. Production call sites
in `service.rs` migrate from `Arc<UdpSocket>` to
`Arc<dyn BidirectionalTransport>` with no behavioral change.

### `LossyTransport` wrapper

Wraps an inner `BidirectionalTransport` with independent
`LossyConfig` per direction (asymmetric loss support — required
for the lomiada-style "outbound packets fine, inbound acks lost"
scenario). RNG is per-direction and seeded.

`LossyProfile` presets:

| Profile | Latency | Loss | Duplicate |
|---|---|---|---|
| `Lan` | 0.5 ms | 0% | 0% |
| `Domestic` | 15 ms | 0.1% | 0% |
| `Transatlantic` | 60 ms | 0.5% | 0% |
| `Mobile` | 80 ms | 1% | 0.2% |

### `PcapReplay`

Loads a pcap file alongside a hex-encoded session key
(`debug/<session>/*-keys.txt` format). Parses Ethernet → IPv4 →
UDP via `etherparse`, decrypts each payload via
`MercuryEncryption::from_session_key`, and yields ordered
`PcapEvent` objects tagged with capture direction.

The lomiada test asserts ≥500 decryptable packets and ≥95%
Mercury-parse success rate on the real fixture. Stronger
assertions (replay against a fresh Channel with exact recovery
tick count) need a `Channel::with_key` constructor that bypasses
the handshake — scoped for a follow-up that also unblocks the
content-aware wireclient layer.

## Alternatives considered

### Drop probability as `f32`

Rejected — floating-point comparisons differ across platforms (FMA
fusion, rounding modes). Integer ratio
`(numerator, denominator)` + `ChaCha20Rng::random_range` is
bitwise-identical everywhere.

### Wrap `UdpSocket` directly instead of adding a trait

Rejected — production code paths would either keep `UdpSocket` and
fork into a separate `LossyUdpSocket` (duplicates the recv loop)
or migrate to a `UdpSocket` newtype (large surface for marginal
gain). The trait approach lets `UdpTransport` (production) and
`LossyTransport` (test) share the same call site with zero
production overhead.

### Pcap replay via `pcap` (system libpcap binding)

Rejected — `pcap` requires native libpcap or WinPcap installed at
build/run time. The pure-Rust `pcap-file` + `etherparse` combo
needs zero system deps and parses the same format. Trade-off: no
live capture support, but the chaos apparatus is offline-only by
design.

### Reuse the issue's L2 design verbatim

The issue spec said "15+ files need their `Arc<UdpSocket>` widened
to `Arc<dyn UdpSocketLike>`." Post-PR #358 (Transport trait), only
**one** site remains (`connect_loop::run_connect_loop`). The
migration is dramatically smaller than the issue foresaw because
the architecture shifted. The current ADR documents what was
actually built rather than the original spec.

## Consequences

### Positive

- The lomiada-class regression — single-packet drop on a long
  reliable session leading to TX-window exhaustion — is now
  catchable offline. Future changes to the retransmit / overflow
  paths will fail `lomiada_single_packet_gap` and friends.
- The pcap-replay infrastructure makes any captured session a
  potential regression fixture. Drop a pcap + key file into
  `debug/<session-name>/` and write a 30-line test.
- `LossyTransport` enables services-layer integration tests
  against simulated network conditions without standing up
  remote infrastructure.
- The `Clock` trait + chaos primitives unblock #355's sibling
  initiatives — any future test that needs deterministic
  time-dependent assertion has the seam ready.

### Negative

- The `Transatlantic` profile integration test takes ~30 seconds
  of wall time because of the 60ms-per-recv latency × 500
  packets. This is the cost of integration fidelity; the test
  isn't run on every commit (could be gated to a slower CI
  profile if needed).
- L2's recv-side migration changes one production type signature
  (`Arc<UdpSocket>` → `Arc<dyn BidirectionalTransport>` in
  `run_connect_loop`). Trivial diff but it's a public-surface
  shift inside the `services` crate.
- The pcap-replay layer adds two new dependencies (`pcap-file`,
  `etherparse`) but both are dev-only.

### Out of scope (future follow-ups)

- **`Channel::with_key` constructor** — needed for full pcap
  replay (drive a fresh Channel through the captured packet
  stream and assert state-machine progression). The current
  lomiada test pins the loader + decryption pipeline; the
  state-machine replay sits in a follow-up issue.
- **Full services-layer chaos scenarios** — the issue's spec
  named `world_entry_survives_transatlantic_loss` and
  `defeat_burst_survives_5pct_loss`. Both need an in-process
  services-layer test harness (BaseService + login + world
  entry) that doesn't exist yet. The
  `chaos_lossy_transport_integration.rs` tests prove the
  `LossyTransport` + `run_connect_loop` seam; the full scenarios
  are scoped for the wireclient harness work.
- **Live capture support** — out of scope. The apparatus is
  offline-only by design (`pcap-file` parses files, not live
  interfaces).

## Confidence Level

**High** on L1 (the chaos primitives + 8 named scenarios all pass
deterministically and are seeded for reproducibility).

**High** on L2's trait extension + production migration (one site,
trivial diff, byte-identical wire behavior — confirmed by the
existing service-side tests still passing).

**Medium** on L3 (the pcap loader works on the lomiada fixture
but the assertion is loader-side, not full state-machine
replay). The follow-up `Channel::with_key` work would lift this
to High.
