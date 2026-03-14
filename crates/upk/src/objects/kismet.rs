//! Kismet sequence node extraction from .umap files.
//!
//! Parses UE3 Kismet visual script nodes (SeqEvent_*, SeqAct_*, SeqCond_*, SeqVar_*,
//! Sequence) and reconstructs the wiring graph for content chain conversion.

use byteorder::{LittleEndian, ByteOrder};
use std::collections::HashMap;

use crate::names::NameEntry;
use crate::package::Package;
use crate::properties::{self, PropValue, TaggedProperty};

/// Kismet class prefixes for identifying Kismet exports.
const KISMET_PREFIXES: &[&str] = &[
    "Sequence", "SeqAct_", "SeqEvent_", "SeqCond_", "SeqVar_",
];

/// Check if a class name is a Kismet sequence class.
pub fn is_kismet_class(class_name: &str) -> bool {
    KISMET_PREFIXES.iter().any(|p| class_name.starts_with(p))
}

/// A connection from one node to another.
#[derive(Debug, Clone)]
pub struct KismetConnection {
    /// Globally unique key of the target node ("tile:export_index").
    pub target_key: String,
    /// Raw export index of the target (1-based).
    pub target_op: i32,
    /// Which input pin on the target to connect to.
    pub input_idx: i32,
}

/// A single link pin on a Kismet node (input, output, variable, or event).
#[derive(Debug, Clone)]
pub struct KismetLink {
    /// Description of this link pin.
    pub desc: String,
    /// Connections through this pin.
    pub connections: Vec<KismetConnection>,
    /// Expected type for variable links.
    pub expected_type: Option<String>,
    /// Property name for variable links.
    pub property_name: Option<String>,
}

/// A single Kismet sequence node.
#[derive(Debug, Clone)]
pub struct KismetNode {
    /// Globally unique key: "tile:export_index".
    pub node_key: String,
    /// 1-based export index within the tile.
    pub export_index: i32,
    /// Class name (e.g., "SeqAct_Interp", "SeqEvent_Touch").
    pub class_name: String,
    /// Object name from the export table.
    pub object_name: String,
    /// Full object path in the package.
    pub full_path: String,
    /// Node key of the parent Sequence.
    pub parent_sequence_key: String,
    /// Name of the parent Sequence.
    pub parent_name: String,
    /// Source tile filename.
    pub tile: String,
    /// ObjComment property if present.
    pub obj_comment: String,

    /// Input link pins.
    pub input_links: Vec<KismetLink>,
    /// Output link pins.
    pub output_links: Vec<KismetLink>,
    /// Variable link pins.
    pub variable_links: Vec<KismetLink>,
    /// Event link pins.
    pub event_links: Vec<KismetLink>,

    /// For Sequence containers: node keys of child objects.
    pub sequence_objects: Vec<String>,

    /// Extra server-relevant properties.
    pub properties: HashMap<String, KismetPropValue>,
}

/// Simplified property value for Kismet node metadata.
#[derive(Debug, Clone)]
pub enum KismetPropValue {
    Int(i32),
    Float(f32),
    Bool(bool),
    Str(String),
    Name(String),
    Object(i32),
}

/// Complete Kismet graph for a zone.
#[derive(Debug)]
pub struct KismetGraph {
    pub zone_name: String,
    pub nodes: HashMap<String, KismetNode>,
    pub sequences: HashMap<String, KismetNode>,
    pub tiles_processed: u32,
    pub errors: Vec<(String, String)>,
}

/// Create a globally unique node key from tile name and export index.
pub fn make_node_key(tile: &str, export_index: i32) -> String {
    format!("{}:{}", tile, export_index)
}

/// Extract all Kismet nodes from a single package/tile.
pub fn extract_kismet_nodes(pkg: &Package, tile_name: &str) -> Vec<KismetNode> {
    let mut nodes = Vec::new();

    for (i, export) in pkg.exports.iter().enumerate() {
        let class_name = pkg.export_class_name(export);
        if !is_kismet_class(class_name) {
            continue;
        }
        if export.serial_size <= 4 {
            continue;
        }

        let data = match pkg.read_export_data(export) {
            Ok(d) => d,
            Err(_) => continue,
        };

        if let Some(node) = parse_kismet_node(pkg, &data, i, class_name, export, tile_name) {
            nodes.push(node);
        }
    }

    nodes
}

