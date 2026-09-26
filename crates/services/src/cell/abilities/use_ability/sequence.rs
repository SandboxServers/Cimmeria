//! `onSequence` argument packing for the three ability-phase sequences
//! (`Ability_Begin`, `Ability_End`, `Ability_Interrupt`).
//!
//! All three carry the same 26-byte `onSequence` body; only the sequence id
//! differs. One builder keeps the phases byte-identical, which matters
//! because a launch and its fire now run in different ticks (AT-10).

/// Pack `onSequence(KismetEventSetSeqID, SourceID, TargetID, PrimaryTarget,
/// ImpactTime, NameValuePairs[], ViewType, InstanceId)` for an ability phase.
pub(super) fn ability_sequence_args(
    sequence_id: i32,
    source_id: u32,
    target_id: i32,
    instance_id: i32,
) -> Vec<u8> {
    let mut seq_args = Vec::with_capacity(28);
    seq_args.extend_from_slice(&sequence_id.to_le_bytes()); // KismetEventSetSeqID (sequence_id)
    seq_args.extend_from_slice(&(source_id as i32).to_le_bytes()); // SourceID
    seq_args.extend_from_slice(&target_id.to_le_bytes()); // TargetID
    seq_args.push(1); // PrimaryTarget = true
    seq_args.extend_from_slice(&0.0f32.to_le_bytes()); // ImpactTime
    seq_args.extend_from_slice(&0u32.to_le_bytes()); // NameValuePairs array count = 0
    seq_args.push(0); // ViewType = KISMET_VIEW_Witness
    seq_args.extend_from_slice(&instance_id.to_le_bytes()); // InstanceId
    seq_args
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Byte layout pinned so the Begin, End and Interrupt emitters cannot
    /// drift apart (Begin and End used to be two hand-rolled copies).
    #[test]
    fn ability_sequence_args_layout() {
        let b = ability_sequence_args(0x0102_0304, 7, -1, 42);
        assert_eq!(b.len(), 26);
        assert_eq!(&b[0..4], &0x0102_0304i32.to_le_bytes());
        assert_eq!(&b[4..8], &7i32.to_le_bytes());
        assert_eq!(&b[8..12], &(-1i32).to_le_bytes());
        assert_eq!(b[12], 1);
        assert_eq!(&b[13..17], &0.0f32.to_le_bytes());
        assert_eq!(&b[17..21], &0u32.to_le_bytes());
        assert_eq!(b[21], 0);
        assert_eq!(&b[22..26], &42i32.to_le_bytes());
    }
}
