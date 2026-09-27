//! The vault-open client-method payload: `onVaultOpen` (106),
//! `onTeamVaultOpen` (107) and `onCommandVaultOpen` (108) all carry
//! `(INT32 EntityId, VECTOR3 Position)`
//! (`docs/protocol/client-method-dispatch-table.md:253-255`,
//! `entities/defs/SGWPlayer.def`).
//!
//! The client only shows the window: `Event_UI_VaultVisibility` runs
//! `VaultMod.onVaultVisibility` (audit A-02). Nothing on the client reads
//! `Position` (BV-E1 Q4), so it is sent for wire fidelity and no server
//! behaviour depends on the client using it.

/// Serialize the `(INT32 EntityId, VECTOR3 Position)` args of a vault-open
/// method: 4 bytes of LE `i32`, then three LE `f32` (x, y, z). 16 bytes.
pub fn build_vault_open_args(entity_id: i32, position: [f32; 3]) -> Vec<u8> {
    let mut args = Vec::with_capacity(16);
    args.extend_from_slice(&entity_id.to_le_bytes());
    for axis in position {
        args.extend_from_slice(&axis.to_le_bytes());
    }
    args
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Byte-exact: INT32 id, then x, y, z as LE f32, no marker or padding.
    /// A reordered or widened field shifts every byte after it.
    #[test]
    fn vault_open_args_are_int32_then_vector3() {
        let args = build_vault_open_args(0x0001_86A5, [1.5, -2.0, 300.25]);
        assert_eq!(
            args,
            vec![
                0xA5, 0x86, 0x01, 0x00, // EntityId 100005
                0x00, 0x00, 0xC0, 0x3F, // x = 1.5
                0x00, 0x00, 0x00, 0xC0, // y = -2.0
                0x00, 0x20, 0x96, 0x43, // z = 300.25
            ]
        );
    }
}
