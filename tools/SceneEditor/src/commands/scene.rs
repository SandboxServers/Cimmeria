use crate::state::{ActorEntry, AppState, LoadedZone, PropertyPair, UndoAction, ZoneBounds};
use cimmeria_upk::objects::actor::extract_actors;
use cimmeria_upk::properties::PropValue;
use cimmeria_upk::Package;
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;

// ---------------------------------------------------------------------------
// IPC response types
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct ZoneInfo {
    pub name: String,
    pub path: String,
    pub tile_count: u32,
}

#[derive(Serialize)]
pub struct ZoneSummary {
    pub zone_name: String,
    pub tile_count: u32,
    pub actor_count: u32,
    pub class_counts: HashMap<String, usize>,
    pub bounds: ZoneBounds,
}

#[derive(Serialize)]
pub struct ActorListEntry {
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
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

/// Scan `{cooked_pc_path}/Maps/` for zone subdirectories containing .umap files.
#[tauri::command]
pub async fn scan_zones(
    state: tauri::State<'_, AppState>,
    cooked_pc_path: String,
) -> Result<Vec<ZoneInfo>, String> {
    tracing::info!("scan_zones: {}", cooked_pc_path);

    let maps_dir = Path::new(&cooked_pc_path).join("Maps");
    if !maps_dir.is_dir() {
        return Err(format!("Maps directory not found: {}", maps_dir.display()));
    }

    // Store the path for later use
    *state.cooked_pc_path.lock().unwrap() = Some(cooked_pc_path.into());

    let mut zones = Vec::new();

    let entries = std::fs::read_dir(&maps_dir)
        .map_err(|e| format!("Failed to read Maps directory: {e}"))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read dir entry: {e}"))?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        // Count .umap files in this directory
        let tile_count = count_umap_files(&path);
        if tile_count == 0 {
            continue;
        }

        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        zones.push(ZoneInfo {
            name,
            path: path.to_string_lossy().to_string(),
            tile_count,
        });
    }

    zones.sort_by(|a, b| a.name.cmp(&b.name));
    tracing::info!("Found {} zones", zones.len());

    Ok(zones)
}

/// Load all .umap tiles in a zone directory and extract actors.
#[tauri::command]
pub async fn load_zone(
    state: tauri::State<'_, AppState>,
    zone_path: String,
) -> Result<ZoneSummary, String> {
    tracing::info!("load_zone: {}", zone_path);

    let zone_dir = Path::new(&zone_path);
    if !zone_dir.is_dir() {
        return Err(format!("Zone directory not found: {}", zone_path));
    }

    let zone_name = zone_dir
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let mut all_actors = Vec::new();
    let mut class_counts: HashMap<String, usize> = HashMap::new();
    let mut bounds = ZoneBounds::empty();
    let mut tile_count = 0u32;

    let entries = std::fs::read_dir(zone_dir)
        .map_err(|e| format!("Failed to read zone directory: {e}"))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read dir entry: {e}"))?;
        let path = entry.path();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext.to_lowercase() != "umap" {
            continue;
        }

        let tile_stem = path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        tracing::debug!("Parsing tile: {}", tile_stem);

        let pkg = match Package::open(&path) {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!("Failed to parse {}: {e}", path.display());
                continue;
            }
        };

        let actors = extract_actors(&pkg);
        tile_count += 1;

        for (idx, actor) in actors.iter().enumerate() {
            let key = format!("{}:{}", tile_stem, idx);

            // Extract DrawScale3D components if present
            let draw_scale_3d = find_vector_prop(&actor.properties, "DrawScale3D");
            let (dsx, dsy, dsz) = draw_scale_3d.unwrap_or((1.0, 1.0, 1.0));

            // Extract static mesh reference from properties
            let static_mesh = find_object_name_prop(&actor.properties, "StaticMesh", &pkg);

            // Convert all properties to display pairs
            let properties = actor
                .properties
                .iter()
                .map(|p| PropertyPair {
                    name: p.name.clone(),
                    value: format_prop_value(&p.value),
                })
                .collect();

            let entry = ActorEntry {
                key,
                class_name: actor.class_name.clone(),
                object_name: actor.object_name.clone(),
                tile: tile_stem.clone(),
                x: actor.location[0],
                y: actor.location[1],
                z: actor.location[2],
                pitch: actor.rotation_deg[0],
                yaw: actor.rotation_deg[1],
                roll: actor.rotation_deg[2],
                draw_scale: actor.draw_scale,
                draw_scale_x: dsx,
                draw_scale_y: dsy,
                draw_scale_z: dsz,
                static_mesh,
                properties,
            };

            bounds.expand(entry.x, entry.y, entry.z);
            *class_counts.entry(entry.class_name.clone()).or_insert(0) += 1;
            all_actors.push(entry);
        }
    }

    let actor_count = all_actors.len() as u32;

    // If no actors were found, set bounds to zero
    if all_actors.is_empty() {
        bounds = ZoneBounds {
            min_x: 0.0,
            max_x: 0.0,
            min_y: 0.0,
            max_y: 0.0,
            min_z: 0.0,
            max_z: 0.0,
        };
    }

    tracing::info!(
        "Loaded zone '{}': {} tiles, {} actors, {} classes",
        zone_name,
        tile_count,
        actor_count,
        class_counts.len()
    );

    let summary = ZoneSummary {
        zone_name: zone_name.clone(),
        tile_count,
        actor_count,
        class_counts: class_counts.clone(),
        bounds: bounds.clone(),
    };

    *state.loaded_zone.lock().unwrap() = Some(LoadedZone {
        zone_name,
        zone_path: zone_path.clone(),
        tile_count,
        actors: all_actors,
        class_counts,
        bounds,
    });

    Ok(summary)
}

