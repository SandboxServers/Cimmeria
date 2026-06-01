//! Live-DB guards for `load_cover_sets` / `load_cover_nodes`.
//!
//! The loader has three soft-failure paths the production code logs+
//! skips: unknown `height` enum, unknown `quality` enum, wrong-length
//! `tail` blob. Only the tail branch is reachable through the schema —
//! PG's `ECoverHeight` / `ECoverQuality` enum constraints reject an
//! out-of-range string at INSERT time, so reproducing the
//! `from_sql_name` returning `None` branch from PG would require
//! `ALTER TYPE` (global state mutation; refuse). The from_sql_name
//! `None` branch itself is unit-tested in `tests.rs` and below.
//!
//! Sentinel base: `0x7000_1900` (next free slot past
//! `request_visuals_live_db_tests::TEST_BASE = 0x7000_1800`). All
//! `chunk_id`s used here are well outside the seed-data range and
//! cleaned up via `ON DELETE CASCADE` from `cover_sets` → `cover_nodes`.

use sqlx::PgPool;

use super::loader::{load_cover_nodes, load_cover_sets, CoverLoadError};
use super::types::{CoverHeight, CoverQuality};
use crate::test_support::require_db_or_skip;

/// Sentinel base for cover-loader tests. One slot past
/// `request_visuals_live_db_tests::TEST_BASE = 0x7000_1800` — see the
/// neighbour map in `crates/services/src/base/character/delete_live_db_tests.rs`.
const TEST_BASE: i32 = 0x7000_1900;

/// `i32::MAX`-safe set of chunk_ids reserved by these tests; we delete by
/// exact id (never by range) on entry and cleanup so a sibling module's
/// slot can't be reached.
const CHUNK_HAPPY: i32 = TEST_BASE + 1;
const CHUNK_BAD_TAIL: i32 = TEST_BASE + 4;
const CHUNK_MIXED_HAPPY_AND_BAD: i32 = TEST_BASE + 5;

/// Delete by exact sentinel only — TESTING.md "Cleanup must `DELETE WHERE
/// <id> = $sentinel`, not by range".
async fn cleanup(pool: &PgPool, chunk_ids: &[i32]) {
    // ON DELETE CASCADE wipes the cover_nodes rows; we still delete
    // those first to keep the failure mode explicit if cascade ever
    // changes.
    for cid in chunk_ids {
        let _ = sqlx::query("DELETE FROM resources.cover_nodes WHERE chunk_id = $1")
            .bind(cid)
            .execute(pool)
            .await;
        let _ = sqlx::query("DELETE FROM resources.cover_sets WHERE chunk_id = $1")
            .bind(cid)
            .execute(pool)
            .await;
    }
}

async fn insert_cover_set(pool: &PgPool, chunk_id: i32, name: &str) {
    sqlx::query(
        "INSERT INTO resources.cover_sets \
            (chunk_id, chunk_name, primary_author, has_variant, src_pak) \
         VALUES ($1, $2, 'test', false, 'test.pak')",
    )
    .bind(chunk_id)
    .bind(name)
    .execute(pool)
    .await
    .expect("insert cover_sets");
}

#[allow(clippy::too_many_arguments)]
async fn insert_cover_node(
    pool: &PgPool,
    chunk_id: i32,
    node_id: i32,
    pos: (f32, f32, f32),
    orient: f32,
    height: &str,
    quality: &str,
    tail: &[u8],
) {
    sqlx::query(
        "INSERT INTO resources.cover_nodes \
            (chunk_id, node_id, pos_x, pos_y, pos_z, orient, height, quality, tail) \
         VALUES ($1, $2, $3, $4, $5, $6, $7::resources.\"ECoverHeight\", \
                 $8::resources.\"ECoverQuality\", $9)",
    )
    .bind(chunk_id)
    .bind(node_id)
    .bind(pos.0)
    .bind(pos.1)
    .bind(pos.2)
    .bind(orient)
    .bind(height)
    .bind(quality)
    .bind(tail)
    .execute(pool)
    .await
    .expect("insert cover_nodes");
}

