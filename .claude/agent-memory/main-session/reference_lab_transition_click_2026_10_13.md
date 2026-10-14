# Report-only world click during a load, 2026-10-13

Source: `crates/lab/src/supervisor/world/click.rs`, the `report_only_click_survives_world_load_before_post_click_read` regression test, and the live SGU client check on 2026-10-13.

The Cellblock Armory ring right-click dispatched native input and completed mission 688, but the client immediately started its Castle load and the tool's post-click UI read timed out. For a transition step, `client_world_click` with `expect = "nothing"` now confirms click dispatch and leaves the after-state fields null with `after_observed = false`. The UAT row must separately assert the destination world and mission state; click dispatch alone is not proof of transition.

The bridge-unavailable-after-click case passes in the simulated client test. A live right-click on General Hammond with `expect = "nothing"` returned `hover_verified = true`, N1, and `after_observed = false`. This check did not reproduce the world load itself; the next full Cellblock replay must exercise that path.
