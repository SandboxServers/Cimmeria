//! Unit tests for the BSP owner classifier and actor placement.
//!
//! Split out of `mod.rs` at the `#[cfg(test)]` seam to keep that module
//! under the repo's 500-line soft cap. Tests that need a whole package
//! build one with [`crate::test_support`]; tests that only exercise the
//! classifier call it directly.

use cimmeria_upk::Package;

use super::*;
use crate::test_support::{scratch_dir, ChunkFixture, ModelPayload, Placement};

#[test]
fn level_owned_models_are_world_space() {
    assert_eq!(classify_owner(Some("Level")), OwnerKind::Level);
    assert!(OwnerKind::Level.emits_geometry());
}

#[test]
fn brush_and_blocking_volume_are_included() {
    assert_eq!(classify_owner(Some("Brush")), OwnerKind::IncludedActor);
    assert_eq!(
        classify_owner(Some("BlockingVolume")),
        OwnerKind::IncludedActor
    );
    assert!(OwnerKind::IncludedActor.emits_geometry());
}

#[test]
fn trigger_volumes_are_excluded() {
    // Both observed Castle volume classes. A TriggerVolume's convex
    // hull spans doorways — emitting it would seal the navmesh.
    for c in ["TriggerVolume", "DynamicTriggerVolume"] {
        assert_eq!(
            classify_owner(Some(c)),
            OwnerKind::ExcludedVolume,
            "{c} must be excluded"
        );
    }
    assert!(!OwnerKind::ExcludedVolume.emits_geometry());
}

#[test]
fn unknown_volume_classes_are_excluded_and_surfaced() {
    // A `*Volume` class we've never seen is excluded conservatively
    // rather than silently emitted, and lands in
    // `unclassified_volume_classes` so it shows up in the report.
    assert_eq!(
        classify_owner(Some("UTKillZVolume")),
        OwnerKind::UnknownVolume
    );
    assert!(!OwnerKind::UnknownVolume.emits_geometry());
}

#[test]
fn unknown_non_volume_owner_is_included() {
    // A Model-owning actor that isn't a volume is brush geometry.
    assert_eq!(
        classify_owner(Some("SGWDoorBrush")),
        OwnerKind::UnknownActor
    );
    assert!(OwnerKind::UnknownActor.emits_geometry());
}

#[test]
fn root_owned_model_is_the_builder_brush() {
    assert_eq!(classify_owner(None), OwnerKind::BuilderBrush);
    assert!(!OwnerKind::BuilderBrush.emits_geometry());
}

#[test]
fn level_model_placement_ignores_transform_and_prepivot() {
    let inst = BspModelInstance {
        export_index: 1,
        owner_class: "Level".into(),
        owner_name: "PersistentLevel".into(),
        is_level_model: true,
        transform: ActorTransform {
            location: [1000.0, 2000.0, 3000.0],
            ..Default::default()
        },
        pre_pivot: [5.0, 5.0, 5.0],
        model: Model::default(),
    };
    // Level CSG is already world space — a stray transform on the
    // Level export must not move it.
    assert_eq!(inst.to_world([10.0, 20.0, 30.0]), [10.0, 20.0, 30.0]);
}

#[test]
fn actor_model_placement_subtracts_prepivot_before_transform() {
    // UE3: FTranslationMatrix(-PrePivot) * Scale * Rotation *
    // Translation. With scale 2 and PrePivot (1,1,1), local (3,1,1)
    // lands at (2*2, 0, 0) + Location.
    let inst = BspModelInstance {
        export_index: 2,
        owner_class: "Brush".into(),
        owner_name: "Brush".into(),
        is_level_model: false,
        transform: ActorTransform {
            location: [100.0, 0.0, 0.0],
            draw_scale: 2.0,
            ..Default::default()
        },
        pre_pivot: [1.0, 1.0, 1.0],
        model: Model::default(),
    };
    assert_eq!(inst.to_world([3.0, 1.0, 1.0]), [104.0, 0.0, 0.0]);
}

