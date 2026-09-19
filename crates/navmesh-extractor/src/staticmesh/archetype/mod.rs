//! Prefab-archetype resolution for cooked `StaticMeshActor`s.
//!
//! # The shape of the problem
//!
//! When the SGW cooker writes a `PrefabInstance` out to a chunk, each
//! actor of the prefab becomes an ordinary `StaticMeshActor` export
//! outered straight to `PersistentLevel` — but its
//! `StaticMeshComponent` export is written as a **stub**: a tagged-
//! property block holding only per-instance overrides (`CullDistance`,
//! `IrrelevantLights`, `BlockRigidBody`, …). The `StaticMesh` reference
//! is *not* in it. UE3 doesn't need it there: the export table's
//! `Archetype` field points at the template inside the prefab's own
//! package, and property lookup falls through to the archetype for
//! anything the instance doesn't override.
//!
//! There are **two** archetype chains per actor and they answer
//! different questions:
//!
//! | chain | rooted at | answers |
//! |---|---|---|
//! | component | `ExportEntry::archetype` of the `StaticMeshComponent` | which `StaticMesh`? |
//! | actor | `ExportEntry::archetype` of the `StaticMeshActor` | does it collide? what rotation / scale? |
//!
//! Measured on `Castle-000a0002`: the *actor* archetype
//! (`Em-Props.EM-ComputerTower00_Pf0.EM-ComputerTower00_Pf0_Arc1`)
//! carries no `StaticMeshComponent` property at all — only
//! `CollisionComponent` — so following it for the mesh is a dead end.
//! The *component* archetype (`…_Arc1.StaticMeshComponent0`) is the one
//! that holds `StaticMesh`.
//!
//! ```text
//! chunk export  StaticMeshComponent  (stub, no StaticMesh)
//!      │ ExportEntry::archetype  (negative => import)
//!      ▼
//! import chain  Em-Props . EM-ComputerTower00_Pf0
//!                        . EM-ComputerTower00_Pf0_Arc1
//!                        . StaticMeshComponent0
//!      │ PackageIndex::find("Em-Props", "EM-ComputerTower00_Pf0") -> file
//!      ▼
//! Em-Props.upk export whose full path is
//!   "EM-ComputerTower00_Pf0.EM-ComputerTower00_Pf0_Arc1.StaticMeshComponent0"
//!      │ its tagged properties (component prefix, offset 8)
//!      ▼
//! StaticMesh = Obj(1541) -> Em-Props:EM-ComputerTower00
//! ```
//!
//! If a template is itself a stub the walk continues up *its*
//! `Archetype`, bounded by [`MAX_ARCHETYPE_DEPTH`] and a visited set —
//! UE3 forbids archetype cycles, but a corrupt or hand-edited package
//! could still present one and an unbounded walk would hang the whole
//! extraction.
//!
//! # `bCollideActors` — the reason this matters beyond coverage
//!
//! 26 of Castle's 86 archetype actors set `bCollideActors = false`,
//! and 17 of those are `bHidden = true` with `Group = PrecipPlanes`:
//! flat cards the artist dropped into tent and bunker doorways so snow
//! renders there. They have real kDOP collision data in their
//! `StaticMesh` — the cook does not strip it — so nothing downstream
//! can tell them apart from a wall. Only the actor's own
//! `bCollideActors`, inherited from the prefab template, says they are
//! not solid.
//!
//! Emitting them is not a cosmetic error. They sit in doorways with
//! zero walkable area, so Recast rasterises them as pure obstacle: the
//! measured effect on `castle.nav` was the exterior splitting from one
//! component into three and the `gate_room_dhd` probe losing its floor
//! entirely. That is why [`ActorArchetypeProps::collides`] gates
//! emission rather than merely being reported.
//!
//! # What is *not* inherited
//!
//! `Location`. A prefab template actor's `Location` is its offset
//! **inside the prefab** (`(128, -2031.99, 0)`), not a world position;
//! inheriting it would teleport the instance to that offset from the
//! world origin. Measured across all 144 Castle chunks: every instance
//! actor carries its own `Location`, so the question never arises —
//! and [`ActorArchetypeProps`] deliberately has no `location` field so
//! it cannot arise by accident later.
//!
//! Component-local `Translation` / `Rotation` / `Scale` / `Scale3D`
//! are likewise absent from every Castle component, instance and
//! template alike, so the actor transform remains the whole story. The
//! `archetype_census` binary re-measures both claims, so a map that
//! behaves differently surfaces rather than silently shifting meshes.

