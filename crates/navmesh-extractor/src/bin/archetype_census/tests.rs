//! Asset-free tests for the census. The cooked client tree can never
//! be committed, so every one of these drives the real walk over
//! synthetic packages built by
//! [`cimmeria_navmesh_extractor::test_support`].

use cimmeria_navmesh_extractor::test_support::{
    index_over, prefab_package, scratch_dir, ChunkFixture, PrefabInstanceSpec, PrefabSpec,
    StaticMeshPayload,
};
use cimmeria_navmesh_extractor::transform::ActorTransform;
use cimmeria_upk_objects::PackageIndex;

use super::census::Census;
use super::geometry::{area3, areas, ue3_to_bw, xz_area, MIN_UP};
use super::*;

fn argv(s: &[&str]) -> Vec<String> {
    s.iter().map(|x| x.to_string()).collect()
}

/// A chunk on disk plus an index over the directory it lives in.
struct Scene {
    dir: PathBuf,
    pkg: Package,
    index: PackageIndex,
}

fn scene(tag: &str, id: u32, build: impl FnOnce(&Path, &mut ChunkFixture)) -> Scene {
    let dir = scratch_dir(tag);
    let mut chunk = ChunkFixture::new();
    build(&dir, &mut chunk);
    let path = chunk.write(&dir, "Fix", id);
    Scene {
        pkg: Package::open(&path).expect("open chunk"),
        index: index_over(&dir),
        dir,
    }
}

fn census_of(scene: &Scene) -> Census {
    let mut census = Census::default();
    let mut cache = ArchetypeCache::default();
    let mut mesh_cache = MeshCache::new();
    census.add_chunk(
        "Fix-chunk",
        &scene.pkg,
        &scene.index,
        &mut cache,
        &mut mesh_cache,
    );
    census
}

fn report_of(census: &Census) -> String {
    let mut out: Vec<u8> = Vec::new();
    report::write_report(&mut out, "Fix", 1, census, (0, 0), 0).unwrap();
    String::from_utf8(out).unwrap()
}

// ---------- geometry ----------

#[test]
fn the_axis_map_puts_ue3_z_on_bigworld_y() {
    // `bw = (ue.Y, ue.Z, ue.X) / 100`. Getting this wrong reports every
    // instance's height as its easting.
    assert_eq!(ue3_to_bw([100.0, 200.0, 300.0]), [2.0, 3.0, 1.0]);
}

