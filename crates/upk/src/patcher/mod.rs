//! Append-only package patcher.
//!
//! Cooked SGW packages store absolute file offsets inside export data (bulk
//! data headers), so a writer that re-lays the file out has to find and fix
//! every one of them. This patcher sidesteps that: **no existing byte moves.**
//!
//! ```text
//! [summary]            patched in place (counts, offsets, compression cleared)
//! [original body]      verbatim, at its original offsets; old tables become dead space
//! [new export data]    appended
//! [name table]         original bytes + new entries
//! [import table]       original bytes + new entries
//! [export table]       original entries (size/offset patched where data was replaced) + new
//! [depends table]      original bytes + one empty list per new export
//! ```
//!
//! The tables stay contiguous from `name_offset` to `total_header_size`, which
//! is the only layout assumption the UE3 linker makes (it precaches that span).
//! Output is always uncompressed.

mod actor_clone;
mod property_remap;
pub mod raw_tables;

use std::collections::{BTreeMap, HashMap};
use std::ops::Range;
use std::path::Path;

use byteorder::{ByteOrder, LittleEndian};

use crate::error::{Result, UpkError};
use crate::package::Package;
use raw_tables::{RawExport, RawImport, SummaryLayout, EXPORT_SERIAL_SIZE_AT, IMPORT_ENTRY_SIZE};

pub use actor_clone::{clone_actors, CloneReport, Placement};

/// `PKG_StoreCompressed`; must be cleared when the output is uncompressed.
const PKG_STORE_COMPRESSED: u32 = 0x0200_0000;

/// A package opened for patching.
pub struct PatchSession {
    pub package: Package,
    image: Vec<u8>,
    layout: SummaryLayout,
    names_end: usize,
    all_names: Vec<String>,
    name_lookup: HashMap<String, i32>,
    original_name_count: usize,
    new_name_flags: u64,
    imports: Vec<RawImport>,
    original_import_count: usize,
    import_lookup: HashMap<String, i32>,
    exports: Vec<(RawExport, Range<usize>)>,
    new_exports: Vec<(RawExport, Vec<u8>)>,
    replaced: BTreeMap<usize, Vec<u8>>,
}

impl PatchSession {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let package = Package::open(path)?;
        let image = package.image()?;
        let h = &package.header;
        let layout = SummaryLayout::locate(&image, h)?;
        if layout.uncompressed_summary_end > h.name_offset as usize {
            return Err(UpkError::Parse(format!(
                "uncompressed summary ({} bytes) would overlap the body at {}",
                layout.uncompressed_summary_end, h.name_offset
            )));
        }
        let names_end =
            raw_tables::name_table_end(&image, h.name_offset as usize, h.name_count as usize)?;
        let imports =
            raw_tables::read_imports(&image, h.import_offset as usize, h.import_count as usize)?;
        let exports =
            raw_tables::read_exports(&image, h.export_offset as usize, h.export_count as usize)?;

        let depends_len = (h.total_header_size - h.depends_offset) as usize;
        if depends_len != exports.len() * 4 {
            // Every QA chunk inspected has one empty list per export. A package
            // with real dependency lists needs a proper walk, not an append.
            return Err(UpkError::Parse(format!(
                "depends table is {depends_len} bytes for {} exports; expected 4 each",
                exports.len()
            )));
        }

        let all_names: Vec<String> = package.names.iter().map(|n| n.name.clone()).collect();
        let mut name_lookup = HashMap::new();
        for (i, n) in all_names.iter().enumerate() {
            name_lookup.entry(n.to_lowercase()).or_insert(i as i32);
        }
        // New names take the flags most existing names carry.
        let mut flag_counts: HashMap<u64, usize> = HashMap::new();
        for n in &package.names {
            *flag_counts.entry(n.flags).or_default() += 1;
        }
        let new_name_flags = flag_counts
            .into_iter()
            .max_by_key(|&(_, c)| c)
            .map(|(f, _)| f)
            .unwrap_or(0);

