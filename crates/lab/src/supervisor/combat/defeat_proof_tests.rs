use super::*;
use crate::proof::{EvVerdict, Evidence};

#[test]
fn respawn_needs_both_reads() {
    // Window closed but no health read: inconclusive, not a pass.
    let none = Evidence::judge(defeat_checks(true, None));
    assert_eq!(none.verdict, EvVerdict::Inconclusive);
    assert_eq!(none.why, Why::NoPostRead);

    // Closed and alive: confirmed.
    let both = Evidence::judge(defeat_checks(true, Some(true)));
    assert_eq!(both.verdict, EvVerdict::Confirmed);

    // Still open: denied, whatever the health read says.
    let open = Evidence::judge(defeat_checks(false, Some(true)));
    assert_eq!(open.verdict, EvVerdict::Denied);
}

#[test]
fn a_dead_player_after_the_window_closes_is_denied() {
    let dead = Evidence::judge(defeat_checks(true, Some(false)));
    assert_eq!(dead.verdict, EvVerdict::Denied);
    assert_eq!(dead.checks[1].why, None);
}
