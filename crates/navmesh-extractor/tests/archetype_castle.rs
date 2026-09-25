//! Real-data acceptance for prefab-archetype resolution.
//!
//! The synthetic fixtures in `staticmesh::archetype_walk_tests` prove
//! the chain walks a shape we *built*. This pins it against the shape
//! the SGW cooker actually wrote, on the chunk the 2026-09-18 colo
//! playtest walked through.
//!
//! Self-skips (loudly) when the cooked asset bundle or a cached
//! `PackageIndex` is absent. A skipped run is not a pass; the reason
//! goes to stderr. Override discovery with:
//!
//! ```text
//! CIMMERIA_COOKED_PC=<...>/SGWGame/CookedPC
//! CIMMERIA_PACKAGE_INDEX=<...>/package_index.bin
//! ```
//!
//! Numbers measured 2026-09-19 against `CookedPC/Maps/Castle` with a
//! 2,821,598-export index; see `crates/navmesh-extractor/README.md`
//! §"Measured Castle coverage".

use std::path::PathBuf;

use cimmeria_navmesh_extractor::coverage::SkipReason;
use cimmeria_navmesh_extractor::staticmesh::{
    collect_static_mesh_instances, extract_chunk, ArchetypeCache,
};
use cimmeria_upk_objects::PackageIndex;

/// The Interrogation Block interior tile: 844 `StaticMeshActor`
/// exports, 147 `PrefabInstance`, 147 archetype-instanced actors.
const CHUNK_FILE: &str = "Castle-000a0002.umap";

/// Every archetype-stub actor in `000a0002`. The README's
/// "147 PrefabInstance / 147 archetype actors / 147 skips" line.
const ARCHETYPE_ACTORS: u64 = 147;
/// ...of which emit geometry once the chain is followed. The other 22
/// are suppressed by `bCollideActors = false` on the prefab template —
/// wall lights, exit signs, wall alarms and a `CA-Relief00`.
const ARCHETYPE_RESOLVED: u64 = 125;

/// Actors in this chunk suppressed by `bCollideActors = false`: the 22
/// above plus 102 non-prefab actors that set the flag on themselves
/// (icicles, floor signs, wall panels, hoses). One tally covers both
/// halves — the split is not separately observable from `SkipTally`,
/// and `archetype_census` is the tool that breaks it down.
const COLLISION_DISABLED: u64 = 124;

fn cooked_pc_dir() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("CIMMERIA_COOKED_PC") {
        let p = PathBuf::from(p);
        return p.is_dir().then_some(p);
    }
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let suffix = PathBuf::from("sgw/Stargate Worlds-QA/Working/SGWGame/CookedPC");
    manifest
        .ancestors()
        .take(10)
        .map(|a| a.join(&suffix))
        .find(|c| c.is_dir())
}

fn try_load_package_index() -> Option<PackageIndex> {
    if let Ok(p) = std::env::var("CIMMERIA_PACKAGE_INDEX") {
        // An explicitly-pointed-at index that fails to load is operator
        // error, not an absent asset — say so rather than degrading
        // into a skip that reads as a pass.
        return match PackageIndex::load(PathBuf::from(&p).as_path()) {
            Ok(idx) => Some(idx),
            Err(e) => {
                eprintln!("CIMMERIA_PACKAGE_INDEX={p} set but could not be loaded: {e}");
                None
            }
        };
    }
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for ancestor in manifest.ancestors().take(10) {
        for name in [
            "package_index.bin",
            "package_index.bincode",
            ".package_index.bin",
        ] {
            let candidate = ancestor.join(name);
            if candidate.exists() {
                match PackageIndex::load(&candidate) {
                    Ok(idx) => return Some(idx),
                    Err(e) => eprintln!("Failed to load {}: {e}", candidate.display()),
                }
            }
        }
    }
    None
}

/// `(chunk path, index)`, or `None` with a loud reason.
fn setup(test: &str) -> Option<(PathBuf, PackageIndex)> {
    let Some(cooked) = cooked_pc_dir() else {
        eprintln!("SKIPPED {test} — no CookedPC tree (set CIMMERIA_COOKED_PC)");
        return None;
    };
    let chunk = cooked.join("Maps").join("Castle").join(CHUNK_FILE);
    if !chunk.is_file() {
        eprintln!("SKIPPED {test} — {} missing", chunk.display());
        return None;
    }
    let Some(index) = try_load_package_index() else {
        eprintln!("SKIPPED {test} — no PackageIndex (set CIMMERIA_PACKAGE_INDEX)");
        return None;
    };
    Some((chunk, index))
}

