//! `handle_reload` holster-interaction tests: the two-phase
//! reload-while-holstered choreography (Phase A defer / Phase B promote),
//! OOC holster-timer re-stamping, the draw-window second-press gate, the
//! in-combat-flag isolation guard, and Phase B cancelling an in-flight
//! holster Phase 2.

use super::super::*;
use super::make_mgr_with_player;
use cimmeria_entity::abilities::AbilityDef;
use cimmeria_entity::cell_entity::BandolierItem;

/// Reload-while-holstered Phase A: a player who's OOC and
/// holstered presses reload. The handler defers the actual reload
/// to give the draw animation time to play. Phase A must:
///   1. Flip `weapon_holstered` to false.
///   2. Stamp `combat_exit_at` so the OOC re-holster timer fires
///      AFTER the eventual Phase B reload completes.
///   3. Set `pending_reload_at = now + UNHOLSTER_DRAW_DURATION` so
///      `pending_reload_tick` can promote Phase A → Phase B.
///   4. Dispatch `RefreshAppearance` (mesh attaches at hand socket).
///   5. NOT start the reload-completion timer or fire `Item_Reload`
///      yet — those land in Phase B.
///
/// Bug shape this catches (the playtest report that drove the fix):
/// firing `Item_Reload` and the appearance change in the same tick
/// makes the weapon "teleport into the hand + reload anim plays on
/// empty space", and the player has to press reload twice.
#[tokio::test]
async fn reload_while_holstered_phase_a_defers_reload() {
    let mut mgr = make_mgr_with_player();
    if let Some(e) = mgr.get_entity_mut(1) {
        e.archetype_id = Some(1);
        e.weapon_visual = Some("WP-Human.WP_Pistol_1A".into());
        e.weapon_holstered = true; // OOC + holstered
        e.combat_exit_at = None;
        e.pending_reload_at = None;
        e.bandolier_items.insert(
            0,
            BandolierItem {
                instance_id: 0,
                item_id: 1,
                clip_size: 30,
                default_ammo_type: 2,
                current_ammo: 0,
                cur_ammo_type: 2,
            },
        );
        e.active_bandolier_slot = 0;
    }

    let (tx, mut rx) = mpsc::channel(64);
    handle_reload(1, &tx, &mut mgr).await;

    let e = mgr.get_entity(1).unwrap();
    assert!(!e.weapon_holstered, "Phase A must draw the weapon");
    assert!(
        e.combat_exit_at.is_some(),
        "Phase A must stamp combat_exit_at so OOC re-holster fires AFTER \
         the eventual reload completes",
    );
    assert!(
        e.pending_reload_at.is_some(),
        "Phase A must set pending_reload_at so the deferred-reload tick \
         can promote to Phase B once the draw window elapses",
    );
    assert!(
        e.reload_complete_at.is_none(),
        "Phase A must NOT start the reload-completion timer — the actual \
         reload hasn't started yet, only the draw. Firing the reload here \
         is the bug shape we're explicitly avoiding (user playtest: \
         'weapon teleports into my hand and I still need to hit reload again')",
    );

    let mut saw_refresh = false;
    while let Ok(msg) = rx.try_recv() {
        if matches!(
            msg,
            CellToBaseMsg::RefreshAppearance {
                holstered: false,
                ..
            }
        ) {
            saw_refresh = true;
            break;
        }
    }
    assert!(
        saw_refresh,
        "Phase A must dispatch RefreshAppearance(holstered=false) so the \
         client attaches the weapon mesh at the hand socket before the \
         draw animation triggers",
    );
}

/// Phase A → Phase B promotion: once the draw window has
/// elapsed, calling `handle_reload` again (as the
/// `pending_reload_tick` does) finds `pending_reload_at` set,
/// clears it, and runs the normal Phase B reload start
/// (`reload_complete_at` armed, `Item_Reload` sequence fired).
///
/// Bug shape this catches: a refactor that forgets to clear
/// `pending_reload_at` in Phase B leaves the tick re-firing
/// `handle_reload` every 100ms forever.
#[tokio::test]
async fn reload_phase_a_to_phase_b_clears_pending_and_starts_reload() {
    let mut mgr = make_mgr_with_player();
    if let Some(e) = mgr.get_entity_mut(1) {
        e.archetype_id = Some(1);
        e.weapon_visual = Some("WP-Human.WP_Pistol_1A".into());
        // Already drawn by Phase A; `pending_reload_at` is what the
        // promotion key reads.
        e.weapon_holstered = false;
        e.combat_exit_at = Some(std::time::Instant::now());
        e.pending_reload_at = Some(std::time::Instant::now());
        e.bandolier_items.insert(
            0,
            BandolierItem {
                instance_id: 0,
                item_id: 1,
                clip_size: 30,
                default_ammo_type: 2,
                current_ammo: 0,
                cur_ammo_type: 2,
            },
        );
        e.active_bandolier_slot = 0;
    }
    mgr.ability_defs.insert(
        596,
        AbilityDef {
            ability_id: 596,
            name: "reload".to_string(),
            cooldown: 1.0,
            warmup: 0.5,
            flags: 0,
            is_ranged: false,
            min_range: 0,
            max_range: 0,
            target_type_id: 0,
            effect_ids: vec![],
            moniker_ids: vec![],
            required_ammo: 0,
            event_set_id: None,
            velocity: 0.0,
        },
    );

    let (tx, _rx) = mpsc::channel(64);
    handle_reload(1, &tx, &mut mgr).await;

    let e = mgr.get_entity(1).unwrap();
    assert!(
        e.pending_reload_at.is_none(),
        "Phase B must clear pending_reload_at so the tick doesn't re-fire \
         handle_reload every 100ms forever",
    );
    assert!(
        e.reload_complete_at.is_some(),
        "Phase B must start the reload (set reload_complete_at) so the \
         completion tick can promote the ammo refill",
    );
}

