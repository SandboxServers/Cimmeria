//! The InitPlayerState handler is the hydrate-on-login site for
//! `CellEntity::system_options`. These guards pin that the
//! incoming `SystemOptions` actually lands on the entity — without
//! this, a regression that drops the field assignment would let
//! the cell fall back to `SystemOptions::default()` every login
//! and the user's saved checkbox values would silently revert
//! after every reconnect.

use super::super::*;
use cimmeria_entity::cell_entity::SystemOptions;
fn make_mgr() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
        .unwrap();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    mgr.connect_entity(1);
    mgr
}

/// The hydrated SystemOptions block must replace the entity's
/// default. Bug shape: a refactor that drops the assignment
/// silently leaves auto_reload=true / reload_on_activate=false
/// regardless of what the DB returned.
#[tokio::test]
async fn init_player_state_assigns_system_options() {
    let mut mgr = make_mgr();
    let engine = ChainEngine::new();
    let (tx, _rx) = mpsc::channel(32);
    // Hydrate values DIFFERENT from `SystemOptions::default()` so a
    // missed assignment is observable. Defaults are auto_reload=true,
    // reload_on_activate=false; flip both.
    let hydrated = SystemOptions {
        auto_reload: false,
        reload_on_activate: true,
    };

    handle_init_player_state(
        1,
        100,
        "Castle_CellBlock".into(),
        1,
        vec![],
        vec![],
        0,
        vec![],
        hydrated.clone(),
        0, // state_field
        0, // access_level
        &tx,
        &mut mgr,
        &engine,
    )
    .await;

    let e = mgr.get_entity(1).unwrap();
    assert_eq!(
        e.system_options, hydrated,
        "InitPlayerState must overwrite the entity's default \
         SystemOptions with the DB-hydrated values",
    );
}

/// **#475 plumbing guard.** `InitPlayerState` must land the
/// session's `access_level` on `CellEntity::access_level` — that's
/// the authoritative value the cell-method GM gate reads. A refactor
/// that drops the assignment would silently leave every entity at
/// access_level 0, so legitimate GMs lose their commands AND (worse,
/// once the gate is the only check) the plumbing that makes the gate
/// meaningful is gone. Pin a non-zero level so a missed assignment is
/// observable.
#[tokio::test]
async fn init_player_state_assigns_access_level() {
    let mut mgr = make_mgr();
    let engine = ChainEngine::new();
    let (tx, _rx) = mpsc::channel(32);

    const GM_LEVEL: u32 = 2; // GameMaster

    handle_init_player_state(
        1,
        100,
        "Castle_CellBlock".into(),
        1,
        vec![],
        vec![],
        0,
        vec![],
        SystemOptions::default(),
        0, // state_field
        GM_LEVEL,
        &tx,
        &mut mgr,
        &engine,
    )
    .await;

    assert_eq!(
        mgr.get_entity(1).unwrap().access_level,
        GM_LEVEL,
        "InitPlayerState must store the session access_level on the \
         cell entity — the GM gate has nothing to check otherwise (#475)"
    );
}

/// **Lomiada's 2026-06-04 18:09 session regression.**
/// `InitPlayerState` must seed the cell-entity's stats from the
/// archetype's base values so the damage scripts see non-zero
/// FOCUS + HEALTH pools. Without this, `RangedPhysicalDamage`
/// reads `target.stats.get(FOCUS).cur` as 0 → focus_overflow ==
/// focus_damage every shot → no shield absorb → full spillover
/// damage every hit.
///
/// Pins both pools at non-zero with the Soldier archetype
/// values (760 HEALTH, 1570 FOCUS — straight out of the
/// hardcoded `mercury::world_data::stats::archetype_stats`
/// table). Reverting the `apply_archetype` call in
/// `handle_init_player_state` trips this guard by leaving the
/// stats at the `StatList::new()` default of (0, 0, 0).
#[tokio::test]
async fn init_player_state_seeds_stats_from_archetype() {
    use cimmeria_entity::stats::{FOCUS, HEALTH};

    let mut mgr = make_mgr();
    let engine = ChainEngine::new();
    let (tx, _rx) = mpsc::channel(32);

    handle_init_player_state(
        1,
        100,
        "Castle_CellBlock".into(),
        1, // archetype_id = Soldier
        vec![],
        vec![],
        0,
        vec![],
        SystemOptions::default(),
        0, // state_field
        0, // access_level
        &tx,
        &mut mgr,
        &engine,
    )
    .await;

    let e = mgr.get_entity(1).unwrap();

    let health = e
        .stats
        .get(HEALTH)
        .expect("HEALTH stat must exist on cell entity");
    assert_eq!(
        health.max, 760,
        "Soldier max HEALTH must be seeded to the archetype's 760 — \
         pre-fix the default (0, 0, 0) tuple meant respawn would \
         restore 0 HP and the player would spawn dead.",
    );
    assert_eq!(
        health.cur, 760,
        "Initial HEALTH cur must equal max — apply_archetype \
         sets (min=0, cur=max, max=max)",
    );

    let focus = e
        .stats
        .get(FOCUS)
        .expect("FOCUS stat must exist on cell entity");
    assert_eq!(
        focus.max, 1570,
        "Soldier max FOCUS must be seeded to the archetype's 1570",
    );
    assert_eq!(
        focus.cur, 1570,
        "Initial FOCUS cur must equal max — without this, \
         RangedPhysicalDamage absorbs zero (overflow == full \
         damage every shot, 86 HP per hit on lomiada's session).",
    );
}

