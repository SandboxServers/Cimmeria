//! Prefab-archetype fixtures — the other half of
//! [`super::chunk_fixtures::add_archetype_stub_actor`].
//!
//! `add_archetype_stub_actor` builds the *unresolvable* stub: an actor
//! whose export `Archetype` points at an import that leads nowhere in
//! particular. To exercise [`crate::staticmesh::archetype`] end to end
//! the chunk needs a stub whose archetype resolves to a real template
//! in a real second package, so this module adds:
//!
//! - [`prefab_package`], which writes the `.upk` a cooked prefab looks
//!   like — `Prefab` > `StaticMeshActor` > `StaticMeshComponent0`,
//!   nested through the export table's `Outer` chain, with the
//!   `StaticMesh` reference on the component;
//! - [`ChunkFixture::add_prefab_instanced_actor`], which writes the
//!   chunk-side instance whose two archetype fields point into it.
//!
//! The shapes here are traced from `Castle-000a0002` +
//! `Em-Props.upk`: the actor archetype is `Pkg.Prefab.Arc`, the
//! component archetype is `Pkg.Prefab.Arc.StaticMeshComponent0`, the
//! instance component carries only `CachedCullDistance`, and the
//! template actor is where `bCollideActors` lives.
//!
//! Added by `nav-archetype`; nothing in `chunk_fixtures` is modified.

use std::path::{Path, PathBuf};

use super::chunk_fixtures::{ChunkFixture, ACTOR_PROPS_OFFSET, COMPONENT_PROPS_OFFSET};
use super::package_bytes::PackageBuilder;
use super::static_mesh_payload::StaticMeshPayload;

/// Where a prefab template component's `StaticMesh` reference points.
#[derive(Debug, Clone, Copy)]
pub enum TemplateMesh<'a> {
    /// A `StaticMesh` export in the prefab's own package — the
    /// `Em-Props.EM-ComputerTower00` shape, and the common one.
    Local(&'a str),
    /// An import into a third package — the
    /// `EM_Earth_Military` → `SGW_Weather:DoorwayPrecipitationPlanes`
    /// shape.
    Imported(&'a str, &'a str),
    /// No `StaticMesh` property at all: the template is itself a stub
    /// and the walk must climb to `component_archetype`.
    None,
}

/// How to write one prefab `.upk`.
#[derive(Debug, Clone)]
pub struct PrefabSpec<'a> {
    /// File stem, and therefore the `PackageIndex` package key.
    pub package: &'a str,
    /// Top-level `Prefab` export name.
    pub prefab: &'a str,
    /// The `StaticMeshActor` template under it.
    pub arc: &'a str,
    pub mesh: TemplateMesh<'a>,
    /// `bCollideActors` on the **template actor**, where Castle's 26
    /// non-colliding prefabs set it.
    pub collide_actors: Option<bool>,
    /// `Rotation` on the template actor.
    pub rotation: Option<[i32; 3]>,
    /// `DrawScale3D` on the template actor.
    pub draw_scale_3d: Option<[f32; 3]>,
    /// When `mesh` is [`TemplateMesh::None`], the
    /// `(package, prefab, arc)` this template component's own
    /// `Archetype` points at — the two-level chain.
    pub component_archetype: Option<(&'a str, &'a str, &'a str)>,
}

impl<'a> PrefabSpec<'a> {
    /// The common case: one template actor whose component references a
    /// `StaticMesh` in the same package, collision left at the UE3
    /// default.
    pub fn local(package: &'a str, prefab: &'a str, arc: &'a str, mesh: &'a str) -> Self {
        Self {
            package,
            prefab,
            arc,
            mesh: TemplateMesh::Local(mesh),
            collide_actors: None,
            rotation: None,
            draw_scale_3d: None,
            component_archetype: None,
        }
    }

    pub fn with_collide_actors(mut self, v: bool) -> Self {
        self.collide_actors = Some(v);
        self
    }

