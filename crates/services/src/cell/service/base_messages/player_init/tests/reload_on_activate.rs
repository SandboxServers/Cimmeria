//! Reload-on-activate gate guards for the `InitPlayerState` world-entry
//! path. Split out of `system_options_assignment` to keep both files under
//! the size cap; pure test-code move (no logic changes).

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

/// Stage an InitPlayerState payload with one bandolier item in
/// slot 0 (clip 30) and the given `current_ammo` (so 30 = full,
/// 10 = partial, 30 = no-op for the reload check). Stamps the
/// caller-chosen `system_options` so each test exercises the
/// gate path it intends. Returns the tuple bound to
/// `handle_init_player_state`'s positional args.
fn init_args_with_bandolier_clip(
    current_ammo: i32,
    system_options: SystemOptions,
) -> (
    i32,
    Vec<i32>,
    i32,
    Vec<(i32, cimmeria_entity::cell_entity::BandolierItem)>,
    SystemOptions,
) {
    (
        1,      // archetype_id
        vec![], // no abilities
        0,      // active_bandolier_slot
        vec![(
            0,
            cimmeria_entity::cell_entity::BandolierItem {
                instance_id: 0,
                item_id: 55,
                clip_size: 30,
                default_ammo_type: 1,
                current_ammo,
                cur_ammo_type: 1,
            },
        )],
        system_options,
    )
}

/// World entry with `reloadOnActivate = true` AND a partial-clip
/// active bandolier weapon must trigger an automatic reload, the
/// same as F1-F4 swap or in-game equip. Without this hook a
/// player who logs in, gate-travels, or cross-world rings to a
/// new map with a half-empty active weapon silently doesn't
/// auto-reload until they manually swap slots and back —
/// surfacing in play as "my option does nothing on login."
///
/// Setup: 10/30 active clip + `reload_on_activate = true`.
/// Assertion: `reload_complete_at` is `Some` post-handler. The
/// Phase A draw guard (weapon holstered + threatened_mobs empty)
/// is bypassed because `weapon_holstered` defaults to `false`
/// on a freshly-restored entity in this test fixture; in
/// production the same `handle_reload` path would walk through
/// Phase A naturally if the weapon were holstered.
#[tokio::test]
async fn init_player_state_triggers_reload_on_activate_when_clip_partial() {
    let mut mgr = make_mgr();
    // Need the ABILITY_RELOAD_WEAPON def for `handle_reload` to
    // resolve warmup/cooldown — otherwise it falls back to its
    // hardcoded 2.0/1.0 defaults and the reload still fires.
    mgr.ability_defs.insert(
        596 /* ABILITY_RELOAD_WEAPON — mirrors the const in cell_methods/player/world */,
        cimmeria_entity::abilities::AbilityDef {
            ability_id: 596 /* ABILITY_RELOAD_WEAPON — mirrors the const in cell_methods/player/world */,
            is_ranged: false,
            min_range: 0,
            name: "Reload".into(),
            warmup: 1.0,
            cooldown: 0.5,
            flags: 0,
            max_range: 0,
            target_type_id: 0,
            effect_ids: vec![],
            moniker_ids: vec![],
            required_ammo: 0,
            event_set_id: None,
            velocity: 0.0,
        },
    );
    // Pre-mark the weapon as drawn so the Phase A draw window
    // doesn't apply. This isolates the test to the
    // reload-on-activate trigger; Phase A is exercised by the
    // `handle_reload` tests in `cell_methods/player/world`.
    if let Some(p) = mgr.get_entity_mut(1) {
        p.weapon_holstered = false;
    }
    let engine = ChainEngine::new();
    let (tx, _rx) = mpsc::channel(32);

    let (archetype, abilities, slot, items, sys_opts) = init_args_with_bandolier_clip(
        10,
        SystemOptions {
            auto_reload: true,
            reload_on_activate: true,
        },
    );

    handle_init_player_state(
        1,
        100,
        "Castle_CellBlock".into(),
        archetype,
        vec![], // saved_missions
        abilities,
        slot,
        items,
        sys_opts,
        0, // state_field
        0, // access_level
        &tx,
        &mut mgr,
        &engine,
    )
    .await;

    let e = mgr.get_entity(1).unwrap();
    assert!(
        e.reload_complete_at.is_some(),
        "InitPlayerState with reload_on_activate=true + partial clip \
         must trigger handle_reload (gate-travel / login coverage). \
         Pre-fix the only triggers were F1-F4 swap and inventory \
         equip — login silently no-op'd."
    );
}

