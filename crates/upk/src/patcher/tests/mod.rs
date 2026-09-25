//! Patcher tests run against small synthetic Epic-486 packages written to the
//! temp dir, so they need no client files.

mod fixtures;

use byteorder::{ByteOrder, LittleEndian};

use super::{clone_objects, CloneRequest, PatchSession, Placement};
use crate::Package;
use fixtures::{
    clone_actors, level_package, rig_package, source_package, target_package, write_temp, Builder,
};

#[test]
fn roundtrip_preserves_every_export_and_clears_compression() {
    let src = write_temp("rt-in", &source_package());
    let bytes = PatchSession::open(&src).unwrap().finish().unwrap();
    let out = write_temp("rt-out", &bytes);

    let (before, after) = (Package::open(&src).unwrap(), Package::open(&out).unwrap());
    assert_eq!(after.header.package_flags, 0x000A_0009);
    assert!(!after.header.is_compressed());
    assert_eq!(after.names.len(), before.names.len());
    assert_eq!(after.imports.len(), before.imports.len());
    for (b, a) in before.exports.iter().zip(&after.exports) {
        // Nothing moves: same offset, same bytes.
        assert_eq!(a.serial_offset, b.serial_offset);
        assert_eq!(
            after.read_export_data(a).unwrap(),
            before.read_export_data(b).unwrap()
        );
    }
    // Tables are contiguous at the end, and total_header_size closes the file.
    assert!(after.header.name_offset as usize >= source_package().len());
    assert_eq!(after.header.total_header_size as usize, bytes.len());
    let _ = (std::fs::remove_file(src), std::fs::remove_file(out));
}

#[test]
fn clone_remaps_refs_names_component_map_and_level_list() {
    let src_path = write_temp("cl-src", &source_package());
    let dst_path = write_temp("cl-dst", &target_package());
    let source = PatchSession::open(&src_path).unwrap();
    let mut target = PatchSession::open(&dst_path).unwrap();

    // Source actor is export index 2 (decoy, level, actor, decoy, component).
    let report = clone_actors(
        &mut target,
        &source,
        &[2],
        Placement::FirstActorAt([1000.0, 2000.0, 3000.0]),
    )
    .unwrap();
    assert_eq!(report.level_actor_count, Some((1, 2)));
    assert_eq!(report.exports_added, 2);
    // Package, mesh, actor class, component class. "staticmeshactor" already
    // exists as a name (different case), so it must not be added again.
    assert_eq!(report.imports_added, 4);
    assert_eq!(report.objects[0].location, Some([1000.0, 2000.0, 3000.0]));

    let out = write_temp("cl-out", &target.finish().unwrap());
    let pkg = Package::open(&out).unwrap();
    assert_eq!(
        pkg.names
            .iter()
            .filter(|n| n.name.eq_ignore_ascii_case("StaticMeshActor"))
            .count(),
        1
    );

    // Target had 2 exports, so the clones are refs 3 (actor) and 4 (component).
    let actor = &pkg.exports[2];
    let comp = &pkg.exports[3];
    assert_eq!(
        pkg.export_full_path(comp),
        "PersistentLevel.staticmeshactor.StaticMeshComponent"
    );
    // Guard: ComponentMap holds a 0-based export INDEX. Remapping it as a
    // 1-based ref (the first implementation) points it at the wrong export.
    assert_eq!(
        actor.component_map,
        vec![("StaticMeshComponent0".to_string(), 3)]
    );

    let data = pkg.read_export_data(actor).unwrap();
    assert_eq!(LittleEndian::read_i32(&data[0..]), actor.class_index);
    assert_eq!(LittleEndian::read_i32(&data[4..]), actor.class_index);
    assert_eq!(
        LittleEndian::read_i32(&data[28..]),
        -1,
        "NetIndex must be INDEX_NONE"
    );
    let props = crate::parse_tagged_properties(&data, 32, &pkg.names);
    assert!(matches!(props[0].value, crate::PropValue::Object(4)));
    assert!(matches!(
        props[1].value,
        crate::PropValue::Vector { x, y, z } if (x, y, z) == (1000.0, 2000.0, 3000.0)
    ));

    let comp_data = pkg.read_export_data(comp).unwrap();
    let comp_props = crate::parse_tagged_properties(&comp_data, 8, &pkg.names);
    let crate::PropValue::Object(mesh) = comp_props[0].value else {
        panic!("StaticMesh is not an object property");
    };
    assert_eq!(
        pkg.resolve_object_path(mesh),
        "GLB-Global.GLB-RingTransporterBase_TC00"
    );

    let level = pkg.read_export_data(&pkg.exports[0]).unwrap();
    assert_eq!(LittleEndian::read_i32(&level[16..]), 2, "actor count");
    assert_eq!(
        LittleEndian::read_i32(&level[20..]),
        2,
        "existing actor kept"
    );
    assert_eq!(
        LittleEndian::read_i32(&level[24..]),
        3,
        "new actor appended"
    );
    assert_eq!(&level[28..], b"TAIL", "data after the array is preserved");
    let _ = [src_path, dst_path, out].map(std::fs::remove_file);
}