    pub fn with_rotation(mut self, v: [i32; 3]) -> Self {
        self.rotation = Some(v);
        self
    }

    pub fn with_draw_scale_3d(mut self, v: [f32; 3]) -> Self {
        self.draw_scale_3d = Some(v);
        self
    }
}

/// Write `<dir>/<package>.upk` holding the prefab template graph, plus
/// (for [`TemplateMesh::Local`]) the `StaticMesh` it references.
///
/// `mesh_payload` is only used by the `Local` variant.
pub fn prefab_package(
    dir: &Path,
    spec: &PrefabSpec<'_>,
    mesh_payload: &StaticMeshPayload,
) -> PathBuf {
    let mut pkg = PackageBuilder::new();

    let prefab_class = pkg.class_ref("Prefab");
    let prefab = pkg.add_export(prefab_class, 0, spec.prefab);

    let actor_class = pkg.class_ref("StaticMeshActor");
    let arc = pkg.add_export(actor_class, prefab, spec.arc);

    let component_class = pkg.class_ref("StaticMeshComponent");
    // Every SGW prefab names it exactly this, which is why the resolver
    // cannot key on the object name alone: `Em-Props.upk` holds 218 of
    // them.
    let component = pkg.add_export(component_class, arc, "StaticMeshComponent0");

    // The mesh reference, resolved the way the template would hold it.
    let mesh_ref = match spec.mesh {
        TemplateMesh::Local(name) => {
            let mesh_class = pkg.class_ref("StaticMesh");
            let export = pkg.add_export(mesh_class, 0, name);
            pkg.set_payload(export, mesh_payload.encode());
            Some(export)
        }
        TemplateMesh::Imported(other_pkg, other_obj) => {
            let package_import = pkg.add_import("Core", "Package", 0, other_pkg);
            Some(pkg.add_import("Engine", "StaticMesh", package_import, other_obj))
        }
        TemplateMesh::None => None,
    };

    // Template actor body: the placement/collision properties the
    // instance inherits. `CollisionComponent` mirrors the real data,
    // where the template actor has that and *not* `StaticMeshComponent`
    // — which is why the resolver follows the component's archetype
    // rather than the actor's.
    let mut actor_body = vec![0u8; ACTOR_PROPS_OFFSET];
    {
        let mut props = pkg.props();
        props.name_value("Tag", "StaticMeshActor");
        if let Some(v) = spec.collide_actors {
            props.boolean("bCollideActors", v);
        }
        if let Some(v) = spec.rotation {
            props.rotator("Rotation", v);
        }
        if let Some(v) = spec.draw_scale_3d {
            props.vector("DrawScale3D", v);
        }
        props.object("CollisionComponent", component);
        actor_body.extend_from_slice(&props.finish());
    }
    pkg.set_payload(arc, actor_body);

    let mut component_body = vec![0u8; COMPONENT_PROPS_OFFSET];
    {
        let mut props = pkg.props();
        if let Some(m) = mesh_ref {
            props.object("StaticMesh", m);
        }
        props.boolean("bIsOwnerAStaticMeshActor", true);
        component_body.extend_from_slice(&props.finish());
    }
    pkg.set_payload(component, component_body);

    // A stub template climbs to its own archetype.
    if let Some((up_pkg, up_prefab, up_arc)) = spec.component_archetype {
        let package_import = pkg.add_import("Core", "Package", 0, up_pkg);
        let prefab_import = pkg.add_import("Engine", "Prefab", package_import, up_prefab);
        let arc_import = pkg.add_import("Engine", "StaticMeshActor", prefab_import, up_arc);
        let comp_import = pkg.add_import(
            "Engine",
            "StaticMeshComponent",
            arc_import,
            "StaticMeshComponent0",
        );
        pkg.set_archetype(component, comp_import);
    }

    let path = dir.join(format!("{}.upk", spec.package));
    pkg.write_to(&path).expect("write prefab package");
    path
}

