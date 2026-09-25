//! Whole-chunk composition: the object graph a cooked `.umap` has.
//!
//! [`ChunkFixture`] starts with a `Level` export named
//! `PersistentLevel`, because that is what
//! [`crate::bsp::classify_owner`] keys the world-geometry decision on,
//! and grows a chunk one actor at a time. [`write`](ChunkFixture::write)
//! names the file `<Map>-<hex8>.umap`, which is the only shape
//! [`crate::chunk_id::ChunkId::from_umap_path`] accepts.
//!
//! [`mesh_package`] writes the other half of the `StaticMeshActor`
//! chain — a separate `.upk` holding the `StaticMesh` itself — so a
//! test can build a real [`cimmeria_upk_objects::PackageIndex`] over a
//! temp directory and exercise the cross-package resolution path that
//! otherwise needs the client tree.

use std::path::{Path, PathBuf};

use cimmeria_upk_objects::PackageIndex;

use super::model_payload::ModelPayload;
use super::package_bytes::PackageBuilder;
use super::static_mesh_payload::StaticMeshPayload;
use super::terrain_payload::TerrainPayload;

/// Bytes before an `AActor`'s property stream. Matches
/// `bsp::ACTOR_PROPS_OFFSET` and the offset the StaticMesh walker uses.
pub const ACTOR_PROPS_OFFSET: usize = 32;

/// Bytes before an `ActorComponent`'s property stream.
pub const COMPONENT_PROPS_OFFSET: usize = 8;

/// Where a synthetic actor sits, and how it is scaled.
///
/// One named struct rather than five positional parameters repeated on
/// every builder: `[0.0; 3], [0; 3], 1.0, [1.0; 3], None` says nothing
/// at a call site, and the two builders that took it were only under
/// the argument-count lint by suppression.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    pub location: [f32; 3],
    /// UE3 rotator units (65536 per turn), `[pitch, yaw, roll]`.
    pub rotation: [i32; 3],
    pub draw_scale: f32,
    pub draw_scale_3d: [f32; 3],
    /// CSG pivot offset. Only a `Brush`-style owner carries one; `None`
    /// omits the property entirely, which is what a plain actor does.
    pub pre_pivot: Option<[f32; 3]>,
}

impl Default for Placement {
    /// Origin, no rotation, unit scale, no `PrePivot` — the UE3 cooked
    /// defaults, i.e. the properties the cooker would have omitted.
    fn default() -> Self {
        Self {
            location: [0.0; 3],
            rotation: [0; 3],
            draw_scale: 1.0,
            draw_scale_3d: [1.0; 3],
            pre_pivot: None,
        }
    }
}

impl Placement {
    /// Defaults but at `location`.
    pub fn at(location: [f32; 3]) -> Self {
        Self {
            location,
            ..Self::default()
        }
    }

    pub fn with_rotation(mut self, rotation: [i32; 3]) -> Self {
        self.rotation = rotation;
        self
    }

    pub fn with_draw_scale(mut self, draw_scale: f32) -> Self {
        self.draw_scale = draw_scale;
        self
    }

    pub fn with_draw_scale_3d(mut self, draw_scale_3d: [f32; 3]) -> Self {
        self.draw_scale_3d = draw_scale_3d;
        self
    }

    pub fn with_pre_pivot(mut self, pre_pivot: [f32; 3]) -> Self {
        self.pre_pivot = Some(pre_pivot);
        self
    }
}

/// A synthetic cooked map chunk.
pub struct ChunkFixture {
    pkg: PackageBuilder,
    level: i32,
}

impl Default for ChunkFixture {
    fn default() -> Self {
        Self::new()
    }
}

impl ChunkFixture {
    /// A chunk holding only its `PersistentLevel`.
    pub fn new() -> Self {
        let mut pkg = PackageBuilder::new();
        let level_class = pkg.class_ref("Level");
        let level = pkg.add_export(level_class, 0, "PersistentLevel");
        Self { pkg, level }
    }

    /// The `PersistentLevel` export index, for use as an `Outer`.
    pub fn level(&self) -> i32 {
        self.level
    }

    /// Escape hatch for a fixture shape this module does not model.
    pub fn package_mut(&mut self) -> &mut PackageBuilder {
        &mut self.pkg
    }

    /// Add a `Terrain` actor owned by the level.
    pub fn add_terrain(&mut self, terrain: &TerrainPayload) -> i32 {
        let class = self.pkg.class_ref("Terrain");
        let n = self.pkg.export_count();
        let export = self
            .pkg
            .add_export(class, self.level, &format!("Terrain_{n}"));
        let body = terrain.encode(&mut self.pkg);
        self.pkg.set_payload(export, body);
        export
    }

    /// Add the level's own world-space CSG `Model`.
    pub fn add_level_model(&mut self, model: &ModelPayload) -> i32 {
        let class = self.pkg.class_ref("Model");
        let export = self.pkg.add_export(class, self.level, "Model_Persistent");
        self.pkg.set_payload(export, model.encode());
        export
    }

