//! Writer for an uncompressed Epic-486 UE3 package.
//!
//! Layout, in file order — the mirror image of what
//! [`cimmeria_upk::Package::open`] reads:
//!
//! ```text
//! FPackageFileSummary   (tag, version, offsets, GUID, generations, ...)
//! name table            name_count  x (FString + u64 flags)
//! import table          import_count x 28 bytes
//! export table          export_count x 68 bytes
//! export bodies         concatenated, in export order
//! ```
//!
//! `compression_flags` is zero and there are no compressed chunks, so
//! the reader takes its uncompressed path and `read_export_data`
//! seeks into the file directly. The 68-byte export record is only
//! fixed-width because every fixture leaves `ComponentMap` and
//! `GenerationNetObjectCount` empty; the encoder debug-asserts that
//! assumption against its own output.

use std::path::Path;

use super::names::{push_fstring, NameTable};
use super::props::PropStream;

/// Epic package version SGW cooks at.
pub const EPIC_VERSION: u16 = 486;

/// Licensee version. Zero rather than a guessed SGW value — nothing in
/// `cimmeria-upk`'s parse path branches on it, so inventing a number
/// would be a false claim in a fixture.
pub const LICENSEE_VERSION: u16 = 0;

struct ImportRecord {
    class_package: i32,
    class_name: i32,
    outer: i32,
    object_name: i32,
}

impl ImportRecord {
    const SIZE: usize = 28;

    fn encode(&self, out: &mut Vec<u8>) {
        let before = out.len();
        push_fname(out, self.class_package);
        push_fname(out, self.class_name);
        out.extend_from_slice(&self.outer.to_le_bytes());
        push_fname(out, self.object_name);
        debug_assert_eq!(out.len() - before, Self::SIZE);
    }
}

struct ExportRecord {
    class: i32,
    outer: i32,
    name: i32,
    archetype: i32,
    payload: Vec<u8>,
}

impl ExportRecord {
    /// Fixed because `ComponentMap` and `GenerationNetObjectCount` are
    /// always empty here: 3 index i32s, FName, archetype, u64 flags,
    /// size, offset, empty-map count, export flags, empty-array count,
    /// 16-byte GUID.
    //
    // Commas, not `+`: a wrapped line starting with `+ ` reads as a
    // Markdown list item to clippy's `doc_lazy_continuation`.
    const SIZE: usize = 68;

    fn encode(&self, out: &mut Vec<u8>, serial_offset: i32) {
        let before = out.len();
        out.extend_from_slice(&self.class.to_le_bytes());
        out.extend_from_slice(&0i32.to_le_bytes()); // SuperIndex
        out.extend_from_slice(&self.outer.to_le_bytes());
        push_fname(out, self.name);
        out.extend_from_slice(&self.archetype.to_le_bytes());
        out.extend_from_slice(&0u64.to_le_bytes()); // ObjectFlags
        out.extend_from_slice(&(self.payload.len() as i32).to_le_bytes());
        out.extend_from_slice(&serial_offset.to_le_bytes());
        out.extend_from_slice(&0i32.to_le_bytes()); // ComponentMap count
        out.extend_from_slice(&0u32.to_le_bytes()); // ExportFlags
        out.extend_from_slice(&0i32.to_le_bytes()); // GenNetObjCount count
        out.extend_from_slice(&[0u8; 16]); // PackageGuid
        debug_assert_eq!(out.len() - before, Self::SIZE);
    }
}

fn push_fname(out: &mut Vec<u8>, index: i32) {
    out.extend_from_slice(&index.to_le_bytes());
    out.extend_from_slice(&0i32.to_le_bytes());
}

/// Assembles a package byte-for-byte.
///
/// Object indices follow the UE3 convention everywhere: a positive
/// value is a 1-based export, a negative value is a negated 1-based
/// import, and 0 is "none". [`add_export`](Self::add_export) and
/// [`add_import`](Self::add_import) hand back indices in that form, so
/// they can be passed straight to `ObjectProperty` values, `Outer`
/// fields and class references without any conversion at the call site
/// — the off-by-one there is the single easiest way to build a fixture
/// that parses but describes the wrong object graph.
#[derive(Default)]
pub struct PackageBuilder {
    names: NameTable,
    imports: Vec<ImportRecord>,
    exports: Vec<ExportRecord>,
}

impl PackageBuilder {
    pub fn new() -> Self {
        Self {
            names: NameTable::new(),
            imports: Vec::new(),
            exports: Vec::new(),
        }
    }

