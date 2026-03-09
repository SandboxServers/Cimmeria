use crate::script::{compiler, definitions, xml_format};
use crate::state::AppState;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// DTOs for JSON transfer to the frontend
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct ScriptEntry {
    pub path: String,
    pub script_type: String,
    pub module: String,
    pub file_name: String,
}

#[derive(Serialize, Deserialize)]
pub struct ScriptFileDto {
    pub version: String,
    pub dataset_version: String,
    pub module: String,
    pub script_type: String,
    pub next_id: u32,
    pub nodes: Vec<ScriptNodeDto>,
    pub connections: Vec<ScriptConnectionDto>,
    pub comments: Vec<ScriptCommentDto>,
}

#[derive(Serialize, Deserialize)]
pub struct ScriptNodeDto {
    pub id: u32,
    pub ref_name: String,
    pub x: i32,
    pub y: i32,
    pub properties: Vec<(String, String)>,
    pub ports: Vec<(String, u32)>,
}

#[derive(Serialize, Deserialize)]
pub struct ScriptConnectionDto {
    pub out_node: u32,
    pub out_port: String,
    pub in_node: u32,
    pub in_port: String,
}

#[derive(Serialize, Deserialize)]
pub struct ScriptCommentDto {
    pub id: u32,
    pub text: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub color: u32,
}

#[derive(Serialize)]
pub struct CompileResult {
    pub output_path: String,
    pub python_code: String,
}

#[derive(Serialize)]
pub struct NodeTemplateDto {
    pub ref_name: String,
    pub display_name: String,
    pub node_type: String,
    pub category: String,
    pub description: String,
    pub ports: Vec<PortTemplateDto>,
    pub properties: Vec<PropertyTemplateDto>,
    pub script_types: Vec<String>,
    pub imports: Vec<String>,
    pub has_static_init: bool,
    pub has_static_teardown: bool,
    pub has_pre_init: bool,
    pub has_init: bool,
    pub has_teardown: bool,
    pub has_persist: bool,
    pub has_restore: bool,
    pub methods: Vec<String>,
}

#[derive(Serialize)]
pub struct PortTemplateDto {
    pub name: String,
    pub port_type: String,
    pub direction: String,
    pub default_hide: bool,
    pub required: bool,
    pub has_script: bool,
}

#[derive(Serialize)]
pub struct PropertyTemplateDto {
    pub name: String,
    pub prop_type: String,
    pub default_value: String,
    pub database_ref: Option<String>,
}

#[derive(Serialize)]
pub struct EnumDefinitionDto {
    pub name: String,
    pub data_type: String,
    pub is_bitfield: bool,
    pub tokens: Vec<EnumTokenDto>,
}

#[derive(Serialize)]
pub struct EnumTokenDto {
    pub name: String,
    pub value: String,
}

// ---------------------------------------------------------------------------
// Conversion helpers
// ---------------------------------------------------------------------------

impl From<&definitions::NodeTemplate> for NodeTemplateDto {
    fn from(t: &definitions::NodeTemplate) -> Self {
        Self {
            ref_name: t.ref_name.clone(),
            display_name: t.display_name.clone(),
            node_type: t.node_type.clone(),
            category: t.category.clone(),
            description: t.description.clone(),
            ports: t.ports.iter().map(PortTemplateDto::from).collect(),
            properties: t.properties.iter().map(PropertyTemplateDto::from).collect(),
            script_types: t.script_types.clone(),
            imports: t.imports.clone(),
            has_static_init: t.static_init_script.is_some(),
            has_static_teardown: t.static_teardown_script.is_some(),
            has_pre_init: t.pre_init_script.is_some(),
            has_init: t.init_script.is_some(),
            has_teardown: t.teardown_script.is_some(),
            has_persist: t.persist_script.is_some(),
            has_restore: t.restore_script.is_some(),
            methods: t.methods.iter().map(|m| m.name.clone()).collect(),
        }
    }
}