/// Return all actors from the loaded zone, optionally filtered by class.
#[tauri::command]
pub async fn list_actors(
    state: tauri::State<'_, AppState>,
    class_filter: Option<String>,
) -> Result<Vec<ActorListEntry>, String> {
    let guard = state.loaded_zone.lock().unwrap();
    let zone = guard.as_ref().ok_or("No zone loaded")?;

    let iter = zone.actors.iter().filter(|a| {
        class_filter
            .as_ref()
            .map_or(true, |f| a.class_name == *f)
    });

    let entries: Vec<ActorListEntry> = iter
        .map(|a| ActorListEntry {
            key: a.key.clone(),
            class_name: a.class_name.clone(),
            object_name: a.object_name.clone(),
            tile: a.tile.clone(),
            x: a.x,
            y: a.y,
            z: a.z,
            yaw: a.yaw,
            pitch: a.pitch,
            roll: a.roll,
            draw_scale: a.draw_scale,
            draw_scale_x: a.draw_scale_x,
            draw_scale_y: a.draw_scale_y,
            draw_scale_z: a.draw_scale_z,
            static_mesh: a.static_mesh.clone(),
        })
        .collect();

    Ok(entries)
}

/// Return full details for one actor by its key.
#[tauri::command]
pub async fn get_actor_details(
    state: tauri::State<'_, AppState>,
    key: String,
) -> Result<ActorEntry, String> {
    let guard = state.loaded_zone.lock().unwrap();
    let zone = guard.as_ref().ok_or("No zone loaded")?;

    zone.actors
        .iter()
        .find(|a| a.key == key)
        .cloned()
        .ok_or_else(|| format!("Actor not found: {key}"))
}

/// Find the nearest actor to a world coordinate within a given radius.
#[tauri::command]
pub async fn pick_actor(
    state: tauri::State<'_, AppState>,
    world_x: f32,
    world_y: f32,
    radius: f32,
) -> Result<Option<String>, String> {
    let guard = state.loaded_zone.lock().unwrap();
    let zone = guard.as_ref().ok_or("No zone loaded")?;

    let radius_sq = radius * radius;
    let mut best_key: Option<String> = None;
    let mut best_dist_sq = f32::MAX;

    for actor in &zone.actors {
        let dx = actor.x - world_x;
        let dy = actor.y - world_y;
        let dist_sq = dx * dx + dy * dy;

        if dist_sq < radius_sq && dist_sq < best_dist_sq {
            best_dist_sq = dist_sq;
            best_key = Some(actor.key.clone());
        }
    }

    Ok(best_key)
}

// ---------------------------------------------------------------------------
// Mutation commands
// ---------------------------------------------------------------------------

/// Delete actors by their keys. Returns the number of actors deleted.
#[tauri::command]
pub async fn delete_actors(
    state: tauri::State<'_, AppState>,
    keys: Vec<String>,
) -> Result<u32, String> {
    let mut guard = state.loaded_zone.lock().unwrap();
    let zone = guard.as_mut().ok_or("No zone loaded")?;

    let key_set: std::collections::HashSet<&str> = keys.iter().map(|s| s.as_str()).collect();
    let mut removed = Vec::new();
    let mut kept = Vec::new();

    for actor in zone.actors.drain(..) {
        if key_set.contains(actor.key.as_str()) {
            removed.push(actor);
        } else {
            kept.push(actor);
        }
    }

    let count = removed.len() as u32;
    zone.actors = kept;

    // Rebuild class counts
    zone.class_counts.clear();
    for a in &zone.actors {
        *zone.class_counts.entry(a.class_name.clone()).or_insert(0) += 1;
    }

    drop(guard);
    state.push_undo(UndoAction::DeleteActors(removed));

    tracing::info!("Deleted {} actors", count);
    Ok(count)
}

