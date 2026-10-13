# Lab world point matching (2026-10-13)

The Cellblock NID Guard corpse can be rendered at its server point while a point-only click hovers the player's avatar instead. A click by entity ID can find a different body pixel or camera view and succeeds. The lab now matches a world point to rendered entities within 0.25 m, reports multiple candidate IDs as ambiguous, and requires a matched ID on `Unit.MouseOver` before input. This is a lab client interaction rule; it does not change server interaction range or content.

Source: `crates/lab/src/supervisor/world/click.rs`, `occlusion_tests.rs`; issue #1708 live reproduction, 2026-10-13.
