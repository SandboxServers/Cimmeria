//! `.umap` package enumeration helpers.
//!
//! Thin wrapper around `cimmeria_upk::Package` that walks a map
//! directory, opens each chunk, and surfaces the per-export class /
//! path / serial-size triples needed by the geometry-extractor
//! modules. The heavy lifting (LZO decompression, header parsing,
//! tagged-property iteration) lives in `crates/upk` — we just expose
//! a navmesh-flavoured iteration API on top.

use std::path::{Path, PathBuf};

use crate::ExtractError;

/// Enumerate every `*.umap` file in `map_dir` that follows the chunk
/// filename convention `<MapName>-<HEX8>.umap`, sorted by filename for
/// deterministic per-chunk output ordering.
///
/// Master `.umap` files that lack the hex suffix (e.g. `Castle_CellBlock.umap`
/// alongside the chunked-out `Castle_CellBlock-XXXXYYYY.umap` siblings)
/// are skipped — they're streaming-level placeholders, not chunks with
/// per-tile geometry. Recursing into subdirectories is **not** done; SGW
/// cooks all chunks for one map into a single flat directory.
pub fn enumerate_chunks(map_dir: &Path) -> crate::Result<Vec<PathBuf>> {
    if !map_dir.exists() {
        return Err(ExtractError::Other(format!(
            "map directory does not exist: {}",
            map_dir.display()
        )));
    }
    if !map_dir.is_dir() {
        return Err(ExtractError::Other(format!(
            "map path is not a directory: {}",
            map_dir.display()
        )));
    }

    let mut chunks: Vec<PathBuf> = std::fs::read_dir(map_dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .map(|x| x.eq_ignore_ascii_case("umap"))
                .unwrap_or(false)
        })
        .filter(|p| is_chunk_filename(p))
        .collect();
    chunks.sort();
    Ok(chunks)
}

/// Return `true` if the file's stem ends with `-<HEX8>` (the SGW
/// per-chunk naming convention). Files that don't match — such as the
/// master streaming-level `.umap` for the map — are skipped.
fn is_chunk_filename(path: &Path) -> bool {
    let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
        return false;
    };
    let Some((_, suffix)) = stem.rsplit_once('-') else {
        return false;
    };
    suffix.len() == 8 && suffix.chars().all(|c| c.is_ascii_hexdigit())
}

/// Summary of a single export inside a `.umap` chunk.
///
/// Used by the per-class decoder dispatcher to decide whether to
/// triangulate (`Terrain` / `StaticMeshActor`), to follow a
/// cross-package reference (`StaticMesh`), or to skip (the editor's
/// builder-cube `Brush` at chunk-relative `(9808, 48, 0)`, lighting
/// volumes, particle effects, etc.).
#[derive(Debug, Clone)]
pub struct ExportSummary {
    pub index: usize,
    pub class: String,
    pub path: String,
    /// Byte offset of this export within the package's decompressed
    /// stream. Inherits the underlying `i32` from `cimmeria_upk` so
    /// round-trip diagnostics see the same value the UPK header
    /// declared.
    pub serial_offset: i32,
    /// Byte length of the export's serialized body.
    pub serial_size: i32,
}

/// Open `chunk_path` with the `cimmeria_upk` parser and return a
/// summary line per export. The full payload bytes are NOT loaded;
/// per-class decoders re-open the package to read what they need.
///
/// **Phase 0/1.1 stub** — currently used only by the round-trip smoke
/// for validation. Phase 1.2 plugs the StaticMesh class into the
/// dispatcher; Phase 1.3 plugs Terrain.
pub fn summarize_exports(chunk_path: &Path) -> crate::Result<Vec<ExportSummary>> {
    let pkg = cimmeria_upk::Package::open(chunk_path)?;
    let summaries = pkg
        .exports
        .iter()
        .enumerate()
        .map(|(i, e)| ExportSummary {
            index: i,
            class: pkg.export_class_name(e).to_string(),
            path: pkg.export_full_path(e),
            serial_offset: e.serial_offset,
            serial_size: e.serial_size,
        })
        .collect();
    Ok(summaries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enumerate_nonexistent_dir_errors() {
        let p = Path::new("/this/path/does/not/exist/anywhere");
        let result = enumerate_chunks(p);
        assert!(result.is_err());
    }

    #[test]
    fn enumerate_skips_non_umap_files() {
        let tmp = tempdir();
        std::fs::write(tmp.join("foo.upk"), b"x").unwrap();
        // Chunk-shaped filename to survive the new chunk-pattern filter.
        std::fs::write(tmp.join("bar-00000000.umap"), b"x").unwrap();
        std::fs::write(tmp.join("baz.txt"), b"x").unwrap();
        let chunks = enumerate_chunks(&tmp).unwrap();
        assert_eq!(chunks.len(), 1);
        assert!(chunks[0].file_name().unwrap() == "bar-00000000.umap");
    }

    /// SGW's per-map directory contains both the chunked tiles
    /// (`<Name>-<HEX8>.umap`) and a master streaming `.umap` with no hex
    /// suffix. The orchestrator should only emit OBJs for the tiles —
    /// the master `.umap` doesn't have per-chunk geometry to extract.
    #[test]
    fn enumerate_skips_master_umap_without_hex_suffix() {
        let tmp = tempdir();
        std::fs::write(tmp.join("Castle_CellBlock.umap"), b"x").unwrap();
        std::fs::write(tmp.join("Castle_CellBlock-00000000.umap"), b"x").unwrap();
        std::fs::write(tmp.join("Castle_CellBlock-FFFEFFFD.umap"), b"x").unwrap();
        let chunks = enumerate_chunks(&tmp).unwrap();
        assert_eq!(chunks.len(), 2);
        assert!(chunks
            .iter()
            .all(|p| p.file_name().unwrap() != "Castle_CellBlock.umap"));
    }

    /// Minimal tempdir without pulling in the `tempfile` crate — the
    /// extractor's dependency budget should stay tight; a few unit
    /// tests don't justify a runtime dep.
    ///
    /// The suffix combines pid + thread id + nanosecond timestamp so
    /// concurrent test threads in the same process don't collide.
    /// Cargo's default test runner parallelises by default, so naming
    /// the dir by pid alone would race two threads into the same path
    /// — one `remove_dir_all` clobbering the other's setup.
    fn tempdir() -> PathBuf {
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!(
            "cimmeria-navmesh-extractor-test-{}-{:?}-{}",
            std::process::id(),
            std::thread::current().id(),
            nanos,
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }
}