#[test]
fn excluded_volume_class_table_carries_a_reason_for_each_entry() {
    // The exclusion list is the kind of thing a future reader will
    // want to challenge; every entry must say why.
    for (class, reason) in EXCLUDED_VOLUME_CLASSES {
        assert!(!class.is_empty());
        assert!(
            reason.len() > 20,
            "{class} needs a real reason, got {reason:?}"
        );
    }
}

// ----- owner placement, against synthetic packages -----

/// One horizontal quad, wound the way a real BSP node pool is.
fn quad() -> ModelPayload {
    ModelPayload::horizontal_quad(0.0, 100.0, 0.0, 100.0, 0.0, 0, [0.0, 0.0, 1.0])
}

fn open_chunk(chunk: &ChunkFixture, tag: &str, id: u32) -> Package {
    let dir = scratch_dir(tag);
    let path = chunk.write(&dir, "Bsp", id);
    Package::open(&path).expect("fixture package parses")
}

#[test]
fn a_brush_owned_model_is_placed_by_its_owner_transform_and_prepivot() {
    // The control for the next test: a well-formed owner must still
    // place its model exactly, PrePivot subtracted before scale.
    let mut chunk = ChunkFixture::new();
    chunk.add_owned_model(
        "Brush",
        "Brush_Good",
        Placement::at([1000.0, 2000.0, 3000.0])
            .with_draw_scale(2.0)
            .with_pre_pivot([50.0, 50.0, 0.0]),
        &quad(),
    );
    let pkg = open_chunk(&chunk, "bsp-placement-ok", 0x0000_0011);

    let (instances, stats) = collect_bsp_models(&pkg);
    assert_eq!(stats.models_failed, 0, "{:?}", stats.parse_errors);
    assert_eq!(stats.actor_models_included, 1);
    assert_eq!(instances.len(), 1);
    let inst = &instances[0];
    assert!(!inst.is_level_model);
    assert_eq!(inst.pre_pivot, [50.0, 50.0, 0.0]);
    assert_eq!(inst.transform.draw_scale, 2.0);
    // Local (0,0,0) - PrePivot(50,50,0) = (-50,-50,0), scaled by 2 is
    // (-100,-100,0), then translated by Location.
    assert_eq!(inst.to_world([0.0, 0.0, 0.0]), [900.0, 1900.0, 3000.0]);
}

#[test]
fn a_model_whose_owner_props_are_truncated_is_counted_and_skipped() {
    // Regression guard for the identity-default bug. The owner's
    // property stream carries a correct `Location` and then a tag whose
    // declared payload runs off the end of the body. A walker that
    // defaults the unread tags emits the model at the partial
    // transform: real geometry in the wrong place, which is the worst
    // outcome for a navmesh because nothing downstream can tell. The
    // model must be dropped, counted, and named in `parse_errors`.
    let mut chunk = ChunkFixture::new();
    chunk.add_owned_model_with_truncated_owner_props(
        "Brush",
        "Brush_Truncated",
        [7000.0, 8000.0, 9000.0],
        &quad(),
    );
    let pkg = open_chunk(&chunk, "bsp-placement-truncated", 0x0000_0012);

    let (instances, stats) = collect_bsp_models(&pkg);
    assert!(
        instances.is_empty(),
        "an unplaceable model must not reach the soup, got {instances:?}"
    );
    assert_eq!(stats.models_failed, 1);
    assert_eq!(stats.parse_errors.len(), 1);
    let err = &stats.parse_errors[0];
    assert!(
        err.contains("owner placement") && err.contains("Brush_Truncated"),
        "error must name the owner and the stage: {err}"
    );

    let mut soup = TriangleSoup::new(None);
    let stats = collect_bsp_triangles(&pkg, &mut soup, BspOptions::default());
    assert_eq!(soup.triangle_count(), 0);
    assert_eq!(stats.triangles_emitted, 0);
}

