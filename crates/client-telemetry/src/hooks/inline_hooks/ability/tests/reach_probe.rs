//! The camera-method router probe (`ReachProbe`, D-PC19).

use super::super::route::test_access::{enter_route, leave_route, mark_reached};
use super::super::route::ReachProbe;
use super::*;

/// Drains the shared joiner, so a test reads only what its own probe noted.
fn drain() {
    with_joiner(|j| {
        j.tag_bundle(0xDEAD);
        j.take_bundle(0xDEAD);
    });
}

/// A reached camera call notes one tag with its send id and no press.
#[test]
fn reached_probe_notes_a_camera_tag() {
    let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    drain();
    let probe = ReachProbe::begin("interact");
    let id = probe.send_id();
    mark_reached();
    assert!(probe.finish(), "a reached call notes a tag");
    let tags = with_joiner(|j| {
        assert!(j.tag_bundle(0x1000));
        j.take_bundle(0x1000).unwrap()
    });
    assert_eq!(tags.len(), 1, "one camera tag per reached call");
    assert_eq!(tags[0].send_id, id);
    assert_eq!(tags[0].method, "interact");
    assert_eq!(tags[0].press_id, None, "a camera send has no press");
    assert_eq!(tags[0].ability_id, None, "a camera send has no ability");
}

/// A call that never reached a `start*Message` notes nothing.
#[test]
fn unreached_probe_notes_nothing() {
    let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    drain();
    let probe = ReachProbe::begin("useItem");
    assert!(!probe.finish(), "an unreached call notes nothing");
    let tagged = with_joiner(|j| j.tag_bundle(0x1000));
    assert!(!tagged, "the joiner holds no camera tag to claim");
}

/// A probe dropped without `finish` still restores the router state.
#[test]
fn probe_restores_the_thread_locals() {
    let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    drain();
    enter_route();
    mark_reached();
    drop(ReachProbe::begin("interact"));
    assert!(leave_route(), "the outer router state survives the probe");
}

/// Two camera calls never share a send id.
#[test]
fn send_ids_are_unique() {
    let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    drain();
    let a = ReachProbe::begin("interact");
    let b = ReachProbe::begin("useItem");
    assert_ne!(a.send_id(), b.send_id(), "each call gets its own id");
}
