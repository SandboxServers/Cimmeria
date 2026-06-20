//! Vendor-session resolution + server-authoritative template-id validation.
//!
//! The open vendor's template id is read from the vendor entity itself
//! rather than trusted from the client request, so a client can't open
//! vendor A then submit a purchase against vendor B's item lists.

use crate::cell::space_manager::SpaceManager;

/// Resolved vendor session for the player, looked up from server-side state.
///
/// `vendor_template_id` is read from the vendor entity itself rather than trusted
/// from the client's request — this prevents a client from opening vendor A
/// then submitting a purchase against vendor B's item lists.
pub(super) struct VendorSession {
    pub(super) player_id: i32,
    pub(super) vendor_entity_id: i32,
    /// Server-side authoritative template id for the currently-open vendor.
    /// May be `None` if the vendor entity has no template attached.
    pub(super) server_template_id: Option<i32>,
}

pub(super) fn vendor_context(entity_id: u32, space_mgr: &SpaceManager) -> Option<VendorSession> {
    let player = space_mgr.get_entity(entity_id)?;
    let player_id = player.player_id?;
    let vendor_entity_id_u32 = player.vendor_entity?;
    let vendor_entity_id = vendor_entity_id_u32 as i32;
    let server_template_id = space_mgr
        .get_entity(vendor_entity_id_u32)
        .and_then(|v| v.template_id);
    Some(VendorSession {
        player_id,
        vendor_entity_id,
        server_template_id,
    })
}

/// Validate that the client-supplied template id matches the vendor that was
/// opened. Returns the authoritative server-side id on success, or `None` after
/// logging the mismatch.
pub(super) fn validate_template_id(
    entity_id: u32,
    op: &str,
    session: &VendorSession,
    client_template_id: i32,
) -> Option<i32> {
    match session.server_template_id {
        Some(server_id) if server_id == client_template_id => Some(server_id),
        Some(server_id) => {
            tracing::warn!(
                entity_id,
                op,
                server_template_id = server_id,
                client_template_id,
                vendor_entity_id = session.vendor_entity_id,
                "vendor op rejected: client supplied template id does not match opened vendor"
            );
            None
        }
        None => {
            tracing::warn!(
                entity_id,
                op,
                client_template_id,
                vendor_entity_id = session.vendor_entity_id,
                "vendor op rejected: opened vendor has no template id (server cannot validate)"
            );
            None
        }
    }
}

#[cfg(test)]
mod vendor_context_tests {
    use super::{vendor_context, VendorSession};
    use crate::test_support::make_space_manager;

    fn assert_session(
        session: Option<VendorSession>,
        expected_player_id: i32,
        expected_vendor_entity_id: i32,
        expected_template_id: Option<i32>,
    ) {
        let s = session.expect("vendor_context should return Some");
        assert_eq!(s.player_id, expected_player_id);
        assert_eq!(s.vendor_entity_id, expected_vendor_entity_id);
        assert_eq!(s.server_template_id, expected_template_id);
    }

    #[test]
    fn returns_none_when_entity_does_not_exist() {
        let mgr = make_space_manager();
        assert!(vendor_context(99999, &mgr).is_none());
    }

    #[test]
    fn returns_none_when_player_id_missing() {
        let mut mgr = make_space_manager();
        mgr.create_entity(1, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        // player.player_id deliberately left None.
        let vendor = mgr.allocate_npc_id();
        mgr.spawn_npc(vendor, "Agnos", [2.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        if let Some(p) = mgr.get_entity_mut(1) {
            p.vendor_entity = Some(vendor);
        }
        assert!(vendor_context(1, &mgr).is_none());
    }

    #[test]
    fn returns_none_when_vendor_entity_unset() {
        let mut mgr = make_space_manager();
        mgr.create_entity(1, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        if let Some(p) = mgr.get_entity_mut(1) {
            p.player_id = Some(4242);
            // p.vendor_entity deliberately left None — no vendor opened.
        }
        assert!(vendor_context(1, &mgr).is_none());
    }

    #[test]
    fn happy_path_returns_session_with_template_id() {
        let mut mgr = make_space_manager();
        mgr.create_entity(1, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        let vendor = mgr.allocate_npc_id();
        mgr.spawn_npc(vendor, "Agnos", [2.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        if let Some(v) = mgr.get_entity_mut(vendor) {
            v.template_id = Some(9001);
        }
        if let Some(p) = mgr.get_entity_mut(1) {
            p.player_id = Some(4242);
            p.vendor_entity = Some(vendor);
        }
        assert_session(vendor_context(1, &mgr), 4242, vendor as i32, Some(9001));
    }

    #[test]
    fn happy_path_returns_session_when_vendor_lacks_template_id() {
        // The function reads template_id authoritatively from the vendor
        // entity, falling back to None rather than fabricating a value. A
        // missing template_id surfaces as `server_template_id: None` —
        // validate_template_id at the caller treats that as a rejection
        // (so a client can't open a templateless vendor and submit ops).
        let mut mgr = make_space_manager();
        mgr.create_entity(1, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        let vendor = mgr.allocate_npc_id();
        mgr.spawn_npc(vendor, "Agnos", [2.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        // Deliberately not setting template_id on the vendor.
        if let Some(p) = mgr.get_entity_mut(1) {
            p.player_id = Some(4242);
            p.vendor_entity = Some(vendor);
        }
        assert_session(vendor_context(1, &mgr), 4242, vendor as i32, None);
    }

    #[test]
    fn returns_session_with_no_template_when_vendor_entity_id_is_stale() {
        // The player has a stale vendor_entity pointing at an id that doesn't
        // exist (vendor despawned or never spawned). vendor_context still
        // returns Some with server_template_id: None — the caller's
        // validate_template_id arm logs and rejects the op.
        let mut mgr = make_space_manager();
        mgr.create_entity(1, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        if let Some(p) = mgr.get_entity_mut(1) {
            p.player_id = Some(4242);
            p.vendor_entity = Some(123456); // no entity at this id
        }
        assert_session(vendor_context(1, &mgr), 4242, 123456, None);
    }
}
