//! Kismet visual script extraction and viewer data for the Scene Editor.
//!
//! Extracts Kismet sequence nodes from loaded zone .umap files and returns
//! graph data suitable for visualization in a React node graph component.

use std::collections::HashMap;
use std::path::Path;

use serde::Serialize;

use cimmeria_upk::objects::kismet;
use cimmeria_upk::Package;

use crate::state::AppState;

// ── Response types ─────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct KismetGraphData {
    pub nodes: Vec<KismetNodeData>,
    pub edges: Vec<KismetEdge>,
    pub sequences: Vec<KismetSequenceData>,
    pub node_count: usize,
    pub edge_count: usize,
}

#[derive(Debug, Serialize)]
pub struct KismetNodeData {
    pub key: String,
    pub class_name: String,
    pub object_name: String,
    pub parent_sequence: String,
    pub parent_name: String,
    pub tile: String,
    pub comment: String,
    pub category: String, // "event", "action", "condition", "variable", "sequence"
    pub input_pins: Vec<String>,
    pub output_pins: Vec<String>,
    pub variable_pins: Vec<String>,
    pub properties: HashMap<String, String>,
}

#[derive(Debug, Serialize)]
pub struct KismetEdge {
    pub source_key: String,
    pub source_pin: usize,
    pub target_key: String,
    pub target_pin: usize,
    pub edge_type: String, // "output", "variable", "event"
}

#[derive(Debug, Serialize)]
pub struct KismetSequenceData {
    pub key: String,
    pub name: String,
    pub child_count: usize,
}

// ── Commands ───────────────────────────────────────────────────────────────

/// Extract Kismet graph from the loaded zone's .umap files.
#[tauri::command]
pub async fn extract_kismet_graph(
    state: tauri::State<'_, AppState>,
) -> Result<KismetGraphData, String> {
    let zone_path = {
        let guard = state.loaded_zone.lock().unwrap();
        let zone = guard.as_ref().ok_or("No zone loaded")?;
        zone.zone_path.clone()
    };

    tracing::info!("Extracting Kismet graph from {}", zone_path);

    let zone_dir = Path::new(&zone_path);
    if !zone_dir.is_dir() {
        return Err(format!("Zone path is not a directory: {zone_path}"));
    }

    // Collect all .umap files
    let mut umap_files: Vec<_> = std::fs::read_dir(zone_dir)
        .map_err(|e| format!("Failed to read zone directory: {e}"))?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map_or(false, |ext| ext.eq_ignore_ascii_case("umap"))
        })
        .map(|e| e.path())
        .collect();
    umap_files.sort();

    let mut all_nodes: HashMap<String, kismet::KismetNode> = HashMap::new();
    let mut all_sequences: HashMap<String, kismet::KismetNode> = HashMap::new();

    for umap_path in &umap_files {
        let tile_name = umap_path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();

        match Package::open(umap_path) {
            Ok(pkg) => {
                let nodes = kismet::extract_kismet_nodes(&pkg, &tile_name);
                for node in nodes {
                    if node.class_name == "Sequence" {
                        all_sequences.insert(node.node_key.clone(), node);
                    } else {
                        all_nodes.insert(node.node_key.clone(), node);
                    }
                }
            }
            Err(e) => {
                tracing::debug!("Failed to parse {} for Kismet: {e}", tile_name);
            }
        }
    }

    // Build the response
    let mut nodes = Vec::with_capacity(all_nodes.len());
    let mut edges = Vec::new();

    for (key, node) in &all_nodes {
        let category = if node.class_name.starts_with("SeqEvent_") {
            "event"
        } else if node.class_name.starts_with("SeqAct_") {
            "action"
        } else if node.class_name.starts_with("SeqCond_") {
            "condition"
        } else if node.class_name.starts_with("SeqVar_") {
            "variable"
        } else {
            "other"
        };

        let mut properties = HashMap::new();
        for (pk, pv) in &node.properties {
            let value_str = match pv {
                kismet::KismetPropValue::Int(v) => v.to_string(),
                kismet::KismetPropValue::Float(v) => format!("{v:.2}"),
                kismet::KismetPropValue::Bool(v) => v.to_string(),
                kismet::KismetPropValue::Str(v) => v.clone(),
                kismet::KismetPropValue::Name(v) => v.clone(),
                kismet::KismetPropValue::Object(v) => format!("obj:{v}"),
            };
            properties.insert(pk.clone(), value_str);
        }

        nodes.push(KismetNodeData {
            key: key.clone(),
            class_name: node.class_name.clone(),
            object_name: node.object_name.clone(),
            parent_sequence: node.parent_sequence_key.clone(),
            parent_name: node.parent_name.clone(),
            tile: node.tile.clone(),
            comment: node.obj_comment.clone(),
            category: category.to_string(),
            input_pins: node.input_links.iter().map(|l| l.desc.clone()).collect(),
            output_pins: node.output_links.iter().map(|l| l.desc.clone()).collect(),
            variable_pins: node
                .variable_links
                .iter()
                .map(|l| l.property_name.as_deref().unwrap_or(&l.desc).to_string())
                .collect(),
            properties,
        });

        // Build edges from output links
        for (pin_idx, link) in node.output_links.iter().enumerate() {
            for conn in &link.connections {
                edges.push(KismetEdge {
                    source_key: key.clone(),
                    source_pin: pin_idx,
                    target_key: conn.target_key.clone(),
                    target_pin: conn.input_idx as usize,
                    edge_type: "output".to_string(),
                });
            }
        }

        // Build edges from variable links
        for (pin_idx, link) in node.variable_links.iter().enumerate() {
            for conn in &link.connections {
                edges.push(KismetEdge {
                    source_key: key.clone(),
                    source_pin: pin_idx,
                    target_key: conn.target_key.clone(),
                    target_pin: 0,
                    edge_type: "variable".to_string(),
                });
            }
        }

        // Build edges from event links
        for (pin_idx, link) in node.event_links.iter().enumerate() {
            for conn in &link.connections {
                edges.push(KismetEdge {
                    source_key: key.clone(),
                    source_pin: pin_idx,
                    target_key: conn.target_key.clone(),
                    target_pin: 0,
                    edge_type: "event".to_string(),
                });
            }
        }
    }

    let sequences: Vec<KismetSequenceData> = all_sequences
        .iter()
        .map(|(key, seq)| KismetSequenceData {
            key: key.clone(),
            name: seq.object_name.clone(),
            child_count: seq.sequence_objects.len(),
        })
        .collect();

    let edge_count = edges.len();
    let node_count = nodes.len();

    tracing::info!(
        nodes = node_count,
        edges = edge_count,
        sequences = sequences.len(),
        "Kismet graph extracted"
    );

    Ok(KismetGraphData {
        nodes,
        edges,
        sequences,
        node_count,
        edge_count,
    })
}
