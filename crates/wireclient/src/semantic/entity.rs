//! The entity family: `onStaticMeshNameUpdate` (0), `onEntityProperty` (7),
//! `onVisible` (8) and `onBeingNameIDUpdate` (11), shared by both kinds, and
//! `onPlayerCommunication` (28), player-only.

use super::primitives::Reader;
use super::{S2CEvent, SemanticError};

/// `onStaticMeshNameUpdate`: the mesh and body set an entity is drawn with.
/// Both names are length-prefixed, so the second is read after the first
/// and the body must end exactly after it.
pub(super) fn static_mesh_name_update(args: &[u8]) -> Result<S2CEvent, SemanticError> {
    let mut r = Reader::new("onStaticMeshNameUpdate", args);
    let event = S2CEvent::StaticMeshNameUpdate {
        static_mesh: r.wstring()?,
        body_set: r.wstring()?,
    };
    r.finish()?;
    Ok(event)
}

/// `onEntityProperty`: one property id and its new value, both `INT32`.
pub(super) fn entity_property(args: &[u8]) -> Result<S2CEvent, SemanticError> {
    let mut r = Reader::new("onEntityProperty", args);
    let event = S2CEvent::EntityProperty {
        property: r.i32()?,
        value: r.i32()?,
    };
    r.finish()?;
    Ok(event)
}

/// `onVisible`: a single `INT8`, non-zero when the entity becomes visible.
pub(super) fn visible(args: &[u8]) -> Result<S2CEvent, SemanticError> {
    let mut r = Reader::new("onVisible", args);
    let event = S2CEvent::Visible { visible: r.i8()? };
    r.finish()?;
    Ok(event)
}

/// `onBeingNameIDUpdate`: the name id a being displays, one `INT32`. Beings
/// only; the AoI cascade sends `onBeingNameUpdate` instead for props.
pub(super) fn being_name_id_update(args: &[u8]) -> Result<S2CEvent, SemanticError> {
    let mut r = Reader::new("onBeingNameIDUpdate", args);
    let event = S2CEvent::BeingNameIdUpdate { name_id: r.i32()? };
    r.finish()?;
    Ok(event)
}

/// `onPlayerCommunication`: a chat line. The two `UINT8`s sit between the
/// two `WSTRING`s, so the reads must follow that order exactly.
pub(super) fn player_communication(args: &[u8]) -> Result<S2CEvent, SemanticError> {
    let mut r = Reader::new("onPlayerCommunication", args);
    let event = S2CEvent::PlayerCommunication {
        speaker: r.wstring()?,
        speaker_flags: r.u8()?,
        channel: r.u8()?,
        text: r.wstring()?,
    };
    r.finish()?;
    Ok(event)
}

#[cfg(test)]
mod tests {
    use super::{
        being_name_id_update, entity_property, player_communication, static_mesh_name_update,
        visible,
    };
    use crate::calls::write_wstring;
    use crate::semantic::{S2CEvent, SemanticError};
    use crate::tap_fixture::TapCapture;

    /// Decodes fixture record `i` as a chat line and checks the speaker,
    /// flags, channel and text against the record's own `decoded` JSON, which
    /// the tap wrote from the server's decoder.
    fn chat_at(cap: &TapCapture, i: usize) -> S2CEvent {
        let rec = cap.expect(i, "onPlayerCommunication");
        let event = player_communication(&rec.args()).unwrap();
        let S2CEvent::PlayerCommunication {
            speaker,
            speaker_flags,
            channel,
            text,
        } = &event
        else {
            panic!("record #{i} did not decode to a chat line");
        };
        let json = rec.decoded.as_ref().unwrap();
        assert_eq!(json["Speaker"], speaker.as_str());
        assert_eq!(json["SpeakerFlags"], *speaker_flags);
        assert_eq!(json["Channel"], *channel);
        assert_eq!(json["Text"], text.as_str());
        event
    }

    #[test]
    fn player_communication_matches_the_capture() {
        let cap = TapCapture::praxis_start();
        assert_eq!(
            chat_at(&cap, 5),
            S2CEvent::PlayerCommunication {
                speaker: "SYSTEM".to_string(),
                speaker_flags: 0,
                channel: 9,
                text: "gmGotoXYZ: teleported to (-325, 73.6, -212.8)".to_string(),
            }
        );
        assert_eq!(
            chat_at(&cap, 50),
            S2CEvent::PlayerCommunication {
                speaker: "SYSTEM".to_string(),
                speaker_flags: 0,
                channel: 9,
                text: "You have learned Pistol Shot.".to_string(),
            }
        );
    }

    /// Flags and channel are both zero or 9 in the capture, so swapping them
    /// would not change the fixture result; distinct hand values pin the order.
    #[test]
    fn player_communication_reads_flags_and_channel_in_wire_order() {
        let mut args = Vec::new();
        write_wstring(&mut args, "A");
        args.extend_from_slice(&[2, 5]);
        write_wstring(&mut args, "B");
        assert_eq!(
            player_communication(&args),
            Ok(S2CEvent::PlayerCommunication {
                speaker: "A".to_string(),
                speaker_flags: 2,
                channel: 5,
                text: "B".to_string(),
            })
        );
    }

    #[test]
    fn entity_property_matches_the_capture() {
        let cap = TapCapture::praxis_start();
        let rec = cap.expect(73, "onEntityProperty");
        assert_eq!(
            entity_property(&rec.args()),
            Ok(S2CEvent::EntityProperty {
                property: 3,
                value: 1,
            })
        );
        let json = rec.decoded.as_ref().unwrap();
        assert_eq!(json["type"], 3);
        assert_eq!(json["value"], 1);
    }

    #[test]
    fn static_mesh_name_update_decodes_frosts_corpse() {
        // Template 14's row, written in the order `append_appearance` uses:
        // the static mesh first, then the body set.
        let mut args = Vec::new();
        write_wstring(&mut args, "CA-Props.CA-PrisonerCorpse00");
        write_wstring(&mut args, "GLB_Components.WorldObject_Small");
        assert_eq!(
            static_mesh_name_update(&args),
            Ok(S2CEvent::StaticMeshNameUpdate {
                static_mesh: "CA-Props.CA-PrisonerCorpse00".to_string(),
                body_set: "GLB_Components.WorldObject_Small".to_string(),
            })
        );
    }

    #[test]
    fn visible_and_being_name_id_decode() {
        assert_eq!(visible(&[1]), Ok(S2CEvent::Visible { visible: 1 }));
        // 7031 is 0x1B77, so the little-endian bytes are 77 1B 00 00.
        assert_eq!(
            being_name_id_update(&[0x77, 0x1B, 0x00, 0x00]),
            Ok(S2CEvent::BeingNameIdUpdate { name_id: 7031 })
        );
    }

    #[test]
    fn visible_rejects_a_trailing_byte() {
        assert_eq!(
            visible(&[1, 0]),
            Err(SemanticError::Trailing {
                method: "onVisible",
                extra: 1,
            })
        );
    }

    #[test]
    fn being_name_id_update_rejects_a_short_body() {
        assert_eq!(
            being_name_id_update(&[0x77, 0x1B, 0x00]),
            Err(SemanticError::Short {
                method: "onBeingNameIDUpdate",
                at: 0,
                need: 4,
                len: 3,
            })
        );
    }
}
