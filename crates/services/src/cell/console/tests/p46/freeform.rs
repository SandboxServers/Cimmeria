//! GM free-form destinations: the two ways a GM reaches somewhere the
//! world-name table wouldn't otherwise let them.
//!
//! Not a legacy packet — legacy had neither behaviour. Both live here
//! because they are the same two adapters ([`super::super::move_subject`]
//! and P45's transfer primitive) the rest of this suite exercises, and they
//! have to keep agreeing with it.
//!
//! 1. **Case-insensitive world names.** `.gotolocation` takes its world name
//!    off a chat line, but every world lookup underneath is an exact-match
//!    `HashMap` hit. A GM typing `harset` got `"Unable to find world:
//!    harset"` with `Harset` sitting in `spaces.xml` — the live bug these
//!    tests guard.
//! 2. **`.gotospace`** — name the destination by loaded space id, so the
//!    world-name table isn't consulted at all and any live instance is
//!    reachable, including one nobody is standing in.

use super::*;

/// **Live-bug regression guard.** Reverting the `canonical_world_name`
/// resolution makes this report `"Unable to find world: castle"` and move
/// nothing.
#[tokio::test]
async fn gotolocation_resolves_a_world_name_case_insensitively() {
    let (mut mgr, gm, _npc) = setup_worlds();
    let castle = mgr
        .default_space_for_world(CASTLE)
        .expect("Castle has a startup space");

    let t = run(
        "gotolocation",
        gm,
        &[&CASTLE.to_lowercase(), "70", "1", "80"],
        None,
        &mut mgr,
    )
    .await;

    assert!(
        !t.mentions("Unable to find world"),
        "a case variant of a declared world must not dead-end; got {:?}",
        t.feedback
    );
    assert_eq!(
        t.only_gate_travel(),
        &(gm, CASTLE.to_string(), Some(castle), [70.0, 1.0, 80.0]),
        "the transfer must carry the canonical spaces.xml spelling"
    );
}

/// The same-world shortcut is case-insensitive too, or a GM correcting their
/// coordinates with a lowercase world name pays a full loading screen (and,
/// on an instanced world, lands in someone else's instance).
#[tokio::test]
async fn gotolocation_same_world_case_insensitive_stays_an_in_place_snap() {
    let (mut mgr, gm, _npc) = setup_worlds();
    let agnos = mgr.get_entity_space_id(gm).unwrap();
    let before = position_of(&mgr, gm);

    let t = run(
        "gotolocation",
        gm,
        &[&AGNOS.to_uppercase(), "40", "0", "40"],
        None,
        &mut mgr,
    )
    .await;

    assert!(
        t.gate_travels.is_empty(),
        "naming the caller's own world in a different case must not enqueue a \
         transfer: {:?}",
        t.gate_travels
    );
    assert_eq!(t.teleports, vec![(gm, agnos, [40.0, 0.0, 40.0], before)]);
    assert_eq!(position_of(&mgr, gm), [40.0, 0.0, 40.0]);
}

// ── .gotospace ───────────────────────────────────────────────────────────

/// The escape hatch: a specific loaded instance, named by id, with the world
/// name derived from it rather than typed. `.goto <player>` can only reach an
/// instance somebody is standing in; this reaches any of them.
#[tokio::test]
async fn gotospace_reaches_an_exact_instance_by_id() {
    let (mut mgr, gm, _npc) = setup_worlds();
    // A second instance of the instanced world, and NOT the one D15's
    // default rule would pick — so resolving by id is the only way here.
    let instance_a = spawn_named_player(&mut mgr, 50, INSTANCED, [1.0, 0.0, 1.0], "Ana");
    let instance_b = spawn_named_player(&mut mgr, 51, INSTANCED, [2.0, 0.0, 2.0], "Bob");
    assert_ne!(instance_a, instance_b);
    assert_eq!(
        mgr.default_space_for_world(INSTANCED),
        Some(instance_a.min(instance_b)),
        "fixture assumes the default rule picks the OTHER instance"
    );
    let target_instance = instance_a.max(instance_b);

    let t = run(
        "gotospace",
        gm,
        &[&target_instance.to_string(), "55", "0", "66"],
        None,
        &mut mgr,
    )
    .await;

    assert_eq!(
        t.only_gate_travel(),
        &(
            gm,
            INSTANCED.to_string(),
            Some(target_instance),
            [55.0, 0.0, 66.0]
        ),
        "the id must survive to the wire verbatim, with the world name derived \
         from it — got {:?}",
        t.gate_travels
    );
}

/// Naming the caller's own space is the cheap in-place snap, same as every
/// other travel command — no teardown, no loading screen.
#[tokio::test]
async fn gotospace_into_the_callers_own_space_snaps_in_place() {
    let (mut mgr, gm, _npc) = setup_worlds();
    let agnos = mgr.get_entity_space_id(gm).unwrap();
    let before = position_of(&mgr, gm);

    let t = run(
        "gotospace",
        gm,
        &[&agnos.to_string(), "33", "0", "44"],
        None,
        &mut mgr,
    )
    .await;

    assert!(t.gate_travels.is_empty(), "{:?}", t.gate_travels);
    assert_eq!(t.teleports, vec![(gm, agnos, [33.0, 0.0, 44.0], before)]);
    assert_eq!(position_of(&mgr, gm), [33.0, 0.0, 44.0]);
}

