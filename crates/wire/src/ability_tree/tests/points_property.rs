//! Byte-exact `onEntityProperty(GENERICPROPERTY_TrainingPoints, n)` payload.

use super::super::*;

#[test]
fn training_points_property_is_prop_id_1_then_value_little_endian() {
    assert_eq!(
        training_points_property_args(7),
        vec![0x01, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00, 0x00]
    );
    assert_eq!(
        training_points_property_args(0x0102_0304),
        vec![0x01, 0x00, 0x00, 0x00, 0x04, 0x03, 0x02, 0x01]
    );
}

#[test]
fn prop_id_matches_enumerations_xml() {
    // entities/defs/enumerations.xml: GENERICPROPERTY_TrainingPoints = 1.
    assert_eq!(GENERICPROPERTY_TRAINING_POINTS, 1);
}
