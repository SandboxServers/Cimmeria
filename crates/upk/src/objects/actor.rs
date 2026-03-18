//! AActor subclass extraction — positions, rotations, and properties from .umap files.

use crate::package::Package;
use crate::properties::{self, PropValue, TaggedProperty};

/// Known AActor subclasses (have 32-byte serial header).
const ACTOR_CLASSES: &[&str] = &[
    "StaticMeshActor", "Brush", "Terrain", "WorldInfo",
    "SGWSpecCoverNode", "PointLight", "SpotLight", "DirectionalLight",
    "AmbientSoundSGW", "AmbientSound", "SpeedTreeActor",
    "Trigger", "TriggerVolume", "BlockingVolume", "PhysicsVolume",
    "PrefabInstance", "DecalActor", "Emitter", "PointLightMovable",
    "Note", "CameraActor", "PlayerStart", "PathNode",
    "InterpActor", "SkeletalMeshActor", "KActor",
    "SGWSpawnRegion", "SGWSpawnSet", "SGWCoverSet",
    "SGWMob", "SGWNpc", "SGWStargate", "SGWInteractable",
    "SGWDoor", "SGWTeleporter", "SGWMinigame",
    "SkyLight", "SGWSkyDomeActor", "HeightFog",
    "MaterialInstanceActor",
];

/// Extracted actor data.
#[derive(Debug, Clone)]
pub struct ActorData {
    pub class_name: String,
    pub object_name: String,
    pub full_path: String,
    pub location: [f32; 3],
    pub rotation_deg: [f32; 3],
    pub draw_scale: f32,
    pub properties: Vec<TaggedProperty>,
}

/// Check if a class name is an AActor subclass.
pub fn is_actor_class(class_name: &str) -> bool {
    ACTOR_CLASSES.contains(&class_name)
}

/// Extract all actors from a parsed package.
pub fn extract_actors(pkg: &Package) -> Vec<ActorData> {
    let mut actors = Vec::new();

    for export in &pkg.exports {
        let class_name = pkg.export_class_name(export);
        if !is_actor_class(class_name) {
            continue;
        }
        if export.serial_size <= 32 {
            continue;
        }

        let data = match pkg.read_export_data(export) {
            Ok(d) => d,
            Err(_) => continue,
        };

        let props = properties::parse_tagged_properties(&data, 32, &pkg.names);

        let location = find_vector(&props, "Location").unwrap_or([0.0; 3]);
        let rotation_raw = find_rotator(&props, "Rotation").unwrap_or([0; 3]);
        let rotation_deg = [
            rotation_raw[0] as f32 * 360.0 / 65536.0,
            rotation_raw[1] as f32 * 360.0 / 65536.0,
            rotation_raw[2] as f32 * 360.0 / 65536.0,
        ];
        let draw_scale = find_float(&props, "DrawScale").unwrap_or(1.0);

        actors.push(ActorData {
            class_name: class_name.to_string(),
            object_name: export.object_name.clone(),
            full_path: pkg.export_full_path(export),
            location,
            rotation_deg,
            draw_scale,
            properties: props,
        });
    }

    actors
}

fn find_vector(props: &[TaggedProperty], name: &str) -> Option<[f32; 3]> {
    props.iter().find(|p| p.name == name).and_then(|p| {
        if let PropValue::Vector { x, y, z } = &p.value {
            Some([*x, *y, *z])
        } else {
            None
        }
    })
}

fn find_rotator(props: &[TaggedProperty], name: &str) -> Option<[i32; 3]> {
    props.iter().find(|p| p.name == name).and_then(|p| {
        if let PropValue::Rotator { pitch, yaw, roll } = &p.value {
            Some([*pitch, *yaw, *roll])
        } else {
            None
        }
    })
}

fn find_float(props: &[TaggedProperty], name: &str) -> Option<f32> {
    props.iter().find(|p| p.name == name).and_then(|p| {
        if let PropValue::Float(v) = &p.value {
            Some(*v)
        } else {
            None
        }
    })
}
