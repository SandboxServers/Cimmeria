use anyhow::{bail, Context, Result};
use quick_xml::events::Event;
use quick_xml::Reader;
use serde::{Deserialize, Serialize};
use std::path::Path;

// -- Node template types parsed from Nodes.xml --

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeTemplate {
    pub ref_name: String,
    pub display_name: String,
    pub node_type: String,
    pub category: String,
    pub description: String,
    pub ports: Vec<PortTemplate>,
    pub properties: Vec<PropertyTemplate>,
    pub script_types: Vec<String>,
    pub imports: Vec<String>,
    pub static_init_script: Option<String>,
    pub static_teardown_script: Option<String>,
    pub pre_init_script: Option<String>,
    pub init_script: Option<String>,
    pub teardown_script: Option<String>,
    pub persist_script: Option<String>,
    pub restore_script: Option<String>,
    pub methods: Vec<MethodTemplate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortTemplate {
    pub name: String,
    pub port_type: String,
    pub direction: String,
    pub default_hide: bool,
    pub required: bool,
    /// Inline trigger script -- only meaningful for Direction=in + Type=Trigger ports
    pub script: Option<String>,
}

impl PortTemplate {
    pub fn is_trigger(&self) -> bool {
        self.port_type == "Trigger"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyTemplate {
    pub name: String,
    pub prop_type: String,
    pub default_value: String,
    pub database_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodTemplate {
    pub name: String,
    pub script: String,
}

// -- Database reference types from Nodes.xml --

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseRef {
    pub ref_name: String,
    pub null_value: String,
    pub find_by_name: String,
    pub find_by_id: String,
}

// -- Enum types from enumerations.xml --

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumDefinition {
    pub name: String,
    pub data_type: String,
    pub is_bitfield: bool,
    pub tokens: Vec<EnumToken>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumToken {
    pub name: String,
    pub value: String,
}

// -- Top-level container --

#[derive(Debug, Clone)]
pub struct ScriptDefinitions {
    pub nodes: Vec<NodeTemplate>,
    #[allow(dead_code)] // Exposed to frontend via DTOs, not used directly in Rust
    pub database_refs: Vec<DatabaseRef>,
    pub enums: Vec<EnumDefinition>,
    pub dataset_version: String,
}

/// Load all script definitions from the repo root.
/// Parses `entities/editor/Nodes.xml` and `entities/editor/enumerations.xml`.
pub fn load_definitions(repo_root: &Path) -> Result<ScriptDefinitions> {
    let nodes_path = repo_root.join("entities/editor/Nodes.xml");
    let enums_path = repo_root.join("entities/editor/enumerations.xml");

    let (nodes, database_refs, dataset_version) =
        parse_nodes_xml(&nodes_path).with_context(|| format!("parsing {}", nodes_path.display()))?;
    let enums =
        parse_enumerations_xml(&enums_path).with_context(|| format!("parsing {}", enums_path.display()))?;

    Ok(ScriptDefinitions {
        nodes,
        database_refs,
        enums,
        dataset_version,
    })
}

// ---------------------------------------------------------------------------
// Nodes.xml parser
// ---------------------------------------------------------------------------

fn parse_nodes_xml(path: &Path) -> Result<(Vec<NodeTemplate>, Vec<DatabaseRef>, String)> {
    let xml = std::fs::read_to_string(path)?;
    let mut reader = Reader::from_str(&xml);

    let mut nodes = Vec::new();
    let mut db_refs = Vec::new();
    let mut dataset_version = String::from("1.2");

    let mut buf = Vec::new();

    enum State {
        Root,
        InDatabaseRef(DatabaseRef),
        InNode(NodeTemplate),
        InNodePort(NodeTemplate, PortTemplate),
        Skip,
    }

    let mut state = State::Root;
    // Track which child element we're inside for text capture
    let mut current_child_tag: Option<String> = None;
    let mut text_buf = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let tag = std::str::from_utf8(e.name().as_ref())?.to_string();
                match &mut state {
                    State::Root => match tag.as_str() {
                        "Nodes" => {
                            for attr in e.attributes().flatten() {
                                if attr.key.as_ref() == b"Version" {
                                    dataset_version =
                                        String::from_utf8(attr.value.to_vec())?;
                                }
                            }
                        }
                        "DatabaseRef" => {
                            let mut db_ref = DatabaseRef {
                                ref_name: String::new(),
                                null_value: String::new(),
                                find_by_name: String::new(),
                                find_by_id: String::new(),
                            };
                            for attr in e.attributes().flatten() {
                                match attr.key.as_ref() {
                                    b"Ref" => {
                                        db_ref.ref_name =
                                            String::from_utf8(attr.value.to_vec())?;
                                    }
                                    b"NullValue" => {
                                        db_ref.null_value =
                                            String::from_utf8(attr.value.to_vec())?;
                                    }
                                    _ => {}
                                }
                            }
                            state = State::InDatabaseRef(db_ref);
                        }
                        "Node" => {
                            let mut node = NodeTemplate {
                                ref_name: String::new(),
                                display_name: String::new(),
                                node_type: String::new(),
                                category: String::new(),
                                description: String::new(),
                                ports: Vec::new(),
                                properties: Vec::new(),
                                script_types: Vec::new(),
                                imports: Vec::new(),
                                static_init_script: None,
                                static_teardown_script: None,
                                pre_init_script: None,
                                init_script: None,
                                teardown_script: None,
                                persist_script: None,
                                restore_script: None,
                                methods: Vec::new(),
                            };
                            for attr in e.attributes().flatten() {
                                match attr.key.as_ref() {
                                    b"Ref" => {
                                        node.ref_name =
                                            String::from_utf8(attr.value.to_vec())?;
                                    }
                                    b"Name" => {
                                        node.display_name =
                                            String::from_utf8(attr.value.to_vec())?;
                                    }
                                    b"Type" => {
                                        node.node_type =
                                            String::from_utf8(attr.value.to_vec())?;
                                    }
                                    b"Category" => {
                                        node.category =
                                            String::from_utf8(attr.value.to_vec())?;
                                    }
                                    _ => {}
                                }
                            }
                            state = State::InNode(node);
                        }
                        _ => {}
                    },
                    State::InDatabaseRef(_) => {
                        current_child_tag = Some(tag);
                        text_buf.clear();
                    }
                    State::InNode(_) => {
                        if tag == "Port" {
                            let mut port = PortTemplate {
                                name: String::new(),
                                port_type: String::new(),
                                direction: String::new(),
                                default_hide: false,
                                required: false,
                                script: None,
                            };
                            for attr in e.attributes().flatten() {
                                match attr.key.as_ref() {
                                    b"Name" => {
                                        port.name =
                                            String::from_utf8(attr.value.to_vec())?;
                                    }
                                    b"Type" => {
                                        port.port_type =
                                            String::from_utf8(attr.value.to_vec())?;
                                    }
                                    b"Direction" => {
                                        port.direction =
                                            String::from_utf8(attr.value.to_vec())?;
                                    }
                                    b"DefaultHide" => {
                                        port.default_hide =
                                            attr.value.as_ref() == b"true";
                                    }
                                    b"Required" => {
                                        port.required =
                                            attr.value.as_ref() == b"true";
                                    }
                                    _ => {}
                                }
                            }
                            // Port has a Start tag, so it may contain inline script text.
                            // We'll transition to InNodePort to capture that.
                            let node =
                                if let State::InNode(n) = std::mem::replace(&mut state, State::Skip)
                                {
                                    n
                                } else {
                                    unreachable!()
                                };
                            state = State::InNodePort(node, port);
                            text_buf.clear();
                        } else if tag == "Method" {
                            let mut method_name = String::new();
                            for attr in e.attributes().flatten() {
                                if attr.key.as_ref() == b"Name" {
                                    method_name =
                                        String::from_utf8(attr.value.to_vec())?;
                                }
                            }
                            current_child_tag = Some(format!("Method:{}", method_name));
                            text_buf.clear();
                        } else {
                            current_child_tag = Some(tag);
                            text_buf.clear();
                        }
                    }
                    State::InNodePort(_, _) => {
                        // Shouldn't have child elements inside a Port, but ignore gracefully
                    }
                    State::Skip => {}
                }
            }
            Ok(Event::Empty(ref e)) => {
                let tag = std::str::from_utf8(e.name().as_ref())?.to_string();
                match &mut state {
                    State::InNode(ref mut node) => {
                        if tag == "Port" {
                            let mut port = PortTemplate {
                                name: String::new(),
                                port_type: String::new(),
                                direction: String::new(),
                                default_hide: false,
                                required: false,
                                script: None,
                            };
                            for attr in e.attributes().flatten() {
                                match attr.key.as_ref() {
                                    b"Name" => {
                                        port.name =
                                            String::from_utf8(attr.value.to_vec())?;
                                    }
                                    b"Type" => {
                                        port.port_type =
                                            String::from_utf8(attr.value.to_vec())?;
                                    }
                                    b"Direction" => {
                                        port.direction =
                                            String::from_utf8(attr.value.to_vec())?;
                                    }
                                    b"DefaultHide" => {
                                        port.default_hide =
                                            attr.value.as_ref() == b"true";
                                    }
                                    b"Required" => {
                                        port.required =
                                            attr.value.as_ref() == b"true";
                                    }
                                    _ => {}
                                }
                            }
                            node.ports.push(port);
                        } else if tag == "Property" {
                            let mut prop = PropertyTemplate {
                                name: String::new(),
                                prop_type: String::new(),
                                default_value: String::new(),
                                database_ref: None,
                            };
                            for attr in e.attributes().flatten() {
                                match attr.key.as_ref() {
                                    b"Name" => {
                                        prop.name =
                                            String::from_utf8(attr.value.to_vec())?;
                                    }
                                    b"Type" => {
                                        prop.prop_type =
                                            String::from_utf8(attr.value.to_vec())?;
                                    }
                                    b"DefaultValue" => {
                                        prop.default_value =
                                            String::from_utf8(attr.value.to_vec())?;
                                    }
                                    b"DatabaseRef" => {
                                        prop.database_ref = Some(
                                            String::from_utf8(attr.value.to_vec())?,
                                        );
                                    }
                                    _ => {}
                                }
                            }
                            node.properties.push(prop);
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(ref e)) => {
                let text = e.unescape()?.to_string();
                match &mut state {
                    State::InDatabaseRef(_) | State::InNode(_) => {
                        text_buf.push_str(&text);
                    }
                    State::InNodePort(_, _) => {
                        text_buf.push_str(&text);
                    }
                    _ => {}
                }
            }
            Ok(Event::CData(ref e)) => {
                let text = String::from_utf8(e.to_vec())?;
                text_buf.push_str(&text);
            }
            Ok(Event::End(ref e)) => {
                let tag = std::str::from_utf8(e.name().as_ref())?.to_string();
                match &mut state {
                    State::InDatabaseRef(ref mut db_ref) => {
                        if tag == "DatabaseRef" {
                            let finished = std::mem::replace(
                                db_ref,
                                DatabaseRef {
                                    ref_name: String::new(),
                                    null_value: String::new(),
                                    find_by_name: String::new(),
                                    find_by_id: String::new(),
                                },
                            );
                            db_refs.push(finished);
                            state = State::Root;
                        } else {
                            let trimmed = text_buf.trim().to_string();
                            match tag.as_str() {
                                "FindByName" => db_ref.find_by_name = trimmed,
                                "FindById" => db_ref.find_by_id = trimmed,
                                _ => {}
                            }
                            current_child_tag = None;
                            text_buf.clear();
                        }
                    }
                    State::InNodePort(ref mut _node, ref mut _port) => {
                        if tag == "Port" {
                            let script_text = text_buf.trim().to_string();
                            let (mut node, mut port) = if let State::InNodePort(n, p) =
                                std::mem::replace(&mut state, State::Skip)
                            {
                                (n, p)
                            } else {
                                unreachable!()
                            };
                            if !script_text.is_empty() {
                                port.script = Some(script_text);
                            }
                            node.ports.push(port);
                            state = State::InNode(node);
                            text_buf.clear();
                        }
                    }
                    State::InNode(ref mut node) => {
                        if tag == "Node" {
                            let finished = std::mem::replace(
                                node,
                                NodeTemplate {
                                    ref_name: String::new(),
                                    display_name: String::new(),
                                    node_type: String::new(),
                                    category: String::new(),
                                    description: String::new(),
                                    ports: Vec::new(),
                                    properties: Vec::new(),
                                    script_types: Vec::new(),
                                    imports: Vec::new(),
                                    static_init_script: None,
                                    static_teardown_script: None,
                                    pre_init_script: None,
                                    init_script: None,
                                    teardown_script: None,
                                    persist_script: None,
                                    restore_script: None,
                                    methods: Vec::new(),
                                },
                            );
                            nodes.push(finished);
                            state = State::Root;
                        } else {
                            let trimmed = text_buf.trim().to_string();
                            if let Some(ref child_tag) = current_child_tag {
                                if child_tag.starts_with("Method:") {
                                    let method_name = child_tag["Method:".len()..].to_string();
                                    if !trimmed.is_empty() {
                                        node.methods.push(MethodTemplate {
                                            name: method_name,
                                            script: trimmed,
                                        });
                                    }
                                } else {
                                    match child_tag.as_str() {
                                        "Description" => node.description = trimmed,
                                        "StaticInitScript" => {
                                            if !trimmed.is_empty() {
                                                node.static_init_script = Some(trimmed);
                                            }
                                        }
                                        "StaticTeardownScript" => {
                                            if !trimmed.is_empty() {
                                                node.static_teardown_script = Some(trimmed);
                                            }
                                        }
                                        "PreInitScript" => {
                                            if !trimmed.is_empty() {
                                                node.pre_init_script = Some(trimmed);
                                            }
                                        }
                                        "InitScript" => {
                                            if !trimmed.is_empty() {
                                                node.init_script = Some(trimmed);
                                            }
                                        }
                                        "TeardownScript" => {
                                            if !trimmed.is_empty() {
                                                node.teardown_script = Some(trimmed);
                                            }
                                        }
                                        "PersistScript" => {
                                            if !trimmed.is_empty() {
                                                node.persist_script = Some(trimmed);
                                            }
                                        }
                                        "RestoreScript" => {
                                            if !trimmed.is_empty() {
                                                node.restore_script = Some(trimmed);
                                            }
                                        }
                                        "Import" => {
                                            if !trimmed.is_empty() {
                                                node.imports.push(trimmed);
                                            }
                                        }
                                        "ScriptType" => {
                                            if !trimmed.is_empty() {
                                                node.script_types.push(trimmed);
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            current_child_tag = None;
                            text_buf.clear();
                        }
                    }
                    State::Skip => {
                        // We shouldn't normally be here, but handle gracefully
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => bail!("XML parse error at position {}: {}", reader.error_position(), e),
            _ => {}
        }
        buf.clear();
    }

    Ok((nodes, db_refs, dataset_version))
}

// ---------------------------------------------------------------------------
// enumerations.xml parser
// ---------------------------------------------------------------------------

fn parse_enumerations_xml(path: &Path) -> Result<Vec<EnumDefinition>> {
    let xml = std::fs::read_to_string(path)?;
    let mut reader = Reader::from_str(&xml);

    let mut enums = Vec::new();
    let mut buf = Vec::new();

    // States for parsing the nested structure.
    // The tricky part: the enum name is the XML tag name itself (e.g., <ELocales>).
    enum State {
        Root,
        InEnum(EnumDefinition),
        InTokens(EnumDefinition),
        InToken(EnumDefinition, EnumToken),
    }
    let mut state = State::Root;
    let mut current_child_tag: Option<String> = None;
    let mut text_buf = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let tag = std::str::from_utf8(e.name().as_ref())?.to_string();
                match &mut state {
                    State::Root => {
                        if tag != "root" {
                            // This is an enum definition element
                            let mut is_bitfield = false;
                            for attr in e.attributes().flatten() {
                                if attr.key.as_ref() == b"IsBitfield"
                                    && attr.value.as_ref() == b"true"
                                {
                                    is_bitfield = true;
                                }
                            }
                            state = State::InEnum(EnumDefinition {
                                name: tag,
                                data_type: String::new(),
                                is_bitfield,
                                tokens: Vec::new(),
                            });
                            text_buf.clear();
                        }
                    }
                    State::InEnum(_) => {
                        if tag == "Tokens" {
                            let def = if let State::InEnum(d) =
                                std::mem::replace(&mut state, State::Root)
                            {
                                d
                            } else {
                                unreachable!()
                            };
                            state = State::InTokens(def);
                        } else {
                            current_child_tag = Some(tag);
                            text_buf.clear();
                        }
                    }
                    State::InTokens(_) => {
                        if tag == "Token" {
                            let def = if let State::InTokens(d) =
                                std::mem::replace(&mut state, State::Root)
                            {
                                d
                            } else {
                                unreachable!()
                            };
                            state = State::InToken(
                                def,
                                EnumToken {
                                    name: String::new(),
                                    value: String::new(),
                                },
                            );
                        }
                    }
                    State::InToken(_, _) => {
                        current_child_tag = Some(tag);
                        text_buf.clear();
                    }
                }
            }
            Ok(Event::Empty(ref e)) => {
                let tag = std::str::from_utf8(e.name().as_ref())?.to_string();
                // Some child elements like <BW_TO_UE3_DIST_CONVERT/> are empty -- ignore them
                match &mut state {
                    State::InEnum(_) => {
                        // Ignore empty child elements (e.g. <BW_TO_UE3_DIST_CONVERT/>)
                    }
                    _ => {}
                }
                let _ = tag;
            }
            Ok(Event::Text(ref e)) => {
                let text = e.unescape()?.to_string();
                text_buf.push_str(&text);
            }
            Ok(Event::End(ref e)) => {
                let tag = std::str::from_utf8(e.name().as_ref())?.to_string();
                match &mut state {
                    State::InEnum(ref mut def) => {
                        if tag == def.name {
                            // End of this enum definition
                            let finished = std::mem::replace(
                                def,
                                EnumDefinition {
                                    name: String::new(),
                                    data_type: String::new(),
                                    is_bitfield: false,
                                    tokens: Vec::new(),
                                },
                            );
                            enums.push(finished);
                            state = State::Root;
                        } else {
                            let trimmed = text_buf.trim().to_string();
                            if let Some(ref child) = current_child_tag {
                                if child == "Type" {
                                    def.data_type = trimmed;
                                }
                            }
                            current_child_tag = None;
                            text_buf.clear();
                        }
                    }
                    State::InTokens(ref mut _def) => {
                        if tag == "Tokens" {
                            let def = if let State::InTokens(d) =
                                std::mem::replace(&mut state, State::Root)
                            {
                                d
                            } else {
                                unreachable!()
                            };
                            state = State::InEnum(def);
                        }
                    }
                    State::InToken(ref mut _def, ref mut _token) => {
                        if tag == "Token" {
                            let (def, token) = if let State::InToken(d, t) =
                                std::mem::replace(&mut state, State::Root)
                            {
                                (d, t)
                            } else {
                                unreachable!()
                            };
                            let mut def = def;
                            def.tokens.push(token);
                            state = State::InTokens(def);
                        } else {
                            let trimmed = text_buf.trim().to_string();
                            if let Some(ref child) = current_child_tag {
                                if let State::InToken(_, ref mut token) = state {
                                    match child.as_str() {
                                        "Name" => token.name = trimmed,
                                        "Value" => token.value = trimmed,
                                        _ => {}
                                    }
                                }
                            }
                            current_child_tag = None;
                            text_buf.clear();
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => bail!("XML parse error at position {}: {}", reader.error_position(), e),
            _ => {}
        }
        buf.clear();
    }

    Ok(enums)
}
