// build/main.rs — Compile-time entity definition code generator.
//
// Reads entities.xml and the corresponding .def files from the entities/defs/
// directory, then generates Rust structs and a type registry. This runs as a
// build script so the generated code is available via include! in lib.rs.
//
// Fault-tolerant: if entity files are missing or malformed, we generate empty
// stubs rather than failing the build.

use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

mod codegen;
mod def_parser;
mod entities_xml;
mod types;

use codegen::generate_rust_code;
use def_parser::parse_def_file;
use entities_xml::parse_entities_xml;
use types::EntityDef;

fn main() {
    let manifest_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set"));
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set"));

    // Entity files live two directories above the crate root.
    let entities_root = manifest_dir.join("..").join("..").join("entities");
    let entities_xml = entities_root.join("entities.xml");
    let defs_dir = entities_root.join("defs");

    // Tell Cargo to rerun if the entity registry changes.
    println!("cargo:rerun-if-changed={}", entities_xml.display());

    // Parse the list of entity type names from entities.xml.
    let type_names = parse_entities_xml(&entities_xml);

    // For each entity type, attempt to parse its .def file.
    let mut entity_defs: Vec<EntityDef> = Vec::new();
    for type_name in &type_names {
        let def_path = defs_dir.join(format!("{}.def", type_name));
        println!("cargo:rerun-if-changed={}", def_path.display());

        if def_path.exists() {
            match parse_def_file(&def_path, type_name) {
                Ok(def) => entity_defs.push(def),
                Err(e) => {
                    println!("cargo:warning=Failed to parse {}.def: {}", type_name, e);
                }
            }
        } else {
            println!(
                "cargo:warning=Entity def file not found: {}",
                def_path.display()
            );
        }
    }

    // Generate the Rust source.
    let generated = generate_rust_code(&entity_defs);

    let output_path = out_dir.join("generated_entities.rs");
    let mut file = fs::File::create(&output_path).expect("Failed to create generated_entities.rs");
    file.write_all(generated.as_bytes())
        .expect("Failed to write generated_entities.rs");
}