/// Every archetype stub in `000a0002` reaches a verdict — a mesh or a
/// collision veto — and none falls out as an unresolved chain.
///
/// This is the coverage claim: before this work all 147 landed in
/// `ArchetypeStubComponent`, which is now reserved for the no-index
/// case. If a future asset-tree or resolver change reintroduces a
/// dangling chain it shows up here as a non-zero `Archetype*` skip.
#[test]
fn castle_000a0002_archetype_stubs_all_reach_a_verdict() {
    let Some((chunk, index)) = setup("castle_000a0002_archetype_stubs_all_reach_a_verdict") else {
        return;
    };
    let pkg = cimmeria_upk::Package::open(&chunk).expect("open chunk");
    let mut cache = ArchetypeCache::default();
    let walk = collect_static_mesh_instances(&pkg, Some(&index), &mut cache, false);

    eprintln!(
        "000a0002: {} actors, {} instances, skips {:?}",
        walk.actors_total,
        walk.instances.len(),
        SkipReason::ALL
            .iter()
            .filter(|r| walk.skips.get(**r) > 0)
            .map(|r| (r.column(), walk.skips.get(*r)))
            .collect::<Vec<_>>()
    );

    assert_eq!(
        walk.archetype_actors, ARCHETYPE_ACTORS,
        "chunk shape changed: expected {ARCHETYPE_ACTORS} archetype-instanced actors"
    );

    // The no-index reason must not fire when an index WAS supplied.
    assert_eq!(
        walk.skips.get(SkipReason::ArchetypeStubComponent),
        0,
        "with an index, no stub may be left unresolved"
    );
    for reason in [
        SkipReason::ArchetypeUnrooted,
        SkipReason::ArchetypePackageNotFound,
        SkipReason::ArchetypeExportNotFound,
        SkipReason::ArchetypeChainLoop,
        SkipReason::ArchetypeNoMesh,
    ] {
        assert_eq!(
            walk.skips.get(reason),
            0,
            "{} fired on real Castle data",
            reason.column()
        );
    }

    let via: u64 = walk.instances.iter().filter(|i| i.via_archetype).count() as u64;
    assert_eq!(
        via, ARCHETYPE_RESOLVED,
        "archetype-resolved instance count drifted"
    );
    assert_eq!(
        walk.skips.get(SkipReason::CollisionDisabled),
        COLLISION_DISABLED,
        "collision-suppression count drifted"
    );

    // The balance invariant the whole coverage report rests on.
    assert_eq!(
        walk.actors_total,
        walk.instances.len() as u64 + walk.skips.total(),
        "actor walk does not balance"
    );
}

/// The regression guard: archetype resolution has to be worth
/// triangles, not just a changed skip column.
///
/// Reverting the resolution (making a stub component return
/// `ArchetypeStubComponent` unconditionally) drops
/// `actors_resolved_via_archetype` to 0 and
/// `triangles_via_archetype` to 0, and this fails on the first
/// assertion. Proven once by reverting the `index`/`cache` arms in
/// `collect_static_mesh_instances`.
#[test]
fn castle_000a0002_archetype_resolution_emits_triangles() {
    let Some((chunk, index)) = setup("castle_000a0002_archetype_resolution_emits_triangles") else {
        return;
    };
    let extraction = extract_chunk(&chunk, Some(&index), false).expect("extract_chunk");
    eprintln!(
        "000a0002: resolved={} via_archetype={} tris={} via_archetype_tris={} prefab_pkgs={}",
        extraction.actors_resolved,
        extraction.actors_resolved_via_archetype,
        extraction.triangles_emitted,
        extraction.triangles_via_archetype,
        extraction.prefab_packages_opened,
    );

    assert!(
        extraction.actors_resolved_via_archetype > 0,
        "archetype resolution produced no instances at all"
    );
    assert_eq!(
        extraction.actors_resolved_via_archetype, ARCHETYPE_RESOLVED,
        "archetype-resolved count drifted"
    );
    assert!(
        extraction.triangles_via_archetype >= 3_000,
        "archetype instances contributed only {} triangles",
        extraction.triangles_via_archetype
    );
    assert!(
        extraction.triangles_via_archetype < extraction.triangles_emitted,
        "archetype triangles cannot exceed the chunk total"
    );
    assert_eq!(
        extraction.actors_total,
        extraction.actors_resolved + extraction.actors_unresolved,
        "chunk extraction does not balance"
    );
}

/// The `bCollideActors` gate, on real data.
///
/// Reverting the gate makes `CollisionDisabled` zero and every one of
/// these actors emit geometry — on the whole map that is 1,570 phantom
/// obstacles, and it splits Castle's exterior navmesh from one
/// walkable component into three.
#[test]
fn castle_000a0002_non_colliding_actors_are_suppressed() {
    let Some((chunk, index)) = setup("castle_000a0002_non_colliding_actors_are_suppressed") else {
        return;
    };
    let pkg = cimmeria_upk::Package::open(&chunk).expect("open chunk");
    let mut cache = ArchetypeCache::default();
    let walk = collect_static_mesh_instances(&pkg, Some(&index), &mut cache, false);

    let suppressed = walk.skips.get(SkipReason::CollisionDisabled);
    eprintln!("000a0002: {suppressed} actors suppressed by bCollideActors = false");
    assert_eq!(
        suppressed, COLLISION_DISABLED,
        "collision-suppression count drifted"
    );
    assert!(
        !walk.instances.is_empty(),
        "the gate must not have swallowed the whole chunk"
    );
}