use std::collections::HashMap;

use cimmeria_upk::{ExportEntry, Package, PropValue, TaggedProperty};
use cimmeria_upk_objects::PackageIndex;

use crate::coverage::SkipReason;
use crate::staticmesh::mesh_ref::{find_object, MeshRefResult};

pub mod chain;
pub mod props;
#[cfg(test)]
mod tests;

pub use chain::{walk_actor_chain, walk_mesh_chain, ActorChainOutcome, ActorProbe, TemplateProbe};
pub use props::{ActorArchetypeProps, ActorArchetypeResolution};

/// How many `Archetype` hops to follow before declaring the chain
/// pathological. Real SGW prefabs resolve in exactly one hop; the
/// budget exists so a cyclic or absurdly deep chain fails loudly
/// instead of spinning.
pub const MAX_ARCHETYPE_DEPTH: usize = 8;

/// Byte offset of the tagged-property block inside a cooked
/// `*Component` export — an 8-byte binary prefix, not the 4-byte one
/// non-component objects use. Same constant the direct path uses; see
/// `mesh_ref::resolve_mesh_ref_from_component`.
const COMPONENT_PROP_OFFSET: usize = 8;

/// Byte offset of the tagged-property block inside a cooked `AActor`
/// export. Actors carry a 32-byte binary prefix.
const ACTOR_PROP_OFFSET: usize = 32;

/// Cross-chunk memo of archetype-path -> outcome.
///
/// Keyed by the full dotted path (`Em-Props.EM-ComputerTower00_Pf0.…`)
/// rather than by `(chunk, export)` because 961 stub actors across
/// Castle share only 86 distinct archetype paths: the memo turns 961
/// package opens into 86.
///
/// Deliberately holds no open [`Package`] — those live in
/// [`OpenPrefabs`], which is per chunk, so peak memory is one chunk's
/// prefab working set (Castle: at most 5 packages) rather than every
/// prefab package the map touches.
#[derive(Debug, Default)]
pub struct ArchetypeCache {
    mesh_refs: HashMap<String, MeshRefResult>,
    actor_props: HashMap<String, ActorArchetypeResolution>,
    hits: u64,
    misses: u64,
}

impl ArchetypeCache {
    /// Memo hits / misses across both chains, for the extraction log.
    pub fn stats(&self) -> (u64, u64) {
        (self.hits, self.misses)
    }

    /// Distinct component-archetype paths resolved so far.
    pub fn len(&self) -> usize {
        self.mesh_refs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.mesh_refs.is_empty()
    }

    /// Distinct actor-archetype paths resolved so far.
    pub fn actor_paths(&self) -> usize {
        self.actor_props.len()
    }
}

/// A prefab package opened for the duration of one chunk, with its
/// export table pre-indexed by full outer path.
struct LoadedPackage {
    pkg: Package,
    by_path: HashMap<String, usize>,
}

impl LoadedPackage {
    fn open(path: &std::path::Path) -> Option<Self> {
        let pkg = Package::open(path).ok()?;
        let mut by_path = HashMap::with_capacity(pkg.exports.len());
        for i in 0..pkg.exports.len() {
            by_path.insert(export_outer_path(&pkg, i), i);
        }
        Some(Self { pkg, by_path })
    }
}

/// Per-chunk scratch: prefab packages opened while resolving this
/// chunk's archetypes. Dropped when the chunk is done.
#[derive(Default)]
pub struct OpenPrefabs {
    packages: HashMap<String, Option<LoadedPackage>>,
}

impl OpenPrefabs {
    /// How many distinct prefab packages this chunk had to open.
    pub fn opened(&self) -> usize {
        self.packages.values().filter(|p| p.is_some()).count()
    }

    /// Locate and open the package a rooted chain names, memoised.
    ///
    /// The chain's first component after the package name is always a
    /// unique top-level object, so the existing `(package, object)`
    /// index answers "which file" exactly. [`PackageIndex::package_file`]
    /// is the fallback for the case where that object was skipped by
    /// the index builder (`serial_size <= 0`).
    fn get(&mut self, chain: &[String], index: &PackageIndex) -> Option<&LoadedPackage> {
        let root = &chain[0];
        self.packages
            .entry(root.clone())
            .or_insert_with(|| {
                let file = index
                    .find(root, &chain[1])
                    .map(|loc| loc.file_path.clone())
                    .or_else(|| index.package_file(root).map(|p| p.to_path_buf()))?;
                LoadedPackage::open(&file)
            })
            .as_ref()
    }
}

