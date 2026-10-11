//! The mission family: `onMissionUpdate` (80), `onStepUpdate` (81) and
//! `onObjectiveUpdate` (82), all player-only. Each body is fixed width, so
//! each decoder reads its fields in wire order and then insists on the end.

use super::primitives::Reader;
use super::{S2CEvent, SemanticError};

/// `onMissionUpdate`: a mission changed state. `Status` is an INT8 on the
/// wire; reading it wider would pull the giver's first bytes into it.
pub(super) fn mission_update(args: &[u8]) -> Result<S2CEvent, SemanticError> {
    let mut r = Reader::new("onMissionUpdate", args);
    let event = S2CEvent::MissionUpdate {
        mission_id: r.i32()?,
        status: r.i8()?,
        giver_name_id: r.i32()?,
    };
    r.finish()?;
    Ok(event)
}

/// `onStepUpdate`: one step inside a mission changed state.
pub(super) fn step_update(args: &[u8]) -> Result<S2CEvent, SemanticError> {
    let mut r = Reader::new("onStepUpdate", args);
    let event = S2CEvent::StepUpdate {
        step_id: r.i32()?,
        status: r.i8()?,
    };
    r.finish()?;
    Ok(event)
}

/// `onObjectiveUpdate`: an objective changed state. The three flags are one
/// byte each and follow the wire order Status, Hidden, Optional.
pub(super) fn objective_update(args: &[u8]) -> Result<S2CEvent, SemanticError> {
    let mut r = Reader::new("onObjectiveUpdate", args);
    let event = S2CEvent::ObjectiveUpdate {
        objective_id: r.i32()?,
        status: r.i8()?,
        hidden: r.i8()?,
        optional: r.i8()?,
    };
    r.finish()?;
    Ok(event)
}

#[cfg(test)]
mod tests {
    use super::{mission_update, objective_update, step_update};
    use crate::semantic::{S2CEvent, SemanticError};
    use crate::tap_fixture::TapCapture;

    /// Decodes fixture record `i` as a mission update and checks each field
    /// against the record's own `decoded` JSON.
    fn mission_at(cap: &TapCapture, i: usize) -> S2CEvent {
        let rec = cap.expect(i, "onMissionUpdate");
        let event = mission_update(&rec.args()).unwrap();
        let json = rec.decoded.as_ref().unwrap();
        let S2CEvent::MissionUpdate {
            mission_id,
            status,
            giver_name_id,
        } = &event
        else {
            panic!("record #{i} did not decode to a mission update");
        };
        assert_eq!(json["MissionID"], *mission_id);
        assert_eq!(json["Status"], *status);
        assert_eq!(json["MissionGiverName"], *giver_name_id);
        event
    }

    /// Decodes fixture record `i` as a step update and checks it against the
    /// record's `decoded` JSON.
    fn step_at(cap: &TapCapture, i: usize) -> S2CEvent {
        let rec = cap.expect(i, "onStepUpdate");
        let event = step_update(&rec.args()).unwrap();
        let json = rec.decoded.as_ref().unwrap();
        let S2CEvent::StepUpdate { step_id, status } = &event else {
            panic!("record #{i} did not decode to a step update");
        };
        assert_eq!(json["StepID"], *step_id);
        assert_eq!(json["Status"], *status);
        event
    }

    /// Decodes fixture record `i` as an objective update and checks it against
    /// the record's `decoded` JSON.
    fn objective_at(cap: &TapCapture, i: usize) -> S2CEvent {
        let rec = cap.expect(i, "onObjectiveUpdate");
        let event = objective_update(&rec.args()).unwrap();
        let json = rec.decoded.as_ref().unwrap();
        let S2CEvent::ObjectiveUpdate {
            objective_id,
            status,
            hidden,
            optional,
        } = &event
        else {
            panic!("record #{i} did not decode to an objective update");
        };
        assert_eq!(json["ObjectiveID"], *objective_id);
        assert_eq!(json["Status"], *status);
        assert_eq!(json["Hidden"], *hidden);
        assert_eq!(json["Optional"], *optional);
        event
    }

