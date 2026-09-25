//! Per-packet log firehoses: every row in the on-disk files, a counted
//! 1-in-N sample in SigNoz.
//!
//! Owner decision (2026-09-25, NA25): whatever reaches `logs/*.log` must also
//! reach SigNoz, with the per-packet firehoses **sampled**. A handful of TRACE
//! rows fire once per datagram or once per (witness, entity) pair per 100 ms
//! tick. Shipped one-for-one they would outweigh every other row in the
//! `cimmeria-trace` index put together.
//!
//! The split is by **target**, because `EnvFilter` can only route on target
//! and level:
//!
//! - The full row keeps its message text but moves to a `wire.firehose.*`
//!   target. The file layer that used to receive it by module path names that
//!   target explicitly (see `FILE_LAYERS` in `crates/server/src/logging/`), and
//!   every OTLP filter turns `wire.firehose` off.
//! - Every N-th occurrence emits a second row on a *different* target that the
//!   OTLP filters do export. It carries `sampled_1_in = N` and `suppressed`,
//!   the number of occurrences since the previous sample. Summing
//!   `1 + suppressed` over the sampled rows gives the true count, so SigNoz
//!   rates still add up.
//!
//! [`FIREHOSES`] is the table the server's parity test walks. Adding a
//! firehose here without a sampled counterpart, or without the file layer
//! naming its target, fails that test.

mod emit;
mod sampler;
#[cfg(test)]
mod tests;

pub use emit::{log_decrypt_ok, log_entity_moved, log_udp_in, EntityMovedRow};
pub use sampler::FirehoseSampler;

/// Prefix of every full-fidelity firehose target. The OTLP filters turn this
/// prefix off; the file layers name each target under it.
pub const FIREHOSE_TARGET_PREFIX: &str = "wire.firehose";

/// Prefix of the sampled rows that go to the `cimmeria-trace` index.
pub const SAMPLED_TARGET_PREFIX: &str = "wire.sampled";

/// `DECRYPT_OK` — plaintext hex of every decrypted inbound datagram.
pub const DECRYPT_OK_TARGET: &str = "wire.firehose.decrypt";
/// The sampled `DECRYPT_OK` row (TRACE, `cimmeria-trace`).
pub const DECRYPT_OK_SAMPLE_TARGET: &str = "wire.sampled.decrypt";

/// `UDP_IN` — raw hex of every inbound datagram, before decryption.
pub const UDP_IN_TARGET: &str = "wire.firehose.udp_in";
/// The sampled `UDP_IN` row (TRACE, `cimmeria-trace`). Carries `len` only,
/// never the hex: pre-login datagrams hold the `baseAppLogin` ticket (see
/// `docs/architecture/negative-logging-convention.md`), and the ciphertext of
/// an established session is unreadable without the key anyway —
/// `DECRYPT_OK` is the readable copy.
pub const UDP_IN_SAMPLE_TARGET: &str = "wire.sampled.udp_in";

/// `AoI: entity position update` — one row per `EntityMoved` relay, i.e. per
/// witness, per moving entity, per 100 ms AoI tick.
pub const AOI_POSITION_TARGET: &str = "wire.firehose.aoi_position";
/// The sampled position row. It is the NA00 `wire.out.avatar_update` DEBUG
/// row, which already sampled this exact call site with the full position,
/// velocity and facing; a second sample on another target would be the same
/// row twice. DEBUG, so it lands in `cimmeria-server`.
pub const AOI_POSITION_SAMPLE_TARGET: &str = "wire.out.avatar_update";

/// 1-in-N for `DECRYPT_OK`. An idle client sends ~6 packets/s and a moving
/// one ~20, so this is one hex sample every ~2.5–9 s per client — enough to
/// catch a recurring malformed bundle, while cutting a colo session of five
/// players from ~75 hex rows/s to ~1.4. Prime so that a client's periodic
/// packet mix (movement, ACK-only, heartbeat) cannot phase-lock the sample
/// onto one packet kind.
pub const DECRYPT_OK_SAMPLE_EVERY: u64 = 53;

/// 1-in-N for `UDP_IN`. Same datagram stream as `DECRYPT_OK`, so the same N.
pub const UDP_IN_SAMPLE_EVERY: u64 = 53;

/// 1-in-N for the AoI position relay. Was 100 since NA00. A counter shared by
/// every (witness, entity) pair only ever samples the pairs at multiples of
/// `gcd(N, pairs_per_tick)`: with 100 and exactly 50 pairs per tick (one
/// player watching 50 moving NPCs) every sample was the SAME pair. 101 is
/// prime, so every pair gets sampled whenever fewer than 101 are live.
pub const AOI_POSITION_SAMPLE_EVERY: u64 = 101;

pub(crate) static DECRYPT_OK_SAMPLER: FirehoseSampler =
    FirehoseSampler::new(DECRYPT_OK_SAMPLE_EVERY);
pub(crate) static UDP_IN_SAMPLER: FirehoseSampler = FirehoseSampler::new(UDP_IN_SAMPLE_EVERY);
pub(crate) static AOI_POSITION_SAMPLER: FirehoseSampler =
    FirehoseSampler::new(AOI_POSITION_SAMPLE_EVERY);

/// One firehose and the sampled row that stands in for it in SigNoz.
#[derive(Debug, Clone, Copy)]
pub struct Firehose {
    /// Full-fidelity target — files only.
    pub full_target: &'static str,
    /// Sampled target — exported.
    pub sample_target: &'static str,
    /// Level the sampled row is emitted at.
    pub sample_level: tracing::Level,
    /// 1-in-N.
    pub every: u64,
}

/// Every sampled firehose. The server's parity test asserts, for each, that
/// the full target reaches a file layer and NO OTLP layer, and that the
/// sample target reaches an OTLP layer.
pub const FIREHOSES: &[Firehose] = &[
    Firehose {
        full_target: DECRYPT_OK_TARGET,
        sample_target: DECRYPT_OK_SAMPLE_TARGET,
        sample_level: tracing::Level::TRACE,
        every: DECRYPT_OK_SAMPLE_EVERY,
    },
    Firehose {
        full_target: UDP_IN_TARGET,
        sample_target: UDP_IN_SAMPLE_TARGET,
        sample_level: tracing::Level::TRACE,
        every: UDP_IN_SAMPLE_EVERY,
    },
    Firehose {
        full_target: AOI_POSITION_TARGET,
        sample_target: AOI_POSITION_SAMPLE_TARGET,
        sample_level: tracing::Level::DEBUG,
        every: AOI_POSITION_SAMPLE_EVERY,
    },
];