/// Dotted outer path of an export **within its own package**, e.g.
/// `EM-ComputerTower00_Pf0.EM-ComputerTower00_Pf0_Arc1.StaticMeshComponent0`.
///
/// Stops at the first non-export ancestor, so the result is directly
/// comparable to the tail of an import chain (which drops the root
/// package name the same way).
pub fn export_outer_path(pkg: &Package, exp_idx: usize) -> String {
    let mut parts: Vec<&str> = Vec::new();
    let mut idx = exp_idx as i32 + 1;
    let mut depth = 0;
    while idx > 0 && depth < 32 {
        let Some(e) = pkg.exports.get((idx - 1) as usize) else {
            break;
        };
        parts.push(e.object_name.as_str());
        idx = e.package_index;
        depth += 1;
    }
    parts.reverse();
    parts.join(".")
}

/// Names along an **import** reference's outer chain, root package
/// first: `["Em-Props", "EM-ComputerTower00_Pf0", …,
/// "StaticMeshComponent0"]`.
///
/// `None` when the reference is not an import, when the chain runs off
/// the table, or when it fails to terminate in a root
/// (`package_index == 0`) import — in every case there is no package
/// name to look up.
pub fn import_chain(pkg: &Package, mut idx: i32) -> Option<Vec<String>> {
    if idx >= 0 {
        return None;
    }
    let mut parts: Vec<String> = Vec::new();
    let mut depth = 0;
    while idx < 0 && depth < 32 {
        let imp = pkg.imports.get((-idx - 1) as usize)?;
        parts.push(imp.object_name.clone());
        idx = imp.package_index;
        depth += 1;
    }
    // A rooted chain terminates at 0. A positive terminator would mean
    // an import outered to an export, which UE3 never writes.
    if idx != 0 {
        return None;
    }
    parts.reverse();
    Some(parts)
}

/// `bCollideActors` / `CollideActors` as a plain bool, if the block
/// sets it at all.
pub fn find_bool(props: &[TaggedProperty], name: &str) -> Option<bool> {
    props.iter().find(|p| p.name == name).and_then(|p| {
        if let PropValue::Bool(b) = p.value {
            Some(b)
        } else {
            None
        }
    })
}

/// Resolve the properties a `StaticMeshActor` inherits from its actor
/// archetype chain.
///
/// `actor_archetype` is the actor export's `Archetype` field. A
/// non-zero archetype that cannot be followed is reported as
/// [`ActorArchetypeResolution::Unresolved`] rather than folded into an
/// all-`None` success — see that type's doc for why the distinction is
/// load-bearing.
pub fn resolve_actor_archetype(
    pkg: &Package,
    actor_archetype: i32,
    index: &PackageIndex,
    cache: &mut ArchetypeCache,
    open: &mut OpenPrefabs,
) -> ActorArchetypeResolution {
    if actor_archetype == 0 {
        return ActorArchetypeResolution::NotInstanced;
    }
    // A positive index is a template in this same package and a
    // negative one that never reaches a root import has no package to
    // look up; neither is followed here, and neither may be assumed
    // harmless.
    let Some(chain) = import_chain(pkg, actor_archetype) else {
        return ActorArchetypeResolution::Unresolved(SkipReason::ArchetypeUnrooted);
    };
    if chain.len() < 2 {
        return ActorArchetypeResolution::Unresolved(SkipReason::ArchetypeUnrooted);
    }
    let key = chain.join(".");
    if let Some(hit) = cache.actor_props.get(&key) {
        cache.hits += 1;
        return *hit;
    }
    cache.misses += 1;
    let outcome = chain::walk_actor_chain(&chain, &mut |c| read_actor_template(c, index, open));
    let resolution = match outcome.unreadable {
        Some(reason) => ActorArchetypeResolution::Unresolved(reason),
        None => ActorArchetypeResolution::Resolved(outcome.props),
    };
    cache.actor_props.insert(key, resolution);
    resolution
}