    #[test]
    fn mission_update_matches_the_capture() {
        let cap = TapCapture::praxis_start();
        assert_eq!(
            mission_at(&cap, 28),
            S2CEvent::MissionUpdate {
                mission_id: 1360,
                status: 0,
                giver_name_id: 0,
            }
        );
        assert_eq!(
            mission_at(&cap, 69),
            S2CEvent::MissionUpdate {
                mission_id: 622,
                status: 1,
                giver_name_id: 0,
            }
        );
    }

    #[test]
    fn step_update_matches_the_capture() {
        let cap = TapCapture::praxis_start();
        let cases = [
            (25, 2113, 1),
            (26, 80623, 0),
            (29, 4037, 0),
            (46, 80623, 1),
            (47, 80622, 0),
            (68, 80622, 1),
        ];
        for (i, step_id, status) in cases {
            assert_eq!(
                step_at(&cap, i),
                S2CEvent::StepUpdate { step_id, status },
                "record #{i}"
            );
        }
    }

    #[test]
    fn objective_update_matches_the_capture() {
        let cap = TapCapture::praxis_start();
        assert_eq!(
            objective_at(&cap, 23),
            S2CEvent::ObjectiveUpdate {
                objective_id: 3238,
                status: 1,
                hidden: 1,
                optional: 1,
            }
        );
        assert_eq!(
            objective_at(&cap, 24),
            S2CEvent::ObjectiveUpdate {
                objective_id: 2452,
                status: 1,
                hidden: 0,
                optional: 0,
            }
        );
        let cases = [
            (27, 90623, 0),
            (30, 4650, 0),
            (45, 90623, 1),
            (48, 90622, 0),
            (67, 90622, 1),
        ];
        for (i, objective_id, status) in cases {
            assert_eq!(
                objective_at(&cap, i),
                S2CEvent::ObjectiveUpdate {
                    objective_id,
                    status,
                    hidden: 0,
                    optional: 0,
                },
                "record #{i}"
            );
        }
    }

    /// Every capture record has Hidden equal to Optional, so a swap of those
    /// two flags passes the fixtures; distinct hand values pin the order.
    #[test]
    fn objective_update_reads_hidden_and_optional_in_wire_order() {
        let args = [7, 0, 0, 0, 3, 5, 9];
        assert_eq!(
            objective_update(&args),
            Ok(S2CEvent::ObjectiveUpdate {
                objective_id: 7,
                status: 3,
                hidden: 5,
                optional: 9,
            })
        );
    }

    /// The capture's givers are all zero, so a swap of the giver with the
    /// status would pass the fixtures; distinct hand values pin the order.
    #[test]
    fn mission_update_reads_giver_after_status_in_wire_order() {
        let args = [7, 0, 0, 0, 2, 9, 0, 0, 0];
        assert_eq!(
            mission_update(&args),
            Ok(S2CEvent::MissionUpdate {
                mission_id: 7,
                status: 2,
                giver_name_id: 9,
            })
        );
    }

    #[test]
    fn mission_update_status_is_one_byte() {
        let cap = TapCapture::praxis_start();
        let args = cap.expect(28, "onMissionUpdate").args();
        assert_eq!(args.len(), 9);
        assert!(mission_update(&args).is_ok());

        // The same body with Status widened to INT32: the giver is read three
        // bytes late and three bytes are left over.
        let mut wide = args[..5].to_vec();
        wide.extend_from_slice(&[0, 0, 0]);
        wide.extend_from_slice(&args[5..]);
        assert_eq!(wide.len(), 12);
        assert_eq!(
            mission_update(&wide),
            Err(SemanticError::Trailing {
                method: "onMissionUpdate",
                extra: 3,
            })
        );
    }
}