/// Duplicate actors by their keys. Returns the new actor list entries.
#[tauri::command]
pub async fn duplicate_actors(
    state: tauri::State<'_, AppState>,
    keys: Vec<String>,
    offset_x: f32,
    offset_y: f32,
    offset_z: f32,
) -> Result<Vec<ActorListEntry>, String> {
    let mut guard = state.loaded_zone.lock().unwrap();
    let zone = guard.as_mut().ok_or("No zone loaded")?;

    let key_set: std::collections::HashSet<&str> = keys.iter().map(|s| s.as_str()).collect();
    let sources: Vec<ActorEntry> = zone.actors.iter()
        .filter(|a| key_set.contains(a.key.as_str()))
        .cloned()
        .collect();

    let mut new_entries = Vec::new();
    let mut new_keys = Vec::new();
    let mut id = state.next_actor_id.lock().unwrap();

    for src in sources {
        let new_key = format!("dup:{}", *id);
        *id += 1;

        let new_actor = ActorEntry {
            key: new_key.clone(),
            object_name: format!("{}_dup", src.object_name),
            x: src.x + offset_x,
            y: src.y + offset_y,
            z: src.z + offset_z,
            ..src
        };

        new_entries.push(ActorListEntry {
            key: new_actor.key.clone(),
            class_name: new_actor.class_name.clone(),
            object_name: new_actor.object_name.clone(),
            tile: new_actor.tile.clone(),
            x: new_actor.x,
            y: new_actor.y,
            z: new_actor.z,
            yaw: new_actor.yaw,
            pitch: new_actor.pitch,
            roll: new_actor.roll,
            draw_scale: new_actor.draw_scale,
            draw_scale_x: new_actor.draw_scale_x,
            draw_scale_y: new_actor.draw_scale_y,
            draw_scale_z: new_actor.draw_scale_z,
            static_mesh: new_actor.static_mesh.clone(),
        });

        *zone.class_counts.entry(new_actor.class_name.clone()).or_insert(0) += 1;
        new_keys.push(new_key);
        zone.actors.push(new_actor);
    }

    drop(guard);
    state.push_undo(UndoAction::CreateActors(new_keys));

    tracing::info!("Duplicated {} actors", new_entries.len());
    Ok(new_entries)
}

/// Move actors to new positions. Takes parallel arrays of keys and new coordinates.
#[tauri::command]
pub async fn move_actors(
    state: tauri::State<'_, AppState>,
    keys: Vec<String>,
    new_x: Vec<f32>,
    new_y: Vec<f32>,
    new_z: Vec<f32>,
) -> Result<(), String> {
    if keys.len() != new_x.len() || keys.len() != new_y.len() || keys.len() != new_z.len() {
        return Err("Array length mismatch".into());
    }

    let mut guard = state.loaded_zone.lock().unwrap();
    let zone = guard.as_mut().ok_or("No zone loaded")?;

    let mut old_positions = Vec::new();

    for (i, key) in keys.iter().enumerate() {
        if let Some(actor) = zone.actors.iter_mut().find(|a| a.key == *key) {
            old_positions.push((key.clone(), actor.x, actor.y, actor.z));
            actor.x = new_x[i];
            actor.y = new_y[i];
            actor.z = new_z[i];
        }
    }

    drop(guard);
    state.push_undo(UndoAction::MoveActors(old_positions));

    Ok(())
}

/// Update rotation for actors.
#[tauri::command]
pub async fn rotate_actors(
    state: tauri::State<'_, AppState>,
    keys: Vec<String>,
    new_yaw: Vec<f32>,
    new_pitch: Vec<f32>,
    new_roll: Vec<f32>,
) -> Result<(), String> {
    if keys.len() != new_yaw.len() || keys.len() != new_pitch.len() || keys.len() != new_roll.len() {
        return Err("Array length mismatch".into());
    }

    let mut guard = state.loaded_zone.lock().unwrap();
    let zone = guard.as_mut().ok_or("No zone loaded")?;

    let mut old_rotations = Vec::new();

    for (i, key) in keys.iter().enumerate() {
        if let Some(actor) = zone.actors.iter_mut().find(|a| a.key == *key) {
            old_rotations.push((key.clone(), actor.yaw, actor.pitch, actor.roll));
            actor.yaw = new_yaw[i];
            actor.pitch = new_pitch[i];
            actor.roll = new_roll[i];
        }
    }

    drop(guard);
    state.push_undo(UndoAction::RotateActors(old_rotations));

    Ok(())
}