/// Insert a cover_node with a deliberately bogus height/quality enum
/// value (a value the SQL enum accepts only because we route via a raw
/// string column we then cast). This is what the schema-drift soft-fail
/// branch was written for. We do this by writing the row with a valid
/// enum value first, then `UPDATE`ing to an out-of-range string via a
/// catalog manipulation — but the simpler path is to write straight to
/// the `tail` field's bad-length variant, since "unknown enum" can't
/// be achieved with a real PG enum without ALTER TYPE.
///
/// We instead exercise the soft-fail by inserting a tail with a
/// wrong length: 8 bytes instead of 4. The canonical extractor always
/// emits 4-byte tails; the loader logs+skips anything else. Pinning
/// this branch is the load-bearing assertion.
async fn insert_cover_node_with_long_tail(pool: &PgPool, chunk_id: i32, node_id: i32) {
    sqlx::query(
        "INSERT INTO resources.cover_nodes \
            (chunk_id, node_id, pos_x, pos_y, pos_z, orient, height, quality, tail) \
         VALUES ($1, $2, 0.0, 0.0, 0.0, 0.0, \
                 'HEIGHT_Mid'::resources.\"ECoverHeight\", \
                 'QUALITY_Best'::resources.\"ECoverQuality\", $3)",
    )
    .bind(chunk_id)
    .bind(node_id)
    .bind(vec![0u8; 8])
    .execute(pool)
    .await
    .expect("insert cover_nodes with 8-byte tail");
}

#[tokio::test]
async fn load_cover_sets_returns_inserted_row_with_correct_fields() {
    let pool = require_db_or_skip!();
    cleanup(&pool, &[CHUNK_HAPPY]).await;
    insert_cover_set(&pool, CHUNK_HAPPY, "test/happy/loader").await;

    let sets = load_cover_sets(&pool)
        .await
        .expect("load_cover_sets must succeed");
    let row = sets
        .iter()
        .find(|s| s.chunk_id == CHUNK_HAPPY)
        .expect("inserted chunk must round-trip");
    // Field-by-field — confirms the `r.get(...)` extraction maps the
    // right column to the right struct field. A column swap (e.g.,
    // `primary_author` ↔ `chunk_name`) would corrupt trigger
    // payloads silently otherwise.
    assert_eq!(row.chunk_name, "test/happy/loader");
    assert_eq!(row.primary_author, "test");
    assert!(!row.has_variant);
    assert_eq!(row.src_pak, "test.pak");

    cleanup(&pool, &[CHUNK_HAPPY]).await;
}

#[tokio::test]
async fn load_cover_nodes_returns_inserted_row_with_correct_enums() {
    let pool = require_db_or_skip!();
    cleanup(&pool, &[CHUNK_HAPPY]).await;
    insert_cover_set(&pool, CHUNK_HAPPY, "test/happy/nodes").await;
    insert_cover_node(
        &pool,
        CHUNK_HAPPY,
        0,
        (1.5, 2.5, 3.5),
        0.25,
        "HEIGHT_High",
        "QUALITY_Better",
        &[0xAA, 0xBB, 0xCC, 0xDD],
    )
    .await;

    let nodes = load_cover_nodes(&pool)
        .await
        .expect("load_cover_nodes must succeed");
    let n = nodes
        .iter()
        .find(|n| n.chunk_id == CHUNK_HAPPY && n.node_id == 0)
        .expect("inserted node must round-trip");
    // Tight: exact pos, orient, enum mapping, tail bytes. A regression
    // that swapped column order in the SELECT would produce a different
    // shape; the cast helpers (`from_sql_name`) catch enum drift.
    assert!((n.pos.x - 1.5).abs() < 1e-4);
    assert!((n.pos.y - 2.5).abs() < 1e-4);
    assert!((n.pos.z - 3.5).abs() < 1e-4);
    assert!((n.orient - 0.25).abs() < 1e-4);
    assert_eq!(n.height, CoverHeight::High);
    assert_eq!(n.quality, CoverQuality::Better);
    assert_eq!(n.tail, [0xAA, 0xBB, 0xCC, 0xDD]);

    cleanup(&pool, &[CHUNK_HAPPY]).await;
}

