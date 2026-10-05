//! Ability mechanics AB-L2: the ability lab dot commands (`.effects`,
//! `.cooldowns`, `.dummy`, `.dummy caster`, `.cleareffects`).
//!
//! Filter prefix: `ab_l2_`.
//!
//! Bug shapes: a non-GM reaching a handler, or a refusal with no feedback; a
//! cooldown cleared on the server but not on the client (the hotbar sweep
//! keeps running), or the clear sent with the wrong bytes; a dummy that
//! fights back, outlives its owner or its ten minutes, or that another GM can
//! clear; an effect strip that leaves an icon, a stat shift or a stun flag
//! behind; a command that changes state with no audit row or no visible line.

mod clear_effects;
mod cooldowns;
mod dummy;
mod dummy_caster;
mod dummy_caster_live_db;
mod dummy_combat;
mod effects;

use cimmeria_entity::cell_entity::{TimedEffectSpec, TimedStacking};
use cimmeria_entity::stats::ACCURACY;

use super::decode_feedback;
use super::pt07_giveability::{say, CALLER};
use crate::cell::client_methods::being::ON_TIMER_UPDATE;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

pub(super) use super::pt07_giveability::{console, world};

/// The feedback lines in `msgs`.
fn lines(msgs: &[CellToBaseMsg]) -> Vec<String> {
    msgs.iter().filter_map(decode_feedback).collect()
}

/// `(entity_id, args)` of every `onTimerUpdate` in `msgs`.
fn timers(msgs: &[CellToBaseMsg]) -> Vec<(u32, Vec<u8>)> {
    msgs.iter()
        .filter_map(|m| match m {
            CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index,
                args,
            } if *method_index == ON_TIMER_UPDATE => Some((*entity_id, args.clone())),
            _ => None,
        })
        .collect()
}

/// Aim (effect 700, ability 637): +200 Accuracy for 15 s, from `invoker`.
fn aim(invoker: u32) -> TimedEffectSpec {
    TimedEffectSpec {
        cast_id: Some(41),
        effect_id: 700,
        ability_id: 637,
        invoker_id: invoker,
        effect_flags: 21,
        moniker_ids: vec![],
        stats: vec![(ACCURACY, 200)],
        absorb: Vec::new(),
        state_flags: 0,
        duration_secs: Some(15.0),
        stacking: TimedStacking::PerSource,
        invoker_identity: Default::default(),
        invoker_name: None,
    }
}

/// A non-GM (and a trial GM) gets the generic refusal for every AB-L2
/// command, and none of them changes anything.
#[tokio::test]
async fn ab_l2_non_gm_is_refused_with_feedback_and_nothing_changes() {
    for access_level in [0, 1] {
        for line in [
            ".effects",
            ".cooldowns reset",
            ".cooldowns reset 592",
            ".dummy",
            ".dummy friendly",
            ".cleareffects",
        ] {
            let (mut mgr, _npc) = world(access_level);
            let caller = mgr.get_entity_mut(CALLER).unwrap();
            caller
                .abilities
                .start_ability_cooldown(592, std::time::Duration::from_secs(30));
            caller
                .apply_timed_effect(aim(CALLER), std::time::Instant::now())
                .unwrap();
            let before = mgr.entity_count();

            let msgs = say(&mut mgr, line).await;

            let cmd = line[1..].split_whitespace().next().unwrap();
            assert_eq!(
                lines(&msgs),
                vec![format!(".{cmd} is a GM command; you do not have GM rights")],
                "level {access_level} {line}"
            );
            assert!(timers(&msgs).is_empty(), "{line}: no timer sent");
            let caller = mgr.get_entity(CALLER).unwrap();
            assert!(
                caller.abilities.is_on_cooldown(592),
                "{line}: cooldown kept"
            );
            assert_eq!(caller.stat_buffs.entries.len(), 1, "{line}: effect kept");
            assert_eq!(mgr.entity_count(), before, "{line}: nothing spawned");
        }
    }
}

/// The entity count across every space, for "nothing was spawned".
trait EntityCount {
    fn entity_count(&self) -> usize;
}

impl EntityCount for SpaceManager {
    fn entity_count(&self) -> usize {
        self.spaces.values().map(|s| s.entities.len()).sum()
    }
}
