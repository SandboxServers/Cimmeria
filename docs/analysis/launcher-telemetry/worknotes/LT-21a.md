# LT-21a worknote

Done: the nine untargeted log-upload calls in `storage/log_upload/{mod.rs,store.rs}` now go through helpers in the new `storage/log_upload/events.rs` (target `launcher.telemetry`). Capture tests: one per helper in `events.rs`, plus `over_the_cap_logs_one_trimmed_event_and_no_blob_name` in `tests.rs` (real over-the-cap path, no `blob` field on any event).

Left: nothing for LT-21a. LT-21b starts from the commit of this packet (same worktree, fresh coder).

Next step: LT-21b, per `work-packets-wave4.md` § LT-21b.

Last check: `cargo test --locked ... -p cimmeria-launcher-engine log_upload` status=ok exit=0 (21 passed, 0 failed).

Accepted deviations:
- `events.rs` is 241 lines against a 180-line target (kept whole rather than split).
- `events.rs` has an extra helper, `log_error_reason`, beyond the per-event helpers.
- `tests.rs` has its own SERIAL pair and a synchronous `block_on` test.
