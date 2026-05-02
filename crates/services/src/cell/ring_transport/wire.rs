//! Wire-format helpers for the ring transporter client method.
//!
//! `onRingTransporterList(RegionInfo aRegion, ARRAY<RegionInfo> aRegionList)`
//! is method index 133 on SGWPlayer (ClientMethod surface).
//!
//! `RegionInfo` is a FIXED_DICT alias (`entities/defs/alias.xml:497`):
//!
//! ```text
//! RegionInfo {
//!     RegionId    : INT32
//!     DisplayName : INT32
//!     WorldId     : INT32
//!     Position    : VECTOR3   // 3 x f32
//! }
//! ```
//!
//! ARRAY uses a `u32 LE` count prefix followed by each element back-to-back —
//! same convention used by `onMailHeaderInfo` (see `cell/mail.rs`).

use super::regions::RingRegion;

/// Append one `RegionInfo` FIXED_DICT to `out`.
///
/// 4 INT32 + 3 FLOAT = 24 bytes per entry.
pub fn encode_region_info(out: &mut Vec<u8>, region: &RingRegion) {
    out.extend_from_slice(&region.region_id.to_le_bytes());
    out.extend_from_slice(&region.display_name_id.to_le_bytes());
    out.extend_from_slice(&region.world_id.to_le_bytes());
    out.extend_from_slice(&region.x.to_le_bytes());
    out.extend_from_slice(&region.y.to_le_bytes());
    out.extend_from_slice(&region.z.to_le_bytes());
}

/// Build the full args payload for `onRingTransporterList`:
///   RegionInfo source + ARRAY<RegionInfo> destinations.
pub fn build_on_ring_transporter_list(
    source: &RingRegion,
    destinations: &[&RingRegion],
) -> Vec<u8> {
    // 24 bytes per RegionInfo + 4-byte ARRAY length prefix.
    let mut out = Vec::with_capacity(24 + 4 + destinations.len() * 24);
    encode_region_info(&mut out, source);
    out.extend_from_slice(&(destinations.len() as u32).to_le_bytes());
    for dst in destinations {
        encode_region_info(&mut out, dst);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_region(id: i32, world_id: i32, name_id: i32, pos: [f32; 3]) -> RingRegion {
        RingRegion {
            region_id: id,
            world_id,
            world_name: format!("World{world_id}"),
            x: pos[0], y: pos[1], z: pos[2],
            tag: format!("Region{id}"),
            height: 1.7,
            radius: 3.5,
            event_set_id: 0,
            display_name_id: name_id,
            destination_ids: vec![],
            point_set_id: 0,
        }
    }

    #[test]
    fn region_info_is_24_bytes() {
        let r = make_region(1, 12, 7508, [-215.455, 65.918, -121.4]);
        let mut out = Vec::new();
        encode_region_info(&mut out, &r);
        assert_eq!(out.len(), 24);

        // Field-by-field byte verification: all little-endian, in declaration order.
        assert_eq!(&out[0..4], &1i32.to_le_bytes());
        assert_eq!(&out[4..8], &7508i32.to_le_bytes());
        assert_eq!(&out[8..12], &12i32.to_le_bytes());
        assert_eq!(&out[12..16], &(-215.455f32).to_le_bytes());
        assert_eq!(&out[16..20], &65.918f32.to_le_bytes());
        assert_eq!(&out[20..24], &(-121.4f32).to_le_bytes());
    }

    #[test]
    fn payload_layout_source_then_array() {
        let src = make_region(1, 12, 7508, [-215.455, 65.918, -121.4]);
        let dst = make_region(2, 12, 7508, [-192.657, 55.258, -154.844]);
        let payload = build_on_ring_transporter_list(&src, &[&dst]);

        // Source RegionInfo (24) + array length (4) + 1 destination (24) = 52
        assert_eq!(payload.len(), 52);

        // Array length prefix is u32 LE = 1.
        let len = u32::from_le_bytes([payload[24], payload[25], payload[26], payload[27]]);
        assert_eq!(len, 1);

        // Destination RegionId starts at offset 28 (right after the length prefix).
        assert_eq!(&payload[28..32], &2i32.to_le_bytes());
    }

    #[test]
    fn empty_destinations_still_writes_zero_count() {
        let src = make_region(1, 12, 7508, [0.0; 3]);
        let payload = build_on_ring_transporter_list(&src, &[]);
        // 24 (source) + 4 (count=0)
        assert_eq!(payload.len(), 28);
        assert_eq!(&payload[24..28], &0u32.to_le_bytes());
    }
}
