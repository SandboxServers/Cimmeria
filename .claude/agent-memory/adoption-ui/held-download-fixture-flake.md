---
name: HeldDownload fixture flake
description: The shared shell test origin intermittently panics with WouldBlock on macOS
type: project
---

# `HeldDownload` reads from a non-blocking socket

**Status 2026-10-04: fixed on `launcher/uat-integration`.** The accepted socket
is made blocking, and `held_download::tests` reproduces the panic when that line
is removed. The text below is the original observation.

2026-10-04. Observed in 3 of 25 runs of the shell test binary on macOS
(Darwin 25.6, Apple Silicon), in a test that used
`shell/src/host/held_download.rs`.

The fixture sets its listener non-blocking so the accept loop can be stopped.
On macOS the accepted socket inherits that mode. The fixture then calls
`stream.read(..).unwrap()`, which panics with `WouldBlock` when the request has
not arrived yet. The client sees a closed connection, so the test under it
fails with a network error or a timeout rather than naming the fixture.

Fix: call `stream.set_nonblocking(false)` right after `accept`. The adoption
tests use their own `StalledOrigin` (`shell/src/host/adoption/fixture.rs`),
which does this and showed 0 failures in 40 runs. The repair and game-update
cancellation tests still use `HeldDownload`; they did not fail in those 25
runs, so the race there is unconfirmed.