/// An id that isn't a loaded instance is refused before anything moves —
/// the same validate-before-teardown contract every other travel leg honours.
#[tokio::test]
async fn gotospace_refuses_an_unloaded_space_id() {
    let (mut mgr, gm, _npc) = setup_worlds();
    let before = position_of(&mgr, gm);
    let space = mgr.get_entity_space_id(gm);

    let t = run(
        "gotospace",
        gm,
        &["999999", "10", "0", "10"],
        None,
        &mut mgr,
    )
    .await;

    assert_no_move(&t);
    assert!(
        t.mentions("not loaded"),
        "an unloaded space id must say so; got {:?}",
        t.feedback
    );
    assert_eq!(position_of(&mgr, gm), before);
    assert_eq!(mgr.get_entity_space_id(gm), space);
}

/// Space ids are `u32` on the wire but the console parses `i32`, so the
/// negative range has to be refused explicitly rather than wrapping into a
/// huge id that then reports "not loaded" for the wrong reason.
#[tokio::test]
async fn gotospace_refuses_a_negative_space_id() {
    let (mut mgr, gm, _npc) = setup_worlds();
    let before = position_of(&mgr, gm);

    let t = run("gotospace", gm, &["-1", "10", "0", "10"], None, &mut mgr).await;

    assert_no_move(&t);
    assert!(
        t.mentions("positive"),
        "a negative space id must be refused on its own terms; got {:?}",
        t.feedback
    );
    assert_eq!(position_of(&mgr, gm), before);
}

/// Zero is neither negative nor a space id — `allocate_space_id` builds every
/// id as `(cell_id << 16) | local` with `cell_id >= 1`, so `0` can never name
/// a loaded instance.
///
/// `u32::try_from` only filters the negative half, so before the explicit
/// check `.gotospace 0` fell through to the loaded-instance lookup and came
/// back "space 0 is not loaded" — telling the GM their id was plausible but
/// stale when in fact no id of that shape can ever exist. Reverting to the
/// bare `try_from` makes this find "not loaded" instead of "positive".
#[tokio::test]
async fn gotospace_refuses_a_zero_space_id() {
    let (mut mgr, gm, _npc) = setup_worlds();
    let before = position_of(&mgr, gm);

    let t = run("gotospace", gm, &["0", "10", "0", "10"], None, &mut mgr).await;

    assert_no_move(&t);
    assert!(
        t.mentions("positive"),
        "zero must be refused for the same reason a negative id is, not \
         reported as an instance that happens not to be loaded; got {:?}",
        t.feedback
    );
    assert_eq!(position_of(&mgr, gm), before);
}

/// **The promise `.gotospace` exists to keep.** A cross-instance move to a
/// live space must not be re-validated against the world-name table: the
/// space id *is* the runtime identity of a loaded instance, and the world
/// name is derived from it rather than typed.
///
/// Before the by-id transfer entry point, only the same-space fast path
/// actually bypassed the table — the cross-space leg derived a world name
/// from the instance and then handed it to `transfer_player_to_space`, which
/// ran it straight back through `canonical_world_name`. A live instance whose
/// world the table does not declare therefore dead-ended on `UnknownWorld`,
/// which is precisely the destination the escape hatch is for. Reverting
/// `move_subject` to the name-based transfer makes this report
/// `"Unable to find world: Ghost_Instance"` and move nothing.
#[tokio::test]
async fn gotospace_reaches_a_live_instance_whose_world_is_not_in_the_table() {
    let (mut mgr, gm, _npc) = setup_worlds();
    // A loaded space whose world `spaces.xml` never declared. Built directly
    // because both production creation paths gate on the world table — which
    // is the point: the table and the live space set are separate sources of
    // truth, and this command answers to the second one.
    const GHOST: &str = "Ghost_Instance";
    let ghost = mgr.allocate_space_id();
    mgr.create_space_instance(ghost, GHOST);
    assert!(
        mgr.canonical_world_name(GHOST).is_none(),
        "fixture precondition: the destination world must be absent from the table"
    );

    let t = run(
        "gotospace",
        gm,
        &[&ghost.to_string(), "12", "0", "34"],
        None,
        &mut mgr,
    )
    .await;

    assert!(
        !t.mentions("Unable to find world"),
        "a loaded instance must be reachable by id regardless of the world \
         table; got {:?}",
        t.feedback
    );
    assert_eq!(
        t.only_gate_travel(),
        &(gm, GHOST.to_string(), Some(ghost), [12.0, 0.0, 34.0]),
        "the transfer must carry the id verbatim and the world name derived \
         from it — got {:?}",
        t.gate_travels
    );
}

/// Non-finite coordinates take the shared `parse_f32` filter, before either
/// move mechanism — same contract as `.gotolocation`.
#[tokio::test]
async fn gotospace_rejects_non_finite_coordinates() {
    for bad in ["NaN", "inf", "-inf"] {
        let (mut mgr, gm, _npc) = setup_worlds();
        let agnos = mgr.get_entity_space_id(gm).unwrap();
        let before = position_of(&mgr, gm);

        let t = run(
            "gotospace",
            gm,
            &[&agnos.to_string(), bad, "0", "0"],
            None,
            &mut mgr,
        )
        .await;

        assert_no_move(&t);
        assert_eq!(
            position_of(&mgr, gm),
            before,
            "{bad} must not move anything"
        );
        assert!(
            t.mentions("finite"),
            "{bad} must feed back a finite-number rejection; got {:?}",
            t.feedback
        );
    }
}
