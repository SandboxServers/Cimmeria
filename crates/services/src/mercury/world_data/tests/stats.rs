//! Tests for archetype stat tables and level-experience curves.

use super::super::*;

#[test]
fn level_exp_boundaries() {
    assert_eq!(super::super::stats::level_exp(0), 0);
    assert_eq!(super::super::stats::level_exp(1), 100);
    assert_eq!(super::super::stats::level_exp(10), 9000);
    assert_eq!(super::super::stats::level_exp(20), 400000);
    assert_eq!(super::super::stats::level_exp(99), 400000); // clamped
}

#[test]
fn archetype_stats_commando_differs() {
    let soldier = archetype_stats(1);
    let commando = archetype_stats(2);
    assert_eq!(soldier.coordination, 5);
    assert_eq!(commando.coordination, 4);
    assert_eq!(commando.perception, 5);
}
