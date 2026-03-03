// build.rs - Compile-time entity definition code generator
//
// Reads entities.xml and the corresponding .def files from the entities/defs/
// directory, then generates Rust structs and a type registry. This runs as a
// build script so the generated code is available via include! in lib.rs.
//
// Fault-tolerant: if entity files are missing or malformed, we generate empty
// stubs rather than failing the build.

use quick_xml::events::Event;
use quick_xml::Reader;
use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// A single property extracted from a .def file's <Properties> section.
#[derive(Debug, Clone)]
struct DefProperty {
    /// Property name (the XML tag name, e.g. "playerName").
    name: String,
    /// BigWorld type string (e.g. "INT32", "WSTRING", "VECTOR3").
    bw_type: String,
    /// Flags value (e.g. "BASE", "CELL_PUBLIC", "CELL_PRIVATE").
    flags: String,
}

/// Method signature extracted from a .def file.
#[derive(Debug, Clone)]
struct DefMethod {
    /// Method name (the XML tag name).
    name: String,
    /// Whether this method is exposed to the client.
    exposed: bool,
    /// Argument types (BigWorld type strings).
    arg_types: Vec<String>,
}

/// All data parsed from a single .def file.
#[derive(Debug)]
struct EntityDef {
    /// Entity type name (e.g. "SGWPlayer").
    name: String,
    /// Parent entity type, if any.
    parent: Option<String>,
    /// Interface names this entity implements.
    interfaces: Vec<String>,
    /// Properties declared in <Properties>.
    properties: Vec<DefProperty>,
    /// Client-callable methods.
    client_methods: Vec<DefMethod>,
    /// Cell-side methods.
    cell_methods: Vec<DefMethod>,
    /// Base-side methods.
    base_methods: Vec<DefMethod>,
}

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
                    println!(
                        "cargo:warning=Failed to parse {}.def: {}",
                        type_name, e
                    );
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

// ---------------------------------------------------------------------------
// entities.xml parser
// ---------------------------------------------------------------------------

/// Parse entities.xml and return the list of entity type names.
///
/// The file format is:
/// ```xml
/// <root>
///     <SGWPlayer/>
///     <Account/>
///     ...
/// </root>
/// ```
/// Each self-closing child tag of <root> is an entity type name.
fn parse_entities_xml(path: &Path) -> Vec<String> {
    let xml = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            println!(
                "cargo:warning=Could not read entities.xml ({}): {}. Generating empty registry.",
                path.display(),
                e
            );
            return Vec::new();
        }
    };

    let mut reader = Reader::from_str(&xml);
    reader.config_mut().trim_text(true);

    let mut names = Vec::new();
    let mut depth: u32 = 0;

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => {
                depth += 1;
                // Depth 1 = inside <root>. Self-closing tags at depth 1 are
                // entity types, but Start + End pairs at depth 1 could also be
                // entity types (the XML may use either style).
                if depth == 1 {
                    let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    if tag != "root" {
                        names.push(tag);
                    }
                }
            }
            Ok(Event::Empty(ref e)) => {
                // Self-closing tags inside <root>.
                if depth == 0 || (depth == 1 && String::from_utf8_lossy(e.name().as_ref()) != "root") {
                    // depth == 0 means we haven't seen <root> yet, which
                    // shouldn't happen, but handle gracefully. depth == 1
                    // means we're inside <root>.
                    if depth >= 1 {
                        let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                        names.push(tag);
                    }
                }
            }
            Ok(Event::End(_)) => {
                if depth > 0 {
                    depth -= 1;
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                println!(
                    "cargo:warning=XML parse error in entities.xml: {}. Returning partial list.",
                    e
                );
                break;
            }
            _ => {}
        }
    }

    names
}

// ---------------------------------------------------------------------------
// .def file parser
// ---------------------------------------------------------------------------

