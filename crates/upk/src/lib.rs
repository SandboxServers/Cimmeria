//! UE3 Package Parser for SGW .upk/.umap files.
//!
//! Parses the binary format used by Stargate Worlds (Epic Version 486, Licensee 6-8).
//! Handles LZO-compressed packages (77% of SGW's 5,039 packages).
//!
//! # Example
//! ```no_run
//! use cimmeria_upk::Package;
//!
//! let pkg = Package::open("path/to/file.upk").unwrap();
//! println!("Names: {}, Imports: {}, Exports: {}",
//!     pkg.names.len(), pkg.imports.len(), pkg.exports.len());
//!
//! for export in &pkg.exports {
//!     let class = pkg.export_class_name(export);
//!     let path = pkg.export_full_path(export);
//!     println!("{}: {} ({}b)", class, path, export.serial_size);
//! }
//! ```

pub mod error;
pub mod exports;
pub mod header;
pub mod imports;
pub mod names;
pub mod objects;
pub mod package;
pub mod properties;
pub mod reader;

pub use error::{Result, UpkError};
pub use exports::ExportEntry;
pub use header::PackageHeader;
pub use imports::ImportEntry;
pub use names::NameEntry;
pub use objects::actor::{extract_actors, ActorData};
pub use package::Package;
pub use properties::{
    parse_tagged_properties, parse_tagged_properties_with_end, PropValue, TaggedProperty,
};
