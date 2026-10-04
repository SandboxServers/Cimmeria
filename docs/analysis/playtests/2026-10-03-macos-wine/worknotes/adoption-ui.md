# Verified copy adoption UI

> **Type:** Implementation handoff
> **Audience:** Desktop launcher integrators
> **Date:** 2026-10-04
> **Branch:** `launcher/adoption-ui`

This replaces the first candidate on this branch, which review found could not
complete the journey. The findings were treated as hypotheses and checked
against the code; three further defects turned up while tracing the real paths
and are listed under "Defects the first candidate's tests could not see".

## What the user can now do

Settings → **Verified copy adoption**, after importing legacy settings:

1. **Choose location for the copy…** opens a native folder dialog. The launcher
   creates a new `Stargate Worlds` folder inside the chosen folder. An existing
   folder of that name is refused before any work starts.
2. The signed release is read from the fixed catalog, its seed and patches are
   downloaded natively, and the pinned Windows helper extracts the reference in
   a private Wine prefix. Progress and **Cancel preparation** are shown.
3. The review lists the original and new folders, the signed release, counts
   for each classification, the paths that differ (the first 60), what happens
   to modified, missing and extra files, the ordered login servers, the patch
   setting, and the telemetry and diagnostics statements. Each required
   confirmation is a visible, unchecked box.
4. **Create verified copy** starts a retained copy with progress and
   **Cancel copy**. When it finishes, the status names the new folder.

Interrupted work is offered only the recovery the native state allows, each
behind a confirmation panel: **Remove preparation files…** for an interrupted
or leftover reference, **Recover adoption…** when a verified staged copy
exists, **Abandon adoption…** when nothing was published.

## Contract

`shell/src/host/adoption/` exposes two commands. `choose_adoption_destination`
takes no arguments. `adoption_command` takes `inspect`, `dismiss`, `cancel`,
`confirm`, `recover`, `abandon` or `abandon_preparation` with identities,
revisions and closed booleans only; unknown fields are rejected.

- **One lock order.** One adoption mutex, then the store. Engine workers take
  only the store. A native preview is released outside the adoption mutex.
- **Status never waits on a copy.** The copy runs without the state mutex, so
  other views stay responsive. While publication holds the store, status serves
  the last facts with live worker progress.
- **A refused Confirm keeps the review.** Handle, revisions and consent are
  checked before the preview is consumed. The review's revisions are the ones
  current when preparation finished.
- **Wine helper admission.** Production previews use `start_preview_wine` with
  the build-pinned helper, verified before admission. Without it the status
  reports `helper_unavailable`; other platforms report `unsupported_platform`.
- **Signed, bounded downloads.** `engine/src/storage/adoption/artifacts.rs`
  refuses a declared length other than the signed size, stops a body that runs
  past it, verifies the signed hash, and keeps nothing on failure. Verified
  blobs live in `adoption-artifacts/` in the state root, are reused after a
  dismissed review, and are removed after publication.
- **Test transport.** `adoption::test_support` (engine feature `test-support`)
  accepts a manifest URL only when its host is a literal loopback address and
  follows no redirects. Production entry points cannot reach it.
- **Other views** are refreshed when a saved operation or preferences revision
  changes, not on every poll.

## Defects the first candidate's tests could not see

1. The review carried the operation revision from before preparation was
   admitted. The journal had advanced by then, so every real Confirm would have
   been refused as stale.
2. A folder dialog returns an existing folder, but confirmation requires the
   destination to be absent. Every real Confirm would have failed after the
   whole preparation.
3. A preview that failed while another thread held the state mutex left a
   Running operation with no worker until restart.

## Validation

Run from the repository root. Rust commands go through the build lane with
`CARGO_BUILD_JOBS=2`.

