//! `resolve_respawn_target` — which (world, position) the respawn fork
//! is handed, across all four priorities. Split out of the monolithic
//! `combat/tests.rs` (CA00); the pre-existing test bodies are unchanged.

use super::super::respawn::resolve_respawn_target;
use super::make_mgr_with_player;
use crate::cell::spawner::RespawnerDef;

/// `resolve_respawn_target` matches a `respawner_id` to its stored
/// (world, pos) tuple. The id-match path is the primary one — pin it
/// so a refactor that drops the iter().find() doesn't fall back
/// silently to the world-default branch (which can pick a different
/// respawner if multiple are registered for the same world).
#[test]
fn resolve_respawn_target_uses_matching_respawner_id() {
    let mut mgr = make_mgr_with_player("Castle_CellBlock");
    mgr.respawners.push(RespawnerDef {
        respawner_id: 42,
        world_name: "Castle_CellBlock".to_string(),
        name: "Hub".to_string(),
        pos: [10.0, 20.0, 30.0],
    });
    let (world, pos) = resolve_respawn_target(42, 1, &mgr);
    assert_eq!(world, "Castle_CellBlock");
    assert_eq!(pos, [10.0, 20.0, 30.0]);
}

/// `resolve_respawn_target` falls back to the world's first respawner
/// when the requested id isn't found. Pin so the fallback path can't
/// silently degrade to the Castle default when the player's world has
/// its own respawner registered.
#[test]
fn resolve_respawn_target_falls_back_to_world_respawner_on_id_miss() {
    let mut mgr = make_mgr_with_player("Agnos_test");
    mgr.respawners.push(RespawnerDef {
        respawner_id: 7,
        world_name: "Agnos_test".to_string(),
        name: "Outpost".to_string(),
        pos: [-5.0, 5.0, -5.0],
    });
    // respawner_id 999 doesn't exist.
    let (world, pos) = resolve_respawn_target(999, 1, &mgr);
    assert_eq!(world, "Agnos_test");
    assert_eq!(pos, [-5.0, 5.0, -5.0]);
}

/// `resolve_respawn_target` returns the Castle default world+pos for
/// `Castle_CellBlock` when no respawners exist (ship-config
/// fallback). Pin the canonical fallback so a regression that uses
/// the in-place path inside Castle can't silently strand players at
/// their corpse.
#[test]
fn resolve_respawn_target_returns_castle_default_when_no_respawners() {
    let mgr = make_mgr_with_player("Castle_CellBlock");
    let (world, pos) = resolve_respawn_target(-1, 1, &mgr);
    assert_eq!(world, "Castle_CellBlock");
    assert_eq!(pos, [-334.231, 73.472, -228.026]);
}

/// `resolve_respawn_target` for non-Castle worlds with no respawner
/// falls back to in-place (current world, current position) — NOT a
/// cross-world snap to Castle. Pin the cross-world teleport-prevention
/// shape: if a content gap leaves a world without a respawner, the
/// player should respawn where they died, not get yanked across worlds.
#[test]
fn resolve_respawn_target_uses_in_place_for_other_worlds_without_respawners() {
    let mgr = make_mgr_with_player("Agnos_test");
    let (world, pos) = resolve_respawn_target(-1, 1, &mgr);
    assert_eq!(world, "Agnos_test");
    assert_eq!(
        pos,
        [42.0, 1.0, 17.0],
        "must respawn in place, not at Castle default"
    );
}

/// A respawner at the world origin is treated as absent on the
/// explicit-id path (the one the Defeat Window drives).
///
/// Bug shape (Castle audit defect B1): all four World 8 rows shipped as
/// `(0,0,0)`. The player clicks "Checkpoint Alpha Respawn" in the Defeat
/// Window, `callForAid` hands the id to `resolve_respawn_target`, the row
/// is found by id, and its zeros are returned — every fallback below is
/// unreachable *because the row exists*. Result: respawn at the world
/// origin, out of bounds, with `unstuck` unimplemented.
///
/// The origin row here belongs to a **different world** from the player's,
/// which does two things:
///
/// 1. It isolates priority 1. A row in the player's own world would also
///    be reachable via priority 2, so the test could pass for the wrong
///    reason (world scan skipping it) even if the explicit-id path still
///    returned zeros.
/// 2. It reproduces the worst real shape of B1. `respawner_id > 0` is
///    client-supplied and `resolve_respawn_target` never constrains it to
///    the player's world (see
///    `docs/security-audit/.../CAT-C-combat-abilities.md`), so pre-fix an
///    id pointing at another world's zero row returned
///    `("Castle", [0,0,0])` — a *cross-world* teleport to the origin,
///    which `handle_respawn` then routes down the GateTravel branch.
///
/// Both halves of the return value are asserted: with the guard removed
/// this yields `("Castle", [0.0, 0.0, 0.0])` instead of the in-place
/// `("Agnos_test", [42.0, 1.0, 17.0])`.
#[test]
fn resolve_respawn_target_skips_origin_respawner_on_explicit_id() {
    let mut mgr = make_mgr_with_player("Agnos_test");
    // Deliberately NOT the player's world.
    mgr.respawners.push(RespawnerDef {
        respawner_id: 3,
        world_name: "Castle".to_string(),
        name: "Unauthored Checkpoint".to_string(),
        pos: [0.0, 0.0, 0.0],
    });

    let (world, pos) = resolve_respawn_target(3, 1, &mgr);

    assert_eq!(
        world, "Agnos_test",
        "skipping the origin row must leave the player in their own world — \
         returning the origin row's world sends them through the cross-world \
         GateTravel branch of handle_respawn"
    );
    assert_eq!(
        pos,
        [42.0, 1.0, 17.0],
        "an all-zero respawner must be treated as absent and the search must \
         continue past it — returning (0,0,0) teleports the player to the world \
         origin (Castle audit defect B1)"
    );
}

