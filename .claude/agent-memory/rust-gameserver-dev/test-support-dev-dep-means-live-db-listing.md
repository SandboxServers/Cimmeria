---
name: test-support-dev-dep-means-live-db-listing
description: Any crate with a cimmeria-test-support dev-dependency must be in tools/test-live-db.{sh,ps1}; for a log capture only (e.g. cimmeria-lab), write a local tracing layer instead
metadata:
  type: project
---

`crates/services/src/live_db_wrapper_tests.rs` fails if a `crates/*` manifest has a `cimmeria-test-support` dev-dependency and is not in `LIVE_DB_CRATES` / `$LiveDbCrates`.

**Why:** TG-PIPE-06 (2026-10-11) added the dev-dep to `cimmeria-lab` just for `LogCapture`. The lab has no live-DB tests and is excluded from CI's workspace jobs, so listing it was wrong; the fix was a local capture layer in `server/handler.rs` (same shape as `supervisor/lifecycle.rs` tests).

**How to apply:** For a crate that only needs log capture, copy the local `Layer` capture; add the dev-dep only when the crate gets real `live_db` tests (then list it in both scripts).
