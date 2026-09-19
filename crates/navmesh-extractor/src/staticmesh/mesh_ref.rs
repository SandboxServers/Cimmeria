//! Actor → `StaticMeshComponent` → `StaticMesh` reference resolution.
//!
//! Split out of the walker so the "which package/object does this actor
//! point at, and if it doesn't, *why not*" decision has one home. Every
//! failure path here returns a typed [`crate::coverage::SkipReason`]
//! rather than a bare `None`, because the whole point of the coverage
//! report is to say precisely where actors fall out of the chain.

use cimmeria_upk::{ImportEntry, Package, PropValue, TaggedProperty};

use crate::coverage::SkipReason;
use crate::transform::ActorTransform;

/// Outcome of resolving one actor's mesh reference.
///
/// The `Ok` payload is `(package_name, object_name)`. An **empty**
/// package name is the sentinel for "a `StaticMesh` export in the
/// package the reference was read from" — see [`is_local`].
pub type MeshRefResult = std::result::Result<(String, String), SkipReason>;

/// Is this a mesh reference local to the chunk it was read from?
///
/// A cross-package reference resolves through a
/// [`cimmeria_upk_objects::PackageIndex`]; a local one must be decoded
/// straight out of the already-open chunk, because the index is keyed
/// on `(package, object)` and there is no package name to key on.
/// Feeding a local reference to the index yields
/// [`SkipReason::MeshNotInIndex`] and drops the geometry, which is what
/// this predicate exists to stop.
pub fn is_local(mesh_ref: &(String, String)) -> bool {
    mesh_ref.0.is_empty()
}

/// Build an [`ActorTransform`] from a slice of tagged properties. Missing
/// properties take their UE3-cooked defaults (origin, no rotation,
/// uniform scale).
pub fn transform_from_actor_props(props: &[TaggedProperty]) -> ActorTransform {
    let location = find_vector(props, "Location").unwrap_or([0.0; 3]);
    let rotation = find_rotator(props, "Rotation").unwrap_or([0, 0, 0]);
    let draw_scale = find_float(props, "DrawScale").unwrap_or(1.0);
    let draw_scale_3d = find_vector(props, "DrawScale3D").unwrap_or([1.0, 1.0, 1.0]);
    ActorTransform {
        location,
        rotation,
        draw_scale,
        draw_scale_3d,
    }
}

/// Resolve a `StaticMeshComponent` export reference into the
/// `(package_name, object_name)` pair its `StaticMesh` property points
/// to, or the reason it could not be resolved.
pub fn resolve_mesh_ref_from_component(pkg: &Package, component_ref: i32) -> MeshRefResult {
    if component_ref <= 0 {
        // Components are typically embedded exports, so a positive
        // 1-based export index. Imports (negative) would be unusual.
        return Err(SkipReason::ComponentUnreadable);
    }
    let idx = (component_ref - 1) as usize;
    let Some(component) = pkg.exports.get(idx) else {
        return Err(SkipReason::ComponentUnreadable);
    };
    if component.serial_size <= 0 {
        return Err(SkipReason::ComponentUnreadable);
    }
    let Ok(data) = pkg.read_export_data(component) else {
        return Err(SkipReason::ComponentUnreadable);
    };

    // Cooked StaticMeshComponent layout: 8-byte binary header (NetIndex
    // + component-specific prefix) then tagged properties. Verified by
    // probing component export 1771 in Castle_CellBlock chunk fffefffd:
    // offset 4 yields 0 properties (parser hits a garbage FName), offset
    // 8 yields the expected 10 properties starting with the `StaticMesh`
    // ObjectProperty. The 4-byte offset that `crates/upk-objects/src/
    // static_mesh.rs` uses is for non-Component objects (StaticMesh
    // itself), which only have a 4-byte NetIndex prefix.
    //
    // The `castle_cellblock_walks_static_mesh_actors` integration test
    // pins this — reverting to offset 4 drops resolvable instances to 0.
    let props = cimmeria_upk::parse_tagged_properties(&data, 8, &pkg.names);

    // A component the level author switched collision off on renders but
    // does not block; rasterising it would add a wall or a floor the
    // player walks straight through. UE3's default is `true` and the
    // cooker omits defaults, so only an explicit `false` gates here.
    //
    // Measured: no Castle component, instance or prefab template, sets
    // the *component*-level flag at all — the 26 Castle prefabs that
    // disable collision do it on the **actor** (`bCollideActors`),
    // which the walker gates on separately via
    // `archetype::ActorArchetypeProps::collides`.
    if crate::staticmesh::archetype::find_bool(&props, "CollideActors") == Some(false) {
        return Err(SkipReason::CollisionDisabled);
    }

    let Some(mesh_obj) = find_object(&props, "StaticMesh") else {
        // No `StaticMesh` property at all. In SGW cooked chunks this is
        // the prefab-archetype stub: the component carries only a
        // `CullDistance` override and inherits the mesh reference from
        // the archetype's component in another package.
        return Err(SkipReason::ArchetypeStubComponent);
    };
    if mesh_obj == 0 {
        return Err(SkipReason::NullMeshRef);
    }

    // Cross-package StaticMesh references are imports — negative obj
    // index. Local-to-this-package StaticMesh exports are positive and
    // unusual for `.umap` files but possible.
    if mesh_obj < 0 {
        let imp_idx = (-mesh_obj - 1) as usize;
        let Some(imp) = pkg.imports.get(imp_idx) else {
            return Err(SkipReason::UnresolvableMeshRef);
        };
        // The import's `package_index` chain leads up to a root package
        // import; walk to the topmost ancestor to recover the .upk name.
        match top_package_name(pkg, imp) {
            Some(package_name) => Ok((package_name, imp.object_name.clone())),
            None => Err(SkipReason::UnresolvableMeshRef),
        }
    } else {
        let Some(exp) = pkg.exports.get((mesh_obj - 1) as usize) else {
            return Err(SkipReason::UnresolvableMeshRef);
        };
        if pkg.export_class_name(exp) != "StaticMesh" {
            // A `StaticMesh` property pointing at something that is not
            // one. Decoding its bytes would yield a plausible-looking
            // mesh built out of an unrelated export.
            return Err(SkipReason::UnresolvableMeshRef);
        }
        // The mesh is an export of the package we are already holding.
        // Recorded with an **empty package name**, which is the
        // loader's sentinel for "decode this locally" — see
        // [`is_local`]. Handing it to the `PackageIndex` instead
        // returns `MeshNotInIndex` and silently drops the collision
        // geometry. SGW's Castle chunks reference imports throughout,
        // so this branch is dormant there, but a map that does not is
        // not a reason to lose its floors.
        Ok((String::new(), exp.object_name.clone()))
    }
}

