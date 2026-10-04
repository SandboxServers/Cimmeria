# Effective imported settings (macOS adopted Play)

> **Type:** Implementation handoff
> **Branch:** `launcher/effective-settings`
> **Date:** 2026-10-04
> **Status:** Repair of candidate `d21194410` after review. Not pushed, not merged.

An adopted installation whose publication verifies can use the existing
prerequisite and Play paths with the settings the user reviewed. Nothing here
starts a game: every result below is from fixture content and fixture evidence.

## What the repair changed

The candidate had five confirmed defects. Each was reproduced before it was
fixed, and each fix has a test that fails when the fix is reverted.

| Finding | Fix | Guard |
|---|---|---|
| Play held the state lock across `launch::dispatch`, which locks it again, on every installation | The guard is scoped to admission in `shell/src/host/launch/mod.rs` | `play_releases_the_store_before_dispatch_and_its_failure_path` |
| The whole bundle was unavailable when the patch DLL was missing, so "patches off" could not Play | The patch artifact is optional in `shell/src/host/launch/resources.rs`; the engine decides per installation | `absent_or_replaced_patch_artifact_leaves_the_rest_of_the_bundle_usable`, `fresh_install_is_never_offered_or_admitted_without_the_patch_artifact` |
| The patch setting was read from the adoption record alone | Settings must also match the retained legacy import and the admission checkpoint | `patch_setting_is_bound_beyond_the_adoption_record`, `checkpoint_removal_and_the_journal_digest_both_refuse_the_plan` |
| A refused adopted record made the signed-minimum check fail, and with it launch Inspect | The minimum uses the owner's signed release only | `signed_minimum_is_reported_even_when_imported_settings_are_refused` |
| Tests depended on adoption's macOS-only test module from an ungated module | Own fixtures; macOS-only tests are gated, policy tests run on every host | Compiles on macOS; other hosts not built here |

## Native contract

**Patch policy.** A fresh install keeps the bundled patch contract: Play is not
offered and not admitted without a verified patch artifact. An adopted copy
follows its imported `client_patches.enabled`. Off drops the artifact from the
launch plan and needs no DLL in the bundle. On requires the bundled artifact, and
a missing or replaced one is refused. `admit_launch` rejects resources that
disagree with the policy, so a host that skips `resolve_play_resources` cannot
inject or drop the patch.

**Binding.** `effective_settings::launch_binding` accepts the imported settings
only when all of these agree:

- the provenance in `installed-content.json`;
- the Published adoption record, through the new read-only accessor
  `adoption::published_plan`;
- the admission checkpoint `adoption-plan-<work>.json`, and the Adopt journal
  digest while Adopt is still the latest operation;
- the retained `legacy-import.json`, which re-derives the settings and their
  digest from the exact legacy JSON on every read.

It also requires the reviewed telemetry acceptance for an opted-in import, the
unchanged ordered login servers on the owner intent, the default catalog, and no
custom patch DLL. Imported identity, consent, auth URL and the exact JSON are
read, never rewritten.

**Minimum.** `installed_launcher_minimum` and the minimum check in
`admit_launch` use the owner's signed release and run before imported settings
are consulted.

**Prerequisites.** `runtime_setup_target` offers prerequisites after a
successful Adopt as it does after a successful Install, with the existing
helper, reopen, backend and journal gates. It withholds the offer while imported
settings do not verify, and `prepare_runtime` refuses admission in that case.

**Inspect.** `resources_available` is false only when the bundle cannot satisfy
the policy. Refused settings withhold `installation_id` instead, so the Play view
does not report a missing build resource.

The launch plan has no new field. The selection is the existing nullable
`resources.client_patches`, so earlier serialized plans keep their digest.

## Tests

Run from the repository root. The shell crate needs `frontend/dist`
(`npm ci --ignore-scripts` and `npm run build` in `crates/launcher/desktop/frontend`).

```sh
bash tools/build-lane/lane.sh cargo test --locked --manifest-path crates/launcher/desktop/Cargo.toml --workspace
bash tools/build-lane/lane.sh cargo clippy --locked --manifest-path crates/launcher/desktop/Cargo.toml --workspace --all-targets -- -D warnings
cargo fmt --all --manifest-path crates/launcher/desktop/Cargo.toml -- --check
```

