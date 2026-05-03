//! Parser for individual .def XML files.

use quick_xml::events::Event;
use quick_xml::Reader;
use std::fs;
use std::path::Path;

use super::types::{DefMethod, DefProperty, EntityDef};

/// Parse a single .def file and return the extracted definition.
pub fn parse_def_file(path: &Path, type_name: &str) -> Result<EntityDef, String> {
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
                        Section::ClientMethods | Section::CellMethods | Section::BaseMethods => {
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
                let text = e
                    .unescape()
                    .map_err(|err| format!("XML unescape error: {}", err))?
                    .trim()
                    .to_string();
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
                            let type_str =
                                text.split_whitespace().next().unwrap_or(&text).to_string();
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