impl From<&definitions::PortTemplate> for PortTemplateDto {
    fn from(p: &definitions::PortTemplate) -> Self {
        Self {
            name: p.name.clone(),
            port_type: p.port_type.clone(),
            direction: p.direction.clone(),
            default_hide: p.default_hide,
            required: p.required,
            has_script: p.script.is_some(),
        }
    }
}

impl From<&definitions::PropertyTemplate> for PropertyTemplateDto {
    fn from(p: &definitions::PropertyTemplate) -> Self {
        Self {
            name: p.name.clone(),
            prop_type: p.prop_type.clone(),
            default_value: p.default_value.clone(),
            database_ref: p.database_ref.clone(),
        }
    }
}

impl From<&definitions::EnumDefinition> for EnumDefinitionDto {
    fn from(e: &definitions::EnumDefinition) -> Self {
        Self {
            name: e.name.clone(),
            data_type: e.data_type.clone(),
            is_bitfield: e.is_bitfield,
            tokens: e.tokens.iter().map(EnumTokenDto::from).collect(),
        }
    }
}

impl From<&definitions::EnumToken> for EnumTokenDto {
    fn from(t: &definitions::EnumToken) -> Self {
        Self {
            name: t.name.clone(),
            value: t.value.clone(),
        }
    }
}

fn dto_to_script_file(dto: &ScriptFileDto) -> xml_format::ScriptFile {
    xml_format::ScriptFile {
        version: dto.version.clone(),
        dataset_version: dto.dataset_version.clone(),
        module: dto.module.clone(),
        script_type: dto.script_type.clone(),
        next_id: dto.next_id,
        nodes: dto
            .nodes
            .iter()
            .map(|n| xml_format::ScriptNode {
                id: n.id,
                ref_name: n.ref_name.clone(),
                x: n.x,
                y: n.y,
                properties: n.properties.clone(),
                ports: n.ports.clone(),
            })
            .collect(),
        connections: dto
            .connections
            .iter()
            .map(|c| xml_format::ScriptConnection {
                out_node: c.out_node,
                out_port: c.out_port.clone(),
                in_node: c.in_node,
                in_port: c.in_port.clone(),
            })
            .collect(),
        comments: dto
            .comments
            .iter()
            .map(|c| xml_format::ScriptComment {
                id: c.id,
                text: c.text.clone(),
                x: c.x,
                y: c.y,
                width: c.width,
                height: c.height,
                color: c.color,
            })
            .collect(),
    }
}

fn script_file_to_dto(sf: &xml_format::ScriptFile) -> ScriptFileDto {
    ScriptFileDto {
        version: sf.version.clone(),
        dataset_version: sf.dataset_version.clone(),
        module: sf.module.clone(),
        script_type: sf.script_type.clone(),
        next_id: sf.next_id,
        nodes: sf
            .nodes
            .iter()
            .map(|n| ScriptNodeDto {
                id: n.id,
                ref_name: n.ref_name.clone(),
                x: n.x,
                y: n.y,
                properties: n.properties.clone(),
                ports: n.ports.clone(),
            })
            .collect(),
        connections: sf
            .connections
            .iter()
            .map(|c| ScriptConnectionDto {
                out_node: c.out_node,
                out_port: c.out_port.clone(),
                in_node: c.in_node,
                in_port: c.in_port.clone(),
            })
            .collect(),
        comments: sf
            .comments
            .iter()
            .map(|c| ScriptCommentDto {
                id: c.id,
                text: c.text.clone(),
                x: c.x,
                y: c.y,
                width: c.width,
                height: c.height,
                color: c.color,
            })
            .collect(),
    }
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

/// List all .script files under data/scripts/ with type/module info.
#[tauri::command]
pub async fn list_scripts(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<ScriptEntry>, String> {
    let scripts_dir = state.scripts_dir();
    let mut entries = Vec::new();

    if !scripts_dir.exists() {
        return Ok(entries);
    }

    fn walk_dir(dir: &std::path::Path, base: &std::path::Path, entries: &mut Vec<ScriptEntry>) {
        let Ok(read) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in read.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk_dir(&path, base, entries);
            } else if path.extension().and_then(|e| e.to_str()) == Some("script") {
                let rel = path.strip_prefix(base).unwrap_or(&path);
                let rel_str = rel.to_string_lossy().replace('\\', "/");

                // Determine script type from first directory component
                let script_type = rel
                    .components()
                    .next()
                    .and_then(|c| {
                        let s = c.as_os_str().to_string_lossy();
                        match s.as_ref() {
                            "missions" => Some("Mission"),
                            "effects" => Some("Effect"),
                            "spaces" => Some("Level"),
                            _ => None,
                        }
                    })
                    .unwrap_or("Unknown")
                    .to_string();

                // Module: strip type dir prefix and .script extension, join with dots
                let module = rel
                    .components()
                    .skip(1)
                    .map(|c| c.as_os_str().to_string_lossy().into_owned())
                    .collect::<Vec<_>>()
                    .join(".")
                    .strip_suffix(".script")
                    .unwrap_or("")
                    .to_string();

                let file_name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();

                entries.push(ScriptEntry {
                    path: rel_str,
                    script_type,
                    module,
                    file_name,
                });
            }
        }
    }

    walk_dir(&scripts_dir, &scripts_dir, &mut entries);
    entries.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(entries)
}

