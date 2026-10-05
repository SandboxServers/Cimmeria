//! Patcher tests run against small synthetic Epic-486 packages written to the
//! temp dir, so they need no client files.

pub(crate) mod fixtures;

use byteorder::{ByteOrder, LittleEndian};

use super::{clone_objects, CloneRequest, PatchSession, Placement};
use crate::Package;
use fixtures::{
    clone_actors, level_package, rig_package, source_package, source_package_with_lod_data,
    target_package, write_temp, Builder,
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

/// Clone the fixture actor whose component ends in `lod_data`.
fn clone_with_lod_data(tag: &str, lod_data: &[i32]) -> crate::error::Result<Vec<u8>> {
    let src_path = write_temp(
        &format!("{tag}-src"),
        &source_package_with_lod_data(lod_data),
    );
    let dst_path = write_temp(&format!("{tag}-dst"), &target_package());
    let source = PatchSession::open(&src_path).unwrap();
    let mut target = PatchSession::open(&dst_path).unwrap();
    let result = clone_actors(&mut target, &source, &[2], Placement::Offset([0.0; 3]))
        .and_then(|_| target.finish());
    let _ = [src_path, dst_path].map(std::fs::remove_file);
    result
}

#[test]
fn an_unlit_mesh_component_lod_entry_is_copied_verbatim() {
    // One LODInfo with no shadow maps and LMT_None: region 3's ring base
    // (Castle_CellBlock-fffeffff export 1616), which the cloner used to refuse.
    let bytes = clone_with_lod_data("lod1", &[1, 0, 0, 0]).unwrap();
    let out = write_temp("lod1-out", &bytes);
    let pkg = Package::open(&out).unwrap();
    let comp = pkg.read_export_data(&pkg.exports[3]).unwrap();
    let tail: Vec<i32> = comp[comp.len() - 16..]
        .chunks(4)
        .map(LittleEndian::read_i32)
        .collect();
    assert_eq!(tail, vec![1, 0, 0, 0]);
    let _ = std::fs::remove_file(out);
}

#[test]
fn a_lod_entry_with_baked_lighting_is_refused() {
    // A non-zero light-map type means texture refs follow; copying them
    // verbatim would point at unrelated objects in the target.
    for lod_data in [&[1, 0, 0, 1][..], &[2, 0, 0, 0][..], &[1, 0, 0, 0, 0][..]] {
        let e = clone_with_lod_data("lodx", lod_data).unwrap_err();
        assert!(
            e.to_string().contains("post-property data"),
            "{lod_data:?}: {e}"
        );
    }
}

#[test]
fn repeated_rig_clones_into_one_session_get_their_own_sequence_names() {
    // DA-08 clones one rig eight times into one chunk; each copy's Kismet must
    // be addressable by its own object path, so each root sequence needs its
    // own instance number and its own SequenceObjects entry.
    let path = write_temp("rig2", &rig_package());
    let source = PatchSession::open(&path).unwrap();
    let mut target = PatchSession::open(&path).unwrap();
    let mut names = Vec::new();
    let mut actor_counts = Vec::new();
    for at in [[0.0, 1000.0, 0.0], [0.0, 2000.0, 0.0]] {
        let report = clone_objects(
            &mut target,
            &source,
            &CloneRequest {
                roots: &[2, 6, 5],
                mapped: &[(1, 1)],
                placement: Placement::FirstActorAt(at),
            },
        )
        .unwrap();
        names.push(report.objects[0].name.clone());
        actor_counts.push(report.level_actor_count.unwrap());
        // The base (first actor root) lands on the point; the ring keeps its
        // 50-unit lift above it.
        let ring = report.objects.iter().find(|o| o.class == "InterpActor");
        assert_eq!(ring.unwrap().location, Some([at[0], at[1], 50.0]));
    }
    assert_eq!(names, vec!["Rig_Seq_0", "Rig_Seq_1"]);
    assert_eq!(actor_counts, vec![(3, 5), (5, 7)]);

    let out = write_temp("rig2-out", &target.finish().unwrap());
    let pkg = Package::open(&out).unwrap();
    let data = pkg.read_export_data(&pkg.exports[1]).unwrap();
    let seq_objects = crate::parse_tagged_properties(&data, 4, &pkg.names)
        .into_iter()
        .find(|p| p.name == "SequenceObjects")
        .unwrap();
    let crate::PropValue::Array(bytes) = seq_objects.value else {
        panic!("SequenceObjects is not an array");
    };
    let refs: Vec<i32> = bytes.chunks(4).map(LittleEndian::read_i32).collect();
    // Count 3: the original rig (3) and both clones (9, then 9 + 5 exports).
    assert_eq!(refs, vec![3, 3, 9, 14]);
    let _ = [path, out].map(std::fs::remove_file);
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

/// Name entries carry load-for-client/server/edit bits, and the client reads
/// an entry that does not load on the client as `NAME_None`. A property tag
/// named `None` ends the list, so the rest of the object is read from the
/// wrong offset: client patch 010 hung every client that loaded its map
/// because Ihpet's `Dynamic` is editor-only there while Castle's, where the
/// rig came from, is not, and the cloned `LightingChannels` struct names it.
#[test]
fn a_cloned_property_name_keeps_the_load_bits_it_had_in_the_source() {
    const LOAD_BITS: u64 = 0x0007_0000_0000_0000;
    const EDITOR_ONLY: u64 = 0x0004_0010_0000_0000;

    let mut sb = Builder::default();
    let actor_class = sb.import("Core", "Class", 0, "StaticMeshActor");
    let level = level_package(&mut sb, &[]);
    let mut actor = Vec::new();
    Builder::i32s(&mut actor, &[actor_class, actor_class, -1, -1, 0, 0, -1, 9]);
    sb.bool_prop(&mut actor, "Dynamic", true);
    sb.vector_prop(&mut actor, "Location", [1.0, 2.0, 3.0]);
    sb.none(&mut actor);
    sb.export(actor_class, level, "StaticMeshActor", actor);
    sb.mark_actor();
    sb.mark_client_loaded();
    let src_path = write_temp("flags-src", &sb.build());

    let mut tb = Builder::default();
    tb.name_flags.insert("Dynamic".into(), EDITOR_ONLY);
    tb.name("Dynamic");
    level_package(&mut tb, &[]);
    let dst_path = write_temp("flags-dst", &tb.build());

    let source = PatchSession::open(&src_path).unwrap();
    let mut target = PatchSession::open(&dst_path).unwrap();
    let target_before = target.export_count();
    clone_actors(&mut target, &source, &[1], Placement::Offset([0.0; 3])).unwrap();
    let out = write_temp("flags-out", &target.finish().unwrap());
    let pkg = Package::open(&out).unwrap();

    // The tool's own check agrees: nothing the clone names reads as None.
    let new_objects = target_before..pkg.exports.len();
    let audit = super::name_audit::audit_client_names(&pkg, new_objects).unwrap();
    assert_eq!(audit.unloadable, vec![]);
    assert_eq!(audit.audited, 1, "the audit must actually see the clone");

    let cloned = pkg.exports.last().unwrap();
    let data = pkg.read_export_data(cloned).unwrap();
    // First tag of the cloned actor, after the 32-byte state frame prefix.
    let tag_name = LittleEndian::read_i32(&data[32..]) as usize;
    assert_eq!(pkg.names[tag_name].name, "Dynamic");
    assert_eq!(
        pkg.names[tag_name].flags & LOAD_BITS,
        LOAD_BITS,
        "the clone must name an entry the client loads as a real name"
    );
    // The stock entry is left as it was, and the table now holds both.
    let dynamics: Vec<_> = pkg
        .names
        .iter()
        .enumerate()
        .filter(|(_, n)| n.name == "Dynamic")
        .collect();
    assert_eq!(dynamics.len(), 2);
    assert_eq!(dynamics[0].1.flags, EDITOR_ONLY);
    assert_ne!(dynamics[0].0, tag_name);
    let _ = [src_path, dst_path, out].map(std::fs::remove_file);
}

/// A name the target already holds with the same load bits is reused, not
/// duplicated: the fix must not bloat tables for the usual case.
#[test]
fn a_name_with_matching_load_bits_is_reused_not_duplicated() {
    let src_path = write_temp("reuse-src", &source_package());
    let dst_path = write_temp("reuse-dst", &target_package());
    let source = PatchSession::open(&src_path).unwrap();
    let mut target = PatchSession::open(&dst_path).unwrap();
    let before = target.package.names.len();
    clone_actors(&mut target, &source, &[2], Placement::Offset([0.0; 3])).unwrap();
    let out = write_temp("reuse-out", &target.finish().unwrap());
    let pkg = Package::open(&out).unwrap();
    let mut seen = std::collections::HashSet::new();
    for n in &pkg.names[before..] {
        assert!(
            seen.insert(n.name.to_lowercase()),
            "{} was added twice",
            n.name
        );
    }
    for n in &pkg.names[before..] {
        assert!(
            !pkg.names[..before]
                .iter()
                .any(|o| o.name.eq_ignore_ascii_case(&n.name)),
            "{} duplicates an entry the target already had",
            n.name
        );
    }
    let _ = [src_path, dst_path, out].map(std::fs::remove_file);
}

/// The audit itself must see the 010 shape: a tag named by an entry the
/// client does not load, whether at the top of an object or nested in a
/// struct property, and a clean object must pass. Without this the check in
/// `upk_patch` could rot into a function that always returns nothing.
#[test]
fn the_name_audit_finds_a_tag_the_client_would_read_as_none() {
    const EDITOR_ONLY: u64 = 0x0004_0010_0000_0000;
    let mut b = Builder::default();
    b.name_flags.insert("Dynamic".into(), EDITOR_ONLY);
    let class = b.import("Core", "Class", 0, "StaticMeshComponent");
    // A component (8-byte prefix): `LightingChannels { bInitialized, Dynamic }`,
    // the struct the 010 clones carried.
    let mut inner = Vec::new();
    b.bool_prop(&mut inner, "bInitialized", true);
    b.bool_prop(&mut inner, "Dynamic", true);
    b.none(&mut inner);
    let mut comp = vec![0u8; 8];
    b.tag(
        &mut comp,
        "LightingChannels",
        "StructProperty",
        inner.len() as i32,
    );
    let s = b.name("LightingChannelContainer");
    Builder::i32s(&mut comp, &[s, 0]);
    comp.extend_from_slice(&inner);
    b.none(&mut comp);
    b.export(class, 0, "StaticMeshComponent", comp);
    b.mark_client_loaded();
    // A clean sibling.
    let mut clean = vec![0u8; 8];
    b.bool_prop(&mut clean, "bInitialized", true);
    b.none(&mut clean);
    b.export(class, 0, "StaticMeshComponent", clean);
    b.mark_client_loaded();
    // An editor-only object (no RF_LoadForClient) naming the same name: stock
    // packages have these (a Brush, a DrawLightConeComponent) and the client
    // never serializes them, so the audit must not report them.
    let mut editor_object = vec![0u8; 8];
    b.bool_prop(&mut editor_object, "Dynamic", true);
    b.none(&mut editor_object);
    b.export(class, 0, "StaticMeshComponent", editor_object);
    // A client-loaded object whose list the audit cannot follow.
    b.export(class, 0, "StaticMeshComponent", vec![0xFF; 16]);
    b.mark_client_loaded();
    let path = write_temp("audit", &b.build());
    let pkg = Package::open(&path).unwrap();

    let audit = super::name_audit::audit_client_names(&pkg, 0..4).unwrap();
    let bad = &audit.unloadable;
    assert_eq!(bad.len(), 1, "{bad:?}");
    assert_eq!((bad[0].export, bad[0].name.as_str()), (0, "Dynamic"));
    assert_eq!(pkg.names[bad[0].name_index].flags, EDITOR_ONLY);
    assert_eq!(audit.audited, 2, "{audit:?}");
    assert_eq!(
        audit.not_audited,
        vec![3],
        "unwalkable objects are reported"
    );
    let _ = std::fs::remove_file(path);
}

/// The remapper clones struct-array elements, so the audit has to look inside
/// them too: an unloadable name there misreads the rest of the object just
/// the same.
#[test]
fn the_name_audit_descends_struct_arrays() {
    const EDITOR_ONLY: u64 = 0x0004_0010_0000_0000;
    let mut b = Builder::default();
    b.name_flags.insert("Dynamic".into(), EDITOR_ONLY);
    let class = b.import("Core", "Class", 0, "StaticMeshComponent");
    let mut element = Vec::new();
    b.bool_prop(&mut element, "Dynamic", true);
    b.none(&mut element);
    let mut comp = vec![0u8; 8];
    b.struct_array_prop(&mut comp, "Elements", &[element]);
    b.none(&mut comp);
    b.export(class, 0, "StaticMeshComponent", comp);
    b.mark_client_loaded();
    // A bare array of ints is not a struct array and must not be misread as one.
    let mut ints = vec![0u8; 8];
    b.tag(&mut ints, "Values", "ArrayProperty", 12);
    Builder::i32s(&mut ints, &[2, 67, 68]);
    b.none(&mut ints);
    b.export(class, 0, "StaticMeshComponent", ints);
    b.mark_client_loaded();
    let path = write_temp("audit-array", &b.build());
    let pkg = Package::open(&path).unwrap();

    let audit = super::name_audit::audit_client_names(&pkg, 0..2).unwrap();
    assert_eq!(audit.unloadable.len(), 1, "{audit:?}");
    assert_eq!(audit.unloadable[0].export, 0);
    assert!(audit.not_audited.is_empty(), "{audit:?}");
    let _ = std::fs::remove_file(path);
}
