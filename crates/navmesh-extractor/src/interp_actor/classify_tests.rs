//! `classify` and `name_rule` against hand-built evidence. The key
//! values are the ones NA40 measured in the cooked client (see
//! `docs/analysis/npc-ai-restoration/worknotes/na40-static-interp-actors.md`).

use super::*;

const AT: [f32; 3] = [100.0, 200.0, 300.0];

fn relative(positions: &[[f32; 3]], rotations: &[[f32; 3]]) -> MoveTrack {
    MoveTrack {
        frame: MoveFrame::RelativeToInitial,
        positions: positions.to_vec(),
        rotations: rotations.to_vec(),
    }
}

fn driven_by(moves: Vec<MoveTrack>, other: &[&str]) -> ActorMotion {
    ActorMotion {
        matinee: vec![MatineeGroup {
            seq_act: 7,
            group: "G".into(),
            tracks: Some(GroupTracks {
                moves,
                other: other.iter().map(|s| s.to_string()).collect(),
            }),
        }],
        other_refs: vec![],
    }
}

/// Ring-transport ring 5, Castle_CellBlock: up 330 cm and back.
fn ring() -> MoveTrack {
    relative(
        &[
            [0.0; 3],
            [0.0, 0.0, 330.0],
            [0.0, 0.0, 320.0],
            [0.0, 0.0, 330.0],
            [0.0; 3],
        ],
        &[[0.0; 3]; 5],
    )
}

/// Castle_CellBlock `OpenDoor`: slides 222 cm and stays there.
fn opening() -> MoveTrack {
    relative(&[[0.0; 3], [0.0, 222.0, 0.0]], &[[0.0; 3]; 2])
}

/// Security-camera sweep: yaw ±45° and back.
fn sweep() -> MoveTrack {
    relative(
        &[[0.0; 3]; 5],
        &[
            [0.0; 3],
            [0.0, 0.0, -45.0],
            [0.0; 3],
            [0.0, 0.0, 45.0],
            [0.0; 3],
        ],
    )
}

/// Every door, Stargate part and camera-head mesh that ships as an
/// `InterpActor` (NA36/NA40 census), plus a made-up future one.
const NEVER_BAKED: &[(&str, NameRule)] = &[
    ("SGC_Door03", NameRule::Door),
    ("SGC_small_door_00", NameRule::Door),
    ("EM-Door_Prison00", NameRule::Door),
    ("CA-CastleEntrance_Door00", NameRule::Door),
    ("CA-CastleEntrance_Door01", NameRule::Door),
    ("XX-BlastDOOR_Big01", NameRule::Door),
    ("GLB-Stargate01", NameRule::StargatePart),
    ("GLB-Stargate_Chevron00", NameRule::StargatePart),
    ("GLB-Stargate_Chevron_Light00", NameRule::StargatePart),
    ("GLB-Stargate_Spinner00", NameRule::StargatePart),
    ("EM-SecurityCam01_Top", NameRule::CameraHead),
];

#[test]
fn a_door_or_gate_part_is_never_included_whatever_the_evidence_says() {
    // The regression guard for the whole module: evidence that would
    // include any other actor (nothing references it; or a Matinee that
    // only lifts it and puts it back) must not bake a door, a Stargate
    // part or a camera head.
    let evidence = [
        ActorMotion::default(),
        driven_by(vec![ring()], &[]),
        driven_by(vec![], &["InterpTrackEvent"]),
    ];
    for (mesh, rule) in NEVER_BAKED {
        assert_eq!(name_rule(mesh), Some(*rule), "{mesh}");
        for motion in &evidence {
            let d = classify(motion, mesh, AT);
            assert_eq!(d, Decision::Exclude(ExcludeRule::Name(*rule)), "{mesh}");
            assert!(!d.is_included(), "{mesh}");
        }
    }
}

#[test]
fn the_static_dressing_that_ships_as_interp_actor_hits_no_name_rule() {
    for mesh in [
        "GLB-RingTransporter00",
        "HT-StreetLamp00",
        "HB-StreetLamp00",
        "HB-Humvee_02",
        "HT-FloatingLight01",
        "EM-ShelfBox10",
        "LUS-MetalBox00",
        "EM-Antenna00",
        "LUS-FanRotor00",
    ] {
        assert_eq!(name_rule(mesh), None, "{mesh}");
    }
}

#[test]
fn an_actor_no_kismet_references_is_included() {
    assert_eq!(
        classify(&ActorMotion::default(), "EM-ShelfBox10", AT),
        Decision::Include(IncludeRule::Unreferenced)
    );
}