        let mut session = Self {
            original_name_count: all_names.len(),
            original_import_count: imports.len(),
            package,
            image,
            layout,
            names_end,
            all_names,
            name_lookup,
            new_name_flags,
            imports,
            import_lookup: HashMap::new(),
            exports,
            new_exports: Vec::new(),
            replaced: BTreeMap::new(),
        };
        for i in 0..session.imports.len() {
            let key = session.import_key(i)?;
            session.import_lookup.entry(key).or_insert(-(i as i32) - 1);
        }
        Ok(session)
    }

    pub fn name(&self, index: i32) -> Result<&str> {
        self.all_names
            .get(index as usize)
            .map(String::as_str)
            .ok_or_else(|| UpkError::Parse(format!("name index {index} out of range")))
    }

    /// Index of `name`, adding it to the name table if absent. UE3 names are
    /// case-insensitive, so the lookup is too.
    pub fn ensure_name(&mut self, name: &str) -> i32 {
        if let Some(&i) = self.name_lookup.get(&name.to_lowercase()) {
            return i;
        }
        let i = self.all_names.len() as i32;
        self.all_names.push(name.to_string());
        self.name_lookup.insert(name.to_lowercase(), i);
        i
    }

    pub fn raw_export(&self, index: usize) -> Result<&RawExport> {
        self.exports
            .get(index)
            .map(|(e, _)| e)
            .ok_or_else(|| UpkError::Parse(format!("export index {index} out of range")))
    }

    pub fn export_count(&self) -> usize {
        self.exports.len() + self.new_exports.len()
    }

    /// Current serial data of an original export (the replacement, if one was set).
    pub fn export_data(&self, index: usize) -> Result<&[u8]> {
        if let Some(data) = self.replaced.get(&index) {
            return Ok(data);
        }
        let e = self.raw_export(index)?;
        let (start, len) = (e.serial_offset as usize, e.serial_size as usize);
        self.image
            .get(start..start + len)
            .ok_or_else(|| UpkError::Parse(format!("export {index} data out of image bounds")))
    }

    /// Index of the `Level` export (`TheWorld.PersistentLevel`).
    pub fn level_export_index(&self) -> Result<usize> {
        let mut found = self
            .package
            .exports
            .iter()
            .enumerate()
            .filter(|(_, e)| self.package.export_class_name(e) == "Level");
        match (found.next(), found.next()) {
            (Some((i, _)), None) => Ok(i),
            (None, _) => Err(UpkError::Parse("package has no Level export".into())),
            _ => Err(UpkError::Parse(
                "package has more than one Level export".into(),
            )),
        }
    }

    /// `ClassPackage.ClassName|Outer.Path.Name`, lowercased: identifies an import
    /// independently of which package's tables it sits in.
    fn import_key(&self, index: usize) -> Result<String> {
        let imp = &self.imports[index];
        let mut parts = vec![self.name(imp.object_name.0)?.to_string()];
        let mut outer = imp.outer;
        let mut depth = 0;
        while outer != 0 {
            depth += 1;
            if depth > 32 {
                return Err(UpkError::Parse(format!(
                    "import {index}: outer chain loops"
                )));
            }
            if outer > 0 {
                return Err(UpkError::Parse(format!(
                    "import {index} is outered to an export; not supported"
                )));
            }
            let o = self
                .imports
                .get((-outer - 1) as usize)
                .ok_or_else(|| UpkError::Parse(format!("import {index}: outer out of range")))?;
            parts.push(self.name(o.object_name.0)?.to_string());
            outer = o.outer;
        }
        parts.reverse();
        Ok(format!(
            "{}.{}|{}",
            self.name(imp.class_package.0)?,
            self.name(imp.class_name.0)?,
            parts.join(".")
        )
        .to_lowercase())
    }

    /// Map an import ref from `source` to the equivalent ref here, adding the
    /// import (and its outer chain, and any names) when this package lacks it.
    pub fn ensure_import_from(&mut self, source: &PatchSession, source_ref: i32) -> Result<i32> {
        if source_ref >= 0 {
            return Err(UpkError::Parse(format!(
                "{source_ref} is not an import ref"
            )));
        }
        let src_index = (-source_ref - 1) as usize;
        if src_index >= source.imports.len() {
            return Err(UpkError::Parse(format!(
                "source import ref {source_ref} out of range"
            )));
        }
        let key = source.import_key(src_index)?;
        if let Some(&r) = self.import_lookup.get(&key) {
            return Ok(r);
        }
        let src = source.imports[src_index].clone();
        let outer = if src.outer == 0 {
            0
        } else {
            self.ensure_import_from(source, src.outer)?
        };
        let remap = |s: &mut Self, n: (i32, i32)| -> Result<(i32, i32)> {
            Ok((s.ensure_name(source.name(n.0)?), n.1))
        };
        let entry = RawImport {
            class_package: remap(self, src.class_package)?,
            class_name: remap(self, src.class_name)?,
            outer,
            object_name: remap(self, src.object_name)?,
        };
        self.imports.push(entry);
        let r = -(self.imports.len() as i32);
        self.import_lookup.insert(key, r);
        Ok(r)
    }

    /// Queue a new export. `entry.serial_size` / `serial_offset` are filled in
    /// by [`Self::finish`]. Returns the new object's 1-based ref.
    pub fn add_export(&mut self, entry: RawExport, data: Vec<u8>) -> i32 {
        self.new_exports.push((entry, data));
        self.export_count() as i32
    }

    /// The ref the next [`Self::add_export`] call will return.
    pub fn next_export_ref(&self) -> i32 {
        self.export_count() as i32 + 1
    }

    /// Replace an original export's serial data. The new data is appended to
    /// the file; the old bytes stay where they are as dead space.
    pub fn replace_export_data(&mut self, index: usize, data: Vec<u8>) -> Result<()> {
        self.raw_export(index)?;
        self.replaced.insert(index, data);
        Ok(())
    }

    /// Number of names, imports and exports this session has added.
    pub fn additions(&self) -> (usize, usize, usize) {
        (
            self.all_names.len() - self.original_name_count,
            self.imports.len() - self.original_import_count,
            self.new_exports.len(),
        )
    }

    /// Serialize the patched, uncompressed package.
    pub fn finish(mut self) -> Result<Vec<u8>> {
        let h = self.package.header.clone();
        let mut out = std::mem::take(&mut self.image);

        // Appended export data.
        let mut relocated: HashMap<usize, (i32, i32)> = HashMap::new();
        for (&index, data) in &self.replaced {
            relocated.insert(index, (data.len() as i32, out.len() as i32));
            out.extend_from_slice(data);
        }
        for (entry, data) in &mut self.new_exports {
            entry.serial_size = data.len() as i32;
            entry.serial_offset = out.len() as i32;
            out.extend_from_slice(data);
        }

        // Tables, contiguous, at the end.
        let name_offset = out.len();
        let original_names = out[h.name_offset as usize..self.names_end].to_vec();
        out.extend_from_slice(&original_names);
        for name in &self.all_names[self.original_name_count..] {
            raw_tables::write_name_entry(&mut out, name, self.new_name_flags);
        }

        let import_offset = out.len();
        let original_imports = out[h.import_offset as usize
            ..h.import_offset as usize + self.original_import_count * IMPORT_ENTRY_SIZE]
            .to_vec();
        out.extend_from_slice(&original_imports);
        for imp in &self.imports[self.original_import_count..] {
            imp.write(&mut out);
        }

        let export_offset = out.len();
        for (index, (_, range)) in self.exports.iter().enumerate() {
            let mut bytes = out[range.clone()].to_vec();
            if let Some(&(size, offset)) = relocated.get(&index) {
                LittleEndian::write_i32(&mut bytes[EXPORT_SERIAL_SIZE_AT..], size);
                LittleEndian::write_i32(&mut bytes[EXPORT_SERIAL_SIZE_AT + 4..], offset);
            }
            out.extend_from_slice(&bytes);
        }
        for (entry, _) in &self.new_exports {
            entry.write(&mut out);
        }

        let depends_offset = out.len();
        let original_depends =
            out[h.depends_offset as usize..h.total_header_size as usize].to_vec();
        out.extend_from_slice(&original_depends);
        out.resize(out.len() + self.new_exports.len() * 4, 0);
        let total_header_size = out.len();

        // Summary.
        let l = self.layout;
        let export_count = self.exports.len() + self.new_exports.len();
        let put = |out: &mut Vec<u8>, at: usize, v: usize| {
            LittleEndian::write_i32(&mut out[at..], v as i32);
        };
        put(&mut out, l.total_header_size_at, total_header_size);
        put(&mut out, l.name_count_at, self.all_names.len());
        put(&mut out, l.name_offset_at, name_offset);
        put(&mut out, l.export_count_at, export_count);
        put(&mut out, l.export_offset_at, export_offset);
        put(&mut out, l.import_count_at, self.imports.len());
        put(&mut out, l.import_offset_at, import_offset);
        put(&mut out, l.depends_offset_at, depends_offset);
        LittleEndian::write_u32(
            &mut out[l.package_flags_at..],
            h.package_flags & !PKG_STORE_COMPRESSED,
        );
        if let Some(last) = h.generations.len().checked_sub(1) {
            let at = l.generations_at + last * 12;
            put(&mut out, at, export_count);
            put(&mut out, at + 4, self.all_names.len());
        }
        // compression_flags = 0, chunk count = 0.
        out[l.compression_flags_at..l.compression_flags_at + 8].fill(0);

        Ok(out)
    }
}

#[cfg(test)]
mod tests;
