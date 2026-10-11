---
name: lab-mcp-raw-args-and-live-db-crate
description: lab-mcp tools - rmcp Parameters<T> parse errors skip the audit row (raw-Value wrapper with delegated JsonSchema fixes it); adding live-DB tests to a crate needs test-support dev-dep + test-live-db.{sh,ps1} line
metadata:
  type: project
---

Found 2026-10-11 while building `server_stage_fixture` (lab-fixtures FX-04).

- **rmcp rejects malformed args before the tool body**, with its own
  invalid_params message and no `audit::emit` row. To audit and prefix a
  bad call, take `Parameters<Wrapper>` where `Wrapper(Value)` deserializes
  any JSON and `impl JsonSchema` delegates `schema_name`/`schema_id`/
  `json_schema` to the real args struct (schemars 1.x via `rmcp::schemars`),
  then parse inside the tool. See `crates/lab-mcp/src/tools/fixture.rs`.
- **Refusal shapes differ on the wire.** `Err(ErrorData::invalid_params)`
  reaches the lab runner as `ToolOutcome::err("lab-mcp error -32602: ...")`
  (extract_jsonrpc_result); the older tools' `CallToolResult::error` arrives
  as an `isError` result. Both are `ok = false`.
- **A crate's first live-DB test**: add `cimmeria-test-support` as a
  dev-dependency, a `#[cfg(test)] mod test_support { pub(crate) use
  cimmeria_test_support::*; }` shim **at the end of lib.rs** (clippy
  `items_after_test_module` fires otherwise), and the crate to both
  `tools/test-live-db.sh` and `.ps1` (guard
  `live_db_wrapper_lists_every_test_support_crate` in cimmeria-services).
  `cargo hakari generate --diff` stayed clean for a workspace dev-dep.
- In quiet lane mode a passing live-DB run prints no proof the DB was hit;
  a revert proof (gate removed -> test fails with real row data) is the
  evidence.

Related: [[tooling-filter-and-path-traps]], [[clippy-items-after-test-module]].
