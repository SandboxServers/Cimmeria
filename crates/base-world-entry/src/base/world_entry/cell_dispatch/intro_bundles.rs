//! Deferred-AoI introduction packets for one lifecycle segment: phase 1
//! (`CREATE_ENTITY` and `UPDATE_AVATAR` per entity), then phase 2 (the
//! `createOnClient()` cascade). Ordinary flushes use two cross-entity bundles.
//! First-login cinematic release sends each phase-1 message separately to
//! keep the client's stale request cursor from dropping the tail.
//!
//! [`Phase1Shape`] controls the ordinary flush shape. Its default is one
//! phase-1 bundle holding every record; the lab chaos flush modes
//! (CH-07, `flush_chaos.rs`) cut the first bundle short and put filler
//! messages ahead of its first record.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::channel_bundle::ChannelBundle;
use cimmeria_mercury::transport::Transport;

use super::super::super::helpers::{
    send_bundle_to_witness_reliable_kind, BundleManifest, BundleSendOutcome,
};
use super::super::super::ConnectedClientState;

/// One entity's message in a phase bundle: `(entity_id, raw message bytes)`.
pub(super) type IntroRecord = (u32, Vec<u8>);

/// How the phase-1 records are cut into bundles. The default sends them all
/// in one bundle with nothing ahead of them.
#[derive(Debug, Default, Clone, PartialEq)]
pub(super) struct Phase1Shape {
    /// Whole messages placed ahead of the first record of the first phase-1
    /// bundle. Each is logged in the bundle manifest with entity id 0.
    pub(super) filler: Vec<Vec<u8>>,
    /// Caps the first phase-1 bundle at this many records; the rest follow
    /// in a second phase-1 bundle, still ahead of phase 2.
    pub(super) first_bundle_records: Option<usize>,
}

#[derive(Debug, Clone, Copy)]
enum Phase {
    CreateBase,
    Cascade,
}

/// Bundle-path analogue of `aoi::log_create_emit` — emits the success
/// (`aoi.create_emit`, DEBUG) or failure (`aoi.create_send_failed`, WARN)
/// seam for one introduction bundle. `entered` is the record count folded
/// into the bundle; `phase` distinguishes the phase-1 (`"create_base"`) and
/// phase-2 (`"cascade"`) bundles. Per-entity ids aren't available here —
/// the bundle carries N entities — so the seam reports the aggregate
/// count instead.
fn log_bundle_emit(
    witness_id: u32,
    witness_name: Option<&'static str>,
    entered: usize,
    phase: &'static str,
    outcome: BundleSendOutcome,
) {
    match outcome {
        BundleSendOutcome::Sent {
            base_seq,
            packets,
            bytes,
            ..
        } => {
            tracing::debug!(
                target: "aoi.create_emit",
                event = "create_emit",
                witness_id,
                witness_name,
                entered,
                phase,
                addr_resolved = true,
                bytes,
                seq = base_seq,
                packets,
                "AoI create emit: {phase} bundle ({entered} NPC) delivered to witness"
            );
        }
        // Empty bundle is a benign no-op (the caller already gates on
        // `is_empty()`), so it never reaches here in practice; treat it as
        // non-failure to avoid a spurious WARN if that gate ever changes.
        BundleSendOutcome::Empty => {}
        failed => {
            tracing::warn!(
                target: "aoi.create_send_failed",
                event = "create_send_failed",
                witness_id,
                witness_name,
                entered,
                phase,
                addr_resolved = failed.addr_resolved(),
                reason = failed.failure_reason().unwrap_or("unknown"),
                "AoI create emit: {phase} bundle ({entered} NPC) NOT delivered to witness -- \
                 entities may be invisible until relog"
            );
        }
    }
}

