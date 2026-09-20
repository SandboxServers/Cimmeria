//! Patcher tests run against small synthetic Epic-486 packages written to the
//! temp dir, so they need no client files.

use std::path::PathBuf;

use byteorder::{ByteOrder, LittleEndian};

use super::raw_tables::{self, RawExport, RawImport};
use super::{clone_actors, PatchSession, Placement};
use crate::Package;

const NAME_FLAGS: u64 = 0x0007_0010_0000_0000;

#[derive(Default)]
struct Builder {
    names: Vec<String>,
    imports: Vec<RawImport>,
    exports: Vec<(RawExport, Vec<u8>)>,
}

impl Builder {
    fn name(&mut self, s: &str) -> i32 {
        if let Some(i) = self.names.iter().position(|n| n == s) {
            return i as i32;
        }
        self.names.push(s.to_string());
        self.names.len() as i32 - 1
    }

    /// Returns the import's (negative) ref.
    fn import(&mut self, class_pkg: &str, class: &str, outer: i32, name: &str) -> i32 {
        let entry = RawImport {
            class_package: (self.name(class_pkg), 0),
            class_name: (self.name(class), 0),
            outer,
            object_name: (self.name(name), 0),
        };
        self.imports.push(entry);
        -(self.imports.len() as i32)
    }

    /// Returns the export's (positive, 1-based) ref.
    fn export(&mut self, class: i32, outer: i32, name: &str, data: Vec<u8>) -> i32 {
        let entry = RawExport {
            class_index: class,
            super_index: 0,
            outer,
            object_name: (self.name(name), 0),
            archetype: 0,
            object_flags: 0,
            serial_size: 0,
            serial_offset: 0,
            component_map: Vec::new(),
            export_flags: 0,
            gen_net_obj_count: Vec::new(),
            package_guid: [0; 4],
        };
        self.exports.push((entry, data));
        self.exports.len() as i32
    }

    fn i32s(out: &mut Vec<u8>, vals: &[i32]) {
        for v in vals {
            out.extend_from_slice(&v.to_le_bytes());
        }
    }

    fn build(&self) -> Vec<u8> {
        let mut names = Vec::new();
        for n in &self.names {
            raw_tables::write_name_entry(&mut names, n, NAME_FLAGS);
        }
        let mut imports = Vec::new();
        for i in &self.imports {
            i.write(&mut imports);
        }
        // Entry sizes do not depend on size/offset values, so lay out twice.
        let entry_len: usize = self
            .exports
            .iter()
            .map(|(e, _)| {
                let mut b = Vec::new();
                e.write(&mut b);
                b.len()
            })
            .sum();
        let summary_len = 4 + 4 + 4 + (4 + 5) + 4 + 7 * 4 + 16 + 4 + 12 + 4 + 4 + 4 + 4;
        let name_offset = summary_len;
        let import_offset = name_offset + names.len();
        let export_offset = import_offset + imports.len();
        let depends_offset = export_offset + entry_len;
        let total_header_size = depends_offset + self.exports.len() * 4;

        let mut exports = Vec::new();
        let mut data = Vec::new();
        for (e, d) in &self.exports {
            let mut e = e.clone();
            e.serial_size = d.len() as i32;
            e.serial_offset = (total_header_size + data.len()) as i32;
            e.write(&mut exports);
            data.extend_from_slice(d);
        }

        let mut out = Vec::new();
        out.extend_from_slice(&0x9E2A_83C1u32.to_le_bytes());
        out.extend_from_slice(&(486u32 | (8 << 16)).to_le_bytes());
        Self::i32s(&mut out, &[total_header_size as i32, 5]);
        out.extend_from_slice(b"None\0");
        // PKG_StoreCompressed set on purpose: finish() must clear it.
        out.extend_from_slice(&0x020A_0009u32.to_le_bytes());
        Self::i32s(
            &mut out,
            &[
                self.names.len() as i32,
                name_offset as i32,
                self.exports.len() as i32,
                export_offset as i32,
                self.imports.len() as i32,
                import_offset as i32,
                depends_offset as i32,
            ],
        );
        out.extend_from_slice(&[0xAB; 16]);
        Self::i32s(
            &mut out,
            &[1, self.exports.len() as i32, self.names.len() as i32, 0],
        );
        Self::i32s(&mut out, &[3004, 41, 0, 0]);
        assert_eq!(out.len(), summary_len);
        out.extend_from_slice(&names);
        out.extend_from_slice(&imports);
        out.extend_from_slice(&exports);
        out.resize(out.len() + self.exports.len() * 4, 0);
        out.extend_from_slice(&data);
        out
    }

    fn tag(&mut self, out: &mut Vec<u8>, name: &str, ty: &str, size: i32) {
        let (n, t) = (self.name(name), self.name(ty));
        Self::i32s(out, &[n, 0, t, 0, size, 0]);
    }

    fn object_prop(&mut self, out: &mut Vec<u8>, name: &str, obj: i32) {
        self.tag(out, name, "ObjectProperty", 4);
        Self::i32s(out, &[obj]);
    }

    fn vector_prop(&mut self, out: &mut Vec<u8>, name: &str, v: [f32; 3]) {
        self.tag(out, name, "StructProperty", 12);
        let s = self.name("Vector");
        Self::i32s(out, &[s, 0]);
        for c in v {
            out.extend_from_slice(&c.to_le_bytes());
        }
    }

