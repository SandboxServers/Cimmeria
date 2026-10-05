//! Cimmeria-side additions to `CookedDataStargates.pak` (category 13).
//!
//! The client's stargate table — id, world, prefab sequence, transform and
//! the six-glyph address of every gate — comes from this catalogue, not from
//! the wire: `setupStargateInfo` and `updateStargateAddress` carry bare ids
//! that the client resolves here. The shipped PAK holds the 28 gates of the
//! 2009 seed (ids 1–28). A gate the server adds needs a cooked entry too, or
//! the client is handed a world gate list naming an id it cannot resolve.
//!
//! The only addition is gate 29, the Debug Area's (world 1300, packet DA-07):
//! the Ihpet_Crater_Light map's own gate prop, which world 1300 loads. It is
//! `resources.stargates` row 29 (`debug_dial_hub = true`), and a live-DB test
//! in `cimmeria-services` (`gate_round_trip_tests::debug_area_gate_seed`)
//! holds the two copies together.
//!
//! Delivery is the same per-key handshake as `super::world_info_overrides`:
//! the entry is added in memory at startup and the category's metadata is
//! bumped, so a client holding the shipped table is resynced with it (#840).
//! **Never edit the PAK on disk** — it stays byte-identical to what clients
//! ship with (`MetaData` 4568); the reasons are in `world_info_overrides`.
//!
//! The emitted XML keeps the shipped element's shape (SOAP namespaces,
//! attribute order `prefabSequence` … `id`, an explicit end tag), since that
//! is what the client demonstrably parses for this element type.

/// One `COOKED_STARGATE` entry the client must hold.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StargateAddition {
    /// Cooked catalogue key (`_<id>`) and `resources.stargates.stargate_id`.
    pub stargate_id: u32,
    pub world_id: u32,
    pub name: &'static str,
    pub prefab_sequence: &'static str,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub roll: f32,
    /// `address1` … `address6`: glyphs 1–38.
    pub address: [u8; 6],
    /// Point-of-origin glyph (1–38) — a glyph, not an identifier.
    pub address_origin: u8,
}

/// Gate 29: the Debug Area (world 1300). Every field but the id, world, name
/// and address is gate 20's (`Ihpet Crater (SGU)`, world 73), because it is
/// the same prop on the same client map. The address 38-37-36-35-34-33 is in
/// no shipped entry, so a glyph sequence typed into the DHD can never resolve
/// to it by accident; nobody may dial it anyway.
pub const DEBUG_AREA_GATE: StargateAddition = StargateAddition {
    stargate_id: 29,
    world_id: 1300,
    name: "Debug Area",
    prefab_sequence: "Ihpet_Crater_Light.Main_Sequence.Prefabs.GLB-Stargate_Prefab_Seq",
    x: 251.25,
    y: 10.606,
    z: -989.781,
    yaw: 0.0,
    pitch: 0.0,
    roll: 0.0,
    address: [38, 37, 36, 35, 34, 33],
    address_origin: 2,
};

/// Every gate Cimmeria adds to the catalogue. Ids must not collide with an
/// entry the shipped PAK already has; the
/// `on_disk_stargate_pak_is_the_client_shipped_file` test enforces that.
pub const STARGATE_ADDITIONS: &[StargateAddition] = &[DEBUG_AREA_GATE];

fn escape_xml_attr(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}

/// Generate the `<COOKED_STARGATE>` entry for one addition.
pub fn generate_stargate_xml(g: &StargateAddition) -> Vec<u8> {
    format!(
        concat!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n",
            "<COOKED_STARGATE",
            " xmlns:SOAP-ENV=\"http://schemas.xmlsoap.org/soap/envelope/\"",
            " xmlns:SOAP-ENC=\"http://schemas.xmlsoap.org/soap/encoding/\"",
            " xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\"",
            " xmlns:xsd=\"http://www.w3.org/2001/XMLSchema\"",
            " xmlns:CookedData1=\"SGW\"",
            " prefabSequence=\"{}\" roll=\"{}\" yaw=\"{}\" pitch=\"{}\"",
            " zPos=\"{}\" yPos=\"{}\" xPos=\"{}\" worldId=\"{}\"",
            " addressOrigin=\"{}\" address6=\"{}\" address5=\"{}\" address4=\"{}\"",
            " address3=\"{}\" address2=\"{}\" address1=\"{}\"",
            " name=\"{}\" id=\"{}\">",
            "</COOKED_STARGATE>",
        ),
        escape_xml_attr(g.prefab_sequence),
        g.roll,
        g.yaw,
        g.pitch,
        g.z,
        g.y,
        g.x,
        g.world_id,
        g.address_origin,
        g.address[5],
        g.address[4],
        g.address[3],
        g.address[2],
        g.address[1],
        g.address[0],
        escape_xml_attr(g.name),
        g.stargate_id,
    )
    .into_bytes()
}
