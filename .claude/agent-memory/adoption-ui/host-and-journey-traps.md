---
name: Adoption host and journey traps
description: Defects in verified-copy adoption that only a real native path exposes, and how to reproduce the Wine and UAT evidence
type: project
---

# Adoption host and journey traps

2026-10-04, branch `launcher/adoption-ui`. Source: tracing the real paths while
repairing the first candidate, then the tests named below. Not yet verified to
`docs/` standard.

## A canned bridge hides these

- **The review must carry the journal revision current when preparation
  finishes.** `preparation::Ownership::claim` begins and observes an Adopt
  operation, which advances the operation revision by two. A review that keeps
  the revision from the request can never be confirmed. Guard:
  `reviewed_copy_publishes_once_and_preserves_source_import_identity_and_consent`.
- **Confirmation requires an absent destination.** `publication` creates the
  folder itself with `create_dir`. A folder dialog can only return an existing
  folder, so the host appends `Stargate Worlds` to the chosen location and the
  engine preview refuses an existing destination up front.
- **`Ownership` settles on `Drop` with `try_lock`.** A worker that fails while
  any reader holds the state mutex leaves a Running operation with no worker
  until restart. Workers that hold no guard call `Ownership::release()`, which
  waits. Guard:
  `a_cancelled_download_settles_its_owner_even_while_a_reader_holds_the_state`.

## Host rules

- Lock order is the adoption mutex, then the store. Engine workers take only
  the store. Release a `Preview` outside the adoption mutex: releasing waits
  for the store.
- Host methods that may reach `mac_wine::repair_recovery::stop_reference` call
  `Handle::block_on`. Run them on `spawn_blocking`, as the Tauri adapter does;
  calling them from an async test thread panics.
- The Windows helper extracts ZIP as well as RAR (it calls the shared
  `unpack::unpack`), so a host Wine test can use the signed ZIP fixture.
  `unpack::test_fixtures` (the RAR writer) is `cfg(test)` in the engine only.

## Reproducing the evidence

- Wine tests are `#[ignore]` and need `CIMMERIA_WINE_HELPER`,
  `CIMMERIA_WINE_HELPER_SHA256` and `CIMMERIA_WINE_RUNTIME_TREE`. The runtime
  tree is copied into each isolated state root; the engine verifies the copy.
- `npm run uat:adoption` needs `ADOPTION_UAT_BINARY`, the shell test binary.
  The standalone desktop workspace builds into
  `crates/launcher/desktop/target/`, not the directory the lane prints. The
  updater UAT bridge is in the engine test binary instead.
- The UAT keeps its state under `ADOPTION_UAT_ROOT` so the script can kill the
  bridge and reopen the same store.

Details: [adoption UI worknote](../../../docs/analysis/playtests/2026-10-03-macos-wine/worknotes/adoption-ui.md).