/// Parse a single Kismet node from its serial data.
fn parse_kismet_node(
    pkg: &Package,
    data: &[u8],
    export_idx: usize,
    class_name: &str,
    export: &crate::exports::ExportEntry,
    tile_name: &str,
) -> Option<KismetNode> {
    // Kismet objects have a 4-byte serial header (not 32 like actors)
    let props = properties::parse_tagged_properties(data, 4, &pkg.names);

    let export_1based = (export_idx + 1) as i32;
    let node_key = make_node_key(tile_name, export_1based);

    // Parent sequence reference
    let parent_seq_raw = find_object_prop(&props, "ParentSequence").unwrap_or(0);
    let parent_seq_key = if parent_seq_raw != 0 {
        make_node_key(tile_name, parent_seq_raw)
    } else {
        String::new()
    };
    let parent_name = if parent_seq_raw != 0 {
        pkg.resolve_object_name(parent_seq_raw).to_string()
    } else {
        String::new()
    };

    // ObjComment
    let obj_comment = find_string_prop(&props, "ObjComment").unwrap_or_default();

    let mut node = KismetNode {
        node_key,
        export_index: export_1based,
        class_name: class_name.to_string(),
        object_name: export.object_name.clone(),
        full_path: pkg.export_full_path(export),
        parent_sequence_key: parent_seq_key,
        parent_name,
        tile: tile_name.to_string(),
        obj_comment,
        input_links: Vec::new(),
        output_links: Vec::new(),
        variable_links: Vec::new(),
        event_links: Vec::new(),
        sequence_objects: Vec::new(),
        properties: HashMap::new(),
    };

    // Parse link arrays
    for (prop_name, target) in &[
        ("InputLinks", LinkTarget::Input),
        ("OutputLinks", LinkTarget::Output),
        ("VariableLinks", LinkTarget::Variable),
        ("EventLinks", LinkTarget::Event),
    ] {
        if let Some(link_data) = find_array_prop(&props, prop_name) {
            let links = parse_link_array(&link_data, &pkg.names, tile_name, pkg);
            match target {
                LinkTarget::Input => node.input_links = links,
                LinkTarget::Output => node.output_links = links,
                LinkTarget::Variable => node.variable_links = links,
                LinkTarget::Event => node.event_links = links,
            }
        }
    }

    // SequenceObjects for Sequence containers
    if let Some(seq_data) = find_array_prop(&props, "SequenceObjects") {
        if seq_data.len() >= 4 {
            let count = LittleEndian::read_i32(&seq_data) as usize;
            for j in 0..count {
                let off = 4 + j * 4;
                if off + 4 <= seq_data.len() {
                    let idx = LittleEndian::read_i32(&seq_data[off..]);
                    node.sequence_objects.push(make_node_key(tile_name, idx));
                }
            }
        }
    }

    // Grab extra server-relevant properties
    for key in &[
        "EventName", "bOpen", "Duration", "PlayRate",
        "bEnabled", "bClientSideOnly", "MaxTriggerCount",
        "ReTriggerDelay", "Originator",
    ] {
        if let Some(val) = extract_kismet_prop(&props, key) {
            node.properties.insert(key.to_string(), val);
        }
    }

    Some(node)
}

enum LinkTarget {
    Input,
    Output,
    Variable,
    Event,
}

/// Parse a link array (InputLinks/OutputLinks/VariableLinks/EventLinks).
///
/// Each array element is a struct serialized as tagged properties terminated by "None".
/// OutputLinks contain an inner "Links" array with LinkedOp and InputLinkIdx.
/// VariableLinks contain a "LinkedVariables" array of object references.
fn parse_link_array(
    data: &[u8],
    names: &[NameEntry],
    tile_name: &str,
    pkg: &Package,
) -> Vec<KismetLink> {
    if data.len() < 4 {
        return Vec::new();
    }

    let count = LittleEndian::read_i32(data) as usize;
    let mut links = Vec::new();
    let mut pos = 4usize;

    for _ in 0..count {
        if pos >= data.len() {
            break;
        }
        let (elem_props, new_pos) = parse_nested_tagged_props(data, pos, names);
        pos = new_pos;

        let desc = find_string_prop(&elem_props, "LinkDesc")
            .or_else(|| find_name_prop(&elem_props, "LinkDesc"))
            .unwrap_or_default();

        let mut link = KismetLink {
            desc,
            connections: Vec::new(),
            expected_type: None,
            property_name: None,
        };

        // Parse inner Links array (OutputLinks have these)
        if let Some(inner_data) = find_array_prop(&elem_props, "Links") {
            if inner_data.len() >= 4 {
                let inner_count = LittleEndian::read_i32(&inner_data) as usize;
                let mut inner_pos = 4usize;
                for _ in 0..inner_count {
                    if inner_pos >= inner_data.len() {
                        break;
                    }
                    let (inner_props, np) = parse_nested_tagged_props(&inner_data, inner_pos, names);
                    inner_pos = np;

                    let target_op = find_object_prop(&inner_props, "LinkedOp").unwrap_or(0);
                    if target_op != 0 {
                        link.connections.push(KismetConnection {
                            target_key: make_node_key(tile_name, target_op),
                            target_op,
                            input_idx: find_int_prop(&inner_props, "InputLinkIdx").unwrap_or(0),
                        });
                    }
                }
            }
        }

        // Parse LinkedVariables for variable links
        if let Some(var_data) = find_array_prop(&elem_props, "LinkedVariables") {
            if var_data.len() >= 4 {
                let var_count = LittleEndian::read_i32(&var_data) as usize;
                for j in 0..var_count {
                    let off = 4 + j * 4;
                    if off + 4 <= var_data.len() {
                        let lv = LittleEndian::read_i32(&var_data[off..]);
                        if lv != 0 {
                            link.connections.push(KismetConnection {
                                target_key: make_node_key(tile_name, lv),
                                target_op: lv,
                                input_idx: 0,
                            });
                        }
                    }
                }
            }
        }

        // Expected type for variable links
        if let Some(et) = find_object_prop(&elem_props, "ExpectedType") {
            link.expected_type = Some(pkg.resolve_object_name(et).to_string());
        }
        if let Some(pn) = find_name_prop(&elem_props, "PropertyName") {
            link.property_name = Some(pn);
        }

        links.push(link);
    }

    links
}

