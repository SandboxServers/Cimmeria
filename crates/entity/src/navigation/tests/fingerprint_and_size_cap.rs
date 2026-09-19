//! Guards for the loader's *identity* and *cost* properties, as opposed
//! to its geometry: the [`NavMeshFingerprint`] a shipped `.nav` produces,
//! and the file-size gate that decides whether the loader will read one
//! at all.
//!
//! [`NavMeshFingerprint`]: super::super::NavMeshFingerprint

use super::super::xrc::{check_file_size, MAX_NAV_FILE_BYTES};
use super::super::NavMesh;
use super::make_tmp_nav_path;

/// The fingerprint the loader must keep producing for a given shipped
/// mesh, forever.
///
/// These are not arbitrary golden values. `navmesh_hash` /
/// `navmesh_short_hash` are already stamped on every playtest `.bug`
/// row, every `movement.validation_reject`, every `npc_ai.path_fail`
/// and every `movement.navmesh` load line that has been shipped to
/// SigNoz. The whole point of the fingerprint is that a row from last
/// month joins to a load line from today; a refactor that changes the
/// value for an unchanged file silently severs every one of those
/// joins, and nothing else in the suite would notice.
///
/// `(path, file_bytes, content_hash)`.
const SHIPPED: &[(&str, u64, &str)] = &[
    (
        "../../data/spaces/castle_cellblock.nav",
        184_520,
        "3f53c30c5eedddf3",
    ),
    (
        "../../data/spaces/castle.nav",
        3_402_055,
        "5308e9033c853148",
    ),
];

#[test]
fn navmesh_hash_is_stable_for_the_shipped_meshes() {
    for (path, file_bytes, content_hash) in SHIPPED {
        let p = std::path::Path::new(path);
        if !p.exists() {
            continue; // fixture-less CI — same skip as the sibling geometry tests
        }
        let mesh = NavMesh::load(p).unwrap_or_else(|e| panic!("load {path}: {e:?}"));
        let fp = mesh.fingerprint();
        assert_eq!(
            fp.file_bytes, *file_bytes,
            "{path}: file size changed — if the mesh was deliberately \
             rebuilt, update this row AND the provenance table in \
             data/spaces/README.md"
        );
        assert_eq!(
            fp.content_hash, *content_hash,
            "{path}: navmesh_hash changed for an unchanged file. Every \
             historical .bug row and movement.validation_reject carries \
             this value; changing how it is computed breaks the join \
             back to them. If the mesh itself was rebuilt, update this \
             row and data/spaces/README.md."
        );
        assert_eq!(
            fp.short_hash,
            &content_hash[..8],
            "{path}: the short form must stay the first 8 of the full hash"
        );
    }
}

// ── The file-size gate ───────────────────────────────────────────────

/// The cap is derived from the `MAX_*` header caps, so it has to stay
/// far above anything the project ships and far below a number that
/// would make the gate pointless.
#[test]
fn max_nav_file_bytes_brackets_the_real_assets() {
    // castle.nav, the largest shipped mesh at the time of writing.
    assert!(
        MAX_NAV_FILE_BYTES > 3_402_055 * 8,
        "the cap must leave generous headroom over the largest shipped \
         mesh, or a legitimate rebuild trips it"
    );
    assert!(
        MAX_NAV_FILE_BYTES < 1 << 30,
        "a cap at or above 1 GiB is not a cap — the whole point is to \
         bound what a corrupt deployment asset can cost at startup"
    );
    assert!(
        check_file_size(MAX_NAV_FILE_BYTES).is_ok(),
        "boundary is ok"
    );
    match check_file_size(MAX_NAV_FILE_BYTES + 1) {
        Err(cimmeria_common::CimmeriaError::NavHeaderOutOfRange { field, .. }) => {
            assert_eq!(field, "file_bytes");
        }
        other => panic!("one byte over the cap must be rejected, got {other:?}"),
    }
}

/// **The regression guard for the oversized-asset path.** Before this,
/// `load` called `std::fs::read` up front to get bytes to hash, so an
/// oversized `.nav` was fully allocated before any cap could look at
/// it. The assertion that distinguishes "the size gate fired" from "the
/// parse failed some other way" is the `field` on the error: a file of
/// zero bytes past the cap would otherwise fail as a short read on the
/// first `read_f32`.
///
/// The file is created with `set_len`, which does not write its
/// contents — on a filesystem that cannot do that cheaply the test
/// skips rather than spending a minute writing zeroes.
#[test]
fn an_oversized_file_is_rejected_by_size_not_by_parsing_it() {
    let path = make_tmp_nav_path("oversized");
    let Ok(f) = std::fs::File::create(&path) else {
        return;
    };
    if f.set_len(MAX_NAV_FILE_BYTES + 1).is_err() {
        drop(f);
        let _ = std::fs::remove_file(&path);
        return;
    }
    drop(f);

    let result = NavMesh::load(&path);
    let _ = std::fs::remove_file(&path);

    match result {
        Err(cimmeria_common::CimmeriaError::NavHeaderOutOfRange { field, .. }) => {
            assert_eq!(
                field, "file_bytes",
                "the size gate must be what rejected it — a different \
                 field means the loader read into the file first"
            );
        }
        other => panic!(
            "expected NavHeaderOutOfRange(file_bytes), got {other:?}\n\
             If this regressed: NavMesh::load stopped checking \
             std::fs::metadata against MAX_NAV_FILE_BYTES before opening \
             the file."
        ),
    }
}
