# Shared foreground during concurrent lab login (2026-10-14)

Source: `crates/lab/src/supervisor/flows/login.rs` and the concurrent fake-IO regression in `login_tests.rs`.

One lab daemon can host multiple client instances, but Windows supplies only one desktop foreground window. Each instance's credential entry focuses its HWND and verifies CEGUI hit tests. Interleaving account and password entry across instances can steal foreground and fail `account_focus` or type into the wrong client. Keep the process-wide async credential lock around the whole account/password pair, including readback and its bounded retry. It must release on errors. The lock protects this transaction only; it is not a cross-process desktop lock. Native two-client UAT is still needed.
