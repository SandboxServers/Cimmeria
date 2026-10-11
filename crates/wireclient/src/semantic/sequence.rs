//! The sequence family: `onSequence` (1, shared), `onKnownAbilitiesUpdate`
//! (101) and `onPlayMovie` (155), the last two player-only.

use super::primitives::Reader;
use super::{S2CEvent, SemanticError, Sequence};

/// `onSequence`: a kismet sequence the server asks the client to run. The
/// trailing `InstanceId` is read here even though the tap's `decoded` JSON
/// leaves it out: `cimmeria-wire-log` ignores it (README F10), so the fixture
/// cannot be the only check on the body's length.
pub(super) fn sequence(args: &[u8]) -> Result<S2CEvent, SemanticError> {
    let mut r = Reader::new("onSequence", args);
    let kismet_event_set_seq_id = r.i32()?;
    let source_id = r.i32()?;
    let target_id = r.i32()?;
    let primary_target = r.i8()?;
    let impact_time = r.f32()?;
    // The pair layout is not needed yet. Refusing a non-empty array here keeps
    // the bytes after it from being read as the fields that follow.
    let nvp_count = r.u32()?;
    if nvp_count != 0 {
        return Err(SemanticError::Unsupported {
            method: "onSequence",
            what: "NameValuePairs",
        });
    }
    let view_type = r.i8()?;
    let instance_id = r.i32()?;
    r.finish()?;
    Ok(S2CEvent::Sequence(Sequence {
        kismet_event_set_seq_id,
        source_id,
        target_id,
        primary_target,
        impact_time,
        nvp_count,
        view_type,
        instance_id,
    }))
}

/// `onKnownAbilitiesUpdate`: the ability ids the player may use. The order is
/// kept because the client shows them in the order the server sends them.
pub(super) fn known_abilities_update(args: &[u8]) -> Result<S2CEvent, SemanticError> {
    let mut r = Reader::new("onKnownAbilitiesUpdate", args);
    let ability_ids = r.array_i32()?;
    r.finish()?;
    Ok(S2CEvent::KnownAbilitiesUpdate { ability_ids })
}

/// `onPlayMovie`: a cinematic the client plays, by its asset name.
pub(super) fn play_movie(args: &[u8]) -> Result<S2CEvent, SemanticError> {
    let mut r = Reader::new("onPlayMovie", args);
    let movie_name = r.wstring()?;
    let full_screen = r.u8()?;
    r.finish()?;
    Ok(S2CEvent::PlayMovie {
        movie_name,
        full_screen,
    })
}

#[cfg(test)]
mod tests {
    use super::{known_abilities_update, play_movie, sequence};
    use crate::semantic::{S2CEvent, SemanticError, Sequence};
    use crate::tap_fixture::TapCapture;

    /// Decodes fixture record `i` and checks each field against the record's
    /// `decoded` JSON. The JSON has no `InstanceId`, so that field is only
    /// pinned by the caller's expected struct.
    fn sequence_at(cap: &TapCapture, i: usize) -> Sequence {
        let rec = cap.expect(i, "onSequence");
        let S2CEvent::Sequence(s) = sequence(&rec.args()).unwrap() else {
            panic!("record #{i} did not decode to a sequence");
        };
        let json = rec.decoded.as_ref().unwrap();
        assert_eq!(json["kismet_event_set_seq_id"], s.kismet_event_set_seq_id);
        assert_eq!(json["source_id"], s.source_id);
        assert_eq!(json["target_id"], s.target_id);
        assert_eq!(json["primary_target"], s.primary_target);
        assert_eq!(json["impact_time"], s.impact_time);
        assert_eq!(json["nvp_count"], s.nvp_count);
        assert_eq!(json["view_type"], s.view_type);
        s
    }

    /// Decodes fixture record `i` as an ability list and checks it against the
    /// record's `decoded.AbilityData.items`.
    fn abilities_at(cap: &TapCapture, i: usize) -> Vec<i32> {
        let rec = cap.expect(i, "onKnownAbilitiesUpdate");
        let S2CEvent::KnownAbilitiesUpdate { ability_ids } =
            known_abilities_update(&rec.args()).unwrap()
        else {
            panic!("record #{i} did not decode to an ability list");
        };
        assert_eq!(
            rec.decoded.as_ref().unwrap()["AbilityData"]["items"],
            serde_json::json!(ability_ids)
        );
        ability_ids
    }

