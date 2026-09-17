//! Packet P03 regression suite: `.stats` (new) plus verification of the six
//! pre-existing granular stat readouts (`.primarystats`, `.speedstats`,
//! `.armorstats`, `.qrstats`, `.absorbstats`, `.stealthstats`).
//!
//! Filter prefix: `legacy_p03_`.

use tokio::sync::mpsc;

use super::super::stats;
use super::{decode_feedback, setup};
use crate::cell::console::exec;
use cimmeria_content_engine::chain::ChainEngine;

/// (command, exact expected feedback lines in order) against a freshly
/// spawned target's default `StatList::new()` values — pins both the field
/// *set*/*order* and the actual cur/max readout. A dropped, added, reordered,
/// or mis-valued field breaks the exact `Vec` equality.
fn expected_lines(cmd: &str) -> Vec<&'static str> {
    match cmd {
        "stats" => vec![
            "health: 100/100",
            "focus: 0/0",
            "healthRegen: 0/0",
            "focusRegen: 0/0",
        ],
        "primarystats" => vec![
            "coordination: 1/1",
            "engagement: 1/1",
            "fortitude: 1/1",
            "morale: 1/1",
            "perception: 1/1",
            "intelligence: 1/1",
        ],
        "speedstats" => vec![
            "movementSpeedMod: 100/500",
            "rotationSpeedMod: 100/500",
            "speedReload: 0/0",
            "speedGrenade: 0/0",
            "speedDeploy: 0/0",
            "speedAttack: 0/0",
        ],
        "armorstats" => vec![
            "physicalAF: 0/50000",
            "energyAF: 0/50000",
            "hazmatAF: 0/50000",
            "psionicAF: 0/50000",
            "kineticRes: 0/2000",
            "mentalRes: 0/2000",
            "healthRes: 0/2000",
            "interruptRes: 0/0",
        ],
        "qrstats" => vec![
            "accuracy: 0/1000",
            "defense: 0/0",
            "qrMod: 0/0",
            "coverQRModifier: 0/0",
            "response: 0/0",
            "damage: 0/100",
            "penetration: 0/100",
            "tracking: 0/0",
            "stabilization: 0/0",
            "awareness: 0/0",
            "coverAccuracy: 0/0",
            "coverDefense: 0/0",
            "crouchingAccuracy: 0/0",
            "crouchingDefense: 0/0",
            "negation: 0/0",
            "mitigation: 0/0",
            "recovery: 0/0",
            "restoration: 0/0",
            "subtlety: 0/0",
        ],
        "absorbstats" => vec![
            "absorbPhysical: 0/1000",
            "absorbEnergy: 0/1000",
            "absorbHazmat: 0/1000",
            "absorbPsionic: 0/1000",
            "absorbUntyped: 0/1000",
            "absorbPhysicalItem: 0/1000",
            "absorbEnergyItem: 0/1000",
            "absorbHazmatItem: 0/1000",
            "absorbPsionicItem: 0/1000",
            "absorbUntypedItem: 0/1000",
            "absorbPhysicalEnergy: 0/1000",
            "absorbEnergyEnergy: 0/1000",
            "absorbHazmatEnergy: 0/1000",
            "absorbPsionicEnergy: 0/1000",
            "absorbUntypedEnergy: 0/1000",
        ],
        "stealthstats" => vec![
            "stealthRating: 0/100",
            "stealthMovement: 0/0",
            "revealRating: 0/100",
            "disguiseRating: 0/500",
            "disguiseDetection: 0/0",
        ],
        other => panic!("no expected-lines fixture for {other}"),
    }
}