#[test]
fn xz_area_is_the_shadow_and_area3_is_the_surface() {
    // A triangle tilted 60 degrees off horizontal: its footprint is
    // half its true area (cos 60 == 0.5).
    let t = [[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [0.0, 3.0f32.sqrt(), 1.0]];
    assert!((xz_area(&t) - 1.0).abs() < 1e-4, "{}", xz_area(&t));
    assert!((area3(&t) - 2.0).abs() < 1e-4, "{}", area3(&t));
}

#[test]
fn only_the_winding_recast_reads_as_floor_counts_as_walkable() {
    // The sign that decides whether "missing geometry" is a surface
    // you can stand on. NavBuilder reverses the winding on load, so a
    // floor's emitted-order normal points *down*.
    let xf = ActorTransform {
        location: [0.0; 3],
        rotation: [0; 3],
        draw_scale: 1.0,
        draw_scale_3d: [1.0; 3],
    };
    // UE3 cm, Z up. This order gives `recast_up = +1` in BigWorld.
    let floor = [[0.0, 0.0, 0.0], [0.0, 100.0, 0.0], [100.0, 0.0, 0.0]];
    let ceiling = [floor[0], floor[2], floor[1]];

    let (fp, walk) = areas(&[floor], &xf);
    assert!((fp - 0.5).abs() < 1e-6, "0.5 m^2 footprint, got {fp}");
    assert!((walk - 0.5).abs() < 1e-6, "a floor is walkable, got {walk}");

    let (fp, walk) = areas(&[ceiling], &xf);
    assert!((fp - 0.5).abs() < 1e-6, "same shadow either way");
    assert_eq!(walk, 0.0, "a ceiling is not walkable");
}

#[test]
fn min_up_is_the_cosine_of_the_45_degree_slope_limit() {
    assert!((MIN_UP - 45.0f32.to_radians().cos()).abs() < 1e-6);
}

// ---------- the walk ----------

#[test]
fn a_prefab_stub_is_counted_measured_and_placed() {
    let s = scene("census-stub", 0x000a_0030, |dir, chunk| {
        prefab_package(
            dir,
            &PrefabSpec::local("Fx-Props", "Fx-Lamp_Pf0", "Fx-Lamp_Pf0_Arc1", "Fx-Lamp00"),
            &StaticMeshPayload::unit_triangle(),
        );
        chunk.add_prefab_instanced_actor(&PrefabInstanceSpec::new(
            "Lamp_0",
            [100.0, 200.0, 300.0],
            ("Fx-Props", "Fx-Lamp_Pf0", "Fx-Lamp_Pf0_Arc1"),
        ));
    });
    let census = census_of(&s);

    assert_eq!(census.stub_total, 1);
    assert_eq!(census.direct_total, 0);
    assert_eq!(census.placements.len(), 1);
    assert_eq!(census.failures, Default::default(), "{:?}", census.failures);
    assert!(census.read_failures.is_empty());
    let (name, stats) = census.stats.iter().next().unwrap();
    assert_eq!(name, "Fx-Props:Fx-Lamp00");
    assert_eq!(stats.instances, 1);
    assert_eq!(stats.tris_per_instance, 1);
    // bw = (ue.Y, ue.Z, ue.X) / 100.
    assert_eq!(census.placements[0].bw, [2.0, 3.0, 1.0]);

    let text = report_of(&census);
    assert!(text.contains("archetype_stub_components\t1"), "{text}");
    assert!(text.contains("resolved\t1"), "{text}");
    assert!(text.contains("unreadable_exports\t0"), "{text}");
    assert!(text.contains("Fx-Props:Fx-Lamp00"), "{text}");
    let _ = std::fs::remove_dir_all(&s.dir);
}

#[test]
fn an_unreadable_component_is_its_own_outcome_not_an_archetype_stub() {
    // `read_export_data(..).unwrap_or_default()` classifies a
    // truncated component as "no StaticMesh property", which is
    // exactly what a legitimate prefab stub looks like. The census
    // then resolves the archetype and reports collision, placement and
    // area for an actor whose real instance overrides were never read
    // -- a successful-looking but wrong row.
    let s = scene("census-unreadable", 0x000a_0031, |dir, chunk| {
        prefab_package(
            dir,
            &PrefabSpec::local("Fx-Props", "Fx-Lamp_Pf0", "Fx-Lamp_Pf0_Arc1", "Fx-Lamp00"),
            &StaticMeshPayload::unit_triangle(),
        );
        chunk.add_prefab_instanced_actor(&PrefabInstanceSpec::new(
            "Lamp_0",
            [0.0; 3],
            ("Fx-Props", "Fx-Lamp_Pf0", "Fx-Lamp_Pf0_Arc1"),
        ));
        // Export 3 is `Lamp_0`'s component (1 = PersistentLevel,
        // 2 = Lamp_0, 3 = its StaticMeshComponent).
        chunk.package_mut().set_unreadable(3);
    });
    let census = census_of(&s);

    assert_eq!(
        census.read_failures.values().sum::<u64>(),
        1,
        "the failure must be counted: {:?}",
        census.read_failures
    );
    assert!(
        census.read_failures.keys().any(|k| k.contains("component")),
        "{:?}",
        census.read_failures
    );
    assert_eq!(census.stub_total, 0, "an unread component is not a stub");
    assert_eq!(census.direct_total, 0);
    assert!(census.placements.is_empty(), "the actor must be skipped");
    assert!(census.stats.is_empty());

    let text = report_of(&census);
    assert!(text.contains("unreadable_exports\t1"), "{text}");
    assert!(text.contains("== unreadable exports"), "{text}");
    let _ = std::fs::remove_dir_all(&s.dir);
}

#[test]
fn a_non_colliding_prefab_is_reported_rather_than_measured() {
    let s = scene("census-nocollide", 0x000a_0032, |dir, chunk| {
        prefab_package(
            dir,
            &PrefabSpec::local("Fx-Props", "Fx-Snow_Pf0", "Fx-Snow_Pf0_Arc0", "Fx-Plane00")
                .with_collide_actors(false),
            &StaticMeshPayload::unit_triangle(),
        );
        chunk.add_prefab_instanced_actor(&PrefabInstanceSpec::new(
            "Snow_0",
            [0.0; 3],
            ("Fx-Props", "Fx-Snow_Pf0", "Fx-Snow_Pf0_Arc0"),
        ));
    });
    let census = census_of(&s);
    assert!(census.placements.is_empty());
    assert_eq!(census.collision_disabled.values().sum::<u64>(), 1);

    let text = report_of(&census);
    assert!(text.contains("bCollideActors = false"), "{text}");
    assert!(text.contains("TOTAL\t1"), "{text}");
    let _ = std::fs::remove_dir_all(&s.dir);
}

#[test]
fn a_traversal_keyword_mesh_is_listed_with_its_collision_state() {
    // An elevator with collision off is exactly the case that would
    // otherwise read as "there is no elevator in this map".
    let s = scene("census-traversal", 0x000a_0033, |dir, chunk| {
        prefab_package(
            dir,
            &PrefabSpec::local("Fx-Props", "Fx-Lift_Pf0", "Fx-Lift_Pf0_Arc0", "Fx-Stair00")
                .with_collide_actors(false),
            &StaticMeshPayload::unit_triangle(),
        );
        chunk.add_prefab_instanced_actor(&PrefabInstanceSpec::new(
            "Stair_0",
            [0.0, 0.0, 5000.0],
            ("Fx-Props", "Fx-Lift_Pf0", "Fx-Lift_Pf0_Arc0"),
        ));
    });
    let census = census_of(&s);
    assert_eq!(census.traversal.len(), 1);
    assert!(!census.traversal[0].collides);
    assert_eq!(census.traversal[0].mesh, "Fx-Props:Fx-Stair00");

    let text = report_of(&census);
    assert!(text.contains("traversal-keyword actors"), "{text}");
    assert!(text.contains("\tNO\t"), "collision state column:\n{text}");
    let _ = std::fs::remove_dir_all(&s.dir);
}

#[test]
fn an_unresolvable_actor_archetype_is_a_failure_row_not_a_measured_instance() {
    let s = scene("census-actor-unreadable", 0x000a_0034, |dir, chunk| {
        prefab_package(
            dir,
            &PrefabSpec::local("Fx-Props", "Fx-Lamp_Pf0", "Fx-Lamp_Pf0_Arc1", "Fx-Lamp00"),
            &StaticMeshPayload::unit_triangle(),
        );
        chunk.add_prefab_instanced_actor(
            &PrefabInstanceSpec::new(
                "Lamp_0",
                [0.0; 3],
                ("Fx-Props", "Fx-Lamp_Pf0", "Fx-Lamp_Pf0_Arc1"),
            )
            .with_actor_template(("Fx-Missing", "Fx-Ghost_Pf0", "Fx-Ghost_Pf0_Arc0")),
        );
    });
    let census = census_of(&s);
    assert!(census.placements.is_empty());
    assert_eq!(census.failures.values().sum::<u64>(), 1);
    assert!(
        census
            .failures
            .keys()
            .any(|k| k.contains("actor-archetype")),
        "{:?}",
        census.failures
    );
    let _ = std::fs::remove_dir_all(&s.dir);
}

#[test]
fn an_empty_map_reports_every_section_as_empty_rather_than_omitting_it() {
    let census = Census::default();
    let text = report_of(&census);
    for section in [
        "== unreadable exports",
        "== actors suppressed by bCollideActors = false ==",
        "== traversal-keyword actors",
        "== resolution failures ==",
        "== component-local transform properties ==",
        "== actor transform properties inherited from the archetype ==",
        "== per-mesh ==",
        "== per-chunk ==",
    ] {
        assert!(text.contains(section), "missing {section}:\n{text}");
    }
    // One `(none...)` marker per empty section that has one: read
    // failures, collision-disabled, traversal, resolution failures,
    // component transforms, inherited transforms.
    assert_eq!(text.matches("(none").count(), 6, "{text}");
    assert!(text.contains("triangles_added\t0"), "{text}");
}

#[test]
fn the_tsv_writers_emit_a_header_and_one_row_per_entity() {
    let s = scene("census-tsv", 0x000a_0035, |dir, chunk| {
        prefab_package(
            dir,
            &PrefabSpec::local("Fx-Props", "Fx-Lamp_Pf0", "Fx-Lamp_Pf0_Arc1", "Fx-Lamp00"),
            &StaticMeshPayload::unit_triangle(),
        );
        for i in 0..3 {
            chunk.add_prefab_instanced_actor(&PrefabInstanceSpec::new(
                &format!("Lamp_{i}"),
                [i as f32 * 100.0, 0.0, 0.0],
                ("Fx-Props", "Fx-Lamp_Pf0", "Fx-Lamp_Pf0_Arc1"),
            ));
        }
    });
    let census = census_of(&s);

    let mut meshes: Vec<u8> = Vec::new();
    report::write_meshes_tsv(&mut meshes, &census).unwrap();
    let meshes = String::from_utf8(meshes).unwrap();
    assert_eq!(meshes.lines().count(), 2, "header + one mesh:\n{meshes}");
    assert!(meshes.starts_with("mesh\tinstances\t"), "{meshes}");

    let mut positions: Vec<u8> = Vec::new();
    report::write_positions_tsv(&mut positions, &census).unwrap();
    let positions = String::from_utf8(positions).unwrap();
    assert_eq!(positions.lines().count(), 4, "header + three:\n{positions}");
    assert!(positions.lines().nth(1).unwrap().contains("Lamp_0"));
    let _ = std::fs::remove_dir_all(&s.dir);
}

// ---------- CLI ----------

#[test]
fn arguments_parse_into_the_fields_they_name() {
    let a = parse_args_from(&argv(&[
        "CookedPC",
        "Castle",
        "--positions",
        "p.tsv",
        "--meshes",
        "m.tsv",
    ]))
    .unwrap();
    assert_eq!(a.cooked, PathBuf::from("CookedPC"));
    assert_eq!(a.map, "Castle");
    assert_eq!(a.positions_out, Some(PathBuf::from("p.tsv")));
    assert_eq!(a.meshes_out, Some(PathBuf::from("m.tsv")));
}

#[test]
fn bad_arguments_are_rejected_rather_than_panicked_on() {
    // The previous shape used `.expect()` and `panic!()`, so a
    // mistyped flag produced a backtrace instead of a usage line.
    assert!(parse_args_from(&argv(&[])).is_err(), "no arguments");
    assert!(parse_args_from(&argv(&["CookedPC"])).is_err(), "no map");
    assert!(parse_args_from(&argv(&["CookedPC", "--positions", "p.tsv"])).is_err());
    assert!(parse_args_from(&argv(&["CookedPC", "Castle", "--nope"])).is_err());
    assert!(parse_args_from(&argv(&["CookedPC", "Castle", "--meshes"])).is_err());
    assert!(parse_args_from(&argv(&["CookedPC", "Castle", "--help"])).is_err());
}

#[test]
fn a_missing_map_directory_is_an_error_not_a_panic() {
    let s = scene("census-nomap", 0x000a_0036, |_dir, _chunk| {});
    let args = parse_args_from(&argv(&[s.dir.to_str().unwrap(), "NoSuchMap"])).unwrap();
    let mut out: Vec<u8> = Vec::new();
    let err = run(&mut out, &args, &s.index).expect_err("a missing map must fail");
    assert!(err.contains("NoSuchMap"), "{err}");
    let _ = std::fs::remove_dir_all(&s.dir);
}

#[test]
fn run_walks_a_map_directory_and_writes_both_tsvs() {
    let root = scratch_dir("census-run");
    let maps = root.join("Maps").join("Fix");
    std::fs::create_dir_all(&maps).unwrap();
    prefab_package(
        &root,
        &PrefabSpec::local("Fx-Props", "Fx-Lamp_Pf0", "Fx-Lamp_Pf0_Arc1", "Fx-Lamp00"),
        &StaticMeshPayload::unit_triangle(),
    );
    let mut chunk = ChunkFixture::new();
    chunk.add_prefab_instanced_actor(&PrefabInstanceSpec::new(
        "Lamp_0",
        [0.0; 3],
        ("Fx-Props", "Fx-Lamp_Pf0", "Fx-Lamp_Pf0_Arc1"),
    ));
    chunk.write(&maps, "Fix", 0x000a_0037);
    let index = index_over(&root);

    let args = parse_args_from(&argv(&[
        root.to_str().unwrap(),
        "Fix",
        "--meshes",
        root.join("m.tsv").to_str().unwrap(),
        "--positions",
        root.join("p.tsv").to_str().unwrap(),
    ]))
    .unwrap();

    let mut out: Vec<u8> = Vec::new();
    assert_eq!(run(&mut out, &args, &index).unwrap(), 0);
    let text = String::from_utf8(out).unwrap();
    assert!(text.contains("map\tFix"), "{text}");
    assert!(text.contains("chunks\t1"), "{text}");
    assert!(text.contains("resolved\t1"), "{text}");
    assert!(root.join("m.tsv").exists());
    assert!(root.join("p.tsv").exists());
    let _ = std::fs::remove_dir_all(&root);
}
