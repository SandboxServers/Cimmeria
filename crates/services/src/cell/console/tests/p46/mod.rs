//! Packet P46 regression suite: `.goto`, `.summon`, `.gotolocation` (absent,
//! new registrations) — the three thin command adapters over P44's online
//! name lookup and P45's cross-space transfer primitive.
//!
//! Split by command, because the three share only their plumbing:
//!
//! - [`goto`] — move the caller-or-selection to a *named player's* exact
//!   position and exact instance.
//! - [`summon`] — move a *named player* to the caller-or-selection's position
//!   and instance (the same move with the roles swapped).
//! - [`gotolocation`] — move the caller-or-selection to explicit coordinates
//!   in a named world, with D15's default-instance rule.
//!
//! What this suite does *not* re-prove: P44's name-matching rules
//! (`cell::space_manager::tests::player_name_lookup`) and P45's
//! validate-before-teardown ordering / disconnect staging
//! (`cell::space_transfer::tests`, `base::world_entry::gate_travel::tests::
//! transfer`). Here the question is only whether the adapters call those two
//! with the right arguments and report the right thing to the GM.
//!
//! The world table mirrors `cell::space_transfer::tests`' fixture — same four
//! worlds, same instanced/non-instanced split — so a scenario described in
//! either suite reads the same way. It is re-declared rather than imported
//! because that module's helpers are `pub(super)` inside `space_transfer`.
//!
//! Filter prefix: `legacy_p46_`.

use cimmeria_content_engine::chain::ChainEngine;
use tokio::sync::mpsc;

use super::{decode_feedback, setup};
use crate::cell::console::exec;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

mod freeform;
mod goto;
mod gotolocation;
mod summon;

/// Non-instanced, created by `setup()`; every fixture starts here.
pub(super) const AGNOS: &str = "Agnos";
/// Non-instanced "other world" for the cross-world legs.
pub(super) const CASTLE: &str = "Castle";
/// Instanced: every `create_entity` against it allocates a fresh space, so
/// this is the world that can hold several simultaneous instances.
pub(super) const INSTANCED: &str = "Castle_CellBlock";
/// Never declared in `spaces.xml` — the `.gotolocation` unknown-world case.
pub(super) const UNKNOWN_WORLD: &str = "Chulak";

/// Legacy `Player.py:308`/`:331`.
pub(super) const NOT_AVAILABLE: &str = "Player is not available on this CellApp";
/// Legacy `Player.py:313`/`:336`.
pub(super) const NOT_REACHABLE: &str = "Player is not on any reachable space";

/// `setup()`'s GM + NPC in Agnos, plus the Castle / Castle_CellBlock worlds.
pub(super) fn setup_worlds() -> (SpaceManager, u32, u32) {
    let (mut mgr, gm, npc) = setup();
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces>
            <Space WorldName="Castle" Instanced="false" MinX="0" MaxX="1000" MinY="0" MaxY="1000" />
            <Space WorldName="Castle_CellBlock" Instanced="true" MinX="0" MaxX="1000" MinY="0" MaxY="1000" />
        </Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    // The GM is a named player too — `.gotolocation`'s feedback line quotes
    // the *subject's* name (legacy `entity.getName()`).
    mgr.get_entity_mut(gm).unwrap().character_name = Some("Vala".into());
    (mgr, gm, npc)
}

/// Create a connected player carrying `name`, i.e. one P44's lookup can
/// resolve: `character_name` set **and** present in the space's `players`
/// set. Returns the space it landed in.
pub(super) fn spawn_named_player(
    mgr: &mut SpaceManager,
    entity_id: u32,
    world: &str,
    position: [f32; 3],
    name: &str,
) -> u32 {
    let space_id = mgr
        .create_entity(entity_id, world, position, [0.0; 3])
        .expect("fixture player must be creatable");
    finish_named_player(mgr, entity_id, name);
    space_id
}

/// Same, but joining one exact already-loaded instance — the only way to put
/// two fixture players in the *same* instance of an instanced world, since
/// `create_entity` allocates a fresh space every time for those.
pub(super) fn spawn_named_player_in_space(
    mgr: &mut SpaceManager,
    entity_id: u32,
    space_id: u32,
    position: [f32; 3],
    name: &str,
) {
    mgr.create_entity_in_space(entity_id, space_id, position, [0.0; 3])
        .expect("fixture instance must be loaded");
    finish_named_player(mgr, entity_id, name);
}

