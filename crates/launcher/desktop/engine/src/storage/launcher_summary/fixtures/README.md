# Launcher-summary golden wire fixtures

These files are the wire contract between the desktop engine (this workspace)
and the server's `launcher_summary` ingest route in `crates/admin-api`. Both
sides test against the same files, so neither can drift alone.

| File | Meaning |
|---|---|
| `mint-request.json` | The exact dev-session mint body for a summary session |
| `request-all.json` | One batch covering every enum value and numeric bound; every element is valid. Launch rows carry only a `starting` phase and no `duration_ms`, as the engine produces them |
| `request-mixed.json` | A batch whose results are accepted, duplicate and rejected, in that order |
| `response-mixed.json` | The server's response to `request-mixed.json` on a fresh process |
| `request-install-failure.json` | The body the engine really sends for one failed install, recorded from a real install worker with injected ids and clock on Linux x86_64; the engine test substitutes its own `os` and `arch` |

Compare them as JSON values, never as bytes. A change here must pass both the
engine tests and the admin-api tests in the same PR.