| Command | Result |
|---|---|
| `cargo test --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-engine -p cimmeria-launcher-desktop` | engine 406 passed, 19 ignored; shell 60 passed, 11 ignored |
| `… -p cimmeria-launcher-engine --lib storage::adoption` | 34 passed, 2 ignored |
| `… -p cimmeria-launcher-desktop adoption` | 11 passed, 3 ignored |
| `… -p cimmeria-launcher-desktop adoption::wine_tests -- --ignored --test-threads=1` | 2 passed (pinned helper under Wine) |
| `… -p cimmeria-launcher-engine --lib storage::adoption::preparation_tests::retained_wine_reference -- --ignored --test-threads=1` | 2 passed (RAR and RAR/CAB fixtures) |
| `cargo clippy … -p cimmeria-launcher-engine -p cimmeria-launcher-desktop --all-targets -- -D warnings` | clean |
| `cargo fmt --manifest-path crates/launcher/desktop/Cargo.toml --all -- --check` | clean |
| `npm run check`, `npm test`, `npm run build` in `crates/launcher/desktop/frontend` | typecheck clean, 69 passed, bundle built |
| `npm run uat:adoption` with `ADOPTION_UAT_BINARY` | passed, portable backend |
| the same with the three Wine variables below | passed, Wine backend |
| `npm run uat:updater`, `npm run uat:migration` | passed (they share the composition) |

The Wine runs need `CIMMERIA_WINE_HELPER`, its independently recorded
`CIMMERIA_WINE_HELPER_SHA256`, and `CIMMERIA_WINE_RUNTIME_TREE` (a prepared
runtime, copied into each isolated state root). The helper used was the one
recorded in the [preparation handoff](published-adoption-preparation.md),
`d0c89fad…c6a3`. It predates the shared archive preflight.

`npm run uat:adoption` drives the production Effect workflow and view against
the production host through a JSON-lines bridge. The bridge owns a real store,
a legacy game tree and a signed loopback origin; the script stands in for the
folder dialog only. It kills the bridge process mid-download and reopens the
same store, loses the Confirm reply, and reads source bytes, the import record,
ownership and backend identity from disk.

Six guards were proven by reverting the fix and watching the test fail: the
bounded download, the worker settling under a held state mutex, cancel on a
dropped preview worker, the copy outside the state mutex, non-blocking status
during a copy, and the review surviving a refused Confirm.

## What this does not show

- All archives are small inert fixtures. The published multi-gigabyte client,
  its RAR/CAB seed and the production HTTPS catalog were not used.
- No game was started. The native folder dialog, Tauri IPC and the packaged
  window were not exercised; there is no visual, focus or layout evidence.
- Windows has no adoption backend. The UI says so.

## Remaining gates

- **Prerequisites and Play for an adopted copy.** The copy records the Wine
  backend and helper identity, but `runtime_setup_target` offers prerequisites
  only after a succeeded Install, and `installed_content_readonly` refuses
  adoption records. Both belong to the effective-settings work. The UAT prints
  the observed prerequisite target, currently none.
- **Cancelled or abandoned copies leave their destination folder.** The engine
  deliberately never deletes a destination. The UI tells the user to choose a
  different location.
- **Changing the diagnostics preference during a copy** makes publication
  refuse the stale preferences. The copy then needs Abandon and a new location.
  Before this change the state mutex blocked that edit for the whole copy.
- **A cancel inside the helper window** is treated as uncertain by the engine
  and needs **Remove preparation files…**. Wine prefixes of finished or
  dismissed previews are retained, as before.
- **Docs.** `crates/launcher/desktop/docs/migration.md` still says the adoption
  API is not exposed in Settings. The coordinator owns that guide.

## Findings outside this packet

- `shell/src/host/held_download.rs` reads the request from a socket that
  inherits the listener's non-blocking mode. On macOS it panicked with
  `WouldBlock` in 3 of 25 runs of the shell suite, each time under an adoption
  test that used it. The adoption tests now use their own origin. The repair
  and game-update tests use the same fixture; they did not fail in those runs.
  The fix is `stream.set_nonblocking(false)` after `accept`.
- `engine/src/storage/adoption/publication.rs` is 545 lines, over the soft cap.
  It was left whole because the effective-settings work reads from it.
- `frontend/ui/style.css` was not in scope and the CSP forbids inline styles,
  so the difference list is bounded natively instead of scrolling.