/// Parse tagged properties from a byte buffer (nested within an array element).
/// Returns (properties, new_position).
fn parse_nested_tagged_props(data: &[u8], offset: usize, names: &[NameEntry]) -> (Vec<TaggedProperty>, usize) {
    let props = properties::parse_tagged_properties(data, offset, names);

    // Calculate how far we consumed. We need to find the "None" terminator position.
    // Re-walk to find the actual end position.
    let mut pos = offset;
    while pos + 8 <= data.len() {
        let name_idx = LittleEndian::read_i32(&data[pos..]) as usize;
        let _name_num = LittleEndian::read_i32(&data[pos + 4..]);
        pos += 8;

        if name_idx >= names.len() {
            break;
        }
        let name = &names[name_idx].name;
        if name == "None" {
            break;
        }

        if pos + 8 > data.len() {
            break;
        }
        let type_idx = LittleEndian::read_i32(&data[pos..]) as usize;
        pos += 8;
        let type_name = if type_idx < names.len() {
            &names[type_idx].name
        } else {
            break;
        };

        if pos + 8 > data.len() {
            break;
        }
        let prop_size = LittleEndian::read_i32(&data[pos..]) as usize;
        pos += 4;
        let _array_idx = LittleEndian::read_i32(&data[pos..]);
        pos += 4;

        if type_name == "StructProperty" {
            if pos + 8 > data.len() {
                break;
            }
            pos += 8; // struct type FName
        }

        if type_name == "BoolProperty" {
            if pos + 4 > data.len() {
                break;
            }
            pos += 4;
            continue;
        }

        if type_name == "ByteProperty" {
            if pos + 8 > data.len() {
                break;
            }
            pos += 8; // enum type FName
        }

        if pos + prop_size > data.len() {
            break;
        }
        pos += prop_size;
    }

    (props, pos)
}

// --- Helper functions for finding properties by name ---

fn find_object_prop(props: &[TaggedProperty], name: &str) -> Option<i32> {
    props.iter().find(|p| p.name == name).and_then(|p| {
        if let PropValue::Object(v) = &p.value {
            Some(*v)
        } else {
            None
        }
    })
}

fn find_int_prop(props: &[TaggedProperty], name: &str) -> Option<i32> {
    props.iter().find(|p| p.name == name).and_then(|p| {
        if let PropValue::Int(v) = &p.value {
            Some(*v)
        } else {
            None
        }
    })
}

fn find_string_prop(props: &[TaggedProperty], name: &str) -> Option<String> {
    props.iter().find(|p| p.name == name).and_then(|p| {
        if let PropValue::Str(v) = &p.value {
            Some(v.clone())
        } else {
            None
        }
    })
}

fn find_name_prop(props: &[TaggedProperty], name: &str) -> Option<String> {
    props.iter().find(|p| p.name == name).and_then(|p| {
        if let PropValue::Name(v) = &p.value {
            Some(v.clone())
        } else {
            None
        }
    })
}

fn find_array_prop(props: &[TaggedProperty], name: &str) -> Option<Vec<u8>> {
    props.iter().find(|p| p.name == name).and_then(|p| {
        if let PropValue::Array(v) = &p.value {
            Some(v.clone())
        } else {
            None
        }
    })
}

/// Extract a server-relevant property as a KismetPropValue.
fn extract_kismet_prop(props: &[TaggedProperty], name: &str) -> Option<KismetPropValue> {
    props.iter().find(|p| p.name == name).and_then(|p| {
        match &p.value {
            PropValue::Int(v) => Some(KismetPropValue::Int(*v)),
            PropValue::Float(v) => Some(KismetPropValue::Float(*v)),
            PropValue::Bool(v) => Some(KismetPropValue::Bool(*v)),
            PropValue::Str(v) => Some(KismetPropValue::Str(v.clone())),
            PropValue::Name(v) => Some(KismetPropValue::Name(v.clone())),
            PropValue::Object(v) => Some(KismetPropValue::Object(*v)),
            _ => None,
        }
    })
}