/// The same skip applies on the world-match path (priority 2), and it
/// must not stop at the first origin row — the scan continues to a later
/// authored row for the same world.
///
/// Shape matters: the Castle seed had four rows for one world. A guard
/// that only checked `respawners.first()` would still hand back zeros
/// whenever the origin row happened to be first. With the guard removed
/// this returns the origin row's `[0.0, 0.0, 0.0]`.
#[test]
fn resolve_respawn_target_skips_origin_row_for_a_later_authored_one() {
    let mut mgr = make_mgr_with_player("Agnos_test");
    mgr.respawners.push(RespawnerDef {
        respawner_id: 1,
        world_name: "Agnos_test".to_string(),
        name: "Unauthored Checkpoint".to_string(),
        pos: [0.0, 0.0, 0.0],
    });
    mgr.respawners.push(RespawnerDef {
        respawner_id: 2,
        world_name: "Agnos_test".to_string(),
        name: "Outpost".to_string(),
        pos: [-5.0, 5.0, -5.0],
    });

    // No explicit id — priority 2 (first respawner for the player's world).
    let (world, pos) = resolve_respawn_target(-1, 1, &mgr);

    assert_eq!(world, "Agnos_test");
    assert_eq!(
        pos,
        [-5.0, 5.0, -5.0],
        "the world scan must skip the origin row and keep looking, not stop at it"
    );
}

/// The origin test is exact equality, not a tolerance band: a respawner
/// authored a few centimetres off the origin is a real respawn point and
/// must still be honoured.
///
/// Pins the guard's blast radius. `(0,0,0)` is a sentinel written by the
/// authoring gap; a proximity check would start silently discarding
/// legitimate coordinates in any world whose geometry straddles the
/// origin (`Castle_CellBlock`'s own default is 334 units out, but
/// `CombatSim` and the test spaces sit on top of it).
#[test]
fn resolve_respawn_target_honours_a_respawner_just_off_the_origin() {
    let mut mgr = make_mgr_with_player("Agnos_test");
    mgr.respawners.push(RespawnerDef {
        respawner_id: 4,
        world_name: "Agnos_test".to_string(),
        name: "Near Origin".to_string(),
        pos: [0.0, 0.05, 0.0],
    });

    let (world, pos) = resolve_respawn_target(4, 1, &mgr);

    assert_eq!(world, "Agnos_test");
    assert_eq!(
        pos,
        [0.0, 0.05, 0.0],
        "only an exactly-zero position is unauthored; a near-origin respawner is a \
         real respawn point"
    );
}

