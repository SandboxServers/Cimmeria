//! PostgreSQL → in-memory loader for `resources.cover_sets` + `cover_nodes`.
//!
//! Runs once at cell-service startup. The resulting `Vec<CoverNode>` feeds
//! the [`super::spatial::CoverIndex`] build pass; the `Vec<CoverSetMeta>`
//! is preserved for trigger payloads (the `chunk_name` is human-readable
//! and useful for chain-author debugging).
//!
//! Edge cases the loader treats as soft failures (logged, skipped, not
//! propagated):
//! - Unknown `height` enum value (DB schema drift) — skipped.
//! - Unknown `quality` enum value — skipped.
//! - Wrong-length `tail` blob — skipped (canonical extractor always
//!   produces 4 bytes).
//!
//! Hard failures (propagated as `CoverLoadError`):
//! - PG connection / query error.
//! - Schema mismatch (column missing).

use cimmeria_common::Vector3;
use sqlx::PgPool;
use sqlx::Row;
use std::fmt;

use super::types::{CoverHeight, CoverNode, CoverQuality, CoverSetMeta};

/// Errors returned by the loader. Most callers convert to `tracing::error!`
/// and fall back to an empty `Cover` service so the cell process still
/// starts.
#[derive(Debug)]
pub enum CoverLoadError {
    /// PostgreSQL query/connection failure.
    Sqlx(sqlx::Error),
}

impl fmt::Display for CoverLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sqlx(e) => write!(f, "cover db query failed: {e}"),
        }
    }
}

impl std::error::Error for CoverLoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Sqlx(e) => Some(e),
        }
    }
}

impl From<sqlx::Error> for CoverLoadError {
    fn from(e: sqlx::Error) -> Self {
        Self::Sqlx(e)
    }
}

/// Load every `cover_sets` row. Sorted by `chunk_id` for stable ordering.
pub async fn load_cover_sets(pool: &PgPool) -> Result<Vec<CoverSetMeta>, CoverLoadError> {
    let rows = sqlx::query(
        "SELECT chunk_id, chunk_name, primary_author, has_variant, src_pak \
         FROM resources.cover_sets \
         ORDER BY chunk_id",
    )
    .fetch_all(pool)
    .await?;

    let mut sets = Vec::with_capacity(rows.len());
    for r in &rows {
        sets.push(CoverSetMeta {
            chunk_id: r.get("chunk_id"),
            chunk_name: r.get("chunk_name"),
            primary_author: r.get("primary_author"),
            has_variant: r.get("has_variant"),
            src_pak: r.get("src_pak"),
        });
    }

    tracing::info!(count = sets.len(), "Loaded cover sets");
    Ok(sets)
}

/// Load every `cover_nodes` row.
///
/// Rows with an out-of-range height/quality enum or a wrong-length tail
/// are skipped with a `tracing::warn!` — these would only fire if the
/// seed data were hand-edited or the schema drifted; the canonical
/// extractor always produces clean rows.
pub async fn load_cover_nodes(pool: &PgPool) -> Result<Vec<CoverNode>, CoverLoadError> {
    // The `height` and `quality` columns come back from sqlx as strings
    // (the SQL enum's textual form) — we then map to the Rust enums via
    // `CoverHeight::from_sql_name` / `CoverQuality::from_sql_name`.
    let rows = sqlx::query(
        "SELECT chunk_id, node_id, pos_x, pos_y, pos_z, orient, \
                height::text AS height_text, quality::text AS quality_text, tail \
         FROM resources.cover_nodes \
         ORDER BY chunk_id, node_id",
    )
    .fetch_all(pool)
    .await?;

    let mut nodes = Vec::with_capacity(rows.len());
    let mut skipped_height = 0usize;
    let mut skipped_quality = 0usize;
    let mut skipped_tail = 0usize;
    for r in &rows {
        let chunk_id: i32 = r.get("chunk_id");
        let node_id: i32 = r.get("node_id");
        let pos_x: f32 = r.get("pos_x");
        let pos_y: f32 = r.get("pos_y");
        let pos_z: f32 = r.get("pos_z");
        let orient: f32 = r.get("orient");
        let height_text: &str = r.get("height_text");
        let quality_text: &str = r.get("quality_text");
        let tail_bytes: &[u8] = r.get("tail");

        let height = match CoverHeight::from_sql_name(height_text) {
            Some(h) => h,
            None => {
                skipped_height += 1;
                tracing::warn!(
                    chunk_id,
                    node_id,
                    height = height_text,
                    "cover_nodes: unknown height enum, skipping row"
                );
                continue;
            }
        };
        let quality = match CoverQuality::from_sql_name(quality_text) {
            Some(q) => q,
            None => {
                skipped_quality += 1;
                tracing::warn!(
                    chunk_id,
                    node_id,
                    quality = quality_text,
                    "cover_nodes: unknown quality enum, skipping row"
                );
                continue;
            }
        };
        if tail_bytes.len() != 4 {
            skipped_tail += 1;
            tracing::warn!(
                chunk_id,
                node_id,
                tail_len = tail_bytes.len(),
                "cover_nodes: tail blob is not 4 bytes, skipping row"
            );
            continue;
        }
        let mut tail = [0u8; 4];
        tail.copy_from_slice(tail_bytes);

        nodes.push(CoverNode {
            chunk_id,
            node_id,
            pos: Vector3::new(pos_x, pos_y, pos_z),
            orient,
            height,
            quality,
            tail,
        });
    }

    tracing::info!(
        count = nodes.len(),
        skipped_height,
        skipped_quality,
        skipped_tail,
        "Loaded cover nodes"
    );
    Ok(nodes)
}
