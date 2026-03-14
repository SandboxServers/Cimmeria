use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use cimmeria_upk_objects::PackageIndex;

/// Top-level application state, managed by Tauri.
pub struct AppState {
    pub cooked_pc_path: Mutex<Option<PathBuf>>,
    pub loaded_zone: Mutex<Option<LoadedZone>>,
    pub mesh_cache: Mutex<HashMap<String, crate::commands::mesh::MeshData>>,
    pub package_index: Mutex<Option<PackageIndex>>,
    pub undo_stack: Mutex<Vec<UndoAction>>,
    pub redo_stack: Mutex<Vec<UndoAction>>,
    pub next_actor_id: Mutex<u64>,
    /// PostgreSQL connection pool for entity placement queries.
    pub db_pool: Mutex<Option<sqlx::PgPool>>,
    /// Cached DB entities from the most recent `load_db_entities` call.
    pub db_entities: Mutex<Vec<crate::commands::database::DbEntity>>,
}

/// An undoable action — stores enough info to reverse the operation.
#[derive(Debug, Clone)]
pub enum UndoAction {
    /// Actors were deleted; stores the removed actors to restore on undo.
    DeleteActors(Vec<ActorEntry>),
    /// Actors were created (duplicated); stores their keys to delete on undo.
    CreateActors(Vec<String>),
    /// Actors were moved; stores (key, old_x, old_y, old_z) for each.
    MoveActors(Vec<(String, f32, f32, f32)>),
    /// Actors were rotated; stores (key, old_yaw, old_pitch, old_roll).
    RotateActors(Vec<(String, f32, f32, f32)>),
    /// Actors were scaled; stores (key, old_draw_scale, old_dsx, old_dsy, old_dsz).
    ScaleActors(Vec<(String, f32, f32, f32, f32)>),
}

/// A fully loaded zone with all extracted actor data.
pub struct LoadedZone {
    pub zone_name: String,
    pub tile_count: u32,
    pub actors: Vec<ActorEntry>,
    pub class_counts: HashMap<String, usize>,
    pub bounds: ZoneBounds,
}

/// A single actor extracted from a .umap tile.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ActorEntry {
    /// Unique key: "tile_stem:export_index"
    pub key: String,
    pub class_name: String,
    pub object_name: String,
    pub tile: String,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub roll: f32,
    pub draw_scale: f32,
    pub draw_scale_x: f32,
    pub draw_scale_y: f32,
    pub draw_scale_z: f32,
    pub static_mesh: String,
    /// All tagged properties as displayable key-value pairs.
    pub properties: Vec<PropertyPair>,
}

/// A simplified property for display in the property window.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PropertyPair {
    pub name: String,
    pub value: String,
}

/// Axis-aligned bounding box for all actors in a zone.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ZoneBounds {
    pub min_x: f32,
    pub max_x: f32,
    pub min_y: f32,
    pub max_y: f32,
    pub min_z: f32,
    pub max_z: f32,
}

impl ZoneBounds {
    pub fn empty() -> Self {
        Self {
            min_x: f32::MAX,
            max_x: f32::MIN,
            min_y: f32::MAX,
            max_y: f32::MIN,
            min_z: f32::MAX,
            max_z: f32::MIN,
        }
    }

    pub fn expand(&mut self, x: f32, y: f32, z: f32) {
        self.min_x = self.min_x.min(x);
        self.max_x = self.max_x.max(x);
        self.min_y = self.min_y.min(y);
        self.max_y = self.max_y.max(y);
        self.min_z = self.min_z.min(z);
        self.max_z = self.max_z.max(z);
    }
}

impl AppState {
    pub fn new() -> Self {
        Self {
            cooked_pc_path: Mutex::new(None),
            loaded_zone: Mutex::new(None),
            mesh_cache: Mutex::new(HashMap::new()),
            package_index: Mutex::new(None),
            undo_stack: Mutex::new(Vec::new()),
            redo_stack: Mutex::new(Vec::new()),
            next_actor_id: Mutex::new(0),
            db_pool: Mutex::new(None),
            db_entities: Mutex::new(Vec::new()),
        }
    }

    /// Push an undo action and clear the redo stack.
    pub fn push_undo(&self, action: UndoAction) {
        self.undo_stack.lock().unwrap().push(action);
        self.redo_stack.lock().unwrap().clear();
    }
}
