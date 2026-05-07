//! Wire-encoding tests for individual entity-method calls (BeingAppearance,
//! onEntityTint) emitted as part of the world-entry packet stream.

use super::super::*;

/// Verify BeingAppearance wire encoding matches C++ reference:
///   msg_id=0x9A (index 26, direct: 26|0x80)
///   word_len=u16 LE (entity_id + bodyset WSTRING + array WSTRING)
///   entity_id=u32 LE
///   bodyset: [u32 char_count][UTF-16LE data]
///   components: [u32 element_count][WSTRING element]*
#[test]
fn being_appearance_wire_encoding() {
    let entity_id: u32 = 42;
    let bodyset = "BS_HumanMale.BS_HumanMale";
    let components = vec![
        "BS_HumanMale.BS_HM_Head_00".to_string(),
        "BS_HumanMale.BS_HM_Torso_00".to_string(),
        "BS_HumanMale.BS_HM_Legs_00".to_string(),
        "AR_Global.Prisoner_Torso".to_string(),
    ];

    let mut args = Vec::new();
    write_wstring(&mut args, bodyset);
    args.extend_from_slice(&(components.len() as u32).to_le_bytes());
    for comp in &components {
        write_wstring(&mut args, comp);
    }

    let mut body = Vec::new();
    append_entity_method(&mut body, method_idx::BEING_APPEARANCE, entity_id, &args);

    // method_index=26 < 61 -> direct: msg_id = 26 | 0x80 = 0x9A
    assert_eq!(
        body[0], 0x9A,
        "msg_id should be 0x9A for BeingAppearance (index 26)"
    );

    // word_len (u16 LE) at offset 1-2
    let word_len = u16::from_le_bytes([body[1], body[2]]);
    let expected_payload = 4 + args.len(); // entity_id + args
    assert_eq!(word_len as usize, expected_payload, "word_len mismatch");

    // entity_id (u32 LE) at offset 3-6
    let eid = u32::from_le_bytes([body[3], body[4], body[5], body[6]]);
    assert_eq!(eid, entity_id, "entity_id mismatch");

    // bodyset WSTRING at offset 7
    let off = 7;
    let bs_char_count =
        u32::from_le_bytes([body[off], body[off + 1], body[off + 2], body[off + 3]]);
    assert_eq!(
        bs_char_count as usize,
        bodyset.len(),
        "bodyset char_count should match"
    );

    // Verify bodyset UTF-16LE chars
    let bs_data_start = off + 4;
    for (i, ch) in bodyset.encode_utf16().enumerate() {
        let wire_ch =
            u16::from_le_bytes([body[bs_data_start + i * 2], body[bs_data_start + i * 2 + 1]]);
        assert_eq!(wire_ch, ch, "bodyset char {i} mismatch");
    }

    // Component array count
    let comp_off = bs_data_start + bodyset.len() * 2;
    let comp_count = u32::from_le_bytes([
        body[comp_off],
        body[comp_off + 1],
        body[comp_off + 2],
        body[comp_off + 3],
    ]);
    assert_eq!(
        comp_count,
        components.len() as u32,
        "component count mismatch"
    );

    // Verify each component WSTRING
    let mut cursor = comp_off + 4;
    for (idx, comp) in components.iter().enumerate() {
        let cc = u32::from_le_bytes([
            body[cursor],
            body[cursor + 1],
            body[cursor + 2],
            body[cursor + 3],
        ]);
        assert_eq!(
            cc as usize,
            comp.len(),
            "component {idx} char_count mismatch"
        );
        cursor += 4;
        for (i, ch) in comp.encode_utf16().enumerate() {
            let wire_ch = u16::from_le_bytes([body[cursor + i * 2], body[cursor + i * 2 + 1]]);
            assert_eq!(wire_ch, ch, "component {idx} char {i} mismatch");
        }
        cursor += comp.len() * 2;
    }

    // Verify total length
    assert_eq!(
        cursor,
        body.len(),
        "total body length should match parsed position"
    );
}

/// Verify onEntityTint wire encoding: 3x u32 LE
#[test]
fn entity_tint_wire_encoding() {
    let entity_id: u32 = 42;
    let primary: u32 = 0;
    let secondary: u32 = 0;
    let skin: u32 = SKIN_TINTS[5]; // skin_color_id = 5

    let mut args = Vec::with_capacity(12);
    args.extend_from_slice(&primary.to_le_bytes());
    args.extend_from_slice(&secondary.to_le_bytes());
    args.extend_from_slice(&skin.to_le_bytes());

    let mut body = Vec::new();
    append_entity_method(&mut body, method_idx::ON_ENTITY_TINT, entity_id, &args);

    // method_index=10 < 61 -> direct: msg_id = 10 | 0x80 = 0x8A
    assert_eq!(
        body[0], 0x8A,
        "msg_id should be 0x8A for onEntityTint (index 10)"
    );

    let word_len = u16::from_le_bytes([body[1], body[2]]);
    assert_eq!(
        word_len, 16,
        "word_len should be 4 (entity_id) + 12 (3x u32)"
    );

    let eid = u32::from_le_bytes([body[3], body[4], body[5], body[6]]);
    assert_eq!(eid, entity_id);

    let p = u32::from_le_bytes([body[7], body[8], body[9], body[10]]);
    assert_eq!(p, 0, "primaryColorId should be 0");

    let s = u32::from_le_bytes([body[11], body[12], body[13], body[14]]);
    assert_eq!(s, 0, "secondaryColorId should be 0");

    let sk = u32::from_le_bytes([body[15], body[16], body[17], body[18]]);
    assert_eq!(sk, SKIN_TINTS[5], "skinTint should match SKIN_TINTS[5]");
}