#[test]
fn a_level_model_survives_a_broken_sibling_brush() {
    // One bad actor must not cost the tile its world geometry -- the
    // whole reason the failure is counted rather than propagated.
    let mut chunk = ChunkFixture::new();
    chunk.add_level_model(&quad());
    chunk.add_owned_model_with_truncated_owner_props("Brush", "Brush_Truncated", [0.0; 3], &quad());
    let pkg = open_chunk(&chunk, "bsp-placement-mixed", 0x0000_0013);

    let mut soup = TriangleSoup::new(None);
    let stats = collect_bsp_triangles(&pkg, &mut soup, BspOptions::default());
    assert_eq!(stats.models_failed, 1);
    assert_eq!(stats.level_models, 1);
    assert_eq!(stats.triangles_emitted, 2, "the level quad still emits");
    assert_eq!(soup.triangle_count(), 2);
}

#[test]
fn an_unknown_volume_owner_is_excluded_and_named_in_the_report() {
    // The classifier unit test above proves the decision; this proves
    // the walker acts on it and surfaces the class name, which is the
    // only way a newly introduced volume class gets noticed.
    let mut chunk = ChunkFixture::new();
    chunk.add_owned_model(
        "UTKillZVolume",
        "KillZ_0",
        Placement::default().with_pre_pivot([0.0; 3]),
        &quad(),
    );
    let pkg = open_chunk(&chunk, "bsp-unknown-volume", 0x0000_0014);

    let mut soup = TriangleSoup::new(None);
    let stats = collect_bsp_triangles(&pkg, &mut soup, BspOptions::default());
    assert_eq!(stats.actor_models_excluded, 1);
    assert_eq!(
        stats.unclassified_volume_classes,
        vec![("UTKillZVolume".to_string(), 1)]
    );
    assert_eq!(soup.triangle_count(), 0);
}

#[test]
fn an_unknown_non_volume_owner_emits_and_is_reported() {
    let mut chunk = ChunkFixture::new();
    chunk.add_owned_model(
        "SGWDoorBrush",
        "Door_0",
        Placement::at([500.0, 0.0, 0.0]).with_pre_pivot([0.0; 3]),
        &quad(),
    );
    let pkg = open_chunk(&chunk, "bsp-unknown-actor", 0x0000_0015);

    let mut soup = TriangleSoup::new(None);
    let stats = collect_bsp_triangles(&pkg, &mut soup, BspOptions::default());
    assert_eq!(stats.actor_models_included, 1);
    assert_eq!(
        stats.unclassified_owner_classes,
        vec![("SGWDoorBrush".to_string(), 1)]
    );
    assert!(stats.unclassified_volume_classes.is_empty());
    assert_eq!(soup.triangle_count(), 2);
    assert!(soup.vertices.contains(&[500.0, 0.0, 0.0]));
}

#[test]
fn the_builder_brush_stub_is_skipped_without_being_a_failure() {
    let mut chunk = ChunkFixture::new();
    chunk.add_builder_brush_model(&ModelPayload::empty());
    let pkg = open_chunk(&chunk, "bsp-builder-brush", 0x0000_0016);

    let (instances, stats) = collect_bsp_models(&pkg);
    assert_eq!(stats.models_total, 1);
    assert_eq!(stats.builder_brush_models, 1);
    assert_eq!(stats.models_failed, 0);
    assert_eq!(stats.models_parsed, 0, "a skipped owner is never decoded");
    assert!(instances.is_empty());
}

#[test]
fn a_model_whose_payload_is_truncated_is_counted_as_failed() {
    // The other half of `models_failed`: the owner is fine, the
    // `UModel` body is not. The deserializer enforces exact
    // consumption, so a short payload must be reported rather than
    // decoded into a partial node list.
    let mut chunk = ChunkFixture::new();
    let level = chunk.level();
    let builder = chunk.package_mut();
    let model_class = builder.class_ref("Model");
    let export = builder.add_export(model_class, level, "Model_Broken");
    builder.set_payload(export, vec![0u8; 40]);
    let pkg = open_chunk(&chunk, "bsp-bad-payload", 0x0000_0017);

    let (instances, stats) = collect_bsp_models(&pkg);
    assert!(instances.is_empty());
    assert_eq!(stats.models_failed, 1);
    assert_eq!(stats.models_parsed, 0);
    assert!(stats.parse_errors[0].contains("Model_Broken"));
}