#[test]
fn a_ring_that_rises_and_returns_is_included_at_its_rest_pose() {
    assert_eq!(
        classify(&driven_by(vec![ring()], &[]), "GLB-RingTransporter00", AT),
        Decision::Include(IncludeRule::RestAnchoredMatinee {
            max_vertical_cm: 330.0
        })
    );
}

#[test]
fn an_idling_vehicle_jiggle_is_within_both_limits() {
    // HB-Humvee_02, Lucia: 8 cm and 1 degree, back to rest.
    let jiggle = relative(
        &[[0.0; 3], [-8.0, 3.0, 0.0], [-8.0, 0.0, -6.0], [0.0; 3]],
        &[[0.0; 3], [-1.0, 1.0, 0.0], [0.0; 3], [0.0; 3]],
    );
    assert!(classify(&driven_by(vec![jiggle], &[]), "HB-Humvee_02", AT).is_included());
}

#[test]
fn a_mover_that_leaves_its_cooked_pose_is_excluded_even_without_a_door_name() {
    assert_eq!(
        classify(&driven_by(vec![opening()], &[]), "LUS-MetalBox00", AT),
        Decision::Exclude(ExcludeRule::MatineeLeavesRest { offset_cm: 222.0 })
    );
    // Starting away from rest is the same failure: the actor jumps to
    // the first key when the sequence plays.
    let starts_away = relative(&[[0.0, 0.0, 50.0], [0.0; 3]], &[[0.0; 3]; 2]);
    assert!(matches!(
        classify(&driven_by(vec![starts_away], &[]), "LUS-MetalBox00", AT),
        Decision::Exclude(ExcludeRule::MatineeLeavesRest { .. })
    ));
}

#[test]
fn one_group_that_leaves_rest_outweighs_another_that_does_not() {
    let mut motion = driven_by(vec![ring()], &[]);
    motion.matinee.push(MatineeGroup {
        seq_act: 8,
        group: "Open".into(),
        tracks: Some(GroupTracks {
            moves: vec![opening()],
            other: vec![],
        }),
    });
    assert!(matches!(
        classify(&motion, "GLB-RingTransporter00", AT),
        Decision::Exclude(ExcludeRule::MatineeLeavesRest { .. })
    ));
}

#[test]
fn a_mover_that_turns_is_excluded() {
    assert_eq!(
        classify(&driven_by(vec![sweep()], &[]), "EM-Antenna00", AT),
        Decision::Exclude(ExcludeRule::MatineeRotates { degrees: 45.0 })
    );
}

#[test]
fn a_round_trip_that_slides_sideways_is_excluded() {
    // SGC_Door03's travel, on a mesh the name rules would not catch.
    let slide = relative(&[[0.0; 3], [0.0, 96.0, 0.0], [0.0; 3]], &[[0.0; 3]; 3]);
    assert_eq!(
        classify(&driven_by(vec![slide], &[]), "LUS-MetalBox00", AT),
        Decision::Exclude(ExcludeRule::MatineeSlides {
            horizontal_cm: 96.0
        })
    );
}

#[test]
fn a_world_frame_track_is_measured_against_the_cooked_location() {
    // Castle_CellBlock `CloseDoor`: world keys, the first 2 cm from the
    // cooked location and the last 220 cm away.
    let loc = [-11447.0, -29231.0, 6545.0];
    let close = MoveTrack {
        frame: MoveFrame::World,
        positions: vec![[-11447.0, -29233.0, 6545.0], [-11447.0, -29011.0, 6545.0]],
        rotations: vec![[0.0, 0.0, -180.0], [0.0, 0.0, -180.0]],
    };
    assert!(matches!(
        classify(&driven_by(vec![close], &[]), "LUS-MetalBox00", loc),
        Decision::Exclude(ExcludeRule::MatineeLeavesRest { offset_cm }) if offset_cm > 200.0
    ));
    // The same frame, pinned to the cooked pose, rotation constant.
    let pinned = MoveTrack {
        frame: MoveFrame::World,
        positions: vec![loc, loc],
        rotations: vec![[0.0, 0.0, -180.0], [0.0, 0.0, -180.0]],
    };
    assert!(classify(&driven_by(vec![pinned], &[]), "LUS-MetalBox00", loc).is_included());
}