    #[test]
    fn sequence_matches_the_capture() {
        let cap = TapCapture::praxis_start();
        assert_eq!(
            sequence_at(&cap, 66),
            Sequence {
                kismet_event_set_seq_id: 10000,
                source_id: 8,
                target_id: 8,
                primary_target: 1,
                impact_time: 0.0,
                nvp_count: 0,
                view_type: 0,
                instance_id: 0,
            }
        );
        assert_eq!(sequence_at(&cap, 72).kismet_event_set_seq_id, 1872);
        assert_eq!(sequence_at(&cap, 77).kismet_event_set_seq_id, 1873);
    }

    /// The capture has source equal to target and zero impact, count, view and
    /// instance, so a swap among those would pass the fixture test. Distinct
    /// hand values pin the wire order.
    #[test]
    fn sequence_reads_fields_in_wire_order() {
        let args: [u8; 26] = [
            7, 0, 0, 0, // KismetEventSetSeqID
            11, 0, 0, 0, // SourceID
            13, 0, 0, 0,    // TargetID
            0xFE, // PrimaryTarget (-2)
            0x00, 0x00, 0xC0, 0x3F, // ImpactTime 1.5
            0, 0, 0, 0, // NameValuePair count
            3, // ViewType
            17, 0, 0, 0, // InstanceId
        ];
        assert_eq!(
            sequence(&args),
            Ok(S2CEvent::Sequence(Sequence {
                kismet_event_set_seq_id: 7,
                source_id: 11,
                target_id: 13,
                primary_target: -2,
                impact_time: 1.5,
                nvp_count: 0,
                view_type: 3,
                instance_id: 17,
            }))
        );
    }

    #[test]
    fn sequence_with_name_value_pairs_is_unsupported() {
        // Fixture #66 with its NameValuePair count set to 1. The count is the
        // u32 at offset 17, after 4+4+4+1+4 bytes of fixed fields.
        let cap = TapCapture::praxis_start();
        let mut args = cap.expect(66, "onSequence").args();
        args[17] = 1;
        assert_eq!(
            sequence(&args),
            Err(SemanticError::Unsupported {
                method: "onSequence",
                what: "NameValuePairs",
            })
        );
    }

    #[test]
    fn sequence_rejects_a_missing_instance_id() {
        // The fixture's JSON stops before InstanceId, so this is the check that
        // the decoder reads the field the wire-log decoder drops (README F10).
        let cap = TapCapture::praxis_start();
        let mut args = cap.expect(66, "onSequence").args();
        args.truncate(22);
        assert_eq!(
            sequence(&args),
            Err(SemanticError::Short {
                method: "onSequence",
                at: 22,
                need: 4,
                len: 22,
            })
        );
    }

    #[test]
    fn known_abilities_match_the_capture() {
        let cap = TapCapture::praxis_start();
        assert_eq!(abilities_at(&cap, 49), vec![597, 1218, 592, 594]);
        assert_eq!(abilities_at(&cap, 71), vec![579, 597, 1218, 592, 594, 708]);
    }

    #[test]
    fn play_movie_decodes_the_first_login_movie() {
        // Count 20, then "Cine-SGWLogo.SGWLogo" as UTF-16LE, then FullScreen 1.
        let args: [u8; 45] = [
            0x14, 0x00, 0x00, 0x00, 0x43, 0x00, 0x69, 0x00, 0x6E, 0x00, 0x65, 0x00, 0x2D, 0x00,
            0x53, 0x00, 0x47, 0x00, 0x57, 0x00, 0x4C, 0x00, 0x6F, 0x00, 0x67, 0x00, 0x6F, 0x00,
            0x2E, 0x00, 0x53, 0x00, 0x47, 0x00, 0x57, 0x00, 0x4C, 0x00, 0x6F, 0x00, 0x67, 0x00,
            0x6F, 0x00, 0x01,
        ];
        assert_eq!(
            play_movie(&args),
            Ok(S2CEvent::PlayMovie {
                movie_name: "Cine-SGWLogo.SGWLogo".to_string(),
                full_screen: 1,
            })
        );
    }
}