#[test]
fn a_not_solid_surface_is_filtered_out_of_the_soup() {
    // `PF_NotSolid` is a PolyFlags bit the default filter drops, and
    // dropping it is what keeps non-blocking CSG out of the mesh.
    let mut chunk = ChunkFixture::new();
    let mut model = ModelPayload::default();
    model.push_quad(
        [
            [0.0, 0.0, 0.0],
            [100.0, 0.0, 0.0],
            [100.0, 100.0, 0.0],
            [0.0, 100.0, 0.0],
        ],
        cimmeria_upk_objects::model::PF_NOT_SOLID,
        [0.0, 0.0, 1.0],
    );
    model.push_quad(
        [
            [0.0, 0.0, 200.0],
            [100.0, 0.0, 200.0],
            [100.0, 100.0, 200.0],
            [0.0, 100.0, 200.0],
        ],
        0,
        [0.0, 0.0, 1.0],
    );
    chunk.add_level_model(&model);
    let pkg = open_chunk(&chunk, "bsp-notsolid", 0x0000_0018);

    let mut soup = TriangleSoup::new(None);
    let stats = collect_bsp_triangles(&pkg, &mut soup, BspOptions::default());
    assert_eq!(stats.triangles_excluded, 2, "the PF_NotSolid quad");
    assert_eq!(stats.triangles_emitted, 2, "the unflagged quad");
    assert!(soup.vertices.iter().all(|v| v[2] == 200.0));
}

#[test]
fn an_invisible_but_solid_surface_still_reaches_the_soup() {
    // `PF_Invisible` controls visibility, not collision. Treating it as
    // non-colliding deletes invisible solid BSP and punches a hole in
    // the navmesh exactly where a player would expect a wall.
    let mut chunk = ChunkFixture::new();
    let mut model = ModelPayload::default();
    model.push_quad(
        [
            [0.0, 0.0, 0.0],
            [100.0, 0.0, 0.0],
            [100.0, 100.0, 0.0],
            [0.0, 100.0, 0.0],
        ],
        cimmeria_upk_objects::model::PF_INVISIBLE,
        [0.0, 0.0, 1.0],
    );
    chunk.add_level_model(&model);
    let pkg = open_chunk(&chunk, "bsp-invisible", 0x0000_001a);

    let mut soup = TriangleSoup::new(None);
    let stats = collect_bsp_triangles(&pkg, &mut soup, BspOptions::default());
    assert_eq!(stats.triangles_excluded, 0);
    assert_eq!(stats.triangles_emitted, 2);
    assert_eq!(soup.triangle_count(), 2);
}

#[test]
fn emitted_bsp_fans_are_reversed_relative_to_the_stored_vertex_pool() {
    // `EMIT_REVERSED` is the difference between a walkable floor and a
    // ceiling. The fixture quad is wound so the stored order's UE3
    // right-hand-rule normal points UP (+Z); NavBuilder only treats a
    // triangle as ground when the emitted order's normal points DOWN.
    let mut chunk = ChunkFixture::new();
    chunk.add_level_model(&quad());
    let pkg = open_chunk(&chunk, "bsp-winding", 0x0000_0019);

    let mut soup = TriangleSoup::new(None);
    collect_bsp_triangles(&pkg, &mut soup, BspOptions::default());
    assert_eq!(soup.triangle_count(), 2);
    for f in &soup.faces {
        let t = [
            soup.vertices[f[0] as usize - 1],
            soup.vertices[f[1] as usize - 1],
            soup.vertices[f[2] as usize - 1],
        ];
        let e1 = [t[1][0] - t[0][0], t[1][1] - t[0][1], t[1][2] - t[0][2]];
        let e2 = [t[2][0] - t[0][0], t[2][1] - t[0][1], t[2][2] - t[0][2]];
        let nz = e1[0] * e2[1] - e1[1] * e2[0];
        assert!(
            nz < 0.0,
            "emitted normal z = {nz} must point down for NavBuilder to walk it"
        );
    }
}