fn finish_named_player(mgr: &mut SpaceManager, entity_id: u32, name: &str) {
    use crate::cell::space_manager::PlayerNameLookup;

    mgr.connect_entity(entity_id);
    mgr.get_entity_mut(entity_id).unwrap().character_name = Some(name.to_string());
    // The fixture must be *visible* to P44's lookup. `Ambiguous` counts:
    // the duplicate-name test deliberately spawns two players with one name,
    // and the whole point of that scenario is that the lookup sees both.
    let visible = match mgr.find_online_player_by_name(name) {
        PlayerNameLookup::Found { entity_id: e, .. } => e == entity_id,
        PlayerNameLookup::Ambiguous { entity_ids } => entity_ids.contains(&entity_id),
        _ => false,
    };
    assert!(
        visible,
        "fixture player {name} ({entity_id}) must be visible to P44's lookup"
    );
}

/// Everything a travel command can put on the base channel.
#[derive(Debug, Default)]
pub(super) struct Traffic {
    /// `(entity_id, space_id, position, prev_pos)` — the same-space snap.
    pub teleports: Vec<(u32, u32, [f32; 3], [f32; 3])>,
    /// `(entity_id, world, destination_space_id, position)` — the cross-space
    /// transfer.
    pub gate_travels: Vec<(u32, String, Option<u32>, [f32; 3])>,
    /// `rotation` from the same `GateTravel` sends, kept parallel to
    /// `gate_travels` rather than folded into its tuple so the existing
    /// exact-tuple assertions at every call site don't all need updating for
    /// a field only the facing-preservation regression test cares about.
    pub gate_travel_rotations: Vec<[f32; 3]>,
    pub feedback: Vec<String>,
}

impl Traffic {
    /// The single destination instance a transfer named, panicking unless
    /// exactly one transfer was enqueued.
    pub fn only_gate_travel(&self) -> &(u32, String, Option<u32>, [f32; 3]) {
        assert_eq!(
            self.gate_travels.len(),
            1,
            "expected exactly one GateTravel, got {:?}",
            self.gate_travels
        );
        &self.gate_travels[0]
    }

    /// `rotation` from the single enqueued `GateTravel`, panicking unless
    /// exactly one transfer was enqueued (same precondition as
    /// [`Self::only_gate_travel`]).
    pub fn only_gate_travel_rotation(&self) -> [f32; 3] {
        assert_eq!(
            self.gate_travel_rotations.len(),
            1,
            "expected exactly one GateTravel, got {:?}",
            self.gate_travel_rotations
        );
        self.gate_travel_rotations[0]
    }

    pub fn has_line(&self, text: &str) -> bool {
        self.feedback.iter().any(|l| l == text)
    }

    pub fn mentions(&self, fragment: &str) -> bool {
        self.feedback.iter().any(|l| l.contains(fragment))
    }
}

fn drain(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Traffic {
    let mut t = Traffic::default();
    while let Ok(msg) = rx.try_recv() {
        match &msg {
            CellToBaseMsg::TeleportPlayer {
                entity_id,
                space_id,
                position,
                prev_pos,
            } => t
                .teleports
                .push((*entity_id, *space_id, *position, *prev_pos)),
            CellToBaseMsg::GateTravel {
                entity_id,
                target_world_name,
                position,
                rotation,
                destination_space_id,
                ..
            } => {
                t.gate_travels.push((
                    *entity_id,
                    target_world_name.clone(),
                    *destination_space_id,
                    *position,
                ));
                t.gate_travel_rotations.push(*rotation);
            }
            _ => {
                if let Some(text) = decode_feedback(&msg) {
                    t.feedback.push(text);
                }
            }
        }
    }
    t
}

/// Drive one console command through `exec` (bypassing only the GM gate and
/// `resolve_target`, so the handler's own target fallback is what is under
/// test) and collect everything it emitted.
pub(super) async fn run(
    cmd: &str,
    caller: u32,
    args: &[&str],
    target: Option<u32>,
    mgr: &mut SpaceManager,
) -> Traffic {
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(64);
    exec(cmd, caller, args, target, &tx, mgr, &engine).await;
    drop(tx);
    drain(&mut rx)
}

/// Position of a still-live entity.
pub(super) fn position_of(mgr: &SpaceManager, entity_id: u32) -> [f32; 3] {
    let p = mgr
        .get_entity(entity_id)
        .unwrap_or_else(|| panic!("entity {entity_id} must still exist"))
        .position;
    [p.x, p.y, p.z]
}

/// Assert nothing moved and nothing was enqueued — the shape every rejection
/// has to have.
pub(super) fn assert_no_move(traffic: &Traffic) {
    assert!(
        traffic.teleports.is_empty(),
        "a rejected travel command must not snap anything: {:?}",
        traffic.teleports
    );
    assert!(
        traffic.gate_travels.is_empty(),
        "a rejected travel command must not enqueue a transfer: {:?}",
        traffic.gate_travels
    );
}
