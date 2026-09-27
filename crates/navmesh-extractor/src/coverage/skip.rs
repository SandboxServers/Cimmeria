//! Why an actor produced no triangles, and the per-reason tally.

/// Why a `StaticMeshActor` produced no triangles.
///
/// Ordered from "earliest in the resolution chain" to "latest" so a TSV
/// reader can see how far each actor got before falling out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SkipReason {
    /// The actor's tagged-property block carries no `StaticMeshComponent`
    /// object reference at all.
    NoComponentRef,
    /// The `StaticMeshComponent` reference points outside the export
    /// table, at an import, or at a zero-length export.
    ComponentUnreadable,
    /// The component parsed but has **no `StaticMesh` property**, and no
    /// [`cimmeria_upk_objects::PackageIndex`] was available to follow
    /// its archetype. This is the archetype-stub shape described in the
    /// `staticmesh` module doc: a cooked component holding only
    /// per-instance overrides, with the real mesh reference living in
    /// the prefab archetype's component in another package.
    ///
    /// With an index supplied this reason no longer fires — the stub is
    /// resolved by `staticmesh::archetype`, or falls into one of the
    /// five `Archetype*` reasons below.
    ArchetypeStubComponent,
    /// The stub's `Archetype` is 0, points at a local export we could
    /// not read, or is an import whose outer chain never reaches a root
    /// package — so there is no `(package, path)` to look up.
    ArchetypeUnrooted,
    /// The archetype's owning package is not in the supplied
    /// [`cimmeria_upk_objects::PackageIndex`], or failed to open.
    ArchetypePackageNotFound,
    /// The archetype's package opened but holds no export at the
    /// template's dotted outer path.
    ArchetypeExportNotFound,
    /// The archetype chain revisited a path, or exceeded
    /// [`crate::staticmesh::archetype::MAX_ARCHETYPE_DEPTH`] hops.
    ArchetypeChainLoop,
    /// The chain terminated at a template that has neither a
    /// `StaticMesh` property nor a further archetype to climb to.
    ArchetypeNoMesh,
    /// The actor's **own** `Archetype` (the chain that supplies
    /// `bCollideActors`, distinct from the component chain above) is
    /// non-zero but could not be followed.
    ///
    /// The actor is skipped rather than emitted, because the
    /// alternative is assuming UE3's `bCollideActors = true` default
    /// for a template that may well have said `false` — and the
    /// component chain can resolve a mesh perfectly well on its own, so
    /// the extractor would happily emit geometry nothing collides with.
    /// On Castle that shape is 26 prefabs' worth of weather cards
    /// sitting in doorways; emitting them split the exterior navmesh.
    ActorArchetypeUnreadable,
    /// `CollideActors` is explicitly `false` on the component or,
    /// through UE3 property inheritance, on its archetype. The mesh is
    /// rendered but nothing collides with it, so rasterising it would
    /// put a wall or a floor in the navmesh that the player walks
    /// straight through.
    CollisionDisabled,
    /// The component has a `StaticMesh` property but it is a `None`-ref
    /// (object index 0).
    NullMeshRef,
    /// The mesh reference is an import whose outer chain never terminates
    /// in a root package, or a package-local export we can't key on.
    UnresolvableMeshRef,
    /// The `(package, object)` key is absent from the supplied
    /// [`cimmeria_upk_objects::PackageIndex`].
    MeshNotInIndex,
    /// The mesh was found in the index but the `StaticMesh` decoder
    /// errored on its bytes.
    MeshDecodeFailed,
    /// The mesh decoded cleanly but `collision_triangles()` came back
    /// empty — no kDOP tree and no LOD0 index buffer.
    MeshNoCollision,
    /// No `PackageIndex` was supplied (degraded mode). Every actor lands
    /// here; the walk still reports `actors_total`.
    NoPackageIndex,
    /// An `InterpActor` whose mesh resolved, left out on evidence by
    /// `interp_actor::classify` (NA40): a door, a Stargate part, a
    /// camera head, or a Matinee mover that leaves its cooked pose.
    InterpActorExcluded,
    /// An `InterpActor` whose mesh resolved, left out because the
    /// classifier could not tell whether it moves. The class census
    /// reports `InterpActor` as a collision risk while any are counted.
    InterpActorUndecided,
}

