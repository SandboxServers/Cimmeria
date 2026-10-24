# LT-17a worknote

Done: commit 0437244e4 (LT-17a: launch lifecycle events). Engine `storage/launch/events.rs` (204 lines) and `events_tests.rs` (67 lines) added; `mod.rs`, `supervisor.rs`, `worker.rs` (631 lines, +25) and shell `host/launch/mod.rs` edited per the packet.

Left: nothing for LT-17a. Next step: LT-17b in the same worktree.

Last check: `status=ok exit=0` (cargo test -p cimmeria-launcher-desktop launch, 7 passed).