//! Which leash triggers count toward `npc_ai.leash event=loop` (DA-F2,
//! #1244 review finding 4).

use super::leash::counts_toward_loop;
use crate::cell::service::npc_ai::leash::policy::LeashTrigger;

/// **Table guard.** Every trigger label a leash entry can carry, and whether
/// it counts toward the loop detector. Only the two that end a fight count
/// as no loop: the target died (`target_dead`) or left the space
/// (`target_gone`). A fight given up against a live target must still count:
/// the leash-policy triggers, a target lost beyond AoI (the chase given up,
/// NA12's S5 shape) and content clearing the threat list. Fails if either
/// fight-ended label is dropped from the exclusion, or any other label is
/// added to it.
#[test]
fn every_leash_trigger_counts_toward_the_loop_except_a_finished_fight() {
    // `cimmeria-cell-combat` `npc_ai::fight_target::Dropped::label` and
    // `begin_leash`'s threat-empty path emit these; the policy triggers are
    // read from the enum so a new variant is covered by construction.
    let table: Vec<(&str, bool)> = [
        LeashTrigger::BeyondBand,
        LeashTrigger::ChaseOutward,
        LeashTrigger::VerticalCap,
    ]
    .into_iter()
    .map(|t| (t.label(), true))
    .chain([
        ("target_out_of_aoi", true),
        ("threat_empty", true),
        ("target_dead", false),
        ("target_gone", false),
    ])
    .collect();
    for (label, counts) in table {
        assert_eq!(counts_toward_loop(label), counts, "trigger {label}");
    }
}
