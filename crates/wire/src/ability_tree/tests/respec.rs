//! Byte-exact respec rejection payload and the pinned price (AT-08).

use super::super::*;

#[test]
fn respec_error_code_is_system_0_instance_0_then_code_little_endian() {
    assert_eq!(
        respec_error_code_args(RESPEC_FEEDBACK_NOT_AT_TRAINER),
        vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x2B, 0x00]
    );
    assert_eq!(
        respec_error_code_args(RESPEC_FEEDBACK_NOTHING_TRAINED),
        vec![0x00, 0x00, 0x00, 0x00, 0x00, 0xA7, 0x00]
    );
    assert_eq!(
        respec_error_code_args(RESPEC_FEEDBACK_NOT_ENOUGH_NAQUADAH),
        vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x23, 0x00]
    );
}

#[test]
fn feedback_codes_match_enumerations_xml() {
    // entities/defs/enumerations.xml, EConditionHandlerFeedback.
    assert_eq!(RESPEC_FEEDBACK_NOT_AT_TRAINER, 43, "OutsideDistanceCheck");
    assert_eq!(
        RESPEC_FEEDBACK_NOTHING_TRAINED, 167,
        "EntityDoesNotHaveAbility"
    );
    assert_eq!(RESPEC_FEEDBACK_NOT_ENOUGH_NAQUADAH, 35, "StatValueLessThan");
}

#[test]
fn respec_price_is_d_at10s_1000_naquadah() {
    assert_eq!(RESPEC_COST_NAQUADAH, 1000);
}
