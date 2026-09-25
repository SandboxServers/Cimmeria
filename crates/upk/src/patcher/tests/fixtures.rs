//! Synthetic Epic-486 packages for the patcher tests.

use std::path::PathBuf;

use super::super::raw_tables::{self, RawExport, RawImport};
use super::super::{clone_objects, CloneReport, CloneRequest, PatchSession, Placement};
use crate::error::Result;

const NAME_FLAGS: u64 = 0x0007_0010_0000_0000;
const RF_HAS_STACK: u64 = 0x0200_0000_0000_0000;

#[derive(Default)]
pub(super) struct Builder {
    pub(super) names: Vec<String>,
    imports: Vec<RawImport>,
    pub(super) exports: Vec<(RawExport, Vec<u8>)>,
}

impl Builder {
    pub(super) fn name(&mut self, s: &str) -> i32 {
        if let Some(i) = self.names.iter().position(|n| n == s) {
            return i as i32;
        }
        self.names.push(s.to_string());
        self.names.len() as i32 - 1
    }

    /// Returns the import's (negative) ref.
    pub(super) fn import(&mut self, class_pkg: &str, class: &str, outer: i32, name: &str) -> i32 {
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
    pub(super) fn export(&mut self, class: i32, outer: i32, name: &str, data: Vec<u8>) -> i32 {
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

    /// Mark the most recent export as an actor (it serializes a state frame).
    pub(super) fn mark_actor(&mut self) {
        self.exports.last_mut().unwrap().0.object_flags |= RF_HAS_STACK;
    }

    pub(super) fn object_array_prop(&mut self, out: &mut Vec<u8>, name: &str, refs: &[i32]) {
        self.tag(out, name, "ArrayProperty", 4 + refs.len() as i32 * 4);
        Self::i32s(out, &[refs.len() as i32]);
        Self::i32s(out, refs);
    }

    pub(super) fn struct_array_prop(
        &mut self,
        out: &mut Vec<u8>,
        name: &str,
        elements: &[Vec<u8>],
    ) {
        let len: usize = elements.iter().map(Vec::len).sum();
        self.tag(out, name, "ArrayProperty", 4 + len as i32);
        Self::i32s(out, &[elements.len() as i32]);
        for e in elements {
            out.extend_from_slice(e);
        }
    }

    pub(super) fn i32s(out: &mut Vec<u8>, vals: &[i32]) {
        for v in vals {
            out.extend_from_slice(&v.to_le_bytes());
        }
    }

    pub(super) fn build(&self) -> Vec<u8> {
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

    pub(super) fn tag(&mut self, out: &mut Vec<u8>, name: &str, ty: &str, size: i32) {
        let (n, t) = (self.name(name), self.name(ty));
        Self::i32s(out, &[n, 0, t, 0, size, 0]);
    }

    pub(super) fn object_prop(&mut self, out: &mut Vec<u8>, name: &str, obj: i32) {
        self.tag(out, name, "ObjectProperty", 4);
        Self::i32s(out, &[obj]);
    }

    pub(super) fn vector_prop(&mut self, out: &mut Vec<u8>, name: &str, v: [f32; 3]) {
        self.tag(out, name, "StructProperty", 12);
        let s = self.name("Vector");
        Self::i32s(out, &[s, 0]);
        for c in v {
            out.extend_from_slice(&c.to_le_bytes());
        }
    }

    pub(super) fn none(&mut self, out: &mut Vec<u8>) {
        let n = self.name("None");
        Self::i32s(out, &[n, 0]);
    }
}

pub(super) fn clone_actors(
    target: &mut PatchSession,
    source: &PatchSession,
    roots: &[usize],
    placement: Placement,
) -> Result<CloneReport> {
    let request = CloneRequest {
        roots,
        mapped: &[],
        placement,
    };
    clone_objects(target, source, &request)
}

pub(super) fn temp_path(tag: &str) -> PathBuf {
    std::env::temp_dir().join(format!("cimmeria-upk-{}-{tag}.upk", std::process::id()))
}

pub(super) fn write_temp(tag: &str, bytes: &[u8]) -> PathBuf {
    let p = temp_path(tag);
    std::fs::write(&p, bytes).unwrap();
    p
}

/// A level package. `actors` lists the refs already in the level's actor array.
pub(super) fn level_package(b: &mut Builder, actors: &[i32]) -> i32 {
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
pub(super) fn source_package() -> Vec<u8> {
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
    b.mark_actor();
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
pub(super) fn target_package() -> Vec<u8> {
    let mut b = Builder::default();
    b.name("staticmeshactor");
    let other_class = b.import("Core", "Class", 0, "Trigger");
    let level = level_package(&mut b, &[2]);
    b.export(other_class, level, "Trigger", vec![0; 40]);
    b.build()
}

/// A level with a wired Kismet rig, shaped like the real ring transporter:
///   [0] Level   [1] Prefabs sequence   [2] rig Sequence (in Prefabs)
///   [3] SeqVar_Object (in rig) -> ring actor   [4] SeqEvent (in rig) -> links to the var
///   [5] ring InterpActor at (100, 0, 50)   [6] base StaticMeshActor at (100, 0, 0), yaw 0
///   [7] un-wired pad base at (0, 500, 0), yaw 16384 (90 degrees)
pub(super) fn rig_package() -> Vec<u8> {
    let mut b = Builder::default();
    let seq_class = b.import("Core", "Class", 0, "Sequence");
    let var_class = b.import("Core", "Class", 0, "SeqVar_Object");
    let event_class = b.import("Core", "Class", 0, "SeqEvent_RegionTeleport");
    let interp_class = b.import("Core", "Class", 0, "InterpActor");
    let sma_class = b.import("Core", "Class", 0, "StaticMeshActor");
    let level = level_package(&mut b, &[6, 7, 8]);
    let (prefabs, rig, var, event, ring) = (level + 1, level + 2, level + 3, level + 4, level + 5);

    let mut data = vec![0, 0, 0, 0];
    b.object_array_prop(&mut data, "SequenceObjects", &[rig]);
    b.none(&mut data);
    b.export(seq_class, 0, "Prefabs", data);

    let mut data = vec![0, 0, 0, 0];
    b.object_array_prop(&mut data, "SequenceObjects", &[var, event]);
    b.object_prop(&mut data, "ParentSequence", prefabs);
    b.none(&mut data);
    b.export(seq_class, prefabs, "Rig_Seq", data);

    let mut data = vec![0, 0, 0, 0];
    b.object_prop(&mut data, "ObjValue", ring);
    b.object_prop(&mut data, "ParentSequence", rig);
    b.none(&mut data);
    b.export(var_class, rig, "SeqVar_Object", data);

    // OutputLinks[0].Links[0].LinkedOp -> the var: a ref two arrays deep.
    let mut link = Vec::new();
    b.object_prop(&mut link, "LinkedOp", var);
    b.none(&mut link);
    let mut output = Vec::new();
    b.struct_array_prop(&mut output, "Links", &[link]);
    b.none(&mut output);
    let mut data = vec![0, 0, 0, 0];
    b.struct_array_prop(&mut data, "OutputLinks", &[output]);
    b.object_array_prop(&mut data, "LinkedVariables", &[var]);
    b.object_prop(&mut data, "ParentSequence", rig);
    // Last, so the odd byte does not misalign the word diff in the test below.
    b.tag(&mut data, "EventType", "ByteProperty", 1);
    data.push(1);
    b.none(&mut data);
    assert_eq!(
        b.export(event_class, rig, "SeqEvent_RegionTeleport", data),
        event
    );

    for (class, name, loc, yaw) in [
        (interp_class, "InterpActor", [100.0, 0.0, 50.0], 0),
        (sma_class, "StaticMeshActor", [100.0, 0.0, 0.0], 0),
        (sma_class, "StaticMeshActor", [0.0, 500.0, 0.0], 16384),
    ] {
        let mut data = Vec::new();
        Builder::i32s(&mut data, &[class, class, -1, -1, 0, 0, -1, 9]);
        b.vector_prop(&mut data, "Location", loc);
        b.tag(&mut data, "Rotation", "StructProperty", 12);
        let rot = b.name("Rotator");
        Builder::i32s(&mut data, &[rot, 0, 0, yaw, 0]);
        b.none(&mut data);
        let r = b.export(class, level, name, data);
        b.mark_actor();
        // Distinct instance numbers, as the editor assigns them.
        b.exports[r as usize - 1].0.object_name.1 = r;
    }
    b.build()
}
