//! Client-call builders for the calls a real client makes, each pinned to
//! the lab tap capture (`tap_fixture`). Argument order and widths follow the
//! `.def` rows in `docs/protocol/cell-method-dispatch-table.md`; the tests
//! check them against the bytes the real client sent.

use crate::session::{word_len_msg, GameSession};

/// The entity id the real client puts in a cell call's prefix (README F6).
/// The server ignores the prefix, but a future server that reads it is then
/// tested against the value a real client sent.
pub const CLIENT_CALL_ENTITY_ID: u32 = 0;

/// Cell method indices, the flat table of
/// `docs/protocol/cell-method-dispatch-table.md`.
pub const CM_MOVE_ITEM: u16 = 38;
pub const CM_INTERACT: u16 = 74;
pub const CM_DIALOG_BUTTON_CHOICE: u16 = 75;
pub const CM_TRIGGER_CLIENT_HINTED_GENERIC_REGION: u16 = 85;
pub const CM_CANCEL_MOVIE: u16 = 108;
pub const CM_GM_GOTO_XYZ: u16 = 163;

/// Client message ids that are not cell or base methods.
pub const MSG_REQUEST_ENTITY_UPDATE: u8 = 0x07;
pub const BASE_CREATE_CHARACTER: u8 = 0xC3;

/// WSTRING: u32 count of UTF-16 code units, then the units little-endian.
pub fn write_wstring(out: &mut Vec<u8>, s: &str) {
    let units: Vec<u16> = s.encode_utf16().collect();
    let count = u32::try_from(units.len()).expect("wstring overflows the u32 count");
    out.extend_from_slice(&count.to_le_bytes());
    for unit in units {
        out.extend_from_slice(&unit.to_le_bytes());
    }
}

/// Concatenated little-endian INT32 values.
fn int32_args(values: &[i32]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}

/// Concatenated little-endian FLOAT values, three for a VECTOR3.
fn float3_args(values: [f32; 3]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}

impl GameSession {
    /// `interact(INT32 overrideTarget)`: the NPC interaction, CM 74.
    pub fn interact(entity_id: u32, override_target: i32) -> Vec<u8> {
        Self::cell_method(CM_INTERACT, entity_id, &int32_args(&[override_target]))
    }

    /// `dialogButtonChoice(INT32 dialogId, INT32 buttonId)`: CM 75.
    pub fn dialog_button_choice(entity_id: u32, dialog_id: i32, button_id: i32) -> Vec<u8> {
        Self::cell_method(
            CM_DIALOG_BUTTON_CHOICE,
            entity_id,
            &int32_args(&[dialog_id, button_id]),
        )
    }

    /// `gmGotoXYZ(FLOAT x, y, z)`: CM 163, a GM teleport.
    pub fn gm_goto_xyz(entity_id: u32, pos: [f32; 3]) -> Vec<u8> {
        Self::cell_method(CM_GM_GOTO_XYZ, entity_id, &float3_args(pos))
    }

    /// `triggerClientHintedGenericRegion(INT32 id, UINT8 bEntering, VECTOR3
    /// position)`: CM 85. The flag is one byte on the wire, not an INT32.
    pub fn trigger_client_hinted_generic_region(
        entity_id: u32,
        region_id: i32,
        entering: bool,
        pos: [f32; 3],
    ) -> Vec<u8> {
        let mut args = int32_args(&[region_id]);
        args.push(u8::from(entering));
        args.extend(float3_args(pos));
        Self::cell_method(CM_TRIGGER_CLIENT_HINTED_GENERIC_REGION, entity_id, &args)
    }

    /// `moveItem(INT32 itemId, INT32 targetBag, INT32 targetSlot, INT32
    /// quantity)`: CM 38, msg 0xA6. `target_slot_wire` is 1-based, as the
    /// client sends it; the cell subtracts one.
    pub fn move_item(
        entity_id: u32,
        item_id: i32,
        target_bag: i32,
        target_slot_wire: i32,
        quantity: i32,
    ) -> Vec<u8> {
        Self::cell_method(
            CM_MOVE_ITEM,
            entity_id,
            &int32_args(&[item_id, target_bag, target_slot_wire, quantity]),
        )
    }

    /// `cancelMovie(WSTRING movieName)`: CM 108, the cinematic-finished signal.
    pub fn cancel_movie(entity_id: u32, movie_name: &str) -> Vec<u8> {
        let mut args = Vec::new();
        write_wstring(&mut args, movie_name);
        Self::cell_method(CM_CANCEL_MOVIE, entity_id, &args)
    }

    /// `[u32 entity_id]`, no cache stamps: this client build sends none.
    pub fn request_entity_update(entity_id: u32) -> Vec<u8> {
        word_len_msg(MSG_REQUEST_ENTITY_UPDATE, &entity_id.to_le_bytes())
    }

    /// `createCharacter` on the base: `[WSTRING Name][WSTRING ExtraName]
    /// [INT32 CharDefId][u32 count][count x (INT32 VisGroupId, INT32
    /// ChoiceId)][INT32 SkinTintColorID]`, the layout that
    /// `handle_create_character` parses.
    pub fn create_character(
        name: &str,
        extra_name: &str,
        char_def_id: i32,
        visual_choices: &[(i32, i32)],
        skin_tint_color_id: i32,
    ) -> Vec<u8> {
        let mut args = Vec::new();
        write_wstring(&mut args, name);
        write_wstring(&mut args, extra_name);
        args.extend(int32_args(&[char_def_id]));
        let count = u32::try_from(visual_choices.len()).expect("visual count overflows u32");
        args.extend_from_slice(&count.to_le_bytes());
        for &(vis_group_id, choice_id) in visual_choices {
            args.extend(int32_args(&[vis_group_id, choice_id]));
        }
        args.extend(int32_args(&[skin_tint_color_id]));
        Self::base_method(BASE_CREATE_CHARACTER, &args)
    }
}

