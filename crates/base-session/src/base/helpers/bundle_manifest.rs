//! Per-message manifest for a multi-message AoI flush bundle (TG-AOI-09).
//!
//! A deferred-AoI flush packs every NPC introduction into one reliable
//! bundle, so the `AoI bundle: flushed` row only says "N messages in M
//! packets". When the client abandons the bundle part-way (an
//! `unpack_fault` at some body offset), nothing on the server said which
//! entity's message sat at that offset. The flush records, for each message
//! it appends, the body offset, the leading message id byte and the entity,
//! and [`log_bundle_manifest`] prints them beside the bundle's `base_seq`.
//!
//! A "message" here is one append: for the phase-1 bundle that is an NPC's
//! `CREATE_ENTITY + UPDATE_AVATAR` pair, for phase 2 its `createOnClient`
//! cascade. `msg_id` is the first wire message id in that append.

use std::net::SocketAddr;

use cimmeria_mercury::channel_bundle::ChannelBundle;

/// The most entries one manifest row lists. A Castle_CellBlock flush is
/// about 28 entries; the cap keeps a pathological flush from shipping a
/// multi-kilobyte log row. `messages` on the row still gives the true count.
pub const BUNDLE_MANIFEST_CAP: usize = 64;

/// One appended message: where its bytes start in the bundle body, its
/// leading message id and the entity it is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BundleManifestEntry {
    pub offset: usize,
    pub msg_id: u8,
    pub entity_id: u32,
}

/// The manifest a flush builds while it fills a [`ChannelBundle`].
#[derive(Debug, Default, Clone)]
pub struct BundleManifest {
    entries: Vec<BundleManifestEntry>,
}

impl BundleManifest {
    pub fn new() -> Self {
        Self::default()
    }

    /// Append `raw_msg` to `bundle` and record it. Always go through this
    /// rather than `bundle.append_raw_message` so the offsets stay true.
    pub fn append_raw_message(
        &mut self,
        bundle: &mut ChannelBundle,
        entity_id: u32,
        raw_msg: &[u8],
    ) {
        self.entries.push(BundleManifestEntry {
            offset: bundle.body_len(),
            msg_id: raw_msg.first().copied().unwrap_or_default(),
            entity_id,
        });
        bundle.append_raw_message(raw_msg);
    }

    pub fn entries(&self) -> &[BundleManifestEntry] {
        &self.entries
    }
}

/// What the manifest row needs from the bundle send, besides the entries.
pub(super) struct ManifestSend<'a> {
    pub(super) addr: SocketAddr,
    pub(super) witness_id: u32,
    pub(super) witness_name: Option<&'static str>,
    pub(super) kind: &'static str,
    pub(super) base_seq: u32,
    pub(super) messages: usize,
    pub(super) packets: usize,
    pub(super) entries: &'a [BundleManifestEntry],
}

/// DEBUG `aoi.bundle_manifest` for a deferred-flush bundle (`send_kind`
/// `aoi_flush_*`) of more than one message. A single-message bundle needs no
/// manifest: its one message is at offset 0. Other bundle sources pass no
/// entries and log nothing.
pub(super) fn log_bundle_manifest(send: &ManifestSend<'_>) {
    if !send.kind.starts_with("aoi_flush_") || send.messages <= 1 || send.entries.is_empty() {
        return;
    }
    let shown = &send.entries[..send.entries.len().min(BUNDLE_MANIFEST_CAP)];
    // The joins are field expressions, so they only run when the target is
    // enabled at DEBUG.
    tracing::debug!(
        target: "aoi.bundle_manifest",
        event = "bundle_manifest",
        addr = %send.addr,
        witness_id = send.witness_id,
        witness_name = send.witness_name,
        send_kind = send.kind,
        base_seq = send.base_seq,
        messages = send.messages,
        packets = send.packets,
        listed = shown.len(),
        msg_offsets = %join(shown.iter().map(|e| e.offset)),
        msg_ids = %join(shown.iter().map(|e| e.msg_id)),
        entity_ids = %join(shown.iter().map(|e| e.entity_id)),
        "AoI bundle: per-message manifest"
    );
}

fn join<T: std::fmt::Display>(items: impl Iterator<Item = T>) -> String {
    items.map(|v| v.to_string()).collect::<Vec<_>>().join(",")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn append_records_offset_and_leading_id() {
        let mut bundle = ChannelBundle::new(true);
        let mut manifest = BundleManifest::new();
        manifest.append_raw_message(&mut bundle, 7, &[0x05, 1, 2]);
        manifest.append_raw_message(&mut bundle, 9, &[0x06, 3]);
        assert_eq!(
            manifest.entries(),
            &[
                BundleManifestEntry {
                    offset: 0,
                    msg_id: 5,
                    entity_id: 7
                },
                BundleManifestEntry {
                    offset: 3,
                    msg_id: 6,
                    entity_id: 9
                },
            ]
        );
        assert_eq!(bundle.num_messages(), 2);
        assert_eq!(bundle.body_len(), 5);
    }
}
