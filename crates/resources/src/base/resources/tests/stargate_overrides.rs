//! Category 13 (`CookedDataStargates.pak`): the committed PAK stays the file
//! clients ship with, and the Debug Area gate (29, DA-07) rides the per-key
//! handshake on top of it.

use super::super::*;
use crate::base::stargate_overrides::{
    generate_stargate_xml, StargateAddition, DEBUG_AREA_GATE, STARGATE_ADDITIONS,
};

/// Version of `CookedDataStargates.pak` that shipped clients hold.
const CLIENT_SHIPPED_STARGATES_VERSION: u32 = 4568;

/// Gate 20, `Ihpet Crater (SGU)`: the shipped entry for the prop the Debug
/// Area gate is a second row of.
const IHPET_CRATER_LIGHT_GATE: u32 = 20;

fn data_dir() -> &'static str {
    concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/cache")
}

fn shipped_stargates() -> CategoryData {
    ResourceCache::load_pak(&format!("{}/CookedDataStargates.pak", data_dir()))
        .expect("committed stargate PAK loads")
}

/// `COOKED_STARGATE` attributes in document order, namespace declarations
/// dropped.
fn attrs(xml: &[u8]) -> Vec<(String, String)> {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let text = std::str::from_utf8(xml).expect("UTF-8");
    let mut reader = Reader::from_str(text);
    let mut out = Vec::new();
    loop {
        match reader.read_event().expect("well-formed XML") {
            Event::Start(e) if e.name().as_ref() == "COOKED_STARGATE" => {
                for a in e.attributes() {
                    let a = a.expect("well-formed attribute");
                    let key = a.key.as_ref().to_string();
                    if key.starts_with("xmlns") {
                        continue;
                    }
                    // Raw value: the attributes compared here need no unescaping.
                    let value = a.value.into_owned();
                    out.push((key, value));
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    assert!(!out.is_empty(), "no COOKED_STARGATE element");
    out
}

fn attr<'a>(a: &'a [(String, String)], key: &str) -> &'a str {
    a.iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.as_str())
        .unwrap_or_else(|| panic!("missing attribute {key}"))
}

fn num(a: &[(String, String)], key: &str) -> f64 {
    attr(a, key)
        .parse()
        .unwrap_or_else(|e| panic!("{key} is not a number: {e}"))
}

fn address(a: &[(String, String)]) -> [u32; 6] {
    std::array::from_fn(|i| num(a, &format!("address{}", i + 1)) as u32)
}

/// The on-disk PAK must stay the file clients already hold: same version, no
/// Cimmeria entries.
#[test]
fn on_disk_stargate_pak_is_the_client_shipped_file() {
    let shipped = shipped_stargates();
    assert_eq!(
        shipped.metadata, CLIENT_SHIPPED_STARGATES_VERSION,
        "do not edit the PAK's MetaData; add a StargateAddition instead"
    );
    for g in STARGATE_ADDITIONS {
        assert!(
            !shipped.elements.contains_key(&g.stargate_id),
            "stargate {} is in the on-disk PAK; an addition must not shadow a shipped gate",
            g.stargate_id
        );
    }
}

/// The handshake contract: category 13 names exactly gate 29 in
/// `InvalidKeys`, serves a version a shipped client does not hold, and
/// leaves every shipped gate untouched. Deleting the `apply_stargate_overrides`
/// call from `load_all` fails this.
#[test]
fn stargates_take_the_per_key_handshake_for_the_debug_area_gate() {
    let cache = ResourceCache::load_all(data_dir()).expect("committed PAKs load");
    assert_eq!(
        cache.overridden_elements(CATEGORY_STARGATES),
        [29].as_slice()
    );

    let served = cache.category(CATEGORY_STARGATES).expect("category 13");
    let bump = compute_stargate_metadata_bump(STARGATE_ADDITIONS);
    assert_eq!(served.metadata, CLIENT_SHIPPED_STARGATES_VERSION + bump);
    assert_ne!(served.metadata, CLIENT_SHIPPED_STARGATES_VERSION);

    let shipped = shipped_stargates();
    assert_eq!(
        served.elements.len(),
        shipped.elements.len() + STARGATE_ADDITIONS.len()
    );
    assert_eq!(
        cache.get(CATEGORY_STARGATES, 29),
        Some(&generate_stargate_xml(&DEBUG_AREA_GATE))
    );
    for (id, xml) in &shipped.elements {
        assert_eq!(
            cache.get(CATEGORY_STARGATES, *id),
            Some(xml),
            "shipped gate {id} must be served untouched"
        );
    }
}

/// The generator emits what the client already parses: fed gate 20's
/// values, it reproduces the shipped `_20` attribute for attribute, in the
/// same order. Floats are compared as numbers — the shipped writer printed
/// `10.6059999` where Rust prints `10.606`, the same `f32`.
#[test]
fn generated_stargate_xml_matches_the_shipped_entry_shape() {
    let shipped = shipped_stargates();
    let actual = attrs(
        shipped
            .elements
            .get(&IHPET_CRATER_LIGHT_GATE)
            .expect("gate 20 ships in the PAK"),
    );
    let generated = attrs(&generate_stargate_xml(&StargateAddition {
        stargate_id: 20,
        world_id: 73,
        name: "Ihpet Crater (SGU)",
        address: [3, 8, 16, 24, 23, 28],
        ..DEBUG_AREA_GATE
    }));

    let keys = |a: &[(String, String)]| a.iter().map(|(k, _)| k.clone()).collect::<Vec<_>>();
    assert_eq!(keys(&generated), keys(&actual), "attribute set and order");
    for (key, value) in &actual {
        match value.parse::<f64>() {
            Ok(n) => assert!(
                (num(&generated, key) - n).abs() < 1e-3,
                "{key}: generated {} vs shipped {value}",
                attr(&generated, key)
            ),
            Err(_) => assert_eq!(attr(&generated, key), value, "{key}"),
        }
    }
}

/// Gate 29 is the gate-20 prop on the same client map: same prefab, same
/// transform, same point of origin. Checked against the PAK itself, so a
/// typo in `DEBUG_AREA_GATE` cannot hide behind a matching typo in a test
/// literal.
#[test]
fn the_debug_area_gate_is_the_ihpet_crater_light_prop() {
    let shipped = shipped_stargates();
    let ihpet = attrs(shipped.elements.get(&IHPET_CRATER_LIGHT_GATE).unwrap());
    let debug = attrs(&generate_stargate_xml(&DEBUG_AREA_GATE));

    assert_eq!(
        attr(&debug, "prefabSequence"),
        attr(&ihpet, "prefabSequence")
    );
    for key in [
        "xPos",
        "yPos",
        "zPos",
        "yaw",
        "pitch",
        "roll",
        "addressOrigin",
    ] {
        assert!(
            (num(&debug, key) - num(&ihpet, key)).abs() < 1e-3,
            "{key}: debug {} vs Ihpet Crater (SGU) {}",
            attr(&debug, key),
            attr(&ihpet, key)
        );
    }
    assert_eq!(attr(&debug, "worldId"), "1300");
    assert_eq!(attr(&debug, "id"), "29");
    assert_eq!(attr(&debug, "name"), "Debug Area");
}

/// The Debug Area address must resolve to nothing else: no shipped gate
/// carries the same six glyphs, and every glyph is a real one (1-38).
/// Shipped gates DO share addresses with each other (each Light/Dark world
/// pair, SGC W1 / SGC W2), which is why this checks the addition only.
#[test]
fn the_debug_area_address_collides_with_no_shipped_gate() {
    let shipped = shipped_stargates();
    let mine: [u32; 6] = DEBUG_AREA_GATE.address.map(u32::from);
    for glyph in mine
        .iter()
        .chain(std::iter::once(&u32::from(DEBUG_AREA_GATE.address_origin)))
    {
        assert!((1..=38).contains(glyph), "glyph {glyph} is outside 1-38");
    }
    for (id, xml) in &shipped.elements {
        assert_ne!(
            address(&attrs(xml)),
            mine,
            "the Debug Area address is shipped gate {id}'s"
        );
    }
}

#[test]
fn stargate_metadata_bump_is_deterministic_non_zero_and_change_sensitive() {
    let a = compute_stargate_metadata_bump(STARGATE_ADDITIONS);
    assert_eq!(a, compute_stargate_metadata_bump(STARGATE_ADDITIONS));
    assert_ne!(a, 0);
    assert_eq!(a & 1, 1, "low bit is always set");

    let changed = [StargateAddition {
        address: [38, 37, 36, 35, 34, 32],
        ..DEBUG_AREA_GATE
    }];
    assert_ne!(
        compute_stargate_metadata_bump(&changed),
        a,
        "a changed address must change the version so clients refetch"
    );
}

#[test]
fn apply_stargate_overrides_is_a_no_op_when_the_category_is_missing() {
    let mut categories = HashMap::new();
    let overridden = ResourceCache::apply_stargate_overrides(&mut categories);
    assert!(overridden.is_empty());
    assert!(categories.is_empty());
}