#[tokio::test]
async fn load_cover_nodes_skips_row_with_wrong_length_tail() {
    let pool = require_db_or_skip!();
    cleanup(&pool, &[CHUNK_BAD_TAIL]).await;
    insert_cover_set(&pool, CHUNK_BAD_TAIL, "test/bad_tail").await;
    insert_cover_node_with_long_tail(&pool, CHUNK_BAD_TAIL, 0).await;

    let nodes = load_cover_nodes(&pool)
        .await
        .expect("load_cover_nodes must succeed even with a bad-tail row");
    assert!(
        !nodes
            .iter()
            .any(|n| n.chunk_id == CHUNK_BAD_TAIL && n.node_id == 0),
        "row with 8-byte tail must be skipped, not loaded with a truncated tail"
    );

    cleanup(&pool, &[CHUNK_BAD_TAIL]).await;
}

#[tokio::test]
async fn load_cover_nodes_skips_bad_row_but_keeps_sibling() {
    // Two nodes in the same chunk: one with a valid tail, one with an
    // 8-byte tail. The loader must skip the bad one while preserving
    // the good one — proving the per-row `continue` keeps the loop
    // going rather than aborting on first malformed row.
    let pool = require_db_or_skip!();
    cleanup(&pool, &[CHUNK_MIXED_HAPPY_AND_BAD]).await;
    insert_cover_set(&pool, CHUNK_MIXED_HAPPY_AND_BAD, "test/mixed").await;
    insert_cover_node(
        &pool,
        CHUNK_MIXED_HAPPY_AND_BAD,
        0,
        (0.0, 0.0, 0.0),
        0.0,
        "HEIGHT_Mid",
        "QUALITY_Good",
        &[0, 0, 0, 0],
    )
    .await;
    insert_cover_node_with_long_tail(&pool, CHUNK_MIXED_HAPPY_AND_BAD, 1).await;

    let nodes = load_cover_nodes(&pool)
        .await
        .expect("load_cover_nodes must succeed");

    let in_chunk: Vec<_> = nodes
        .iter()
        .filter(|n| n.chunk_id == CHUNK_MIXED_HAPPY_AND_BAD)
        .collect();
    assert_eq!(
        in_chunk.len(),
        1,
        "exactly one row from this chunk should survive (node_id=0); \
         the bad-tail sibling (node_id=1) must be skipped. Got: {in_chunk:?}"
    );
    assert_eq!(in_chunk[0].node_id, 0);

    cleanup(&pool, &[CHUNK_MIXED_HAPPY_AND_BAD]).await;
}

/// `CoverLoadError::Display` formatting + `Error::source()` return the
/// underlying sqlx error. Pure unit-style test; exercises the impls
/// that the live-DB happy paths above never trip into.
#[test]
fn cover_load_error_display_and_source_propagate_inner_sqlx() {
    let inner = sqlx::Error::RowNotFound;
    let inner_msg = inner.to_string();
    let err: CoverLoadError = inner.into();

    let displayed = format!("{err}");
    assert!(
        displayed.starts_with("cover db query failed: "),
        "Display must prefix with the loader-specific tag so logs at the \
         call site can distinguish cover-load failures from other sqlx \
         errors; got {displayed:?}"
    );
    assert!(
        displayed.contains(&inner_msg),
        "Display must include the inner sqlx error's message verbatim so \
         the cause isn't lost; got {displayed:?}"
    );

    let src = std::error::Error::source(&err)
        .expect("source() must return the wrapped sqlx error for `?` chaining");
    assert_eq!(
        src.to_string(),
        inner_msg,
        "source() must hand back the exact inner sqlx error so callers can \
         downcast or chain `Caused by:` lines"
    );
}

/// `CoverHeight::from_sql_name` skip-on-unknown path (sibling to the
/// quality one). Already covered by `cover_height_sql_round_trip` in
/// `tests.rs` for the `None` return; this is the parallel for `quality`
/// to ensure both branches stay symmetrical when the enums diverge in
/// future schema work.
#[test]
fn cover_quality_from_sql_name_returns_none_for_unknown_value() {
    assert_eq!(CoverQuality::from_sql_name("QUALITY_NONEXISTENT"), None);
    assert_eq!(CoverQuality::from_sql_name(""), None);
    // Round-trip sanity so we don't pass for the wrong reason.
    assert_eq!(
        CoverQuality::from_sql_name("QUALITY_None"),
        Some(CoverQuality::None_)
    );
}