    /// Add a `Model` owned by a newly created actor of `owner_class`,
    /// placed by the supplied transform and `PrePivot`.
    ///
    /// This is the path that distinguishes `Brush` (included),
    /// `TriggerVolume` (excluded) and an unknown `*Volume` (excluded and
    /// reported) — the owner's *class* is the whole classifier.
    pub fn add_owned_model(
        &mut self,
        owner_class: &str,
        owner_name: &str,
        placement: Placement,
        model: &ModelPayload,
    ) -> (i32, i32) {
        let owner = self.add_placed_actor(owner_class, owner_name, placement);
        let model_class = self.pkg.class_ref("Model");
        let model_export = self
            .pkg
            .add_export(model_class, owner, &format!("Model_{owner_name}"));
        self.pkg.set_payload(model_export, model.encode());
        (owner, model_export)
    }

    /// Add a `Model` whose owner export body is deliberately malformed:
    /// the placement properties are truncated mid-tag, so the
    /// tagged-property parser stops early and yields a partial list.
    ///
    /// The point of the fixture is that `Location` is *present and
    /// correct* before the truncation, so a walker that silently falls
    /// back to identity defaults still places the model somewhere
    /// plausible — the failure this shape is meant to catch is exactly
    /// the one that does not look like a failure.
    pub fn add_owned_model_with_truncated_owner_props(
        &mut self,
        owner_class: &str,
        owner_name: &str,
        location: [f32; 3],
        model: &ModelPayload,
    ) -> (i32, i32) {
        let owner_class_ref = self.pkg.class_ref(owner_class);
        let owner = self.pkg.add_export(owner_class_ref, self.level, owner_name);

        // A `DrawScale` tag whose declared payload size runs far past
        // the end of the body. The parser consumes `Location`, reaches
        // this tag, finds the value truncated, and stops — leaving a
        // partial property list with no `None` terminator.
        let float_type = self.pkg.intern("FloatProperty");
        let scale_name = self.pkg.intern("DrawScale");
        let mut body = vec![0u8; ACTOR_PROPS_OFFSET];
        let mut props = self.pkg.props();
        props.vector("Location", location);
        props.raw(&scale_name.to_le_bytes());
        props.raw(&0i32.to_le_bytes());
        props.raw(&float_type.to_le_bytes());
        props.raw(&0i32.to_le_bytes());
        props.raw(&0x7000i32.to_le_bytes()); // declared size
        props.raw(&0i32.to_le_bytes()); // array index
        body.extend_from_slice(&props.finish_unterminated());
        self.pkg.set_payload(owner, body);

        let model_class = self.pkg.class_ref("Model");
        let model_export = self
            .pkg
            .add_export(model_class, owner, &format!("Model_{owner_name}"));
        self.pkg.set_payload(model_export, model.encode());
        (owner, model_export)
    }

    /// Add a `Model` owned by nothing — the package-root editor builder
    /// brush.
    pub fn add_builder_brush_model(&mut self, model: &ModelPayload) -> i32 {
        let class = self.pkg.class_ref("Model");
        let export = self.pkg.add_export(class, 0, "Model_Builder");
        self.pkg.set_payload(export, model.encode());
        export
    }

    /// Add a placed actor of any class, with the four placement
    /// properties and optionally a `PrePivot`.
    pub fn add_placed_actor(&mut self, class_name: &str, name: &str, at: Placement) -> i32 {
        let class = self.pkg.class_ref(class_name);
        let export = self.pkg.add_export(class, self.level, name);
        let mut body = vec![0u8; ACTOR_PROPS_OFFSET];
        let mut props = self.pkg.props();
        props.placement(at.location, at.rotation, at.draw_scale, at.draw_scale_3d);
        if let Some(p) = at.pre_pivot {
            props.vector("PrePivot", p);
        }
        body.extend_from_slice(&props.finish());
        self.pkg.set_payload(export, body);
        export
    }

    /// Add a `StaticMeshActor` plus its cooked `StaticMeshComponent`,
    /// pointing at `(package, object)` through a two-level import
    /// chain — the shape `mesh_ref::top_package_name` walks.
    pub fn add_static_mesh_actor(
        &mut self,
        name: &str,
        location: [f32; 3],
        draw_scale: f32,
        mesh_ref: (&str, &str),
    ) -> i32 {
        self.add_mesh_actor_of_class("StaticMeshActor", name, location, draw_scale, mesh_ref)
    }

