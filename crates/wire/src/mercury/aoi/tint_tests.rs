//! Byte-exact tests for an NPC's `onEntityTint` in the `createOnClient`
//! cascade: a template that opts in with `entity_templates.send_tint` (the
//! Debug Area's Visual NPC Lineup) sends its three packed `0xRRGGBB__`
//! colours, and every other NPC still sends `onEntityTint(0, 0, 0)`
//! (`cimmeria_entity::cell_entity::EntityTint` has the colour evidence).

use cimmeria_entity::cell_entity::EntityTint;

use super::compose_create_entity_cascade_body;
use crate::cell::messages::NpcAoIData;
use crate::mercury::SGWMOB_CLASS_ID;

/// The `onEntityTint` message: direct msg id `0x80 + 10`, a u16 payload
/// length of 16, the entity id, then primary, secondary and skin, each a
/// little-endian u32.
fn tint_message(entity_id: u32, primary: u32, secondary: u32, skin: u32) -> Vec<u8> {
    let mut m = vec![0x8A, 16, 0];
    for v in [entity_id, primary, secondary, skin] {
        m.extend_from_slice(&v.to_le_bytes());
    }
    m
}

/// A humanoid NPC that opts in sends its template's colours, byte for
/// byte, in its one `onEntityTint`. The values are template 146's (NID
/// Guard - Castle outside, lineup clone 1426): -65536, -16777216 and the
/// negative skin -256076032, which go out as `0xFFFF0000`, `0xFF000000`
/// and `0xF0BC9700`. Revert proof: send `0, 0, 0` again (or negate the
/// negative columns, as the legacy Python loader did) and this fails.
#[test]
fn an_opted_in_tint_sends_the_templates_colours() {
    let tint = EntityTint::from_template_columns(-65536, -16777216, -256076032).unwrap();
    let npc = NpcAoIData {
        body_set: Some("BS_HumanMale.BS_HumanMale".into()),
        components: vec!["BS_HumanMale.BS_HM_Head_01".into()],
        tint: Some(tint),
        ..NpcAoIData::default()
    };
    let body = compose_create_entity_cascade_body(100014, SGWMOB_CLASS_ID, 1, Some(&npc));
    let want = tint_message(100014, 0xFFFF_0000, 0xFF00_0000, 0xF0BC_9700);
    assert_eq!(
        &want[..],
        &[
            0x8A, 16, 0, 0xAE, 0x86, 0x01, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0xFF,
            0x00, 0x97, 0xBC, 0xF0
        ][..],
    );
    assert_eq!(
        body.windows(want.len()).filter(|w| *w == &want[..]).count(),
        1,
        "exactly one onEntityTint carrying the template's colours"
    );
}

/// A humanoid NPC that has not opted in (every template outside the
/// lineup) still sends `onEntityTint(0, 0, 0)`, exactly as before. Revert
/// proof: send `onEntityTint` only when `tint` is `Some` and this fails;
/// the loader side (no template outside the lineup opts in) is
/// `live_db_debug_area_lineup_tint`.
#[test]
fn a_template_that_has_not_opted_in_still_sends_zero_tint() {
    let npc = NpcAoIData {
        body_set: Some("BS_HumanMale.BS_HumanMale".into()),
        components: vec!["BS_HumanMale.BS_HM_Head_01".into()],
        tint: None,
        ..NpcAoIData::default()
    };
    let body = compose_create_entity_cascade_body(100015, SGWMOB_CLASS_ID, 1, Some(&npc));
    let zero = tint_message(100015, 0, 0, 0);
    assert_eq!(
        body.windows(zero.len()).filter(|w| *w == &zero[..]).count(),
        1,
        "exactly one onEntityTint(0, 0, 0)"
    );
}