/// Reload-while-in-OOC-grace (weapon already drawn): the timer
/// must be RE-STAMPED so it doesn't fire `OOC_HOLSTER_DELAY`
/// seconds after combat ended — which could land mid-reload and
/// holster the weapon while the animation is still playing.
#[tokio::test]
async fn reload_during_ooc_grace_resets_holster_timer() {
    let mut mgr = make_mgr_with_player();
    let stale_stamp = std::time::Instant::now() - std::time::Duration::from_secs(8);
    if let Some(e) = mgr.get_entity_mut(1) {
        e.archetype_id = Some(1);
        e.weapon_visual = Some("WP-Human.WP_Pistol_1A".into());
        e.weapon_holstered = false; // OOC but still drawn
        e.combat_exit_at = Some(stale_stamp);
        e.bandolier_items.insert(
            0,
            BandolierItem {
                instance_id: 0,
                item_id: 1,
                clip_size: 30,
                default_ammo_type: 2,
                current_ammo: 0,
                cur_ammo_type: 2,
            },
        );
        e.active_bandolier_slot = 0;
    }

    let (tx, _rx) = mpsc::channel(64);
    handle_reload(1, &tx, &mut mgr).await;

    let e = mgr.get_entity(1).unwrap();
    assert!(!e.weapon_holstered, "already-drawn weapon stays drawn");
    let new_stamp = e.combat_exit_at.expect("timer must remain armed");
    assert!(
        new_stamp > stale_stamp,
        "timer must be re-stamped to current time so the existing \
         OOC_HOLSTER_DELAY countdown doesn't expire mid-reload",
    );
}

/// Second reload press during the Phase A draw window must be
/// silently ignored. Without this gate, the second press falls
/// through to Phase B, clears `pending_reload_at` early, and
/// starts the reload cooldown immediately — defeating the draw
/// animation timing.
///
/// Bug shape: refactor drops the `now < pending_reload_at` check
/// at the top of Phase B; a player mashing R during the draw
/// window triggers Phase B prematurely and the reload anim
/// chains in mid-draw (the symptom that drove the original
/// two-phase split).
#[tokio::test]
async fn reload_second_press_during_draw_window_is_ignored() {
    let mut mgr = make_mgr_with_player();
    let future = std::time::Instant::now() + std::time::Duration::from_millis(800);
    if let Some(e) = mgr.get_entity_mut(1) {
        e.archetype_id = Some(1);
        e.weapon_visual = Some("WP-Human.WP_Pistol_1A".into());
        e.weapon_holstered = false; // weapon drawn (Phase A finished its draw)
        e.combat_exit_at = Some(std::time::Instant::now());
        e.bandolier_items.insert(
            0,
            BandolierItem {
                instance_id: 0,
                item_id: 1,
                clip_size: 30,
                default_ammo_type: 2,
                current_ammo: 0,
                cur_ammo_type: 2,
            },
        );
        e.active_bandolier_slot = 0;
        // Phase A already fired — Phase B is queued for the future.
        e.pending_reload_at = Some(future);
    }
    // No reload ability def needed — the gate fires before any
    // ability lookup.

    let (tx, _rx) = mpsc::channel(64);
    handle_reload(1, &tx, &mut mgr).await;

    let e = mgr.get_entity(1).unwrap();
    assert_eq!(
        e.pending_reload_at,
        Some(future),
        "second press must NOT clear pending_reload_at — the \
         tick still owns the Phase B promotion at the right time",
    );
    assert!(
        e.reload_complete_at.is_none(),
        "second press must NOT start the reload cooldown — Phase B \
         would otherwise fire mid-draw and chain the reload \
         animation before the unholster motion finishes",
    );
}