    /// Same as [`Self::add_static_mesh_actor`], but with the export's
    /// class name as a parameter — NA36's `InterpActor` / `KActor` /
    /// `FracturedStaticMeshActor` fixtures use this to prove
    /// `staticmesh::collect_static_mesh_instances` walks the whole
    /// `MESH_ACTOR_CLASSES` family identically, not just the literal
    /// `StaticMeshActor` class.
    pub fn add_mesh_actor_of_class(
        &mut self,
        class_name: &str,
        name: &str,
        location: [f32; 3],
        draw_scale: f32,
        mesh_ref: (&str, &str),
    ) -> i32 {
        let actor_class = self.pkg.class_ref(class_name);
        let actor = self.pkg.add_export(actor_class, self.level, name);
        let component_class = self.pkg.class_ref("StaticMeshComponent");
        let component = self
            .pkg
            .add_export(component_class, actor, &format!("{name}_Component"));

        let package_import = self.pkg.add_import("Core", "Package", 0, mesh_ref.0);
        let mesh_import = self
            .pkg
            .add_import("Engine", "StaticMesh", package_import, mesh_ref.1);

        let mut actor_body = vec![0u8; ACTOR_PROPS_OFFSET];
        let mut props = self.pkg.props();
        props
            .placement(location, [0; 3], draw_scale, [1.0, 1.0, 1.0])
            .object("StaticMeshComponent", component);
        actor_body.extend_from_slice(&props.finish());
        self.pkg.set_payload(actor, actor_body);

        let mut component_body = vec![0u8; COMPONENT_PROPS_OFFSET];
        let mut props = self.pkg.props();
        props.object("StaticMesh", mesh_import);
        component_body.extend_from_slice(&props.finish());
        self.pkg.set_payload(component, component_body);

        actor
    }

    /// Add a `StaticMeshActor` whose cooked component carries no
    /// `StaticMesh` property — the prefab-archetype stub. Its export
    /// `Archetype` field points at an import, which is what makes
    /// `archetype_label` return `Some`.
    pub fn add_archetype_stub_actor(&mut self, name: &str, archetype_package: &str) -> i32 {
        let actor_class = self.pkg.class_ref("StaticMeshActor");
        let actor = self.pkg.add_export(actor_class, self.level, name);
        let component_class = self.pkg.class_ref("StaticMeshComponent");
        let component = self
            .pkg
            .add_export(component_class, actor, &format!("{name}_Component"));

        let package_import = self.pkg.add_import("Core", "Package", 0, archetype_package);
        let template = self.pkg.add_import(
            "Engine",
            "StaticMeshActor",
            package_import,
            &format!("{name}_Template"),
        );
        self.pkg.set_archetype(actor, template);

        let mut actor_body = vec![0u8; ACTOR_PROPS_OFFSET];
        let mut props = self.pkg.props();
        props
            .placement([0.0; 3], [0; 3], 1.0, [1.0; 3])
            .object("StaticMeshComponent", component);
        actor_body.extend_from_slice(&props.finish());
        self.pkg.set_payload(actor, actor_body);

        let mut component_body = vec![0u8; COMPONENT_PROPS_OFFSET];
        let mut props = self.pkg.props();
        props.float("CachedCullDistance", 5000.0);
        component_body.extend_from_slice(&props.finish());
        self.pkg.set_payload(component, component_body);

        actor
    }

    /// Serialise the chunk bytes.
    pub fn build(&self) -> Vec<u8> {
        self.pkg.build()
    }

    /// Write `<dir>/<map>-<chunk_id:08x>.umap` and return its path.
    pub fn write(&self, dir: &Path, map: &str, chunk_id: u32) -> PathBuf {
        let path = dir.join(format!("{map}-{chunk_id:08x}.umap"));
        self.pkg.write_to(&path).expect("write chunk fixture");
        path
    }
}

/// Write a standalone `.upk` holding one `StaticMesh` export, and
/// return its path.
///
/// `PackageIndex` keys on `(file stem, object name)`, so the file is
/// named `<package>.upk` and the export `<object>` — exactly the pair
/// `add_static_mesh_actor` points its import chain at.
pub fn mesh_package(dir: &Path, package: &str, object: &str, mesh: &StaticMeshPayload) -> PathBuf {
    let mut pkg = PackageBuilder::new();
    let class = pkg.class_ref("StaticMesh");
    let export = pkg.add_export(class, 0, object);
    pkg.set_payload(export, mesh.encode());
    let path = dir.join(format!("{package}.upk"));
    pkg.write_to(&path).expect("write mesh package");
    path
}

/// Build a real [`PackageIndex`] over a directory of fixture packages.
pub fn index_over(dir: &Path) -> PackageIndex {
    PackageIndex::build(dir).expect("index fixture directory")
}

/// A unique, empty scratch directory under the system temp dir.
///
/// Named by process and thread id so parallel test binaries cannot
/// collide, and removed-then-recreated so a rerun never sees a stale
/// OBJ from the previous one — an extractor test that asserts on the
/// *set* of files in its output directory is otherwise trivially
/// polluted.
pub fn scratch_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "cimmeria-navmesh-{tag}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    dir
}
