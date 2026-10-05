use super::*;

const WINE: RoundTrip = RoundTrip {
    written: 4,
    read: 0,
    intact: false,
    put_span: Some(4),
    mark_at_start: Some(true),
};

#[test]
fn the_mark_is_raised_only_when_writes_have_passed_it() {
    assert_eq!(raised(0x5000, 0x5004), Some(0x5004));
    assert_eq!(raised(0x5004, 0x5004), None, "already there");
    assert_eq!(raised(0x5010, 0x5004), None, "past it, after a seek");
    assert_eq!(raised(0x5000, 0), None, "no put area");
    assert_eq!(raised(0, 0x5004), Some(0x5004), "a mark never set");
}

#[test]
fn a_round_trip_is_judged_by_what_came_back_and_how_the_object_looks() {
    assert_eq!(verdict(&WINE), Verdict::LosesWrites);
    let healthy = RoundTrip {
        read: 4,
        intact: true,
        ..WINE
    };
    assert_eq!(verdict(&healthy), Verdict::Healthy);
    // Four bytes back that are not the four written is the same fault
    // to the caller; the second probe decides whether the shim helped.
    let garbled = RoundTrip { read: 4, ..WINE };
    assert_eq!(verdict(&garbled), Verdict::LosesWrites);
    // A short read from an object that is not laid out as expected is
    // not this fault: do not write into it.
    for odd in [
        RoundTrip {
            put_span: None,
            ..WINE
        },
        RoundTrip {
            put_span: Some(32),
            ..WINE
        },
        RoundTrip {
            mark_at_start: Some(false),
            ..WINE
        },
        RoundTrip {
            mark_at_start: None,
            ..WINE
        },
    ] {
        assert_eq!(verdict(&odd), Verdict::Unrecognised, "{odd:?}");
    }
    let unwritten = RoundTrip { written: 0, ..WINE };
    assert_eq!(verdict(&unwritten), Verdict::WriteFailed);
}

#[test]
fn the_slot_is_the_only_one_holding_the_export() {
    // The pinned runtime's vtable: underflow is the fifth entry.
    let slots = [
        0x1007_4450u32,
        0x1007_35c0,
        0x1007_3ac0,
        0x1004_9900,
        0x1007_4250,
        0x1004_a090,
    ];
    assert_eq!(find_slot(&slots, 0x1007_4250), Some(4));
    assert_eq!(find_slot(&slots, 0x1234_5678), None, "absent");
    assert_eq!(find_slot(&[7, 9, 7], 7), None, "ambiguous");
    assert_eq!(find_slot(&[0, 0], 0), None, "a null export");
    assert_eq!(find_slot(&[], 7), None);
}

#[test]
fn every_outcome_says_what_was_done_to_the_runtime() {
    let repaired = Outcome::Repaired {
        before: WINE,
        slot: 4,
    };
    assert!(repaired.installed());
    let line = describe(&repaired);
    assert!(line.contains("wrote 4, read 0"), "{line}");
    assert!(line.contains("vtable slot 4"), "{line}");
    assert!(line.contains("now reads 4"), "{line}");

    for (outcome, needle) in [
        (Outcome::NotWine, "left alone"),
        (
            Outcome::RuntimeIncomplete("msvcp80.dll is not loaded"),
            "msvcp80.dll is not loaded; the C++ runtime is left alone",
        ),
        (Outcome::ProbeFaulted, "left alone"),
        (Outcome::Healthy, "no repair needed"),
        (
            Outcome::NotRepairable(Verdict::Unrecognised, WINE),
            "left alone",
        ),
        (
            Outcome::SlotUnavailable("was not found in the vtable"),
            "not repaired",
        ),
        (Outcome::StillBroken(WINE), "slot restored, not repaired"),
    ] {
        assert!(!outcome.installed(), "{outcome:?}");
        let line = describe(&outcome);
        assert!(line.starts_with("strstream: "), "{line}");
        assert!(line.contains(needle), "{line}");
    }
}