/// Read one template actor out of its package, for
/// [`chain::walk_actor_chain`].
fn read_actor_template(
    chain: &[String],
    index: &PackageIndex,
    open: &mut OpenPrefabs,
) -> Result<ActorProbe, SkipReason> {
    let path = chain[1..].join(".");
    let loaded = open
        .get(chain, index)
        .ok_or(SkipReason::ArchetypePackageNotFound)?;
    let idx = *loaded
        .by_path
        .get(&path)
        .ok_or(SkipReason::ArchetypeExportNotFound)?;
    let tmpl = &loaded.pkg.exports[idx];
    let data = loaded
        .pkg
        .read_export_data(tmpl)
        .map_err(|_| SkipReason::ArchetypeExportNotFound)?;
    let props = cimmeria_upk::parse_tagged_properties(&data, ACTOR_PROP_OFFSET, &loaded.pkg.names);
    Ok(ActorProbe {
        props: ActorArchetypeProps::from_props(&props),
        next: import_chain(&loaded.pkg, tmpl.archetype),
    })
}

/// Resolve a stub component's `StaticMesh` by walking its archetype
/// chain.
///
/// `component` is the **instance** component export (the stub) in
/// `pkg`. Returns the same `(package_name, object_name)` key shape the
/// direct path returns, so the caller feeds both into the one mesh
/// loader.
pub fn resolve_via_archetype(
    pkg: &Package,
    component: &ExportEntry,
    index: &PackageIndex,
    cache: &mut ArchetypeCache,
    open: &mut OpenPrefabs,
) -> MeshRefResult {
    // The stub may carry a component-level collision override of its
    // own even though it has no mesh; check before paying for the walk.
    if let Ok(data) = pkg.read_export_data(component) {
        let props = cimmeria_upk::parse_tagged_properties(&data, COMPONENT_PROP_OFFSET, &pkg.names);
        if find_bool(&props, "CollideActors") == Some(false) {
            return Err(SkipReason::CollisionDisabled);
        }
    }

    let chain = match import_chain(pkg, component.archetype) {
        Some(chain) => chain,
        // A positive archetype is a template in this same package:
        // rare in cooked chunks, since the cooker imports prefab
        // templates, but the editor writes it for a prefab instanced
        // from a template in the same map.
        None if component.archetype > 0 => match walk_local_chain(pkg, component.archetype)? {
            LocalWalk::Mesh(mesh) => return mesh,
            LocalWalk::Import(next) => next,
        },
        None => return Err(SkipReason::ArchetypeUnrooted),
    };
    if chain.len() < 2 {
        // A bare package reference with no object under it.
        return Err(SkipReason::ArchetypeUnrooted);
    }

    let key = chain.join(".");
    if let Some(hit) = cache.mesh_refs.get(&key) {
        cache.hits += 1;
        return hit.clone();
    }
    cache.misses += 1;
    let outcome = chain::walk_mesh_chain(&chain, &mut |c| read_mesh_template(c, index, open));
    cache.mesh_refs.insert(key, outcome.clone());
    outcome
}

/// Where a walk through same-package templates ended up.
enum LocalWalk {
    /// A template carried a `StaticMesh`.
    Mesh(MeshRefResult),
    /// A template had no mesh but pointed at an import chain — hand off
    /// to [`chain::walk_mesh_chain`] from there.
    Import(Vec<String>),
}

/// Follow a chain of same-package template components to a mesh or to
/// the point where it leaves this package.
///
/// [`import_chain`] returns `None` for *every* positive index, so
/// reading only `TemplateProbe::next` stops dead at a local stub whose
/// own `Archetype` is another local export — the walk returns
/// `ArchetypeNoMesh` one hop short of the parent template that actually
/// holds the mesh. Bounded by [`MAX_ARCHETYPE_DEPTH`] and a visited set
/// for the same reason the import walk is.
fn walk_local_chain(pkg: &Package, mut archetype: i32) -> Result<LocalWalk, SkipReason> {
    let mut visited: Vec<i32> = Vec::new();
    for _ in 0..MAX_ARCHETYPE_DEPTH {
        if visited.contains(&archetype) {
            return Err(SkipReason::ArchetypeChainLoop);
        }
        visited.push(archetype);

        let probe = local_template_probe(pkg, archetype)?;
        if probe.collide_actors == Some(false) {
            return Err(SkipReason::CollisionDisabled);
        }
        if let Some(mesh) = probe.mesh {
            return Ok(LocalWalk::Mesh(mesh));
        }
        if let Some(next) = probe.next {
            return Ok(LocalWalk::Import(next));
        }
        // No mesh and no import chain: climb this template's own
        // `Archetype` if it too is a local export.
        let parent = pkg
            .exports
            .get((archetype - 1) as usize)
            .ok_or(SkipReason::ArchetypeExportNotFound)?
            .archetype;
        if parent <= 0 {
            return Err(SkipReason::ArchetypeNoMesh);
        }
        archetype = parent;
    }
    Err(SkipReason::ArchetypeChainLoop)
}