/// Update scale for actors.
#[tauri::command]
pub async fn scale_actors(
    state: tauri::State<'_, AppState>,
    keys: Vec<String>,
    new_draw_scale: Vec<f32>,
    new_dsx: Vec<f32>,
    new_dsy: Vec<f32>,
    new_dsz: Vec<f32>,
) -> Result<(), String> {
    if keys.len() != new_draw_scale.len() {
        return Err("Array length mismatch".into());
    }

    let mut guard = state.loaded_zone.lock().unwrap();
    let zone = guard.as_mut().ok_or("No zone loaded")?;

    let mut old_scales = Vec::new();

    for (i, key) in keys.iter().enumerate() {
        if let Some(actor) = zone.actors.iter_mut().find(|a| a.key == *key) {
            old_scales.push((
                key.clone(),
                actor.draw_scale,
                actor.draw_scale_x,
                actor.draw_scale_y,
                actor.draw_scale_z,
            ));
            actor.draw_scale = new_draw_scale[i];
            actor.draw_scale_x = new_dsx[i];
            actor.draw_scale_y = new_dsy[i];
            actor.draw_scale_z = new_dsz[i];
        }
    }

    drop(guard);
    state.push_undo(UndoAction::ScaleActors(old_scales));

    Ok(())
}

/// Undo the last action. Returns true if an action was undone.
#[tauri::command]
pub async fn undo(state: tauri::State<'_, AppState>) -> Result<bool, String> {
    let action = state.undo_stack.lock().unwrap().pop();
    let Some(action) = action else { return Ok(false) };

    let mut guard = state.loaded_zone.lock().unwrap();
    let zone = guard.as_mut().ok_or("No zone loaded")?;

    let redo_action = match action {
        UndoAction::DeleteActors(actors) => {
            // Re-insert deleted actors
            let keys: Vec<String> = actors.iter().map(|a| a.key.clone()).collect();
            for a in &actors {
                *zone.class_counts.entry(a.class_name.clone()).or_insert(0) += 1;
            }
            zone.actors.extend(actors);
            UndoAction::CreateActors(keys)
        }
        UndoAction::CreateActors(keys) => {
            // Remove the created actors
            let key_set: std::collections::HashSet<&str> = keys.iter().map(|s| s.as_str()).collect();
            let removed: Vec<ActorEntry> = zone.actors.iter()
                .filter(|a| key_set.contains(a.key.as_str()))
                .cloned()
                .collect();
            zone.actors.retain(|a| !key_set.contains(a.key.as_str()));
            zone.class_counts.clear();
            for a in &zone.actors {
                *zone.class_counts.entry(a.class_name.clone()).or_insert(0) += 1;
            }
            UndoAction::DeleteActors(removed)
        }
        UndoAction::MoveActors(old_positions) => {
            let mut new_positions = Vec::new();
            for (key, old_x, old_y, old_z) in &old_positions {
                if let Some(actor) = zone.actors.iter_mut().find(|a| &a.key == key) {
                    new_positions.push((key.clone(), actor.x, actor.y, actor.z));
                    actor.x = *old_x;
                    actor.y = *old_y;
                    actor.z = *old_z;
                }
            }
            UndoAction::MoveActors(new_positions)
        }
        UndoAction::RotateActors(old_rotations) => {
            let mut new_rotations = Vec::new();
            for (key, old_yaw, old_pitch, old_roll) in &old_rotations {
                if let Some(actor) = zone.actors.iter_mut().find(|a| &a.key == key) {
                    new_rotations.push((key.clone(), actor.yaw, actor.pitch, actor.roll));
                    actor.yaw = *old_yaw;
                    actor.pitch = *old_pitch;
                    actor.roll = *old_roll;
                }
            }
            UndoAction::RotateActors(new_rotations)
        }
        UndoAction::ScaleActors(old_scales) => {
            let mut new_scales = Vec::new();
            for (key, old_ds, old_dsx, old_dsy, old_dsz) in &old_scales {
                if let Some(actor) = zone.actors.iter_mut().find(|a| &a.key == key) {
                    new_scales.push((
                        key.clone(),
                        actor.draw_scale,
                        actor.draw_scale_x,
                        actor.draw_scale_y,
                        actor.draw_scale_z,
                    ));
                    actor.draw_scale = *old_ds;
                    actor.draw_scale_x = *old_dsx;
                    actor.draw_scale_y = *old_dsy;
                    actor.draw_scale_z = *old_dsz;
                }
            }
            UndoAction::ScaleActors(new_scales)
        }
        UndoAction::PropertyChange(key, prop_name, old_value) => {
            if let Some(actor) = zone.actors.iter_mut().find(|a| a.key == key) {
                match old_value {
                    Some(old_val) => {
                        // Property existed before — swap current value back
                        let current_val = actor
                            .properties
                            .iter_mut()
                            .find(|p| p.name == prop_name)
                            .map(|p| {
                                let cur = p.value.clone();
                                p.value = old_val;
                                cur
                            })
                            .unwrap_or_default();
                        UndoAction::PropertyChange(key, prop_name, Some(current_val))
                    }
                    None => {
                        // Property was created — remove it, stash the current value for redo
                        let current_val = actor
                            .properties
                            .iter()
                            .find(|p| p.name == prop_name)
                            .map(|p| p.value.clone())
                            .unwrap_or_default();
                        actor.properties.retain(|p| p.name != prop_name);
                        // Redo should re-create with the value that was there
                        UndoAction::PropertyChange(key, prop_name, Some(current_val))
                    }
                }
            } else {
                // Actor gone — nothing to redo
                UndoAction::PropertyChange(key, prop_name, old_value)
            }
        }
    };

    drop(guard);
    state.redo_stack.lock().unwrap().push(redo_action);

    tracing::info!("Undo performed");
    Ok(true)
}

