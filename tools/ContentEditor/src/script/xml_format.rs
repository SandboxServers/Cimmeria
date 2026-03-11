use anyhow::{bail, Context, Result};
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, Event};
use quick_xml::{Reader, Writer};
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use std::path::Path;

/// A full .script file: graph nodes, connections, and comments.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptFile {
    pub version: String,
    pub dataset_version: String,
    pub module: String,
    pub script_type: String,
    pub next_id: u32,
    pub nodes: Vec<ScriptNode>,
    pub connections: Vec<ScriptConnection>,
    pub comments: Vec<ScriptComment>,
}

/// A node placed in the visual graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptNode {
    pub id: u32,
    pub ref_name: String,
    pub x: i32,
    pub y: i32,
    pub properties: Vec<(String, String)>,
    pub ports: Vec<(String, u32)>,
}

/// A connection between an output port on one node and an input port on another.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptConnection {
    pub out_node: u32,
    pub out_port: String,
    pub in_node: u32,
    pub in_port: String,
}

/// A visual comment box in the graph editor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptComment {
    pub id: u32,
    pub text: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub color: u32,
}

// ---------------------------------------------------------------------------
// Load
// ---------------------------------------------------------------------------

pub fn load_script(path: &Path) -> Result<ScriptFile> {
    let xml = std::fs::read_to_string(path)
        .with_context(|| format!("reading script file {}", path.display()))?;
    let mut reader = Reader::from_str(&xml);

    let mut script = ScriptFile {
        version: String::new(),
        dataset_version: String::new(),
        module: String::new(),
        script_type: String::new(),
        next_id: 0,
        nodes: Vec::new(),
        connections: Vec::new(),
        comments: Vec::new(),
    };

    let mut buf = Vec::new();
    let mut current_node: Option<ScriptNode> = None;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                let tag = std::str::from_utf8(e.name().as_ref())?.to_string();

                match tag.as_str() {
                    "Script" => {
                        for attr in e.attributes().flatten() {
                            match attr.key.as_ref() {
                                b"Version" => {
                                    script.version =
                                        String::from_utf8(attr.value.to_vec())?;
                                }
                                b"DatasetVersion" => {
                                    script.dataset_version =
                                        String::from_utf8(attr.value.to_vec())?;
                                }
                                b"Module" => {
                                    script.module =
                                        String::from_utf8(attr.value.to_vec())?;
                                }
                                b"Type" => {
                                    script.script_type =
                                        String::from_utf8(attr.value.to_vec())?;
                                }
                                b"NextId" => {
                                    let s = String::from_utf8(attr.value.to_vec())?;
                                    script.next_id = s.parse().unwrap_or(0);
                                }
                                _ => {}
                            }
                        }
                    }
                    "Node" => {
                        let mut node = ScriptNode {
                            id: 0,
                            ref_name: String::new(),
                            x: 0,
                            y: 0,
                            properties: Vec::new(),
                            ports: Vec::new(),
                        };
                        for attr in e.attributes().flatten() {
                            match attr.key.as_ref() {
                                b"Id" => {
                                    let s = String::from_utf8(attr.value.to_vec())?;
                                    node.id = s.parse().unwrap_or(0);
                                }
                                b"Ref" => {
                                    node.ref_name =
                                        String::from_utf8(attr.value.to_vec())?;
                                }
                                b"X" => {
                                    let s = String::from_utf8(attr.value.to_vec())?;
                                    node.x = s.parse().unwrap_or(0);
                                }
                                b"Y" => {
                                    let s = String::from_utf8(attr.value.to_vec())?;
                                    node.y = s.parse().unwrap_or(0);
                                }
                                _ => {}
                            }
                        }
                        current_node = Some(node);
                    }
                    "Property" => {
                        if let Some(ref mut node) = current_node {
                            let mut name = String::new();
                            let mut value = String::new();
                            for attr in e.attributes().flatten() {
                                match attr.key.as_ref() {
                                    b"Name" => {
                                        name = String::from_utf8(attr.value.to_vec())?;
                                    }
                                    b"Value" => {
                                        value = String::from_utf8(attr.value.to_vec())?;
                                    }
                                    _ => {}
                                }
                            }
                            node.properties.push((name, value));
                        }
                    }
                    "Port" => {
                        if let Some(ref mut node) = current_node {
                            let mut name = String::new();
                            let mut flags: u32 = 0;
                            for attr in e.attributes().flatten() {
                                match attr.key.as_ref() {
                                    b"Name" => {
                                        name = String::from_utf8(attr.value.to_vec())?;
                                    }
                                    b"Flags" => {
                                        let s =
                                            String::from_utf8(attr.value.to_vec())?;
                                        flags = s.parse().unwrap_or(0);
                                    }
                                    _ => {}
                                }
                            }
                            node.ports.push((name, flags));
                        }
                    }
                    "Connection" => {
                        let mut conn = ScriptConnection {
                            out_node: 0,
                            out_port: String::new(),
                            in_node: 0,
                            in_port: String::new(),
                        };
                        for attr in e.attributes().flatten() {
                            match attr.key.as_ref() {
                                b"OutNode" => {
                                    let s = String::from_utf8(attr.value.to_vec())?;
                                    conn.out_node = s.parse().unwrap_or(0);
                                }
                                b"OutPort" => {
                                    conn.out_port =
                                        String::from_utf8(attr.value.to_vec())?;
                                }
                                b"InNode" => {
                                    let s = String::from_utf8(attr.value.to_vec())?;
                                    conn.in_node = s.parse().unwrap_or(0);
                                }
                                b"InPort" => {
                                    conn.in_port =
                                        String::from_utf8(attr.value.to_vec())?;
                                }
                                _ => {}
                            }
                        }
                        script.connections.push(conn);
                    }
                    "Comment" => {
                        let mut comment = ScriptComment {
                            id: 0,
                            text: String::new(),
                            x: 0,
                            y: 0,
                            width: 0,
                            height: 0,
                            color: 0,
                        };
                        for attr in e.attributes().flatten() {
                            match attr.key.as_ref() {
                                b"Id" => {
                                    let s = String::from_utf8(attr.value.to_vec())?;
                                    comment.id = s.parse().unwrap_or(0);
                                }
                                b"Text" => {
                                    comment.text =
                                        String::from_utf8(attr.value.to_vec())?;
                                }
                                b"X" => {
                                    let s = String::from_utf8(attr.value.to_vec())?;
                                    comment.x = s.parse().unwrap_or(0);
                                }
                                b"Y" => {
                                    let s = String::from_utf8(attr.value.to_vec())?;
                                    comment.y = s.parse().unwrap_or(0);
                                }
                                b"Width" => {
                                    let s = String::from_utf8(attr.value.to_vec())?;
                                    comment.width = s.parse().unwrap_or(0);
                                }
                                b"Height" => {
                                    let s = String::from_utf8(attr.value.to_vec())?;
                                    comment.height = s.parse().unwrap_or(0);
                                }
                                b"Color" => {
                                    let s = String::from_utf8(attr.value.to_vec())?;
                                    comment.color = s.parse().unwrap_or(0);
                                }
                                _ => {}
                            }
                        }
                        script.comments.push(comment);
                    }
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => {
                let name = e.name();
                let tag = std::str::from_utf8(name.as_ref())?;
                if tag == "Node" {
                    if let Some(node) = current_node.take() {
                        script.nodes.push(node);
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => bail!("XML parse error at position {}: {}", reader.error_position(), e),
            _ => {}
        }
        buf.clear();
    }

    Ok(script)
}

// ---------------------------------------------------------------------------
// Save
// ---------------------------------------------------------------------------

pub fn save_script(path: &Path, script: &ScriptFile) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut writer = Writer::new_with_indent(Cursor::new(Vec::new()), b'\t', 1);

    // XML declaration
    writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))?;

    // <Script ...>
    let mut script_elem = BytesStart::new("Script");
    script_elem.push_attribute(("Version", script.version.as_str()));
    script_elem.push_attribute(("DatasetVersion", script.dataset_version.as_str()));
    script_elem.push_attribute(("Module", script.module.as_str()));
    script_elem.push_attribute(("Type", script.script_type.as_str()));
    script_elem.push_attribute(("NextId", script.next_id.to_string().as_str()));
    writer.write_event(Event::Start(script_elem))?;

    // Nodes
    for node in &script.nodes {
        let mut node_elem = BytesStart::new("Node");
        node_elem.push_attribute(("Id", node.id.to_string().as_str()));
        node_elem.push_attribute(("Ref", node.ref_name.as_str()));
        node_elem.push_attribute(("X", node.x.to_string().as_str()));
        node_elem.push_attribute(("Y", node.y.to_string().as_str()));
        writer.write_event(Event::Start(node_elem))?;

        for (name, value) in &node.properties {
            let mut prop_elem = BytesStart::new("Property");
            prop_elem.push_attribute(("Name", name.as_str()));
            prop_elem.push_attribute(("Value", value.as_str()));
            writer.write_event(Event::Empty(prop_elem))?;
        }

        for (name, flags) in &node.ports {
            let mut port_elem = BytesStart::new("Port");
            port_elem.push_attribute(("Name", name.as_str()));
            port_elem.push_attribute(("Flags", flags.to_string().as_str()));
            writer.write_event(Event::Empty(port_elem))?;
        }

        writer.write_event(Event::End(BytesEnd::new("Node")))?;
    }

    // Connections
    for conn in &script.connections {
        let mut conn_elem = BytesStart::new("Connection");
        conn_elem.push_attribute(("OutNode", conn.out_node.to_string().as_str()));
        conn_elem.push_attribute(("OutPort", conn.out_port.as_str()));
        conn_elem.push_attribute(("InNode", conn.in_node.to_string().as_str()));
        conn_elem.push_attribute(("InPort", conn.in_port.as_str()));
        writer.write_event(Event::Empty(conn_elem))?;
    }

    // Comments
    for comment in &script.comments {
        let mut comment_elem = BytesStart::new("Comment");
        comment_elem.push_attribute(("Id", comment.id.to_string().as_str()));
        comment_elem.push_attribute(("Text", comment.text.as_str()));
        comment_elem.push_attribute(("X", comment.x.to_string().as_str()));
        comment_elem.push_attribute(("Y", comment.y.to_string().as_str()));
        comment_elem.push_attribute(("Width", comment.width.to_string().as_str()));
        comment_elem.push_attribute(("Height", comment.height.to_string().as_str()));
        comment_elem.push_attribute(("Color", comment.color.to_string().as_str()));
        writer.write_event(Event::Empty(comment_elem))?;
    }

    writer.write_event(Event::End(BytesEnd::new("Script")))?;

    let result = writer.into_inner().into_inner();
    std::fs::write(path, result)?;

    Ok(())
}