/// Negative-log guard for the explicit-id origin seam.
///
/// Per [`docs/architecture/negative-logging-convention.md`], a skip that
/// changes where the player lands must be greppable: an operator seeing
/// "I respawned somewhere odd" needs one query that names the offending
/// row. Without the log the guard is silent — the seed bug simply stops
/// being visible, which is how the origin rows survived in the first
/// place.
///
/// Pins level, `reason`, the identifying fields, and the *count*. Count
/// matters because the two seams sit on the same call path: a refactor
/// that let the world scan also fire for the explicit-id row would
/// double-log every respawn in an unauthored world.
#[test]
fn origin_respawner_warn_fires_once_on_the_explicit_id_path() {
    use crate::test_support::LogCapture;
    use tracing::Level;

    let mut mgr = make_mgr_with_player("Agnos_test");
    mgr.respawners.push(RespawnerDef {
        respawner_id: 3,
        world_name: "Castle".to_string(),
        name: "Unauthored Checkpoint".to_string(),
        pos: [0.0, 0.0, 0.0],
    });

    let capture = LogCapture::install();
    let _ = resolve_respawn_target(3, 1, &mgr);

    let hits: Vec<_> = capture
        .all()
        .into_iter()
        .filter(|c| c.level == Level::WARN && c.has_field("reason", "respawner_at_origin"))
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "the explicit-id origin skip must emit exactly one WARN with \
         reason=respawner_at_origin; got {:#?}",
        capture.all()
    );

    let ev = &hits[0];
    assert!(
        ev.has_field("respawner_id", "3"),
        "must carry respawner_id so the operator can find the row: {ev:#?}"
    );
    assert!(
        ev.has_field("respawner_name", "Unauthored Checkpoint"),
        "must carry respawner_name — the id alone doesn't say which checkpoint \
         the player clicked: {ev:#?}"
    );
    assert!(
        ev.has_field("world", "Castle"),
        "must carry the offending row's world (field name `world`, matching the \
         other respawn-resolution logs): {ev:#?}"
    );
    assert!(
        ev.has_field("entity_id", "1"),
        "must carry entity_id per the convention's field-naming rules: {ev:#?}"
    );
    assert!(
        ev.message_contains("db/resources/Worlds/Seed/respawners.sql"),
        "the message must name the file to fix — that is the whole point of \
         the seam: {ev:#?}"
    );

    // The world-scan seam must NOT also fire: the player's own world has
    // no respawners at all here, so nothing was skipped for it.
    assert!(
        capture
            .all()
            .iter()
            .all(|c| !c.has_field("reason", "world_respawners_all_at_origin")),
        "the world-scan seam must stay silent when no row for the player's \
         world was skipped"
    );
}

/// Negative-log guard for the world-scan origin seam, plus the silence
/// pin on the authored path.
///
/// `skipped` is the field that makes this seam actionable: it says how
/// many rows for the world were discarded, which is what tells an
/// operator whether the world has one bad row or is entirely unauthored
/// (the Castle case before CA00: four rows, all zeros).
#[test]
fn origin_respawner_warn_fires_once_on_the_world_scan_path() {
    use crate::test_support::LogCapture;
    use tracing::Level;

    let mut mgr = make_mgr_with_player("Agnos_test");
    for (id, name) in [(1, "Unauthored A"), (2, "Unauthored B")] {
        mgr.respawners.push(RespawnerDef {
            respawner_id: id,
            world_name: "Agnos_test".to_string(),
            name: name.to_string(),
            pos: [0.0, 0.0, 0.0],
        });
    }

    let capture = LogCapture::install();
    // No explicit id — straight to priority 2.
    let _ = resolve_respawn_target(-1, 1, &mgr);

    let hits: Vec<_> = capture
        .all()
        .into_iter()
        .filter(|c| {
            c.level == Level::WARN && c.has_field("reason", "world_respawners_all_at_origin")
        })
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "an exhausted world scan must emit exactly one WARN with \
         reason=world_respawners_all_at_origin — one per resolve, not one per \
         skipped row; got {:#?}",
        capture.all()
    );

    let ev = &hits[0];
    assert!(
        ev.has_field("world", "Agnos_test"),
        "must carry the player's world: {ev:#?}"
    );
    assert!(
        ev.has_field("skipped", "2"),
        "must carry how many rows were skipped — 1 bad row and a wholly \
         unauthored world need different fixes: {ev:#?}"
    );
    assert!(
        ev.has_field("entity_id", "1"),
        "must carry entity_id per the convention's field-naming rules: {ev:#?}"
    );
    assert!(
        ev.message_contains("db/resources/Worlds/Seed/respawners.sql"),
        "the message must name the file to fix: {ev:#?}"
    );

    drop(capture);

    // Authored path: one usable row for the world. Neither seam may fire —
    // a warn on the healthy path is noise that trains operators to ignore
    // the real one.
    let mut ok_mgr = make_mgr_with_player("Agnos_test");
    ok_mgr.respawners.push(RespawnerDef {
        respawner_id: 9,
        world_name: "Agnos_test".to_string(),
        name: "Outpost".to_string(),
        pos: [-5.0, 5.0, -5.0],
    });

    let quiet = LogCapture::install();
    let (_, pos) = resolve_respawn_target(9, 1, &ok_mgr);
    assert_eq!(
        pos,
        [-5.0, 5.0, -5.0],
        "fixture sanity: authored row is used"
    );
    assert!(
        quiet.all().iter().all(|c| {
            !c.has_field("reason", "respawner_at_origin")
                && !c.has_field("reason", "world_respawners_all_at_origin")
        }),
        "neither origin seam may fire when the respawner is authored: {:#?}",
        quiet.all()
    );
}
