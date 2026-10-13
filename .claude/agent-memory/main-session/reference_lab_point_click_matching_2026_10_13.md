# Lab world point matching (2026-10-13)

The Cellblock NID Guard corpse can be rendered at its server point while a point-only click hovers the player's avatar instead. A click by entity ID can find a different body pixel or camera view and succeeds. The lab now matches a world point to rendered entities within 0.05 m, reports multiple candidate IDs as ambiguous, and requires a matched ID on `Unit.MouseOver` before input. This is a lab client interaction rule; it does not change server interaction range or content.

Local lab UAT with God mode on (2026-10-13): the old installed build clicked Frost's point near the player's avatar, left `mouse_over_after=2`, and opened no dialog. With this branch installed, a fresh character at the same spot clicked Frost's point, observed `own_avatar` on its first hover, searched upward, verified Frost's ID 100494, and opened the Frost dialog. The unnamed Guard corpse point similarly resolved rendered ID 100491, verified hover, and opened the Guard dialog and tutorial.

Source: `crates/lab/src/supervisor/world/click.rs`, `occlusion_tests.rs`; issue #1708 live reproduction, 2026-10-13.

Review follow-up: a successful body-height probe may not describe the final cursor state after moving back to its chosen pixel. Read `Unit.MouseOver` again after that move and reject a point click when it no longer equals the matched ID. The simulator reproduces this with mouse-over switching to the avatar on the repeated cursor move.
