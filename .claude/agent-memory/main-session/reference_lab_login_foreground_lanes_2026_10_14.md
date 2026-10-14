# Shared foreground during concurrent lab login (2026-10-14)

Source: `crates/lab/src/supervisor/flows/login.rs` and the concurrent fake-IO regression in `login_tests.rs`.

One lab daemon can host multiple client instances, but Windows supplies only one desktop foreground window. Each instance's credential entry focuses its HWND and verifies CEGUI hit tests. Interleaving account and password entry across instances can steal foreground and fail `account_focus` or type into the wrong client. Keep the process-wide async credential lock around the whole account/password pair, including readback and its bounded retry. It must release on errors. The lock protects this transaction only; it is not a cross-process desktop lock. Native two-client UAT is still needed.

Follow-up: Windows may deny both physical activation attempts even with credential entry serialized. `lab_login` now verifies that the target HWND still belongs to its client PID, then treats activation denial as a diagnostic (target and foreground HWND/PID). Input remains PID-scoped via `PostMessage` and virtual focus; CEGUI hit-test, native edit click, and typed-value readback remain mandatory. Source: `crates/lab/src/supervisor/flows/login.rs` and focused flow tests. Native two-client UAT remains pending.

The foreground probe is injected at the supervisor field-preparation seam in tests. The fake bridge still answers the real CEGUI location and dispatch hit-test queries, so a production branch that rejects denied activation fails the regression. After installing the build, run a native recovery probe with a second foreground window to confirm both credentials are read back by the intended client; that remains unverified here.
