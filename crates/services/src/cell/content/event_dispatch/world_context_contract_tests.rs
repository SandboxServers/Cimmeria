//! The `world` condition contract (Harset H07): every `fire_*` dispatcher
//! populates `ctx.world_id` from the acting player's space.
//!
//! `Condition::World` fails closed on a context with no world id, so a
//! dispatcher that forgets `populate_world_context` does not misfire — it
//! silently never fires a `world`-gated chain. That is the shape these
//! tests guard, and it is a shape merges produce: `fire_stargate_dialed`,
//! `fire_stargate_crossed` (Castle CA10) and `fire_player_flanked_npc`
//! (Cellblock C06) all landed on `main` while H07 was on a branch, merged
//! cleanly, and shipped without the populator call.
//!
//! `chain_replay_tests/world_condition.rs` proves the loader, the evaluator
//! and `fire_enter_region`. This module covers the dispatchers that one does
//! not reach, in memory (no database), one positive and one negative each.
//! The positive half is the guard: it goes red when the populator call is
//! removed. The negative half proves the gate still discriminates, so the
//! positive half cannot be satisfied by an ungated chain.

use std::collections::HashMap;

use tokio::sync::mpsc;

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::{Chain, ChainEngine};
use cimmeria_content_engine::conditions::{ComparisonOp, Condition};
use cimmeria_content_engine::triggers::Trigger;

use super::{fire_player_flanked_npc, fire_stargate_crossed, fire_stargate_dialed};
use crate::cell::space_manager::SpaceManager;

/// `resources.worlds.world_id` for `Harset`, the world the chains gate on.
const HARSET: i32 = 57;
/// The adjacent world the same chains must not fire in.
const HARSET_CMD_CENTER: i32 = 68;

const PLAYER_EID: u32 = 1;
const NPC_EID: u32 = 2;
const PLAYER_ID: i32 = 100;
const COUNTER: &str = "world_gate_fired";

/// Both Harset worlds with their real ids stamped on, and a connected
/// player plus one NPC standing in `world_name`.
fn make_mgr(world_name: &str) -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces>
        <Space WorldName="Harset" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" />
        <Space WorldName="Harset_CmdCenter" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" />
    </Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces>
        <Space WorldName="Harset" />
        <Space WorldName="Harset_CmdCenter" />
    </Spaces>"#,
    )
    .unwrap();
    mgr.stamp_world_ids(&HashMap::from([
        ("Harset".to_string(), HARSET),
        ("Harset_CmdCenter".to_string(), HARSET_CMD_CENTER),
    ]));
    mgr.create_entity(PLAYER_EID, world_name, [0.0; 3], [0.0; 3])
        .unwrap();
    mgr.create_entity(NPC_EID, world_name, [0.0; 3], [0.0; 3])
        .unwrap();
    if let Some(p) = mgr.get_entity_mut(PLAYER_EID) {
        p.is_player = true;
        p.player_id = Some(PLAYER_ID);
    }
    mgr.connect_entity(PLAYER_EID);
    mgr
}

/// A chain on `trigger`, gated `world eq 57`, whose only action bumps a
/// counter on the acting player (observable without a database).
fn harset_gated_engine(id: i64, trigger: Trigger) -> ChainEngine {
    let mut engine = ChainEngine::new();
    engine.register_chain(Chain {
        id,
        name: format!("test: world-gated {id}"),
        enabled: true,
        trigger,
        conditions: vec![Condition::World {
            operator: ComparisonOp::Eq,
            world_id: HARSET,
        }],
        actions: vec![Action::IncrementCounter {
            counter_name: COUNTER.to_string(),
            amount: 1,
        }],
        action_delays: Vec::new(),
        priority: 0,
    });
    engine
}

fn fired(mgr: &SpaceManager) -> bool {
    mgr.get_entity(PLAYER_EID)
        .expect("player entity must still exist")
        .counters
        .contains_key(COUNTER)
}

#[derive(Clone, Copy, Debug)]
enum Dispatcher {
    StargateDialed,
    StargateCrossed,
    PlayerFlankedNpc,
}

impl Dispatcher {
    fn trigger(self) -> Trigger {
        match self {
            Self::StargateDialed => Trigger::OnStargateDialed {
                destination_world: None,
            },
            Self::StargateCrossed => Trigger::OnStargateCrossed {
                destination_world: None,
            },
            Self::PlayerFlankedNpc => Trigger::OnPlayerFlankedNpc { npc_template: None },
        }
    }

    async fn fire(self, engine: &ChainEngine, mgr: &mut SpaceManager) {
        let (tx, _rx) = mpsc::channel(16);
        match self {
            Self::StargateDialed => {
                fire_stargate_dialed(PLAYER_EID, PLAYER_ID, "Castle", engine, &tx, mgr).await;
            }
            Self::StargateCrossed => {
                fire_stargate_crossed(PLAYER_EID, PLAYER_ID, "Castle", engine, &tx, mgr).await;
            }
            Self::PlayerFlankedNpc => {
                fire_player_flanked_npc(NPC_EID, PLAYER_EID, "Jaffa Guard", engine, &tx, mgr).await;
            }
        }
    }
}

const ALL: [Dispatcher; 3] = [
    Dispatcher::StargateDialed,
    Dispatcher::StargateCrossed,
    Dispatcher::PlayerFlankedNpc,
];

/// A `world eq 57` chain fires for a player standing in Harset, through
/// each dispatcher. Red when that dispatcher stops populating `world_id`.
#[tokio::test]
async fn world_gated_chain_fires_in_the_authored_world() {
    for (i, dispatcher) in ALL.into_iter().enumerate() {
        let engine = harset_gated_engine(0x7007_0100 + i as i64, dispatcher.trigger());
        let mut mgr = make_mgr("Harset");
        dispatcher.fire(&engine, &mut mgr).await;
        assert!(
            fired(&mgr),
            "{dispatcher:?}: a `world eq 57` chain must fire for a player in Harset (57). \
             Not firing means the condition failed closed: the dispatcher never called \
             populate_world_context, so ctx.world_id was None",
        );
    }
}

/// The same chain does not fire one world over, so the positive test
/// above cannot be satisfied by a chain that lost its gate.
#[tokio::test]
async fn world_gated_chain_does_not_fire_in_the_adjacent_world() {
    for (i, dispatcher) in ALL.into_iter().enumerate() {
        let engine = harset_gated_engine(0x7007_0110 + i as i64, dispatcher.trigger());
        let mut mgr = make_mgr("Harset_CmdCenter");
        dispatcher.fire(&engine, &mut mgr).await;
        assert!(
            !fired(&mgr),
            "{dispatcher:?}: a `world eq 57` chain must NOT fire for a player in \
             Harset_CmdCenter (68)",
        );
    }
}