/// Walk an import's outer chain back to the root package and return
/// its name (the `.upk` stem).
pub fn top_package_name<'a>(pkg: &'a Package, mut imp: &'a ImportEntry) -> Option<String> {
    let mut depth = 0;
    while depth < 32 {
        if imp.package_index == 0 {
            // Reached the root — the class_package is the .upk stem the
            // engine looks up at load time.
            return Some(imp.object_name.clone());
        }
        if imp.package_index >= 0 {
            return None;
        }
        let next_idx = (-imp.package_index - 1) as usize;
        imp = pkg.imports.get(next_idx)?;
        depth += 1;
    }
    None
}

/// Resolve an export's `Archetype` field to a readable
/// `<package>.<object>` string, for the coverage report's prefab
/// accounting. `None` when the export is not archetype-instanced.
///
/// A negative archetype index is an **import** — the template lives in
/// another package (e.g. `Em-Props.upk:EM-WallLight02_Pf0`), which is
/// precisely the case the Phase 1.2 walker cannot follow.
pub fn archetype_label(pkg: &Package, archetype: i32) -> Option<String> {
    if archetype == 0 {
        return None;
    }
    if archetype < 0 {
        let imp = pkg.imports.get((-archetype - 1) as usize)?;
        let package = top_package_name(pkg, imp).unwrap_or_else(|| "<unrooted>".to_string());
        Some(format!("{package}.{}", imp.object_name))
    } else {
        let exp = pkg.exports.get((archetype - 1) as usize)?;
        Some(format!("<local>.{}", exp.object_name))
    }
}

/// `true` when the export's `Outer` chain passes through an export whose
/// class is `PrefabInstance`.
///
/// This is the direct test for "does a PrefabInstance *own* this actor
/// in the export table", as opposed to "was this actor instanced from a
/// prefab archetype" (which [`archetype_label`] answers).
pub fn has_prefab_instance_outer(pkg: &Package, mut outer: i32) -> bool {
    let mut depth = 0;
    while outer != 0 && depth < 32 {
        if outer < 0 {
            // An import outer is the containing package, never a
            // PrefabInstance actor.
            return false;
        }
        let Some(exp) = pkg.exports.get((outer - 1) as usize) else {
            return false;
        };
        if pkg.export_class_name(exp) == "PrefabInstance" {
            return true;
        }
        outer = exp.package_index;
        depth += 1;
    }
    false
}

// ----- property lookup helpers (copies of the actor.rs helpers; we
// don't re-export those from cimmeria_upk because their signatures
// return owned data shapes we don't share) -----

pub fn find_vector(props: &[TaggedProperty], name: &str) -> Option<[f32; 3]> {
    props.iter().find(|p| p.name == name).and_then(|p| {
        if let PropValue::Vector { x, y, z } = &p.value {
            Some([*x, *y, *z])
        } else {
            None
        }
    })
}

pub fn find_rotator(props: &[TaggedProperty], name: &str) -> Option<[i32; 3]> {
    props.iter().find(|p| p.name == name).and_then(|p| {
        if let PropValue::Rotator { pitch, yaw, roll } = &p.value {
            Some([*pitch, *yaw, *roll])
        } else {
            None
        }
    })
}

pub fn find_float(props: &[TaggedProperty], name: &str) -> Option<f32> {
    props.iter().find(|p| p.name == name).and_then(|p| {
        if let PropValue::Float(v) = &p.value {
            Some(*v)
        } else {
            None
        }
    })
}

pub fn find_object(props: &[TaggedProperty], name: &str) -> Option<i32> {
    props.iter().find(|p| p.name == name).and_then(|p| {
        if let PropValue::Object(v) = &p.value {
            Some(*v)
        } else {
            None
        }
    })
}