#[test]
fn ensure_import_reuses_an_existing_import() {
    let src_path = write_temp("imp-src", &source_package());
    let source = PatchSession::open(&src_path).unwrap();
    let mut target = PatchSession::open(&src_path).unwrap();
    // Same package on both sides: every import already exists.
    assert_eq!(target.ensure_import_from(&source, -2).unwrap(), -2);
    assert_eq!(target.additions(), (0, 0, 0));
    let _ = std::fs::remove_file(src_path);
}

#[test]
fn clone_rejects_an_array_it_cannot_type() {
    // A bare i32 array under a name that is not a known object array: it could
    // be ints or refs, and guessing wrong corrupts the package.
    let mut b = Builder::default();
    let actor_class = b.import("Core", "Class", 0, "StaticMeshActor");
    let level = level_package(&mut b, &[]);
    let mut actor = Vec::new();
    Builder::i32s(&mut actor, &[actor_class, actor_class, -1, -1, 0, 0, -1, 1]);
    b.vector_prop(&mut actor, "Location", [0.0, 0.0, 1.0]);
    b.object_array_prop(&mut actor, "Touching", &[2]);
    b.none(&mut actor);
    b.export(actor_class, level, "StaticMeshActor", actor);
    b.mark_actor();
    let src_path = write_temp("arr-src", &b.build());
    let dst_path = write_temp("arr-dst", &target_package());

    let source = PatchSession::open(&src_path).unwrap();
    let mut target = PatchSession::open(&dst_path).unwrap();
    let e = clone_actors(&mut target, &source, &[1], Placement::Offset([0.0; 3])).unwrap_err();
    assert!(e.to_string().contains("Touching"), "{e}");
    assert!(e.to_string().contains("not a known object array"), "{e}");
    let _ = [src_path, dst_path].map(std::fs::remove_file);
}

#[test]
fn rig_clone_in_the_same_package_remaps_nested_refs_and_attaches_the_sequence() {
    let path = write_temp("rig", &rig_package());
    let source = PatchSession::open(&path).unwrap();
    let mut target = PatchSession::open(&path).unwrap();

    // Clone the rig sequence (index 2) and its ring (5); Prefabs (1) already
    // exists in the target, and the clone is carried from base 6 to base 7.
    let report = clone_objects(
        &mut target,
        &source,
        &CloneRequest {
            roots: &[2, 5],
            mapped: &[(1, 1)],
            placement: Placement::Anchor {
                source: 6,
                target: 7,
            },
        },
    )
    .unwrap();
    // No names or imports are new in a same-package clone.
    assert_eq!(
        (
            report.names_added,
            report.imports_added,
            report.exports_added
        ),
        (0, 0, 4)
    );
    // Original was 8 exports: rig=9, var=10, event=11, ring=12.
    assert_eq!(report.sequences_attached, vec![(1, 9)]);
    // Guard: "Rig_Seq" already exists under Prefabs, so the clone must take a new
    // instance number or UE3 treats both exports as one object.
    assert_eq!(report.objects[0].name, "Rig_Seq_0");
    // Ring sat 50 above the source base; the yaw delta is +90 degrees.
    let ring = report
        .objects
        .iter()
        .find(|o| o.class == "InterpActor")
        .unwrap();
    let loc = ring.location.unwrap();
    assert!(
        loc[0].abs() < 0.01 && (loc[1] - 500.0).abs() < 0.01,
        "{loc:?}"
    );
    assert_eq!(loc[2], 50.0);

    let out = write_temp("rig-out", &target.finish().unwrap());
    let pkg = Package::open(&out).unwrap();
    let prop = |index: usize, start: usize, name: &str| {
        let data = pkg.read_export_data(&pkg.exports[index]).unwrap();
        crate::parse_tagged_properties(&data, start, &pkg.names)
            .into_iter()
            .find(|p| p.name == name)
            .unwrap()
            .value
    };
    let i32s = |v: crate::PropValue| match v {
        crate::PropValue::Array(bytes) => bytes
            .chunks(4)
            .map(LittleEndian::read_i32)
            .collect::<Vec<_>>(),
        other => panic!("not an array: {other:?}"),
    };

    // Parent sequence gained the clone; its original entry is untouched.
    assert_eq!(i32s(prop(1, 4, "SequenceObjects")), vec![2, 3, 9]);
    // The cloned rig lists its own children, not the originals.
    assert_eq!(i32s(prop(8, 4, "SequenceObjects")), vec![2, 10, 11]);
    assert!(matches!(
        prop(8, 4, "ParentSequence"),
        crate::PropValue::Object(2)
    ));
    // The var drives the cloned ring, not the original.
    assert!(matches!(
        prop(9, 4, "ObjValue"),
        crate::PropValue::Object(12)
    ));
    assert_eq!(i32s(prop(10, 4, "LinkedVariables")), vec![1, 10]);
    // The ref two arrays deep: diff the cloned event against the original word
    // by word. NetIndex, then LinkedOp, LinkedVariables[0], ParentSequence.
    let event = pkg.read_export_data(&pkg.exports[10]).unwrap();
    let original = pkg.read_export_data(&pkg.exports[4]).unwrap();
    assert_eq!(event.len(), original.len());
    let differing: Vec<(i32, i32)> = (0..event.len() / 4)
        .map(|w| {
            (
                LittleEndian::read_i32(&original[w * 4..]),
                LittleEndian::read_i32(&event[w * 4..]),
            )
        })
        .filter(|(a, b)| a != b)
        .collect();
    assert_eq!(differing, vec![(0, -1), (4, 10), (4, 10), (3, 9)]);
    // Rotation carried the yaw delta.
    assert!(matches!(
        prop(11, 32, "Rotation"),
        crate::PropValue::Rotator { yaw: 16384, .. }
    ));
    let _ = [path, out].map(std::fs::remove_file);
}