/// Probe a template component that lives in this same package.
fn local_template_probe(pkg: &Package, archetype: i32) -> Result<TemplateProbe, SkipReason> {
    let tmpl = pkg
        .exports
        .get((archetype - 1) as usize)
        .ok_or(SkipReason::ArchetypeExportNotFound)?;
    let data = pkg
        .read_export_data(tmpl)
        .map_err(|_| SkipReason::ArchetypeExportNotFound)?;
    let props = cimmeria_upk::parse_tagged_properties(&data, COMPONENT_PROP_OFFSET, &pkg.names);
    Ok(TemplateProbe {
        // `own_package` is empty, which `mesh_key_in` turns into the
        // "local to the package we are already holding" key shape —
        // the same conclusion the direct path reaches, and the one
        // `staticmesh::load_local_static_mesh` knows how to decode.
        mesh: find_object(&props, "StaticMesh").map(|m| match m {
            0 => Err(SkipReason::NullMeshRef),
            m => mesh_key_in(pkg, "", m),
        }),
        collide_actors: find_bool(&props, "CollideActors"),
        next: import_chain(pkg, tmpl.archetype),
    })
}

/// Read one template component out of its package, for
/// [`chain::walk_mesh_chain`].
fn read_mesh_template(
    chain: &[String],
    index: &PackageIndex,
    open: &mut OpenPrefabs,
) -> Result<TemplateProbe, SkipReason> {
    let path = chain[1..].join(".");
    let loaded = open
        .get(chain, index)
        .ok_or(SkipReason::ArchetypePackageNotFound)?;
    let idx = *loaded
        .by_path
        .get(&path)
        .ok_or(SkipReason::ArchetypeExportNotFound)?;
    let tmpl = &loaded.pkg.exports[idx];
    let data = loaded
        .pkg
        .read_export_data(tmpl)
        .map_err(|_| SkipReason::ArchetypeExportNotFound)?;
    let props =
        cimmeria_upk::parse_tagged_properties(&data, COMPONENT_PROP_OFFSET, &loaded.pkg.names);
    Ok(TemplateProbe {
        mesh: find_object(&props, "StaticMesh").map(|m| match m {
            0 => Err(SkipReason::NullMeshRef),
            m => mesh_key_in(&loaded.pkg, &chain[0], m),
        }),
        collide_actors: find_bool(&props, "CollideActors"),
        next: import_chain(&loaded.pkg, tmpl.archetype),
    })
}

/// Turn a `StaticMesh` object index, as seen from inside `pkg`, into
/// the `(package_name, object_name)` key the mesh loader wants.
///
/// `own_package` is `pkg`'s own name, used when the reference is a
/// local export. It is **empty** when `pkg` is the chunk itself, which
/// is the loader's sentinel for "decode this from the package already
/// open" — see [`crate::staticmesh::mesh_ref::is_local`].
fn mesh_key_in(pkg: &Package, own_package: &str, mesh_obj: i32) -> MeshRefResult {
    if mesh_obj < 0 {
        let chain = import_chain(pkg, mesh_obj).ok_or(SkipReason::UnresolvableMeshRef)?;
        if chain.len() < 2 {
            return Err(SkipReason::UnresolvableMeshRef);
        }
        let last = chain[chain.len() - 1].clone();
        Ok((chain[0].clone(), last))
    } else {
        let exp = pkg
            .exports
            .get((mesh_obj - 1) as usize)
            .ok_or(SkipReason::UnresolvableMeshRef)?;
        if pkg.export_class_name(exp) != "StaticMesh" {
            // A `StaticMesh` property pointing at something that is not
            // one: decoding its bytes would produce a plausible-looking
            // mesh out of an unrelated export.
            return Err(SkipReason::UnresolvableMeshRef);
        }
        Ok((own_package.to_string(), exp.object_name.clone()))
    }
}
