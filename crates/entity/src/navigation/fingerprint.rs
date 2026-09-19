//! Identity of a loaded `.nav` file.
//!
//! # Why a mesh needs an identity
//!
//! The load log used to carry a polygon count and nothing else, which is
//! not enough to tie a player session to the mesh it actually ran on. When
//! `Castle_CellBlock`'s navmesh was rebuilt in September 2026 because
//! SigNoz showed real players being snapped back where the 2013 mesh had
//! holes, reconstructing "which mesh was loaded during that session" had to
//! be done by hand from deploy timestamps. A stable content hash on the
//! load line — and repeated as a short hash on every per-event navmesh log
//! — makes that a join instead of an archaeology exercise.
//!
//! # Hash choice
//!
//! FNV-1a 64 over the raw file bytes, hand-rolled. This is **not** a
//! cryptographic hash and must never be used as one: it identifies an
//! asset build, it does not authenticate it. FNV-1a was chosen over
//! `sha2` (which the workspace already builds, for auth) because this
//! needs no new direct dependency in `cimmeria-entity`, runs at roughly
//! memory bandwidth over a few hundred KB at startup, and produces a
//! fixed-width value short enough to paste into a log filter.

/// Everything about a loaded `.nav` file that identifies *which* mesh it
/// is, as opposed to what it can answer.
///
/// Held by value on [`super::NavMesh`] so any consumer holding the mesh
/// can name it — see [`super::NavMesh::short_hash`].
#[derive(Debug, Clone)]
pub struct NavMeshFingerprint {
    /// The path the mesh was loaded from, as the caller spelled it.
    pub path: String,
    /// Size of the `.nav` file in bytes.
    pub file_bytes: u64,
    /// FNV-1a 64 of the whole file, as 16 lowercase hex digits.
    pub content_hash: String,
    /// First 8 characters of [`Self::content_hash`] — what per-event logs
    /// carry, so a hot-path log line stays short while still being
    /// joinable back to the load line.
    pub short_hash: String,
    /// Vertex count from the XRC header.
    pub nverts: u32,
    /// Polygon count from the XRC header.
    pub npolys: u32,
    /// Agent capsule height the mesh was baked for.
    pub agent_height: f32,
    /// Maximum step-up the mesh was baked for.
    pub agent_climb: f32,
    /// Agent capsule radius the mesh was baked for. Also the source of
    /// the runtime containment gates — see
    /// [`super::NavMesh::diagnose_point`].
    pub agent_radius: f32,
}

impl NavMeshFingerprint {
    /// Build a fingerprint from the raw file bytes plus the header values
    /// [`super::NavMesh::load`] already parsed out of them.
    #[allow(clippy::too_many_arguments)] // flat header fields; a params
                                         // struct here would just be this struct one field short.
    pub(super) fn new(
        path: &std::path::Path,
        bytes: &[u8],
        nverts: u32,
        npolys: u32,
        agent_height: f32,
        agent_climb: f32,
        agent_radius: f32,
    ) -> Self {
        let content_hash = format!("{:016x}", fnv1a_64(bytes));
        // 8 hex digits = 32 bits. Collisions are irrelevant here: the
        // short form only ever disambiguates between the handful of
        // meshes one deployment has loaded, and the full hash is on the
        // load line for anyone who needs certainty.
        let short_hash = content_hash[..8].to_string();
        Self {
            path: path.display().to_string(),
            file_bytes: bytes.len() as u64,
            content_hash,
            short_hash,
            nverts,
            npolys,
            agent_height,
            agent_climb,
            agent_radius,
        }
    }
}

/// FNV-1a, 64-bit. Non-cryptographic; see the module doc.
fn fnv1a_64(bytes: &[u8]) -> u64 {
    const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut hash = OFFSET_BASIS;
    for &b in bytes {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(PRIME);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The canonical FNV-1a 64 test vectors. Pinning these means a
    /// "harmless" refactor of the hash loop (swapping the xor and the
    /// multiply, or using the FNV-1 order instead of FNV-1a) changes
    /// every shipped mesh's hash silently — which would break the join
    /// between a historical load line and a current one.
    #[test]
    fn fnv1a_64_matches_the_reference_vectors() {
        assert_eq!(fnv1a_64(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a_64(b"a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(fnv1a_64(b"foobar"), 0x8594_4171_f739_67e8);
    }

    #[test]
    fn short_hash_is_the_first_eight_of_the_full_hash() {
        let fp = NavMeshFingerprint::new(
            std::path::Path::new("data/spaces/x.nav"),
            b"foobar",
            1,
            2,
            1.8,
            0.6,
            0.6,
        );
        assert_eq!(fp.content_hash, "85944171f73967e8");
        assert_eq!(fp.short_hash, "85944171");
        assert_eq!(fp.file_bytes, 6);
    }

    /// A one-byte change must change the hash — the whole point is that a
    /// rebuilt mesh is distinguishable from the one it replaced.
    #[test]
    fn a_single_byte_difference_changes_the_hash() {
        let a = NavMeshFingerprint::new(
            std::path::Path::new("a.nav"),
            b"mesh-v1",
            0,
            0,
            0.0,
            0.0,
            0.0,
        );
        let b = NavMeshFingerprint::new(
            std::path::Path::new("a.nav"),
            b"mesh-v2",
            0,
            0,
            0.0,
            0.0,
            0.0,
        );
        assert_ne!(a.content_hash, b.content_hash);
    }
}