#[test]
fn mapped_source_objects_are_redirected_not_cloned() {
    let path = write_temp("map", &rig_package());
    let source = PatchSession::open(&path).unwrap();
    let mut target = PatchSession::open(&path).unwrap();
    // Reuse an existing actor (7) for the ring instead of cloning ring 5.
    let report = clone_objects(
        &mut target,
        &source,
        &CloneRequest {
            roots: &[2],
            mapped: &[(1, 1), (5, 7)],
            placement: Placement::Offset([0.0; 3]),
        },
    )
    .unwrap();
    assert_eq!(report.exports_added, 3);
    assert_eq!(report.level_actor_count, None, "no actor was cloned");
    let out = write_temp("map-out", &target.finish().unwrap());
    let pkg = Package::open(&out).unwrap();
    let data = pkg.read_export_data(&pkg.exports[9]).unwrap();
    let props = crate::parse_tagged_properties(&data, 4, &pkg.names);
    assert!(
        matches!(props[0].value, crate::PropValue::Object(8)),
        "{props:?}"
    );
    let _ = [path, out].map(std::fs::remove_file);
}

#[test]
fn level_splice_refuses_inline_bulk_data_offsets() {
    // A level whose tail holds its own absolute file offset, as an inline
    // FUntypedBulkData header would. Relocating it verbatim would corrupt it.
    let mut b = Builder::default();
    let level_class = b.import("Core", "Class", 0, "Level");
    let trigger = b.import("Core", "Class", 0, "Trigger");
    let mut data = Vec::new();
    Builder::i32s(&mut data, &[7]);
    b.none(&mut data);
    Builder::i32s(&mut data, &[1, 0, 0]); // owner, count, placeholder
    b.export(level_class, 0, "PersistentLevel", data);
    b.export(trigger, 1, "Trigger", vec![0; 8]);
    let mut bytes = b.build();
    let pkg_path = write_temp("bulk-probe", &bytes);
    let offset = Package::open(&pkg_path).unwrap().exports[0].serial_offset as usize;
    // Placeholder is the last 4 bytes of the level data (at +20); a bulk header
    // stores the offset of the byte after itself.
    LittleEndian::write_i32(&mut bytes[offset + 20..], (offset + 24) as i32);
    std::fs::write(&pkg_path, &bytes).unwrap();

    let src_path = write_temp("bulk-src", &source_package());
    let source = PatchSession::open(&src_path).unwrap();
    let mut target = PatchSession::open(&pkg_path).unwrap();
    let e = clone_actors(&mut target, &source, &[2], Placement::Offset([0.0; 3])).unwrap_err();
    assert!(e.to_string().contains("self-referential"), "{e}");
    let _ = [src_path, pkg_path].map(std::fs::remove_file);
}
