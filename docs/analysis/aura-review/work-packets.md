# aura-review work packets

Source: reviews in `.claude/reports/aura-review/` (main checkout). Base: origin/main.

## AR-01 content-engine log fix and speed-test fix (S)

Files (only these): `crates/content-engine/src/loader/mod.rs`, `crates/entity/src/movement_validation/tests.rs`.

1. `loader/mod.rs` near line 258-261: the `None =>` arm warns "Unknown action_type, skipping" for every dropped action, including i32-range drops that `opt_i32` already warned about. Reword the message to `"action dropped (unknown action_type or unusable params; see the preceding WARN)"`. Keep the fields.
2. `movement_validation/tests.rs` near line 241: test `zero_distance_at_zero_dt_never_warns` passes with `SpeedWindow::push` deleted (distance 0.0 hits the early exit). Rewrite its body to seed the window first: `observe_speed(1, 0.8125, 0.4, TOP)`, then `assert!(observe_speed(1, 0.0, 0.0, TOP).is_none())`, then `assert!(observe_speed(1, 0.8125, 0.0, TOP).is_none())`. Match the file's existing call signature and constants exactly; read the neighbouring tests first.
3. Run through the lane: `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-content-engine` and the entity crate (check the real package names in each Cargo.toml). Run from the worktree (`Set-Location <wt>;` every call).

Not in scope: N2/N3 (wait for #1973), anything under `loader/action_params.rs`.

## AR-02 #1967 stale docs (S) - worktree on PR #1967 branch

Source: `.claude/reports/aura-review/minigame-inventory-review.md` notes N3, N4 (read them; they are exact). Files only: `docs/gameplay/minigame-system.md` (CRLF, keep), `.claude/agent-memory/minigame-systems-advisor/public-port-hardening.md`, `.claude/agent-memory/minigame-systems-advisor/game-implementation-status.md`, `crates/minigame/src/minigame/session.rs` (comments only). Verify with the greps in the notes. Doc/comment-only: run `pwsh tools/build-lane/lane.ps1 cargo check -p cimmeria-minigame` (confirm package name).

## AR-03 #1963 stale docs and test labels (S) - worktree on PR #1963 branch

Source: same report, notes N5, N6, N7. Files only: `docs/guides/unified-uat.md` (K38, mail row 3a, bank row 8), `docs/commands.md` (/gmgiveitem row), `docs/gameplay/mail-system.md`, `crates/cell-catalog/src/item_placement.rs` (comment `// Pin: passes before and after #1675` on the two named tests; no logic change), `docs/analysis/ready-batch-2026-10/worknotes/P08.md` (add status line `Done in #1963; kept as a record`). CRLF in docs. Run `cargo check -p` for cell-catalog via the lane.

## AR-04 #1966 speed window gaps (M) - worktree on PR #1966 branch

Source: `.claude/reports/aura-review/content-movement-review.md` notes N5, N7 (read them). N5: add `MovementValidator::clear_speed_window(&mut self, entity_id: u32)` and call it in the `movement_unrestricted` branch of `crates/cell-world/src/cell/space_manager/client_move.rs`; add the seam test described. N7: add `window_secs` field to the speed WARN row as described, plus a seam test. Run nextest for the entity and cell-world crates via the lane. If a test helper the note assumes does not exist, stop and report `blocked`.
