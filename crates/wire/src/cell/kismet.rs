//! Shared `onSequence` argument builder.
//!
//! Every server-driven kismet sequence — ring transport, gate events,
//! item animations, ability fire — sends the same 26-byte `onSequence`
//! payload; only the sequence id, the source/target entity and the view
//! type differ. The layout lived in `ring_transport::wire_helpers` until
//! the gate emitter needed it too; it's here now so there is one
//! definition of the byte order rather than two.

/// `KISMET_VIEW_EventInvoker` — the camera follows the entity that
/// triggered the sequence. `entities/defs/enumerations.xml:1581`
/// (`deprecated/python/Atrea/enums.py:1159`).
pub const KISMET_VIEW_EVENT_INVOKER: u8 = 3;

/// Build an `onSequence` payload.
///
/// `entity_id` is used for BOTH `SourceID` and `TargetID` — that is what
/// the 2009 server does at every call site we've recovered
/// (`deprecated/python/cell/SGWPlayer.py:2112`, `:2124` pass
/// `self.entityId` twice).
pub fn build_on_sequence_args(seq_id: i32, entity_id: u32, view_type: u8) -> Vec<u8> {
    let mut args = Vec::with_capacity(26);
    args.extend_from_slice(&seq_id.to_le_bytes()); // KismetEventSetSeqID
    args.extend_from_slice(&(entity_id as i32).to_le_bytes()); // SourceID
    args.extend_from_slice(&(entity_id as i32).to_le_bytes()); // TargetID
    args.push(1); // PrimaryTarget = true
    args.extend_from_slice(&0.0f32.to_le_bytes()); // ImpactTime
    args.extend_from_slice(&0u32.to_le_bytes()); // NameValuePairs count = 0
    args.push(view_type); // ViewType
    args.extend_from_slice(&0i32.to_le_bytes()); // InstanceId
    args
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Byte-exact layout pin. 26 bytes, little-endian throughout, with
    /// the entity id repeated as source and target.
    #[test]
    fn on_sequence_args_are_26_le_bytes_with_duplicated_entity_id() {
        let args = build_on_sequence_args(0x1122_3344, 0x00AA_BBCC, KISMET_VIEW_EVENT_INVOKER);
        assert_eq!(args.len(), 26, "onSequence payload is exactly 26 bytes");
        assert_eq!(
            args,
            vec![
                0x44, 0x33, 0x22, 0x11, // seq id (LE)
                0xCC, 0xBB, 0xAA, 0x00, // SourceID
                0xCC, 0xBB, 0xAA, 0x00, // TargetID
                0x01, // PrimaryTarget
                0x00, 0x00, 0x00, 0x00, // ImpactTime = 0.0f
                0x00, 0x00, 0x00, 0x00, // NVP count = 0
                0x03, // ViewType = KISMET_VIEW_EventInvoker
                0x00, 0x00, 0x00, 0x00, // InstanceId
            ]
        );
    }
}