/// Redo the last undone action. Returns true if an action was redone.
#[tauri::command]
pub async fn redo(state: tauri::State<'_, AppState>) -> Result<bool, String> {
    let action = state.redo_stack.lock().unwrap().pop();
    let Some(action) = action else { return Ok(false) };

    let mut guard = state.loaded_zone.lock().unwrap();
    let zone = guard.as_mut().ok_or("No zone loaded")?;

    // Redo is the same as applying the inverse of the redo action
    let undo_action = match action {
        UndoAction::DeleteActors(actors) => {
            let keys: Vec<String> = actors.iter().map(|a| a.key.clone()).collect();
            for a in &actors {
                *zone.class_counts.entry(a.class_name.clone()).or_insert(0) += 1;
            }
            zone.actors.extend(actors);
            UndoAction::CreateActors(keys)
        }
        UndoAction::CreateActors(keys) => {
            let key_set: std::collections::HashSet<&str> = keys.iter().map(|s| s.as_str()).collect();
            let removed: Vec<ActorEntry> = zone.actors.iter()
                .filter(|a| key_set.contains(a.key.as_str()))
                .cloned()
                .collect();
            zone.actors.retain(|a| !key_set.contains(a.key.as_str()));
            zone.class_counts.clear();
            for a in &zone.actors {
                *zone.class_counts.entry(a.class_name.clone()).or_insert(0) += 1;
            }
            UndoAction::DeleteActors(removed)
        }
        UndoAction::MoveActors(positions) => {
            let mut old_positions = Vec::new();
            for (key, x, y, z) in &positions {
                if let Some(actor) = zone.actors.iter_mut().find(|a| &a.key == key) {
                    old_positions.push((key.clone(), actor.x, actor.y, actor.z));
                    actor.x = *x;
                    actor.y = *y;
                    actor.z = *z;
                }
            }
            UndoAction::MoveActors(old_positions)
        }
        UndoAction::RotateActors(rotations) => {
            let mut old_rotations = Vec::new();
            for (key, yaw, pitch, roll) in &rotations {
                if let Some(actor) = zone.actors.iter_mut().find(|a| &a.key == key) {
                    old_rotations.push((key.clone(), actor.yaw, actor.pitch, actor.roll));
                    actor.yaw = *yaw;
                    actor.pitch = *pitch;
                    actor.roll = *roll;
                }
            }
            UndoAction::RotateActors(old_rotations)
        }
        UndoAction::ScaleActors(scales) => {
            let mut old_scales = Vec::new();
            for (key, ds, dsx, dsy, dsz) in &scales {
                if let Some(actor) = zone.actors.iter_mut().find(|a| &a.key == key) {
                    old_scales.push((
                        key.clone(),
                        actor.draw_scale,
                        actor.draw_scale_x,
                        actor.draw_scale_y,
                        actor.draw_scale_z,
                    ));
                    actor.draw_scale = *ds;
                    actor.draw_scale_x = *dsx;
                    actor.draw_scale_y = *dsy;
                    actor.draw_scale_z = *dsz;
                }
            }
            UndoAction::ScaleActors(old_scales)
        }
        UndoAction::PropertyChange(key, prop_name, old_value) => {
            if let Some(actor) = zone.actors.iter_mut().find(|a| a.key == key) {
                match old_value {
                    Some(val) => {
                        // Redo: set property to the stored value
                        let current_val = if let Some(prop) =
                            actor.properties.iter_mut().find(|p| p.name == prop_name)
                        {
                            let cur = prop.value.clone();
                            prop.value = val;
                            Some(cur)
                        } else {
                            // Property was removed by undo — recreate it
                            actor.properties.push(PropertyPair {
                                name: prop_name.clone(),
                                value: val,
                            });
                            None
                        };
                        UndoAction::PropertyChange(key, prop_name, current_val)
                    }
                    None => {
                        // This shouldn't happen in normal redo flow, but handle gracefully
                        UndoAction::PropertyChange(key, prop_name, None)
                    }
                }
            } else {
                UndoAction::PropertyChange(key, prop_name, old_value)
            }
        }
    };

    drop(guard);
    state.undo_stack.lock().unwrap().push(undo_action);

    tracing::info!("Redo performed");
    Ok(true)
}