/// Default-holstered world-entry path: the real production fixture
/// for login / gate-travel / cross-world ring has
/// `weapon_holstered = true` (the `CellEntity::new` default — the
/// pawn is freshly instantiated and starts with no weapon drawn).
/// The drawn-weapon test above isolates the trigger logic from
/// Phase A draw choreography; this companion guard pins that the
/// holstered-weapon path also queues a reload. Bug shape: a
/// future refactor that adds a `weapon_holstered` short-circuit to
/// `maybe_trigger_reload_on_activate` would silently break every
/// real login.
///
/// Assertion: `pending_reload_at` is `Some` (Phase A draw deadline)
/// post-handler. Phase A is the correct entry path because the
/// weapon is still holstered and OOC — `handle_reload` defers the
/// real reload until the draw animation has played.
#[tokio::test]
async fn init_player_state_triggers_reload_on_activate_when_holstered() {
    let mut mgr = make_mgr();
    mgr.ability_defs.insert(
        596,
        cimmeria_entity::abilities::AbilityDef {
            ability_id: 596,
            is_ranged: false,
            min_range: 0,
            name: "Reload".into(),
            warmup: 1.0,
            cooldown: 0.5,
            flags: 0,
            max_range: 0,
            target_type_id: 0,
            effect_ids: vec![],
            moniker_ids: vec![],
            required_ammo: 0,
            event_set_id: None,
            velocity: 0.0,
        },
    );
    // Sanity: the fixture starts with the production default
    // (holstered=true). Pin it so a future fixture change can't
    // silently relax this test into a no-op.
    assert!(
        mgr.get_entity(1).unwrap().weapon_holstered,
        "fixture sanity: CellEntity::new defaults weapon_holstered=true"
    );
    let engine = ChainEngine::new();
    let (tx, _rx) = mpsc::channel(32);

    let (archetype, abilities, slot, items, sys_opts) = init_args_with_bandolier_clip(
        10,
        SystemOptions {
            auto_reload: true,
            reload_on_activate: true,
        },
    );

    handle_init_player_state(
        1,
        100,
        "Castle_CellBlock".into(),
        archetype,
        vec![],
        abilities,
        slot,
        items,
        sys_opts,
        0, // state_field
        0, // access_level
        &tx,
        &mut mgr,
        &engine,
    )
    .await;

    let e = mgr.get_entity(1).unwrap();
    assert!(
        e.pending_reload_at.is_some(),
        "holstered world-entry with reload_on_activate=true + partial clip \
         must queue Phase A (the draw window); pre-fix only the slot-swap \
         trigger path covered this and login was silent"
    );
    assert!(
        !e.weapon_holstered,
        "Phase A entry must flip weapon_holstered=false so the draw \
         animation plays — without the flip the client renders the \
         reload with the weapon still holstered"
    );
}

/// Symmetric negative: option DEFAULT (`reload_on_activate = false`)
/// must NOT trigger on login, even with a partial clip. The XML
/// default is off — players who never touched the checkbox
/// shouldn't get behavior change.
#[tokio::test]
async fn init_player_state_does_not_reload_when_option_off() {
    let mut mgr = make_mgr();
    let engine = ChainEngine::new();
    let (tx, _rx) = mpsc::channel(32);

    let (archetype, abilities, slot, items, sys_opts) = init_args_with_bandolier_clip(
        10, // partial clip
        SystemOptions {
            auto_reload: true,
            reload_on_activate: false, // XML default
        },
    );

    handle_init_player_state(
        1,
        100,
        "Castle_CellBlock".into(),
        archetype,
        vec![],
        abilities,
        slot,
        items,
        sys_opts,
        0, // state_field
        0, // access_level
        &tx,
        &mut mgr,
        &engine,
    )
    .await;

    let e = mgr.get_entity(1).unwrap();
    assert!(
        e.reload_complete_at.is_none() && e.pending_reload_at.is_none(),
        "InitPlayerState with option off must NOT trigger any reload"
    );
}

/// Symmetric negative #2: full clip on login + option on → no-op.
/// `maybe_trigger_reload_on_activate` has a `active_ammo() <
/// active_clip_size()` gate; this guard pins that gate at the
/// handler boundary so a future refactor that removes it (e.g.
/// to "always reload on activate") would trip here.
#[tokio::test]
async fn init_player_state_does_not_reload_when_clip_full() {
    let mut mgr = make_mgr();
    let engine = ChainEngine::new();
    let (tx, _rx) = mpsc::channel(32);

    let (archetype, abilities, slot, items, sys_opts) = init_args_with_bandolier_clip(
        30, // already full
        SystemOptions {
            auto_reload: true,
            reload_on_activate: true,
        },
    );

    handle_init_player_state(
        1,
        100,
        "Castle_CellBlock".into(),
        archetype,
        vec![],
        abilities,
        slot,
        items,
        sys_opts,
        0, // state_field
        0, // access_level
        &tx,
        &mut mgr,
        &engine,
    )
    .await;

    let e = mgr.get_entity(1).unwrap();
    assert!(
        e.reload_complete_at.is_none() && e.pending_reload_at.is_none(),
        "full-clip InitPlayerState must NOT queue a reload"
    );
}