#[cfg(test)]
mod tests {
    use super::{GameSession, CLIENT_CALL_ENTITY_ID};
    use crate::tap_fixture::{TapCapture, TapRecord};

    /// The builder's output is the real client's bytes: the msg id, the
    /// word length, then the whole inbound payload, prefix included.
    fn assert_matches_record(out: &[u8], rec: &TapRecord) {
        assert_eq!(out[0], rec.msg_id.unwrap(), "{}: msg id", rec.msg_name);
        assert_eq!(
            u16::from_le_bytes([out[1], out[2]]) as usize,
            rec.args_len,
            "{}: word length",
            rec.msg_name
        );
        assert_eq!(
            &out[3..],
            rec.args().as_slice(),
            "{}: payload",
            rec.msg_name
        );
    }

    /// The `f32` at byte `at` of a captured payload. Positions come from the
    /// capture's bits (README F12): the region trigger's z is one ulp from
    /// the decimal `-212.8`, so a literal would not match.
    fn f32_at(args: &[u8], at: usize) -> f32 {
        f32::from_le_bytes(args[at..at + 4].try_into().unwrap())
    }

    #[test]
    fn dialog_button_choice_matches_the_capture() {
        let cap = TapCapture::praxis_start();
        let cases = [
            (1, 2982, -1),
            (33, 3995, -1),
            (59, 5882, -1),
            (61, 3996, -1),
        ];
        for (i, dialog_id, button_id) in cases {
            let rec = cap.expect(i, "dialogButtonChoice");
            let out =
                GameSession::dialog_button_choice(CLIENT_CALL_ENTITY_ID, dialog_id, button_id);
            assert_matches_record(&out, rec);
        }
    }

    #[test]
    fn gm_goto_xyz_matches_the_capture() {
        let cap = TapCapture::praxis_start();
        for i in [4, 35] {
            let rec = cap.expect(i, "gmGotoXYZ");
            let a = rec.args();
            let pos = [f32_at(&a, 5), f32_at(&a, 9), f32_at(&a, 13)];
            let out = GameSession::gm_goto_xyz(CLIENT_CALL_ENTITY_ID, pos);
            assert_matches_record(&out, rec);
        }
    }

    #[test]
    fn region_trigger_matches_the_capture() {
        let cap = TapCapture::praxis_start();
        let rec = cap.expect(8, "triggerClientHintedGenericRegion");
        let a = rec.args();
        let pos = [f32_at(&a, 10), f32_at(&a, 14), f32_at(&a, 18)];
        let out = GameSession::trigger_client_hinted_generic_region(
            CLIENT_CALL_ENTITY_ID,
            14,
            false,
            pos,
        );
        assert_matches_record(&out, rec);
    }

    #[test]
    fn interact_matches_the_capture() {
        let cap = TapCapture::praxis_start();
        for (i, target) in [(19, 100751), (42, 100748)] {
            let rec = cap.expect(i, "interact");
            let out = GameSession::interact(CLIENT_CALL_ENTITY_ID, target);
            assert_matches_record(&out, rec);
        }
    }

    /// Direct encoding: msg 166 is `0x80 | 38`, no sub-slot byte.
    #[test]
    fn move_item_matches_the_capture() {
        let cap = TapCapture::praxis_start();
        let rec = cap.expect(65, "moveItem");
        let out = GameSession::move_item(CLIENT_CALL_ENTITY_ID, 10345, 3, 1, 1);
        assert_matches_record(&out, rec);
    }

    /// The one call whose argument is the entity id itself, with no prefix.
    #[test]
    fn request_entity_update_matches_the_capture() {
        let cap = TapCapture::praxis_start();
        let rec = cap.expect(9, "requestEntityUpdate");
        let out = GameSession::request_entity_update(100745);
        assert_matches_record(&out, rec);
    }

    /// Hand bytes: the extended msg `0xBD`, the sub-slot 47 (108 - 61), then
    /// a WSTRING of 20 units, each ASCII character followed by a zero byte.
    #[test]
    fn cancel_movie_encodes_a_wstring() {
        let mut want = vec![0xBD, 49, 0, 0, 0, 0, 0, 47, 20, 0, 0, 0];
        want.extend_from_slice(b"C\0i\0n\0e\0-\0S\0G\0W\0L\0o\0g\0o\0.\0S\0G\0W\0L\0o\0g\0o\0");
        let out = GameSession::cancel_movie(CLIENT_CALL_ENTITY_ID, "Cine-SGWLogo.SGWLogo");
        assert_eq!(out, want);
    }

    /// Hand bytes for `createCharacter`, checked against the layout
    /// `handle_create_character` parses: name "Ab", empty extra name, char
    /// def 3, one visual choice (group 1, choice 2), skin tint 0.
    #[test]
    fn create_character_layout() {
        let want = [
            0xC3, 32, 0, // base msg 0xC3, word length 32
            2, 0, 0, 0, 0x41, 0, 0x62, 0, // WSTRING "Ab"
            0, 0, 0, 0, // WSTRING "" (extra name)
            3, 0, 0, 0, // INT32 CharDefId
            1, 0, 0, 0, // u32 visual choice count
            1, 0, 0, 0, 2, 0, 0, 0, // (INT32 VisGroupId 1, INT32 ChoiceId 2)
            0, 0, 0, 0, // INT32 SkinTintColorID
        ];
        let out = GameSession::create_character("Ab", "", 3, &[(1, 2)], 0);
        assert_eq!(out, want);
    }
}