#[test]
fn an_unfamiliar_kismet_reference_is_undecided_a_benign_one_is_not() {
    let toggled = ActorMotion {
        matinee: vec![],
        other_refs: vec!["SeqAct_Toggle[Target]".into()],
    };
    assert_eq!(
        classify(&toggled, "EM-ShelfBox10", AT),
        Decision::Undecided(UndecidedReason::KismetReference(
            "SeqAct_Toggle[Target]".into()
        ))
    );
    let touched = ActorMotion {
        matinee: vec![],
        other_refs: vec!["SeqEvent_Touch.Originator".into()],
    };
    assert_eq!(
        classify(&touched, "EM-ShelfBox10", AT),
        Decision::Include(IncludeRule::BenignReferencesOnly)
    );
    // Lucia's street lamps: a bob Matinee plus a PlaySound on the lamp.
    let mut lamp = driven_by(
        vec![relative(
            &[[0.0; 3], [0.0, 0.0, -17.0], [0.0; 3]],
            &[[0.0; 3]; 3],
        )],
        &[],
    );
    lamp.other_refs.push("SeqAct_PlaySound[Target]".into());
    assert_eq!(
        classify(&lamp, "HT-StreetLamp00", AT),
        Decision::Include(IncludeRule::RestAnchoredMatinee {
            max_vertical_cm: 17.0
        })
    );
    // ...but a sound does not make a door bakeable.
    assert!(!classify(&lamp, "SGC_Door03", AT).is_included());
}

#[test]
fn an_unreadable_group_an_unknown_track_or_a_keyless_move_is_undecided() {
    let unreadable = ActorMotion {
        matinee: vec![MatineeGroup {
            seq_act: 3,
            group: "Lift".into(),
            tracks: None,
        }],
        other_refs: vec![],
    };
    assert!(matches!(
        classify(&unreadable, "EM-ShelfBox10", AT),
        Decision::Undecided(UndecidedReason::MatineeGroupUnreadable(_))
    ));
    assert_eq!(
        classify(
            &driven_by(vec![ring()], &["InterpTrackVisibility"]),
            "GLB-RingTransporter00",
            AT
        ),
        Decision::Undecided(UndecidedReason::UnknownTrack(
            "InterpTrackVisibility".into()
        ))
    );
    let keyless = relative(&[], &[]);
    assert!(matches!(
        classify(&driven_by(vec![keyless], &[]), "EM-ShelfBox10", AT),
        Decision::Undecided(UndecidedReason::MatineeGroupUnreadable(_))
    ));
}

#[test]
fn an_event_track_beside_the_move_does_not_change_the_answer() {
    assert!(classify(
        &driven_by(vec![ring()], &["InterpTrackEvent", "InterpTrackSound"]),
        "GLB-RingTransporter00",
        AT
    )
    .is_included());
}

#[test]
fn the_decision_log_names_the_rule_and_its_measurement() {
    assert_eq!(
        classify(&driven_by(vec![ring()], &[]), "GLB-RingTransporter00", AT).rule(),
        "matinee:rest-anchored rise=330cm"
    );
    assert_eq!(
        classify(&ActorMotion::default(), "SGC_Door03", AT).rule(),
        "name:door"
    );
    assert_eq!(
        classify(&driven_by(vec![sweep()], &[]), "EM-Antenna00", AT).verdict(),
        "exclude"
    );
}

#[test]
fn interp_actor_mode_parses_its_two_spellings_only() {
    assert_eq!(InterpActorMode::default(), InterpActorMode::Classify);
    for mode in [InterpActorMode::Off, InterpActorMode::Classify] {
        assert_eq!(InterpActorMode::parse(mode.label()), Ok(mode));
    }
    assert!(InterpActorMode::parse("true").is_err());
    assert!(InterpActorMode::parse("").is_err());
}

#[test]
fn the_decision_log_is_sorted_and_carries_bigworld_coordinates() {
    let record = |chunk: &str, export_index: usize| InterpActorRecord {
        chunk: chunk.into(),
        actor: "InterpActor_3".into(),
        export_index,
        mesh: "GLB-RingTransporter00".into(),
        location: [100.0, 200.0, 300.0],
        decision: Decision::Include(IncludeRule::Unreferenced),
        evidence: "-".into(),
    };
    let mut out = Vec::new();
    write_decision_log(
        &mut out,
        &[record("M-00000002", 1), record("M-00000001", 9)],
    )
    .unwrap();
    let text = String::from_utf8(out).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines[0], InterpActorRecord::TSV_HEADER);
    assert!(
        lines[1].starts_with("M-00000001\tInterpActor_3\t10\t"),
        "{}",
        lines[1]
    );
    // BigWorld (x, y, z) = UE3 (Y, Z, X) / 100.
    assert!(
        lines[1].contains("\t2.00\t3.00\t1.00\tinclude\tkismet:unreferenced\t-"),
        "{}",
        lines[1]
    );
    assert_eq!(tally(&[record("a", 0)]), (1, 0, 0));
}