/// Parse a single .def file and return the extracted definition.
fn parse_def_file(path: &Path, type_name: &str) -> Result<EntityDef, String> {
    let xml = fs::read_to_string(path).map_err(|e| format!("read error: {}", e))?;

    let mut reader = Reader::from_str(&xml);
    reader.config_mut().trim_text(true);

    let mut def = EntityDef {
        name: type_name.to_string(),
        parent: None,
        interfaces: Vec::new(),
        properties: Vec::new(),
        client_methods: Vec::new(),
        cell_methods: Vec::new(),
        base_methods: Vec::new(),
    };

    // We use a simple state machine: track which top-level section we're in.
    #[derive(Debug, Clone, PartialEq)]
    enum Section {
        None,
        Properties,
        ClientMethods,
        CellMethods,
        BaseMethods,
        Implements,
        Other(String),
    }

    let mut section = Section::None;
    let mut depth: u32 = 0;

    // State for parsing a single <Property> or <Method> entry.
    let mut current_prop_name: Option<String> = None;
    let mut current_prop_type: Option<String> = None;
    let mut current_prop_flags: Option<String> = None;

    // For methods, we track similarly.
    let mut current_method_name: Option<String> = None;
    let mut current_method_exposed = false;
    let mut current_method_args: Vec<String> = Vec::new();

    // For the <Implements> section, we need to read <Interface> content.
    let mut in_interface = false;

    // For reading <Parent> content.
    let mut in_parent = false;

    // Track what inner element we're reading inside a property definition.
    #[derive(Debug, Clone, PartialEq)]
    enum PropField {
        None,
        Type,
        Flags,
        Other,
    }
    let mut prop_field = PropField::None;

    // Track what inner element we're reading inside a method definition.
    #[derive(Debug, Clone, PartialEq)]
    enum MethodField {
        None,
        Arg,
        Other,
    }
    let mut method_field = MethodField::None;

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => {
                depth += 1;
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();

                if depth == 1 && tag == "root" {
                    // Entering root, do nothing special.
                } else if depth == 2 {
                    // Top-level children of <root>.
                    match tag.as_str() {
                        "Properties" => section = Section::Properties,
                        "ClientMethods" => section = Section::ClientMethods,
                        "CellMethods" => section = Section::CellMethods,
                        "BaseMethods" => section = Section::BaseMethods,
                        "Implements" => section = Section::Implements,
                        "Parent" => in_parent = true,
                        _ => section = Section::Other(tag),
                    }
                } else if depth == 3 {
                    match &section {
                        Section::Properties => {
                            // This tag is a property name.
                            current_prop_name = Some(tag);
                            current_prop_type = None;
                            current_prop_flags = None;
                        }
                        Section::ClientMethods
                        | Section::CellMethods
                        | Section::BaseMethods => {
                            // This tag is a method name.
                            current_method_name = Some(tag);
                            current_method_exposed = false;
                            current_method_args = Vec::new();
                        }
                        Section::Implements => {
                            if tag == "Interface" {
                                in_interface = true;
                            }
                        }
                        _ => {}
                    }
                } else if depth > 3 {
                    // Inside a property or method definition.
                    if current_prop_name.is_some() {
                        match tag.as_str() {
                            "Type" => prop_field = PropField::Type,
                            "Flags" => prop_field = PropField::Flags,
                            _ => prop_field = PropField::Other,
                        }
                    }
                    if current_method_name.is_some() {
                        match tag.as_str() {
                            "Arg" => method_field = MethodField::Arg,
                            "Exposed" => current_method_exposed = true,
                            _ => method_field = MethodField::Other,
                        }
                    }
                }
            }
            Ok(Event::Empty(ref e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();

                if depth == 1 {
                    // Self-closing top-level sections (e.g. <ServerOnly/>).
                    // We ignore these for code generation.
                } else if depth == 2 {
                    // Self-closing element at top section level in root.
                    // e.g. <Volatile/> or <ServerOnly/>
                } else if depth >= 3 && current_method_name.is_some() {
                    match tag.as_str() {
                        "Exposed" => current_method_exposed = true,
                        _ => {}
                    }
                }
            }
            Ok(Event::Text(ref e)) => {
                let text = e.unescape().unwrap_or_default().trim().to_string();
                if text.is_empty() {
                    continue;
                }

                if in_parent {
                    def.parent = Some(text);
                } else if in_interface {
                    def.interfaces.push(text);
                } else if current_prop_name.is_some() {
                    match prop_field {
                        PropField::Type => {
                            current_prop_type = Some(text);
                        }
                        PropField::Flags => {
                            current_prop_flags = Some(text);
                        }
                        _ => {}
                    }
                } else if current_method_name.is_some() {
                    match method_field {
                        MethodField::Arg => {
                            // The text before <ArgName> is the type.
                            // We take the first whitespace-delimited word as the type.
                            let type_str = text
                                .split_whitespace()
                                .next()
                                .unwrap_or(&text)
                                .to_string();
                            if !type_str.is_empty() {
                                current_method_args.push(type_str);
                            }
                        }
                        _ => {}
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();

                if in_parent && tag == "Parent" {
                    in_parent = false;
                } else if in_interface && tag == "Interface" {
                    in_interface = false;
                } else if depth == 3 && current_prop_name.is_some() {
                    // Closing a property element.
                    if let Some(ref name) = current_prop_name {
                        let bw_type = current_prop_type
                            .take()
                            .unwrap_or_else(|| "PYTHON".to_string());
                        let flags = current_prop_flags
                            .take()
                            .unwrap_or_else(|| "CELL_PRIVATE".to_string());
                        def.properties.push(DefProperty {
                            name: name.clone(),
                            bw_type,
                            flags,
                        });
                    }
                    current_prop_name = None;
                    prop_field = PropField::None;
                } else if depth == 3 && current_method_name.is_some() {
                    // Closing a method element.
                    if let Some(ref name) = current_method_name {
                        let method = DefMethod {
                            name: name.clone(),
                            exposed: current_method_exposed,
                            arg_types: current_method_args.clone(),
                        };
                        match &section {
                            Section::ClientMethods => def.client_methods.push(method),
                            Section::CellMethods => def.cell_methods.push(method),
                            Section::BaseMethods => def.base_methods.push(method),
                            _ => {}
                        }
                    }
                    current_method_name = None;
                    current_method_args.clear();
                    current_method_exposed = false;
                    method_field = MethodField::None;
                } else if depth > 3 {
                    // Closing a sub-element inside a property/method.
                    prop_field = PropField::None;
                    method_field = MethodField::None;
                }
                if depth == 2 {
                    section = Section::None;
                }

                if depth > 0 {
                    depth -= 1;
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(format!("XML parse error: {}", e));
            }
            _ => {}
        }
    }

    Ok(def)
}

// ---------------------------------------------------------------------------
// Rust code generation
// ---------------------------------------------------------------------------

/// Map a BigWorld type string to a Rust type string.
fn bw_type_to_rust(bw_type: &str) -> String {
    // Normalize: trim whitespace, uppercase for comparison.
    let trimmed = bw_type.trim();
    let upper = trimmed.to_uppercase();

    match upper.as_str() {
        "STRING" => "String".to_string(),
        "WSTRING" => "String".to_string(),
        "INT8" => "i8".to_string(),
        "INT16" => "i16".to_string(),
        "INT32" => "i32".to_string(),
        "INT64" => "i64".to_string(),
        "UINT8" => "u8".to_string(),
        "UINT16" => "u16".to_string(),
        "UINT32" => "u32".to_string(),
        "UINT64" => "u64".to_string(),
        "FLOAT" => "f32".to_string(),
        "FLOAT32" => "f32".to_string(),
        "FLOAT64" => "f64".to_string(),
        "BOOL" => "bool".to_string(),
        "VECTOR3" => "cimmeria_common::math::Vector3".to_string(),
        // Types we map to opaque wrappers for now:
        "MAILBOX" => "()".to_string(),       // Placeholder: mailbox references
        "PYTHON" => "()".to_string(),         // Placeholder: Python object blobs
        "DBID" => "i64".to_string(),          // Database ID
        "CONTROLLER_ID" => "i32".to_string(), // Controller IDs are int32 under the hood
        _ => {
            // Handle ARRAY types and unknown/custom types.
            if upper.starts_with("ARRAY") {
                // For now, map all arrays to Vec<()> since inner types may be
                // complex (ARRAY <of> INT32 </of>, ARRAY <of> PYTHON </of>).
                "Vec<()>".to_string()
            } else {
                // Unknown or custom types (e.g. CharacterInfoList, VisualChoices)
                // get mapped to an opaque placeholder.
                format!("() /* {} */", trimmed)
            }
        }
    }
}

/// Map a BigWorld flags string to an enum variant name.
fn bw_flags_to_variant(flags: &str) -> &'static str {
    let upper = flags.trim().to_uppercase();
    match upper.as_str() {
        "BASE" => "Base",
        "CELL_PUBLIC" => "CellPublic",
        "CELL_PRIVATE" => "CellPrivate",
        "ALL_CLIENTS" => "AllClients",
        "OWN_CLIENT" => "OwnClient",
        "OTHER_CLIENTS" => "OtherClients",
        "CELL_PUBLIC_AND_OWN" => "CellPublicAndOwn",
        _ => "CellPrivate",
    }
}

/// Convert a BigWorld property name to a valid Rust identifier.
///
/// Property names in .def files may start with underscores and use camelCase.
/// We keep them as-is but ensure they are valid Rust identifiers.
fn sanitize_ident(name: &str) -> String {
    let mut result = String::with_capacity(name.len());
    for (i, ch) in name.chars().enumerate() {
        if i == 0 && ch.is_ascii_digit() {
            result.push('_');
        }
        if ch.is_ascii_alphanumeric() || ch == '_' {
            result.push(ch);
        } else {
            result.push('_');
        }
    }
    // Avoid Rust keywords.
    match result.as_str() {
        "type" => "r#type".to_string(),
        "move" => "r#move".to_string(),
        "self" => "r#self".to_string(),
        "super" => "r#super".to_string(),
        "crate" => "r#crate".to_string(),
        "mod" => "r#mod".to_string(),
        "fn" => "r#fn".to_string(),
        "let" => "r#let".to_string(),
        "mut" => "r#mut".to_string(),
        "ref" => "r#ref".to_string(),
        "match" => "r#match".to_string(),
        "loop" => "r#loop".to_string(),
        "return" => "r#return".to_string(),
        "where" => "r#where".to_string(),
        _ => result,
    }
}

/// Generate all Rust code for the parsed entity definitions.
fn generate_rust_code(defs: &[EntityDef]) -> String {
    let mut out = String::with_capacity(16 * 1024);

    // Header.
    out.push_str("// =============================================================================\n");
    out.push_str("// AUTO-GENERATED by cimmeria-defs/build.rs -- DO NOT EDIT\n");
    out.push_str("// =============================================================================\n");
    out.push_str("\n");

    // Property flags enum (shared across all entities).
    out.push_str("/// BigWorld property distribution flags.\n");
    out.push_str("#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]\n");
    out.push_str("pub enum PropertyFlags {\n");
    out.push_str("    Base,\n");
    out.push_str("    CellPublic,\n");
    out.push_str("    CellPrivate,\n");
    out.push_str("    AllClients,\n");
    out.push_str("    OwnClient,\n");
    out.push_str("    OtherClients,\n");
    out.push_str("    CellPublicAndOwn,\n");
    out.push_str("}\n\n");

    // PropertyInfo struct for runtime introspection.
    out.push_str("/// Runtime metadata for a single entity property.\n");
    out.push_str("#[derive(Debug, Clone)]\n");
    out.push_str("pub struct PropertyInfo {\n");
    out.push_str("    pub name: &'static str,\n");
    out.push_str("    pub bw_type: &'static str,\n");
    out.push_str("    pub flags: PropertyFlags,\n");
    out.push_str("}\n\n");

    // MethodInfo struct for runtime introspection.
    out.push_str("/// Runtime metadata for a single entity method.\n");
    out.push_str("#[derive(Debug, Clone)]\n");
    out.push_str("pub struct MethodInfo {\n");
    out.push_str("    pub name: &'static str,\n");
    out.push_str("    pub exposed: bool,\n");
    out.push_str("    pub arg_count: usize,\n");
    out.push_str("}\n\n");

    // EntityDef trait: every generated entity type implements this.
    out.push_str("/// Trait implemented by all generated entity definition structs.\n");
    out.push_str("pub trait EntityDefTrait {\n");
    out.push_str("    /// The BigWorld entity type name (e.g. \"SGWPlayer\").\n");
    out.push_str("    fn type_name() -> &'static str;\n");
    out.push_str("    /// Parent entity type name, if any.\n");
    out.push_str("    fn parent_type() -> Option<&'static str>;\n");
    out.push_str("    /// Interface names this entity implements.\n");
    out.push_str("    fn interfaces() -> &'static [&'static str];\n");
    out.push_str("    /// Property metadata for introspection.\n");
    out.push_str("    fn property_infos() -> &'static [PropertyInfo];\n");
    out.push_str("    /// Client method metadata.\n");
    out.push_str("    fn client_method_infos() -> &'static [MethodInfo];\n");
    out.push_str("    /// Cell method metadata.\n");
    out.push_str("    fn cell_method_infos() -> &'static [MethodInfo];\n");
    out.push_str("    /// Base method metadata.\n");
    out.push_str("    fn base_method_infos() -> &'static [MethodInfo];\n");
    out.push_str("}\n\n");

    // Generate a struct + impl for each entity.
    for def in defs {
        generate_entity_struct(&mut out, def);
    }

    // Generate the registry population function.
    generate_registry_data(&mut out, defs);

    out
}

/// Generate the struct and EntityDefTrait impl for a single entity definition.
fn generate_entity_struct(out: &mut String, def: &EntityDef) {
    let struct_name = format!("{}Props", def.name);

    // Doc comment.
    out.push_str(&format!(
        "/// Properties struct for the `{}` entity type.\n",
        def.name
    ));
    if let Some(ref parent) = def.parent {
        out.push_str(&format!("///\n/// Parent: `{}`\n", parent));
    }
    if !def.interfaces.is_empty() {
        out.push_str(&format!(
            "///\n/// Implements: {}\n",
            def.interfaces.join(", ")
        ));
    }

    // Struct definition.
    out.push_str("#[derive(Debug, Clone)]\n");
    out.push_str(&format!("pub struct {} {{\n", struct_name));

    for prop in &def.properties {
        let rust_type = bw_type_to_rust(&prop.bw_type);
        let ident = sanitize_ident(&prop.name);
        out.push_str(&format!(
            "    /// `{}` ({}, {})\n",
            prop.name, prop.bw_type, prop.flags
        ));
        out.push_str(&format!("    pub {}: {},\n", ident, rust_type));
    }

    // If there are no properties, add a marker field to avoid empty structs.
    if def.properties.is_empty() {
        out.push_str("    _marker: (),\n");
    }

    out.push_str("}\n\n");

    // Default impl.
    out.push_str(&format!("impl Default for {} {{\n", struct_name));
    out.push_str("    fn default() -> Self {\n");
    out.push_str("        Self {\n");
    for prop in &def.properties {
        let ident = sanitize_ident(&prop.name);
        let default = default_for_rust_type(&bw_type_to_rust(&prop.bw_type));
        out.push_str(&format!("            {}: {},\n", ident, default));
    }
    if def.properties.is_empty() {
        out.push_str("            _marker: (),\n");
    }
    out.push_str("        }\n");
    out.push_str("    }\n");
    out.push_str("}\n\n");

    // EntityDefTrait impl.
    out.push_str(&format!("impl EntityDefTrait for {} {{\n", struct_name));

    // type_name()
    out.push_str(&format!(
        "    fn type_name() -> &'static str {{ \"{}\" }}\n",
        def.name
    ));

    // parent_type()
    match &def.parent {
        Some(p) => out.push_str(&format!(
            "    fn parent_type() -> Option<&'static str> {{ Some(\"{}\") }}\n",
            p
        )),
        None => out.push_str(
            "    fn parent_type() -> Option<&'static str> { None }\n",
        ),
    }

    // interfaces()
    out.push_str("    fn interfaces() -> &'static [&'static str] {\n");
    out.push_str("        &[\n");
    for iface in &def.interfaces {
        out.push_str(&format!("            \"{}\",\n", iface));
    }
    out.push_str("        ]\n");
    out.push_str("    }\n");

    // property_infos()
    out.push_str("    fn property_infos() -> &'static [PropertyInfo] {\n");
    out.push_str("        &[\n");
    for prop in &def.properties {
        let variant = bw_flags_to_variant(&prop.flags);
        out.push_str(&format!(
            "            PropertyInfo {{ name: \"{}\", bw_type: \"{}\", flags: PropertyFlags::{} }},\n",
            prop.name, prop.bw_type, variant
        ));
    }
    out.push_str("        ]\n");
    out.push_str("    }\n");

    // client_method_infos()
    generate_method_infos(out, "client_method_infos", &def.client_methods);

    // cell_method_infos()
    generate_method_infos(out, "cell_method_infos", &def.cell_methods);

    // base_method_infos()
    generate_method_infos(out, "base_method_infos", &def.base_methods);

    out.push_str("}\n\n");
}

/// Generate a method info accessor function.
fn generate_method_infos(out: &mut String, fn_name: &str, methods: &[DefMethod]) {
    out.push_str(&format!(
        "    fn {}() -> &'static [MethodInfo] {{\n",
        fn_name
    ));
    out.push_str("        &[\n");
    for method in methods {
        out.push_str(&format!(
            "            MethodInfo {{ name: \"{}\", exposed: {}, arg_count: {} }},\n",
            method.name, method.exposed, method.arg_types.len()
        ));
    }
    out.push_str("        ]\n");
    out.push_str("    }\n");
}

/// Return a Rust default-value expression for the given Rust type string.
fn default_for_rust_type(rust_type: &str) -> &str {
    match rust_type {
        "String" => "String::new()",
        "i8" => "0",
        "i16" => "0",
        "i32" => "0",
        "i64" => "0",
        "u8" => "0",
        "u16" => "0",
        "u32" => "0",
        "u64" => "0",
        "f32" => "0.0",
        "f64" => "0.0",
        "bool" => "false",
        "cimmeria_common::math::Vector3" => "cimmeria_common::math::Vector3::zero()",
        "()" => "()",
        _ => {
            if rust_type.starts_with("Vec<") {
                "Vec::new()"
            } else {
                "()"
            }
        }
    }
}

/// Generate the `ENTITY_TYPE_DATA` array and `entity_type_count()` function
/// used by the runtime registry.
fn generate_registry_data(out: &mut String, defs: &[EntityDef]) {
    out.push_str("/// Static entity type data for populating the runtime registry.\n");
    out.push_str("///\n");
    out.push_str("/// Each tuple is: (name, type_id, property_count)\n");
    out.push_str("pub static ENTITY_TYPE_DATA: &[(&str, u16, usize)] = &[\n");

    for (i, def) in defs.iter().enumerate() {
        out.push_str(&format!(
            "    (\"{}\", {}, {}),\n",
            def.name,
            i as u16,
            def.properties.len()
        ));
    }

    out.push_str("];\n\n");

    out.push_str("/// Number of entity types that were successfully parsed.\n");
    out.push_str(&format!(
        "pub fn entity_type_count() -> usize {{ {} }}\n",
        defs.len()
    ));
}
