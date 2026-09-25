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
//!
//! # Why the hash is streaming
//!
//! FNV-1a folds one byte at a time, so it composes over a chunked read
//! with the identical result. [`HashingReader`] exploits that: the
//! loader parses straight out of a `BufReader` — rejecting a hostile
//! header count after 60 bytes, before any count-driven allocation — and
//! still ends up with a hash over every byte of the file, exactly as a
//! read-it-all-then-hash would produce. The hashes are load-bearing
//! across time (playtest `.bug` rows and historical load lines carry
//! them), so "identical" is a pinned assertion, not an aspiration: see
//! `navigation::tests::navmesh_hash_is_stable_for_the_shipped_meshes`.

use std::io::Read as IoRead;

/// The agent capsule a `.nav` file was baked for — section 1 of the XRC
/// header.
///
/// Grouped rather than passed as three loose `f32`s because they are
/// meaningless apart and trivially transposable: `(height, climb,
/// radius)` and `(radius, climb, height)` are both three positive floats
/// and the type system cannot tell them apart at a call site.
#[derive(Debug, Clone, Copy)]
pub(super) struct AgentParams {
    pub height: f32,
    pub climb: f32,
    pub radius: f32,
}

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
    /// Build a fingerprint from what [`super::NavMesh::load`]'s single
    /// streaming pass produced: the file's total length and FNV-1a 64
    /// (from [`HashingReader::finish`]), plus the header values it
    /// parsed on the way through.
    pub(super) fn new(
        path: &std::path::Path,
        file_bytes: u64,
        content_hash: u64,
        nverts: u32,
        npolys: u32,
        agent: AgentParams,
    ) -> Self {
        let content_hash = format!("{content_hash:016x}");
        // 8 hex digits = 32 bits. Collisions are irrelevant here: the
        // short form only ever disambiguates between the handful of
        // meshes one deployment has loaded, and the full hash is on the
        // load line for anyone who needs certainty.
        let short_hash = content_hash[..8].to_string();
        Self {
            path: path.display().to_string(),
            file_bytes,
            content_hash,
            short_hash,
            nverts,
            npolys,
            agent_height: agent.height,
            agent_climb: agent.climb,
            agent_radius: agent.radius,
        }
    }
}

/// FNV-1a, 64-bit, as an incremental accumulator. Non-cryptographic; see
/// the module doc.
#[derive(Debug, Clone, Copy)]
pub(super) struct Fnv1a64 {
    state: u64,
}

impl Fnv1a64 {
    const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;

    pub(super) fn new() -> Self {
        Self {
            state: Self::OFFSET_BASIS,
        }
    }

    pub(super) fn update(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.state ^= u64::from(b);
            self.state = self.state.wrapping_mul(Self::PRIME);
        }
    }

    pub(super) fn finish(self) -> u64 {
        self.state
    }
}

/// A reader that folds every byte it hands out into an [`Fnv1a64`] and
/// counts them.
///
/// This is what lets the loader have both properties at once: the parse
/// reads the header first (so [`super::xrc::check_count`] can reject a
/// hostile count before the allocation it would drive), and the
/// fingerprint still covers the whole file rather than only the parsed
/// prefix. [`Self::finish`] drains whatever the parser did not consume,
/// so trailing bytes are hashed exactly as a whole-file read would hash
/// them.
pub(super) struct HashingReader<R> {
    inner: R,
    hash: Fnv1a64,
    consumed: u64,
}

impl<R: IoRead> HashingReader<R> {
    pub(super) fn new(inner: R) -> Self {
        Self {
            inner,
            hash: Fnv1a64::new(),
            consumed: 0,
        }
    }

    /// Hash whatever is left unread, then return `(content_hash,
    /// total_bytes)`.
    ///
    /// Draining matters for byte-exactness: a `.nav` with trailing bytes
    /// past the detail-triangle section parses fine, and the pre-existing
    /// `std::fs::read` hash covered those bytes. The drain is bounded by
    /// the same file-size cap the loader checked before opening the file.
    pub(super) fn finish(mut self) -> std::io::Result<(u64, u64)> {
        let mut scratch = [0u8; 8192];
        loop {
            let n = self.read(&mut scratch)?;
            if n == 0 {
                break;
            }
        }
        Ok((self.hash.finish(), self.consumed))
    }
}

impl<R: IoRead> IoRead for HashingReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = self.inner.read(buf)?;
        self.hash.update(&buf[..n]);
        self.consumed += n as u64;
        Ok(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fnv1a_64(bytes: &[u8]) -> u64 {
        let mut h = Fnv1a64::new();
        h.update(bytes);
        h.finish()
    }

    fn fingerprint_of(path: &str, bytes: &[u8]) -> NavMeshFingerprint {
        NavMeshFingerprint::new(
            std::path::Path::new(path),
            bytes.len() as u64,
            fnv1a_64(bytes),
            1,
            2,
            AgentParams {
                height: 1.8,
                climb: 0.6,
                radius: 0.6,
            },
        )
    }

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

    /// Chunking must not change the answer. The loader feeds this
    /// accumulator whatever a `BufReader` happens to hand back, so a
    /// hash that depended on the chunk boundaries would make a mesh's
    /// fingerprint a property of the IO layer rather than of the file.
    #[test]
    fn incremental_update_matches_a_single_shot_hash() {
        let data: Vec<u8> = (0u8..=255).cycle().take(9_000).collect();
        let one_shot = fnv1a_64(&data);
        for chunk in [1usize, 7, 64, 4096, 8192] {
            let mut h = Fnv1a64::new();
            for part in data.chunks(chunk) {
                h.update(part);
            }
            assert_eq!(
                h.finish(),
                one_shot,
                "hashing in {chunk}-byte chunks must equal the one-shot hash"
            );
        }
    }

    /// `HashingReader` must cover every byte of the source, including
    /// the tail the parser never asked for — that is what keeps the
    /// streaming loader's fingerprint identical to the whole-file-read
    /// one it replaced.
    #[test]
    fn hashing_reader_covers_the_unparsed_tail() {
        let data: Vec<u8> = (0u8..=200).collect();
        let mut r = HashingReader::new(std::io::Cursor::new(data.clone()));
        let mut head = [0u8; 16];
        r.read_exact(&mut head).expect("read head");
        let (hash, total) = r.finish().expect("drain tail");
        assert_eq!(total, data.len() as u64);
        assert_eq!(
            hash,
            fnv1a_64(&data),
            "the drained tail must be hashed — otherwise a `.nav` with \
             trailing bytes changes fingerprint the moment the loader \
             stops reading the whole file into memory"
        );
    }

    #[test]
    fn short_hash_is_the_first_eight_of_the_full_hash() {
        let fp = fingerprint_of("data/spaces/x.nav", b"foobar");
        assert_eq!(fp.content_hash, "85944171f73967e8");
        assert_eq!(fp.short_hash, "85944171");
        assert_eq!(fp.file_bytes, 6);
    }

    /// A one-byte change must change the hash — the whole point is that a
    /// rebuilt mesh is distinguishable from the one it replaced.
    #[test]
    fn a_single_byte_difference_changes_the_hash() {
        let a = fingerprint_of("a.nav", b"mesh-v1");
        let b = fingerprint_of("a.nav", b"mesh-v2");
        assert_ne!(a.content_hash, b.content_hash);
    }
}