/// Update a single property on an actor by key. Creates the property if it
/// doesn't exist. Returns the old value (or null if the property was created).
#[tauri::command]
pub async fn update_actor_property(
    state: tauri::State<'_, AppState>,
    key: String,
    property_name: String,
    new_value: String,
) -> Result<Option<String>, String> {
    let mut guard = state.loaded_zone.lock().unwrap();
    let zone = guard.as_mut().ok_or("No zone loaded")?;

    let actor = zone
        .actors
        .iter_mut()
        .find(|a| a.key == key)
        .ok_or_else(|| format!("Actor not found: {key}"))?;

    let old_value = if let Some(prop) = actor.properties.iter_mut().find(|p| p.name == property_name)
    {
        let old = prop.value.clone();
        prop.value = new_value;
        Some(old)
    } else {
        // Property doesn't exist yet — create it
        actor.properties.push(PropertyPair {
            name: property_name.clone(),
            value: new_value,
        });
        None
    };

    drop(guard);
    state.push_undo(UndoAction::PropertyChange(key, property_name, old_value.clone()));

    Ok(old_value)
}

// ---------------------------------------------------------------------------
// Procedural placement commands
// ---------------------------------------------------------------------------

/// Helper: find a source actor by key, clone it N times at the given positions,
/// assign new keys, update zone state, push undo, and return list entries.
fn place_copies(
    state: &AppState,
    source_key: &str,
    positions: Vec<(f32, f32, f32)>,
) -> Result<Vec<ActorListEntry>, String> {
    let mut guard = state.loaded_zone.lock().unwrap();
    let zone = guard.as_mut().ok_or("No zone loaded")?;

    let src = zone
        .actors
        .iter()
        .find(|a| a.key == source_key)
        .cloned()
        .ok_or_else(|| format!("Source actor not found: {source_key}"))?;

    let mut new_entries = Vec::with_capacity(positions.len());
    let mut new_keys = Vec::with_capacity(positions.len());
    let mut id = state.next_actor_id.lock().unwrap();

    for (x, y, z) in positions {
        let new_key = format!("dup:{}", *id);
        *id += 1;

        let new_actor = ActorEntry {
            key: new_key.clone(),
            object_name: format!("{}_dup", src.object_name),
            x,
            y,
            z,
            ..src.clone()
        };

        new_entries.push(ActorListEntry {
            key: new_actor.key.clone(),
            class_name: new_actor.class_name.clone(),
            object_name: new_actor.object_name.clone(),
            tile: new_actor.tile.clone(),
            x: new_actor.x,
            y: new_actor.y,
            z: new_actor.z,
            yaw: new_actor.yaw,
            pitch: new_actor.pitch,
            roll: new_actor.roll,
            draw_scale: new_actor.draw_scale,
            draw_scale_x: new_actor.draw_scale_x,
            draw_scale_y: new_actor.draw_scale_y,
            draw_scale_z: new_actor.draw_scale_z,
            static_mesh: new_actor.static_mesh.clone(),
        });

        *zone
            .class_counts
            .entry(new_actor.class_name.clone())
            .or_insert(0) += 1;
        new_keys.push(new_key);
        zone.actors.push(new_actor);
    }

    drop(id);
    drop(guard);
    state.push_undo(UndoAction::CreateActors(new_keys));

    Ok(new_entries)
}

/// Place copies of an actor in a grid pattern.
/// The source actor's position is used as the grid origin (row 0, col 0).
#[tauri::command]
pub async fn place_grid(
    state: tauri::State<'_, AppState>,
    source_key: String,
    spacing_x: f64,
    spacing_y: f64,
    rows: u32,
    cols: u32,
) -> Result<Vec<ActorListEntry>, String> {
    if rows == 0 || cols == 0 {
        return Err("rows and cols must be > 0".into());
    }

    // Read the source position so the grid is anchored there
    let (origin_x, origin_y, origin_z) = {
        let guard = state.loaded_zone.lock().unwrap();
        let zone = guard.as_ref().ok_or("No zone loaded")?;
        let src = zone
            .actors
            .iter()
            .find(|a| a.key == source_key)
            .ok_or_else(|| format!("Source actor not found: {source_key}"))?;
        (src.x as f64, src.y as f64, src.z as f64)
    };

    let mut positions = Vec::with_capacity((rows * cols) as usize);
    for row in 0..rows {
        for col in 0..cols {
            // Skip (0,0) — that's the source actor itself
            if row == 0 && col == 0 {
                continue;
            }
            let x = origin_x + col as f64 * spacing_x;
            let y = origin_y + row as f64 * spacing_y;
            positions.push((x as f32, y as f32, origin_z as f32));
        }
    }

    let result = place_copies(&state, &source_key, positions)?;
    tracing::info!(
        "place_grid: {} new actors ({}x{}, spacing {}/{})",
        result.len(),
        rows,
        cols,
        spacing_x,
        spacing_y
    );
    Ok(result)
}