    fn none(&mut self, out: &mut Vec<u8>) {
        let n = self.name("None");
        Self::i32s(out, &[n, 0]);
    }
}

fn temp_path(tag: &str) -> PathBuf {
    std::env::temp_dir().join(format!("cimmeria-upk-{}-{tag}.upk", std::process::id()))
}

fn write_temp(tag: &str, bytes: &[u8]) -> PathBuf {
    let p = temp_path(tag);
    std::fs::write(&p, bytes).unwrap();
    p
}

/// A level package. `actors` lists the refs already in the level's actor array.
fn level_package(b: &mut Builder, actors: &[i32]) -> i32 {
    let level_class = b.import("Core", "Class", 0, "Level");
    let level_ref = b.exports.len() as i32 + 1;
    let mut data = Vec::new();
    Builder::i32s(&mut data, &[7]); // NetIndex
    b.none(&mut data);
    Builder::i32s(&mut data, &[level_ref, actors.len() as i32]);
    Builder::i32s(&mut data, actors);
    data.extend_from_slice(b"TAIL");
    b.export(level_class, 0, "PersistentLevel", data)
}

/// Source package: a level, one StaticMeshActor at (100, 200, 300), and its
/// component. Decoy exports keep source refs from lining up with target refs.
fn source_package() -> Vec<u8> {
    let mut b = Builder::default();
    let pkg = b.import("Core", "Package", 0, "GLB-Global");
    let mesh = b.import("Engine", "StaticMesh", pkg, "GLB-RingTransporterBase_TC00");
    let actor_class = b.import("Core", "Class", 0, "StaticMeshActor");
    let comp_class = b.import("Core", "Class", 0, "StaticMeshComponent");
    let decoy_class = b.import("Core", "Class", 0, "Decoy");
    b.export(decoy_class, 0, "Decoy", vec![0; 12]);
    let level = level_package(&mut b, &[]);
    // An unrelated export sits between the actor and its component, as in real
    // chunks: that gap is what makes a ComponentMap index differ from a ref.
    let (actor_ref, comp_ref) = (level + 1, level + 3);

    let mut actor = Vec::new();
    Builder::i32s(
        &mut actor,
        &[actor_class, actor_class, -1, -1, 0x6f0074, 0, -1, 55],
    );
    b.object_prop(&mut actor, "StaticMeshComponent", comp_ref);
    b.vector_prop(&mut actor, "Location", [100.0, 200.0, 300.0]);
    b.none(&mut actor);
    assert_eq!(
        b.export(actor_class, level, "StaticMeshActor", actor),
        actor_ref
    );
    let cm_name = (b.name("StaticMeshComponent0"), 0);
    b.exports[actor_ref as usize - 1]
        .0
        .component_map
        .push((cm_name, comp_ref - 1));

    b.export(decoy_class, 0, "Decoy", vec![0; 12]);

    let mut comp = Vec::new();
    Builder::i32s(&mut comp, &[0, 99]);
    b.object_prop(&mut comp, "StaticMesh", mesh);
    b.none(&mut comp);
    Builder::i32s(&mut comp, &[0]); // empty LODData
    assert_eq!(
        b.export(comp_class, actor_ref, "StaticMeshComponent", comp),
        comp_ref
    );
    b.build()
}

/// Target package: a level holding one pre-existing actor, no mesh imports,
/// and the class name in different case to exercise case-insensitive lookup.
fn target_package() -> Vec<u8> {
    let mut b = Builder::default();
    b.name("staticmeshactor");
    let other_class = b.import("Core", "Class", 0, "Trigger");
    let level = level_package(&mut b, &[2]);
    b.export(other_class, level, "Trigger", vec![0; 40]);
    b.build()
}

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
    assert_eq!(report.level_actor_count, (1, 2));
    assert_eq!(report.exports_added, 2);
    // Package, mesh, actor class, component class. "staticmeshactor" already
    // exists as a name (different case), so it must not be added again.
    assert_eq!(report.imports_added, 4);
    assert_eq!(report.actors[0].location, [1000.0, 2000.0, 3000.0]);

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
fn clone_rejects_a_property_type_it_cannot_remap() {
    let mut b = Builder::default();
    let actor_class = b.import("Core", "Class", 0, "StaticMeshActor");
    let level = level_package(&mut b, &[]);
    let mut actor = Vec::new();
    Builder::i32s(&mut actor, &[actor_class, actor_class, -1, -1, 0, 0, -1, 1]);
    b.vector_prop(&mut actor, "Location", [0.0, 0.0, 1.0]);
    b.tag(&mut actor, "Touching", "ArrayProperty", 4);
    Builder::i32s(&mut actor, &[0]);
    b.none(&mut actor);
    b.export(actor_class, level, "StaticMeshActor", actor);
    let src_path = write_temp("arr-src", &b.build());
    let dst_path = write_temp("arr-dst", &target_package());

    let source = PatchSession::open(&src_path).unwrap();
    let mut target = PatchSession::open(&dst_path).unwrap();
    let e = clone_actors(&mut target, &source, &[1], Placement::Offset([0.0; 3])).unwrap_err();
    assert!(e.to_string().contains("ArrayProperty"), "{e}");
    let _ = [src_path, dst_path].map(std::fs::remove_file);
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
