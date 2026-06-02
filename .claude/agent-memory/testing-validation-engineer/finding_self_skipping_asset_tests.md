---
name: finding-self-skipping-asset-tests
description: Integration tests that self-skip on missing cooked assets silently "pass" in CI — coverage gap to flag in audit
metadata:
  type: feedback
---

When an integration test under `crates/navmesh-extractor/tests/` calls `skip_if_missing(&dir, ...)` and returns early because the cooked Castle_CellBlock asset bundle isn't present, the test reports `ok` to cargo. CI then shows a green test that proved nothing.

**Why this matters for revert+audit:** PR #436's `castle_cellblock_walks_static_mesh_actors` is the canonical regression guard for the StaticMeshComponent offset (4 vs 8). The agent verified locally that reverting offset 8 → 4 trips the test. But on a machine without the cooked assets (most CI runners, most reviewer machines), the test silently no-ops. The verification only works in the dev environment with `sgw/Stargate Worlds-QA/...` co-located.

**How to apply:**
1. In audit output, when a test self-skips on missing assets, note it as a coverage gap (CI doesn't run this regression guard).
2. Suggest emitting a `tracing::warn!` or `eprintln!` with `SKIP` prefix that CI can grep for, AND/OR landing a smaller asset-free unit test that pins the offset constant directly (`assert_eq!(STATIC_MESH_COMPONENT_OFFSET, 8)`).
3. If the agent reports they verified the guard locally, treat as VERIFIED-locally but note it's a low-confidence verification in CI.

**Examples from audit:** PR #436 `castle_cellblock_walks_static_mesh_actors` and `castle_cellblock_extract_chunk_without_index_emits_no_geometry` both self-skipped. The non-asset unit tests in `crates/upk-objects/src/static_mesh.rs` (14 tests covering bounds/normals/kdop) all ran cleanly.