    /// Start a tagged-property stream that interns into this package's
    /// name table.
    pub fn props(&mut self) -> PropStream<'_> {
        PropStream::new(&mut self.names)
    }

    /// Intern a name directly.
    pub fn intern(&mut self, name: &str) -> i32 {
        self.names.intern(name)
    }

    /// Add an import; returns its negative object index.
    pub fn add_import(
        &mut self,
        class_package: &str,
        class_name: &str,
        outer: i32,
        object_name: &str,
    ) -> i32 {
        let rec = ImportRecord {
            class_package: self.names.intern(class_package),
            class_name: self.names.intern(class_name),
            outer,
            object_name: self.names.intern(object_name),
        };
        self.imports.push(rec);
        -(self.imports.len() as i32)
    }

    /// A `Core.Class` import named `class_name`, reused if one already
    /// exists.
    ///
    /// Cooked `.umap` exports name their class through an import, and
    /// `Package::export_class_name` resolves it by object name — so
    /// this is what makes `pkg.export_class_name(e) == "Terrain"` true.
    pub fn class_ref(&mut self, class_name: &str) -> i32 {
        if let Some(existing) = self
            .imports
            .iter()
            .position(|i| Some(i.object_name) == self.names.get(class_name) && i.outer == 0)
        {
            return -((existing + 1) as i32);
        }
        self.add_import("Core", "Class", 0, class_name)
    }

    /// Add an export with an empty body; returns its positive object
    /// index. Fill the body in later with
    /// [`set_payload`](Self::set_payload) — an actor's property stream
    /// usually has to reference the index of a component added after
    /// it.
    pub fn add_export(&mut self, class: i32, outer: i32, name: &str) -> i32 {
        let rec = ExportRecord {
            class,
            outer,
            name: self.names.intern(name),
            archetype: 0,
            payload: Vec::new(),
        };
        self.exports.push(rec);
        self.exports.len() as i32
    }

    /// Set an export's serial body.
    pub fn set_payload(&mut self, export: i32, payload: Vec<u8>) {
        self.export_mut(export).payload = payload;
    }

    /// Set an export's `Archetype` field — what
    /// `staticmesh::mesh_ref::archetype_label` reads to decide an actor
    /// was instanced from a prefab template.
    pub fn set_archetype(&mut self, export: i32, archetype: i32) {
        self.export_mut(export).archetype = archetype;
    }

    fn export_mut(&mut self, export: i32) -> &mut ExportRecord {
        assert!(export > 0, "export index is 1-based and positive");
        self.exports
            .get_mut((export - 1) as usize)
            .unwrap_or_else(|| panic!("no export {export} in this fixture"))
    }

    /// Number of exports added so far.
    pub fn export_count(&self) -> usize {
        self.exports.len()
    }

    /// Serialise the whole package.
    pub fn build(&self) -> Vec<u8> {
        let names = self.names.to_bytes();
        let header_len = self.header_bytes(0, 0, 0, 0).len();
        let name_offset = header_len as i32;
        let import_offset = name_offset + names.len() as i32;
        let export_offset = import_offset + (self.imports.len() * ImportRecord::SIZE) as i32;
        let payload_base = export_offset + (self.exports.len() * ExportRecord::SIZE) as i32;

        let mut out = self.header_bytes(name_offset, import_offset, export_offset, payload_base);
        assert_eq!(
            out.len(),
            header_len,
            "header length must not depend on the offsets written into it"
        );
        out.extend_from_slice(&names);
        for i in &self.imports {
            i.encode(&mut out);
        }

        let mut cursor = payload_base;
        for e in &self.exports {
            let offset = if e.payload.is_empty() { 0 } else { cursor };
            e.encode(&mut out, offset);
            cursor += e.payload.len() as i32;
        }
        assert_eq!(out.len(), payload_base as usize);
        for e in &self.exports {
            out.extend_from_slice(&e.payload);
        }
        out
    }

    /// Serialise and write to `path`, creating parent directories.
    pub fn write_to(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        std::fs::write(path, self.build())
    }

    fn header_bytes(
        &self,
        name_offset: i32,
        import_offset: i32,
        export_offset: i32,
        total_header_size: i32,
    ) -> Vec<u8> {
        let mut h = Vec::new();
        h.extend_from_slice(&cimmeria_upk::header::PACKAGE_FILE_TAG.to_le_bytes());
        let packed = (EPIC_VERSION as u32) | ((LICENSEE_VERSION as u32) << 16);
        h.extend_from_slice(&packed.to_le_bytes());
        h.extend_from_slice(&total_header_size.to_le_bytes());
        push_fstring(&mut h, "None"); // FolderName
        h.extend_from_slice(&0u32.to_le_bytes()); // PackageFlags
        h.extend_from_slice(&(self.names.len() as i32).to_le_bytes());
        h.extend_from_slice(&name_offset.to_le_bytes());
        h.extend_from_slice(&(self.exports.len() as i32).to_le_bytes());
        h.extend_from_slice(&export_offset.to_le_bytes());
        h.extend_from_slice(&(self.imports.len() as i32).to_le_bytes());
        h.extend_from_slice(&import_offset.to_le_bytes());
        h.extend_from_slice(&total_header_size.to_le_bytes()); // DependsOffset
        h.extend_from_slice(&[0u8; 16]); // GUID
        h.extend_from_slice(&1i32.to_le_bytes()); // one generation
        h.extend_from_slice(&(self.exports.len() as i32).to_le_bytes());
        h.extend_from_slice(&(self.names.len() as i32).to_le_bytes());
        h.extend_from_slice(&0i32.to_le_bytes()); // NetObjectCount
        h.extend_from_slice(&0i32.to_le_bytes()); // EngineVersion
        h.extend_from_slice(&0i32.to_le_bytes()); // CookerVersion
        h.extend_from_slice(&0u32.to_le_bytes()); // CompressionFlags — none
        h.extend_from_slice(&0i32.to_le_bytes()); // compressed chunk count
        h
    }
}