/// Chunk-side instance of a prefab actor.
#[derive(Debug, Clone)]
pub struct PrefabInstanceSpec<'a> {
    pub name: &'a str,
    pub location: [f32; 3],
    /// `(package, prefab, arc)` — where both archetype fields point.
    pub template: (&'a str, &'a str, &'a str),
    /// A `StaticMesh` the *instance* component overrides the archetype
    /// with, as `(package, object)`. `None` writes the real cooked
    /// shape: a stub with only `CachedCullDistance`.
    pub instance_mesh: Option<(&'a str, &'a str)>,
    /// `bCollideActors` on the instance actor, overriding whatever the
    /// template says.
    pub instance_collide_actors: Option<bool>,
}

impl<'a> PrefabInstanceSpec<'a> {
    pub fn new(name: &'a str, location: [f32; 3], template: (&'a str, &'a str, &'a str)) -> Self {
        Self {
            name,
            location,
            template,
            instance_mesh: None,
            instance_collide_actors: None,
        }
    }

    pub fn with_instance_mesh(mut self, package: &'a str, object: &'a str) -> Self {
        self.instance_mesh = Some((package, object));
        self
    }

    pub fn with_instance_collide_actors(mut self, v: bool) -> Self {
        self.instance_collide_actors = Some(v);
        self
    }
}

impl ChunkFixture {
    /// Add the cooked form of one prefab actor: a `StaticMeshActor`
    /// outered straight to `PersistentLevel`, whose export `Archetype`
    /// points at `Pkg.Prefab.Arc` and whose component's `Archetype`
    /// points at `Pkg.Prefab.Arc.StaticMeshComponent0`.
    ///
    /// This is the shape [`prefab_package`] resolves against, and the
    /// shape all 961 of Castle's archetype-stub actors have.
    pub fn add_prefab_instanced_actor(&mut self, spec: &PrefabInstanceSpec<'_>) -> i32 {
        let (tpkg, tprefab, tarc) = spec.template;
        let level = self.level();
        let pkg = self.package_mut();

        let actor_class = pkg.class_ref("StaticMeshActor");
        let actor = pkg.add_export(actor_class, level, spec.name);
        let component_class = pkg.class_ref("StaticMeshComponent");
        let component = pkg.add_export(component_class, actor, "StaticMeshComponent");

        // Import chain into the prefab package, shared by both
        // archetype fields.
        let package_import = pkg.add_import("Core", "Package", 0, tpkg);
        let prefab_import = pkg.add_import("Engine", "Prefab", package_import, tprefab);
        let arc_import = pkg.add_import("Engine", "StaticMeshActor", prefab_import, tarc);
        let comp_import = pkg.add_import(
            "Engine",
            "StaticMeshComponent",
            arc_import,
            "StaticMeshComponent0",
        );
        pkg.set_archetype(actor, arc_import);
        pkg.set_archetype(component, comp_import);

        let instance_mesh_import = spec.instance_mesh.map(|(p, o)| {
            let pi = pkg.add_import("Core", "Package", 0, p);
            pkg.add_import("Engine", "StaticMesh", pi, o)
        });

        let mut actor_body = vec![0u8; ACTOR_PROPS_OFFSET];
        {
            let mut props = pkg.props();
            props.vector("Location", spec.location);
            if let Some(v) = spec.instance_collide_actors {
                props.boolean("bCollideActors", v);
            }
            props.object("StaticMeshComponent", component);
            props.object("CollisionComponent", component);
            actor_body.extend_from_slice(&props.finish());
        }
        pkg.set_payload(actor, actor_body);

        let mut component_body = vec![0u8; COMPONENT_PROPS_OFFSET];
        {
            let mut props = pkg.props();
            if let Some(m) = instance_mesh_import {
                props.object("StaticMesh", m);
            }
            // What a real cooked stub carries, and nothing else.
            props.float("CullDistance", 16000.0);
            props.float("CachedCullDistance", 16000.0);
            component_body.extend_from_slice(&props.finish());
        }
        pkg.set_payload(component, component_body);

        actor
    }
}
