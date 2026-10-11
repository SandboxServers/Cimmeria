//! The dialog family: `onDialogDisplay` (105, player) and `InteractionType`
//! (3, shared). Both bodies are fixed width, so each decoder reads its fields
//! in wire order and then insists on the end of the body.

use super::primitives::Reader;
use super::{DialogDisplay, S2CEvent, SemanticError};

/// `onDialogDisplay`: the dialog an entity opens for the player.
pub(super) fn dialog_display(args: &[u8]) -> Result<S2CEvent, SemanticError> {
    let mut r = Reader::new("onDialogDisplay", args);
    let event = DialogDisplay {
        entity_id: r.i32()?,
        dialog_id: r.i32()?,
        mission_flags: r.i32()?,
        is_immediate: r.u8()?,
        mission_id: r.i32()?,
    };
    r.finish()?;
    Ok(S2CEvent::DialogDisplay(event))
}

/// `InteractionType`: the interaction an entity offers. The body is a single
/// `UINT64`, so a 4-byte body is a short read, not a smaller type.
pub(super) fn interaction_type(args: &[u8]) -> Result<S2CEvent, SemanticError> {
    let mut r = Reader::new("InteractionType", args);
    let type_id = r.u64()?;
    r.finish()?;
    Ok(S2CEvent::InteractionType { type_id })
}

#[cfg(test)]
mod tests {
    use super::{dialog_display, interaction_type};
    use crate::semantic::{DialogDisplay, S2CEvent, SemanticError};
    use crate::tap_fixture::TapCapture;

    /// Decodes fixture record `i` and checks each field against the record's
    /// own `decoded` JSON, which the tap wrote from the server's decoder.
    fn dialog_at(cap: &TapCapture, i: usize) -> DialogDisplay {
        let rec = cap.expect(i, "onDialogDisplay");
        let S2CEvent::DialogDisplay(d) = dialog_display(&rec.args()).unwrap() else {
            panic!("record #{i} did not decode to a dialog");
        };
        let json = rec.decoded.as_ref().unwrap();
        assert_eq!(json["entity_id"], d.entity_id);
        assert_eq!(json["dialog_id"], d.dialog_id);
        assert_eq!(json["is_immediate"], d.is_immediate);
        assert_eq!(json["mission_flags"], d.mission_flags);
        assert_eq!(json["mission_id"], d.mission_id);
        d
    }

    /// Decodes fixture record `i` as an interaction and checks it against the
    /// record's `decoded.TypeId`.
    fn interaction_at(cap: &TapCapture, i: usize) -> u64 {
        let rec = cap.expect(i, "InteractionType");
        let S2CEvent::InteractionType { type_id } = interaction_type(&rec.args()).unwrap() else {
            panic!("record #{i} did not decode to an interaction");
        };
        assert_eq!(rec.decoded.as_ref().unwrap()["TypeId"], type_id);
        type_id
    }

    #[test]
    fn dialog_display_matches_the_capture() {
        let cap = TapCapture::praxis_start();
        assert_eq!(
            dialog_at(&cap, 20),
            DialogDisplay {
                entity_id: 100751,
                dialog_id: 3995,
                mission_flags: 0,
                is_immediate: 1,
                mission_id: 0,
            }
        );
        assert_eq!(
            dialog_at(&cap, 43),
            DialogDisplay {
                entity_id: 100748,
                dialog_id: 3996,
                mission_flags: 0,
                is_immediate: 1,
                mission_id: 0,
            }
        );
        assert_eq!(
            dialog_at(&cap, 54),
            DialogDisplay {
                entity_id: 8,
                dialog_id: 5882,
                mission_flags: 0,
                is_immediate: 1,
                mission_id: 0,
            }
        );
    }

    #[test]
    fn interaction_type_matches_the_capture() {
        let cap = TapCapture::praxis_start();
        assert_eq!(interaction_at(&cap, 21), 0);
        assert_eq!(interaction_at(&cap, 22), 0x4000_0000);
        assert_eq!(interaction_at(&cap, 44), 0);
    }

    #[test]
    fn dialog_display_rejects_a_trailing_byte() {
        let cap = TapCapture::praxis_start();
        let mut args = cap.expect(20, "onDialogDisplay").args();
        args.push(0);
        assert_eq!(
            dialog_display(&args),
            Err(SemanticError::Trailing {
                method: "onDialogDisplay",
                extra: 1,
            })
        );
    }

    #[test]
    fn dialog_display_rejects_a_short_body() {
        // Dropping the last byte leaves the final i32 (mission_id) incomplete.
        let cap = TapCapture::praxis_start();
        let mut args = cap.expect(20, "onDialogDisplay").args();
        args.pop();
        assert!(matches!(
            dialog_display(&args),
            Err(SemanticError::Short {
                method: "onDialogDisplay",
                ..
            })
        ));
    }
}
