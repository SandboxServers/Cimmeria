# AX-26 coder report

Raw `client_input_key` down/up/tap now go through `hold_key_at` (DLL `input_hold` with lease; old DLL falls back to posts). One release path: `Supervisor::release_all_input` (new `supervisor/input_release.rs`) calls `input_release` then clears the HoldBook; used by `client_input_release`, both `server/uat.rs` lease-lost sites and `release_all_holds` (non-fallback). `spawn_input_guards` renews holds every 1 s, started beside both `spawn_sweeper` calls in `main.rs`. `input_key` keeps the hwnd lookup and delegates to `input_key_at` (test seam, lives in input_release.rs so input.rs shrank).

Deviation: `input_key_at` takes the resolved hwnd and calls `hold_key_at` (not `hold_key`) so tests need no game window; behaviour identical.

Tests (supervisor/input_release_tests.rs, server/input_tools_tests.rs): input_key_down_is_a_dll_hold, input_key_tap_holds_then_lets_go, old_dll_key_still_posts, release_all_input_clears_the_book, release_all_input_clears_even_on_error, tool_release_goes_through_the_helper. Each fails on revert of its step (the router test fails because the old tool proxied without clearing the book).

Line counts: supervisor/input.rs 586; server/uat.rs 643; main.rs 309.
No tool added; policy.rs unchanged. Live check is AX-64's key fail-safe row.
Checks: L1 clippy ok; L2 see commit.