/// Place copies of an actor evenly around a circle.
#[tauri::command]
pub async fn place_circle(
    state: tauri::State<'_, AppState>,
    source_key: String,
    center_x: f64,
    center_y: f64,
    center_z: f64,
    radius: f64,
    count: u32,
    start_angle: f64,
) -> Result<Vec<ActorListEntry>, String> {
    if count == 0 {
        return Err("count must be > 0".into());
    }

    let step = std::f64::consts::TAU / count as f64;
    let start_rad = start_angle.to_radians();

    let positions: Vec<(f32, f32, f32)> = (0..count)
        .map(|i| {
            let angle = start_rad + i as f64 * step;
            let x = center_x + radius * angle.cos();
            let y = center_y + radius * angle.sin();
            (x as f32, y as f32, center_z as f32)
        })
        .collect();

    let result = place_copies(&state, &source_key, positions)?;
    tracing::info!(
        "place_circle: {} actors, radius={}, start={}deg",
        result.len(),
        radius,
        start_angle
    );
    Ok(result)
}

/// Place copies of an actor along an Archimedean spiral.
#[tauri::command]
pub async fn place_spiral(
    state: tauri::State<'_, AppState>,
    source_key: String,
    center_x: f64,
    center_y: f64,
    center_z: f64,
    start_radius: f64,
    end_radius: f64,
    turns: f64,
    count: u32,
) -> Result<Vec<ActorListEntry>, String> {
    if count == 0 {
        return Err("count must be > 0".into());
    }
    if turns <= 0.0 {
        return Err("turns must be > 0".into());
    }

    let total_angle = turns * std::f64::consts::TAU;

    let positions: Vec<(f32, f32, f32)> = (0..count)
        .map(|i| {
            let t = if count == 1 {
                0.0
            } else {
                i as f64 / (count - 1) as f64
            };
            let angle = t * total_angle;
            let r = start_radius + t * (end_radius - start_radius);
            let x = center_x + r * angle.cos();
            let y = center_y + r * angle.sin();
            (x as f32, y as f32, center_z as f32)
        })
        .collect();

    let result = place_copies(&state, &source_key, positions)?;
    tracing::info!(
        "place_spiral: {} actors, radius {}..{}, {} turns",
        result.len(),
        start_radius,
        end_radius,
        turns
    );
    Ok(result)
}

// ---------------------------------------------------------------------------
// Export / Save commands
// ---------------------------------------------------------------------------

/// SGW classes that are meaningful for server-side spawn/placement data.
const SGW_CLASSES: &[&str] = &[
    "SGWSpawnRegion",
    "SGWSpawnSet",
    "SGWMob",
    "SGWNpc",
    "SGWInteractable",
    "SGWStargate",
    "SGWDoor",
    "SGWTeleporter",
    "SGWMinigame",
    "PlayerStart",
];

/// Export actors as SQL INSERT statements for the Cimmeria game database.
#[tauri::command]
pub async fn export_sql(
    state: tauri::State<'_, AppState>,
    file_path: String,
    class_filter: Option<Vec<String>>,
) -> Result<u32, String> {
    let guard = state.loaded_zone.lock().unwrap();
    let zone = guard.as_ref().ok_or("No zone loaded")?;

    // Determine which classes to export
    let allowed_classes: Vec<&str> = match &class_filter {
        Some(filter) => filter.iter().map(|s| s.as_str()).collect(),
        None => SGW_CLASSES.to_vec(),
    };

    // Group actors by class for readable output
    let mut by_class: HashMap<String, Vec<&ActorEntry>> = HashMap::new();
    for actor in &zone.actors {
        if allowed_classes.iter().any(|c| *c == actor.class_name) {
            by_class
                .entry(actor.class_name.clone())
                .or_default()
                .push(actor);
        }
    }

    let timestamp = {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        // Format as ISO-ish: good enough without pulling in chrono
        format!("{now} (unix)")
    };

    let mut sql = String::new();
    sql.push_str(&format!("-- Zone: {}\n", zone.zone_name));
    sql.push_str(&format!("-- Exported: {}\n\n", timestamp));

    let mut total_count = 0u32;

    // Sort class names for deterministic output
    let mut class_names: Vec<&String> = by_class.keys().collect();
    class_names.sort();

    for class_name in class_names {
        let actors = &by_class[class_name];
        sql.push_str(&format!(
            "-- Class: {} ({} actors)\n\n",
            class_name,
            actors.len()
        ));

        for actor in actors {
            sql.push_str(&format!(
                "INSERT INTO zone_actors (zone_name, class_name, object_name, tile, x, y, z, pitch, yaw, roll, draw_scale, draw_scale_x, draw_scale_y, draw_scale_z, static_mesh)\nVALUES ('{}', '{}', '{}', '{}', {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, '{}');\n\n",
                escape_sql(&zone.zone_name),
                escape_sql(class_name),
                escape_sql(&actor.object_name),
                escape_sql(&actor.tile),
                actor.x,
                actor.y,
                actor.z,
                actor.pitch,
                actor.yaw,
                actor.roll,
                actor.draw_scale,
                actor.draw_scale_x,
                actor.draw_scale_y,
                actor.draw_scale_z,
                escape_sql(&actor.static_mesh),
            ));
            total_count += 1;
        }
    }

    drop(guard);

    std::fs::write(&file_path, sql)
        .map_err(|e| format!("Failed to write SQL file: {e}"))?;

    tracing::info!(
        "Exported {} actors to SQL: {}",
        total_count,
        file_path
    );

    Ok(total_count)
}