/// Pins the exact field set/order/values for every one of the 7 stat-dump
/// groups (the new `.stats` plus the six pre-existing groups) against a
/// target's default stat block. Any drop, add, reorder, or wrong-value
/// mutation to `stats.rs::stat_set` breaks this.
#[tokio::test]
async fn legacy_p03_all_groups_report_exact_field_sets() {
    for cmd in [
        "stats",
        "primarystats",
        "speedstats",
        "armorstats",
        "qrstats",
        "absorbstats",
        "stealthstats",
    ] {
        let (mut mgr, gm, npc) = setup();
        let engine = ChainEngine::new();
        let (tx, mut rx) = mpsc::channel(64);
        exec(cmd, gm, &[], Some(npc), &tx, &mut mgr, &engine).await;

        let mut lines = Vec::new();
        while let Ok(msg) = rx.try_recv() {
            if let Some(text) = decode_feedback(&msg) {
                lines.push(text);
            }
        }
        assert_eq!(
            lines.first().map(String::as_str),
            Some(format!("{cmd} [{npc}]:").as_str()),
            "{cmd}: header line must name the command and resolved target"
        );
        // Each stat line is 4-space indented (`format_stat_line`'s
        // `"    {label}: {cur}/{max}"`); strip it so `expected_lines`' table
        // doesn't have to repeat that indentation 60+ times.
        let body: Vec<&str> = lines[1..]
            .iter()
            .map(|l| l.trim_start_matches("    "))
            .collect();
        assert_eq!(
            body,
            expected_lines(cmd),
            "{cmd}: field set/order/values must match the legacy statIds list exactly"
        );
    }
}

/// A GM with a selection sees the TARGET's stats, not their own — even when
/// the two entities' values genuinely differ. Mutates the NPC's HEALTH stat
/// away from the GM's own (unmodified) default so the assertion can't pass
/// by coincidence.
#[tokio::test]
async fn legacy_p03_stats_reports_selected_targets_values_not_callers() {
    let (mut mgr, gm, npc) = setup();
    if let Some(e) = mgr.get_entity_mut(npc) {
        e.stats.get_mut(cimmeria_entity::stats::HEALTH).unwrap().cur = 55;
        e.stats.get_mut(cimmeria_entity::stats::HEALTH).unwrap().max = 200;
    }
    // Sanity: the GM's own HEALTH is still the untouched default (100/100),
    // distinct from the NPC's mutated value, so a caller/target mixup would
    // be caught below.
    assert_eq!(
        mgr.get_entity(gm)
            .unwrap()
            .stats
            .get(cimmeria_entity::stats::HEALTH)
            .map(|s| (s.cur, s.max)),
        Some((100, 100))
    );

    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(16);
    exec("stats", gm, &[], Some(npc), &tx, &mut mgr, &engine).await;

    let mut lines = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let Some(text) = decode_feedback(&msg) {
            lines.push(text);
        }
    }
    assert!(
        lines.iter().any(|l| l == "    health: 55/200"),
        ".stats must report the selected target's HEALTH, not the caller's: {lines:?}"
    );
    assert!(
        !lines.iter().any(|l| l == "    health: 100/100"),
        ".stats must NOT report the caller's own HEALTH when a target is selected: {lines:?}"
    );
}

/// Every stat-dump feedback line is delivered to the caller only — the
/// `EntityMethodCall::entity_id` recipient is always the GM, never the
/// selected target (which receives no UI change at all; these commands are
/// entirely read-only for the target's client).
#[tokio::test]
async fn legacy_p03_stats_feedback_is_caller_only() {
    let (mut mgr, gm, npc) = setup();
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(64);
    exec("qrstats", gm, &[], Some(npc), &tx, &mut mgr, &engine).await;

    let mut saw_any = false;
    while let Ok(msg) = rx.try_recv() {
        let crate::cell::messages::CellToBaseMsg::EntityMethodCall { entity_id, .. } = msg else {
            continue;
        };
        saw_any = true;
        assert_eq!(
            entity_id, gm,
            "stat-dump feedback must route to the caller ({gm}), never the target ({npc})"
        );
    }
    assert!(saw_any, "expected at least one feedback line from .qrstats");
}

/// `stats::show` reports "no entity." rather than panicking or fabricating
/// zeros when the resolved target id no longer exists in the space (e.g. it
/// despawned between selection and dispatch).
#[tokio::test]
async fn legacy_p03_stats_on_vanished_target_reports_no_entity() {
    let (mut mgr, gm, npc) = setup();
    mgr.destroy_entity(npc);
    let (tx, mut rx) = mpsc::channel(16);
    // Call `stats::show` directly with the now-stale id, bypassing
    // `resolve_target` (which would normally reject a dead selection before
    // `exec` is ever reached). This exercises `show`'s own defensive
    // "unknown entity" branch in isolation.
    stats::show("primarystats", gm, npc, &tx, &mut mgr).await;

    let mut saw = false;
    while let Ok(msg) = rx.try_recv() {
        if let Some(text) = decode_feedback(&msg) {
            if text == "primarystats: no entity." {
                saw = true;
            }
        }
    }
    assert!(saw, "a vanished target must report 'no entity.', not panic");
}
