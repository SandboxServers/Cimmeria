//! Deployables Phase 0, combat half: the ground-point launch, the fire that
//! places the object, and the pulse tick.
//!
//! The fixture is the seed: 1012 Deployable: Microwave Emitter (cooldown
//! 30, warmup 2, range 500 UE3 units = 5 m, no event set) with 5065
//! (30 x 1 s) and 5066 (Medium = 10 m, FocusDamage 100,
//! `RangedPhysicalDamage`), template 400 and
//! its `deployables` row. The owner stands at (5, 0, 10) in a shared Castle
//! space.

use std::time::{Duration, Instant};

use tokio::sync::mpsc;

use cimmeria_cell_world::test_fixtures::{add_pet_owner, seed_deployable, DEPLOYABLE_ABILITY};
use cimmeria_entity::stats::{FOCUS, HEALTH};

use crate::cell::abilities::resolve_warmups;
use crate::cell::combat::HOSTILE_FACTION;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::mercury::method_idx;
use crate::test_support::NoContentEvents;

use super::super::handle_use_ability_on_ground;

mod launch;
mod logs;
mod pulse;

const OWNER: u32 = 1;
const OWNER_POS: [f32; 3] = [5.0, 0.0, 10.0];
/// A point 4 m from the owner, inside 1012's 5 m range.
const SPOT: [f32; 3] = [9.0, 0.0, 10.0];
/// Plenty of health, so a pulse never kills unless a test wants it to.
const FULL: i32 = 100_000;

/// One shared Castle space, the owner (a trained Scientist) and the 1012
/// fixture.
fn deploy_mgr() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    add_pet_owner(&mut mgr, OWNER, "Castle", OWNER_POS, 25);
    mgr.get_entity_mut(OWNER)
        .unwrap()
        .abilities
        .add_ability(DEPLOYABLE_ABILITY);
    seed_deployable(&mut mgr);
    mgr
}

/// An NPC `id` at `pos` with `faction`, full health and 200 focus.
fn npc(mgr: &mut SpaceManager, id: u32, pos: [f32; 3], faction: u8) {
    mgr.spawn_npc(id, "Castle", pos, [0.0; 3]).unwrap();
    let e = mgr.get_entity_mut(id).unwrap();
    e.faction = faction;
    let hp = e.stats.get_mut(HEALTH).unwrap();
    hp.update(0, FULL, FULL);
    hp.clear_dirty();
    let f = e.stats.get_mut(FOCUS).unwrap();
    f.update(0, 200, 200);
    f.clear_dirty();
}

fn hostile(mgr: &mut SpaceManager, id: u32, pos: [f32; 3]) {
    npc(mgr, id, pos, HOSTILE_FACTION);
}

fn drain(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<CellToBaseMsg> {
    let mut out = Vec::new();
    while let Ok(m) = rx.try_recv() {
        out.push(m);
    }
    out
}

/// `(entity, method, args)` of every entity method sent.
fn calls(msgs: &[CellToBaseMsg]) -> Vec<(u32, u16, Vec<u8>)> {
    msgs.iter()
        .filter_map(|m| match m {
            CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index,
                args,
            }
            | CellToBaseMsg::WitnessEntityMethod {
                entity_id,
                method_index,
                args,
                ..
            } => Some((*entity_id, *method_index, args.clone())),
            _ => None,
        })
        .collect()
}

/// The `onErrorCode` codes sent to `entity` for `ability`.
fn error_codes(msgs: &[CellToBaseMsg], entity: u32, ability: i32) -> Vec<u16> {
    calls(msgs)
        .into_iter()
        .filter(|(e, m, a)| {
            *e == entity
                && *m == method_idx::ON_ERROR_CODE
                && a.len() == 7
                && i32::from_le_bytes([a[1], a[2], a[3], a[4]]) == ability
        })
        .map(|(_, _, a)| u16::from_le_bytes([a[5], a[6]]))
        .collect()
}

/// True when `entity` was sent a `CHAN_FEEDBACK` chat line containing
/// `text`. The line is a WSTRING, so the needle is matched as UTF-16LE.
fn got_chat(msgs: &[CellToBaseMsg], entity: u32, text: &str) -> bool {
    let needle: Vec<u8> = text.encode_utf16().flat_map(u16::to_le_bytes).collect();
    calls(msgs).into_iter().any(|(e, m, a)| {
        e == entity
            && m == method_idx::ON_PLAYER_COMMUNICATION
            && a.windows(needle.len()).any(|w| w == needle.as_slice())
    })
}

/// A moment past 1012's 2 s warmup.
fn after_warmup() -> Instant {
    Instant::now() + Duration::from_millis(2_050)
}

/// Cast 1012 at `spot` and complete its warmup. Returns the object placed.
async fn deploy_at(
    mgr: &mut SpaceManager,
    spot: [f32; 3],
    tx: &mpsc::Sender<CellToBaseMsg>,
) -> u32 {
    let before = mgr.deployables.of_owner(OWNER, DEPLOYABLE_ABILITY);
    handle_use_ability_on_ground(OWNER, DEPLOYABLE_ABILITY, spot, tx, mgr).await;
    assert_eq!(
        resolve_warmups(after_warmup(), tx, mgr, &NoContentEvents).await,
        1,
        "the cast fires once its warmup ends"
    );
    let now: Vec<u32> = mgr
        .deployables
        .of_owner(OWNER, DEPLOYABLE_ABILITY)
        .into_iter()
        .filter(|d| !before.contains(d))
        .collect();
    assert_eq!(now.len(), 1, "exactly one new object");
    now[0]
}

/// Clear 1012's cooldown so a test can cast again at once.
fn ready_again(mgr: &mut SpaceManager) {
    mgr.get_entity_mut(OWNER)
        .unwrap()
        .abilities
        .clear_ability_cooldown(DEPLOYABLE_ABILITY);
}