Results on macOS, 2026-10-04: shell 53 passed and 9 ignored, engine 404 passed
and 21 ignored, runtime-probe 16 passed. Clippy and the format check are clean.
`--workspace` matters: the manifest's default member is the engine alone.

The adopted journeys that reach Play admission need a Wine backend, and that
needs the pinned runtime and Windows archive helper. They are `#[ignore]` tests:

```sh
# CIMMERIA_WINE_HELPER, CIMMERIA_WINE_HELPER_SHA256: the pinned archive helper and its build digest
# CIMMERIA_WINE_RUNTIME_TREE: a verified runtime tree to clone, so nothing is downloaded
bash tools/build-lane/lane.sh cargo test --locked --manifest-path crates/launcher/desktop/Cargo.toml \
  -p cimmeria-launcher-engine --lib effective_settings::wine_tests -- --ignored --test-threads=1
bash tools/build-lane/lane.sh cargo test --locked --manifest-path crates/launcher/desktop/Cargo.toml \
  -p cimmeria-launcher-desktop host::launch::adopted_tests::wine_adopted -- --ignored --test-threads=1
```

All three passed on 2026-10-04 with the archive helper whose SHA-256 is
`d0c89fad444cb4dc6478f1db8a5e62bc54d5696ee2a84a63d740bf3a5b92c6a3`:

- engine, patches off: Wine adoption, reopen, refused before prerequisites,
  recorded prerequisite evidence, reopen, one admission with no patch artifact,
  identical retry not dispatched, a changed retry and a second Play refused;
- engine, patches on: missing artifact refused, replaced artifact refused,
  verified artifact admitted once;
- shell: Wine adoption, host reopen, prerequisites offered, prerequisite
  admission retained once and cancelled, offer withdrawn and restored with the
  record, one Play through `launch_command` with an identical retry, `NotStarted`.

Each asserts the ordered login servers, unchanged preferences, preserved
imported identity and telemetry answer, and a byte-identical source snapshot.

## What these tests are not

- **Not gameplay UAT.** The content is an inert signed ZIP with a stub
  executable. No game, login or world entry was attempted.
- **Not a prepared runtime.** Prerequisite success is recorded through the public
  observation entry points. No prerequisite installer ran and no game prefix was
  created. The tests move their private runtime copy aside after adoption, so
  the retained prerequisite and Play workers stop before starting Wine.
- **Not a real bundle.** Launch helper, patch DLL and graphics artifacts are
  fixture files with fixture digests.
- **Not built off macOS.** The Windows and Linux builds of this change were not
  compiled or run. The shell resource test is gated to macOS because its fixture
  paths are not valid Windows verbatim paths.

## For integration

- **Edits outside this packet's ownership**, all needed by the change:
  `engine/src/lib.rs` (one re-export),
  `engine/src/storage/adoption/{mod.rs,publication.rs}` (the read-only
  `published_plan` accessor; `verify_provenance` behaves as before), and one
  assertion in `engine/src/storage/adoption/tests.rs`, which pinned the old
  "adopted copy is always Busy" gate.
- **`adoption-plan-<work>.json` must outlive publication.** Removing it after a
  successful adoption makes the copy unplayable.
- **`publication.rs` is 504 lines**, four over the soft cap, after the accessor.
- **`Resources` is unchanged.** `resolve_play_resources` uses struct update
  syntax, so a new `Resources` field needs no edit there. Its return type is
  `Result<Option<Resources>, StorageError>`; `None` means the policy needs a
  patch artifact the bundle lacks.
- **`crates/launcher/desktop/docs/launch.md` is stale** where it says the shell
  requires client patches and never drops them. That now holds for fresh installs
  and for adopted copies with patches on.
- **The Play view has no state for refused imported settings.** It shows the
  generic "finish installation and compatibility checks" line. A distinct
  message needs a `LaunchStatus` field and frontend work.
- **Engine `admit_runtime_setup` does not check imported settings.** The host
  does, in `prepare_runtime`. Moving the check into the engine is a change to
  `engine/src/storage/runtime_setup/`.

## Remaining gates

1. Native Windows build, clippy and tests in CI.
2. An adoption UI that publishes through `start_preview_wine`; until then no
   user-made adopted copy can reach this path.
3. A packaged build with the real launch helper, patch DLL and graphics
   artifacts, exercising patches off with the DLL absent.
4. Real prerequisite preparation for an adopted copy, then a supervised Play of
   the real client by the coordinator.
