//! The one `mercury::aoi` test that drives a `SpaceManager`.
//!
//! The AoI packet builders and the rest of their wire-layout tests moved to
//! `cimmeria-wire` (`mercury/aoi/tests.rs`) in wave W3a of the services crate
//! split. This test also needs `SpaceManager`, which is still in this crate,
//! so it stays here until the cell-world crate takes `space_manager`.

use crate::mercury::aoi::pack_angle;

/// A client facing byte must survive the store-as-radians round trip, in the
/// right slot: wire order is (yaw, pitch, roll), `direction` is
/// `[pitch, yaw, roll]`, and the broadcast reads yaw from `direction.y`.
#[test]
fn client_facing_bytes_round_trip_through_radians() {
    use crate::cell::space_manager::SpaceManager;
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();
    mgr.create_entity(1, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();

    for yaw in [0i8, 1, 64, 127, -1, -64, -128] {
        mgr.update_entity_position(1, [10.0, 0.0, 10.0], [yaw, 5, -7], [0.0; 3]);
        let d = mgr.get_entity(1).unwrap().direction;
        assert_eq!(pack_angle(d.y), yaw as u8, "yaw byte {yaw} must round-trip");
        assert_eq!(pack_angle(d.x), 5, "pitch lands in direction.x");
        assert_eq!(pack_angle(d.z), (-7i8) as u8, "roll lands in direction.z");
        assert!(
            d.y.abs() <= std::f32::consts::PI + 1e-3,
            "stored as radians"
        );
    }
}