/// Reload-in-isolation regression: reloading without any aggro must
/// NOT flip BSF_InCombat on the player. The previous bug: the
/// reload handler set the bit raw, but reload doesn't generate
/// threat on anything — so no NPC death would ever clear the bit,
/// stranding the player in the in-combat HUD/cursor forever (and
/// blocking the out-of-combat regen tick, which gates on
/// `threatened_mobs.is_empty()`).
///
#[tokio::test]
async fn reload_in_isolation_does_not_flip_bsf_in_combat() {
    use crate::cell::combat::BSF_IN_COMBAT;

    let mut mgr = make_mgr_with_player();
    if let Some(e) = mgr.get_entity_mut(1) {
        e.bandolier_items.insert(
            0,
            BandolierItem {
                instance_id: 0,
                item_id: 1,
                clip_size: 30,
                default_ammo_type: 2,
                current_ammo: 0,
                cur_ammo_type: 2,
            },
        );
        e.active_bandolier_slot = 0;
    }
    // Seed the reload AbilityDef so the warmup path runs.
    mgr.ability_defs.insert(
        596,
        AbilityDef {
            ability_id: 596,
            name: "reload".to_string(),
            cooldown: 1.0,
            warmup: 0.5,
            flags: 0,
            is_ranged: false,
            min_range: 0,
            max_range: 0,
            target_type_id: 0,
            effect_ids: vec![],
            moniker_ids: vec![],
            required_ammo: 0,
            event_set_id: None,
            velocity: 0.0,
        },
    );

    let (tx, _rx) = mpsc::channel(64);
    handle_reload(1, &tx, &mut mgr).await;

    let s = mgr.get_entity(1).unwrap().state_field;
    assert_eq!(
        s & BSF_IN_COMBAT,
        0,
        "reload MUST NOT flip BSF_InCombat — reload-without-aggro had no \
         NPC-death clear path and the bit would strand forever"
    );
    assert!(
        mgr.get_entity(1).unwrap().threatened_mobs.is_empty(),
        "reload must leave threatened_mobs empty — the source of truth \
         for the in-combat state"
    );
}

/// `handle_reload` Phase B must cancel any in-flight OOC holster
/// Phase 2 (`holster_animation_complete_at`). Reload semantics imply
/// "the weapon stays drawn"; without this cancel, the Phase 2
/// deadline still elapses mid-reload and broadcasts a fresh
/// `BeingAppearance` with no weapon attached. The user sees the
/// reload animation play, then the weapon mesh vanishes on the
/// next AoI tick.
///
/// Bug shape: player drains clip with `autoReload = false`,
/// watches OOC holster Phase 1 fire (`Item_Unequip` animation),
/// then manually presses R during the ~half-second animation
/// window before Phase 2 elapses. Pre-fix Phase B was missing the
/// `holster_animation_complete_at = None` clear that Phase A
/// already had (the Phase A path), so the holster timer kept running.
///
/// Pin: stage `holster_animation_complete_at = Some(future)`,
/// drain the clip, call `handle_reload` with the weapon drawn
/// (forcing the Phase B path), assert the field is `None`
/// afterward AND a fresh reload deadline is armed.
#[tokio::test]
async fn handle_reload_phase_b_cancels_in_flight_holster_phase_2() {
    use std::time::{Duration, Instant};

    let mut mgr = make_mgr_with_player();
    if let Some(e) = mgr.get_entity_mut(1) {
        // Weapon drawn — forces Phase B (drawn-reload) path, not
        // Phase A (reload-while-holstered).
        e.weapon_holstered = false;
        e.bandolier_items.insert(
            0,
            BandolierItem {
                instance_id: 0,
                item_id: 1,
                clip_size: 30,
                default_ammo_type: 2,
                current_ammo: 0, // empty → reload will arm
                cur_ammo_type: 2,
            },
        );
        e.active_bandolier_slot = 0;
        // Stage the in-flight Phase 2 holster — Phase 1 fired
        // moments ago, animation is playing, mesh-drop deadline is
        // ~half a second out.
        e.holster_animation_complete_at = Some(Instant::now() + Duration::from_millis(500));
    }
    mgr.ability_defs.insert(
        596,
        AbilityDef {
            ability_id: 596,
            name: "reload".to_string(),
            cooldown: 1.0,
            warmup: 0.5,
            flags: 0,
            is_ranged: false,
            min_range: 0,
            max_range: 0,
            target_type_id: 0,
            effect_ids: vec![],
            moniker_ids: vec![],
            required_ammo: 0,
            event_set_id: None,
            velocity: 0.0,
        },
    );

    let (tx, _rx) = mpsc::channel(16);
    handle_reload(1, &tx, &mut mgr).await;

    let e = mgr.get_entity(1).unwrap();
    assert!(
        e.holster_animation_complete_at.is_none(),
        "Phase B must clear holster_animation_complete_at — reload \
         semantics imply weapon stays drawn. Pre-fix, Phase 2 would \
         still elapse mid-reload and drop the weapon mesh."
    );
    assert!(
        e.reload_complete_at.is_some(),
        "fixture sanity: Phase B must arm reload_complete_at — if this \
         fails, the test isn't exercising the Phase B branch"
    );
}