/// **RE-driven defensive resync.** `InitPlayerState` must re-emit
/// `onActiveSlotUpdate` to the client AFTER `onClientReady`,
/// carrying the player's persisted active bandolier slot. The Ghidra-recovered client behaviour is that
/// the NetIn handler `FUN_00da9ce0` walks `SGWPlayer.bagList`
/// (at `+0x8c → +0x24`) and silently no-ops when the bag-list
/// map is uninitialized. The login-burst `onActiveSlotUpdate`
/// from `mapLoaded` can arrive before the bag-init packets in
/// the same bundle are processed, leaving the cached active-slot
/// value at the default (0). The Lua gate inside
/// `BandolierMod.ActivateBandolierSlotN` then suppresses the
/// wire emit for any F-key matching the stale value — symptom:
/// "F2 doesn't swap to the P90," reported by lomiada
/// 2026-06-04 18:09 with zero `requestActiveSlotChange` events
/// in the entire session.
///
/// Pin: handler must emit at least one `onActiveSlotUpdate`
/// (method index `crate::cell::client_methods::inventory::ON_ACTIVE_SLOT_UPDATE`)
/// carrying bag_id=3 and the 1-indexed wire slot (server slot + 1).
/// Reverting the resend block trips this by leaving the only
/// `onActiveSlotUpdate` emission on the wire as the one from the
/// login burst — which the InitPlayerState handler doesn't see
/// directly (it's a separate wire site).
#[tokio::test]
async fn init_player_state_resends_active_slot_update_for_resync() {
    let mut mgr = make_mgr();
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(64);
    const SERVER_SLOT: i32 = 1; // arbitrary non-zero so the bag_id=3 + (slot+1)=2 encoding is observable

    handle_init_player_state(
        1,
        100,
        "Castle_CellBlock".into(),
        1,      // archetype_id
        vec![], // no abilities
        vec![], // no missions
        SERVER_SLOT,
        vec![], // no bandolier items (slot can still be active over an empty slot)
        SystemOptions::default(),
        0, // state_field
        0, // access_level
        &tx,
        &mut mgr,
        &engine,
    )
    .await;

    // Walk the channel for the onActiveSlotUpdate emit. Decode the
    // 8-byte payload: bag_id (i32 LE) + wire_slot (i32 LE).
    let mut found = None;
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall {
            entity_id: 1,
            method_index,
            args,
        } = msg
        {
            if method_index == crate::cell::client_methods::inventory::ON_ACTIVE_SLOT_UPDATE
                && args.len() == 8
            {
                let bag_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let wire_slot = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                found = Some((bag_id, wire_slot));
                break;
            }
        }
    }

    let (bag_id, wire_slot) = found.expect(
        "InitPlayerState must re-send onActiveSlotUpdate as a defensive \
         resync against the client bag-list init race documented in \
         docs/reverse-engineering/findings/client-wire-emit-suppression.md",
    );
    assert_eq!(bag_id, 3, "bag_id must be CONTAINER_BANDOLIER (3)");
    assert_eq!(
        wire_slot,
        SERVER_SLOT + 1,
        "wire slot is 1-indexed (server slot {SERVER_SLOT} + 1)"
    );
}

/// Hydrating with the same value as the default still has to
/// assign (not skip) — otherwise a hand-edited row that explicitly
/// stores the defaults could be silently treated as "unset" if
/// somebody added a "skip if equals default" optimisation.
#[tokio::test]
async fn init_player_state_assigns_default_values_explicitly() {
    let mut mgr = make_mgr();
    if let Some(p) = mgr.get_entity_mut(1) {
        // Pre-stuff the entity with non-defaults so the assignment
        // is observable even when the hydrated value is default.
        p.system_options.auto_reload = false;
        p.system_options.reload_on_activate = true;
    }
    let engine = ChainEngine::new();
    let (tx, _rx) = mpsc::channel(32);

    handle_init_player_state(
        1,
        100,
        "Castle_CellBlock".into(),
        1,
        vec![],
        vec![],
        0,
        vec![],
        SystemOptions::default(),
        0, // state_field
        0, // access_level
        &tx,
        &mut mgr,
        &engine,
    )
    .await;

    let e = mgr.get_entity(1).unwrap();
    assert_eq!(
        e.system_options,
        SystemOptions::default(),
        "InitPlayerState must always overwrite — even an explicit \
         default-equal hydrate must reset prior in-memory state",
    );
}