impl SkipReason {
    /// Every variant, in declaration order. Used for deterministic TSV
    /// column ordering and for the `merge` / `total` loops.
    pub const ALL: [SkipReason; 18] = [
        SkipReason::NoComponentRef,
        SkipReason::ComponentUnreadable,
        SkipReason::ArchetypeStubComponent,
        SkipReason::ArchetypeUnrooted,
        SkipReason::ArchetypePackageNotFound,
        SkipReason::ArchetypeExportNotFound,
        SkipReason::ArchetypeChainLoop,
        SkipReason::ArchetypeNoMesh,
        SkipReason::ActorArchetypeUnreadable,
        SkipReason::CollisionDisabled,
        SkipReason::NullMeshRef,
        SkipReason::UnresolvableMeshRef,
        SkipReason::MeshNotInIndex,
        SkipReason::MeshDecodeFailed,
        SkipReason::MeshNoCollision,
        SkipReason::NoPackageIndex,
        SkipReason::InterpActorExcluded,
        SkipReason::InterpActorUndecided,
    ];

    /// Stable snake_case identifier — used verbatim as a TSV column head.
    pub fn column(self) -> &'static str {
        match self {
            SkipReason::NoComponentRef => "skip_no_component_ref",
            SkipReason::ComponentUnreadable => "skip_component_unreadable",
            SkipReason::ArchetypeStubComponent => "skip_archetype_stub_component",
            SkipReason::ArchetypeUnrooted => "skip_archetype_unrooted",
            SkipReason::ArchetypePackageNotFound => "skip_archetype_package_not_found",
            SkipReason::ArchetypeExportNotFound => "skip_archetype_export_not_found",
            SkipReason::ArchetypeChainLoop => "skip_archetype_chain_loop",
            SkipReason::ArchetypeNoMesh => "skip_archetype_no_mesh",
            SkipReason::ActorArchetypeUnreadable => "skip_actor_archetype_unreadable",
            SkipReason::CollisionDisabled => "skip_collision_disabled",
            SkipReason::NullMeshRef => "skip_null_mesh_ref",
            SkipReason::UnresolvableMeshRef => "skip_unresolvable_mesh_ref",
            SkipReason::MeshNotInIndex => "skip_mesh_not_in_index",
            SkipReason::MeshDecodeFailed => "skip_mesh_decode_failed",
            SkipReason::MeshNoCollision => "skip_mesh_no_collision",
            SkipReason::NoPackageIndex => "skip_no_package_index",
            SkipReason::InterpActorExcluded => "skip_interp_actor_excluded",
            SkipReason::InterpActorUndecided => "skip_interp_actor_undecided",
        }
    }

    pub(super) fn slot(self) -> usize {
        match self {
            SkipReason::NoComponentRef => 0,
            SkipReason::ComponentUnreadable => 1,
            SkipReason::ArchetypeStubComponent => 2,
            SkipReason::ArchetypeUnrooted => 3,
            SkipReason::ArchetypePackageNotFound => 4,
            SkipReason::ArchetypeExportNotFound => 5,
            SkipReason::ArchetypeChainLoop => 6,
            SkipReason::ArchetypeNoMesh => 7,
            SkipReason::ActorArchetypeUnreadable => 8,
            SkipReason::CollisionDisabled => 9,
            SkipReason::NullMeshRef => 10,
            SkipReason::UnresolvableMeshRef => 11,
            SkipReason::MeshNotInIndex => 12,
            SkipReason::MeshDecodeFailed => 13,
            SkipReason::MeshNoCollision => 14,
            SkipReason::NoPackageIndex => 15,
            SkipReason::InterpActorExcluded => 16,
            SkipReason::InterpActorUndecided => 17,
        }
    }
}

/// Fixed-slot counter over [`SkipReason`].
///
/// A plain array rather than a `HashMap` so iteration order is the
/// declaration order of [`SkipReason::ALL`] and a row always has the same
/// columns whether or not a reason fired.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SkipTally {
    counts: [u64; SkipReason::ALL.len()],
}

impl SkipTally {
    /// Record one skipped actor.
    pub fn add(&mut self, reason: SkipReason) {
        self.add_n(reason, 1);
    }

    /// Record `n` skipped actors at once — the mesh-load stage fails a
    /// whole instance group in one go.
    pub fn add_n(&mut self, reason: SkipReason, n: u64) {
        self.counts[reason.slot()] += n;
    }

    /// Count for one reason.
    pub fn get(&self, reason: SkipReason) -> u64 {
        self.counts[reason.slot()]
    }

    /// Sum across all reasons.
    pub fn total(&self) -> u64 {
        self.counts.iter().sum()
    }

    /// Accumulate another tally into this one.
    pub fn merge(&mut self, other: &SkipTally) {
        for (dst, src) in self.counts.iter_mut().zip(other.counts.iter()) {
            *dst += *src;
        }
    }
}