/// Send one segment's phase-1 records (shaped by `shape`), then its phase-2
/// records. Empty phases send nothing; filler never goes out on its own.
#[allow(clippy::too_many_arguments)]
pub(super) async fn send_intro_bundles(
    witness_id: u32,
    witness_name: Option<&'static str>,
    mut phase1: Vec<IntroRecord>,
    phase2: Vec<IntroRecord>,
    shape: &Phase1Shape,
    tail_safe_phase1: bool,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    if tail_safe_phase1 {
        // One IntroRecord is TWO Mercury messages. In the first-login hold,
        // put each message in its own reliable packet so the client's stale
        // next-request offset cannot collide with a later message cursor.
        // The create packet ends its transaction before that entity's avatar
        // update, and all updates precede the property cascade below.
        for (entity_id, body) in &phase1 {
            let create_len = body
                .get(1..3)
                .map(|len| usize::from(u16::from_le_bytes([len[0], len[1]])) + 3);
            let Some(create_len) = create_len
                // This composer emits precisely the 11-byte create and
                // 26-byte avatar update. If that contract changes, keep the
                // record intact and flag the wire assumption for review.
                .filter(|&len| {
                    len == 11
                        && body.len() == 37
                        && body.first() == Some(&0x09)
                        && body.get(len) == Some(&0x10)
                })
            else {
                tracing::error!(
                    witness_id,
                    entity_id,
                    body_len = body.len(),
                    "AoI tail-safe flush: unexpected create/update body; sending intact"
                );
                send_one(
                    witness_id,
                    witness_name,
                    Phase::CreateBase,
                    &[],
                    &[(*entity_id, body.clone())],
                    transport,
                    connected,
                    entity_to_addr,
                )
                .await;
                continue;
            };
            send_one(
                witness_id,
                witness_name,
                Phase::CreateBase,
                &[],
                &[(*entity_id, body[..create_len].to_vec())],
                transport,
                connected,
                entity_to_addr,
            )
            .await;
            send_one(
                witness_id,
                witness_name,
                Phase::CreateBase,
                &[],
                &[(*entity_id, body[create_len..].to_vec())],
                transport,
                connected,
                entity_to_addr,
            )
            .await;
        }
        if !phase2.is_empty() {
            send_one(
                witness_id,
                witness_name,
                Phase::Cascade,
                &[],
                &phase2,
                transport,
                connected,
                entity_to_addr,
            )
            .await;
        }
        return;
    }
    let entered = phase1.len();
    let cut = shape
        .first_bundle_records
        .map_or(entered, |n| n.min(entered));
    let rest = phase1.split_off(cut);
    let bundles = [
        (Phase::CreateBase, shape.filler.as_slice(), phase1),
        (Phase::CreateBase, &[][..], rest),
        (Phase::Cascade, &[][..], phase2),
    ];
    for (phase, filler, records) in bundles {
        if records.is_empty() {
            continue;
        }
        send_one(
            witness_id,
            witness_name,
            phase,
            filler,
            &records,
            transport,
            connected,
            entity_to_addr,
        )
        .await;
    }
}

#[allow(clippy::too_many_arguments)]
async fn send_one(
    witness_id: u32,
    witness_name: Option<&'static str>,
    phase: Phase,
    filler: &[Vec<u8>],
    records: &[IntroRecord],
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let mut bundle = ChannelBundle::new(true);
    // Per-message offset / id / entity (TG-AOI-09), logged on
    // `aoi.bundle_manifest` when the bundle is sent.
    let mut manifest = BundleManifest::new();
    for msg in filler {
        manifest.append_raw_message(&mut bundle, 0, msg);
    }
    for (entity_id, body) in records {
        manifest.append_raw_message(&mut bundle, *entity_id, body);
    }
    let entered = records.len();
    let (kind, phase_label) = match phase {
        Phase::CreateBase => {
            tracing::debug!(
                witness_id,
                witness_name,
                entered,
                phase1_bytes = bundle.body_len(),
                phase1_packets = bundle.estimated_packet_count(),
                "AoI flush: phase-1 bundle (CREATE_ENTITY + UPDATE_AVATAR per NPC)"
            );
            ("aoi_flush_create_base", "create_base")
        }
        Phase::Cascade => {
            tracing::debug!(
                witness_id,
                witness_name,
                entered,
                phase2_bytes = bundle.body_len(),
                phase2_packets = bundle.estimated_packet_count(),
                "AoI flush: phase-2 bundle (createOnClient() cascade per NPC)"
            );
            ("aoi_flush_cascade", "cascade")
        }
    };
    let outcome = send_bundle_to_witness_reliable_kind(
        transport,
        connected,
        entity_to_addr,
        witness_id,
        bundle,
        kind,
        manifest.entries(),
    )
    .await;
    log_bundle_emit(witness_id, witness_name, entered, phase_label, outcome);
}
