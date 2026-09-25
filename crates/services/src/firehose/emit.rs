//! The three firehose emitters. Each writes the full row on its
//! `wire.firehose.*` target and, when the sampler admits it, the sampled row.
//!
//! The sampler is a parameter so a test can count from zero; production call
//! sites pass the `static` next to the target in [`super`].

use std::net::SocketAddr;

use super::{
    FirehoseSampler, AOI_POSITION_SAMPLE_TARGET, AOI_POSITION_TARGET, DECRYPT_OK_SAMPLE_TARGET,
    DECRYPT_OK_TARGET, UDP_IN_SAMPLE_TARGET, UDP_IN_TARGET,
};
use crate::base::helpers::to_hex;

/// `DECRYPT_OK`: the plaintext of one decrypted inbound datagram.
pub fn log_decrypt_ok(sampler: &FirehoseSampler, addr: SocketAddr, plaintext: &[u8]) {
    // Message text unchanged from before NA25 so `grep DECRYPT_OK logs/base.log`
    // still works.
    tracing::trace!(target: DECRYPT_OK_TARGET, %addr, len = plaintext.len(), hex = %to_hex(plaintext), "DECRYPT_OK");
    if let Some(suppressed) = sampler.admit() {
        tracing::trace!(
            target: DECRYPT_OK_SAMPLE_TARGET,
            %addr,
            len = plaintext.len(),
            hex = %to_hex(plaintext),
            sampled_1_in = sampler.every(),
            suppressed,
            "DECRYPT_OK (sampled)"
        );
    }
}

/// `UDP_IN`: one raw inbound datagram. The sampled row has no hex — see
/// [`super::UDP_IN_SAMPLE_TARGET`].
pub fn log_udp_in(sampler: &FirehoseSampler, addr: SocketAddr, datagram: &[u8]) {
    tracing::trace!(target: UDP_IN_TARGET, %addr, len = datagram.len(), hex = %to_hex(datagram), "UDP_IN");
    if let Some(suppressed) = sampler.admit() {
        tracing::trace!(
            target: UDP_IN_SAMPLE_TARGET,
            %addr,
            len = datagram.len(),
            sampled_1_in = sampler.every(),
            suppressed,
            "UDP_IN (sampled)"
        );
    }
}

/// What one `EntityMoved` relay sent to one witness.
#[derive(Debug, Clone, Copy)]
pub struct EntityMovedRow {
    pub witness_id: u32,
    pub entity_id: u32,
    pub position: [f32; 3],
    /// `[pitch, yaw, roll]`, radians.
    pub direction: [f32; 3],
    pub velocity: [f32; 3],
    pub npc_moved_since_last: Option<bool>,
}

/// The AoI position relay. The full row is the old `AoI: entity position
/// update` TRACE; the sample is the NA00 `wire.out.avatar_update` DEBUG row,
/// which now also says how many sends it stands for.
pub fn log_entity_moved(sampler: &FirehoseSampler, row: &EntityMovedRow) {
    let EntityMovedRow {
        witness_id,
        entity_id,
        position,
        direction,
        velocity,
        npc_moved_since_last,
    } = *row;
    tracing::trace!(target: AOI_POSITION_TARGET, witness_id, entity_id, "AoI: entity position update");
    // UPDATE_AVATAR is unreliable and never reaches `wire.out`, so without
    // this there is no record of the position / facing a client was given.
    let Some(suppressed) = sampler.admit() else {
        return;
    };
    tracing::debug!(
        target: AOI_POSITION_SAMPLE_TARGET,
        witness_id,
        entity_id,
        msg_id = crate::mercury::aoi::BASEMSG_UPDATE_AVATAR_NO_ALIAS_FULL_POS_YPR,
        pos_variant = "FullPos",
        x = position[0],
        y = position[1],
        z = position[2],
        vx = velocity[0],
        vy = velocity[1],
        vz = velocity[2],
        yaw_rad = direction[1],
        yaw_byte = crate::mercury::aoi::pack_angle(direction[1]),
        pitch_byte = crate::mercury::aoi::pack_angle(direction[0]),
        // NA02: `false` with a non-zero velocity is an NPC the client
        // is animating as running while it stands still (audit S1).
        // The client animates NPC movement from velocity alone, so no
        // movement-type field is logged here.
        npc_moved_since_last,
        sampled_1_in = sampler.every(),
        suppressed,
        "UPDATE_AVATAR sent (sampled) -- position and facing as transmitted to this witness"
    );
}
