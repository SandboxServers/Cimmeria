//! `ChannelBundle` — accumulator for multiple application-level messages
//! that the client processes as ONE frame after fragment reassembly.
//!
//! # Why bundling
//!
//! Today each `services` send site builds a packet body, encrypts, and puts
//! one UDP datagram on the wire per send. For bursts (28-NPC AoI on world
//! entry, world-entry payload, BSF state-change fanout, …) this consumes
//! more TX-window slots than necessary: every UDP datagram carries ~28 bytes
//! of IP+UDP overhead AND occupies one slot in the client's reliable-ack
//! tracker. A bundle collapses N messages targeting different entities into
//! one (or a few fragmented) packets, sharing one ACK piggyback and one
//! TX-window slot per fragment instead of N.
//!
//! # The "one bundle == one client frame" rule
//!
//! **Every message in a finalized bundle arrives at the client in one
//! reassembled frame.** This is the source of the bundling savings AND the
//! source of bundle's main hazard.
//!
//! When the client processes `CREATE_ENTITY(X)`, entity X enters a
//! "creation transaction" that holds until the end-of-bundle marker.
//! Subsequent same-entity messages in the same bundle hit the client's
//! HOLD-FOR-TRANSACTION path and are **silently dropped**. The existing
//! deliberate two-bundle split in
//! [`crates/services/src/base/world_entry/map_loaded.rs`] exists for
//! exactly this reason — combining `CELL_PLAYER` (which creates the player
//! entity) with same-entity `BeingAppearance` in one bundle dropped the
//! appearance message.
//!
//! ## Safe to combine in one bundle
//!
//! - `CREATE_ENTITY(A)` + `CREATE_ENTITY(B)` + … (different entity ids)
//! - Cross-entity AoI updates: `EnteredAoI(A)` + `EnteredAoI(B)` + …
//! - Property updates for an entity already created in a **prior** bundle
//!
//! ## Unsafe (silently dropped by client)
//!
//! - `CREATE_ENTITY(A)` + `BeingAppearance(A)` in the same bundle
//! - `CREATE_ENTITY(A)` + `onStatUpdate(A)` in the same bundle
//! - `CELL_PLAYER` + any same-player entity method in the same bundle
//!
//! When in doubt, split into two bundles. The TX-window savings of cross-
//! entity bundling are larger than the per-entity intra-bundle savings.
//!
//! # Caller-owned, not channel-owned
//!
//! `ChannelBundle` does NOT live on `Channel`. The caller constructs a
//! bundle, appends messages, calls [`ChannelBundle::finalize`] to get the
//! encrypted packets, sends them on the wire, and registers each emitted
//! packet with the channel's TX window via
//! `Channel::register_sent_packet`. This matches the channel's existing
//! shadow-register flow (see
//! [`crates/services/src/base/helpers.rs`]'s `shadow_register_reliable_send`).
//!
//! A per-channel auto-accumulator (every send appends to the same
//! channel-owned bundle, flushed on tick boundary) would re-introduce the
//! HOLD-FOR-TRANSACTION drop the map_loaded split exists to prevent —
//! every `CREATE_ENTITY` would collide with whatever same-entity message
//! happened to land in the same tick. Caller ownership keeps the "one
//! bundle == one client frame" decision in the caller's hands where the
//! transaction-state semantics are visible.
//!
//! # Wire format
//!
//! The bundle body uses the same byte layout as
//! [`crates/services/src/mercury/mod.rs`]'s `append_entity_method`:
//!
//! - Direct (method_index 0–60): `[(index | 0x80): u8][word_len: u16 LE]
//!   [entity_id: u32 LE][args...]`
//! - Extended (method_index ≥ 61): `[0xBD: u8][word_len: u16 LE]
//!   [entity_id: u32 LE][(index - 61): u8][args...]`
//!
//! On finalize, the body is handed to
//! [`crate::packet::build_fragmented_bundle`] which produces 1 packet for
//! bodies ≤ [`crate::packet::FRAGMENT_BODY_SIZE`] (1300 bytes) and
//! `ceil(body / 1300)` fragments otherwise. ACKs ride only the first
//! fragment.
//!
//! # Module layout
//!
//! - [`idbase`] — the per-entity sub-slot threshold (`idBase`) constants and
//!   the formula that derives them from an entity's exposed-method count.
//! - [`bundle`] — the [`ChannelBundle`] accumulator and its finalize path.

mod bundle;
mod idbase;

pub use bundle::ChannelBundle;
pub use idbase::{
    idbase_from_exposed_method_count, EXTENDED_ENCODING_MARKER, IDBASE_NPC_DEFAULT,
    IDBASE_SGW_PLAYER,
};
