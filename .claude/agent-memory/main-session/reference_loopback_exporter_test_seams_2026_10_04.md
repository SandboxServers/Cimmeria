---
name: reference-loopback-exporter-test-seams
description: "Desktop engine exporter tests: how a dead loopback endpoint behaves under WSL2 mirrored networking, where a wiremock responder runs, and what the engine's reqwest lacks (2026-10-04)"
metadata:
  type: reference
---

Learned while adding `storage/launcher_summary/export.rs` and its wiremock
tests to the desktop engine (`crates/launcher/desktop/engine`) on 2026-10-04:

- **A closed `127.0.0.1` port is not always refused.** Under WSL2 with
  `networkingMode=mirrored`, a connection to an unbound port on `127.0.0.1`
  (or `localhost`) hangs until the caller's own deadline; it is not refused.
  `[::1]` and other `127/8` addresses refuse at once. A test that needs a
  dead endpoint uses `http://[::1]:9` (macOS has no `127.0.0.2` by default, so
  that one is not portable either). With `127.0.0.1:9` the refused-connection
  test took 90 s (three 30 s deadlines) instead of milliseconds.
- **A wiremock 0.6 responder is synchronous and runs on the mock server's own
  thread, while that server's state lock is held.** It can therefore lock
  shared test state at an exact point of a request, which is how the opt-out
  races are sequenced. It must not call `server.received_requests()` (same
  lock). A request the client gave up on (deadline) is recorded when the server
  reads it; a response held with `set_delay` keeps the server free, a blocking
  responder does not.
- **`MockServer::start()` hands out pooled servers.** A responder closure that
  owns an `Arc` keeps it alive after the test. Responders hold a `Weak` to the
  state instead.
- **The engine's `reqwest` has no `json` feature** (`rustls-tls`, `stream`
  only). Bodies are built with `serde_json::to_vec` plus an explicit
  `Content-Type`, and responses are read with `Response::chunk()`.
- **Selecting on the consent cancel token around a request would make the
  generation re-checks untestable**: the token fires inside the responder, so
  the answer would never reach the code under test. The exporter selects on
  it only around its wait, and decides under the lock.
