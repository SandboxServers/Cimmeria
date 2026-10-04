//! The interrupt resolver: a landed interrupt cancels the target's
//! channel, and each attempt rolls afresh (the per-request nonce), on a
//! channel and on one warmup alike.
//!
//! Entity 2 channels a `pulse_count = 0` effect on entity 1, or warms up a
//! cast. A channel has no warmup instance, and repeated hits on one warmup
//! share its instance, so before the nonce every attempt by one attacker
//! with one effect rolled the same number.

use std::collections::HashMap;
use std::time::Instant;

use tokio::sync::mpsc;

use cimmeria_common::Vector3;
use cimmeria_entity::abilities::EffectDef;
use cimmeria_entity::cell_entity::{ActiveEffectInstance, PendingCast};
use cimmeria_entity::stats::{COORDINATION, INTERRUPT_RES};

use super::interrupt::resolve_interrupts_for;
use crate::cell::effects::interrupt_request::{InterruptCause, InterruptRequest};
use crate::cell::space_manager::SpaceManager;

const CHANNEL_EFFECT: i32 = 7100;
const CHANNELLER: u32 = 2;

fn mgr(interrupt_res: i32) -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="W" Instanced="false" MinX="-100" MaxX="100" MinY="-100" MaxY="100" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces><Space WorldName="W" /></Spaces>"#)
        .unwrap();
    mgr.create_entity(1, "W", [0.0; 3], [0.0; 3]).unwrap();
    mgr.create_entity(CHANNELLER, "W", [5.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    mgr.effect_defs.insert(
        CHANNEL_EFFECT,
        EffectDef {
            effect_id: CHANNEL_EFFECT,
            ability_id: 1234,
            pulse_count: 0,
            pulse_duration: 0.5,
            params: HashMap::new(),
            ..Default::default()
        },
    );
    let c = mgr.get_entity_mut(CHANNELLER).unwrap();
    for (stat, value) in [(INTERRUPT_RES, interrupt_res), (COORDINATION, 0)] {
        if let Some(s) = c.stats.get_mut(stat) {
            s.update(0, value, value.max(0));
        }
    }
    mgr
}

fn start_channel(mgr: &mut SpaceManager) {
    mgr.get_entity_mut(1)
        .unwrap()
        .active_effects
        .push(ActiveEffectInstance {
            invoker_identity: Default::default(),
            cast_id: None,
            effect_id: CHANNEL_EFFECT,
            ability_id: 1234,
            invoker_id: CHANNELLER,
            remaining_pulses: 20,
            total_pulses: 20,
            next_pulse_at: Instant::now(),
            pulse_interval_secs: 0.5,
            invoker_position_at_register: None,
        });
}

fn channelling(mgr: &SpaceManager) -> bool {
    mgr.get_entity(1)
        .unwrap()
        .active_effects
        .iter()
        .any(|i| i.invoker_id == CHANNELLER)
}

/// One attempt by player 9's EMP round (effect 9120) at `chance_pct`.
async fn attempt(mgr: &mut SpaceManager, chance_pct: i32) -> bool {
    let (tx, _rx) = mpsc::channel(512);
    mgr.request_interrupt(InterruptRequest {
        cast_id: None,
        source_id: 9,
        target_id: CHANNELLER,
        effect_id: 9120,
        ability_id: 1445,
        chance_pct,
        cause: InterruptCause::Effect,
        nonce: 0,
    });
    resolve_interrupts_for(CHANNELLER, &tx, mgr).await;
    !channelling(mgr)
}

/// A certain interrupt cancels the target's channel.
#[tokio::test]
async fn an_interrupt_cancels_the_targets_channel() {
    let mut mgr = mgr(0);
    start_channel(&mut mgr);
    assert!(attempt(&mut mgr, 100).await, "the channel was cancelled");
}

/// **Regression guard (the frozen roll).** Twenty 50 % attempts by the
/// same attacker and effect on one channel land some and miss some. Seeded
/// without the nonce, a channel's attempts share one seed and every one has
/// the same outcome (`both outcomes` fails).
#[tokio::test]
async fn each_attempt_on_a_channel_rolls_afresh() {
    let mut mgr = mgr(0);
    let mut outcomes = Vec::new();
    for _ in 0..20 {
        start_channel(&mut mgr);
        outcomes.push(attempt(&mut mgr, 50).await);
        mgr.get_entity_mut(1).unwrap().active_effects.clear();
    }
    assert!(
        outcomes.contains(&true) && outcomes.contains(&false),
        "both outcomes: {outcomes:?}"
    );
}

/// Park one warmup on the channeller: always instance 77, so a restored
/// cast looks to the old seed exactly like the first.
fn start_warmup(mgr: &mut SpaceManager) {
    let space_id = mgr.get_entity(CHANNELLER).unwrap().space_id;
    mgr.get_entity_mut(CHANNELLER).unwrap().pending_cast = Some(PendingCast {
        ability_id: 50,
        target_id: 1,
        wire_target_id: 1,
        ground: None,
        effect_seq: 77,
        fire_at: Instant::now() + std::time::Duration::from_secs(5),
        warmup_secs: 5.0,
        anchor: Vector3::new(5.0, 0.0, 0.0),
        space_id,
        weapon_instance: None,
    });
    mgr.pending_casts.insert(CHANNELLER);
}

/// **Regression guard (repeated hits on one warmup).** Twenty 50 % EMP hits
/// on the same warmup land some and miss some; a landed one breaks the cast
/// and the test restores it with the same instance. Without the nonce every
/// hit shares one seed (`both outcomes` fails).
#[tokio::test]
async fn each_hit_on_one_warmup_rolls_afresh() {
    let mut mgr = mgr(0);
    let mut outcomes = Vec::new();
    for _ in 0..20 {
        if mgr.get_entity(CHANNELLER).unwrap().pending_cast.is_none() {
            start_warmup(&mut mgr);
        }
        let (tx, _rx) = mpsc::channel(512);
        mgr.request_interrupt(InterruptRequest {
            cast_id: None,
            source_id: 9,
            target_id: CHANNELLER,
            effect_id: 9120,
            ability_id: 1445,
            chance_pct: 50,
            cause: InterruptCause::Effect,
            nonce: 0,
        });
        resolve_interrupts_for(CHANNELLER, &tx, &mut mgr).await;
        outcomes.push(mgr.get_entity(CHANNELLER).unwrap().pending_cast.is_none());
    }
    assert!(
        outcomes.contains(&true) && outcomes.contains(&false),
        "both outcomes: {outcomes:?}"
    );
}
