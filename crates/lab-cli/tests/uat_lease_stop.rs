//! LX-18c1 (F-LX10): a lease wait that Ctrl+C cut short does not brake the lane.

mod uat_support;

use cimmeria_lab_cli::daemon::mcp::McpError;
use uat_support::rig::{LaneRig, Reply};
use uat_support::{ids, lane};

fn timeout() -> Reply {
    Err(McpError::Unreachable("operation timed out".to_string()))
}

#[test]
fn ctrl_c_during_the_lease_wait_does_not_call_the_lane_stuck() {
    let mut rig = LaneRig::new(vec![("default", vec![timeout()])]);
    rig.wait_result = false; // the wait gave up ...
    rig.stop_on_wait = true; // ... because Ctrl+C was pressed in it
    rig.run(&[lane(1, "default")], &ids(&["FS-P1"]), 3, 0, 0);
    assert_eq!(
        rig.calls.lock().unwrap().len(),
        1,
        "no new run after Ctrl+C"
    );
    assert_eq!(rig.waits.lock().unwrap().len(), 1);
    let records = rig.records();
    assert_eq!(records.len(), 1);
    assert!(!records[0].ok);
    assert!(!records[0].braked, "Ctrl+C is not a stuck lease");
    assert!(
        rig.notices.lock().unwrap().is_empty(),
        "no 'lease did not clear' notice"
    );
}

#[test]
fn a_lease_that_never_clears_still_stops_the_lane() {
    let mut rig = LaneRig::new(vec![("default", vec![timeout()])]);
    rig.wait_result = false;
    rig.run(&[lane(1, "default")], &ids(&["FS-P1"]), 3, 0, 0);
    assert_eq!(rig.calls.lock().unwrap().len(), 1);
    let records = rig.records();
    assert_eq!(records.len(), 1);
    assert!(records[0].braked);
    assert_eq!(
        rig.notices.lock().unwrap().as_slice(),
        [(
            "default".to_string(),
            "its lease did not clear after the timeout; lane stopped".to_string()
        )]
    );
}