/// Save all actors in the loaded zone to a JSON file.
#[tauri::command]
pub async fn save_zone(
    state: tauri::State<'_, AppState>,
    file_path: String,
) -> Result<u32, String> {
    let guard = state.loaded_zone.lock().unwrap();
    let zone = guard.as_ref().ok_or("No zone loaded")?;

    let count = zone.actors.len() as u32;

    let json = serde_json::to_string_pretty(&zone.actors)
        .map_err(|e| format!("Failed to serialize actors: {e}"))?;

    drop(guard);

    std::fs::write(&file_path, json)
        .map_err(|e| format!("Failed to write JSON file: {e}"))?;

    tracing::info!("Saved {} actors to JSON: {}", count, file_path);

    Ok(count)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Escape single quotes for SQL string literals.
fn escape_sql(s: &str) -> String {
    s.replace('\'', "''")
}

fn count_umap_files(dir: &Path) -> u32 {
    std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.path()
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .map_or(false, |ext| ext.eq_ignore_ascii_case("umap"))
                })
                .count() as u32
        })
        .unwrap_or(0)
}

fn find_vector_prop(
    props: &[cimmeria_upk::TaggedProperty],
    name: &str,
) -> Option<(f32, f32, f32)> {
    props.iter().find(|p| p.name == name).and_then(|p| {
        if let PropValue::Vector { x, y, z } = &p.value {
            Some((*x, *y, *z))
        } else {
            None
        }
    })
}

fn find_object_name_prop(
    props: &[cimmeria_upk::TaggedProperty],
    name: &str,
    pkg: &Package,
) -> String {
    props
        .iter()
        .find(|p| p.name == name)
        .and_then(|p| {
            if let PropValue::Object(idx) = &p.value {
                Some(pkg.resolve_object_name(*idx).to_string())
            } else {
                None
            }
        })
        .unwrap_or_default()
}

fn format_prop_value(value: &PropValue) -> String {
    match value {
        PropValue::Int(v) => v.to_string(),
        PropValue::Float(v) => format!("{v:.4}"),
        PropValue::Bool(v) => v.to_string(),
        PropValue::Str(v) => v.clone(),
        PropValue::Name(v) => v.clone(),
        PropValue::Object(v) => format!("Object({v})"),
        PropValue::Vector { x, y, z } => format!("({x:.1}, {y:.1}, {z:.1})"),
        PropValue::Rotator { pitch, yaw, roll } => {
            let p = *pitch as f32 * 360.0 / 65536.0;
            let y = *yaw as f32 * 360.0 / 65536.0;
            let r = *roll as f32 * 360.0 / 65536.0;
            format!("({p:.1}, {y:.1}, {r:.1})")
        }
        PropValue::Color { r, g, b, a } => format!("RGBA({r}, {g}, {b}, {a})"),
        PropValue::LinearColor { r, g, b, a } => format!("({r:.3}, {g:.3}, {b:.3}, {a:.3})"),
        PropValue::Array(data) => format!("Array[{} bytes]", data.len()),
        PropValue::Struct { struct_type, data } => {
            format!("{struct_type}[{} bytes]", data.len())
        }
        PropValue::Byte(data) => {
            if data.len() == 8 {
                // Likely an FName enum value
                let idx = u32::from_le_bytes(data[0..4].try_into().unwrap_or([0; 4]));
                format!("Byte(idx={idx})")
            } else {
                format!("Byte[{} bytes]", data.len())
            }
        }
        PropValue::Raw { type_name, data } => format!("{type_name}[{} bytes]", data.len()),
    }
}