/// Load a .script file and return its parsed contents.
#[tauri::command]
pub async fn load_script(
    state: tauri::State<'_, AppState>,
    path: String,
) -> Result<ScriptFileDto, String> {
    let full_path = state.scripts_dir().join(&path);
    let sf = xml_format::load_script(&full_path).map_err(|e| format!("Failed to load script: {e}"))?;
    Ok(script_file_to_dto(&sf))
}

/// Save a script file.
#[tauri::command]
pub async fn save_script(
    state: tauri::State<'_, AppState>,
    path: String,
    script: ScriptFileDto,
) -> Result<(), String> {
    let full_path = state.scripts_dir().join(&path);
    let sf = dto_to_script_file(&script);
    xml_format::save_script(&full_path, &sf).map_err(|e| format!("Failed to save script: {e}"))
}

/// Compile a script to Python and return the output path + generated code.
#[tauri::command]
pub async fn compile_script(
    state: tauri::State<'_, AppState>,
    path: String,
) -> Result<CompileResult, String> {
    let full_path = state.scripts_dir().join(&path);
    let sf = xml_format::load_script(&full_path).map_err(|e| format!("Failed to load script: {e}"))?;

    let defs = state
        .script_definitions()
        .map_err(|e| format!("Failed to load definitions: {e}"))?;

    let python_code =
        compiler::compile_script(&sf, defs).map_err(|e| format!("Compilation failed: {e}"))?;

    let output_path = compiler::compile_to_file(&sf, defs, &state.repo_root)
        .map_err(|e| format!("Failed to write output: {e}"))?;

    Ok(CompileResult {
        output_path: output_path
            .strip_prefix(&state.repo_root)
            .unwrap_or(&output_path)
            .to_string_lossy()
            .to_string(),
        python_code,
    })
}

/// Load all node templates from Nodes.xml (cached in AppState).
#[tauri::command]
pub async fn load_node_templates(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<NodeTemplateDto>, String> {
    let defs = state
        .script_definitions()
        .map_err(|e| format!("Failed to load definitions: {e}"))?;
    Ok(defs.nodes.iter().map(NodeTemplateDto::from).collect())
}

/// Load all enumerations from enumerations.xml (cached in AppState).
#[tauri::command]
pub async fn load_enumerations(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<EnumDefinitionDto>, String> {
    let defs = state
        .script_definitions()
        .map_err(|e| format!("Failed to load definitions: {e}"))?;
    Ok(defs.enums.iter().map(EnumDefinitionDto::from).collect())
}
